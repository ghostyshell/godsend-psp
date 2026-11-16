//! Store download flow: pick the best link, resolve through debrid when
//! connected, download with aria2c, record the game in the local catalog.

use crate::config::Config;
use crate::device::PspPaths;
use crate::error::{Error, Result};
use crate::sources::GameHit;
use crate::transfer::JobContext;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Pick the preferred link: torrents/ddl the debrid providers support first.
fn pick_link(hit: &GameHit) -> Option<&crate::sources::GameLink> {
    hit.links.iter().max_by_key(|l| {
        let supported = l.supported_by.len();
        let torrent_bonus = if l.link_type != "ddl" { 2 } else { 1 };
        supported * 10 + torrent_bonus
    })
}

/// Run a store download inside a job: resolve -> aria2c -> catalog upsert.
pub fn download_game(hit: GameHit, config: Config, ctx: &JobContext) -> Result<PathBuf> {
    let link = pick_link(&hit)
        .ok_or_else(|| Error::NotFound(format!("no links for {}", hit.title)))?
        .clone();
    ctx.message(format!("resolving via {} ({})", link.host, link.link_type));
    let direct = crate::debrid::resolve(
        &link.url,
        &link.supported_by,
        &config.rd_api_key,
        &config.tb_api_key,
    )?;

    let dest_dir = crate::aria2::download_dir(&config);
    let name = file_name_for(&hit, &direct);
    ctx.message("downloading");
    let path = crate::aria2::download(&direct, &dest_dir, &name, &config, ctx)?;

    // Remember the game in the local catalog (PPSSPP-style db).
    let serial = hit.slug.to_uppercase();
    let _ = crate::catalog::upsert(vec![crate::catalog::CatalogGame {
        serial: String::new(),
        title: hit.title.clone(),
        region: String::new(),
        size_bytes: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
        cover: hit.cover.clone().unwrap_or_default(),
        animation: String::new(),
        source: hit.source.clone(),
        slug: hit.slug.clone(),
    }]);
    let _ = serial;
    ctx.message(format!("downloaded to {}", dest_dir.display()));
    Ok(path)
}

fn file_name_for(hit: &GameHit, direct_url: &str) -> String {
    let path = direct_url.split('?').next().unwrap_or(direct_url);
    let mut candidates: Vec<&str> = vec![path.rsplit('/').next().unwrap_or("")];
    // Query parameters can also carry the file name (debrid direct links).
    if let Some(query) = direct_url.split('?').nth(1) {
        for pair in query.split('&') {
            if let Some(value) = pair.split('=').nth(1) {
                candidates.push(value);
            }
        }
    }
    for candidate in candidates {
        if candidate.len() > 4 && candidate.contains('.') {
            return crate::media::fat_safe_name(candidate);
        }
    }
    let ext = if direct_url.starts_with("magnet:") || hit.link_type == "torrent" || hit.link_type == "magnet" {
        "torrent"
    } else {
        "zip"
    };
    format!("{}.{}", crate::media::fat_safe_name(&hit.title), ext)
}

/// Push a local download onto the PSP, unpacking and routing by content.
///
/// `.iso`/`.cso` go to `ISO/`, a `.pbp` is wrapped as
/// `PSP/GAME/<stem>/EBOOT.PBP`, and a `.zip` is extracted and routed by what
/// it holds: a `PSP/` tree merges over the stick, EBOOT.PBP folders install
/// under `PSP/GAME`, loose ISOs/CSOs go to `ISO/` and bare PBPs are wrapped.
pub fn push_local_to_psp(local_path: &Path, paths: &PspPaths, ctx: &JobContext) -> Result<PathBuf> {
    push_local(local_path, paths, Some(ctx))
}

/// Same routing with no job attached, for tests and one-off callers.
fn push_local(local_path: &Path, paths: &PspPaths, ctx: Option<&JobContext>) -> Result<PathBuf> {
    check_cancel(ctx)?;
    if !local_path.exists() {
        return Err(Error::NotFound(local_path.display().to_string()));
    }
    if local_path.is_dir() {
        // An EBOOT folder pushed straight from the library keeps its name.
        let name = file_name(local_path)?;
        let folder = crate::media::fat_safe_name(&name);
        let dest = paths.game().join(&folder);
        note(ctx, format!("copying {} to PSP/GAME", folder));
        crate::homebrew::merge_tree(local_path, &dest)?;
        return Ok(dest);
    }
    let ext = local_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "iso" | "cso" => push_iso(local_path, paths, ctx),
        "pbp" => push_pbp(local_path, paths, ctx),
        "zip" => push_zip(local_path, paths, ctx),
        "7z" | "rar" => Err(Error::Invalid(
            "only .zip is unpacked; extract .7z/.rar first and push the ISO".into(),
        )),
        "chd" => Err(Error::Invalid(
            "CHD is not bootable on the PSP; convert it to ISO or CSO first".into(),
        )),
        _ => Err(Error::Invalid(format!(
            "cannot push {}: supported types are .iso, .cso, .pbp and .zip",
            local_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| local_path.display().to_string())
        ))),
    }
}

fn file_name(path: &Path) -> Result<String> {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| Error::Invalid(format!("bad path {}", path.display())))
}

fn note(ctx: Option<&JobContext>, message: impl Into<String>) {
    if let Some(ctx) = ctx {
        ctx.message(message);
    }
}

fn progress(ctx: Option<&JobContext>, done: u64, total: u64) {
    if let Some(ctx) = ctx {
        ctx.progress(done, total);
    }
}

fn check_cancel(ctx: Option<&JobContext>) -> Result<()> {
    if let Some(ctx) = ctx {
        ctx.check_cancel()?;
    }
    Ok(())
}

/// Copy one file, reporting bytes when a job is attached.
fn copy_file(src: &Path, dest: &Path, ctx: Option<&JobContext>) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    match ctx {
        Some(ctx) => crate::transfer::copy_with_progress(src, dest, ctx),
        None => {
            fs::copy(src, dest)?;
            Ok(fs::metadata(dest).map(|m| m.len()).unwrap_or(0))
        }
    }
}

/// `.iso` / `.cso`: copy into `ISO/` under a FAT safe name.
fn push_iso(src: &Path, paths: &PspPaths, ctx: Option<&JobContext>) -> Result<PathBuf> {
    let name = crate::media::fat_safe_name(&file_name(src)?);
    let dest = paths.iso().join(&name);
    note(ctx, format!("copying {} to ISO", name));
    copy_file(src, &dest, ctx)?;
    Ok(dest)
}

/// `.pbp`: wrap as `PSP/GAME/<stem>/EBOOT.PBP` (what the XMB boots).
fn push_pbp(src: &Path, paths: &PspPaths, ctx: Option<&JobContext>) -> Result<PathBuf> {
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "homebrew".into());
    let folder = crate::media::fat_safe_name(&stem);
    let dir = paths.game().join(&folder);
    let dest = dir.join("EBOOT.PBP");
    note(ctx, format!("installing PSP/GAME/{}/EBOOT.PBP", folder));
    copy_file(src, &dest, ctx)?;
    Ok(dir)
}

/// Extract a zip into a private cache dir and route what it holds.
fn push_zip(src: &Path, paths: &PspPaths, ctx: Option<&JobContext>) -> Result<PathBuf> {
    let total = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
    note(ctx, "extracting archive");
    progress(ctx, 0, total);

    let temp = temp_extract_dir();
    fs::create_dir_all(&temp)?;
    let result = (|| -> Result<PathBuf> {
        crate::homebrew::extract_zip(src, &temp)?;
        progress(ctx, total, total);
        check_cancel(ctx)?;
        route_extracted(&temp, paths, ctx)
    })();
    // The staging tree never outlives the push, on success or failure.
    let _ = fs::remove_dir_all(&temp);
    result
}

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_extract_dir() -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    crate::config::cache_dir().join(format!(
        "push-{}-{}-{}",
        std::process::id(),
        stamp,
        seq
    ))
}

fn route_extracted(root: &Path, paths: &PspPaths, ctx: Option<&JobContext>) -> Result<PathBuf> {
    // 1. A full PSP tree merges over the stick (PSP/GAME, PSP/SAVEDATA, ...).
    if let Some(psp_src) = read_dir_named(root, "PSP") {
        for entry in fs::read_dir(&psp_src)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_junk_name(&name) {
                continue;
            }
            check_cancel(ctx)?;
            note(ctx, format!("installing PSP/{}", name));
            let dest = paths.psp().join(&name);
            if entry.path().is_dir() {
                crate::homebrew::merge_tree(&entry.path(), &dest)?;
            } else {
                fs::create_dir_all(paths.psp())?;
                fs::copy(entry.path(), &dest)?;
            }
        }
        return Ok(paths.psp());
    }

    // 2. One or more EBOOT.PBP folders become PSP/GAME/<folder>.
    let mut first: Option<PathBuf> = None;
    for dir in crate::homebrew::find_eboot_dirs(root, 4, 0) {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "homebrew".into());
        if is_junk_name(&name) {
            continue;
        }
        check_cancel(ctx)?;
        let folder = crate::media::fat_safe_name(&name);
        let dest = paths.game().join(&folder);
        note(ctx, format!("installing PSP/GAME/{}", folder));
        crate::homebrew::copy_tree(&dir, &dest)?;
        first.get_or_insert(dest);
    }
    if let Some(dest) = first {
        return Ok(dest);
    }

    // 3. Any ISO/CSO in the archive goes to ISO/ (Minerva per-game zips).
    let isos = files_with_ext(root, &["iso", "cso"]);
    if !isos.is_empty() {
        let mut first: Option<PathBuf> = None;
        for iso in isos {
            let dest = push_iso(&iso, paths, ctx)?;
            first.get_or_insert(dest);
        }
        if let Some(dest) = first {
            return Ok(dest);
        }
    }

    // 4. Bare PBPs get wrapped per archive member.
    let pbos = files_with_ext(root, &["pbp"]);
    if !pbos.is_empty() {
        let mut first: Option<PathBuf> = None;
        for pbp in pbos {
            let dest = push_pbp(&pbp, paths, ctx)?;
            first.get_or_insert(dest);
        }
        if let Some(dest) = first {
            return Ok(dest);
        }
    }

    Err(Error::Invalid(
        "archive has no PSP game: expected an ISO/CSO, EBOOT.PBP or a PSP folder tree".into(),
    ))
}

/// Direct child of `dir` named `name` (case-insensitive), when it is a dir.
fn read_dir_named(dir: &Path, name: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case(name) && entry.path().is_dir()
        {
            return Some(entry.path());
        }
    }
    None
}

fn files_with_ext(root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if is_junk_name(&name) {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let lower = name.to_lowercase();
                if exts.iter().any(|ext| lower.ends_with(&format!(".{}", ext))) {
                    out.push(path);
                }
            }
        }
    }
    out.sort();
    out
}

fn is_junk_name(name: &str) -> bool {
    name.starts_with('.') || name.eq_ignore_ascii_case("__macosx")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("godsend-push-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A temp stick root with PSP/GAME created, like a real memory stick.
    fn stick(tag: &str) -> (PathBuf, PspPaths) {
        let dir = temp_dir(tag);
        let paths = PspPaths::new(&dir);
        fs::create_dir_all(paths.game()).unwrap();
        (dir, paths)
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn zip_iso_lands_in_iso_folder() {
        let (dir, paths) = stick("zip-iso");
        let zip = dir.join("download.zip");
        write_zip(&zip, &[("Game.iso", b"iso-bytes")]);

        let dest = push_local(&zip, &paths, None).expect("push");
        assert_eq!(dest, paths.iso().join("Game.iso"));
        assert_eq!(fs::read(&dest).unwrap(), b"iso-bytes");
        // The zip is never copied onto the stick.
        assert!(!paths.game().join("download.zip").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_psp_tree_merges_at_stick_root() {
        let (dir, paths) = stick("zip-tree");
        let zip = dir.join("homebrew.zip");
        write_zip(
            &zip,
            &[
                ("PSP/GAME/Homebrew/EBOOT.PBP", b"eboot"),
                ("PSP/SAVEDATA/ARK_01234/ARK.BIN", b"ark"),
                ("__MACOSX/PSP/._EBOOT.PBP", b"junk"),
            ],
        );

        let dest = push_local(&zip, &paths, None).expect("push");
        assert_eq!(dest, paths.psp());
        assert_eq!(
            fs::read(paths.game().join("Homebrew/EBOOT.PBP")).unwrap(),
            b"eboot"
        );
        assert_eq!(
            fs::read(paths.savedata().join("ARK_01234/ARK.BIN")).unwrap(),
            b"ark"
        );
        assert!(!paths.psp().join("__MACOSX").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_eboot_folder_installs_under_game() {
        let (dir, paths) = stick("zip-eboot");
        let zip = dir.join("cool.zip");
        write_zip(
            &zip,
            &[
                ("Cool Game/EBOOT.PBP", b"eboot"),
                ("Cool Game/PARAM.SFO", b"sfo"),
                ("Cool Game/._EBOOT.PBP", b"junk"),
            ],
        );

        let dest = push_local(&zip, &paths, None).expect("push");
        assert_eq!(dest, paths.game().join("Cool Game"));
        assert_eq!(
            fs::read(paths.game().join("Cool Game/EBOOT.PBP")).unwrap(),
            b"eboot"
        );
        assert!(!paths.game().join("Cool Game/._EBOOT.PBP").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_bare_pbp_is_wrapped_as_eboot() {
        let (dir, paths) = stick("zip-pbp");
        let zip = dir.join("save.zip");
        write_zip(&zip, &[("save.pbp", b"pbp-bytes")]);

        let dest = push_local(&zip, &paths, None).expect("push");
        assert_eq!(dest, paths.game().join("save"));
        assert_eq!(
            fs::read(paths.game().join("save/EBOOT.PBP")).unwrap(),
            b"pbp-bytes"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_without_a_game_errors() {
        let (dir, paths) = stick("zip-empty");
        let zip = dir.join("docs.zip");
        write_zip(&zip, &[("readme.txt", b"hello")]);

        let err = push_local(&zip, &paths, None).expect_err("should fail");
        assert!(err.to_string().contains("no PSP game"), "got {err}");
        assert!(!paths.game().join("docs.zip").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn chd_explains_the_conversion() {
        let (dir, paths) = stick("chd");
        let chd = dir.join("game.chd");
        fs::write(&chd, b"chd").unwrap();

        let err = push_local(&chd, &paths, None).expect_err("should fail");
        assert!(err.to_string().contains("convert it to ISO or CSO"), "got {err}");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn direct_iso_lands_in_iso_folder() {
        let (dir, paths) = stick("direct-iso");
        let iso = dir.join("LocoRoco.ISO");
        fs::write(&iso, b"iso").unwrap();

        let dest = push_local(&iso, &paths, None).expect("push");
        assert_eq!(dest, paths.iso().join("LocoRoco.ISO"));
        assert_eq!(fs::read(&dest).unwrap(), b"iso");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_traversal_entry_stays_outside_the_stick() {
        let (dir, paths) = stick("zip-escape");
        let zip = dir.join("evil.zip");
        write_zip(
            &zip,
            &[("Game.iso", b"iso"), ("../escape.iso", b"traversal")],
        );

        push_local(&zip, &paths, None).expect("push");
        assert!(paths.iso().join("Game.iso").is_file());
        // Neither the stick nor a directory above the staging area got the file.
        assert!(!paths.root.join("escape.iso").exists());
        assert!(!crate::config::cache_dir().join("escape.iso").exists());
        assert!(!crate::config::cache_dir().parent().unwrap().join("escape.iso").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pick_prefers_debrid_torrents() {
        let hit = GameHit {
            id: "1".into(),
            slug: "t".into(),
            title: "T".into(),
            platform: "psp".into(),
            source: String::new(),
            host: "a".into(),
            url: "https://a/f".into(),
            link_type: "ddl".into(),
            supported_by: vec![],
            cover: None,
            rawg_cover: None,
            size: None,
            links: vec![
                crate::sources::GameLink {
                    host: "a".into(),
                    url: "https://a/f".into(),
                    link_type: "ddl".into(),
                    supported_by: vec!["torbox".into()],
                },
                crate::sources::GameLink {
                    host: "b".into(),
                    url: "magnet:?xt=1".into(),
                    link_type: "magnet".into(),
                    supported_by: vec!["torbox".into(), "realdebrid".into()],
                },
            ],
        };
        let picked = pick_link(&hit).expect("link");
        assert_eq!(picked.link_type, "magnet");
    }

    #[test]
    fn file_names() {
        let hit = GameHit {
            id: String::new(),
            slug: "locoroco-2".into(),
            title: "LocoRoco 2".into(),
            platform: "psp".into(),
            source: String::new(),
            host: String::new(),
            url: String::new(),
            link_type: "ddl".into(),
            supported_by: vec![],
            cover: None,
            rawg_cover: None,
            size: None,
            links: vec![],
        };
        assert_eq!(
            file_name_for(&hit, "https://host/get?file=locoroco2.zip"),
            "locoroco2.zip"
        );
        assert_eq!(file_name_for(&hit, "magnet:?xt=urn:btih:x"), "LocoRoco 2.torrent");
    }
}
