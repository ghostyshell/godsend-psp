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
    /// Generate a 160x120 `.THM` thumbnail next to pushed videos.
    pub video_thumbnails: bool,
    /// Convert music to the PSP format by default.
    pub convert_music: bool,
    /// Convert video to the PSP format by default.
    pub convert_video: bool,
    /// x264 speed preset; veryslow gives the best compression at a given CRF.
    /// (Replaces the old unused profile id.)
    pub video_preset: String,
    /// x264 CRF; lower is better. 16 is effectively lossless at PSP sizes.
    pub video_crf: u32,
    /// Frame rate cap (PSP AVC level 3.0 tops out around 30).
    pub video_fps: u32,
    /// AAC audio bitrate in kbps.
    pub video_audio_kbps: u32,
    /// Target resolution, or "source" to keep the input size.
    pub video_resolution: String,
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
    /// Internet Archive S3 access key, for items that need a signed download.
    pub ia_access_key: String,
    /// Internet Archive S3 secret key. Never log this value.
    pub ia_secret_key: String,
    /// Internet Archive account email. Reserved for cookie-based access.
    pub ia_email: String,
    /// Internet Archive account password. Reserved; never log this value.
    pub ia_password: String,
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
            video_preset: "veryslow".into(),
            video_thumbnails: true,
            convert_music: true,
            convert_video: true,
            video_crf: 16,
            video_fps: 30,
            video_audio_kbps: 256,
            video_resolution: "480x272".into(),
            ffmpeg_path: String::new(),
            ffprobe_path: String::new(),
            cso_level: 6,
            download_dir: String::new(),
            rd_api_key: String::new(),
            tb_api_key: String::new(),
            gamesearch_base: "https://debrid.link/api".into(),
            ia_access_key: String::new(),
            ia_secret_key: String::new(),
            ia_email: String::new(),
            ia_password: String::new(),
            covers_base: "https://covers.debrid.in".into(),
            mods_base: "https://psp-mods.debrid.in".into(),
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

pub fn secrets_path() -> PathBuf {
    data_dir().join("secrets.json")
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
        let mut cfg = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                tracing::warn!("config at {} is corrupt ({}), using defaults", path.display(), e);
                Config::default()
            }),
            Err(_) => Config::default(),
        };
        // Credentials also live in secrets.json, so an updated config (or a
        // reset one) never loses connected accounts.
        let secrets = read_secrets();
        if cfg.rd_api_key.is_empty() {
            cfg.rd_api_key = secrets.rd_api_key;
        }
        if cfg.tb_api_key.is_empty() {
            cfg.tb_api_key = secrets.tb_api_key;
        }
        if cfg.ia_access_key.is_empty() {
            cfg.ia_access_key = secrets.ia_access_key;
        }
        if cfg.ia_secret_key.is_empty() {
            cfg.ia_secret_key = secrets.ia_secret_key;
        }
        if cfg.ia_email.is_empty() {
            cfg.ia_email = secrets.ia_email;
        }
        if cfg.ia_password.is_empty() {
            cfg.ia_password = secrets.ia_password;
        }
        cfg
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        fs::write(path, bytes)?;
        write_secrets(self)
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

    /// True when both Internet Archive S3 keys are set.
    pub fn ia_keys_set(&self) -> bool {
        !self.ia_access_key.is_empty() && !self.ia_secret_key.is_empty()
    }

    /// Strip the Internet Archive credentials from any text headed for a log
    /// or an error message (aria2c echoes the request URL on failure).
    pub fn redact(&self, text: &str) -> String {
        let mut out = text.to_string();
        for secret in [&self.ia_secret_key, &self.ia_access_key] {
            if secret.len() >= 4 {
                out = out.replace(secret.as_str(), "***");
            }
        }
        out
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

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Secrets {
    #[serde(default)]
    rd_api_key: String,
    #[serde(default)]
    tb_api_key: String,
    #[serde(default)]
    ia_access_key: String,
    #[serde(default)]
    ia_secret_key: String,
    #[serde(default)]
    ia_email: String,
    #[serde(default)]
    ia_password: String,
}

fn read_secrets() -> Secrets {
    fs::read(secrets_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn write_secrets(cfg: &Config) -> Result<()> {
    let path = secrets_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let secrets = Secrets {
        rd_api_key: cfg.rd_api_key.clone(),
        tb_api_key: cfg.tb_api_key.clone(),
        ia_access_key: cfg.ia_access_key.clone(),
        ia_secret_key: cfg.ia_secret_key.clone(),
        ia_email: cfg.ia_email.clone(),
        ia_password: cfg.ia_password.clone(),
    };
    fs::write(&path, serde_json::to_vec_pretty(&secrets)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
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
        assert_eq!(cfg.video_preset, "veryslow");
    }

    #[test]
    fn ia_secrets_redact() {
        let mut cfg = Config::default();
        cfg.ia_access_key = "ACCESS1234".into();
        cfg.ia_secret_key = "SECRET9876".into();
        let text = "https://archive.org/download/x.iso?access_key=ACCESS1234&secret_key=SECRET9876";
        let clean = cfg.redact(text);
        assert!(!clean.contains("ACCESS1234"));
        assert!(!clean.contains("SECRET9876"));
        assert!(clean.contains("archive.org"));
        assert!(cfg.ia_keys_set());
    }
}
