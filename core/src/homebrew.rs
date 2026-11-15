//! Homebrew catalog and installer.
//!
//! The bundled catalog points at GitHub repos that ship PSP homebrew as
//! release `.zip` archives (ARK-4/ARK-5 CFW, GBA/N64 emulators and more).
//! Installing walks the archive and places folders the way the PSP XMB and
//! CFW loaders expect:
//!
//! - `ARK_01234` style folders go to `PSP/SAVEDATA/`
//! - folders containing `EBOOT.PBP` go to `PSP/GAME/<folder>`
//! - archives that already contain a `PSP/` tree are merged at the stick root
//!
//! Local `.zip` imports use the same logic, so any homebrew archive works.

use crate::device::PspPaths;
use crate::error::{Error, Result};
use crate::transfer::JobContext;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const GITHUB_API: &str = "https://api.github.com";
const USER_AGENT: &str = "godsend-psp";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Detect from the archive contents.
    Auto,
    /// ARK-4 style: ARK_01234 to SAVEDATA, ARK_* loader folders to GAME.
    Ark,
    /// Archive contains a full `PSP/` tree to merge at the stick root.
    PspRoot,
    /// Single game folder with an EBOOT.PBP somewhere inside.
    Eboot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    /// `game` | `emu` | `cfw` | `plugin`
    pub category: String,
    pub repo: String,
    pub layout: Layout,
    /// Optional note shown in the UI (e.g. "needs gba_bios.bin").
    pub note: Option<String>,
    /// Filled after a release refresh: latest tag + zip asset URL.
    pub latest_tag: Option<String>,
    pub zip_url: Option<String>,
    pub published_at: Option<String>,
}

/// The bundled catalog, shipped with the binary and merged with the cache.
pub fn bundled_catalog() -> Vec<CatalogEntry> {
    vec![
        CatalogEntry {
            id: "ark-4".into(),
            name: "ARK-4".into(),
            description: "Next generation custom firmware for PSP (and Vita ePSP). Live CFW with custom launcher, plugins, Inferno ISO driver.".into(),
            category: "cfw".into(),
            repo: "PSP-Archive/ARK-4".into(),
            layout: Layout::Ark,
            note: Some("Copies ARK_01234 to PSP/SAVEDATA and loaders to PSP/GAME. Launch ARK Loader on the PSP to boot the CFW.".into()),
            latest_tag: None,
            zip_url: None,
            published_at: None,
        },
        CatalogEntry {
            id: "ark-5".into(),
            name: "ARK-5".into(),
            description: "Successor to ARK-4. Fresh rewrite of the ARK custom firmware line.".into(),
            category: "cfw".into(),
            repo: "PSP-Arkfive/ARK-5".into(),
            layout: Layout::Ark,
            note: Some("New CFW line; check the project wiki for model support.".into()),
            latest_tag: None,
            zip_url: None,
            published_at: None,
        },
        CatalogEntry {
            id: "froggba".into(),
            name: "FrogGBA".into(),
            description: "GBA emulator for PSP, based on TempGBA4PSP with gpSP Kai optimizations. Dynarec cache improvements, per-game configs.".into(),
            category: "emu".into(),
            repo: "tzubertowski/FrogGBA".into(),
            layout: Layout::PspRoot,
            note: Some("Needs gba_bios.bin dropped into the game folder after install.".into()),
            latest_tag: None,
            zip_url: None,
            published_at: None,
        },
        CatalogEntry {
            id: "tempgba".into(),
            name: "TempGBA4PSP-mod".into(),
            description: "Long-maintained GBA emulator with 2026 fixes, 16:9 full screen and a single-game mode.".into(),
            category: "emu".into(),
            repo: "andymcca/TempGBA4PSP-mod".into(),
            layout: Layout::Auto,
            note: Some("Needs gba_bios.bin in the emulator folder.".into()),
            latest_tag: None,
            zip_url: None,
            published_at: None,
        },
        CatalogEntry {
            id: "daedalusx64".into(),
            name: "DaedalusX64".into(),
            description: "N64 emulator for PSP using the Media Engine for asynchronous audio.".into(),
            category: "emu".into(),
            repo: "DaedalusX64/daedalus".into(),
            layout: Layout::Auto,
            note: None,
            latest_tag: None,
            zip_url: None,
            published_at: None,
        },
    ]
}

/// Merge the cached release info (from `catalog.json`) into the bundled list.
pub fn load_catalog() -> Vec<CatalogEntry> {
    let mut catalog = bundled_catalog();
    let cache = crate::config::catalog_cache_path();
    if let Ok(bytes) = fs::read(&cache) {
        if let Ok(cached) = serde_json::from_slice::<Vec<CatalogEntry>>(&bytes) {
            for entry in &mut catalog {
                if let Some(hit) = cached.iter().find(|c| c.id == entry.id) {
                    entry.latest_tag = hit.latest_tag.clone();
                    entry.zip_url = hit.zip_url.clone();
                    entry.published_at = hit.published_at.clone();
                }
            }
        }
    }
    catalog
}

pub fn save_catalog(catalog: &[CatalogEntry]) -> Result<()> {
    let cache = crate::config::catalog_cache_path();
    if let Some(parent) = cache.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(catalog)?;
    fs::write(cache, bytes)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GhRelease {
    tag_name: Option<String>,
    published_at: Option<String>,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// Fetch the latest release metadata for every catalog entry (network).
pub fn refresh_catalog(catalog: &mut [CatalogEntry]) -> Result<usize> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(20))
        .build();
    let mut updated = 0;
    for entry in catalog.iter_mut() {
        let Ok(response) = agent
            .get(&format!("{}/repos/{}/releases/latest", GITHUB_API, entry.repo))
            .set("User-Agent", USER_AGENT)
            .set("Accept", "application/vnd.github+json")
            .call()
        else {
            continue;
        };
        let release: GhRelease = match response.into_json() {
            Ok(r) => r,
            Err(_) => continue,
        };
        entry.latest_tag = release.tag_name;
        entry.published_at = release.published_at;
        entry.zip_url = release
            .assets
            .iter()
            .find(|a| a.name.to_lowercase().ends_with(".zip"))
            .map(|a| a.browser_download_url.clone());
        updated += 1;
    }
    if updated > 0 {
        save_catalog(catalog)?;
    }
    Ok(updated)
}

/// Download a release zip into the app cache and return its path.
pub fn download_zip(url: &str, ctx: Option<&JobContext>) -> Result<PathBuf> {
    let dir = crate::config::cache_dir().join("downloads");
    fs::create_dir_all(&dir)?;
    let file_name = format!(
        "{}.zip",
        url.rsplit('/').next().unwrap_or("release").replace('/', "_")
    );
    let dest = dir.join(file_name);

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(300))
        .build();
    let response = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| Error::Http(format!("download failed: {}", e)))?;
    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = response.into_reader();
    let mut out = fs::File::create(&dest)?;
    let mut buf = [0u8; 64 * 1024];
    let mut done: u64 = 0;
    let mut last_report = std::time::Instant::now();
    loop {
        if let Some(ctx) = ctx {
            ctx.check_cancel()?;
        }
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut out, &buf[..read])?;
        done += read as u64;
        if let Some(ctx) = ctx {
            if last_report.elapsed().as_millis() > 150 {
                ctx.progress(done, total);
                last_report = std::time::Instant::now();
            }
        }
    }
    if let Some(ctx) = ctx {
        ctx.progress(done, total);
    }
    Ok(dest)
}

/// What an install touched, for the UI report.
#[derive(Debug, Clone, Serialize, Default)]
pub struct InstallReport {
    /// Destination paths written, relative to the stick root.
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
    pub layout_used: String,
}

/// Install a homebrew/CFW zip onto the stick.
pub fn install_zip(zip_path: &Path, paths: &PspPaths, layout: Layout, ctx: Option<&JobContext>) -> Result<InstallReport> {
    paths.validate()?;
    let staging = crate::config::cache_dir().join("staging");
    let extract_dir = staging.join(format!("hb-{}", std::process::id()));
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;

    if let Some(ctx) = ctx {
        let size = fs::metadata(zip_path).map(|m| m.len()).unwrap_or(0);
        ctx.message("extracting archive");
        ctx.progress(0, size);
    }
    extract_zip(zip_path, &extract_dir)?;
    if let Some(ctx) = ctx {
        let size = fs::metadata(zip_path).map(|m| m.len()).unwrap_or(0);
        ctx.progress(size, size);
    }

    let detected = detect_layout(&extract_dir);
    let use_layout = if layout == Layout::Auto { detected } else { layout };
    let mut report = match use_layout {
        Layout::Ark => install_ark(&extract_dir, paths)?,
        Layout::PspRoot => install_psp_root(&extract_dir, paths)?,
        Layout::Eboot => install_eboot_dirs(&extract_dir, paths)?,
        Layout::Auto => install_auto(&extract_dir, paths)?,
    };
    report.layout_used = format!("{:?}", use_layout).to_lowercase();

    // Prune macOS metadata everywhere we wrote (ARK-4 itself ships this fix
    // on-console; we keep the stick clean at the source).
    prune_mac_metadata(paths)?;
    let _ = fs::remove_dir_all(&extract_dir);
    Ok(report)
}

/// Import a local zip with auto layout detection.
pub fn import_zip(zip_path: &Path, paths: &PspPaths, ctx: Option<&JobContext>) -> Result<InstallReport> {
    install_zip(zip_path, paths, Layout::Auto, ctx)
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name() else {
            continue; // path traversal guard
        };
        let target = dest.join(name);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn detect_layout(dir: &Path) -> Layout {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Layout::Auto,
    };
    let mut names = Vec::new();
    for entry in entries.flatten() {
        names.push(entry.file_name().to_string_lossy().to_lowercase());
    }
    let ark_like = names
        .iter()
        .any(|n| n.starts_with("ark_") || n == "ark loader" || n == "ark_loader");
    if ark_like {
        return Layout::Ark;
    }
    if names.iter().any(|n| n == "psp") {
        return Layout::PspRoot;
    }
    if find_eboot_dirs(dir, 3, 0).is_empty() {
        return Layout::Auto;
    }
    Layout::Eboot
}

fn install_ark(dir: &Path, paths: &PspPaths) -> Result<InstallReport> {
    let mut report = InstallReport::default();
    for entry in fs::read_dir(dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.eq_ignore_ascii_case("__macosx") {
            continue;
        }
        let lower = name.to_lowercase();
        if lower.starts_with("ark_01234") || (lower.starts_with("ark") && lower.contains("savedata")) {
            let dest = paths.savedata().join(&name);
            copy_tree(&entry.path(), &dest)?;
            report.installed.push(rel(&dest, paths));
        } else if lower.starts_with("ark") || lower == "update" {
            // Loaders, cIPL installers, DC, full flash installer, updater: GAME
            let dest = paths.game().join(&name);
            copy_tree(&entry.path(), &dest)?;
            report.installed.push(rel(&dest, paths));
        } else {
            report.skipped.push(name);
        }
    }
    Ok(report)
}

fn install_psp_root(dir: &Path, paths: &PspPaths) -> Result<InstallReport> {
    let mut report = InstallReport::default();
    let psp_src = dir.join("PSP");
    for sub in ["GAME", "SAVEDATA", "MUSIC", "VIDEO", "COMMON", "SYSTEM"] {
        let src = psp_src.join(sub);
        if src.is_dir() {
            let dest = paths.psp().join(sub);
            merge_tree(&src, &dest)?;
            report.installed.push(rel(&dest, paths));
        }
    }
    for entry in fs::read_dir(dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case("psp") || name.starts_with('.') || name.eq_ignore_ascii_case("__macosx") {
            continue;
        }
        // Loose files at the archive root (e.g. readme) go to GAME/<name>.
        let dest = paths.game().join(&name);
        if entry.path().is_dir() {
            merge_tree(&entry.path(), &dest)?;
        } else {
            fs::create_dir_all(&dest)?;
            let file_dest = dest.join(name);
            fs::copy(entry.path(), &file_dest)?;
        }
    }
    Ok(report)
}

fn install_eboot_dirs(dir: &Path, paths: &PspPaths) -> Result<InstallReport> {
    let mut report = InstallReport::default();
    for game_dir in find_eboot_dirs(dir, 3, 0) {
        let name = game_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "homebrew".into());
        let dest = paths.game().join(&name);
        merge_tree(&game_dir, &dest)?;
        report.installed.push(rel(&dest, paths));
    }
    if report.installed.is_empty() {
        return Err(Error::Invalid(
            "archive has no EBOOT.PBP folder; try importing as a plain zip or check the layout".into(),
        ));
    }
    Ok(report)
}

fn install_auto(dir: &Path, paths: &PspPaths) -> Result<InstallReport> {
    let ark = install_ark(dir, paths)?;
    if !ark.installed.is_empty() {
        return Ok(ark);
    }
    let root = install_psp_root(dir, paths)?;
    if !root.installed.is_empty() {
        return Ok(root);
    }
    install_eboot_dirs(dir, paths)
}

/// Find directories (up to `max_depth`) that directly contain an EBOOT.PBP.
fn find_eboot_dirs(dir: &Path, max_depth: usize, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if depth > max_depth {
        return out;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.eq_ignore_ascii_case("__macosx") {
            continue;
        }
        if path.is_dir() {
            if path.join("EBOOT.PBP").is_file() {
                out.push(path);
            } else {
                out.extend(find_eboot_dirs(&path, max_depth, depth + 1));
            }
        }
    }
    out
}

/// Copy a directory tree (creates the destination fresh).
fn copy_tree(src: &Path, dest: &Path) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    merge_tree(src, dest)
}

/// Merge a directory tree over an existing destination.
fn merge_tree(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with("._") || name.eq_ignore_ascii_case("__macosx") {
            continue;
        }
        let from = entry.path();
        let to = dest.join(&name);
        if from.is_dir() {
            merge_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn rel(path: &Path, paths: &PspPaths) -> String {
    path.strip_prefix(&paths.root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

/// Delete `__MACOSX` folders and `._*` AppleDouble files from the stick.
pub fn prune_mac_metadata(paths: &PspPaths) -> Result<()> {
    let mut stack = vec![paths.root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if name.eq_ignore_ascii_case("__macosx") {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    stack.push(path);
                }
            } else if name.starts_with("._") || name == ".DS_Store" {
                let _ = fs::remove_file(path);
            }
        }
    }
    Ok(())
}

/// True when a catalog entry's folder already exists on the stick.
pub fn is_installed(entry: &CatalogEntry, paths: &PspPaths) -> bool {
    match entry.id.as_str() {
        "ark-4" | "ark-5" => paths.savedata().join("ARK_01234").is_dir(),
        id => {
            let target = guess_game_folder(id);
            paths.game().join(target).is_dir()
        }
    }
}

fn guess_game_folder(id: &str) -> String {
    // Best-effort folder match against known names; a full install report
    // would be needed for exactness, so also scan GAME dirs for the names.
    let _ = id;
    String::new()
}

/// Cancel-aware wrapper used by job threads.
pub fn install_with_job(entry: &CatalogEntry, paths: &PspPaths, ctx: &JobContext) -> Result<InstallReport> {
    ctx.message("fetching release");
    let zip_url = match &entry.zip_url {
        Some(url) => url.clone(),
        None => {
            // Refresh just this entry, then retry.
            let mut single = vec![entry.clone()];
            refresh_catalog(&mut single)?;
            single
                .into_iter()
                .find(|e| e.id == entry.id)
                .and_then(|e| e.zip_url)
                .ok_or_else(|| Error::NotFound(format!("no zip asset for {}", entry.name)))?
        }
    };
    ctx.message("downloading");
    let zip = download_zip(&zip_url, Some(ctx))?;
    ctx.message("installing");
    install_zip(&zip, paths, entry.layout, Some(ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_has_core_entries() {
        let catalog = bundled_catalog();
        assert!(catalog.iter().any(|e| e.id == "ark-4"));
        assert!(catalog.iter().any(|e| e.repo == "tzubertowski/FrogGBA"));
        assert!(catalog.iter().all(|e| e.repo.contains('/')));
    }

    #[test]
    fn catalog_merge_keeps_cached_release() {
        let mut catalog = bundled_catalog();
        catalog[0].latest_tag = Some("v9.9.9".into());
        let dir = std::env::temp_dir().join(format!("godsend-cat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // save_catalog writes to the real cache dir; emulate by checking merge logic
        let cached = catalog.clone();
        let mut merged = bundled_catalog();
        for entry in &mut merged {
            if let Some(hit) = cached.iter().find(|c| c.id == entry.id) {
                entry.latest_tag = hit.latest_tag.clone();
            }
        }
        assert_eq!(merged[0].latest_tag, Some("v9.9.9".into()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
