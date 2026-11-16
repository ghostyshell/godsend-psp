//! aria2c-powered downloads with 16-connection splitting, plus a plain HTTP
//! fallback when aria2c is not installed.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::transfer::JobContext;
use std::path::{Path, PathBuf};
use std::process::Command;

/// aria2 exits as soon as a torrent finishes instead of seeding at 99-100%.
const SEED_FLAGS: [&str; 3] = ["--seed-time=0", "--seed-ratio=0.0", "--summary-interval=1"];

/// One parsed aria2 console summary line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Aria2Summary {
    pub done: u64,
    pub total: u64,
    pub speed: u64,
    pub eta_secs: u64,
}

/// Parse a size token such as `1.2GiB`, `300KB` or `42B` into bytes.
fn parse_size(token: &str) -> Option<u64> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let split = token
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(token.len());
    let (number, unit) = token.split_at(split);
    let value = number.parse::<f64>().ok()?;
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
        _ => return None,
    };
    Some((value * mult).round() as u64)
}

/// Parse an aria2 ETA token: `12s`, `1m20s`, `1h2m`, `1h2m3s`.
fn parse_eta(token: &str) -> Option<u64> {
    let token = token.trim();
    if token.is_empty() || token.starts_with('-') {
        return None;
    }
    let mut secs = 0u64;
    let mut number = String::new();
    let mut any = false;
    for c in token.chars() {
        if c.is_ascii_digit() {
            number.push(c);
        } else {
            let value = number.parse::<u64>().ok()?;
            let mult = match c.to_ascii_lowercase() {
                's' => 1,
                'm' => 60,
                'h' => 3600,
                _ => return None,
            };
            secs += value * mult;
            number.clear();
            any = true;
        }
    }
    if !number.is_empty() || !any {
        return None;
    }
    Some(secs)
}

/// Parse an aria2 summary line, e.g.
/// `[#2089b0 1.2GiB/1.5GiB(78%)CN:16 DL:3.2MiB ETA:1m20s]`.
/// Returns None when the line does not carry a `<done>/<total>(<pct>%)` pair.
pub fn parse_summary(line: &str) -> Option<Aria2Summary> {
    let open = line.find('(')?;
    let close = open + line[open..].find(')')?;
    let segment = line[..open]
        .rsplit(|c: char| c.is_whitespace())
        .next()
        .unwrap_or("");
    let (done_token, total_token) = segment.split_once('/')?;
    let done = parse_size(done_token)?;
    let total = parse_size(total_token)?;
    if total == 0 {
        return None;
    }
    let rest = &line[close + 1..];
    let speed = rest
        .find("DL:")
        .and_then(|pos| rest[pos + 3..].split(|c: char| c.is_whitespace() || c == ']').next())
        .and_then(parse_size)
        .unwrap_or(0);
    let eta_secs = rest
        .find("ETA:")
        .and_then(|pos| rest[pos + 4..].split(|c: char| c.is_whitespace() || c == ']').next())
        .and_then(parse_eta)
        .unwrap_or(0);
    Some(Aria2Summary {
        done,
        total,
        speed,
        eta_secs,
    })
}

/// Find aria2c: explicit path, app-bundled resources, PATH, then the usual
/// package-manager locations. GUI apps on macOS do not inherit the shell
/// PATH, so `/opt/homebrew/bin` must be checked explicitly.
pub fn find_aria2(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() {
        let p = PathBuf::from(configured);
        if p.is_file() && is_aria2(&p) {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let bundled = dir.join("../Resources/aria2c");
            if bundled.is_file() {
                return Some(bundled);
            }
            let beside = dir.join("aria2c");
            if beside.is_file() {
                return Some(beside);
            }
        }
    }
    if let Some(found) = crate::media::which_bin("aria2c") {
        if is_aria2(&found) {
            return Some(found);
        }
    }
    let name = if cfg!(windows) { "aria2c.exe" } else { "aria2c" };
    for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/snap/bin"] {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Guard against a wrong binary being passed in (the old code handed the
/// configured ffmpeg path to the aria2 lookup).
fn is_aria2(path: &Path) -> bool {
    path.file_name()
        .map(|name| name.to_string_lossy().to_lowercase().contains("aria2"))
        .unwrap_or(false)
}

/// Default download directory: config override, else `<Downloads>/godsend-psp`.
pub fn download_dir(config: &Config) -> PathBuf {
    if !config.download_dir.is_empty() {
        return PathBuf::from(&config.download_dir);
    }
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
    let downloads = if home.is_empty() {
        std::env::temp_dir()
    } else {
        PathBuf::from(&home).join("Downloads")
    };
    downloads.join("godsend-psp")
}

/// Download `url` into `dest` with aria2c when available (16 connections),
/// otherwise a single-connection ureq stream. Reports progress and honors
/// cancellation. Returns the final file path.
pub fn download(
    url: &str,
    dest_dir: &Path,
    dest_name: &str,
    _config: &Config,
    ctx: &JobContext,
) -> Result<PathBuf> {
    std::fs::create_dir_all(dest_dir)?;
    let dest = dest_dir.join(dest_name);
    let dest_str = dest.to_string_lossy().to_string();
    ctx.message(if find_aria2("").is_some() {
        "downloading (aria2c, 16 connections)"
    } else {
        "downloading (single connection; install aria2c for speed)"
    });

    if let Some(aria2) = find_aria2("") {
        return run_aria2(&aria2, url, dest_dir, &dest_str, ctx);
    }
    run_fallback(url, &dest, ctx)
}

fn run_aria2(
    aria2: &Path,
    url: &str,
    dest_dir: &Path,
    dest: &str,
    ctx: &JobContext,
) -> Result<PathBuf> {
    let mut command = base_command(aria2, dest_dir);
    // A .torrent or magnet passed here is a whole-torrent download, so exit
    // the moment the payload completes instead of seeding.
    if url.starts_with("magnet:") || url.ends_with(".torrent") || url.contains("/torrent/") {
        command.args(SEED_FLAGS);
    }
    command
        .arg("-o")
        .arg(Path::new(dest).file_name().expect("dest has file name"))
        .arg(url);

    let dest_path = Path::new(dest);
    let (status, tail) = spawn_and_watch(command, Some(dest_path), ctx)?;
    if !status.success() {
        // aria2 exits non-zero with --continue when the server lacks ranges;
        // fall back once to the single-connection path.
        if tail.contains("Range") || tail.contains("range") || tail.contains("416") {
            return run_fallback(url, dest_path, ctx);
        }
        return Err(Error::Other(format!("aria2c exited badly: {tail}")));
    }
    let size = std::fs::metadata(dest_path).map(|m| m.len()).unwrap_or(0);
    ctx.progress(size, size);
    Ok(dest_path.to_path_buf())
}

/// Download one file out of a torrent URL. aria2 fetches the torrent, picks
/// the entry with `select_file` (1-based, as reported by `torrent::parse_files`)
/// and writes it under `dest_dir` keeping the torrent's folder layout.
/// Returns `<dest_dir>/<expect_path>` when it landed there, else the basename.
pub fn download_selected(
    url: &str,
    dest_dir: &Path,
    expect_path: &str,
    select_file: u32,
    _config: &Config,
    ctx: &JobContext,
) -> Result<PathBuf> {
    std::fs::create_dir_all(dest_dir)?;
    let aria2 = find_aria2("").ok_or_else(|| {
        Error::Other(
            "aria2c is required to download one file from a torrent. Install it (brew install aria2 or apt install aria2) or put it in /opt/homebrew/bin, /usr/local/bin or /usr/bin."
                .into(),
        )
    })?;
    ctx.message("downloading one file (aria2c torrent selection)");

    let mut command = base_command(&aria2, dest_dir);
    command.args(SEED_FLAGS);
    command
        .arg("--follow-torrent=mem")
        .arg(format!("--select-file={select_file}"))
        .arg(url);
    let (status, tail) = spawn_and_watch(command, None, ctx)?;
    if !status.success() {
        return Err(Error::Other(format!("aria2c exited badly: {tail}")));
    }

    let basename = Path::new(expect_path)
        .file_name()
        .map(|name| name.to_owned())
        .unwrap_or_default();
    let found = safe_relative(expect_path)
        .map(|rel| dest_dir.join(rel))
        .filter(|candidate| candidate.is_file())
        .or_else(|| find_file(dest_dir, &basename, 4))
        .ok_or_else(|| {
            Error::Other(format!(
                "aria2c finished but {} was not found under {}",
                basename.to_string_lossy(),
                dest_dir.display()
            ))
        })?;
    // aria2 keeps the torrent's folder tree. Move the single game up so it
    // lands flat in the download folder and shows on the Downloads page.
    let flat = dest_dir.join(&basename);
    if found != flat {
        std::fs::rename(&found, &flat)?;
        let mut parent = found.parent();
        while let Some(dir) = parent {
            if dir == dest_dir || !dir.starts_with(dest_dir) || std::fs::remove_dir(dir).is_err() {
                break;
            }
            parent = dir.parent();
        }
    }
    let size = std::fs::metadata(&flat).map(|m| m.len()).unwrap_or(0);
    ctx.progress(size, size);
    Ok(flat)
}

/// Case-sensitive search for a file name below `dir`, depth limited.
fn find_file(dir: &Path, name: &std::ffi::OsStr, depth: usize) -> Option<PathBuf> {
    if depth == 0 || name.is_empty() {
        return None;
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name() == Some(name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, name, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// The shared aria2c flags for HTTP and torrent downloads. The caller adds
/// the destination name or selection flags and the URL.
fn base_command(aria2: &Path, dest_dir: &Path) -> Command {
    let mut command = Command::new(aria2);
    command.args([
        "-x", "16", "-s", "16", "-k", "1M",
        "--file-allocation=none",
        "--console-log-level=warn",
        "--summary-interval=2",
        "--continue=true",
        "--auto-file-renaming=false",
        "--allow-overwrite=true",
    ]);
    command.arg("-d").arg(dest_dir);
    command
}

/// Run aria2c, relaying percentage readouts and honoring cancellation.
/// Returns the exit status and the last few stderr lines.
fn spawn_and_watch(
    mut command: Command,
    cleanup: Option<&Path>,
    ctx: &JobContext,
) -> Result<(std::process::ExitStatus, String)> {
    use std::io::BufRead;
    command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| Error::Other(format!("aria2c failed to start: {e}")))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines().map_while(std::result::Result::ok) {
            if ctx.check_cancel().is_err() {
                let _ = child.kill();
                if let Some(path) = cleanup {
                    let _ = std::fs::remove_file(path);
                }
                return Err(Error::Cancelled(ctx.job_id()));
            }
            // console readout: `[#8f01 12MiB/1.5GiB(8%)CN:16 DL:3.2MiB ETA:1m20s]`
            if let Some(summary) = parse_summary(&line) {
                ctx.metrics(summary.done, summary.total, summary.speed, summary.eta_secs);
            } else if let (Some(open), Some(close)) = (line.rfind('('), line.rfind('%')) {
                // Fallback for readouts that carry a percentage but no bytes.
                if close > open {
                    if let Ok(pct) = line[open + 1..close].parse::<u64>() {
                        ctx.progress(pct, 100);
                    }
                }
            }
        }
    }
    let mut tail = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        use std::io::Read;
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        tail = buf.lines().rev().take(4).collect::<Vec<_>>().join("; ");
    }
    Ok((child.wait()?, tail))
}

/// Reject absolute paths and `..` so a torrent path cannot escape dest_dir.
fn safe_relative(path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split(['/', '\\']) {
        let part = part.trim();
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        out.push(part);
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn run_fallback(url: &str, dest: &Path, ctx: &JobContext) -> Result<PathBuf> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(3600))
        .build();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| Error::Http(format!("download failed: {e}")))?;
    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let mut reader = response.into_reader();
    let mut out = std::fs::File::create(dest)?;
    let mut buf = [0u8; 256 * 1024];
    let mut done: u64 = 0;
    let mut last = std::time::Instant::now();
    loop {
        ctx.check_cancel()?;
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut out, &buf[..read])?;
        done += read as u64;
        if last.elapsed().as_millis() > 200 {
            ctx.progress(done, total);
            last = std::time::Instant::now();
        }
    }
    ctx.progress(done, total);
    Ok(dest.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_dir_default() {
        let mut cfg = Config::default();
        cfg.download_dir = "/tmp/custom-dl".into();
        assert_eq!(download_dir(&cfg), PathBuf::from("/tmp/custom-dl"));
        cfg.download_dir = String::new();
        assert!(download_dir(&cfg).ends_with("godsend-psp"));
    }

    #[test]
    fn selected_paths_stay_inside_the_download_dir() {
        assert_eq!(
            safe_relative("Redump/Sony - PSP/007 - Game.zip"),
            Some(PathBuf::from("Redump/Sony - PSP/007 - Game.zip"))
        );
        assert_eq!(safe_relative("/etc/passwd"), Some(PathBuf::from("etc/passwd")));
        assert_eq!(safe_relative("../../escape.zip"), Some(PathBuf::from("escape.zip")));
        assert_eq!(safe_relative(""), None);
        assert_eq!(safe_relative("/"), None);
    }

    #[test]
    fn summary_line_parses_mib_and_eta() {
        let summary =
            parse_summary("[#2089b0 1.2GiB/1.5GiB(78%)CN:16 DL:3.2MiB ETA:1m20s]").expect("summary");
        assert_eq!(summary.done, 1_288_490_189);
        assert_eq!(summary.total, 1_610_612_736);
        assert_eq!(summary.speed, 3_355_443);
        assert_eq!(summary.eta_secs, 80);
    }

    #[test]
    fn summary_line_without_eta_keeps_speed() {
        let summary = parse_summary("[#2089b0 123MiB/456MiB(27%)CN:16 DL:1.0MiB]").expect("summary");
        assert_eq!(summary.done, 128_974_848);
        assert_eq!(summary.total, 478_150_656);
        assert_eq!(summary.speed, 1_048_576);
        assert_eq!(summary.eta_secs, 0);
    }

    #[test]
    fn summary_line_parses_plain_kb() {
        let summary = parse_summary("[#8f01 1200KB/1500KB(80%)CN:4 DL:300KB ETA:12s]").expect("summary");
        assert_eq!(summary.done, 1_200_000);
        assert_eq!(summary.total, 1_500_000);
        assert_eq!(summary.speed, 300_000);
        assert_eq!(summary.eta_secs, 12);
        assert_eq!(parse_eta("1h2m"), Some(3720));
        assert!(parse_summary("no progress here").is_none());
    }

    #[test]
    fn aria2_guard_rejects_other_binaries() {
        assert!(is_aria2(Path::new("/opt/homebrew/bin/aria2c")));
        assert!(is_aria2(Path::new("C:/tools/aria2c.exe")));
        assert!(!is_aria2(Path::new("/opt/homebrew/bin/ffmpeg")));
        assert!(!is_aria2(Path::new("/usr/bin/curl")));
    }
}
