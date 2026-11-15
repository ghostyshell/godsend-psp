//! Save game management: list PSP/SAVEDATA folders, zip backups, restores.

use crate::device::PspPaths;
use crate::error::{Error, Result};
use crate::pbp::{Pbp, Section};
use crate::sfo::Sfo;
use crate::transfer::JobContext;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Save folders that are actually CFW payloads, not saves.
const NON_SAVE_FOLDERS: &[&str] = &["ARK_01234"];

#[derive(Debug, Clone, Serialize)]
pub struct SaveEntry {
    /// Folder name on the stick.
    pub folder: String,
    /// SFO TITLE (game name).
    pub title: Option<String>,
    /// SFO SUBTITLE (save slot description).
    pub subtitle: Option<String>,
    pub title_id: Option<String>,
    pub size_bytes: u64,
    pub has_icon: bool,
}

pub fn list(paths: &PspPaths) -> Result<Vec<SaveEntry>> {
    let dir = paths.savedata();
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let mut folders: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    for folder in folders {
        let name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if NON_SAVE_FOLDERS.iter().any(|f| name.eq_ignore_ascii_case(f)) {
            continue;
        }
        let mut entry = SaveEntry {
            folder: name.clone(),
            title: None,
            subtitle: None,
            title_id: None,
            size_bytes: dir_size(&folder),
            has_icon: folder.join("ICON0.PNG").is_file(),
        };
        if let Ok(bytes) = fs::read(folder.join("PARAM.SFO")) {
            if let Ok(sfo) = Sfo::parse(&bytes) {
                entry.title = sfo.title().map(str::to_owned);
                entry.subtitle = sfo.string("SUBTITLE").map(str::to_owned);
                entry.title_id = sfo.title_id().map(str::to_owned);
            }
        }
        // Savedata written by homebrew may store a PBP-shaped PARAM inside ICON.
        if entry.title.is_none() {
            if let Ok(bytes) = fs::read(folder.join("SAVEICON.PNG")) {
                let _ = bytes;
            }
        }
        out.push(entry);
    }
    Ok(out)
}

/// Zip the given save folders (empty list = all) into `dest_zip`.
pub fn backup(paths: &PspPaths, folders: &[String], dest_zip: &Path, ctx: Option<&JobContext>) -> Result<u64> {
    paths.validate()?;
    let all = if folders.is_empty() {
        list(paths)?
            .into_iter()
            .map(|e| e.folder)
            .collect::<Vec<_>>()
    } else {
        folders.to_vec()
    };
    if all.is_empty() {
        return Err(Error::NotFound("no save folders to back up".into()));
    }

    let file = fs::File::create(dest_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'static, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Pre-size the job.
    let mut total = 0u64;
    for folder in &all {
        total += dir_size(&paths.savedata().join(folder));
    }
    let mut done = 0u64;
    for folder in &all {
        let src_dir = paths.savedata().join(folder);
        if !src_dir.is_dir() {
            continue;
        }
        let mut stack = vec![src_dir.clone()];
        while let Some(dir) = stack.pop() {
            if let Some(ctx) = ctx {
                ctx.check_cancel()?;
            }
            for entry in fs::read_dir(&dir)?.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name.starts_with("._") {
                    continue;
                }
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let arc = path
                    .strip_prefix(&src_dir)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or(name);
                let zip_name = format!("{}/{}", folder, arc.replace('\\', "/"));
                zip.start_file(zip_name.as_str(), options.clone())?;
                let mut f = fs::File::open(&path)?;
                let mut buf = vec![0u8; 256 * 1024];
                loop {
                    let read = f.read(&mut buf)?;
                    if read == 0 {
                        break;
                    }
                    zip.write_all(&buf[..read])?;
                    done += read as u64;
                    if let Some(ctx) = ctx {
                        ctx.progress(done, total);
                    }
                }
            }
        }
    }
    zip.finish()?;
    Ok(done)
}

/// Extract a save backup zip into PSP/SAVEDATA (zip-slip guarded).
pub fn restore(paths: &PspPaths, zip_path: &Path, ctx: Option<&JobContext>) -> Result<usize> {
    paths.validate()?;
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let savedata = paths.savedata();
    fs::create_dir_all(&savedata)?;
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if let Some(ctx) = ctx {
            ctx.check_cancel()?;
        }
        let target = savedata.join(name);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
        count += 1;
    }
    if let Some(ctx) = ctx {
        ctx.message(format!("restored {} files", count));
    }
    Ok(count)
}

/// ICON0.PNG from a save folder, exactly what the PSP save utility shows.
pub fn icon_bytes(paths: &PspPaths, folder: &str) -> Option<Vec<u8>> {
    let dir = paths.savedata().join(folder);
    std::fs::read(dir.join("ICON0.PNG")).ok()
}

/// Delete a save folder from the stick.
pub fn delete_save(paths: &PspPaths, folder: &str) -> Result<()> {
    if NON_SAVE_FOLDERS.iter().any(|f| folder.eq_ignore_ascii_case(f)) {
        return Err(Error::Invalid("refusing to delete a CFW payload folder".into()));
    }
    let dir = paths.savedata().join(folder);
    if !dir.starts_with(paths.savedata()) || !dir.is_dir() {
        return Err(Error::NotFound(folder.to_string()));
    }
    fs::remove_dir_all(dir)?;
    Ok(())
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

/// Parse helper used by the UI to label a save from an arbitrary SFO blob.
pub fn parse_save_sfo(bytes: &[u8]) -> Option<(String, Option<String>)> {
    let sfo = Sfo::parse(bytes).ok()?;
    let _ = Section::ParamSfo;
    let pbp_probe = Pbp::parse(bytes).ok();
    let _ = pbp_probe;
    Some((sfo.title()?.to_string(), sfo.string("SUBTITLE").map(str::to_owned)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_and_restore_roundtrip() {
        let dir = std::env::temp_dir().join(format!("godsend-sv-{}", std::process::id()));
        let paths = PspPaths::new(&dir);
        let save_dir = paths.savedata().join("UCUS98737DATA00");
        std::fs::create_dir_all(&save_dir).unwrap();
        std::fs::write(save_dir.join("data.bin"), b"save-bytes-0123").unwrap();
        let sfo = crate::sfo::build_sfo(&[
            ("TITLE", crate::sfo::SfoValue::Utf8("Wipeout Pure".into())),
            ("SUBTITLE", crate::sfo::SfoValue::Utf8("Slot 1".into())),
        ]);
        std::fs::write(save_dir.join("PARAM.SFO"), &sfo).unwrap();

        let saves = list(&paths).unwrap();
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].title.as_deref(), Some("Wipeout Pure"));

        let zip_path = dir.join("backup.zip");
        let written = backup(&paths, &[], &zip_path, None).unwrap();
        assert!(written > 0);

        // wipe and restore
        std::fs::remove_dir_all(&save_dir).unwrap();
        assert!(list(&paths).unwrap().is_empty());
        let files = restore(&paths, &zip_path, None).unwrap();
        assert!(files >= 2);
        assert_eq!(
            std::fs::read(save_dir.join("data.bin")).unwrap(),
            b"save-bytes-0123"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
