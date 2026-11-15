//! PARAM.SFO parser.
//!
//! PARAM.SFO is the metadata file Sony ships inside every game, homebrew
//! EBOOT and save folder. Layout (all little endian):
//!
//! ```text
//! header  : "\0PSF" | version u32 | key_table_start u32 | data_table_start u32
//! index   : repeated 16-byte rows
//!           key_offset u16 | fmt u16 | data_len u32 | data_max_len u32 | data_offset u32
//! keys    : null-terminated strings, starting at key_table_start
//! data    : starting at data_table_start
//! fmt     : 0x0004 utf8 | 0x0204 int32 | 0x0404 utf8-special
//! ```

use crate::error::{Error, Result};
use serde::Serialize;
use std::collections::BTreeMap;

const MAGIC: &[u8; 4] = b"\0PSF";
const FMT_UTF8: u16 = 0x0004;
const FMT_UTF8_SPECIAL: u16 = 0x0404;
const FMT_INT: u16 = 0x0204;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum SfoValue {
    Utf8(String),
    Int(i32),
}

/// Parsed PARAM.SFO as an ordered key/value map.
#[derive(Debug, Clone, Default)]
pub struct Sfo {
    entries: BTreeMap<String, SfoValue>,
}

impl Sfo {
    pub fn parse(bytes: &[u8]) -> Result<Sfo> {
        if bytes.len() < 20 || &bytes[0..4] != MAGIC {
            return Err(Error::Invalid("not a PARAM.SFO (bad magic)".into()));
        }
        let key_table_start = read_u32(bytes, 8) as usize;
        let data_table_start = read_u32(bytes, 12) as usize;
        if key_table_start > bytes.len() || data_table_start > bytes.len() {
            return Err(Error::Invalid("PARAM.SFO table offsets out of range".into()));
        }

        let mut entries = BTreeMap::new();
        // Index rows start at byte 16 and run until the key table.
        let mut pos = 16usize;
        while pos + 16 <= key_table_start {
            let key_offset = read_u16(bytes, pos) as usize;
            let fmt = read_u16(bytes, pos + 2);
            let data_len = read_u32(bytes, pos + 4) as usize;
            let data_offset = read_u32(bytes, pos + 12) as usize;

            let key = read_cstring(bytes, key_table_start + key_offset, key_table_start)
                .ok_or_else(|| Error::Invalid("PARAM.SFO key table overrun".into()))?;
            if key.is_empty() {
                pos += 16;
                continue;
            }

            let data_start = data_table_start + data_offset;
            let value = match fmt {
                FMT_INT => {
                    if data_start + 4 > bytes.len() {
                        return Err(Error::Invalid("PARAM.SFO int out of range".into()));
                    }
                    SfoValue::Int(read_u32(bytes, data_start) as i32)
                }
                FMT_UTF8 | FMT_UTF8_SPECIAL => {
                    let end = (data_start + data_len).min(bytes.len());
                    let raw = &bytes[data_start..end];
                    let text = String::from_utf8_lossy(raw);
                    SfoValue::Utf8(text.trim_end_matches('\0').to_string())
                }
                other => SfoValue::Utf8(format!("<unknown fmt 0x{other:04x}>")),
            };
            entries.insert(key, value);
            pos += 16;
        }
        Ok(Sfo { entries })
    }

    pub fn get(&self, key: &str) -> Option<&SfoValue> {
        self.entries.get(key)
    }

    pub fn string(&self, key: &str) -> Option<&str> {
        match self.entries.get(key) {
            Some(SfoValue::Utf8(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<i32> {
        match self.entries.get(key) {
            Some(SfoValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Human title ("God of War: Chains of Olympus").
    pub fn title(&self) -> Option<&str> {
        self.string("TITLE")
    }

    /// Disc/serial id like `UCUS-98737` or `NPUG-80124`.
    pub fn title_id(&self) -> Option<&str> {
        self.string("TITLE_ID")
    }

    /// Category: `MG` (UMD/Memory Stick game), `MS` (saved game data),
    /// `HG` (homebrew in the wild), `UG`, `PE` (PSN emulated) and friends.
    pub fn category(&self) -> Option<&str> {
        self.string("CATEGORY")
    }

    /// Minimum PSP firmware version, e.g. "5.55".
    pub fn psp_system_ver(&self) -> Option<&str> {
        self.string("PSP_SYSTEM_VER")
    }

    /// SFO version string ("1.00" etc).
    pub fn version(&self) -> Option<&str> {
        self.string("VERSION")
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }
}

fn read_u16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

fn read_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn read_cstring(b: &[u8], start: usize, limit: usize) -> Option<String> {
    let mut end = start;
    while end < b.len() && end < limit + 4096 && b[end] != 0 {
        end += 1;
    }
    if end >= b.len() {
        return None;
    }
    Some(String::from_utf8_lossy(&b[start..end]).to_string())
}

/// Build a minimal but valid PARAM.SFO in memory (used by tests and mock data).
#[cfg(test)]
pub fn build_sfo(entries: &[(&str, SfoValue)]) -> Vec<u8> {
    let mut keys: Vec<u8> = Vec::new();
    let mut data: Vec<u8> = Vec::new();
    let mut index: Vec<u8> = Vec::new();
    let count = entries.len();
    let key_table_start = 16 + count * 16;
    let mut key_offset = 0u16;
    for (name, value) in entries {
        let data_offset = data.len() as u32;
        let (fmt, bytes) = match value {
            SfoValue::Utf8(s) => {
                let mut v = s.as_bytes().to_vec();
                v.push(0);
                (FMT_UTF8, v)
            }
            SfoValue::Int(v) => (FMT_INT, v.to_le_bytes().to_vec()),
        };
        let data_len = bytes.len() as u32;
        index.extend_from_slice(&key_offset.to_le_bytes());
        index.extend_from_slice(&fmt.to_le_bytes());
        index.extend_from_slice(&data_len.to_le_bytes());
        index.extend_from_slice(&data_len.to_le_bytes()); // max len
        index.extend_from_slice(&data_offset.to_le_bytes());
        keys.extend_from_slice(name.as_bytes());
        keys.push(0);
        key_offset += name.len() as u16 + 1;
        data.extend_from_slice(&bytes);
    }
    let mut out = Vec::with_capacity(key_table_start + keys.len() + data.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&0x0000_0101u32.to_le_bytes()); // version 1.1
    out.extend_from_slice(&(key_table_start as u32).to_le_bytes());
    out.extend_from_slice(&((key_table_start + keys.len()) as u32).to_le_bytes());
    out.extend_from_slice(&index);
    out.extend_from_slice(&keys);
    out.extend_from_slice(&data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_roundtrip_sfo() {
        let bytes = build_sfo(&[
            ("CATEGORY", SfoValue::Utf8("MG".into())),
            ("PSP_SYSTEM_VER", SfoValue::Utf8("5.55".into())),
            ("TITLE", SfoValue::Utf8("LocoRoco".into())),
            ("TITLE_ID", SfoValue::Utf8("UCUS-98716".into())),
            ("VERSION", SfoValue::Utf8("1.00".into())),
            ("PARENTAL_LEVEL", SfoValue::Int(1)),
        ]);
        let sfo = Sfo::parse(&bytes).expect("parse");
        assert_eq!(sfo.title(), Some("LocoRoco"));
        assert_eq!(sfo.title_id(), Some("UCUS-98716"));
        assert_eq!(sfo.category(), Some("MG"));
        assert_eq!(sfo.psp_system_ver(), Some("5.55"));
        assert_eq!(sfo.int("PARENTAL_LEVEL"), Some(1));
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(Sfo::parse(b"NOPE").is_err());
    }
}
