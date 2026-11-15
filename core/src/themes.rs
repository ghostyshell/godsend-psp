//! XMB theme management: `.ptf` (official themes) and `.ctf` (CXMB custom
//! themes, only active when the CXMB plugin is enabled for VSH).

use crate::device::PspPaths;
use crate::error::{Error, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct ThemeEntry {
    pub name: String,
    /// `ptf` | `ctf`
    pub format: String,
    pub size_bytes: u64,
}

pub fn list(paths: &PspPaths) -> Result<Vec<ThemeEntry>> {
    let dir = paths.theme();
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext != "ptf" && ext != "ctf" {
            continue;
        }
        out.push(ThemeEntry {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            format: ext,
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Install (copy) a theme file into PSP/THEME.
pub fn install(paths: &PspPaths, src: &Path) -> Result<ThemeEntry> {
    let ext = src
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext != "ptf" && ext != "ctf" {
        return Err(Error::Invalid("theme must be a .ptf or .ctf file".into()));
    }
    let dir = paths.theme();
    fs::create_dir_all(&dir)?;
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::Invalid("invalid theme path".into()))?;
    let dest = dir.join(&name);
    std::fs::copy(src, &dest)?;
    Ok(ThemeEntry {
        name,
        format: ext,
        size_bytes: fs::metadata(&dest).map(|m| m.len()).unwrap_or(0),
    })
}

pub fn delete(paths: &PspPaths, name: &str) -> Result<()> {
    let path = paths.theme().join(name);
    // Never delete anything outside the theme dir.
    if !path.starts_with(paths.theme()) {
        return Err(Error::Invalid("theme path escapes THEME dir".into()));
    }
    if !path.is_file() {
        return Err(Error::NotFound(name.to_string()));
    }
    fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_list_delete() {
        let dir = std::env::temp_dir().join(format!("godsend-thm-{}", std::process::id()));
        let paths = PspPaths::new(&dir);
        let fake = dir.join("source");
        std::fs::create_dir_all(&fake).unwrap();
        let src = fake.join("cool.ctf");
        std::fs::write(&src, b"theme-bytes").unwrap();

        let entry = install(&paths, &src).unwrap();
        assert_eq!(entry.name, "cool.ctf");
        assert_eq!(entry.format, "ctf");
        assert_eq!(list(&paths).unwrap().len(), 1);

        let escape = std::path::Path::new("../../etc/passwd");
        let _ = escape;
        delete(&paths, "cool.ctf").unwrap();
        assert!(list(&paths).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
