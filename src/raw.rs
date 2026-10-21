//! Raw access to Civil 3D (AECC) records.
//!
//! acadrust does not understand AECC classes, so it keeps each such record
//! as opaque bytes (`UnknownEntity::raw_dwg_data` for entities,
//! `ObjectType::Unknown { raw_dwg_data, .. }` for objects) and writes them
//! back verbatim. This module exposes those bytes plus bit-level helpers for
//! studying them.
//!
//! DWG object data is a bit stream: bytes are consumed most-significant bit
//! first, and a raw multi-byte value (a "RD" double or "RL" long in the
//! public ODA specification) is a run of whole stream bytes taken at any bit
//! offset and assembled little-endian. The helpers below read such values at
//! arbitrary bit offsets.

use acadrust::document::CadDocument;
use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
use std::path::Path;

/// One AECC record as stored in the file.
#[derive(Debug, Clone, PartialEq)]
pub struct RawRecord {
    /// Object handle.
    pub handle: u64,
    /// DXF class name, e.g. `AECC_ALIGNMENT`.
    pub class: String,
    /// True if the record is an entity (has graphics, lives in a block).
    pub is_entity: bool,
    /// Record bytes as acadrust stores them (merged data/string/handle
    /// streams, without the size prefix and CRC).
    pub data: Vec<u8>,
}

/// All records of DXF class `class` in `doc`, sorted by handle.
pub fn records(doc: &CadDocument, class: &str) -> Vec<RawRecord> {
    let mut out = Vec::new();
    for e in doc.entities() {
        if let EntityType::Unknown(u) = e {
            if u.dxf_name == class {
                if let Some(d) = &u.raw_dwg_data {
                    out.push(RawRecord {
                        handle: u.common.handle.value(),
                        class: u.dxf_name.clone(),
                        is_entity: true,
                        data: d.clone(),
                    });
                }
            }
        }
    }
    for (h, o) in &doc.objects {
        if let ObjectType::Unknown {
            type_name,
            raw_dwg_data: Some(d),
            ..
        } = o
        {
            if type_name == class {
                out.push(RawRecord {
                    handle: h.value(),
                    class: type_name.clone(),
                    is_entity: false,
                    data: d.clone(),
                });
            }
        }
    }
    out.sort_by_key(|r| r.handle);
    out
}

/// Mutable bytes of the AECC record with this handle (entity or object).
///
/// Changing their length is not supported by the rest of this crate; edits
/// should keep the record size.
pub fn record_data_mut(doc: &mut CadDocument, handle: u64) -> Option<&mut Vec<u8>> {
    let is_entity = doc
        .entities()
        .any(|e| matches!(e, EntityType::Unknown(u) if u.common.handle.value() == handle));
    if is_entity {
        return doc.entities_mut().find_map(|e| match e {
            EntityType::Unknown(u) if u.common.handle.value() == handle => u.raw_dwg_data.as_mut(),
            _ => None,
        });
    }
    doc.objects.iter_mut().find_map(|(h, o)| match o {
        ObjectType::Unknown { raw_dwg_data, .. } if h.value() == handle => raw_dwg_data.as_mut(),
        _ => None,
    })
}

/// Result of opening a DWG for study.
pub struct Opened {
    pub doc: CadDocument,
    /// Set when acadrust needed failsafe mode. Records it could not parse
    /// are dropped and would be missing from a saved copy.
    pub failsafe_reason: Option<String>,
}

/// Open a DWG, falling back to acadrust's failsafe mode if a normal read
/// fails (acadrust 0.6.5 cannot parse some 3DSOLID records).
pub fn open(path: impl AsRef<Path>) -> Result<Opened, acadrust::DxfError> {
    let path = path.as_ref();
    match acadrust::DwgReader::from_file(path).and_then(|mut r| r.read()) {
        Ok(doc) => Ok(Opened {
            doc,
            failsafe_reason: None,
        }),
        Err(e) => {
            let opts = acadrust::io::dwg::DwgReadOptions::failsafe();
            let doc = acadrust::DwgReader::from_file_with_options(path, opts)?.read()?;
            Ok(Opened {
                doc,
                failsafe_reason: Some(e.to_string()),
            })
        }
    }
}

/// Number of bits in `buf`.
pub fn bit_len(buf: &[u8]) -> usize {
    buf.len() * 8
}

/// Read one stream byte starting at bit offset `bit` (may straddle two
/// bytes; bits past the end read as zero).
pub fn get_byte(buf: &[u8], bit: usize) -> u8 {
    let i = bit / 8;
    let s = bit % 8;
    let b0 = buf.get(i).copied().unwrap_or(0);
    if s == 0 {
        b0
    } else {
        let b1 = buf.get(i + 1).copied().unwrap_or(0);
        (b0 << s) | (b1 >> (8 - s))
    }
}

/// Write one stream byte at bit offset `bit`.
///
/// # Panics
/// If the byte does not fit inside `buf`.
pub fn set_byte(buf: &mut [u8], bit: usize, v: u8) {
    let i = bit / 8;
    let s = bit % 8;
    if s == 0 {
        buf[i] = v;
    } else {
        buf[i] = (buf[i] & (0xFF << (8 - s))) | (v >> s);
        buf[i + 1] = (buf[i + 1] & (0xFF >> s)) | (v << (8 - s));
    }
}

/// Raw little-endian double at bit offset `bit`.
pub fn get_f64(buf: &[u8], bit: usize) -> f64 {
    let mut b = [0u8; 8];
    for (j, x) in b.iter_mut().enumerate() {
        *x = get_byte(buf, bit + 8 * j);
    }
    f64::from_le_bytes(b)
}

/// Write a raw little-endian double at bit offset `bit`.
pub fn set_f64(buf: &mut [u8], bit: usize, v: f64) {
    for (j, x) in v.to_le_bytes().iter().enumerate() {
        set_byte(buf, bit + 8 * j, *x);
    }
}

/// Raw little-endian 32-bit integer at bit offset `bit`.
pub fn get_i32(buf: &[u8], bit: usize) -> i32 {
    let mut b = [0u8; 4];
    for (j, x) in b.iter_mut().enumerate() {
        *x = get_byte(buf, bit + 8 * j);
    }
    i32::from_le_bytes(b)
}

/// One bit at offset `bit` (0 past the end).
pub fn get_bit(buf: &[u8], bit: usize) -> u8 {
    buf.get(bit / 8).map_or(0, |b| (b >> (7 - bit % 8)) & 1)
}

/// Read a bit-coded long ("BL" in the public ODA specification) at `bit`.
///
/// A 2-bit code selects the form: `00` a raw 32-bit long follows, `01` an
/// unsigned byte follows, `10` the value is 0, `11` is not used for BL.
/// Returns the value and the bit offset after it, or `None` for code `11`
/// or a read past the end.
pub fn read_bl(buf: &[u8], bit: usize) -> Option<(i32, usize)> {
    if bit + 2 > bit_len(buf) {
        return None;
    }
    match (get_bit(buf, bit), get_bit(buf, bit + 1)) {
        (0, 0) if bit + 34 <= bit_len(buf) => Some((get_i32(buf, bit + 2), bit + 34)),
        (0, 1) if bit + 10 <= bit_len(buf) => Some((get_byte(buf, bit + 2) as i32, bit + 10)),
        (1, 0) => Some((0, bit + 2)),
        _ => None,
    }
}

/// Bit offsets in `[from, to)` where the doubles `vals` occur back to back.
pub fn find_f64s(buf: &[u8], vals: &[f64], from: usize, to: usize) -> Vec<usize> {
    let pat: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut hits = Vec::new();
    if pat.is_empty() {
        return hits;
    }
    let last = to.min(bit_len(buf)).saturating_sub(pat.len() * 8);
    for bit in from..=last {
        if pat
            .iter()
            .enumerate()
            .all(|(j, p)| get_byte(buf, bit + 8 * j) == *p)
        {
            hits.push(bit);
        }
    }
    hits
}

/// Printable UTF-16LE runs of at least `min` characters, at any bit
/// alignment, as `(bit offset, text)` sorted by offset.
pub fn utf16_strings(buf: &[u8], min: usize) -> Vec<(usize, String)> {
    let nbits = bit_len(buf);
    let mut out = Vec::new();
    for phase in 0..16 {
        let mut bit = phase;
        let mut cur = String::new();
        let mut start = bit;
        while bit + 16 <= nbits {
            let lo = get_byte(buf, bit);
            let hi = get_byte(buf, bit + 8);
            if hi == 0 && (0x20..0x7f).contains(&lo) {
                if cur.is_empty() {
                    start = bit;
                }
                cur.push(lo as char);
            } else {
                if cur.len() >= min {
                    out.push((start, cur.clone()));
                }
                cur.clear();
            }
            bit += 16;
        }
        if cur.len() >= min {
            out.push((start, cur));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// True for a string-stream entry that looks like plain text, not a path,
/// placeholder token or binary noise read as text.
fn is_clean_text(s: &str) -> bool {
    let alnum = s.chars().filter(|c| c.is_ascii_alphanumeric()).count();
    !s.contains('\\') && !s.starts_with('<') && !s.contains('@') && alnum * 2 >= s.len()
}

/// The plain strings that follow the implementation class name
/// (`AeccImp…`) in a record's string stream, in order, up to the first
/// path, token or noise string.
///
/// For alignments this is `[name]` or `[user, name]`: an optional user name
/// (seen after edits) precedes the object name (docs/alignment-object.md).
pub fn strings_after_impl(data: &[u8]) -> Vec<String> {
    let strs = utf16_strings(data, 3);
    let Some(i) = strs.iter().position(|(_, s)| s.starts_with("AeccImp")) else {
        return Vec::new();
    };
    strs[i + 1..]
        .iter()
        .map(|(_, s)| s.clone())
        .take_while(|s| is_clean_text(s))
        .collect()
}

/// Object name from the string stream: the last plain string after the
/// implementation class name.
pub fn object_name(data: &[u8]) -> Option<String> {
    strings_after_impl(data).pop()
}

/// Ranges `[start, end)` of bits that differ between two equal-length
/// buffers. Runs closer than `merge` bits are merged.
pub fn changed_bit_ranges(a: &[u8], b: &[u8], merge: usize) -> Vec<(usize, usize)> {
    let n = bit_len(a).min(bit_len(b));
    let bit = |buf: &[u8], i: usize| (buf[i / 8] >> (7 - i % 8)) & 1;
    let mut out: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        if bit(a, i) != bit(b, i) {
            match out.last_mut() {
                Some(r) if i <= r.1 + merge => r.1 = i + 1,
                _ => out.push((i, i + 1)),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_roundtrip_at_every_bit_offset() {
        for off in 0..24 {
            let mut buf = vec![0xA5u8; 24];
            let before = buf.clone();
            set_f64(&mut buf, off, 1191137.1451780864);
            assert_eq!(get_f64(&buf, off), 1191137.1451780864);
            // Bits before and after the field are untouched.
            let changed = changed_bit_ranges(&before, &buf, 0);
            assert!(changed.iter().all(|&(s, e)| s >= off && e <= off + 64));
        }
    }

    #[test]
    fn finds_doubles_at_odd_offsets() {
        let mut buf = vec![0u8; 64];
        set_f64(&mut buf, 13, 100.0);
        set_f64(&mut buf, 77, -50.0);
        assert_eq!(find_f64s(&buf, &[100.0, -50.0], 0, bit_len(&buf)), vec![13]);
        assert!(find_f64s(&buf, &[-50.0, 100.0], 0, bit_len(&buf)).is_empty());
    }

    #[test]
    fn reads_utf16_at_odd_offset() {
        let mut buf = vec![0u8; 32];
        for (i, c) in "TEST".bytes().enumerate() {
            set_byte(&mut buf, 5 + i * 16, c);
            set_byte(&mut buf, 5 + i * 16 + 8, 0);
        }
        assert!(utf16_strings(&buf, 3).contains(&(5, "TEST".to_string())));
    }

    fn put_utf16(buf: &mut [u8], bit: usize, text: &str) -> usize {
        for (i, c) in text.bytes().enumerate() {
            set_byte(buf, bit + i * 16, c);
            set_byte(buf, bit + i * 16 + 8, 0);
        }
        bit + text.len() * 16 + 16
    }

    #[test]
    fn object_name_skips_optional_user_string() {
        let mut buf = vec![0u8; 256];
        let k = put_utf16(&mut buf, 3, "AeccImpAlignment");
        let k = put_utf16(&mut buf, k, "felix");
        let k = put_utf16(&mut buf, k, "Alignment - (2)");
        put_utf16(&mut buf, k, "C:\\Data\\");
        assert_eq!(strings_after_impl(&buf), vec!["felix", "Alignment - (2)"]);
        assert_eq!(object_name(&buf).as_deref(), Some("Alignment - (2)"));

        let mut buf = vec![0u8; 128];
        let k = put_utf16(&mut buf, 0, "AeccImpAlignment");
        put_utf16(&mut buf, k, "CB17");
        assert_eq!(object_name(&buf).as_deref(), Some("CB17"));

        // Binary noise that decodes as text ends the run.
        let mut buf = vec![0u8; 160];
        let k = put_utf16(&mut buf, 0, "AeccImpAlignment");
        let k = put_utf16(&mut buf, k, "VAULT PROFILE CONTROL");
        put_utf16(&mut buf, k, "'*)");
        assert_eq!(object_name(&buf).as_deref(), Some("VAULT PROFILE CONTROL"));
    }

    #[test]
    fn changed_ranges_merge_nearby_bits() {
        let a = vec![0u8; 4];
        let mut b = a.clone();
        b[0] = 0b1000_0001; // bits 0 and 7
        b[3] = 0b0000_0001; // bit 31
        assert_eq!(changed_bit_ranges(&a, &b, 8), vec![(0, 8), (31, 32)]);
        assert_eq!(
            changed_bit_ranges(&a, &b, 0),
            vec![(0, 1), (7, 8), (31, 32)]
        );
    }
}
