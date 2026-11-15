//! App configuration persisted under the platform data directory.

use crate::error::{Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Default audio conversion format for music pushes (`mp3` or `m4a`).
    pub music_format: String,
    /// Default video preset id (`psp_avc_480`).
    pub video_preset: String,
    /// Generate a 160x120 `.THM` thumbnail next to pushed videos.
    pub video_thumbnails: bool,
    /// Convert music to the PSP format by default.
    pub convert_music: bool,
    /// Convert video to the PSP format by default.
    pub convert_video: bool,
    /// Optional explicit path to an ffmpeg binary. Empty means auto-detect.
    pub ffmpeg_path: String,
    /// Optional explicit path to an ffprobe binary. Empty means auto-detect.
    pub ffprobe_path: String,
    /// ISO to CSO compression level (1-9).
    pub cso_level: u32,
    /// Where store downloads land. Empty = `<Downloads>/godsend-psp`.
    pub download_dir: String,
    /// Real-Debrid API key. Empty = not connected.
    pub rd_api_key: String,
    /// TorBox API key. Empty = not connected.
    pub tb_api_key: String,
    /// game-search service base URL (the debrid.link-style index).
    pub gamesearch_base: String,
    /// Public cover base. Default is the production covers host; empty falls
    /// back to catalog cover URLs only.
    pub covers_base: String,
    /// Mods catalog base (JSON per serial). Empty = mods disabled.
    pub mods_base: String,
    /// Serve the loopback MCP endpoint while the desktop app runs.
    pub mcp_enabled: bool,
    /// Loopback port for the MCP endpoint.
    pub mcp_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_format: "mp3".into(),
            video_preset: "psp_avc_480".into(),
            video_thumbnails: true,
            convert_music: true,
            convert_video: true,
            ffmpeg_path: String::new(),
            ffprobe_path: String::new(),
            cso_level: 6,
            download_dir: String::new(),
            rd_api_key: String::new(),
            tb_api_key: String::new(),
            gamesearch_base: "http://localhost:3011".into(),
            covers_base: "https://covers.debrid.in".into(),
            mods_base: "https://ghostyshell.github.io/godsend-psp".into(),
            mcp_enabled: true,
            mcp_port: 8849,
        }
    }
}

/// `click.godsendpsp.app` style qualifier so the data dir is
/// `~/Library/Application Support/godsend-psp` on macOS and friends elsewhere.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("click", "godsendpsp", "godsend-psp")
}

pub fn data_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("godsend-psp"))
}

pub fn cache_dir() -> PathBuf {
    data_dir().join("cache")
}

pub fn icons_cache_dir() -> PathBuf {
    cache_dir().join("icons")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

pub fn jobs_path() -> PathBuf {
    data_dir().join("jobs.json")
}

pub fn catalog_cache_path() -> PathBuf {
    cache_dir().join("catalog.json")
}

impl Config {
    pub fn load() -> Config {
        let path = config_path();
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                tracing::warn!("config at {} is corrupt ({}), using defaults", path.display(), e);
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        fs::write(path, bytes)?;
        Ok(())
    }

    /// Overwrite only the fields present in `patch` (a partial JSON object).
    pub fn patch_from_json(&mut self, patch: &serde_json::Value) -> Result<()> {
        let merged = merge_json(
            &serde_json::to_value(&*self)?,
            patch,
        );
        let next: Config = serde_json::from_value(merged)
            .map_err(|e| Error::Invalid(format!("config patch rejected: {}", e)))?;
        *self = next;
        self.save()
    }
}

/// Shallow-merge `patch` over `base` (recursively for nested objects).
fn merge_json(base: &serde_json::Value, patch: &serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            let mut out = b.clone();
            for (k, v) in p {
                let merged = match out.get(k) {
                    Some(existing) => merge_json(existing, v),
                    None => v.clone(),
                };
                out.insert(k.clone(), merged);
            }
            serde_json::Value::Object(out)
        }
        (_, v) => v.clone(),
    }
}

/// Ensure the data/cache directories exist and are writable.
pub fn ensure_dirs() -> Result<()> {
    for dir in [data_dir(), cache_dir(), icons_cache_dir()] {
        fs::create_dir_all(&dir)?;
        probe_writable(&dir)?;
    }
    Ok(())
}

fn probe_writable(dir: &Path) -> Result<()> {
    let probe = dir.join(".write-probe");
    fs::write(&probe, b"ok")?;
    fs::remove_file(&probe)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_merges_shallow_fields() {
        let mut cfg = Config::default();
        let patch = serde_json::json!({ "music_format": "m4a", "cso_level": 9 });
        cfg.patch_from_json(&patch).expect("patch must apply");
        assert_eq!(cfg.music_format, "m4a");
        assert_eq!(cfg.cso_level, 9);
        assert_eq!(cfg.video_preset, "psp_avc_480");
    }
}
