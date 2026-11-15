//! Store sources: the game-search debrid index and Internet Archive
//! (r/Roms megathread style PSP sets).

use crate::error::Result;
use serde::{Deserialize, Serialize};

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(25))
        .build()
}

// ---------- game-search (debrid.link style index) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLink {
    pub host: String,
    pub url: String,
    #[serde(rename = "linkType")]
    pub link_type: String, // ddl | magnet | torrent
    #[serde(rename = "supportedBy", default)]
    pub supported_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameHit {
    pub id: String,
    #[serde(rename = "gameSlug", alias = "slug")]
    pub slug: String,
    #[serde(default)]
    pub title: String,
    #[serde(default = "default_platform")]
    pub platform: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "linkType", default)]
    pub link_type: String,
    #[serde(rename = "supportedBy", default)]
    pub supported_by: Vec<String>,
    #[serde(default)]
    pub cover: Option<String>,
    /// RAWG box art, as returned by the game-search detail rows.
    #[serde(rename = "rawgCover", default)]
    pub rawg_cover: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    /// Filled by the detail fetch (game links live there).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<GameLink>,
}

fn default_platform() -> String {
    "psp".into()
}

/// Browse or search the game-search index for PSP games.
pub fn gamesearch(base: &str, query: &str, limit: u32) -> Result<Vec<GameHit>> {
    let base = base.trim_end_matches('/');
    let mut url = format!("{base}/games?platform=psp&limit={limit}");
    if !query.is_empty() {
        url.push_str(&format!("&q={}", urlencode(query)));
    }
    let hits: serde_json::Value = agent().get(&url).call()?.into_json()?;
    let list = hits
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for v in list {
        // The index may return bare game rows or link rows; accept both.
        let mut hit: GameHit = serde_json::from_value(v)?;
        if hit.title.is_empty() {
            hit.title = hit.slug.replace('-', " ");
        }
        if hit.links.is_empty() && !hit.url.is_empty() {
            hit.links.push(GameLink {
                host: hit.host.clone(),
                url: hit.url.clone(),
                link_type: hit.link_type.clone(),
                supported_by: hit.supported_by.clone(),
            });
        }
        out.push(hit);
    }
    Ok(out)
}

/// Fetch one game's detail (carries the full link list).
pub fn gamesearch_detail(base: &str, slug: &str) -> Result<GameHit> {
    let base = base.trim_end_matches('/');
    let url = format!("{base}/games/{}", urlencode(slug));
    let v: serde_json::Value = agent().get(&url).call()?.into_json()?;
    // Either the game object directly or { game: {...}, links: [...] }.
    let game = v.get("game").cloned().unwrap_or_else(|| v.clone());
    let mut hit: GameHit = serde_json::from_value(game)?;
    if hit.title.is_empty() {
        hit.title = hit.slug.replace('-', " ");
    }
    if let Some(links) = v.get("game_links").or_else(|| v.get("links")) {
        hit.links = serde_json::from_value(links.clone())?;
    } else if !hit.url.is_empty() {
        hit.links.push(GameLink {
            host: hit.host.clone(),
            url: hit.url.clone(),
            link_type: hit.link_type.clone(),
            supported_by: hit.supported_by.clone(),
        });
    }
    Ok(hit)
}

/// Hosts known to the debrid providers (informational).
pub fn debrid_hosts(base: &str) -> Result<serde_json::Value> {
    let base = base.trim_end_matches('/');
    let v: serde_json::Value = agent().get(&format!("{base}/debrid/hosts")).call()?.into_json()?;
    Ok(v)
}

// ---------- Internet Archive (roms megathread sets) ----------

#[derive(Debug, Clone, Serialize)]
pub struct IaItem {
    pub id: String,
    pub title: String,
    pub files_count: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IaFile {
    pub name: String,
    pub size_bytes: u64,
    /// Direct download URL on the IA download host.
    pub url: String,
}

/// Search IA items matching a query (megathread-style PSP sets).
pub fn ia_search(query: &str, rows: u32) -> Result<Vec<IaItem>> {
    // Megathread sets are PSP-specific; keep the query anchored so IA does
    // not return unrelated software items.
    let terms = if query.is_empty() {
        "PSP".to_string()
    } else {
        format!("({query})")
    };
    let q = format!(
        "{terms} AND mediatype:(software) AND (title:(psp) OR identifier:(psp) OR title:(\"playstation portable\") OR subject:(psp))"
    );
    let url = format!(
        "https://archive.org/advancedsearch.php?q={}&fl%5B%5D=identifier&fl%5B%5D=title&fl%5B%5D=item_size&fl%5B%5D=files_count&rows={}&page=1&output=json",
        urlencode(&q),
        rows.max(1).min(100)
    );
    let v: serde_json::Value = agent().get(&url).call()?.into_json()?;
    let docs = v
        .pointer("/response/docs")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for doc in docs {
        let id = doc.get("identifier").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        // Post-filter: identifiers and titles must look PSP-related.
        let title = doc
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or(&id)
            .to_string();
        let probe = format!("{} {}", id.to_lowercase(), title.to_lowercase());
        if !(probe.contains("psp") || probe.contains("playstation portable")) {
            continue;
        }
        out.push(IaItem {
            title,
            files_count: doc.get("files_count").and_then(|f| f.as_u64()).unwrap_or(0),
            size_bytes: doc
                .get("item_size")
                .and_then(|s| s.as_u64())
                .or_else(|| doc.get("item_size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok()))
                .unwrap_or(0),
            id,
        });
    }
    Ok(out)
}

/// Megathread-style default queries surfaced in the Store.
pub fn ia_seed_queries() -> Vec<&'static str> {
    // Drawn from the r/Roms megathread PSP rows.
    vec![
        "PSP ISO",
        "PSP ROM set",
        "PlayStation Portable",
        "PSP Redump",
        "PSP No-Intro",
    ]
}

// ---------- Minerva Archive (Myrient PSP sets) ----------

const MINERVA_ASSETS: &str = "https://minerva-archive.org/assets/Minerva_Myrient_v0.3";

#[derive(Debug, Clone, Serialize)]
pub struct MinervaSet {
    /// Torrent file name, used as the download id.
    pub id: String,
    pub title: String,
    pub url: String,
}

/// Probe the known PSP torrent names and return the ones that exist.
pub fn minerva_sets() -> Result<Vec<MinervaSet>> {
    let candidates = [
        "Minerva_Myrient - Redump - Sony - PlayStation Portable.torrent",
        "Minerva_Myrient - No-Intro - Sony - PlayStation Portable.torrent",
        "Minerva_Myrient - Redump - Sony - PlayStation Portable (PSP).torrent",
        "Minerva_Myrient - No-Intro - Sony - PlayStation Portable (PSN).torrent",
    ];
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();
    let mut out = Vec::new();
    for name in candidates {
        let url = format!("{MINERVA_ASSETS}/{}", urlencode(name));
        let ok = agent
            .head(&url)
            .call()
            .map(|r| r.status() == 200)
            .unwrap_or(false);
        if ok {
            out.push(MinervaSet {
                id: name.to_string(),
                title: name
                    .trim_start_matches("Minerva_Myrient - ")
                    .trim_end_matches(".torrent")
                    .to_string(),
                url,
            });
        }
    }
    Ok(out)
}

pub fn minerva_download(id: &str) -> Option<String> {
    if id.contains("..") || id.contains('/') {
        return None;
    }
    Some(format!("{MINERVA_ASSETS}/{}", urlencode(id)))
}

/// List the downloadable game files inside an IA item.
pub fn ia_item_files(id: &str) -> Result<Vec<IaFile>> {
    let url = format!("https://archive.org/metadata/{}", urlencode(id));
    let v: serde_json::Value = agent().get(&url).call()?.into_json()?;
    let files = v
        .get("files")
        .and_then(|f| f.as_array())
        .cloned()
        .unwrap_or_default();
    let server = v
        .get("server")
        .and_then(|s| s.as_str())
        .unwrap_or("archive.org");
    let dir = v.get("dir").and_then(|d| d.as_str()).unwrap_or(id);
    let mut out = Vec::new();
    for f in files {
        let name = f.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        let lower = name.to_lowercase();
        let is_game = lower.ends_with(".zip")
            || lower.ends_with(".7z")
            || lower.ends_with(".chd")
            || lower.ends_with(".iso")
            || lower.ends_with(".cso")
            || lower.ends_with(".pbp");
        if !is_game || name.contains('_') && lower.ends_with(".xml") {
            continue;
        }
        let size = f
            .get("size")
            .and_then(|s| s.as_u64())
            .or_else(|| f.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        out.push(IaFile {
            url: format!("https://{server}/{dir}/{name}"),
            name,
            size_bytes: size,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn urlencode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoding() {
        assert_eq!(urlencode("PSP ISO collection"), "PSP%20ISO%20collection");
        assert_eq!(urlencode("a+b/c"), "a%2Bb%2Fc");
    }

    #[test]
    fn seeds_present() {
        assert!(!ia_seed_queries().is_empty());
    }
}
