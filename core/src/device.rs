//! Mounted volume discovery and PSP detection.
//!
//! A PSP in USB mode exposes its Memory Stick as a plain USB mass storage
//! volume whose root contains a `PSP` directory (and, with CFW, an `ISO`
//! directory). Detection is therefore filesystem based and needs no libusb.

use crate::error::{Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// A mounted volume that may or may not be a PSP.
#[derive(Debug, Clone, Serialize)]
pub struct Volume {
    /// Mount point (e.g. `/Volumes/MEMORY STICK` or `E:\`).
    pub mount: String,
    /// Volume label as reported by the OS / directory name.
    pub label: String,
    /// True when the volume root contains a `PSP` directory.
    pub is_psp: bool,
    /// Free/total bytes when the OS exposes them, else 0.
    pub free_bytes: u64,
    pub total_bytes: u64,
}

/// Filesystem layout of a PSP memory stick.
#[derive(Debug, Clone)]
pub struct PspPaths {
    /// Volume mount point.
    pub root: PathBuf,
}

impl PspPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Validate that `root` looks like a PSP memory stick.
    pub fn validate(&self) -> Result<()> {
        if self.game().is_dir() || self.psp().is_dir() {
            Ok(())
        } else {
            Err(Error::DeviceRequired(format!(
                "{} does not look like a PSP memory stick (no PSP/ directory)",
                self.root.display()
            )))
        }
    }

    pub fn psp(&self) -> PathBuf {
        self.root.join("PSP")
    }
    /// PSP/GAME: homebrew, PSN digital games and CFW loaders.
    pub fn game(&self) -> PathBuf {
        self.psp().join("GAME")
    }
    /// PSP/SAVEDATA: game saves (and ARK-4's ARK_01234 folder).
    pub fn savedata(&self) -> PathBuf {
        self.psp().join("SAVEDATA")
    }
    /// PSP/MUSIC: audio, artist/album subfolders allowed.
    pub fn music(&self) -> PathBuf {
        self.psp().join("MUSIC")
    }
    /// PSP/VIDEO: videos (MP4).
    pub fn video(&self) -> PathBuf {
        self.psp().join("VIDEO")
    }
    /// PSP/PHOTO.
    pub fn photo(&self) -> PathBuf {
        self.psp().join("PHOTO")
    }
    /// PSP/THEME: XMB themes (.ptf official, .ctf with CXMB).
    pub fn theme(&self) -> PathBuf {
        self.psp().join("THEME")
    }
    /// ISO/: disc images loaded by the CFW ISO driver.
    pub fn iso(&self) -> PathBuf {
        self.root.join("ISO")
    }
    /// VIDEO: legacy ISO/VIDEO folder used by some loaders.
    pub fn iso_video(&self) -> PathBuf {
        self.root.join("VIDEO")
    }
    /// seplugins/: CFW plugin loader lists (game.txt, vsh.txt, pops.txt).
    pub fn seplugins(&self) -> PathBuf {
        self.root.join("seplugins")
    }
}

/// List candidate mount roots for the current platform.
pub fn scan_volumes() -> Vec<Volume> {
    candidate_roots()
        .into_iter()
        .filter_map(inspect_volume)
        .collect()
}

/// All PSP volumes currently mounted.
pub fn psp_volumes() -> Vec<Volume> {
    scan_volumes().into_iter().filter(|v| v.is_psp).collect()
}

/// First PSP volume, or `NoDevice`.
pub fn require_psp() -> Result<(Volume, PspPaths)> {
    let mut volumes = psp_volumes();
    volumes.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
    let volume = volumes
        .into_iter()
        .next()
        .ok_or(Error::NoDevice)?;
    let paths = PspPaths::new(&volume.mount);
    paths.validate()?;
    Ok((volume, paths))
}

fn inspect_volume(root: PathBuf) -> Option<Volume> {
    let label = volume_label(&root);
    let is_psp = root.join("PSP").is_dir();
    let (free_bytes, total_bytes) = space(&root);
    let mount = root.to_string_lossy().to_string();
    Some(Volume {
        mount,
        label,
        is_psp,
        free_bytes,
        total_bytes,
    })
}

fn volume_label(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

#[cfg(target_os = "macos")]
fn candidate_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let volumes = Path::new("/Volumes");
    if let Ok(entries) = std::fs::read_dir(volumes) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Skip hidden system volumes (PreBoot, .Spotlight-V100 mounts, ...).
            if path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(true)
            {
                continue;
            }
            if path.is_dir() {
                out.push(path);
            }
        }
    }
    out
}

#[cfg(target_os = "windows")]
fn candidate_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.is_dir() {
            out.push(root);
        }
    }
    out
}

#[cfg(all(unix, not(target_os = "macos")))]
fn candidate_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let user = std::env::var("USER").unwrap_or_default();
    let mut bases: Vec<PathBuf> = vec![
        PathBuf::from("/media"),
        PathBuf::from("/run/media"),
        PathBuf::from("/mnt"),
    ];
    if !user.is_empty() {
        bases.push(PathBuf::from("/media").join(&user));
        bases.push(PathBuf::from("/run/media").join(&user));
    }
    for base in bases {
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                    out.push(path);
                }
            }
        }
    }
    out
}

fn space(root: &Path) -> (u64, u64) {
    match fs4::free_space(root) {
        Ok(free) => {
            let total = fs4::total_space(root).unwrap_or(0);
            (free, total)
        }
        Err(_) => (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psp_paths_layout() {
        let p = PspPaths::new("/Volumes/MS");
        assert_eq!(p.game(), PathBuf::from("/Volumes/MS/PSP/GAME"));
        assert_eq!(p.iso(), PathBuf::from("/Volumes/MS/ISO"));
        assert_eq!(p.seplugins(), PathBuf::from("/Volumes/MS/seplugins"));
    }
}
