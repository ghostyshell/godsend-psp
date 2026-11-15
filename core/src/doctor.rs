//! Environment checks surfaced in Settings and the status bar.

use crate::config::Config;
use crate::media;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Run every environment check.
pub fn run(config: &Config) -> Vec<Check> {
    let mut out = Vec::new();

    // Data dir
    let data_ok = crate::config::ensure_dirs().is_ok();
    out.push(Check {
        name: "data dir".into(),
        ok: data_ok,
        detail: crate::config::data_dir().to_string_lossy().to_string(),
    });

    // ffmpeg
    match media::find_ffmpeg(&config.ffmpeg_path) {
        Some(path) => out.push(Check {
            name: "ffmpeg".into(),
            ok: true,
            detail: media::version(&path),
        }),
        None => out.push(Check {
            name: "ffmpeg".into(),
            ok: false,
            detail: "not found; music/video conversion disabled (installs and copies still work)".into(),
        }),
    }

    // ffprobe
    let probe_ok = media::find_ffprobe(&config.ffprobe_path).is_some();
    out.push(Check {
        name: "ffprobe".into(),
        ok: probe_ok,
        detail: if probe_ok {
            "found".into()
        } else {
            "not found; conversion progress will be coarse".into()
        },
    });

    // Device
    let volumes = crate::device::psp_volumes();
    out.push(Check {
        name: "device".into(),
        ok: !volumes.is_empty(),
        detail: if volumes.is_empty() {
            "no PSP in USB mode detected".into()
        } else {
            volumes
                .iter()
                .map(|v| v.mount.clone())
                .collect::<Vec<_>>()
                .join(", ")
        },
    });

    // Catalog cache
    let cache = crate::config::catalog_cache_path();
    let cached = std::fs::metadata(&cache).ok();
    out.push(Check {
        name: "catalog cache".into(),
        ok: cached.is_some(),
        detail: match cached {
            Some(meta) => {
                let age = meta
                    .modified()
                    .ok()
                    .and_then(|m| m.elapsed().ok())
                    .map(|d| format!("{}h ago", d.as_secs() / 3600))
                    .unwrap_or_else(|| "unknown age".into());
                format!("cached ({})", age)
            }
            None => "empty; refresh on first catalog open".into(),
        },
    });

    out
}

/// True when every check passes.
pub fn all_ok(checks: &[Check]) -> bool {
    checks.iter().all(|c| c.ok)
}
