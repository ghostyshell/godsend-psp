//! Minimal bencode reader for .torrent files.
//!
//! Store downloads need the file list out of a torrent and aria2's 1-based
//! `--select-file` index for each entry, so this parses bencode directly
//! instead of adding a dependency. Ints, byte strings, lists and dicts are
//! handled, with a nesting depth cap and a total input cap.

use crate::error::{Error, Result};
use std::io::Read;

/// A deeper nest than this is never a real torrent.
const MAX_DEPTH: usize = 32;

/// Refuse torrent bodies larger than this; the biggest PSP set is ~3 MB.
pub const MAX_TORRENT_BYTES: usize = 32 * 1024 * 1024;

/// One file inside a torrent. `index` is 1-based, matching aria2's
/// `--select-file` numbering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentFile {
    pub index: u32,
    pub path: String,
    pub length: u64,
}

/// Parse a bencoded torrent into its file rows, in torrent order.
pub fn parse_files(bytes: &[u8]) -> Result<Vec<TorrentFile>> {
    if bytes.len() > MAX_TORRENT_BYTES {
        return Err(Error::Invalid(format!(
            "torrent: {} bytes exceeds the {} MB cap",
            bytes.len(),
            MAX_TORRENT_BYTES / (1024 * 1024)
        )));
    }
    let root = Decoder::new(bytes).value(0)?;
    let info = root
        .get("info")
        .ok_or_else(|| Error::Invalid("torrent: no info dictionary".into()))?;

    if let Some(files) = info.get("files").and_then(Value::as_list) {
        // aria2 writes multi-file torrents under `<info.name>/`, so the stored
        // path must include it for the post-download lookup to find the file.
        let root_name = info
            .get("name")
            .and_then(Value::as_bytes)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let mut out = Vec::with_capacity(files.len());
        for (pos, entry) in files.iter().enumerate() {
            let path = entry
                .get("path")
                .and_then(Value::as_list)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(Value::as_bytes)
                        .map(|part| String::from_utf8_lossy(part).into_owned())
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default();
            if path.is_empty() {
                continue;
            }
            let path = if root_name.is_empty() {
                path
            } else {
                format!("{root_name}/{path}")
            };
            // Keep the torrent's own numbering even when a row is skipped.
            let index = (pos + 1) as u32;
            let length = entry.get("length").and_then(Value::as_int).unwrap_or(0).max(0) as u64;
            out.push(TorrentFile { index, path, length });
        }
        if out.is_empty() {
            return Err(Error::Invalid("torrent: info.files has no usable paths".into()));
        }
        return Ok(out);
    }

    // Single-file torrents carry a name and a length instead of a file list.
    let name = info
        .get("name")
        .and_then(Value::as_bytes)
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .ok_or_else(|| Error::Invalid("torrent: no info.files or info.name".into()))?;
    let length = info
        .get("length")
        .and_then(Value::as_int)
        .ok_or_else(|| Error::Invalid("torrent: single-file info has no length".into()))?;
    Ok(vec![TorrentFile {
        index: 1,
        path: name,
        length: length.max(0) as u64,
    }])
}

/// Fetch a torrent over HTTP and parse its file list.
pub fn fetch_files(url: &str) -> Result<Vec<TorrentFile>> {
    parse_files(&fetch_bytes(url)?)
}

/// Fetch a torrent body over HTTP, with the same size cap as the parser.
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();
    let response = agent
        .get(url)
        .call()
        .map_err(|err| Error::Http(format!("torrent fetch failed: {err}")))?;
    if let Some(len) = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok())
    {
        if len > MAX_TORRENT_BYTES as u64 {
            return Err(Error::Invalid(format!(
                "torrent: body of {len} bytes exceeds the {} MB cap",
                MAX_TORRENT_BYTES / (1024 * 1024)
            )));
        }
    }
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_TORRENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_TORRENT_BYTES {
        return Err(Error::Invalid(format!(
            "torrent: body over {} MB",
            MAX_TORRENT_BYTES / (1024 * 1024)
        )));
    }
    Ok(bytes)
}

/// Lowercase hex SHA-1 of the exact bencoded `info` dictionary bytes, ready
/// for a `magnet:?xt=urn:btih:` info hash.
pub fn info_hash(bytes: &[u8]) -> Result<String> {
    if bytes.len() > MAX_TORRENT_BYTES {
        return Err(Error::Invalid(format!(
            "torrent: {} bytes exceeds the {} MB cap",
            bytes.len(),
            MAX_TORRENT_BYTES / (1024 * 1024)
        )));
    }
    let mut decoder = Decoder::new(bytes);
    if decoder.peek()? != b'd' {
        return Err(Error::Invalid("torrent: body is not a dictionary".into()));
    }
    decoder.pos += 1;
    loop {
        if decoder.peek()? == b'e' {
            break;
        }
        let key = decoder.byte_string()?;
        let start = decoder.pos;
        let _ = decoder.value(1)?;
        if key == b"info" {
            return Ok(hex(&sha1(&bytes[start..decoder.pos])));
        }
    }
    Err(Error::Invalid("torrent: no info dictionary".into()))
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

/// Minimal SHA-1 so info hashes need no dependency.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in message.chunks(64) {
        let mut words = [0u32; 80];
        for (i, word) in words.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            words[i] =
                (words[i - 3] ^ words[i - 8] ^ words[i - 14] ^ words[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, word) in words.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, value) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Bytes(Vec<u8>),
    List(Vec<Value>),
    Dict(Vec<(Vec<u8>, Value)>),
}

impl Value {
    fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Dict(entries) => entries
                .iter()
                .find(|(name, _)| name == key.as_bytes())
                .map(|(_, value)| value),
            _ => None,
        }
    }

    fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(number) => Some(*number),
            _ => None,
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Decoder { bytes, pos: 0 }
    }

    fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return Err(Error::Invalid("torrent: bencode nesting too deep".into()));
        }
        match self.peek()? {
            b'i' => {
                self.pos += 1;
                let end = self.find(b'e')?;
                let text = std::str::from_utf8(&self.bytes[self.pos..end])
                    .map_err(|_| Error::Invalid("torrent: bad bencode integer".into()))?;
                let number = text
                    .parse::<i64>()
                    .map_err(|_| Error::Invalid("torrent: bad bencode integer".into()))?;
                self.pos = end + 1;
                Ok(Value::Int(number))
            }
            b'l' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    if self.peek()? == b'e' {
                        self.pos += 1;
                        return Ok(Value::List(items));
                    }
                    items.push(self.value(depth + 1)?);
                }
            }
            b'd' => {
                self.pos += 1;
                let mut entries = Vec::new();
                loop {
                    if self.peek()? == b'e' {
                        self.pos += 1;
                        return Ok(Value::Dict(entries));
                    }
                    let key = self.byte_string()?;
                    let value = self.value(depth + 1)?;
                    entries.push((key, value));
                }
            }
            b'0'..=b'9' => Ok(Value::Bytes(self.byte_string()?)),
            other => Err(Error::Invalid(format!(
                "torrent: unexpected bencode tag 0x{other:02x}"
            ))),
        }
    }

    /// Read `<length>:<bytes>` at the current position.
    fn byte_string(&mut self) -> Result<Vec<u8>> {
        let end = self.find(b':')?;
        let text = std::str::from_utf8(&self.bytes[self.pos..end])
            .map_err(|_| Error::Invalid("torrent: bad bencode string length".into()))?;
        let len = text
            .parse::<usize>()
            .map_err(|_| Error::Invalid("torrent: bad bencode string length".into()))?;
        let start = end + 1;
        let stop = start
            .checked_add(len)
            .filter(|stop| *stop <= self.bytes.len())
            .ok_or_else(|| Error::Invalid("torrent: truncated bencode string".into()))?;
        self.pos = stop;
        Ok(self.bytes[start..stop].to_vec())
    }

    fn peek(&self) -> Result<u8> {
        self.bytes
            .get(self.pos)
            .copied()
            .ok_or_else(|| Error::Invalid("torrent: truncated bencode".into()))
    }

    fn find(&self, byte: u8) -> Result<usize> {
        self.bytes[self.pos..]
            .iter()
            .position(|candidate| *candidate == byte)
            .map(|offset| self.pos + offset)
            .ok_or_else(|| Error::Invalid("torrent: unterminated bencode value".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bencode a byte string.
    fn s(value: &str) -> String {
        format!("{}:{}", value.len(), value)
    }

    /// Bencode one `info.files` entry with a multi-part path.
    fn file(length: u64, parts: &[&str]) -> String {
        let path: String = parts.iter().map(|part| s(part)).collect();
        format!("d6:lengthi{length}e4:pathl{path}ee")
    }

    #[test]
    fn multi_file_torrent_keeps_order_and_indexes() {
        let files = format!(
            "l{}{}{}e",
            file(268435456, &["Redump", "Sony - PlayStation Portable", "007 - From Russia with Love (Europe).zip"]),
            file(700, &["Redump", "Sony - PlayStation Portable", "Ape Escape P (Europe).iso"]),
            file(12, &["Redump", "Sony - PlayStation Portable", "Readme.txt"]),
        );
        let info = format!("d5:files{files}4:name{}6:pieces{}e", s("PSP"), s("xxxx"));
        let torrent = format!("d8:announce{}4:info{info}e", s("http://tracker.example/announce"));

        let parsed = parse_files(torrent.as_bytes()).expect("parse multi-file torrent");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].index, 1);
        assert_eq!(
            parsed[0].path,
            "PSP/Redump/Sony - PlayStation Portable/007 - From Russia with Love (Europe).zip"
        );
        assert_eq!(parsed[0].length, 268435456);
        assert_eq!(parsed[1].index, 2);
        assert_eq!(parsed[1].path, "PSP/Redump/Sony - PlayStation Portable/Ape Escape P (Europe).iso");
        assert_eq!(parsed[1].length, 700);
        assert_eq!(parsed[2].index, 3);
        assert_eq!(parsed[2].path, "PSP/Redump/Sony - PlayStation Portable/Readme.txt");
        assert_eq!(parsed[2].length, 12);
    }

    #[test]
    fn single_file_torrent_uses_name_and_length() {
        let info = format!("d6:lengthi123456e4:name{}6:pieces{}e", s("game.iso"), s("yyyy"));
        let torrent = format!("d4:info{info}e");

        let parsed = parse_files(torrent.as_bytes()).expect("parse single-file torrent");
        assert_eq!(
            parsed,
            vec![TorrentFile {
                index: 1,
                path: "game.iso".into(),
                length: 123456,
            }]
        );
    }

    #[test]
    fn sha1_matches_the_known_vector() {
        assert_eq!(
            hex(&sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&sha1(b"")),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }

    #[test]
    fn info_hash_is_the_bencoded_info_dict() {
        let info = format!("d6:lengthi123456e4:name{}6:pieces{}e", s("game.iso"), s("yyyy"));
        let torrent = format!("d8:announce{}4:info{info}e", s("http://tracker.example/announce"));
        let hash = info_hash(torrent.as_bytes()).expect("info hash");
        assert_eq!(hash.len(), 40);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash, hash.to_ascii_lowercase());
        // The hash covers the info dict alone, not the announce list.
        let bare = info_hash(format!("d4:info{info}e").as_bytes()).expect("bare info hash");
        assert_eq!(hash, bare);
    }

    #[test]
    fn malformed_payloads_error() {
        assert!(parse_files(b"d4:infod6:lengthi12e").is_err());
        assert!(parse_files(b"not bencode").is_err());
        assert!(parse_files(b"d4:info" ).is_err());
        assert!(parse_files(b"d4:infoi12ee").is_err());
    }
}
