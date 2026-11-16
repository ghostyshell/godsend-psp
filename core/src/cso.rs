//! ISO <-> CSO (Compressed ISO) conversion.
//!
//! CSO v1 layout: 24 byte header, then `(blocks + 1)` little endian u32 index
//! entries, then the data frames. Index entry bit 31 marks a zlib compressed
//! frame; when clear the frame is stored raw. Frame offsets are stored in
//! `1 << align` byte units.

use crate::error::{Error, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const CSO_MAGIC: &[u8; 4] = b"CISO";
/// Older builds wrote `CSIO`. Files with that header still decompress.
pub const LEGACY_CSO_MAGIC: &[u8; 4] = b"CSIO";
const HEADER_SIZE: usize = 24;
const BLOCK_SIZE: usize = 2048;

fn is_cso_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && (&bytes[0..4] == CSO_MAGIC || &bytes[0..4] == LEGACY_CSO_MAGIC)
}

#[derive(Debug, Clone, Serialize)]
pub struct CsoInfo {
    pub is_cso: bool,
    pub version: u8,
    pub block_size: u32,
    pub total_bytes: u64,
    pub blocks: u32,
}

/// Read the CSO header of a file (or report a non-CSO file).
pub fn probe(path: &Path) -> Result<CsoInfo> {
    let mut f = File::open(path)?;
    let mut header = [0u8; HEADER_SIZE];
    if f.read_exact(&mut header).is_err() || !is_cso_magic(&header) {
        return Ok(CsoInfo {
            is_cso: false,
            version: 0,
            block_size: 0,
            total_bytes: 0,
            blocks: 0,
        });
    }
    let hdr_size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    let _ = hdr_size;
    let total_bytes = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let block_size = u32::from_le_bytes(header[16..20].try_into().unwrap());
    let version = header[20];
    let blocks = blocks_count(total_bytes, block_size);
    Ok(CsoInfo {
        is_cso: true,
        version,
        block_size,
        total_bytes,
        blocks: blocks.try_into().unwrap_or(0),
    })
}

fn blocks_count(total_bytes: u64, block_size: u32) -> u64 {
    let bs = if block_size == 0 { BLOCK_SIZE as u64 } else { block_size as u64 };
    (total_bytes + bs - 1) / bs
}

/// Convert a plain `.iso` into a zlib-compressed `.cso`.
pub fn iso_to_cso(
    src: &Path,
    dst: &Path,
    level: u32,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    if probe(src)?.is_cso {
        return Err(Error::Invalid(
            "source is already a CSO; decompress it first".into(),
        ));
    }
    // Build beside the destination and rename on success, so an interrupted
    // job never leaves a half-written CSO on the stick.
    let tmp = dst.with_extension("cso.part");
    if let Err(err) = write_cso(src, &tmp, level, progress, cancel) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err);
    }
    std::fs::rename(&tmp, dst)?;
    Ok(())
}

fn write_cso(
    src: &Path,
    dst: &Path,
    level: u32,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    let mut input = File::open(src)?;
    let total = input.metadata()?.len();
    let blocks = blocks_count(total, BLOCK_SIZE as u32) as usize;
    let compression = Compression::new(level.clamp(1, 9));

    let mut out = File::create(dst)?;
    // header placeholder
    out.write_all(CSO_MAGIC)?;
    out.write_all(&(HEADER_SIZE as u32).to_le_bytes())?;
    out.write_all(&total.to_le_bytes())?;
    out.write_all(&(BLOCK_SIZE as u32).to_le_bytes())?;
    out.write_all(&[1u8, 0u8, 0u8, 0u8])?; // ver 1, align 0, rsv
    let index_pos = HEADER_SIZE as u64;
    out.seek(SeekFrom::Start(index_pos + (blocks as u64 + 1) * 4))?;

    let mut index: Vec<u32> = Vec::with_capacity(blocks + 1);
    let mut raw = vec![0u8; BLOCK_SIZE];
    let mut done: u64 = 0;
    for block in 0..blocks {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = std::fs::remove_file(dst);
            return Err(Error::Cancelled(0));
        }
        let want = if block == blocks - 1 {
            (total - done) as usize
        } else {
            BLOCK_SIZE
        };
        input.read_exact(&mut raw[..want])?;
        let offset_units = out.stream_position()? as u32;
        index.push(0x8000_0000 | offset_units);
        let mut enc = ZlibEncoder::new(Vec::new(), compression);
        enc.write_all(&raw[..want])?;
        let compressed = enc.finish()?;
        if compressed.len() < want {
            out.write_all(&compressed)?;
        } else {
            // Not worth compressing: store raw, clear the flag.
            let entry = index.len() - 1;
            index[entry] = offset_units; // raw frame, no 0x8000_0000 bit
            out.write_all(&raw[..want])?;
        }
        done += want as u64;
        progress(done, total);
    }
    let end_units = out.stream_position()? as u32;
    index.push(end_units);

    // Patch the index at the front.
    let mut index_bytes = Vec::with_capacity(index.len() * 4);
    for entry in &index {
        index_bytes.extend_from_slice(&entry.to_le_bytes());
    }
    out.seek(SeekFrom::Start(index_pos))?;
    out.write_all(&index_bytes)?;
    Ok(())
}

/// Decompress a `.cso` back into a plain `.iso`.
pub fn cso_to_iso(
    src: &Path,
    dst: &Path,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    let tmp = dst.with_extension("iso.part");
    if let Err(err) = write_iso(src, &tmp, progress, cancel) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err);
    }
    std::fs::rename(&tmp, dst)?;
    Ok(())
}

fn write_iso(
    src: &Path,
    dst: &Path,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    let mut input = File::open(src)?;
    let mut header = [0u8; HEADER_SIZE];
    input.read_exact(&mut header)?;
    if !is_cso_magic(&header) {
        return Err(Error::Invalid("not a CSO file".into()));
    }
    let hdr_size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64;
    let total_bytes = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let block_size = {
        let bs = u32::from_le_bytes(header[16..20].try_into().unwrap());
        if bs == 0 {
            BLOCK_SIZE as u32
        } else {
            bs
        }
    };
    let align = header[21] as u64;
    let blocks = blocks_count(total_bytes, block_size) as usize;

    let mut index = vec![0u32; blocks + 1];
    input.seek(SeekFrom::Start(hdr_size.max(HEADER_SIZE as u64)))?;
    let mut index_bytes = vec![0u8; (blocks + 1) * 4];
    input.read_exact(&mut index_bytes)?;
    for (i, entry) in index.iter_mut().enumerate() {
        let at = i * 4;
        *entry = u32::from_le_bytes(index_bytes[at..at + 4].try_into().unwrap());
    }

    let mut out = File::create(dst)?;
    let mut block = vec![0u8; block_size as usize];
    for i in 0..blocks {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = std::fs::remove_file(dst);
            return Err(Error::Cancelled(0));
        }
        let frame_from = ((index[i] & 0x7FFF_FFFF) as u64) << align;
        let frame_to = ((index[i + 1] & 0x7FFF_FFFF) as u64) << align;
        let compressed = index[i] & 0x8000_0000 != 0;
        let want = (block_size as usize).min((total_bytes - (i as u64 * block_size as u64)) as usize);
        input.seek(SeekFrom::Start(frame_from))?;
        if compressed {
            let mut raw = vec![0u8; (frame_to - frame_from) as usize];
            input.read_exact(&mut raw)?;
            let mut dec = ZlibDecoder::new(&raw[..]);
            dec.read_exact(&mut block[..want])?;
        } else {
            input.read_exact(&mut block[..want])?;
        }
        out.write_all(&block[..want])?;
        let done = (i as u64 + 1) * block_size as u64;
        progress(done.min(total_bytes), total_bytes);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn dummy_iso(blocks: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(blocks * BLOCK_SIZE);
        for i in 0..blocks {
            // low-entropy blocks so compression is worth it
            let fill = (i as u8).wrapping_mul(7);
            data.extend(std::iter::repeat(fill).take(BLOCK_SIZE));
        }
        data
    }

    #[test]
    fn roundtrip_iso_cso_iso() {
        let dir = std::env::temp_dir().join(format!("godsend-cso-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso_path = dir.join("t.iso");
        let cso_path = dir.join("t.cso");
        let iso2_path = dir.join("t2.iso");
        let data = dummy_iso(8);
        std::fs::write(&iso_path, &data).unwrap();

        let cancel = AtomicBool::new(false);
        iso_to_cso(
            &iso_path,
            &cso_path,
            9,
            &mut |_, _| {},
            &cancel,
        )
        .expect("compress");
        let cso_size = std::fs::metadata(&cso_path).unwrap().len();
        assert!(cso_size < data.len() as u64, "cso must be smaller");

        let info = probe(&cso_path).expect("probe");
        assert!(info.is_cso);
        assert_eq!(info.total_bytes, data.len() as u64);

        cso_to_iso(&cso_path, &iso2_path, &mut |_, _| {}, &cancel).expect("decompress");
        let back = std::fs::read(&iso2_path).unwrap();
        assert_eq!(back, data);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn writes_the_real_ciso_magic() {
        let dir = std::env::temp_dir().join(format!("godsend-cso-magic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso_path = dir.join("m.iso");
        let cso_path = dir.join("m.cso");
        std::fs::write(&iso_path, dummy_iso(4)).unwrap();
        let cancel = AtomicBool::new(false);
        iso_to_cso(&iso_path, &cso_path, 9, &mut |_, _| {}, &cancel).unwrap();
        let bytes = std::fs::read(&cso_path).unwrap();
        assert_eq!(&bytes[0..4], b"CISO", "PSP tools require the CISO magic");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reads_legacy_csio_header() {
        let dir = std::env::temp_dir().join(format!("godsend-cso-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso_path = dir.join("l.iso");
        let cso_path = dir.join("l.cso");
        let out_path = dir.join("l2.iso");
        let data = dummy_iso(6);
        std::fs::write(&iso_path, &data).unwrap();
        let cancel = AtomicBool::new(false);
        iso_to_cso(&iso_path, &cso_path, 9, &mut |_, _| {}, &cancel).unwrap();

        let mut bytes = std::fs::read(&cso_path).unwrap();
        bytes[0..4].copy_from_slice(LEGACY_CSO_MAGIC);
        std::fs::write(&cso_path, &bytes).unwrap();

        assert!(probe(&cso_path).unwrap().is_cso);
        cso_to_iso(&cso_path, &out_path, &mut |_, _| {}, &cancel).expect("legacy read");
        assert_eq!(std::fs::read(&out_path).unwrap(), data);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn refuses_to_compress_a_cso() {
        let dir = std::env::temp_dir().join(format!("godsend-cso-guard-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso_path = dir.join("g.iso");
        let cso_path = dir.join("g.cso");
        let again = dir.join("g2.cso");
        std::fs::write(&iso_path, dummy_iso(4)).unwrap();
        let cancel = AtomicBool::new(false);
        iso_to_cso(&iso_path, &cso_path, 9, &mut |_, _| {}, &cancel).unwrap();
        let err = iso_to_cso(&cso_path, &again, 9, &mut |_, _| {}, &cancel);
        assert!(err.is_err(), "a CSO must not be compressed again");
        assert!(!again.exists(), "no partial output on refusal");
        std::fs::remove_dir_all(&dir).ok();
    }
}
