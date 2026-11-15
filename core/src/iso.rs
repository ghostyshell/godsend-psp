//! ISO9660 reader for UMD disc images.
//!
//! PSP retail images carry the game data in a `PSP_GAME` directory at the
//! ISO root (`PARAM.SFO`, `ICON0.PNG`, `PIC1.PNG`, `SYSDIR/EBOOT.BIN`), so
//! walking the directory tree gives us title, serial id and cover art without
//! ever touching the payload.

use crate::error::{Error, Result};
use crate::sfo::Sfo;
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const SECTOR: u64 = 2048;
const PVD_SECTOR: u64 = 16;

#[derive(Debug, Clone, Serialize, Default)]
pub struct IsoInfo {
    pub is_iso9660: bool,
    /// True when a PSP_GAME/PARAM.SFO was found (retail UMD image).
    pub is_umd: bool,
    pub title: Option<String>,
    pub title_id: Option<String>,
    pub psp_system_ver: Option<String>,
}

/// Analyze a `.iso` file: extract title/id/cover presence.
pub fn analyze(path: &Path) -> Result<IsoInfo> {
    let mut f = File::open(path)?;
    let pvd = read_sector(&mut f, PVD_SECTOR)?;
    if !is_pvd(&pvd) {
        return Ok(IsoInfo::default());
    }
    let mut info = IsoInfo {
        is_iso9660: true,
        ..Default::default()
    };
    let root = dir_record(&pvd, 156).ok_or(Error::Invalid("PVD without root record".into()))?;
    let (lba, size) = (record_lba(&root), record_size(&root));
    let dir = read_extent(&mut f, lba, size)?;

    for rec in iter_records(&dir) {
        let name = record_name(&rec);
        match name.as_str() {
            "PSP_GAME" => {
                let sub = read_dir(&mut f, &rec)?;
                for entry in iter_records(&sub) {
                    match record_name(&entry).as_str() {
                        "PARAM.SFO" => {
                            let bytes = read_extent(&mut f, record_lba(&entry), record_size(&entry))?;
                            if let Ok(sfo) = Sfo::parse(&bytes) {
                                info.is_umd = true;
                                info.title = sfo.title().map(str::to_owned);
                                info.title_id = sfo.title_id().map(str::to_owned);
                                info.psp_system_ver = sfo.psp_system_ver().map(str::to_owned);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if info.is_umd {
            break;
        }
    }
    let _ = dir;
    Ok(info)
}

/// Read `PSP_GAME/ICON0.PNG` bytes (cover art) from a UMD image.
pub fn read_icon0(path: &Path) -> Result<Option<Vec<u8>>> {
    let mut f = File::open(path)?;
    let pvd = read_sector(&mut f, PVD_SECTOR)?;
    if !is_pvd(&pvd) {
        return Ok(None);
    }
    let root = dir_record(&pvd, 156).ok_or(Error::Invalid("PVD without root record".into()))?;
    let dir = read_dir(&mut f, &root)?;
    for rec in iter_records(&dir) {
        if record_name(&rec) == "PSP_GAME" {
            let sub = read_dir(&mut f, &rec)?;
            for entry in iter_records(&sub) {
                if record_name(&entry) == "ICON0.PNG" {
                    let bytes = read_extent(&mut f, record_lba(&entry), record_size(&entry))?;
                    return Ok(Some(bytes));
                }
            }
        }
    }
    Ok(None)
}

fn is_pvd(sector: &[u8]) -> bool {
    sector.len() >= SECTOR as usize && sector[0] == 1 && &sector[1..6] == b"CD001"
}

fn read_sector(f: &mut File, sector: u64) -> Result<Vec<u8>> {
    f.seek(SeekFrom::Start(sector * SECTOR))?;
    let mut buf = vec![0u8; SECTOR as usize];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

fn read_extent(f: &mut File, lba: u32, size: u32) -> Result<Vec<u8>> {
    f.seek(SeekFrom::Start(lba as u64 * SECTOR))?;
    let mut buf = vec![0u8; size as usize];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

/// Read a whole directory extent for a directory record.
fn read_dir(f: &mut File, rec: &[u8]) -> Result<Vec<u8>> {
    if rec[25] & 0x02 == 0 {
        return Err(Error::Invalid("expected a directory record".into()));
    }
    read_extent(f, record_lba(rec), record_size(rec))
}

fn dir_record(parent: &[u8], at: usize) -> Option<Vec<u8>> {
    if at + 34 > parent.len() {
        return None;
    }
    let len = parent[at] as usize;
    if len == 0 || at + len > parent.len() {
        return None;
    }
    Some(parent[at..at + len].to_vec())
}

/// Iterate directory records inside a raw directory extent, skipping the
/// self/parent entries (which repeat at 2048 sector boundaries).
fn iter_records(dir: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < dir.len() {
        let len = dir[pos] as usize;
        if len == 0 {
            // Records never span sector boundaries; skip to the next one.
            pos = (pos / SECTOR as usize + 1) * SECTOR as usize;
            continue;
        }
        if pos + len > dir.len() {
            break;
        }
        let rec = &dir[pos..pos + len];
        let flags = rec[25];
        let name = record_name(rec);
        let is_self_or_parent = name == "." || name == "..";
        if !is_self_or_parent {
            out.push(rec.to_vec());
        }
        let _ = flags;
        pos += len;
    }
    out
}

fn record_name(rec: &[u8]) -> String {
    let name_len = rec[32] as usize;
    if name_len == 0 || 33 + name_len > rec.len() {
        return String::new();
    }
    let raw = &rec[33..33 + name_len];
    // ISO9660 self/parent records use the single bytes 0x00 / 0x01.
    match raw {
        [0] => ".".to_string(),
        [1] => "..".to_string(),
        _ => {
            let name = String::from_utf8_lossy(raw);
            name.split(';').next().unwrap_or("").to_string()
        }
    }
}

fn record_lba(rec: &[u8]) -> u32 {
    u32::from_le_bytes([rec[2], rec[3], rec[4], rec[5]])
}

fn record_size(rec: &[u8]) -> u32 {
    u32::from_le_bytes([rec[10], rec[11], rec[12], rec[13]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_name_strips_version_suffix() {
        let mut rec = vec![0u8; 48];
        rec[32] = 11;
        rec[33..44].copy_from_slice(b"PARAM.SFO;1");
        assert_eq!(record_name(&rec), "PARAM.SFO");
    }

    #[test]
    fn iter_records_skips_self_and_parent() {
        let mut dir = vec![0u8; 2048];
        // self record at 0: len 34, name_len 1, name 0x00 (".")
        dir[0] = 34;
        dir[32] = 1;
        dir[33] = 0;
        // parent record at 34: name 0x01 ("..")
        dir[34] = 34;
        dir[34 + 32] = 1;
        dir[34 + 33] = 1;
        // a real record at 68: len 41 (33 fixed + 8 name), name "PSP_GAME"
        dir[68] = 41;
        dir[68 + 32] = 8;
        dir[68 + 33..68 + 41].copy_from_slice(b"PSP_GAME");
        let records = iter_records(&dir);
        assert_eq!(records.len(), 1);
        assert_eq!(record_name(&records[0]), "PSP_GAME");
    }
}
