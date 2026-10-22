//! The string stream of an AECC record, and renaming objects.
//!
//! Observed in every alignment record (S1, S25; docs/alignment-object.md
//! "Names and the string stream"), matching the R2007+ layout in the public
//! ODA specification:
//!
//! - the data stream ends with a flag bit (1 = has strings) at
//!   `data_end − 1`, preceded by a 16-bit little-endian size at
//!   `data_end − 17`: the string stream's length in bits;
//! - the string stream occupies the `size` bits before that field;
//! - each string is a bit-coded short length (`01` + 1 byte for 1–255,
//!   `10` for 0, `00` + 16 bits otherwise) followed by that many UTF-16LE
//!   code units.
//!
//! [`set_name`] rewrites one string in place, shifting everything after it
//! and the handle stream as a block; the record size can change.

use crate::raw;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum StringStreamError {
    /// No string-stream flag / size at the end of the data stream.
    NoStringStream,
    /// The size uses the 32-bit extension (≥ 0x8000 bits), not handled.
    LargeStream,
    /// The strings don't parse back-to-back to the size field.
    Malformed(String),
    NotFound(String),
    TooLong(usize),
}

impl fmt::Display for StringStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStringStream => write!(f, "record has no string stream"),
            Self::LargeStream => write!(f, "string stream too large (extended size) — not handled"),
            Self::Malformed(why) => write!(f, "string stream doesn't parse: {why}"),
            Self::NotFound(s) => write!(f, "string {s:?} not found in the string stream"),
            Self::TooLong(n) => write!(f, "string too long ({n} code units)"),
        }
    }
}

impl std::error::Error for StringStreamError {}

/// One string in the stream.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamString {
    /// Bit offset of its length field.
    pub bit: usize,
    /// Bit offset just past its last code unit.
    pub end: usize,
    pub text: String,
}

/// Location of the string stream.
#[derive(Debug, Clone, PartialEq)]
pub struct StringStream {
    pub start: usize,
    /// Bit offset of the 16-bit size field (= end of the strings).
    pub size_field: usize,
    /// End of the data stream (start of the handle stream).
    pub data_end: usize,
    pub strings: Vec<StreamString>,
}

fn read_bs(data: &[u8], bit: usize) -> Option<(usize, usize)> {
    match (raw::get_bit(data, bit), raw::get_bit(data, bit + 1)) {
        (0, 1) => Some((raw::get_byte(data, bit + 2) as usize, bit + 10)),
        (1, 0) => Some((0, bit + 2)),
        (0, 0) => Some((
            raw::get_byte(data, bit + 2) as usize | (raw::get_byte(data, bit + 10) as usize) << 8,
            bit + 18,
        )),
        _ => None,
    }
}

fn read_u16(data: &[u8], bit: usize) -> usize {
    raw::get_byte(data, bit) as usize | (raw::get_byte(data, bit + 8) as usize) << 8
}

/// Parse the string stream of a record with `handle_bits` bits of handle
/// stream.
pub fn parse(data: &[u8], handle_bits: usize) -> Result<StringStream, StringStreamError> {
    let data_end = raw::bit_len(data)
        .checked_sub(handle_bits)
        .ok_or(StringStreamError::NoStringStream)?;
    if data_end < 17 || raw::get_bit(data, data_end - 1) != 1 {
        return Err(StringStreamError::NoStringStream);
    }
    let size_field = data_end - 17;
    let size = read_u16(data, size_field);
    if size & 0x8000 != 0 {
        return Err(StringStreamError::LargeStream);
    }
    let start = size_field
        .checked_sub(size)
        .ok_or(StringStreamError::NoStringStream)?;
    let mut strings = Vec::new();
    let mut bit = start;
    while bit < size_field {
        let (n, chars) = read_bs(data, bit)
            .ok_or_else(|| StringStreamError::Malformed(format!("bad length at {bit}")))?;
        let end = chars + 16 * n;
        if end > size_field {
            return Err(StringStreamError::Malformed(format!(
                "string at {bit} runs past the size field"
            )));
        }
        let units: Vec<u16> = (0..n)
            .map(|i| read_u16(data, chars + 16 * i) as u16)
            .collect();
        strings.push(StreamString {
            bit,
            end,
            text: String::from_utf16_lossy(&units),
        });
        bit = end;
    }
    if bit != size_field {
        return Err(StringStreamError::Malformed("strings overrun".into()));
    }
    Ok(StringStream {
        start,
        size_field,
        data_end,
        strings,
    })
}

fn push_bits(out: &mut Vec<u8>, v: u64, n: usize) {
    for k in (0..n).rev() {
        out.push(((v >> k) & 1) as u8);
    }
}

fn push_le16(out: &mut Vec<u8>, v: u16) {
    push_bits(out, (v & 0xFF) as u64, 8);
    push_bits(out, (v >> 8) as u64, 8);
}

/// Replace string number `index` of the stream with `new`. Returns the new
/// record bytes and handle-stream size in bits.
pub fn replace(
    data: &[u8],
    handle_bits: usize,
    index: usize,
    new: &str,
) -> Result<(Vec<u8>, usize), StringStreamError> {
    let ss = parse(data, handle_bits)?;
    let old = ss
        .strings
        .get(index)
        .ok_or_else(|| StringStreamError::NotFound(format!("#{index}")))?;
    let units: Vec<u16> = new.encode_utf16().collect();
    if units.len() > 0x7FFF {
        return Err(StringStreamError::TooLong(units.len()));
    }
    let bit = |b: usize| raw::get_bit(data, b);
    let total = raw::bit_len(data);
    let mut out: Vec<u8> = (0..old.bit).map(bit).collect();
    match units.len() {
        0 => push_bits(&mut out, 0b10, 2),
        n @ 1..=255 => {
            push_bits(&mut out, 0b01, 2);
            push_bits(&mut out, n as u64, 8);
        }
        n => {
            push_bits(&mut out, 0b00, 2);
            push_le16(&mut out, n as u16);
        }
    }
    for u in &units {
        push_le16(&mut out, *u);
    }
    out.extend((old.end..ss.size_field).map(bit));
    let new_size = out.len() - ss.start;
    if new_size & 0x8000 != 0 {
        return Err(StringStreamError::LargeStream);
    }
    push_le16(&mut out, new_size as u16);
    out.push(1); // has-strings flag
    let new_data_end = out.len();
    out.extend((ss.data_end..total).map(bit));
    while !out.len().is_multiple_of(8) {
        out.push(0);
    }
    let mut bytes = vec![0u8; out.len() / 8];
    for (i, b) in out.iter().enumerate() {
        bytes[i / 8] |= b << (7 - i % 8);
    }
    let hb = out.len() - new_data_end;
    Ok((bytes, hb))
}

/// Rename an object: replace the stream string equal to `old_name` (the
/// last such string, as the name follows the `AeccImp…` class string)
/// with `new_name`.
pub fn set_name(
    data: &[u8],
    handle_bits: usize,
    old_name: &str,
    new_name: &str,
) -> Result<(Vec<u8>, usize), StringStreamError> {
    let ss = parse(data, handle_bits)?;
    let index = ss
        .strings
        .iter()
        .rposition(|s| s.text == old_name)
        .ok_or_else(|| StringStreamError::NotFound(old_name.into()))?;
    replace(data, handle_bits, index, new_name)
}

/// Set an object's description: strings 1 and 2 of the stream, right after
/// the `AeccImp…` class string, which hold the same text (S38 with a
/// description; empty otherwise). Line breaks are stored as `\r\n`.
pub fn set_description(
    data: &[u8],
    handle_bits: usize,
    text: &str,
) -> Result<(Vec<u8>, usize), StringStreamError> {
    let ss = parse(data, handle_bits)?;
    if ss.strings.len() < 4 || !ss.strings[0].text.starts_with("AeccImp") {
        return Err(StringStreamError::Malformed(
            "no AeccImp class string at the start".into(),
        ));
    }
    if ss.strings[1].text != ss.strings[2].text {
        return Err(StringStreamError::Malformed(
            "description copies differ".into(),
        ));
    }
    let text = text.replace("\r\n", "\n").replace('\n', "\r\n");
    let (d, hb) = replace(data, handle_bits, 1, &text)?;
    replace(&d, hb, 2, &text)
}

/// The description (string 1), if the stream has the alignment layout.
pub fn description(data: &[u8], handle_bits: usize) -> Option<String> {
    let ss = parse(data, handle_bits).ok()?;
    (ss.strings.len() >= 4 && ss.strings[0].text.starts_with("AeccImp"))
        .then(|| ss.strings[1].text.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// data bits "101", strings ["AeccImpX", "Name"], size, flag, then a
    /// 12-bit handle stream.
    fn record(name: &str) -> (Vec<u8>, usize) {
        let mut out = vec![1u8, 0, 1];
        let start = out.len();
        for s in ["AeccImpX", name] {
            let u: Vec<u16> = s.encode_utf16().collect();
            push_bits(&mut out, 0b01, 2);
            push_bits(&mut out, u.len() as u64, 8);
            for c in u {
                push_le16(&mut out, c);
            }
        }
        let size = out.len() - start;
        push_le16(&mut out, size as u16);
        out.push(1);
        let de = out.len();
        push_bits(&mut out, 0xABC, 12);
        while !out.len().is_multiple_of(8) {
            out.push(0);
        }
        let mut b = vec![0u8; out.len() / 8];
        for (i, x) in out.iter().enumerate() {
            b[i / 8] |= x << (7 - i % 8);
        }
        let hb = out.len() - de;
        (b, hb)
    }

    #[test]
    fn parses_and_renames() {
        let (d, hb) = record("Alignment - (2)");
        let ss = parse(&d, hb).unwrap();
        let t: Vec<&str> = ss.strings.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(t, ["AeccImpX", "Alignment - (2)"]);
        for new in ["A", "", "A much longer alignment name than before"] {
            let (d2, hb2) = set_name(&d, hb, "Alignment - (2)", new).unwrap();
            let ss2 = parse(&d2, hb2).unwrap();
            assert_eq!(ss2.strings[1].text, new);
            assert_eq!(ss2.strings[0].text, "AeccImpX");
            // Handle stream bits carried over.
            let v = (0..12).fold(0u32, |a, i| {
                a << 1 | raw::get_bit(&d2, ss2.data_end + i) as u32
            });
            assert_eq!(v, 0xABC);
            assert_eq!(raw::get_bit(&d2, 0), 1);
        }
    }

    #[test]
    fn description_round_trip() {
        let (d, hb) = record_n(&["AeccImpAlignment", "", "", "B", ""]);
        let (d2, hb2) = set_description(&d, hb, "line one\nline two").unwrap();
        let ss = parse(&d2, hb2).unwrap();
        assert_eq!(ss.strings[1].text, "line one\r\nline two");
        assert_eq!(ss.strings[2].text, ss.strings[1].text);
        assert_eq!(ss.strings[3].text, "B");
        assert_eq!(raw::object_name(&d2).as_deref(), Some("B"));
        assert_eq!(
            description(&d2, hb2).as_deref(),
            Some("line one\r\nline two")
        );
    }

    fn record_n(strs: &[&str]) -> (Vec<u8>, usize) {
        let mut out = vec![1u8, 0, 1];
        let start = out.len();
        for s in strs {
            let u: Vec<u16> = s.encode_utf16().collect();
            if u.is_empty() {
                push_bits(&mut out, 0b10, 2);
            } else {
                push_bits(&mut out, 0b01, 2);
                push_bits(&mut out, u.len() as u64, 8);
            }
            for c in u {
                push_le16(&mut out, c);
            }
        }
        let size = out.len() - start;
        push_le16(&mut out, size as u16);
        out.push(1);
        let de = out.len();
        push_bits(&mut out, 0x5A, 8);
        let mut b = vec![0u8; out.len().div_ceil(8)];
        for (i, x) in out.iter().enumerate() {
            b[i / 8] |= x << (7 - i % 8);
        }
        let hb = b.len() * 8 - de;
        (b, hb)
    }
}
