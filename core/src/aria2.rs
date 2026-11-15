//! aria2c-powered downloads with 16-connection splitting, plus a plain HTTP
//! fallback when aria2c is not installed.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::transfer::JobContext;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find aria2c: explicit config path, app-bundled resources, then PATH.
pub fn find_aria2(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() {
        let p = PathBuf::from(configured);
        if p.is_file() {
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
    crate::media::which_bin("aria2c")
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
    config: &Config,
    ctx: &JobContext,
) -> Result<PathBuf> {
    std::fs::create_dir_all(dest_dir)?;
    let dest = dest_dir.join(dest_name);
    let dest_str = dest.to_string_lossy().to_string();
    ctx.message(if find_aria2(&config.ffmpeg_path).is_some() || find_aria2("").is_some() {
        "downloading (aria2c, 16 connections)"
    } else {
        "downloading (single connection; install aria2c for speed)"
    });

    if let Some(aria2) = find_aria2("").or_else(|| find_aria2(&config.ffmpeg_path)) {
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
    use std::io::BufRead;
    let mut child = Command::new(aria2)
        .args([
            "-x", "16", "-s", "16", "-k", "1M",
            "--file-allocation=none",
            "--console-log-level=warn",
            "--summary-interval=2",
            "--continue=true",
            "--auto-file-renaming=false",
            "--allow-overwrite=true",
            "-d",
        ])
        .arg(dest_dir)
        .arg("-o")
        .arg(Path::new(dest).file_name().expect("dest has file name"))
        .arg(url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Other(format!("aria2c failed to start: {e}")))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines().map_while(std::result::Result::ok) {
            if ctx.check_cancel().is_err() {
                let _ = child.kill();
                let _ = std::fs::remove_file(dest);
                return Err(Error::Cancelled(ctx.job_id()));
            }
            // console readout: `[#8f01 12MiB/1.5GiB(8%)CN:16 DL:3.2MiB]`
            if let (Some(open), Some(close)) = (line.rfind('('), line.rfind('%')) {
                if close > open {
                    if let Ok(pct) = line[open + 1..close].parse::<u64>() {
                        ctx.progress(pct, 100);
                    }
                }
            }
        }
    }
    let status = child.wait()?;
    if !status.success() {
        let stderr = child
            .stderr
            .map(|mut e| {
                use std::io::Read;
                let mut buf = String::new();
                let _ = e.read_to_string(&mut buf);
                buf
            })
            .unwrap_or_default();
        let tail: String = stderr.lines().rev().take(4).collect::<Vec<_>>().join("; ");
        // aria2 exits non-zero with --continue when the server lacks ranges;
        // fall back once to the single-connection path.
        if tail.contains("Range") || tail.contains("range") || tail.contains("416") {
            return run_fallback(url, Path::new(dest), ctx);
        }
        return Err(Error::Other(format!("aria2c exited badly: {tail}")));
    }
    let size = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    ctx.progress(size, size);
    Ok(Path::new(dest).to_path_buf())
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
}
