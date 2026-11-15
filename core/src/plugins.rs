//! CFW plugin manager for `seplugins/game.txt`, `vsh.txt`, `pops.txt`.
//!
//! Each file lists one plugin per line: `ms0:/seplugins/foo.prx 1` where the
//! trailing number is the enabled flag. ARK-4 also supports runlevel-scoped
//! `PLUGINS.TXT` CSV, which we surface read-only via `list_all_sections`.

use crate::device::PspPaths;
use crate::error::{Error, Result};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Game,
    Vsh,
    Pops,
}

impl Section {
    pub fn file_name(&self) -> &'static str {
        match self {
            Section::Game => "game.txt",
            Section::Vsh => "vsh.txt",
            Section::Pops => "pops.txt",
        }
    }

    pub fn parse(name: &str) -> Option<Section> {
        match name {
            "game" => Some(Section::Game),
            "vsh" => Some(Section::Vsh),
            "pops" => Some(Section::Pops),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginEntry {
    /// `ms0:/seplugins/...` path as written on the stick.
    pub path: String,
    pub enabled: bool,
    pub exists: bool,
    pub section: Section,
}

fn section_file(paths: &PspPaths, section: Section) -> PathBuf {
    paths.seplugins().join(section.file_name())
}

/// Read all entries for a section.
pub fn list(paths: &PspPaths, section: Section) -> Result<Vec<PluginEntry>> {
    let file = section_file(paths, section);
    if !file.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&file)?;
    let mut entries = parse_lines(&text, section);
    // Resolve existence against the stick root now that we have it.
    for entry in &mut entries {
        let disk_path = entry.path.replacen("ms0:/", "", 1);
        entry.exists = paths.root.join(&disk_path).is_file();
    }
    Ok(entries)
}

fn parse_lines(text: &str, section: Section) -> Vec<PluginEntry> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: <path> <0|1>
        let (path_part, flag) = match line.rsplit_once(char::is_whitespace) {
            Some((p, f)) => (p.trim(), f.trim()),
            None => (line, "0"),
        };
        let enabled = matches!(flag, "1" | "on" | "true" | "enabled");
        out.push(PluginEntry {
            path: path_part.to_string(),
            enabled,
            exists: false,
            section,
        });
    }
    out
}

/// Toggle one plugin line's enable flag (adds the line if missing).
pub fn toggle(paths: &PspPaths, section: Section, plugin_path: &str, enable: bool) -> Result<()> {
    let file = section_file(paths, section);
    let mut lines: Vec<String> = if file.is_file() {
        fs::read_to_string(&file)?
            .lines()
            .map(str::to_string)
            .collect()
    } else {
        Vec::new()
    };
    let plugin_path = plugin_path.trim();
    let mut found = false;
    for line in lines.iter_mut() {
        if line.split_whitespace().next() == Some(plugin_path) {
            *line = format!("{} {}", plugin_path, if enable { 1 } else { 0 });
            found = true;
        }
    }
    if !found {
        lines.push(format!("{} {}", plugin_path, if enable { 1 } else { 0 }));
    }
    write_lines(&file, &lines)
}

/// Remove a plugin line from a section.
pub fn remove(paths: &PspPaths, section: Section, plugin_path: &str) -> Result<()> {
    let file = section_file(paths, section);
    let mut lines: Vec<String> = if file.is_file() {
        fs::read_to_string(&file)?
            .lines()
            .map(str::to_string)
            .collect()
    } else {
        Vec::new()
    };
    let plugin_path = plugin_path.trim();
    lines.retain(|line| line.split_whitespace().next() != Some(plugin_path));
    write_lines(&file, &lines)
}

/// Copy a `.prx` into `seplugins/` and register it (enabled) in `section`.
pub fn add(paths: &PspPaths, section: Section, prx: &std::path::Path) -> Result<PluginEntry> {
    if prx.extension().map(|e| e.to_string_lossy().to_lowercase()) != Some("prx".into()) {
        return Err(Error::Invalid("plugin must be a .prx file".into()));
    }
    let name = prx
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::Invalid("invalid plugin path".into()))?;
    let seplugins = paths.seplugins();
    fs::create_dir_all(&seplugins)?;
    let dest = seplugins.join(&name);
    std::fs::copy(prx, &dest)?;
    let stick_path = format!("ms0:/seplugins/{}", name);
    toggle(paths, section, &stick_path, true)?;
    Ok(PluginEntry {
        path: stick_path,
        enabled: true,
        exists: true,
        section,
    })
}

fn write_lines(file: &std::path::Path, lines: &[String]) -> Result<()> {
    fs::create_dir_all(file.parent().ok_or_else(|| Error::Invalid("bad path".into()))?)?;
    let mut text = lines.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    fs::write(file, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plugin_lines() {
        let text = "ms0:/seplugins/cwcheat/cwcheat.prx 1\nms0:/seplugins/old.prx 0\n\nms0:/seplugins/x.prx";
        let entries = parse_lines(text, Section::Game);
        assert_eq!(entries.len(), 3);
        assert!(entries[0].enabled);
        assert!(!entries[1].enabled);
        assert!(!entries[2].enabled);
        assert_eq!(entries[0].path, "ms0:/seplugins/cwcheat/cwcheat.prx");
    }

    #[test]
    fn toggle_and_remove_roundtrip() {
        let dir = std::env::temp_dir().join(format!("godsend-plg-{}", std::process::id()));
        let paths = PspPaths::new(&dir);
        toggle(&paths, Section::Vsh, "ms0:/seplugins/a.prx", true).unwrap();
        toggle(&paths, Section::Vsh, "ms0:/seplugins/a.prx", false).unwrap();
        let entries = list(&paths, Section::Vsh).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].enabled);
        toggle(&paths, Section::Game, "ms0:/seplugins/b.prx", true).unwrap();
        remove(&paths, Section::Game, "ms0:/seplugins/b.prx").unwrap();
        assert!(list(&paths, Section::Game).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
