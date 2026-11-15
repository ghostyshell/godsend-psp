//! Music and video support: ffmpeg discovery plus PSP-format presets.
//!
//! PSP plays MP3/AAC/WMA music and MPEG-4 AVC video at 480x272 in an MP4
//! container. Pushing a file that is already compatible is a straight copy;
//! anything else goes through ffmpeg with a PSP preset.

use crate::error::{Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

pub const MUSIC_EXTS: &[&str] = &["mp3", "m4a", "aac", "wav", "ogg", "flac", "wma"];
pub const VIDEO_EXTS: &[&str] = &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "m4v"];

#[derive(Debug, Clone, Serialize)]
pub struct ToolInfo {
    pub found: bool,
    pub path: String,
    pub version: String,
}

/// Find ffmpeg: config override, then app-bundled resource, then PATH.
pub fn find_ffmpeg(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() {
        let p = PathBuf::from(configured);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in bundled_candidates("ffmpeg") {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    which("ffmpeg")
}

pub fn find_ffprobe(configured: &str) -> Option<PathBuf> {
    if !configured.is_empty() {
        let p = PathBuf::from(configured);
        if p.is_file() {
            return Some(p);
        }
    }
    for candidate in bundled_candidates("ffprobe") {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    which("ffprobe")
}

fn bundled_candidates(name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // macOS app bundle: .../Contents/MacOS/tool -> ../Resources/tool
            out.push(dir.join("../Resources").join(name));
            out.push(dir.join(name));
        }
    }
    out
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        if cfg!(windows) {
            let exe = dir.join(format!("{}.exe", name));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

/// `ffmpeg -version` first line, for the status bar.
pub fn version(bin: &Path) -> String {
    Command::new(bin)
        .arg("-version")
        .output()
        .ok()
        .and_then(|out| String::from_utf8_lossy(&out.stdout).lines().next().map(str::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

/// Probe media duration in seconds via ffprobe.
pub fn duration(ffprobe: &Path, file: &Path) -> Result<f64> {
    let out = Command::new(ffprobe)
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(file)
        .output()
        .map_err(|e| Error::Ffmpeg(format!("ffprobe failed to start: {}", e)))?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim()
        .parse::<f64>()
        .map_err(|_| Error::Ffmpeg(format!("no duration for {}", file.display())))
}

/// Parse `ffmpeg -progress pipe:1` `out_time_ms=` values into seconds.
pub fn parse_out_time_ms(line: &str) -> Option<f64> {
    line.strip_prefix("out_time_ms=")
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|micros| micros / 1_000_000.0)
}

#[derive(Debug, Clone, Copy)]
pub enum AudioFormat {
    Mp3,
    M4a,
}

impl AudioFormat {
    pub fn ext(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
        }
    }
}

/// Convert any audio input to PSP-friendly MP3/M4A.
pub fn convert_audio(
    ffmpeg: &Path,
    ffprobe: Option<&Path>,
    src: &Path,
    dst: &Path,
    format: &AudioFormat,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<()> {
    let args: &[&str] = match format {
        AudioFormat::Mp3 => &["-vn", "-c:a", "libmp3lame", "-b:a", "192k", "-ar", "44100"],
        AudioFormat::M4a => &["-vn", "-c:a", "aac", "-b:a", "192k", "-ar", "44100"],
    };
    let total_secs = ffprobe.and_then(|probe| duration(probe, src).ok()).unwrap_or(0.0);
    run_progress(ffmpeg, src, dst, args, total_secs, cancel, progress)
}

/// Convert any video input to PSP AVC 480x272 MP4 (+ optional 160x120 .THM).
pub fn convert_video(
    ffmpeg: &Path,
    ffprobe: Option<&Path>,
    src: &Path,
    dst: &Path,
    thm: Option<&Path>,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<()> {
    let args = "-c:v libx264 -profile:v main -level:v 3.0 -preset medium \
        -vf scale=480:272:force_original_aspect_ratio=decrease,pad=480:272:(ow-iw)/2:(oh-ih)/2 \
        -pix_fmt yuv420p -c:a aac -b:a 128k -movflags +faststart"
        .split_whitespace()
        .collect::<Vec<_>>();
    let total_secs = ffprobe.and_then(|probe| duration(probe, src).ok()).unwrap_or(0.0);
    run_progress(ffmpeg, src, dst, &args, total_secs, cancel, progress)?;
    if let Some(thm_path) = thm {
        let status = Command::new(ffmpeg)
            .args(["-y", "-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
            .arg(src)
            .args(["-frames:v", "1", "-vf", "scale=160:120"])
            .arg(thm_path)
            .status()
            .map_err(|e| Error::Ffmpeg(format!("thumbnail failed: {}", e)))?;
        if !status.success() {
            // Thumbnails are best-effort; a failure should not fail the push.
            let _ = std::fs::remove_file(thm_path);
        }
    }
    Ok(())
}

fn run_progress(
    ffmpeg: &Path,
    src: &Path,
    dst: &Path,
    encode_args: &[&str],
    total_secs: f64,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<()> {
    use std::io::BufRead;
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut child = Command::new(ffmpeg)
        .args(["-y", "-nostdin", "-hide_banner", "-loglevel", "error", "-progress", "pipe:1", "-i"])
        .arg(src)
        .args(encode_args)
        .arg(dst)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Ffmpeg(format!("ffmpeg failed to start: {}", e)))?;

    let total = if total_secs > 0.0 { total_secs as u64 } else { 0 };
    if let Some(stdout) = child.stdout.take() {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines().map_while(std::result::Result::ok) {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = child.kill();
                let _ = std::fs::remove_file(dst);
                return Err(Error::Cancelled(0));
            }
            if let Some(secs) = parse_out_time_ms(&line) {
                let done = (secs as u64).min(total);
                progress(done, total);
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
        let tail: String = stderr.lines().rev().take(3).collect::<Vec<_>>().join("; ");
        return Err(Error::Ffmpeg(format!(
            "ffmpeg exited with {} ({})",
            status.code().unwrap_or(-1),
            tail
        )));
    }
    let size = std::fs::metadata(dst).map(|m| m.len()).unwrap_or(0);
    progress(size, size);
    Ok(())
}

/// Is this file already PSP-playable music (skip conversion)?
pub fn is_psp_audio(path: &Path) -> bool {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "mp3" | "m4a" | "aac" | "wav")
}

/// Is this file already PSP-playable video (straight copy)?
pub fn is_psp_video(path: &Path) -> bool {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    ext == "mp4"
}

/// Sanitize a file name for FAT32 (the Memory Stick filesystem).
pub fn fat_safe_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let safe = matches!(ch,
            'a'..='z' | 'A'..='Z' | '0'..='9'
            | ' ' | '-' | '_' | '(' | ')' | '.' | '\'' | '!' | '&' | '+' | ',' | ';' | '=' | '@' | '[' | ']' | '~'
        );
        if safe {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim();
    let trimmed = if trimmed.is_empty() {
        "untitled"
    } else {
        trimmed
    };
    // All-underscore results (from input like "///") read as untitled too.
    if trimmed.chars().all(|c| c == '_' || c == '.') {
        return "untitled".to_string();
    }
    // FAT name limit: keep headroom for extensions.
    trimmed.chars().take(180).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_line_parse() {
        assert_eq!(parse_out_time_ms("out_time_ms=1500000"), Some(1.5));
        assert_eq!(parse_out_time_ms("frame=12"), None);
    }

    #[test]
    fn fat_safe() {
        assert_eq!(fat_safe_name("My: Song/2026?"), "My_ Song_2026_");
        assert_eq!(fat_safe_name("///"), "untitled");
    }

    #[test]
    fn psp_audio_check() {
        assert!(is_psp_audio(Path::new("/x/song.MP3")));
        assert!(!is_psp_audio(Path::new("/x/song.flac")));
        assert!(is_psp_video(Path::new("/x/v.mp4")));
        assert!(!is_psp_video(Path::new("/x/v.mkv")));
    }
}
