//! Store sources: the game-search debrid index and Internet Archive
//! (r/Roms megathread style PSP sets).

use crate::config::Config;
use crate::error::{Error, Result};
use crate::torrent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

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
    #[serde(default)]
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
    let body: serde_json::Value = agent().get(&url).call()?.into_json()?;
    // The live index wraps rows in { games: [...], limit, offset }; older
    // builds returned a bare array, so keep accepting both.
    let list = body
        .get("games")
        .and_then(|games| games.as_array())
        .or_else(|| body.as_array())
        .cloned()
        .unwrap_or_default();
    parse_game_hits(&list)
}

/// Parse a list of game rows, filling in the derived fields the UI needs.
fn parse_game_hits(list: &[serde_json::Value]) -> Result<Vec<GameHit>> {
    let mut out = Vec::new();
    for v in list {
        // The index may return bare game rows or link rows; accept both.
        let mut hit: GameHit = serde_json::from_value(v.clone())?;
        if hit.id.is_empty() {
            hit.id = hit.slug.clone();
        }
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
    if hit.id.is_empty() {
        hit.id = hit.slug.clone();
    }
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

// ---------- r-roms (PSP listing page) ----------

/// The r-roms PSP page. GitHub Pages serves the extension-less path; the
/// trailing-slash form is tried as a fallback.
pub const RROMS_PSP_URL: &str = "https://r-roms.github.io/Sony/sony-playstation-portable";
const RROMS_PSP_URL_SLASH: &str = "https://r-roms.github.io/Sony/sony-playstation-portable/";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RromsItem {
    pub id: String,
    pub title: String,
    pub url: String,
    /// `ia_item` for archive.org items, else `zip`, `7z` or `torrent`.
    pub kind: String,
}

/// Read the r-roms PSP page and return its item links.
pub fn rroms_items() -> Result<Vec<RromsItem>> {
    let mut last: Option<Error> = None;
    for url in [RROMS_PSP_URL, RROMS_PSP_URL_SLASH] {
        match agent().get(url).call() {
            Ok(resp) => match read_text(resp) {
                Ok(html) => {
                    let items = parse_rroms_html(url, &html);
                    if !items.is_empty() {
                        return Ok(items);
                    }
                    last = Some(Error::Http("r-roms PSP page returned no item links".into()));
                }
                Err(err) => last = Some(err),
            },
            Err(err) => last = Some(Error::Ureq(err)),
        }
    }
    Err(last.unwrap_or_else(|| Error::Http("r-roms PSP page unavailable".into())))
}

/// Files behind an r-roms entry. archive.org items expand to their game files;
/// direct archive links are a single file.
pub fn rroms_files(id: &str) -> Result<Vec<IaFile>> {
    let item = rroms_items()?
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| Error::NotFound(format!("r-roms item {id}")))?;
    if item.kind == "ia_item" {
        return archive_item_files(&item.url);
    }
    let path = item.url.split('?').next().unwrap_or(&item.url);
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    Ok(vec![IaFile {
        name,
        size_bytes: 0,
        url: item.url,
    }])
}

/// Append Internet Archive S3 credentials to an archive.org download URL.
/// Other hosts and non-download paths are returned unchanged.
pub fn ia_signed_url(url: &str, config: &Config) -> String {
    if !config.ia_keys_set() {
        return url.to_string();
    }
    let lower = url.to_ascii_lowercase();
    let downloadable = lower.starts_with("https://archive.org/download/")
        || lower.starts_with("http://archive.org/download/");
    if !downloadable {
        return url.to_string();
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    format!(
        "{url}{sep}access_key={}&secret_key={}",
        urlencode(&config.ia_access_key),
        urlencode(&config.ia_secret_key)
    )
}

/// Expand an archive.org item (or subfolder) URL into its game files.
fn archive_item_files(url: &str) -> Result<Vec<IaFile>> {
    let Some((identifier, subdir)) = archive_item_path(url) else {
        return Ok(vec![IaFile {
            name: url.rsplit('/').next().unwrap_or(url).to_string(),
            size_bytes: 0,
            url: url.to_string(),
        }]);
    };
    let meta_url = format!("https://archive.org/metadata/{}", urlencode(&identifier));
    let v: serde_json::Value = agent().get(&meta_url).call()?.into_json()?;
    let files = v
        .get("files")
        .and_then(|f| f.as_array())
        .cloned()
        .unwrap_or_default();
    let server = v
        .get("server")
        .and_then(|s| s.as_str())
        .unwrap_or("archive.org");
    let dir = v
        .get("dir")
        .and_then(|d| d.as_str())
        .unwrap_or(&identifier)
        .trim_start_matches('/');
    let mut matched = Vec::new();
    let mut all = Vec::new();
    for f in files {
        let name = f.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        if !is_game_file(&name) {
            continue;
        }
        let size = f
            .get("size")
            .and_then(|s| s.as_u64())
            .or_else(|| f.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        let encoded = name
            .split('/')
            .map(urlencode)
            .collect::<Vec<_>>()
            .join("/");
        // Keep keys out of listings; the download path signs the URL.
        let file_url = format!("https://{server}/{dir}/{encoded}");
        all.push(IaFile {
            name: name.clone(),
            size_bytes: size,
            url: file_url.clone(),
        });
        let prefix = format!("{subdir}/");
        if !subdir.is_empty() && name.starts_with(&prefix) {
            matched.push(IaFile {
                name: name[prefix.len()..].to_string(),
                size_bytes: size,
                url: file_url,
            });
        }
    }
    let mut out = if subdir.is_empty() { all } else { matched };
    if out.is_empty() {
        return Err(Error::NotFound(format!("no downloadable files in {identifier}")));
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn archive_item_path(url: &str) -> Option<(String, String)> {
    let path = url.split(['?', '#']).next()?;
    let marker = "archive.org/download/";
    let idx = path.find(marker)?;
    let rest = path[idx + marker.len()..].trim_end_matches('/');
    if rest.is_empty() {
        return None;
    }
    let mut parts = rest.splitn(2, '/');
    let id = percent_decode(parts.next()?);
    let sub = percent_decode(parts.next().unwrap_or(""));
    Some((id, sub))
}

fn is_game_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".zip", ".7z", ".chd", ".iso", ".cso", ".pbp", ".rvz"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// Parse the links out of an r-roms page. Pure so it is testable offline:
/// table rows give each link a title from the first cell, plain pages fall
/// back to the nearest heading above the anchor.
pub fn parse_rroms_html(base: &str, html: &str) -> Vec<RromsItem> {
    let area = content_area(html);
    let mut out = Vec::new();
    let rows = table_rows(area);
    if rows.is_empty() {
        for (pos, href, text) in anchors(area) {
            let url = resolve_url(base, &href);
            let title = if text.is_empty() {
                heading_before(area, pos).unwrap_or_else(|| title_from_url(&url))
            } else {
                text
            };
            push_rroms(&url, &title, &mut out);
        }
    } else {
        for row in rows {
            let title = first_cell_text(row);
            for (_, href, text) in anchors(row) {
                let url = resolve_url(base, &href);
                let label = if title.is_empty() || title.eq_ignore_ascii_case("title") {
                    if text.is_empty() {
                        title_from_url(&url)
                    } else {
                        text
                    }
                } else {
                    title.clone()
                };
                push_rroms(&url, &label, &mut out);
            }
        }
    }
    out
}

fn push_rroms(url: &str, title: &str, out: &mut Vec<RromsItem>) {
    let Some(kind) = link_kind(url) else { return };
    let id = item_id(url);
    if id.is_empty() || out.iter().any(|i| i.id == id) {
        return;
    }
    out.push(RromsItem {
        id,
        title: title.to_string(),
        url: url.to_string(),
        kind: kind.to_string(),
    });
}

fn link_kind(url: &str) -> Option<&'static str> {
    let lower = url.to_ascii_lowercase();
    let path = lower.split(['?', '#']).next().unwrap_or(&lower);
    if path.ends_with(".zip") {
        return Some("zip");
    }
    if path.ends_with(".7z") {
        return Some("7z");
    }
    if path.ends_with(".torrent") {
        return Some("torrent");
    }
    let is_archive = path.starts_with("https://archive.org/") || path.starts_with("http://archive.org/");
    if is_archive {
        let tail = path.split("archive.org/").nth(1).unwrap_or("");
        if tail.starts_with("download/") || tail.starts_with("details/") || tail.starts_with("metadata/") {
            return Some("ia_item");
        }
    }
    None
}

fn item_id(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let path = path.trim_end_matches('/');
    if let Some((identifier, subdir)) = archive_item_path(path) {
        return if subdir.is_empty() {
            identifier
        } else {
            format!("{identifier}/{subdir}")
        };
    }
    let tail = path.rsplit('/').next().unwrap_or(path);
    percent_decode(tail)
}

fn title_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url).trim_end_matches('/');
    let tail = path.rsplit('/').next().unwrap_or(path);
    percent_decode(tail)
}

fn content_area(html: &str) -> &str {
    if let Some(start) = html.find("<main") {
        if let Some(end) = html[start..].find("</main>") {
            return &html[start..start + end];
        }
    }
    html
}

fn table_rows(fragment: &str) -> Vec<&str> {
    let lower = fragment.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(rel) = lower[pos..].find("<tr") {
        let start = pos + rel;
        let Some(close) = lower[start..].find("</tr>") else { break };
        out.push(&fragment[start..start + close]);
        pos = start + close + 5;
    }
    out
}

/// Anchors in document order as (byte offset, href, text).
fn anchors(fragment: &str) -> Vec<(usize, String, String)> {
    let lower = fragment.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(rel) = lower[pos..].find("<a") {
        let start = pos + rel;
        let Some(gt) = lower[start..].find('>') else { break };
        let tag_end = start + gt;
        let tag = &fragment[start..=tag_end];
        let Some(href) = attr_value(tag, "href") else {
            pos = tag_end + 1;
            continue;
        };
        let Some(close) = lower[tag_end..].find("</a>") else { break };
        let close = tag_end + close;
        out.push((start, href, strip_tags(&fragment[tag_end + 1..close])));
        pos = close + 4;
    }
    out
}

fn attr_value(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{name}=");
    let bytes = tag.as_bytes();
    let mut from = 0;
    while let Some(rel) = lower[from..].find(&needle) {
        let at = from + rel;
        let prev_ok = at == 0 || bytes[at - 1].is_ascii_whitespace();
        if !prev_ok {
            from = at + needle.len();
            continue;
        }
        let rest = tag[at + needle.len()..].trim_start();
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let body = &rest[quote.len_utf8()..];
            let end = body.find(quote)?;
            return Some(body[..end].to_string());
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    None
}

fn first_cell_text(row: &str) -> String {
    let lower = row.to_ascii_lowercase();
    let mut pos = 0;
    while let Some(rel) = lower[pos..].find("<td") {
        let start = pos + rel;
        let Some(gt) = row[start..].find('>') else { break };
        let tag_end = start + gt;
        let Some(close) = lower[tag_end..].find("</td>") else { break };
        let text = strip_tags(&row[tag_end + 1..tag_end + close]);
        if !text.is_empty() {
            return text;
        }
        pos = tag_end + 1;
    }
    String::new()
}

fn heading_before(fragment: &str, index: usize) -> Option<String> {
    let head = &fragment[..index];
    let lower = head.to_ascii_lowercase();
    let mut best: Option<(usize, u8)> = None;
    for level in 1..=5u8 {
        let tag = format!("<h{level}");
        if let Some(start) = lower.rfind(&tag) {
            if best.map(|(pos, _)| start > pos).unwrap_or(true) {
                best = Some((start, level));
            }
        }
    }
    let (start, level) = best?;
    let tag_end = start + head[start..].find('>')?;
    let closer = format!("</h{level}>");
    let close = head[tag_end..].find(&closer)?;
    let text = strip_tags(&head[tag_end + 1..tag_end + close]);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn strip_tags(fragment: &str) -> String {
    let mut text = String::with_capacity(fragment.len());
    let mut in_tag = false;
    for ch in fragment.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    let decoded = decode_entities(&text);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(text: &str) -> String {
    let mut out = text.to_string();
    for (from, to) in [
        ("&nbsp;", " "),
        ("&amp;", "&"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&#x27;", "'"),
        ("&apos;", "'"),
        ("&lt;", "<"),
        ("&gt;", ">"),
    ] {
        out = out.replace(from, to);
    }
    let mut result = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(pos) = rest.find("&#") {
        result.push_str(&rest[..pos]);
        let tail = &rest[pos + 2..];
        let decoded = tail.find(';').filter(|semi| *semi <= 8).and_then(|semi| {
            let num = &tail[..semi];
            let code = if let Some(hex) = num.strip_prefix('x') {
                u32::from_str_radix(hex, 16).ok()
            } else {
                num.parse::<u32>().ok()
            };
            code.and_then(char::from_u32).map(|ch| (ch, semi))
        });
        if let Some((ch, semi)) = decoded {
            result.push(ch);
            rest = &tail[semi + 1..];
        } else {
            result.push_str("&#");
            rest = tail;
        }
    }
    result.push_str(rest);
    result
}

/// Resolve a page-relative href against the base URL.
pub fn resolve_url(base: &str, href: &str) -> String {
    let href = href.trim();
    if href.is_empty() {
        return String::new();
    }
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    let Some(scheme) = base.find("://") else {
        return href.to_string();
    };
    let scheme_end = scheme + 3;
    let host_end = base[scheme_end..]
        .find('/')
        .map(|i| scheme_end + i)
        .unwrap_or(base.len());
    let origin = &base[..host_end];
    let base_path = &base[host_end..];
    if href.starts_with("//") {
        return format!("{}:{href}", &base[..scheme]);
    }
    if href.starts_with('/') {
        return format!("{origin}{href}");
    }
    if href.starts_with('#') || href.starts_with('?') {
        return format!("{base}{href}");
    }
    let dir = match base_path.rfind('/') {
        Some(i) => &base_path[..=i],
        None => "/",
    };
    match normalize_path(&format!("{dir}{href}")) {
        Some(path) => format!("{origin}{path}"),
        None => format!("{origin}{dir}{href}"),
    }
}

fn normalize_path(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }
    let mut segs: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            other => segs.push(other),
        }
    }
    let mut out = format!("/{}", segs.join("/"));
    if path.ends_with('/') && !out.ends_with('/') {
        out.push('/');
    }
    Some(out)
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_text(response: ureq::Response) -> Result<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(16 * 1024 * 1024)
        .read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

// ---------- NoPayStation (PSP TSV index) ----------

/// The legacy per-console endpoint (302s to the TSV) and the current index.
pub const NPS_PSP_URL: &str = "https://nopaystation.com/psp/nps.php";
pub const NPS_PSP_TSV: &str = "https://nopaystation.com/tsv/PSP_GAMES.tsv";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpsEntry {
    pub title_id: String,
    pub title: String,
    pub region: String,
    pub url: String,
    pub size_bytes: u64,
    /// Decryption key from the zRIF column, or the RAP column when the TSV
    /// carries one instead.
    #[serde(default)]
    pub zrif: Option<String>,
}

/// Fetch the NoPayStation PSP index, preferring the classic endpoint and
/// falling back to the published TSV when it no longer serves the table.
pub fn nps_entries() -> Result<Vec<NpsEntry>> {
    let mut last: Option<Error> = None;
    for url in [NPS_PSP_URL, NPS_PSP_TSV] {
        match agent().get(url).call() {
            Ok(resp) => match read_text(resp) {
                Ok(text) => {
                    let entries = parse_nps_tsv(&text);
                    if !entries.is_empty() {
                        return Ok(entries);
                    }
                    last = Some(Error::Http(format!("{url} returned no PSP rows")));
                }
                Err(err) => last = Some(err),
            },
            Err(err) => last = Some(Error::Ureq(err)),
        }
    }
    Err(last.unwrap_or_else(|| Error::Http("NoPayStation index unavailable".into())))
}

/// Filter the PSP index by a substring of the title or title id.
pub fn nps_search(query: &str, limit: u32) -> Result<Vec<NpsEntry>> {
    let all = nps_entries()?;
    let needle = query.trim().to_ascii_lowercase();
    let limit = limit.max(1) as usize;
    let mut out = Vec::new();
    for entry in all {
        let hit = needle.is_empty()
            || entry.title.to_ascii_lowercase().contains(&needle)
            || entry.title_id.to_ascii_lowercase().contains(&needle);
        if hit {
            out.push(entry);
            if out.len() >= limit {
                break;
            }
        }
    }
    Ok(out)
}

pub fn nps_entry(title_id: &str) -> Result<NpsEntry> {
    nps_entries()?
        .into_iter()
        .find(|e| e.title_id == title_id)
        .ok_or_else(|| Error::NotFound(format!("NoPayStation title {title_id}")))
}

/// Local .pkg name for a NoPayStation entry, with the sidecar stem matching.
pub fn nps_pkg_name(entry: &NpsEntry) -> String {
    format!(
        "{} [{}].pkg",
        crate::media::fat_safe_name(&entry.title),
        crate::media::fat_safe_name(&entry.title_id)
    )
}

/// Parse the NoPayStation TSV. Columns are looked up by header name so the
/// extra Type/Content ID/RAP columns do not shift the ones we need.
pub fn parse_nps_tsv(text: &str) -> Vec<NpsEntry> {
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .unwrap_or("")
        .split('\t')
        .map(|c| c.trim().to_ascii_lowercase())
        .collect();
    let find = |names: &[&str]| {
        header
            .iter()
            .position(|h| names.iter().any(|n| h == n))
    };
    let Some(id_idx) = find(&["title id", "titleid", "title_id"]) else {
        return Vec::new();
    };
    let Some(url_idx) = find(&["pkg direct link", "pkg directlink", "url", "link"]) else {
        return Vec::new();
    };
    let title_idx = find(&["name", "title"]);
    let region_idx = find(&["region"]);
    let size_idx = find(&["file size", "size"]);
    let zrif_idx = find(&["zrif", "z-rif"]).or_else(|| find(&["rap"]));

    let mut out = Vec::new();
    for line in lines {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        let col = |idx: Option<usize>| idx.and_then(|i| cols.get(i)).map(|s| s.trim()).unwrap_or("");
        let title_id = col(Some(id_idx)).to_string();
        let url = col(Some(url_idx)).to_string();
        if title_id.is_empty() || !url.starts_with("http") {
            continue;
        }
        let title = match title_idx {
            Some(i) => col(Some(i)).to_string(),
            None => String::new(),
        };
        let size_bytes = col(size_idx)
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        let zrif = col(zrif_idx);
        out.push(NpsEntry {
            title: if title.is_empty() {
                title_id.clone()
            } else {
                title
            },
            title_id,
            region: col(region_idx).to_string(),
            url,
            size_bytes,
            zrif: if zrif.is_empty() {
                None
            } else {
                Some(zrif.to_string())
            },
        });
    }
    out
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

// ---------- Store games (a flat per-game view of the collections) ----------

/// One game inside a collection, ready for a per-game store row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreGame {
    /// `<set>#<file index>` for Minerva, `<item>/<file name>` for r-roms.
    pub id: String,
    pub title: String,
    pub size_bytes: u64,
    /// Human name of the collection the game came from.
    pub container: String,
    pub source: String,
}

const GAME_CACHE_TTL: Duration = Duration::from_secs(600);

type GameCache = Mutex<HashMap<&'static str, (Instant, Vec<StoreGame>)>>;

fn game_cache() -> &'static GameCache {
    static CACHE: OnceLock<GameCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Build a per-game index once, then serve clones for 10 minutes so tab
/// switches and local filtering never refetch a torrent or IA metadata.
/// Only successful builds are cached.
fn store_games(
    key: &'static str,
    build: impl FnOnce() -> Result<Vec<StoreGame>>,
) -> Result<Vec<StoreGame>> {
    if let Some((at, games)) = game_cache().lock().expect("game cache lock").get(key) {
        if at.elapsed() < GAME_CACHE_TTL {
            return Ok(games.clone());
        }
    }
    let games = build()?;
    game_cache()
        .lock()
        .expect("game cache lock")
        .insert(key, (Instant::now(), games.clone()));
    Ok(games)
}

/// File name from a torrent or archive path, with the game container
/// extension stripped.
pub fn game_title(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    let lower = name.to_ascii_lowercase();
    for ext in [".zip", ".7z", ".iso", ".cso", ".pbp", ".chd", ".rvz"] {
        if lower.ends_with(ext) && name.len() > ext.len() {
            return name[..name.len() - ext.len()].to_string();
        }
    }
    name.to_string()
}

fn sort_games(games: &mut [StoreGame]) {
    games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
}

/// Every game file in every existing Minerva PSP torrent.
pub fn minerva_games() -> Result<Vec<StoreGame>> {
    store_games("minerva", || {
        let mut out = Vec::new();
        for set in minerva_sets()? {
            for file in torrent::fetch_files(&set.url)? {
                let title = game_title(&file.path);
                if title.is_empty() {
                    continue;
                }
                out.push(StoreGame {
                    id: format!("{}#{}", set.id, file.index),
                    title,
                    size_bytes: file.length,
                    container: set.title.clone(),
                    source: "minerva".into(),
                });
            }
        }
        sort_games(&mut out);
        Ok(out)
    })
}

/// Every game file inside the r-roms Internet Archive items.
pub fn rroms_games() -> Result<Vec<StoreGame>> {
    store_games("rroms", || {
        let mut out = Vec::new();
        for item in rroms_items()?.into_iter().filter(|item| item.kind == "ia_item") {
            // Some linked items hold no game files (DLC folders, docs); skip
            // them, but still fail loudly when Internet Archive itself is down.
            let files = match rroms_files(&item.id) {
                Ok(files) => files,
                Err(Error::NotFound(_)) => continue,
                Err(err) => return Err(err),
            };
            for file in files {
                let title = game_title(&file.name);
                if title.is_empty() {
                    continue;
                }
                out.push(StoreGame {
                    id: format!("{}/{}", item.id, file.name),
                    title,
                    size_bytes: file.size_bytes,
                    container: item.title.clone(),
                    source: "r-roms".into(),
                });
            }
        }
        sort_games(&mut out);
        Ok(out)
    })
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
    fn gamesearch_parses_the_live_envelope() {
        // Mirrors https://game-debrid.link/api/games?platform=psp&limit=2:
        // an envelope, no per-row `id`, and a rawgCover image.
        let body = r#"{
            "games": [
                {
                    "slug": "pes-2008-pro-evolution-soccer-usa",
                    "title": "PES 2008 - Pro Evolution Soccer (USA)",
                    "platform": "psp",
                    "source": "rroms",
                    "linkCount": 1,
                    "rawgCover": "https://media.rawg.io/media/games/e41/e41d8f1826d550f382a97b72f54e5614.jpg"
                }
            ],
            "limit": 2,
            "offset": 0
        }"#;
        let body: serde_json::Value = serde_json::from_str(body).expect("fixture json");
        let rows = body
            .get("games")
            .and_then(|games| games.as_array())
            .cloned()
            .unwrap_or_default();
        let hits = parse_game_hits(&rows).expect("parse hits");
        assert_eq!(hits.len(), 1);
        let hit = &hits[0];
        assert_eq!(hit.id, hit.slug);
        assert_eq!(hit.id, "pes-2008-pro-evolution-soccer-usa");
        assert_eq!(hit.title, "PES 2008 - Pro Evolution Soccer (USA)");
        assert_eq!(hit.platform, "psp");
        assert_eq!(
            hit.rawg_cover.as_deref(),
            Some("https://media.rawg.io/media/games/e41/e41d8f1826d550f382a97b72f54e5614.jpg")
        );
    }

    #[test]
    fn gamesearch_accepts_a_bare_array() {
        let rows: Vec<serde_json::Value> = serde_json::from_str(
            r#"[{"slug":"daxter-usa","title":"Daxter (USA)","platform":"psp"}]"#,
        )
        .expect("fixture json");
        let hits = parse_game_hits(&rows).expect("parse hits");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "daxter-usa");
    }

    #[test]
    fn seeds_present() {
        assert!(!ia_seed_queries().is_empty());
    }

    #[test]
    fn relative_links_resolve() {
        let base = "https://r-roms.github.io/Sony/sony-playstation-portable";
        assert_eq!(
            resolve_url(base, "https://archive.org/download/set1"),
            "https://archive.org/download/set1"
        );
        assert_eq!(
            resolve_url(base, "../General/torrents/psp.torrent"),
            "https://r-roms.github.io/General/torrents/psp.torrent"
        );
        assert_eq!(
            resolve_url(base, "/Sony/files/game.zip"),
            "https://r-roms.github.io/Sony/files/game.zip"
        );
        assert_eq!(
            resolve_url("https://r-roms.github.io/Sony/page/", "part2.7z"),
            "https://r-roms.github.io/Sony/page/part2.7z"
        );
    }

    #[test]
    fn rroms_table_parsing() {
        let html = r#"
        <header><a href="../General/torrents">nav</a></header>
        <main>
          <h1>PSP</h1>
          <table><tr><th>Title</th><th>IA</th></tr>
          <tr><td>PSP Redump .iso (Part 1)</td>
              <td><a href="https://archive.org/download/sony_psp_part1">Download</a></td></tr>
          <tr><td>PSP Redump .iso (Part 2)</td>
              <td><a href="https://archive.org/download/sony_psp_part2/">Download</a></td></tr>
          <tr><td>Homebrew pack</td><td><a href="../files/homebrew.zip">Download</a></td></tr>
          <tr><td>Other</td><td><a href="https://example.com/page">ignore</a></td></tr>
          </table>
        </main>
        <footer><a href="https://example.com/credit">credit</a></footer>
        "#;
        let items = parse_rroms_html(RROMS_PSP_URL, html);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].title, "PSP Redump .iso (Part 1)");
        assert_eq!(items[0].kind, "ia_item");
        assert_eq!(items[0].id, "sony_psp_part1");
        assert_eq!(items[1].id, "sony_psp_part2");
        assert_eq!(items[2].kind, "zip");
        assert_eq!(items[2].url, "https://r-roms.github.io/files/homebrew.zip");
    }

    #[test]
    fn rroms_archive_subfolder_id() {
        let html = r#"<main><table>
        <tr><td>PSP DLC</td>
        <td><a href="https://archive.org/download/PSP-DLC/PSP%20DLC/">Download</a></td></tr>
        </table></main>"#;
        let items = parse_rroms_html(RROMS_PSP_URL, html);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "PSP-DLC/PSP DLC");
    }

    #[test]
    fn ia_url_signing_only_for_downloads() {
        let mut config = Config::default();
        assert_eq!(ia_signed_url("https://archive.org/download/x/y.zip", &config), "https://archive.org/download/x/y.zip");
        config.ia_access_key = "KEY".into();
        config.ia_secret_key = "SECRET".into();
        assert_eq!(
            ia_signed_url("https://archive.org/download/x/y.zip", &config),
            "https://archive.org/download/x/y.zip?access_key=KEY&secret_key=SECRET"
        );
        assert_eq!(
            ia_signed_url("https://example.com/y.zip", &config),
            "https://example.com/y.zip"
        );
    }

    #[test]
    fn nps_tsv_parsing() {
        let tsv = "Title ID\tRegion\tType\tName\tPKG direct link\tContent ID\tLast Modification Date\tRAP\tDownload .RAP file\tFile Size\tSHA256\n\
NPUH10001\tUS\tPSP\tExample Game\thttps://zeus.dl.playstation.net/cdn/x.pkg\tUP0000-NPUH10001_00-EXAMPLExxxxxxxxx\t2021-01-01 00:00:00\tRAPKEY\t\t18637808\tabc\n\
NPUH10002\tEU\tPSP\tSecond Game\thttps://zeus.dl.playstation.net/cdn/y.pkg\tUP0000-NPUH10002_00-EXAMPLEyyyyyyyy\t2021-01-01 00:00:00\t\t\t100\tdef\n\
broken row without tabs\n";
        let entries = parse_nps_tsv(tsv);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title_id, "NPUH10001");
        assert_eq!(entries[0].title, "Example Game");
        assert_eq!(entries[0].region, "US");
        assert_eq!(entries[0].size_bytes, 18637808);
        assert_eq!(entries[0].zrif.as_deref(), Some("RAPKEY"));
        assert_eq!(entries[1].size_bytes, 100);
        assert!(entries[1].zrif.is_none());
    }

    #[test]
    fn nps_pkg_name_is_fat_safe() {
        let entry = NpsEntry {
            title_id: "NPUH10001".into(),
            title: "Example: Game/One".into(),
            region: "US".into(),
            url: "https://example.invalid/x.pkg".into(),
            size_bytes: 1,
            zrif: None,
        };
        assert_eq!(nps_pkg_name(&entry), "Example_ Game_One [NPUH10001].pkg");
    }

    #[test]
    fn game_titles_strip_container_extensions() {
        assert_eq!(
            game_title("Redump/Sony - PlayStation Portable/007 - From Russia with Love (Europe).zip"),
            "007 - From Russia with Love (Europe)"
        );
        assert_eq!(game_title("Ape Escape P (USA).ISO"), "Ape Escape P (USA)");
        assert_eq!(game_title("game.pbp"), "game");
        assert_eq!(game_title("game.chd"), "game");
        assert_eq!(game_title("no-extension"), "no-extension");
    }

    /// Plausible game title: at least a few letters spread over two words.
    fn plausible_game_title(title: &str) -> bool {
        let words = title
            .split_whitespace()
            .filter(|word| word.chars().any(|ch| ch.is_ascii_alphabetic()))
            .count();
        title.len() >= 5 && words >= 2
    }

    #[test]
    #[ignore = "network: builds the live Minerva game index"]
    fn minerva_games_live() {
        let games = minerva_games().expect("minerva game index");
        assert!(!games.is_empty(), "minerva game index is empty");
        assert!(
            games.iter().any(|game| plausible_game_title(&game.title)),
            "no plausible game title in the minerva index"
        );
        println!("minerva games: {}", games.len());
        let containers: Vec<&str> = games.iter().take(3).map(|game| game.container.as_str()).collect();
        println!("containers: {containers:?}");
        let samples: Vec<&str> = games.iter().take(5).map(|game| game.title.as_str()).collect();
        println!("samples: {samples:?}");
    }

    #[test]
    #[ignore = "network: builds the live r-roms game index"]
    fn rroms_games_live() {
        let games = rroms_games().expect("r-roms game index");
        assert!(!games.is_empty(), "r-roms game index is empty");
        assert!(
            games.iter().any(|game| plausible_game_title(&game.title)),
            "no plausible game title in the r-roms index"
        );
        println!("r-roms games: {}", games.len());
        let containers: Vec<&str> = games.iter().take(3).map(|game| game.container.as_str()).collect();
        println!("containers: {containers:?}");
        let samples: Vec<&str> = games.iter().take(5).map(|game| game.title.as_str()).collect();
        println!("samples: {samples:?}");
    }
}
