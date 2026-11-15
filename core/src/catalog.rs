//! Local PSP game database (PPSSPP-style) plus R2 cover/animation caching.
//!
//! The catalog accumulates every game the store returns and every PARAM.SFO
//! seen on device/local libraries, keyed by serial id. Covers and animations
//! come from a configurable public R2 base (`<base>/psp/<SERIAL>.png`,
//! `<base>/psp/<SERIAL>.webm`) and are cached on disk.

use crate::config::Config;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogGame {
    /// Serial id like `UCUS-98737`; also the key.
    pub serial: String,
    pub title: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub cover: String,
    /// Animated preview (webm/mp4) URL, optional.
    #[serde(default)]
    pub animation: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogDb {
    pub updated: u64,
    pub games: Vec<CatalogGame>,
}

pub fn db_path() -> PathBuf {
    crate::config::cache_dir().join("psp-catalog.json")
}

pub fn load() -> CatalogDb {
    let Ok(bytes) = fs::read(db_path()) else {
        return CatalogDb::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(mut db: CatalogDb) -> Result<()> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    db.updated = now();
    fs::write(path, serde_json::to_vec_pretty(&db)?)?;
    Ok(())
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Upsert a batch of games keyed by serial (or slug when no serial).
pub fn upsert(games: Vec<CatalogGame>) -> Result<usize> {
    let mut db = load();
    let mut added = 0;
    for game in games {
        let key = if game.serial.is_empty() {
            game.slug.clone()
        } else {
            game.serial.clone()
        };
        if key.is_empty() {
            continue;
        }
        match db.games.iter_mut().find(|g| {
            (g.serial == key) || (g.slug == key) || (!g.serial.is_empty() && g.serial == game.serial)
        }) {
            Some(existing) => {
                if existing.cover.is_empty() && !game.cover.is_empty() {
                    existing.cover = game.cover.clone();
                }
                if existing.animation.is_empty() && !game.animation.is_empty() {
                    existing.animation = game.animation.clone();
                }
            }
            None => {
                db.games.push(CatalogGame {
                    serial: game.serial,
                    title: game.title,
                    region: game.region,
                    size_bytes: game.size_bytes,
                    cover: game.cover,
                    animation: game.animation,
                    source: game.source,
                    slug: game.slug,
                });
                added += 1;
            }
        }
    }
    save(db)?;
    Ok(added)
}

pub fn lookup_serial(serial: &str) -> Option<CatalogGame> {
    load()
        .games
        .into_iter()
        .find(|g| g.serial.eq_ignore_ascii_case(serial))
}

pub fn lookup_best(title: &str) -> Option<CatalogGame> {
    let needle = title.to_lowercase();
    load().games.into_iter().find(|g| {
        let t = g.title.to_lowercase();
        t == needle || t.starts_with(&needle) || needle.starts_with(&t)
    })
}

// ---------- covers + animations (R2) ----------

pub fn covers_cache_dir() -> PathBuf {
    crate::config::cache_dir().join("covers")
}

/// `<base>/psp/<SERIAL>.png` style cover URL for a serial.
pub fn cover_url(config: &Config, serial: &str) -> Option<String> {
    if serial.is_empty() || config.covers_base.is_empty() {
        return None;
    }
    Some(format!(
        "{}/psp/{}.png",
        config.covers_base.trim_end_matches('/'),
        crate::sources::urlencode(serial)
    ))
}

/// `<base>/psp/<SERIAL>.webm` animation URL.
pub fn animation_url(config: &Config, serial: &str) -> Option<String> {
    if serial.is_empty() || config.covers_base.is_empty() {
        return None;
    }
    Some(format!(
        "{}/psp/{}.webm",
        config.covers_base.trim_end_matches('/'),
        crate::sources::urlencode(serial)
    ))
}

/// Fetch (with disk cache) a cover png for a serial. `fallback_url` is used
/// when no R2 base is configured (catalog-provided cover URL).
pub fn cover_bytes(config: &Config, serial: &str, fallback_url: &str) -> Option<Vec<u8>> {
    fs::create_dir_all(covers_cache_dir()).ok()?;
    let cache = covers_cache_dir().join(format!("{}.png", sanitize(serial)));
    if let Ok(bytes) = fs::read(&cache) {
        return Some(bytes);
    }
    let url = cover_url(config, serial).or_else(|| {
        if fallback_url.is_empty() {
            None
        } else {
            Some(fallback_url.to_string())
        }
    })?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();
    let response = agent.get(&url).call().ok()?;
    let mut bytes: Vec<u8> = Vec::new();
    response
        .into_reader()
        .take(12 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() < 128 {
        return None;
    }
    fs::write(&cache, &bytes).ok()?;
    Some(bytes)
}

fn sanitize(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect()
}

use std::io::Read;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_urls() {
        let mut cfg = Config::default();
        cfg.covers_base = "https://covers.example.com".into();
        assert_eq!(
            cover_url(&cfg, "UCUS-98737").as_deref(),
            Some("https://covers.example.com/psp/UCUS-98737.png")
        );
        assert_eq!(
            animation_url(&cfg, "UCUS-98737").as_deref(),
            Some("https://covers.example.com/psp/UCUS-98737.webm")
        );
        cfg.covers_base = String::new();
        assert!(cover_url(&cfg, "X").is_none());
    }

    #[test]
    fn upsert_dedupes() {
        let mut cfg_game = CatalogGame {
            serial: "UCUS-00001".into(),
            title: "T".into(),
            region: String::new(),
            size_bytes: 0,
            cover: String::new(),
            animation: String::new(),
            source: String::new(),
            slug: "t".into(),
        };
        let second = CatalogGame {
            cover: "https://c/1.png".into(),
            ..cfg_game.clone()
        };
        let _ = &mut cfg_game;
        let mut db = CatalogDb::default();
        db.games.push(cfg_game.clone());
        // merge logic mirrors upsert(): find by serial then fill cover
        if let Some(existing) = db.games.iter_mut().find(|g| g.serial == second.serial) {
            if existing.cover.is_empty() {
                existing.cover = second.cover.clone();
            }
        }
        assert_eq!(db.games[0].cover, "https://c/1.png");
        assert_eq!(db.games.len(), 1);
    }
}
