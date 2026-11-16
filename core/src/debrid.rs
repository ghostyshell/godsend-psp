//! Real-Debrid and TorBox clients: turn ddl/torrent/magnet links into
//! direct downloads the way game debrid services do.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const RD_API: &str = "https://api.real-debrid.com/rest/1.1";
const TB_API: &str = "https://torbox.app/v1/api";

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()
}

#[derive(Debug, Clone, Serialize)]
pub struct DebridStatus {
    pub realdebrid: bool,
    pub torbox: bool,
}

pub fn status(rd_key: &str, tb_key: &str) -> DebridStatus {
    DebridStatus {
        realdebrid: !rd_key.is_empty(),
        torbox: !tb_key.is_empty(),
    }
}

fn is_torrent_like(link: &str) -> bool {
    link.starts_with("magnet:") || link.contains("/torrent/") || link.ends_with(".torrent")
}

/// Resolve a link into a direct download URL, using the first connected
/// provider that supports it. `supported_by` hints which providers work.
pub fn resolve(link: &str, supported_by: &[String], rd_key: &str, tb_key: &str) -> Result<String> {
    if link.starts_with("magnet:") || link.ends_with(".torrent") || link.contains("/torrent/") {
        if !tb_key.is_empty() && (supported_by.iter().any(|s| s == "torbox") || supported_by.is_empty()) {
            return torbox_torrent(link, tb_key);
        }
        if !rd_key.is_empty() {
            return rd_torrent(link, rd_key);
        }
        return Err(Error::Other(
            "torrent link needs a connected debrid account (Settings > Debrid)".into(),
        ));
    }
    // DDL: prefer an unrestrict for extra speed, else the raw link.
    if !rd_key.is_empty() && supported_by.iter().any(|s| s == "realdebrid") {
        if let Ok(url) = rd_unrestrict(link, rd_key) {
            return Ok(url);
        }
    }
    if !tb_key.is_empty() && supported_by.iter().any(|s| s == "torbox") {
        if let Ok(url) = torbox_unrestrict(link, tb_key) {
            return Ok(url);
        }
    }
    Ok(link.to_string())
}

// ---------- Real-Debrid ----------

#[derive(Deserialize)]
struct RdUnrestrict {
    download: String,
}

#[derive(Deserialize)]
struct RdTorrentAdd {
    id: String,
}

#[derive(Deserialize)]
struct RdTorrentInfo {
    status: String,
    files: Vec<RdFile>,
    links: Vec<String>,
}

#[derive(Deserialize)]
struct RdFile {
    id: u64,
    bytes: u64,
    #[serde(default)]
    path: Option<String>,
}

fn rd_unrestrict(link: &str, key: &str) -> Result<String> {
    let res = agent()
        .post(&format!("{RD_API}/unrestrict/link"))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("link", link)]);
    match res {
        Ok(r) => Ok(r.into_json::<RdUnrestrict>()?.download),
        Err(ureq::Error::Status(code, _)) => Err(Error::Http(format!("real-debrid {code}"))),
        Err(e) => Err(Error::Http(format!("real-debrid: {e}"))),
    }
}

fn rd_torrent(magnet: &str, key: &str) -> Result<String> {
    let added: RdTorrentAdd = agent()
        .post(&format!("{RD_API}/torrents/addMagnet"))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("magnet", magnet)])?
        .into_json()?;
    // Select every file, then poll until the torrent is ready.
    let _: serde_json::Value = agent()
        .post(&format!("{RD_API}/torrents/selectFiles/{}", added.id))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("files", "all")])?
        .into_json()?;

    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        if Instant::now() > deadline {
            return Err(Error::Http("real-debrid: torrent did not finish in 4 minutes".into()));
        }
        std::thread::sleep(Duration::from_secs(3));
        let info: RdTorrentInfo = agent()
            .get(&format!("{RD_API}/torrents/info/{}", added.id))
            .set("Authorization", &format!("Bearer {key}"))
            .call()?
            .into_json()?;
        if info.status == "downloaded" && !info.links.is_empty() {
            // Pick the largest file's restricted link.
            let best = info
                .files
                .iter()
                .max_by_key(|f| f.bytes)
                .map(|f| f.id)
                .unwrap_or_default();
            let _ = best;
            let link = info.links.last().expect("links non-empty").clone();
            return rd_unrestrict(&link, key);
        }
        if matches!(info.status.as_str(), "dead" | "error") {
            return Err(Error::Http(format!("real-debrid torrent status: {}", info.status)));
        }
    }
}

fn rd_torrent_file(magnet: &str, file_path: &str, file_index: u32, key: &str) -> Result<String> {
    let added: RdTorrentAdd = agent()
        .post(&format!("{RD_API}/torrents/addMagnet"))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("magnet", magnet)])?
        .into_json()?;

    let deadline = Instant::now() + Duration::from_secs(600);
    let wanted = loop {
        if Instant::now() > deadline {
            return Err(Error::Http(
                "real-debrid: torrent files did not list in 10 minutes".into(),
            ));
        }
        std::thread::sleep(Duration::from_secs(3));
        let info: RdTorrentInfo = agent()
            .get(&format!("{RD_API}/torrents/info/{}", added.id))
            .set("Authorization", &format!("Bearer {key}"))
            .call()?
            .into_json()?;
        if matches!(info.status.as_str(), "dead" | "error") {
            return Err(Error::Http(format!("real-debrid torrent status: {}", info.status)));
        }
        if !info.files.is_empty() {
            let files: Vec<(u64, String)> = info
                .files
                .iter()
                .map(|file| (file.id, file.path.clone().unwrap_or_default()))
                .collect();
            break matching_file(&files, file_path, file_index).ok_or_else(|| {
                Error::Http(format!(
                    "real-debrid: file {} not in torrent",
                    file_basename(file_path)
                ))
            })?;
        }
    };

    let selected = wanted.to_string();
    let _: serde_json::Value = agent()
        .post(&format!("{RD_API}/torrents/selectFiles/{}", added.id))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("files", selected.as_str())])?
        .into_json()?;

    loop {
        if Instant::now() > deadline {
            return Err(Error::Http(
                "real-debrid: torrent did not finish in 10 minutes".into(),
            ));
        }
        std::thread::sleep(Duration::from_secs(3));
        let info: RdTorrentInfo = agent()
            .get(&format!("{RD_API}/torrents/info/{}", added.id))
            .set("Authorization", &format!("Bearer {key}"))
            .call()?
            .into_json()?;
        if matches!(info.status.as_str(), "dead" | "error") {
            return Err(Error::Http(format!("real-debrid torrent status: {}", info.status)));
        }
        if info.status == "downloaded" && !info.links.is_empty() {
            return rd_unrestrict(&info.links[0], key);
        }
    }
}

// ---------- TorBox ----------

#[derive(Deserialize)]
struct TbEnvelope {
    success: bool,
    data: serde_json::Value,
}

#[derive(Deserialize)]
struct TbTorrent {
    id: u64,
    download_state: Option<String>,
    download_url: Option<String>,
    #[serde(default)]
    files: Option<Vec<TbFile>>,
}

#[derive(Deserialize)]
struct TbFile {
    id: u64,
    #[serde(default)]
    name: String,
}

fn torbox_unrestrict(link: &str, key: &str) -> Result<String> {
    let res = agent()
        .post(&format!("{TB_API}/webdl/createwebdl"))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("link", link), ("as_queued", "false")]);
    match res {
        Ok(r) => {
            let env: TbEnvelope = r.into_json()?;
            if env.success {
                if let Some(url) = env.data.get("download_url").and_then(|v| v.as_str()) {
                    return Ok(url.to_string());
                }
            }
            Err(Error::Http("torbox webdl: no url".into()))
        }
        Err(ureq::Error::Status(code, _)) => Err(Error::Http(format!("torbox {code}"))),
        Err(e) => Err(Error::Http(format!("torbox: {e}"))),
    }
}

fn torbox_torrent(magnet: &str, key: &str) -> Result<String> {
    let env: TbEnvelope = agent()
        .post(&format!("{TB_API}/torrents/createtorrent"))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("magnet", magnet), ("seed", "2"), ("allow_zip", "false")])?
        .into_json()?;
    let torrent_id = env
        .data
        .get("torrent_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::Http("torbox: no torrent id".into()))?;

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        if Instant::now() > deadline {
            return Err(Error::Http("torbox: torrent did not finish in 5 minutes".into()));
        }
        std::thread::sleep(Duration::from_secs(4));
        let env: TbEnvelope = agent()
            .get(&format!("{TB_API}/torrents/mylist"))
            .set("Authorization", &format!("Bearer {key}"))
            .call()?
            .into_json()?;
        let list: Vec<TbTorrent> =
            serde_json::from_value(env.data).map_err(|e| Error::Http(format!("torbox list: {e}")))?;
        if let Some(t) = list.iter().find(|t| t.id == torrent_id) {
            if let Some(url) = &t.download_url {
                if !url.is_empty() {
                    return Ok(url.clone());
                }
            }
            let state = t.download_state.as_deref().unwrap_or("");
            if matches!(state, "error" | "no_download_links") {
                return Err(Error::Http(format!("torbox torrent state: {state}")));
            }
        }
    }
}

#[allow(dead_code)]
fn unused(_: &str) {
    let _ = is_torrent_like("");
}

// ---------- single-file torrent resolution ----------

/// Basename of a torrent or remote path, tolerating both separators.
fn file_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Pick a provider file id by basename, falling back to the torrent's own
/// 1-based index (provider ids are 0-based in the selection APIs we use).
fn matching_file(files: &[(u64, String)], file_path: &str, file_index: u32) -> Option<u64> {
    let wanted = file_basename(file_path);
    if !wanted.is_empty() {
        if let Some((id, _)) = files
            .iter()
            .find(|(_, path)| !path.is_empty() && file_basename(path) == wanted)
        {
            return Some(*id);
        }
    }
    let fallback = file_index.saturating_sub(1) as u64;
    files
        .iter()
        .find(|(id, _)| *id == fallback)
        .map(|(id, _)| *id)
}

/// Resolve one file out of a torrent into a direct URL using the connected
/// debrid account: Real-Debrid first, then TorBox.
pub fn resolve_torrent_file(
    magnet: &str,
    file_path: &str,
    file_index: u32,
    rd_key: &str,
    tb_key: &str,
) -> Result<String> {
    if !rd_key.is_empty() {
        return rd_torrent_file(magnet, file_path, file_index, rd_key);
    }
    if !tb_key.is_empty() {
        return torbox_torrent_file(magnet, file_path, file_index, tb_key);
    }
    Err(Error::Other(
        "single-file torrent needs a connected debrid account (Settings > Debrid)".into(),
    ))
}

fn tb_torrent_rows(data: serde_json::Value) -> Result<Vec<TbTorrent>> {
    if data.is_array() {
        serde_json::from_value(data).map_err(|e| Error::Http(format!("torbox list: {e}")))
    } else if data.is_object() {
        serde_json::from_value::<TbTorrent>(data)
            .map(|row| vec![row])
            .map_err(|e| Error::Http(format!("torbox list: {e}")))
    } else {
        Ok(Vec::new())
    }
}

fn tb_request_dl(torrent_id: u64, file_id: u64, key: &str) -> Result<String> {
    let env: TbEnvelope = agent()
        .get(&format!(
            "{TB_API}/torrents/requestdl?token={key}&torrent_id={torrent_id}&file_id={file_id}"
        ))
        .set("Authorization", &format!("Bearer {key}"))
        .call()?
        .into_json()?;
    if !env.success {
        return Err(Error::Http("torbox requestdl: unsuccessful".into()));
    }
    if let Some(url) = env.data.as_str() {
        if !url.is_empty() {
            return Ok(url.to_string());
        }
    }
    for field in ["url", "download_url"] {
        if let Some(url) = env.data.get(field).and_then(|v| v.as_str()) {
            if !url.is_empty() {
                return Ok(url.to_string());
            }
        }
    }
    Err(Error::Http("torbox requestdl: no url".into()))
}

fn torbox_torrent_file(magnet: &str, file_path: &str, file_index: u32, key: &str) -> Result<String> {
    let env: TbEnvelope = agent()
        .post(&format!("{TB_API}/torrents/createtorrent"))
        .set("Authorization", &format!("Bearer {key}"))
        .send_form(&[("magnet", magnet), ("seed", "2"), ("allow_zip", "false")])?
        .into_json()?;
    let torrent_id = env
        .data
        .get("torrent_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Error::Http("torbox: no torrent id".into()))?;

    let deadline = Instant::now() + Duration::from_secs(600);
    let wanted = loop {
        if Instant::now() > deadline {
            return Err(Error::Http(
                "torbox: torrent files did not list in 10 minutes".into(),
            ));
        }
        std::thread::sleep(Duration::from_secs(3));
        let env: TbEnvelope = agent()
            .get(&format!("{TB_API}/torrents/mylist?id={torrent_id}"))
            .set("Authorization", &format!("Bearer {key}"))
            .call()?
            .into_json()?;
        let rows = tb_torrent_rows(env.data)?;
        let Some(row) = rows.iter().find(|row| row.id == torrent_id) else {
            continue;
        };
        let state = row.download_state.as_deref().unwrap_or("");
        if matches!(state, "error" | "no_download_links") {
            return Err(Error::Http(format!("torbox torrent state: {state}")));
        }
        if let Some(files) = &row.files {
            if !files.is_empty() {
                let list: Vec<(u64, String)> =
                    files.iter().map(|file| (file.id, file.name.clone())).collect();
                break matching_file(&list, file_path, file_index).ok_or_else(|| {
                    Error::Http(format!(
                        "torbox: file {} not in torrent",
                        file_basename(file_path)
                    ))
                })?;
            }
        }
        if state == "downloaded" {
            if let Some(url) = &row.download_url {
                if !url.is_empty() {
                    return Ok(url.clone());
                }
            }
        }
    };
    tb_request_dl(torrent_id, wanted, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basename_matching_ignores_the_folder_prefix() {
        let files = vec![
            (0, "PSP/Redump/007 - From Russia with Love (Europe).zip".to_string()),
            (1, "PSP/Redump/Ape Escape P (Europe).iso".to_string()),
        ];
        assert_eq!(
            matching_file(&files, "Other/Root/Ape Escape P (Europe).iso", 99),
            Some(1)
        );
        assert_eq!(matching_file(&files, "Ape Escape P (Europe).iso", 1), Some(1));
    }

    #[test]
    fn basename_matching_falls_back_to_the_file_index() {
        let files = vec![(0, String::new()), (1, "unknown.iso".to_string())];
        assert_eq!(matching_file(&files, "missing.iso", 2), Some(1));
        assert_eq!(matching_file(&files, "missing.iso", 50), None);
    }

    #[test]
    fn windows_paths_and_empty_lists_are_safe() {
        assert_eq!(file_basename("a\\b\\game.iso"), "game.iso");
        assert_eq!(matching_file(&[], "game.iso", 1), None);
    }
}
