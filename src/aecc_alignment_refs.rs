//! The handle references of an `AECC_ALIGNMENT` record, and changing its
//! alignment style.
//!
//! Every alignment in S1–S19 ends its handle stream with the same seven
//! references (S1 has one more in front, a hard owner to an extension
//! dictionary). Counting from the end (docs/alignment-object.md, "Handle
//! stream"):
//!
//! | from end | code | target |
//! |---------:|------|--------|
//! | 7 | 5 | layer (table record) |
//! | 6 | 4 | null |
//! | 5 | 5 | companion `AECC_FEATURE_STYLE` object (handle = alignment − 1 in every sample) |
//! | 4 | 5 | the same companion object again |
//! | 3 | 4 | null |
//! | 2 | 5 | `AECC_ALIGNMENT_STYLE` — the alignment style (S18 vs S19) |
//! | 1 | 5 | null |

use crate::handle_refs::{self, HandleRef};
use crate::raw::{self, RawRecord};
use acadrust::document::CadDocument;
use std::fmt;

/// The named references of an alignment record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentRefs {
    pub layer: HandleRef,
    pub companion: HandleRef,
    pub style: HandleRef,
}

/// Pick the named references out of `rec`'s handle stream, or `None` if it
/// doesn't have the known shape.
pub fn alignment_refs(rec: &RawRecord) -> Option<AlignmentRefs> {
    let refs = handle_refs::of_record(rec);
    let n = refs.len();
    if n < 7 {
        return None;
    }
    let t = &refs[n - 7..];
    let shape = [5, 4, 5, 5, 4, 5, 5];
    if t.iter().zip(shape).any(|(r, c)| r.code != c)
        || t[1].handle != 0
        || t[4].handle != 0
        || t[6].handle != 0
        || t[2].handle != t[3].handle
    {
        return None;
    }
    Some(AlignmentRefs {
        layer: t[0],
        companion: t[2],
        style: t[5],
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum StyleError {
    NoSuchAlignment(u64),
    UnknownLayout,
    /// The target isn't an `AECC_ALIGNMENT_STYLE` record (class given).
    NotAlignmentStyle(u64, Option<String>),
    /// The new handle needs a different number of bytes than the stored one,
    /// which would change the record size (not supported yet).
    SizeChange {
        old: u64,
        new: u64,
    },
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchAlignment(h) => write!(f, "no AECC_ALIGNMENT with handle {h:X}"),
            Self::UnknownLayout => write!(f, "alignment handle stream has an unknown layout"),
            Self::NotAlignmentStyle(h, c) => write!(
                f,
                "{h:X} is not an alignment style ({})",
                c.as_deref().unwrap_or("no such object")
            ),
            Self::SizeChange { old, new } => write!(
                f,
                "style handle {new:X} has a different byte length than {old:X}; record size would change"
            ),
        }
    }
}

impl std::error::Error for StyleError {}

/// Alignment styles in `doc`: (handle, name). The style collection, a
/// record of the same class that soft-owns every style (9B3 in S18), is
/// left out.
pub fn alignment_styles(doc: &CadDocument) -> Vec<(u64, String)> {
    raw::records(doc, "AECC_ALIGNMENT_STYLE")
        .into_iter()
        .filter(|r| !handle_refs::of_record(r).iter().any(|h| h.code == 2))
        .map(|r| {
            let n = raw::style_name(&r.data).unwrap_or_else(|| "(no name stored)".into());
            (r.handle, n)
        })
        .collect()
}

/// Point alignment `alignment` at alignment style `style`. Returns the
/// previous style handle.
pub fn set_alignment_style(
    doc: &mut CadDocument,
    alignment: u64,
    style: u64,
) -> Result<u64, StyleError> {
    let class = raw::class_of(doc, style);
    if class.as_deref() != Some("AECC_ALIGNMENT_STYLE") {
        return Err(StyleError::NotAlignmentStyle(style, class));
    }
    let rec = raw::records(doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == alignment)
        .ok_or(StyleError::NoSuchAlignment(alignment))?;
    let refs = alignment_refs(&rec).ok_or(StyleError::UnknownLayout)?;
    let old = refs.style.handle;
    let data =
        raw::record_data_mut(doc, alignment).ok_or(StyleError::NoSuchAlignment(alignment))?;
    if !handle_refs::retarget(data, &refs.style, style) {
        return Err(StyleError::SizeChange { old, new: style });
    }
    Ok(old)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(stream: &[u8], lead_bits: usize) -> RawRecord {
        // `lead_bits` zero bits of "data" before a byte-aligned stream is
        // enough for of_record, which only looks at the last handle_bits.
        let mut data = vec![0u8; lead_bits / 8];
        data.extend_from_slice(stream);
        RawRecord {
            handle: 0x8C5C,
            handle_bits: stream.len() * 8,
            class: "AECC_ALIGNMENT".into(),
            is_entity: true,
            data,
        }
    }

    const S18: [u8; 14] = [
        0x51, 0x10, 0x40, 0x52, 0x8C, 0x5B, 0x52, 0x8C, 0x5B, 0x40, 0x52, 0x8D, 0xE7, 0x50,
    ];

    #[test]
    fn named_refs_s18() {
        let r = alignment_refs(&rec(&S18, 64)).unwrap();
        assert_eq!(r.layer.handle, 0x10);
        assert_eq!(r.companion.handle, 0x8C5B);
        assert_eq!(r.style.handle, 0x8DE7);
    }

    #[test]
    fn named_refs_with_leading_xdict_like_s1() {
        let mut s = vec![0x32, 0x8C, 0x49];
        s.extend_from_slice(&S18);
        let r = alignment_refs(&rec(&s, 8)).unwrap();
        assert_eq!(r.style.handle, 0x8DE7);
    }

    #[test]
    fn rejects_unknown_shape() {
        let mut s = S18;
        s[2] = 0x50; // null soft pointer became a hard pointer
        assert!(alignment_refs(&rec(&s, 0)).is_none());
    }
}
