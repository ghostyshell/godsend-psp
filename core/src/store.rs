//! Store download flow: pick the best link, resolve through debrid when
//! connected, download with aria2c, record the game in the local catalog.

use crate::config::Config;
use crate::device::PspPaths;
use crate::error::{Error, Result};
use crate::sources::GameHit;
use crate::transfer::JobContext;
use std::path::PathBuf;

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

/// Copy a downloaded store game onto the PSP (helper used by the Downloads
/// page push button): chooses ISO folder for iso/cso files, GAME for pbp/dirs.
pub fn push_local_to_psp(local_path: &std::path::Path, paths: &PspPaths, ctx: &JobContext) -> Result<PathBuf> {
    let lower = local_path.to_string_lossy().to_lowercase();
    let dest_kind = if lower.ends_with(".iso") || lower.ends_with(".cso") {
        "iso"
    } else {
        "game"
    };
    crate::fm::copy_to_psp(local_path, paths, dest_kind, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

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
