//! Scan a PSP memory stick for installed games and disc images.

use crate::device::PspPaths;
use crate::error::Result;
use crate::pbp::{Pbp, Section};
use crate::sfo::Sfo;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameKind {
    /// PSN/digital title in PSP/GAME with a proper serial id.
    Digital,
    /// Homebrew EBOOT (no or non-standard PARAM.SFO).
    Homebrew,
    /// CFW loader folder (ARK_Loader, UPDATE, ...) shown separately in UI.
    Loader,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameEntry {
    /// Stable id: "game/<folder>".
    pub id: String,
    /// Folder name on the stick.
    pub folder: String,
    pub title: String,
    pub title_id: Option<String>,
    pub category: Option<String>,
    pub kind: GameKind,
    pub path: String,
    pub size_bytes: u64,
    pub has_icon: bool,
    pub psp_system_ver: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscImage {
    /// Stable id: "iso/<file name>".
    pub id: String,
    pub file: String,
    /// `iso` | `cso` | `pbp`
    pub format: String,
    pub title: Option<String>,
    pub title_id: Option<String>,
    pub is_umd: bool,
    pub path: String,
    pub size_bytes: u64,
}

/// Everything installed on the stick.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Library {
    pub games: Vec<GameEntry>,
    pub images: Vec<DiscImage>,
}

/// Scan PSP/GAME (EBOOT folders) plus the ISO/ directory.
pub fn scan(paths: &PspPaths) -> Result<Library> {
    Ok(Library {
        games: scan_games(paths)?,
        images: scan_images(paths)?,
    })
}

pub fn scan_games(paths: &PspPaths) -> Result<Vec<GameEntry>> {
    let mut games = Vec::new();
    let game_dir = paths.game();
    if !game_dir.is_dir() {
        return Ok(games);
    }
    let mut folders: Vec<PathBuf> = fs::read_dir(&game_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();

    for folder in folders {
        let eboot = folder.join("EBOOT.PBP");
        if !eboot.is_file() {
            continue;
        }
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut entry = GameEntry {
            id: format!("game/{}", name),
            folder: name.clone(),
            title: name.clone(),
            title_id: None,
            category: None,
            kind: GameKind::Homebrew,
            path: folder.to_string_lossy().to_string(),
            size_bytes: dir_size(&folder),
            has_icon: false,
            psp_system_ver: None,
            version: None,
        };

        // PARAM.SFO next to the EBOOT, or inside the PBP itself.
        let sfo_bytes = match fs::read(folder.join("PARAM.SFO")) {
            Ok(bytes) => Some(bytes),
            Err(_) => Pbp::read_header(&eboot)
                .ok()
                .and_then(|pbp| pbp.read_section(&eboot, Section::ParamSfo).ok().flatten()),
        };
        if let Some(bytes) = sfo_bytes {
            if let Ok(sfo) = Sfo::parse(&bytes) {
                entry.title = sfo.title().unwrap_or(&name).to_string();
                entry.title_id = sfo.title_id().map(str::to_owned);
                entry.category = sfo.category().map(str::to_owned);
                entry.psp_system_ver = sfo.psp_system_ver().map(str::to_owned);
                entry.version = sfo.version().map(str::to_owned);
            }
        }

        entry.has_icon = icon_path(paths, &entry.id).is_some() || folder.join("ICON0.PNG").is_file();
        entry.kind = classify(&entry);
        games.push(entry);
    }
    Ok(games)
}

fn classify(entry: &GameEntry) -> GameKind {
    // CFW loader folders installed by ARK.
    if entry.folder.starts_with("ARK_") {
        return GameKind::Loader;
    }
    // A well-formed serial id (e.g. UCUS-98737, NPUG-80124) means an official
    // digital title. Homebrew rarely ships one.
    if let Some(id) = &entry.title_id {
        if is_serial(id) {
            return GameKind::Digital;
        }
    }
    GameKind::Homebrew
}

fn is_serial(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 10
        && bytes[0..4].iter().all(u8::is_ascii_uppercase)
        && bytes[4] == b'-'
        && bytes[5..].iter().all(u8::is_ascii_digit)
}

pub fn scan_images(paths: &PspPaths) -> Result<Vec<DiscImage>> {
    let mut images = Vec::new();
    for dir in [paths.iso(), paths.iso_video()] {
        if !dir.is_dir() {
            continue;
        }
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        files.sort();
        for file in files {
            let ext = file
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let format = match ext.as_str() {
                "iso" => "iso",
                "cso" => "cso",
                "pbp" => "pbp",
                _ => continue,
            };
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let size = fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
            let mut image = DiscImage {
                id: format!("iso/{}", name),
                file: name.clone(),
                format: format.to_string(),
                title: None,
                title_id: None,
                is_umd: false,
                path: file.to_string_lossy().to_string(),
                size_bytes: size,
            };
            if format == "iso" {
                if let Ok(info) = crate::iso::analyze(&file) {
                    image.title = info.title;
                    image.title_id = info.title_id;
                    image.is_umd = info.is_umd;
                }
            } else if format == "cso" {
                if let Ok(info) = crate::cso::probe(&file) {
                    image.is_umd = info.is_cso && info.total_bytes > 0;
                }
            }
            images.push(image);
        }
    }
    Ok(images)
}

/// Cached ICON0.PNG path for a game/image id, when available.
///
/// Icons are extracted once (from the game folder, or out of the PBP/ISO) and
/// stored under the app cache so the UI can request them cheaply.
pub fn icon_path(paths: &PspPaths, id: &str) -> Option<PathBuf> {
    let cache = crate::config::icons_cache_dir().join(format!("{}.png", hash_key(id)));
    if cache.is_file() {
        return Some(cache);
    }
    let source = match id.strip_prefix("game/") {
        Some(folder) => paths.game().join(folder).join("ICON0.PNG"),
        None => return None,
    };
    if !source.is_file() {
        return None;
    }
    // Fall through: caller uses icon_bytes to populate the cache.
    if cache.is_file() {
        Some(cache)
    } else {
        Some(source)
    }
}

/// Read (and lazily cache) an ICON0.PNG for a game or ISO id.
pub fn icon_bytes(paths: &PspPaths, id: &str) -> Option<Vec<u8>> {
    let cache = crate::config::icons_cache_dir().join(format!("{}.png", hash_key(id)));
    if let Ok(bytes) = fs::read(&cache) {
        return Some(bytes);
    }
    let bytes: Option<Vec<u8>> = match id.strip_prefix("game/") {
        Some(folder) => {
            let folder = paths.game().join(folder);
            match fs::read(folder.join("ICON0.PNG")) {
                Ok(b) => Some(b),
                Err(_) => {
                    let eboot = folder.join("EBOOT.PBP");
                    Pbp::read_header(&eboot)
                        .ok()
                        .and_then(|p| p.read_section(&eboot, Section::Icon0).ok().flatten())
                }
            }
        }
        None => match id.strip_prefix("iso/") {
            Some(file) => {
                let file = paths.iso().join(file);
                crate::iso::read_icon0(&file).ok().flatten()
            }
            None => None,
        },
    };
    if let Some(bytes) = &bytes {
        let _ = fs::create_dir_all(crate::config::icons_cache_dir());
        let _ = fs::write(&cache, bytes);
    }
    bytes
}

/// Remove a game folder (or disc image) from the stick.
pub fn delete_entry(paths: &PspPaths, id: &str) -> Result<()> {
    let target = match id.strip_prefix("game/") {
        Some(folder) => paths.game().join(folder),
        None => match id.strip_prefix("iso/") {
            Some(file) => paths.iso().join(file),
            None => return Err(crate::error::Error::NotFound(id.to_string())),
        },
    };
    if !target.exists() {
        return Err(crate::error::Error::NotFound(target.display().to_string()));
    }
    let result = if target.is_dir() {
        fs::remove_dir_all(&target)
    } else {
        fs::remove_file(&target)
    };
    let cache = crate::config::icons_cache_dir().join(format!("{}.png", hash_key(id)));
    let _ = fs::remove_file(cache);
    result?;
    Ok(())
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn hash_key(key: &str) -> String {
    // FNV-1a 64: stable, dependency-free cache key.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in key.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_detection() {
        assert!(is_serial("UCUS-98737"));
        assert!(is_serial("NPUG-80124"));
        assert!(!is_serial("ARK_LOADER"));
        assert!(!is_serial("homebrew1"));
    }

    #[test]
    fn icon_cache_key_is_stable() {
        assert_eq!(hash_key("game/FrogGBA"), hash_key("game/FrogGBA"));
        assert_ne!(hash_key("game/A"), hash_key("game/B"));
    }
}
