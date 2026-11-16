//! Theme Studio packs: wallpaper packs plus an app accent theme.
//!
//! CXMB `.ctf` and Sony `.ptf` are closed containers, so this module does not
//! pretend to author them. A pack is a small zip (or folder once installed)
//! holding `manifest.json` and an optional `wallpaper.png`; installed packs
//! live in `PSP/PICTURE/<slug>/` where the console photo viewer and the
//! desktop app can both use them. Existing `.ctf`/`.ptf` files stay a
//! separate install/import path in `themes`.

use crate::device::PspPaths;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Manifest written into a pack (and its `.godsend-theme.zip`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemePack {
    pub name: String,
    /// Accent colour as a hex string (`#rrggbb`).
    pub accent: String,
    /// Wallpaper file name inside the pack, when one was supplied.
    pub background: Option<String>,
    /// Creation timestamp, ISO 8601 UTC.
    pub created_at: String,
}

/// Folder inside `PSP` that holds installed packs.
fn picture_dir(paths: &PspPaths) -> PathBuf {
    paths.psp().join("PICTURE")
}

/// Lowercase, dash-separated folder name. Never empty, never escapes.
pub fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    let out = if trimmed.is_empty() { "theme" } else { trimmed };
    out.chars().take(48).collect::<String>().trim_end_matches('-').to_string()
}

/// Normalise an accent value, rejecting anything that is not a hex colour.
fn accent_value(accent: &str) -> Result<String> {
    let accent = accent.trim();
    let accent = if accent.is_empty() { "#6d8bff" } else { accent };
    let hex = accent.strip_prefix('#').unwrap_or(accent);
    let valid = matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(Error::Invalid(format!("accent is not a hex colour: {accent}")));
    }
    Ok(if accent.starts_with('#') { accent.to_string() } else { format!("#{accent}") })
}

/// Build a pack and write `<slug>.godsend-theme.zip` into `out_dir`.
pub fn create(
    name: &str,
    accent: &str,
    wallpaper: Option<&Path>,
    out_dir: &Path,
) -> Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("theme name is required".into()));
    }
    let slug = slug(name);
    let accent = accent_value(accent)?;
    if let Some(path) = wallpaper {
        if !path.is_file() {
            return Err(Error::NotFound(format!("wallpaper not found: {}", path.display())));
        }
    }
    let pack = ThemePack {
        name: name.to_string(),
        accent,
        background: wallpaper.map(|_| "wallpaper.png".to_string()),
        created_at: now_iso(),
    };

    fs::create_dir_all(out_dir)?;
    let dest = out_dir.join(format!("{slug}.godsend-theme.zip"));
    let file = fs::File::create(&dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::SimpleFileOptions =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(&pack)?)?;

    if let Some(path) = wallpaper {
        zip.start_file("wallpaper.png", options)?;
        let bytes = fs::read(path)?;
        zip.write_all(&bytes)?;
    }
    zip.finish()?;
    Ok(dest)
}

/// Install a pack zip into `PSP/PICTURE/<slug>/` and return its manifest.
pub fn install(paths: &PspPaths, pack: &Path) -> Result<ThemePack> {
    let file = fs::File::open(pack)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let manifest = read_manifest(&mut archive)?;
    let slug = slug(&manifest.name);
    let dest = picture_dir(paths).join(&slug);
    fs::create_dir_all(&dest)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name() else {
            continue; // path traversal guard
        };
        let file_name = match name.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };
        // Only the two files a pack actually owns; ignore everything else.
        if entry.is_dir() || (file_name != "manifest.json" && file_name != "wallpaper.png") {
            continue;
        }
        let target = dest.join(file_name);
        let mut out = fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
    }

    // Normalise the manifest on the stick so `list` always reads one shape.
    fs::write(dest.join("manifest.json"), serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest)
}

/// Read installed packs from `PSP/PICTURE/*/manifest.json`, skipping junk.
pub fn list(paths: &PspPaths) -> Result<Vec<ThemePack>> {
    let dir = picture_dir(paths);
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest = path.join("manifest.json");
        if !manifest.is_file() {
            continue;
        }
        match fs::read(&manifest).ok().and_then(|bytes| serde_json::from_slice::<ThemePack>(&bytes).ok()) {
            Some(pack) => out.push(pack),
            None => tracing::warn!("skipping junk theme pack manifest: {}", manifest.display()),
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.name.cmp(&b.name)));
    Ok(out)
}

/// Remove an installed pack folder. `slug` is resolved inside PICTURE.
pub fn delete(paths: &PspPaths, folder: &str) -> Result<()> {
    let dir = picture_dir(paths);
    let safe = slug(folder);
    let dest = dir.join(&safe);
    if !dest.starts_with(&dir) || safe.is_empty() {
        return Err(Error::Invalid("theme pack path escapes PICTURE dir".into()));
    }
    if !dest.is_dir() {
        return Err(Error::NotFound(safe));
    }
    fs::remove_dir_all(dest)?;
    Ok(())
}

fn read_manifest(archive: &mut zip::ZipArchive<fs::File>) -> Result<ThemePack> {
    let mut found = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if name.file_name().and_then(|n| n.to_str()) == Some("manifest.json") {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut bytes)?;
            found = Some(bytes);
            break;
        }
    }
    let bytes = found.ok_or_else(|| Error::Invalid("theme pack has no manifest.json".into()))?;
    let pack: ThemePack = serde_json::from_slice(&bytes)?;
    if pack.name.trim().is_empty() {
        return Err(Error::Invalid("theme pack manifest has no name".into()));
    }
    Ok(pack)
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    iso_from_unix(secs)
}

fn iso_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT: AtomicU32 = AtomicU32::new(0);

    fn temp_root(tag: &str) -> PathBuf {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "godsend-tpack-{tag}-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn slug_sanitizes() {
        assert_eq!(slug("Midnight Brass!"), "midnight-brass");
        assert_eq!(slug("  ///  "), "theme");
        assert_eq!(slug("../../etc/passwd"), "etc-passwd");
    }

    #[test]
    fn create_install_list_round_trip() {
        let root = temp_root("round");
        let out = root.join("out");
        let wallpaper = root.join("wall.png");
        fs::write(&wallpaper, b"\x89PNG\r\n\x1a\n-fake").unwrap();

        let zip = create("Midnight Brass", "#f2c078", Some(&wallpaper), &out).unwrap();
        assert_eq!(zip.file_name().unwrap().to_string_lossy(), "midnight-brass.godsend-theme.zip");
        assert!(zip.is_file());

        let paths = PspPaths::new(&root);
        let pack = install(&paths, &zip).unwrap();
        assert_eq!(pack.name, "Midnight Brass");
        assert_eq!(pack.accent, "#f2c078");
        assert_eq!(pack.background.as_deref(), Some("wallpaper.png"));
        assert!(root.join("PSP/PICTURE/midnight-brass/wallpaper.png").is_file());

        let packs = list(&paths).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0], pack);

        delete(&paths, "midnight-brass").unwrap();
        assert!(list(&paths).unwrap().is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_name_is_rejected() {
        let root = temp_root("empty");
        let err = create("   ", "#fff", None, &root).unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn junk_manifest_is_skipped() {
        let root = temp_root("junk");
        let paths = PspPaths::new(&root);
        let pic = root.join("PSP/PICTURE");
        fs::create_dir_all(pic.join("junk")).unwrap();
        fs::write(pic.join("junk/manifest.json"), b"not json at all").unwrap();
        fs::create_dir_all(pic.join("empty")).unwrap();
        fs::write(
            pic.join("empty/manifest.json"),
            r##"{"name":"Kept","accent":"#123456","background":null,"created_at":"2026-01-02T03:04:05Z"}"##,
        )
        .unwrap();

        let packs = list(&paths).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "Kept");
        fs::remove_dir_all(&root).ok();
    }
}
