//! In-app update check, verified download and guided install.
//!
//! The release feed (`downloads.json`) lists every artifact with the free-host
//! links plus a direct R2 mirror. Only the R2 copy is a plain binary download;
//! GoFile and file.kiwi serve HTML pages, so they stay the manual fallback for
//! the website and the update path only uses them when R2 is missing.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::transfer::JobContext;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Canonical release feed. Pages caches for ten minutes, so requests carry a
/// cache-busting query.
pub const UPDATE_FEED: &str = "https://ghostyshell.github.io/godsend-psp/downloads.json";

/// One platform build from the feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateArtifact {
    pub name: String,
    pub label: String,
    pub size_bytes: u64,
    pub sha256: String,
    /// Direct download (R2 when present, else file.kiwi, else GoFile).
    pub url: String,
    /// Second direct host to try when `url` fails.
    pub fallback_url: String,
}

/// Result of comparing the running version against the feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub artifact: Option<UpdateArtifact>,
}

#[derive(Debug, Deserialize)]
struct RawFeed {
    #[serde(default)]
    version: String,
    #[serde(default)]
    artifacts: Vec<RawArtifact>,
}

#[derive(Debug, Deserialize)]
struct RawArtifact {
    #[serde(default)]
    name: String,
    #[serde(default)]
    label: String,
    /// Human-readable size from the feed (`44.3 MiB`).
    #[serde(default)]
    size: String,
    /// Exact byte count when the publisher writes it.
    #[serde(default)]
    bytes: Option<u64>,
    #[serde(default)]
    sha256: String,
    #[serde(default)]
    gofile: String,
    #[serde(default)]
    kiwi: String,
    #[serde(default)]
    r2: String,
}

struct Feed {
    version: String,
    artifacts: Vec<UpdateArtifact>,
}

/// Fetch the feed and compare it with `current`.
pub fn check(current: &str) -> Result<UpdateInfo> {
    check_from(
        UPDATE_FEED,
        current,
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

fn check_from(feed_url: &str, current: &str, os: &str, arch: &str) -> Result<UpdateInfo> {
    let body = fetch(feed_url)?;
    let feed = parse_feed(&body)?;
    let latest = feed.version.trim().to_string();
    let available = compare_versions(&latest, current) == Ordering::Greater;
    let artifact = if available {
        pick_artifact(&feed.artifacts, &latest, os, arch)
    } else {
        None
    };
    Ok(UpdateInfo {
        current: current.to_string(),
        latest,
        available,
        artifact,
    })
}

fn fetch(url: &str) -> Result<String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let separator = if url.contains('?') { '&' } else { '?' };
    let busted = format!("{url}{separator}v={stamp}");
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    let response = agent
        .get(&busted)
        .set("Cache-Control", "no-cache")
        .call()
        .map_err(|e| Error::Http(format!("update feed fetch failed: {e}")))?;
    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .map_err(|e| Error::Http(format!("update feed read failed: {e}")))?;
    Ok(body)
}

fn parse_feed(json: &str) -> Result<Feed> {
    let raw: RawFeed = serde_json::from_str(json)?;
    if raw.version.trim().is_empty() {
        return Err(Error::Invalid("update feed carries no version".into()));
    }
    let mut artifacts = Vec::new();
    for entry in raw.artifacts {
        let name = entry.name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        // The direct mirror order: R2 first, then file.kiwi, then GoFile.
        let mut direct = [&entry.r2, &entry.kiwi, &entry.gofile]
            .into_iter()
            .map(|u| u.trim())
            .filter(|u| !u.is_empty());
        let Some(url) = direct.next() else { continue };
        let fallback_url = direct.next().unwrap_or("").to_string();
        let size_bytes = match entry.bytes {
            Some(bytes) if bytes > 0 => bytes,
            _ => parse_human_size(&entry.size),
        };
        artifacts.push(UpdateArtifact {
            name,
            label: entry.label,
            size_bytes,
            sha256: entry.sha256.trim().to_ascii_lowercase(),
            url: url.to_string(),
            fallback_url,
        });
    }
    Ok(Feed {
        version: raw.version,
        artifacts,
    })
}

/// Parse `44.3 MiB` / `2.1 GB` / `980 KB` into a rounded byte count.
fn parse_human_size(text: &str) -> u64 {
    let text = text.trim();
    if text.is_empty() {
        return 0;
    }
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let Ok(value) = number.parse::<f64>() else {
        return 0;
    };
    let mult = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "kb" => 1_000.0,
        "kib" => 1024.0,
        "mb" => 1_000_000.0,
        "mib" => 1024.0 * 1024.0,
        "gb" => 1_000_000_000.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        "tb" => 1_000_000_000_000.0,
        "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return 0,
    };
    (value * mult).round() as u64
}

/// Pick the build for a platform. Names come from the release script, for
/// example `godsend-psp-0.2.46-arm64.dmg`.
pub fn pick_artifact(
    artifacts: &[UpdateArtifact],
    version: &str,
    os: &str,
    arch: &str,
) -> Option<UpdateArtifact> {
    let arch = arch.to_ascii_lowercase();
    let os = os.to_ascii_lowercase();
    let suffixes: &[&str] = match (os.as_str(), arch.as_str()) {
        ("macos" | "darwin", "aarch64" | "arm64") => &["-arm64.dmg"],
        ("macos" | "darwin", _) => &["-x64.dmg"],
        ("windows" | "win32", _) => &["-x64-setup.exe"],
        ("linux", _) => &["-linux-x64.appimage", "-linux-x64.deb"],
        _ => return None,
    };
    for suffix in suffixes {
        let suffix = suffix.to_ascii_lowercase();
        let best = artifacts
            .iter()
            .filter(|a| a.name.to_ascii_lowercase().ends_with(&suffix))
            .max_by_key(|a| a.name.contains(version));
        if let Some(found) = best {
            return Some(found.clone());
        }
    }
    None
}

/// Compare dotted versions: `0.2.9` sorts before `0.2.10`.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let left = version_parts(a);
    let right = version_parts(b);
    let len = left.len().max(right.len());
    for i in 0..len {
        let l = left.get(i).copied().unwrap_or(0);
        let r = right.get(i).copied().unwrap_or(0);
        match l.cmp(&r) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

fn version_parts(text: &str) -> Vec<u64> {
    let trimmed = text.trim().trim_start_matches(['v', 'V']);
    let mut parts = Vec::new();
    for chunk in trimmed.split(|c: char| !c.is_ascii_digit()) {
        if chunk.is_empty() {
            continue;
        }
        parts.push(chunk.parse::<u64>().unwrap_or(0));
    }
    parts
}

/// Download `artifact` into `dest_dir`, retrying the fallback mirror, then
/// verify the size and SHA-256. A mismatch deletes the file.
pub fn download_verified(
    artifact: &UpdateArtifact,
    dest_dir: &Path,
    ctx: &JobContext,
) -> Result<PathBuf> {
    let config = Config::default();
    let mut attempt = crate::aria2::download(
        &artifact.url,
        dest_dir,
        &artifact.name,
        &config,
        ctx,
    );
    if attempt.is_err() && !artifact.fallback_url.is_empty() {
        ctx.message("primary mirror failed; trying the fallback");
        attempt = crate::aria2::download(
            &artifact.fallback_url,
            dest_dir,
            &artifact.name,
            &config,
            ctx,
        );
    }
    let path = attempt?;

    ctx.message("verifying the download");
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    // Feed sizes are rounded to 0.1 MiB, so allow a one-percent band. When the
    // publisher ships an exact `bytes` value the check is effectively exact.
    if artifact.size_bytes > 0 {
        let diff = size.abs_diff(artifact.size_bytes);
        let slack = artifact.size_bytes / 100;
        if diff > slack {
            let _ = std::fs::remove_file(&path);
            return Err(Error::Invalid(format!(
                "{} is {size} bytes but the feed lists {}",
                artifact.name, artifact.size_bytes
            )));
        }
    }

    let digest = sha256_file(&path)?;
    if !artifact.sha256.is_empty() && digest != artifact.sha256 {
        let _ = std::fs::remove_file(&path);
        return Err(Error::Invalid(format!(
            "{} failed sha256 verification (got {digest}, expected {})",
            artifact.name, artifact.sha256
        )));
    }
    ctx.message(format!("verified {}", artifact.name));
    Ok(path)
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex(&hasher.finish()))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Lowercase hex SHA-256 of `data`.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex(&hasher.finish())
}

/// Minimal SHA-256 so verification needs no crate.
struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffered: usize,
    length_bits: u64,
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
    0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
    0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
    0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
    0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
    0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
                0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: [0u8; 64],
            buffered: 0,
            length_bits: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.length_bits = self
            .length_bits
            .wrapping_add((data.len() as u64).wrapping_mul(8));
        if self.buffered > 0 {
            let take = (64 - self.buffered).min(data.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
            self.buffered += take;
            data = &data[take..];
            if self.buffered == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.compress(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buffer[..data.len()].copy_from_slice(data);
            self.buffered = data.len();
        }
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0u32; 64];
        for (i, word) in words.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = words[i - 15].rotate_right(7)
                ^ words[i - 15].rotate_right(18)
                ^ (words[i - 15] >> 3);
            let s1 = words[i - 2].rotate_right(17)
                ^ words[i - 2].rotate_right(19)
                ^ (words[i - 2] >> 10);
            words[i] = words[i - 16]
                .wrapping_add(s0)
                .wrapping_add(words[i - 7])
                .wrapping_add(s1);
        }

        let mut h = self.state;
        for (i, word) in words.iter().enumerate() {
            let s1 = h[4].rotate_right(6) ^ h[4].rotate_right(11) ^ h[4].rotate_right(25);
            let ch = (h[4] & h[5]) ^ ((!h[4]) & h[6]);
            let temp1 = h[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(*word);
            let s0 = h[0].rotate_right(2) ^ h[0].rotate_right(13) ^ h[0].rotate_right(22);
            let maj = (h[0] & h[1]) ^ (h[0] & h[2]) ^ (h[1] & h[2]);
            let temp2 = s0.wrapping_add(maj);
            h[7] = h[6];
            h[6] = h[5];
            h[5] = h[4];
            h[4] = h[3].wrapping_add(temp1);
            h[3] = h[2];
            h[2] = h[1];
            h[1] = h[0];
            h[0] = temp1.wrapping_add(temp2);
        }
        for i in 0..8 {
            self.state[i] = self.state[i].wrapping_add(h[i]);
        }
    }

    fn finish(mut self) -> [u8; 32] {
        let length_bits = self.length_bits;
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0x00]);
        }
        // `update` counted the padding; write the original bit length by hand.
        self.buffer[56..64].copy_from_slice(&length_bits.to_be_bytes());
        let block = self.buffer;
        self.compress(&block);
        let mut out = [0u8; 32];
        for (i, value) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
    {
      "version": "0.2.47",
      "artifacts": [
        {
          "name": "godsend-psp-0.2.47-arm64.dmg",
          "label": "macOS (Apple Silicon)",
          "size": "44.3 MiB",
          "bytes": 46497151,
          "sha256": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
          "gofile": "https://gofile.io/d/x",
          "kiwi": "https://file.kiwi/x#y",
          "r2": "https://covers.debrid.in/psp/releases/0.2.47/godsend-psp-0.2.47-arm64.dmg"
        },
        {
          "name": "godsend-psp-0.2.47-x64.dmg",
          "label": "macOS (Intel)",
          "size": "59.2 MiB",
          "sha256": "BBBB",
          "gofile": "https://gofile.io/d/y",
          "kiwi": "https://file.kiwi/y#z"
        },
        {
          "name": "godsend-psp-0.2.47-x64-setup.exe",
          "label": "Windows (x86_64, installer)",
          "size": "45.1 MiB",
          "sha256": "CCCC",
          "gofile": "https://gofile.io/d/w",
          "kiwi": "https://file.kiwi/w#v",
          "r2": "https://covers.debrid.in/psp/releases/0.2.47/godsend-psp-0.2.47-x64-setup.exe"
        },
        {
          "name": "godsend-psp-0.2.47-linux-x64.AppImage",
          "label": "Linux (x86_64, AppImage)",
          "size": "131.9 MiB",
          "sha256": "DDDD",
          "gofile": "https://gofile.io/d/a",
          "kiwi": "https://file.kiwi/a#b",
          "r2": "https://covers.debrid.in/psp/releases/0.2.47/godsend-psp-0.2.47-linux-x64.AppImage"
        },
        {
          "name": "godsend-psp-0.2.47-linux-x64.deb",
          "label": "Linux (x86_64, deb)",
          "size": "62.8 MiB",
          "sha256": "EEEE",
          "gofile": "https://gofile.io/d/c",
          "kiwi": "https://file.kiwi/c#d"
        }
      ]
    }
    "#;

    fn artifact(name: &str) -> UpdateArtifact {
        UpdateArtifact {
            name: name.into(),
            label: String::new(),
            size_bytes: 1,
            sha256: String::new(),
            url: "https://example.invalid/direct".into(),
            fallback_url: String::new(),
        }
    }

    #[test]
    fn versions_sort_numerically() {
        assert_eq!(compare_versions("0.2.9", "0.2.10"), Ordering::Less);
        assert_eq!(compare_versions("0.2.10", "0.2.9"), Ordering::Greater);
        assert_eq!(compare_versions("0.2.47", "0.2.47"), Ordering::Equal);
        assert_eq!(compare_versions("v0.3", "0.2.99"), Ordering::Greater);
    }

    #[test]
    fn feed_parse_prefers_r2_and_falls_back_to_kiwi() {
        let feed = parse_feed(FIXTURE).expect("fixture parses");
        assert_eq!(feed.version, "0.2.47");
        assert_eq!(feed.artifacts.len(), 5);
        let mac = &feed.artifacts[0];
        assert_eq!(
            mac.url,
            "https://covers.debrid.in/psp/releases/0.2.47/godsend-psp-0.2.47-arm64.dmg"
        );
        assert_eq!(mac.fallback_url, "https://file.kiwi/x#y");
        assert_eq!(mac.size_bytes, 46497151);
        // No r2: the direct url is the kiwi mirror and GoFile is the fallback.
        let intel = &feed.artifacts[1];
        assert_eq!(intel.url, "https://file.kiwi/y#z");
        assert_eq!(intel.fallback_url, "https://gofile.io/d/y");
        assert_eq!(intel.size_bytes, parse_human_size("59.2 MiB"));
    }

    #[test]
    fn artifact_picking_covers_every_target() {
        let feed = parse_feed(FIXTURE).expect("fixture parses");
        let mac_arm = pick_artifact(&feed.artifacts, "0.2.47", "macos", "aarch64").expect("arm");
        assert_eq!(mac_arm.name, "godsend-psp-0.2.47-arm64.dmg");
        let mac_x64 = pick_artifact(&feed.artifacts, "0.2.47", "macos", "x86_64").expect("intel");
        assert_eq!(mac_x64.name, "godsend-psp-0.2.47-x64.dmg");
        let win = pick_artifact(&feed.artifacts, "0.2.47", "windows", "x86_64").expect("win");
        assert_eq!(win.name, "godsend-psp-0.2.47-x64-setup.exe");
        let linux =
            pick_artifact(&feed.artifacts, "0.2.47", "linux", "x86_64").expect("linux appimage");
        assert_eq!(linux.name, "godsend-psp-0.2.47-linux-x64.AppImage");
        // Only the deb present: the AppImage match misses and the deb wins.
        let deb_only = vec![artifact("godsend-psp-0.2.47-linux-x64.deb")];
        let deb = pick_artifact(&deb_only, "0.2.47", "linux", "x86_64").expect("deb");
        assert_eq!(deb.name, "godsend-psp-0.2.47-linux-x64.deb");
        assert!(pick_artifact(&feed.artifacts, "0.2.47", "freebsd", "x86_64").is_none());
    }

    #[test]
    fn newer_current_is_not_an_update() {
        assert_eq!(compare_versions("0.2.47", "0.2.48"), Ordering::Less);
        let feed = parse_feed(FIXTURE).expect("fixture parses");
        let latest = feed.version.clone();
        assert_eq!(compare_versions(&latest, "0.2.48"), Ordering::Less);
    }

    #[test]
    fn sha256_matches_the_test_vectors() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn human_sizes_parse() {
        assert_eq!(parse_human_size("44.3 MiB"), 46_451_917);
        assert_eq!(parse_human_size("2.1 GiB"), 2_254_857_830);
        assert_eq!(parse_human_size("980 KB"), 980_000);
        assert_eq!(parse_human_size(""), 0);
    }

    /// Live feed check. Run with `cargo test -p godsend-core -- --ignored`.
    #[test]
    #[ignore]
    fn live_feed_reports_the_latest_release() {
        let info = check("0.0.1").expect("live feed must answer");
        println!(
            "current={} latest={} available={} artifact={}",
            info.current,
            info.latest,
            info.available,
            info.artifact
                .as_ref()
                .map(|a| a.name.as_str())
                .unwrap_or("<none>")
        );
        assert!(!info.latest.is_empty());
        assert!(info.available);
        assert!(info.artifact.is_some(), "a build must exist for this host");
    }
}
