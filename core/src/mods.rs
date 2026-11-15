//! Per-game mods catalog: `<mods_base>/psp/mods/<SERIAL>.json` arrays of
//! mod entries; downloads land in the download dir under `mods/<SERIAL>/`.

use crate::config::Config;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    pub url: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub notes: String,
    /// `iso_patch` | `plugin` | `translation` | `cheat` | `other`
    #[serde(default = "default_kind")]
    pub kind: String,
}

fn default_kind() -> String {
    "other".into()
}

/// List mods for a serial. Empty list when no base is configured or the
/// catalog has no entry (404).
pub fn list(config: &Config, serial: &str) -> Result<Vec<ModEntry>> {
    if config.mods_base.is_empty() || serial.is_empty() {
        return Ok(Vec::new());
    }
    let url = format!(
        "{}/psp/mods/{}.json",
        config.mods_base.trim_end_matches('/'),
        crate::sources::urlencode(serial)
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(12))
        .build();
    let response = match agent.get(&url).call() {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => return Ok(Vec::new()),
        Err(e) => return Err(Error::Http(format!("mods fetch: {e}"))),
    };
    let entries: Vec<ModEntry> = response.into_json()?;
    Ok(entries)
}

/// Destination directory for a mod download.
pub fn mods_dir(config: &Config, serial: &str) -> std::path::PathBuf {
    crate::aria2::download_dir(config).join("mods").join(serial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mods_dir_layout() {
        let mut cfg = Config::default();
        cfg.download_dir = "/tmp/dl".into();
        assert_eq!(
            mods_dir(&cfg, "UCUS-98737"),
            std::path::PathBuf::from("/tmp/dl/mods/UCUS-98737")
        );
    }
}
