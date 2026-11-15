//! File manager (local <-> PSP browser) and the local library
//! (downloaded PSP content) with copy jobs in both directions.

use crate::device::PspPaths;
use crate::error::{Error, Result};
use crate::transfer::JobContext;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct FmEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    /// `iso` | `cso` | `pbp` | `eboot_dir` | `other`
    pub kind: String,
}

/// List one directory (local side). Sorts dirs first.
pub fn list_dir(path: &Path) -> Result<Vec<FmEntry>> {
    let mut out = Vec::new();
    if !path.is_dir() {
        return Err(Error::NotFound(path.display().to_string()));
    }
    for entry in fs::read_dir(path)?.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        if name.starts_with('.')
            || name.starts_with("._")
            || lower.ends_with(".aria2")
            || lower.ends_with(".part")
            || lower.ends_with(".partial")
            || lower.ends_with(".tmp")
            || lower.ends_with(".crdownload")
        {
            continue;
        }
        let is_dir = p.is_dir();
        let mut kind = "other".to_string();
        if is_dir {
            if p.join("EBOOT.PBP").is_file() {
                kind = "eboot_dir".into();
            }
        } else {
            kind = ext_kind(&name);
        }
        out.push(FmEntry {
            name,
            path: p.to_string_lossy().to_string(),
            is_dir,
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
            kind,
        });
    }
    out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(out)
}

fn ext_kind(name: &str) -> String {
    let lower = name.to_lowercase();
    for ext in ["iso", "cso", "pbp"] {
        if lower.ends_with(ext) {
            return ext.to_string();
        }
    }
    "other".into()
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalGame {
    /// Stable id: `local/<file or folder name>`.
    pub id: String,
    pub name: String,
    /// `iso` | `cso` | `pbp` | `eboot_dir`
    pub kind: String,
    pub path: String,
    pub size_bytes: u64,
    /// Serial id when a PARAM.SFO was readable (eboot dirs and pbp only).
    pub serial: Option<String>,
    pub title: Option<String>,
}

/// Scan the local library root (default: download dir) for PSP content.
pub fn local_library(root: &Path) -> Result<Vec<LocalGame>> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(root)?.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with("._") {
            continue;
        }
        let mut serial = None;
        let mut title = None;
        let kind;
        if p.is_dir() && p.join("EBOOT.PBP").is_file() {
            kind = "eboot_dir".into();
            if let Ok(bytes) = fs::read(p.join("PARAM.SFO")) {
                if let Ok(sfo) = crate::sfo::Sfo::parse(&bytes) {
                    serial = sfo.title_id().map(str::to_owned);
                    title = sfo.title().map(str::to_owned);
                }
            }
        } else if p.is_file() {
            let k = ext_kind(&name);
            if k == "other" {
                continue;
            }
            kind = k;
            if kind == "pbp" {
                if let Ok(pbp) = crate::pbp::Pbp::read_header(&p) {
                    if let Ok(Some(bytes)) = pbp.read_section(&p, crate::pbp::Section::ParamSfo) {
                        if let Ok(sfo) = crate::sfo::Sfo::parse(&bytes) {
                            serial = sfo.title_id().map(str::to_owned);
                            title = sfo.title().map(str::to_owned);
                        }
                    }
                }
            } else if kind == "iso" {
                if let Ok(info) = crate::iso::analyze(&p) {
                    serial = info.title_id;
                    title = info.title;
                }
            }
        } else {
            continue;
        }
        out.push(LocalGame {
            id: format!("local/{name}"),
            name,
            kind,
            path: p.to_string_lossy().to_string(),
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
            serial,
            title,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// Copy a local file or folder to the PSP. `dest_kind` selects `iso`
/// (ISO folder) or `game` (PSP/GAME for eboot dirs and pbp).
pub fn copy_to_psp(
    local_path: &Path,
    paths: &PspPaths,
    dest_kind: &str,
    ctx: &JobContext,
) -> Result<PathBuf> {
    ctx.check_cancel()?;
    let name = local_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::Invalid("bad path".into()))?;
    let dest = match dest_kind {
        "iso" => {
            let dest = paths.iso().join(&name);
            std::fs::create_dir_all(paths.iso())?;
            dest
        }
        _ => {
            let dest = paths.game().join(&name);
            std::fs::create_dir_all(paths.game())?;
            dest
        }
    };
    ctx.message("copying to PSP");
    copy_sized_tree(local_path, &dest, ctx)?;
    Ok(dest)
}

/// Copy a PSP entry (game folder or ISO image) to the local library.
pub fn copy_from_psp(
    psp_id: &str,
    paths: &PspPaths,
    local_root: &Path,
    ctx: &JobContext,
) -> Result<PathBuf> {
    ctx.check_cancel()?;
    let (src, name) = if let Some(folder) = psp_id.strip_prefix("game/") {
        (paths.game().join(folder), folder.to_string())
    } else if let Some(file) = psp_id.strip_prefix("iso/") {
        (paths.iso().join(file), file.to_string())
    } else {
        return Err(Error::Invalid(format!("bad psp id {psp_id}")));
    };
    if !src.exists() {
        return Err(Error::NotFound(src.display().to_string()));
    }
    std::fs::create_dir_all(local_root)?;
    let dest = local_root.join(&name);
    ctx.message("copying from PSP");
    copy_sized_tree(&src, &dest, ctx)?;
    Ok(dest)
}

fn copy_sized_tree(src: &Path, dest: &Path, ctx: &JobContext) -> Result<()> {
    if src.is_dir() {
        let total = crate::transfer::dir_size(src);
        ctx.progress(0, total);
        let mut done = 0u64;
        let mut stack = vec![(src.to_path_buf(), dest.to_path_buf())];
        while let Some((from, to)) = stack.pop() {
            ctx.check_cancel()?;
            fs::create_dir_all(&to)?;
            for entry in fs::read_dir(&from)?.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name.starts_with("._") || name == "__MACOSX" {
                    continue;
                }
                let f = entry.path();
                let t = to.join(&name);
                if f.is_dir() {
                    stack.push((f, t));
                } else {
                    done += crate::transfer::copy_with_progress(&f, &t, ctx)?;
                    ctx.progress(done.min(total), total);
                }
            }
        }
        ctx.progress(total, total);
    } else {
        let total = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        ctx.progress(0, total);
        crate::transfer::copy_with_progress(src, dest, ctx)?;
    }
    Ok(())
}

/// Delete a local file or folder (file manager + downloads management).
pub fn delete_local(path: &Path, allowed_root: &Path) -> Result<()> {
    if !path.starts_with(allowed_root) && !allowed_root.starts_with(path) {
        return Err(Error::Invalid("path outside allowed roots".into()));
    }
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.is_file() {
        fs::remove_file(path)?;
    } else {
        return Err(Error::NotFound(path.display().to_string()));
    }
    Ok(())
}

/// Create a directory in the local tree.
pub fn mkdir(parent: &Path, name: &str) -> Result<PathBuf> {
    let safe = crate::media::fat_safe_name(name);
    let dir = parent.join(safe);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_library_kinds() {
        let dir = std::env::temp_dir().join(format!("godsend-fm-{}", std::process::id()));
        let root = dir.join("lib");
        fs::create_dir_all(root.join("MyGame")).unwrap();
        fs::write(root.join("MyGame/EBOOT.PBP"), b"x").unwrap();
        let sfo = crate::sfo::build_sfo(&[
            ("TITLE", crate::sfo::SfoValue::Utf8("LocoRoco".into())),
            ("TITLE_ID", crate::sfo::SfoValue::Utf8("UCUS-98716".into())),
        ]);
        fs::write(root.join("MyGame/PARAM.SFO"), &sfo).unwrap();
        fs::write(root.join("game.iso"), b"iso-bytes").unwrap();
        fs::write(root.join("readme.txt"), b"skip").unwrap();

        let games = local_library(&root).unwrap();
        assert_eq!(games.len(), 2);
        let eboot = games.iter().find(|g| g.kind == "eboot_dir").unwrap();
        assert_eq!(eboot.title.as_deref(), Some("LocoRoco"));
        assert_eq!(eboot.serial.as_deref(), Some("UCUS-98716"));
        let iso = games.iter().find(|g| g.kind == "iso").unwrap();
        assert_eq!(iso.name, "game.iso");

        mkdir(&root, "new: folder").unwrap();
        assert!(root.join("new_ folder").is_dir());
        delete_local(&root.join("new_ folder"), &root).unwrap();
        fs::remove_dir_all(&dir).ok();
    }
}
