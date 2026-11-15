//! EBOOT.PBP parser.
//!
//! A PBP file is a small header followed by up to seven concatenated sections.
//! Header (32 bytes, little endian):
//!
//! ```text
//! "\0PBP" | version u32
//!  param_sfo u32 | icon0 u32 | icon1 u32 | pic0 u32 | pic1 u32
//!  snd0 u32 | data_psar u32
//! ```
//!
//! Section i occupies `[off[i], off[i+1])`; the last section ends at EOF.

use crate::error::{Error, Result};
use std::fs;
use std::path::Path;

pub const MAGIC: &[u8; 4] = b"\0PBP";
/// magic(4) + version(4) + 8 offsets (7 sections + end marker).
const HEADER_SIZE: usize = 40;
/// Minimum: magic + version + 7 offsets (some writers omit the end marker).
const HEADER_MIN: usize = 36;
pub const SECTION_COUNT: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    ParamSfo = 0,
    Icon0 = 1,
    Icon1 = 2,
    Pic0 = 3,
    Pic1 = 4,
    Snd0 = 5,
    DataPsar = 6,
}

/// Parsed offsets from a PBP header.
#[derive(Debug, Clone)]
pub struct Pbp {
    pub version: u32,
    offsets: [u32; SECTION_COUNT + 1],
    file_size: u64,
}

impl Pbp {
    /// Parse the header from a full in-memory PBP.
    pub fn parse(bytes: &[u8]) -> Result<Pbp> {
        if bytes.len() < HEADER_MIN || &bytes[0..4] != MAGIC {
            return Err(Error::Invalid("not a PBP file (bad magic)".into()));
        }
        let take = HEADER_SIZE.min(bytes.len());
        Self::from_header(&bytes[..take], bytes.len() as u64)
    }

    /// Parse a header read from disk; `file_size` bounds the last section.
    pub fn from_header(header: &[u8], file_size: u64) -> Result<Pbp> {
        if header.len() < HEADER_MIN || &header[0..4] != MAGIC {
            return Err(Error::Invalid("not a PBP header".into()));
        }
        let version = read_u32(header, 4);
        let mut offsets = [0u32; SECTION_COUNT + 1];
        for (i, off) in offsets.iter_mut().enumerate() {
            let at = 8 + i * 4;
            *off = if at + 4 <= header.len() {
                read_u32(header, at)
            } else {
                0 // omitted end marker: fall back to file_size
            };
        }
        Ok(Pbp {
            version,
            offsets,
            file_size,
        })
    }

    pub fn read_header(path: &Path) -> Result<Pbp> {
        let f = fs::File::open(path)?;
        let meta = f.metadata()?;
        let mut header = [0u8; HEADER_SIZE];
        use std::io::Read;
        let mut f = f;
        let mut filled = 0usize;
        while filled < HEADER_SIZE {
            let read = f.read(&mut header[filled..])?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        Pbp::from_header(&header[..filled], meta.len())
    }

    /// Byte range of a section, empty when the offset is zero.
    pub fn section_range(&self, section: Section) -> Option<(u64, u64)> {
        let i = section as usize;
        let start = self.offsets[i] as u64;
        if start == 0 {
            return None;
        }
        let end = if i + 1 <= SECTION_COUNT && self.offsets[i + 1] != 0 {
            self.offsets[i + 1] as u64
        } else {
            self.file_size
        };
        if end <= start {
            return None;
        }
        Some((start, end))
    }

    /// Read one section fully into memory.
    pub fn read_section(&self, path: &Path, section: Section) -> Result<Option<Vec<u8>>> {
        let Some((start, end)) = self.section_range(section) else {
            return Ok(None);
        };
        use std::io::{Read, Seek, SeekFrom};
        let mut f = fs::File::open(path)?;
        f.seek(SeekFrom::Start(start))?;
        let mut buf = vec![0u8; (end - start) as usize];
        f.read_exact(&mut buf)?;
        Ok(Some(buf))
    }
}

fn read_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assemble a minimal PBP with just a PARAM.SFO and an ICON0.
    fn make_pbp(sfo: &[u8], icon0: &[u8]) -> Vec<u8> {
        let mut offsets = [0u32; SECTION_COUNT + 1];
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&0u32.to_le_bytes());
        // placeholder offsets, patched below
        for _ in 0..=SECTION_COUNT {
            out.extend_from_slice(&0u32.to_le_bytes());
        }
        let mut pos = HEADER_SIZE as u32;
        let mut sections: Vec<Option<&[u8]>> = vec![None; SECTION_COUNT];
        sections[Section::ParamSfo as usize] = Some(sfo);
        sections[Section::Icon0 as usize] = Some(icon0);
        for (i, sec) in sections.iter().enumerate() {
            offsets[i] = pos;
            if let Some(bytes) = sec {
                pos += bytes.len() as u32;
            }
        }
        offsets[SECTION_COUNT] = pos; // end-of-file marker
        // patch offsets
        for (i, off) in offsets.iter().enumerate() {
            let at = 8 + i * 4;
            out[at..at + 4].copy_from_slice(&off.to_le_bytes());
        }
        for sec in sections.iter().flatten() {
            out.extend_from_slice(sec);
        }
        out
    }

    #[test]
    fn parses_sections() {
        let sfo = crate::sfo::build_sfo(&[
            ("TITLE", crate::sfo::SfoValue::Utf8("Test Homebrew".into())),
            ("CATEGORY", crate::sfo::SfoValue::Utf8("MG".into())),
        ]);
        let icon0 = b"\x89PNG fake bytes";
        let pbp_bytes = make_pbp(&sfo, icon0);
        let pbp = Pbp::parse(&pbp_bytes).expect("parse");
        let sfo_back = pbp.read_section(Path::new("memory"), Section::ParamSfo);
        // read_section opens the path; for the in-memory test, use ranges + slicing.
        let (s, e) = pbp.section_range(Section::ParamSfo).expect("sfo present");
        assert_eq!(&pbp_bytes[s as usize..e as usize], sfo.as_slice());
        let (s, e) = pbp.section_range(Section::Icon0).expect("icon present");
        assert_eq!(&pbp_bytes[s as usize..e as usize], icon0);
        assert!(pbp.section_range(Section::Snd0).is_none());
        let _ = sfo_back;
    }
}
