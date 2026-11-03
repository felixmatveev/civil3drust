//! Design-criteria settings of an alignment (O3, S79).
//!
//! The Design Criteria check boxes are bits near the end of section 2 (the
//! bits between the proxy graphics and the element list), counted back
//! from its end: 19 criteria-based design, 18 design criteria file,
//! 17 design check set, 14 check for tangency (on by default). S79 has all
//! on; in S81 Civil 3D cleared 18, 17 and 14 of an alignment without a
//! criteria file or check set and the tangency bit, and its dialog showed exactly that
//! (owner's screenshot).
//!
//! With criteria on, Civil 3D also stores in the string stream (the five
//! strings after the name): the criteria file path, the same path with
//! `<DATADIR>`, the Minimum Radius Table, the Transition Length Table and
//! the Attainment Method; and the alignment's last handle reference points at
//! an `AECC_ALIGNMENT_DESIGN_CHECK_SET` (S74, S79; null in alignments
//! created without one). See `docs/alignment-object.md`, "Design criteria".

use crate::aecc_elements::decode_elements;
use crate::raw::{self, RawRecord};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriteriaError(pub String);

impl fmt::Display for CriteriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "design criteria: {}", self.0)
    }
}

impl std::error::Error for CriteriaError {}

/// End of section 2: the element count (BL) and an 11-bit prefix precede
/// the first element record's header (21 bits for the counts seen).
pub fn section2_end(rec: &RawRecord) -> Result<usize, CriteriaError> {
    let d = decode_elements(rec).map_err(|e| CriteriaError(e.to_string()))?;
    d.first_record_bit()
        .and_then(|b| b.checked_sub(21))
        .ok_or_else(|| CriteriaError("no elements".into()))
}

/// The Design Criteria check boxes, as stored (S79, S81).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Criteria {
    /// Use criteria-based design (19 bits before the section-2 end).
    pub criteria_based: bool,
    /// Use design criteria file (18 bits before).
    pub criteria_file: bool,
    /// Use design check set (17 bits before).
    pub check_set: bool,
    /// Check for tangency between elements (14 bits before; on by default).
    pub check_tangency: bool,
}

impl Default for Criteria {
    /// Civil 3D's defaults for a new alignment (every sample but S79/S81).
    fn default() -> Self {
        Self {
            criteria_based: false,
            criteria_file: false,
            check_set: false,
            check_tangency: true,
        }
    }
}

/// Bits before the section-2 end, in field order.
const BITS: [usize; 4] = [19, 18, 17, 14];

/// The check boxes of `rec`.
pub fn criteria(rec: &RawRecord) -> Result<Criteria, CriteriaError> {
    let e = section2_end(rec)?;
    let [a, b, c, d] = BITS.map(|k| raw::get_bit(&rec.data, e - k) == 1);
    Ok(Criteria {
        criteria_based: a,
        criteria_file: b,
        check_set: c,
        check_tangency: d,
    })
}

/// Write the check boxes into `data` (laid out like `rec`) in place. A
/// criteria file also needs its strings, and a check set the last handle
/// reference, or Civil 3D clears the box when the alignment is edited
/// (S81 `C1 flags`).
pub fn set_criteria(rec: &RawRecord, data: &mut [u8], c: Criteria) -> Result<(), CriteriaError> {
    let e = section2_end(rec)?;
    let vals = [
        c.criteria_based,
        c.criteria_file,
        c.check_set,
        c.check_tangency,
    ];
    for (k, v) in BITS.iter().zip(vals) {
        let b = e - k;
        let (byte, m) = (b / 8, 1u8 << (7 - b % 8));
        if v {
            data[byte] |= m;
        } else {
            data[byte] &= !m;
        }
    }
    Ok(())
}

/// The Constraint Editing options of Alignment Properties, as stored next
/// to the Design Criteria check boxes (MIST15, S163, S164).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstraintEditing {
    /// "Always perform implied tangency constraint swapping" (16 bits
    /// before the section-2 end; S163).
    pub swap_tangency: bool,
    /// "Lock all parameter constraints" (15 bits before; on by default).
    /// Every element header also carries it as the flag byte `0x02`
    /// (MIST7, S164).
    pub lock_parameters: bool,
}

/// The Constraint Editing options of `rec` (read only; changing the lock
/// also needs every element header changed).
pub fn constraint_editing(rec: &RawRecord) -> Result<ConstraintEditing, CriteriaError> {
    let e = section2_end(rec)?;
    Ok(ConstraintEditing {
        swap_tangency: raw::get_bit(&rec.data, e - 16) == 1,
        lock_parameters: raw::get_bit(&rec.data, e - 15) == 1,
    })
}

/// Set the Constraint Editing options. The lock is stored twice, as bit 15
/// and as the flag byte of every element header (MIST7, S164), so every
/// header is rewritten: `0x02` added when locking, removed when unlocking
/// (other flag bits are kept). Returns the new bytes and handle-stream
/// size. Alignments with spiral groups are refused (their inner headers
/// are not enumerated yet).
pub fn set_constraint_editing(
    rec: &RawRecord,
    c: ConstraintEditing,
) -> Result<(Vec<u8>, usize), CriteriaError> {
    use crate::aecc_alignment::{read_header, ELEMENT_LOCKED};
    use crate::aecc_build::{encode_header, Bits};
    let d = decode_elements(rec).map_err(|e| CriteriaError(e.to_string()))?;
    if d.has_groups() {
        return Err(CriteriaError("spiral groups not handled yet".into()));
    }
    let e = section2_end(rec)?;
    let mut data = rec.data.clone();
    for (k, v) in [(16, c.swap_tangency), (15, c.lock_parameters)] {
        let b = e - k;
        let (byte, m) = (b / 8, 1u8 << (7 - b % 8));
        if v {
            data[byte] |= m;
        } else {
            data[byte] &= !m;
        }
    }
    let mut headers: Vec<usize> = d.all().map(|x| x.record.header_bit()).collect();
    headers.sort_unstable();
    let total = raw::bit_len(&data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    let mut at = 0;
    for h in headers {
        let hd = read_header(&data, h)
            .ok_or_else(|| CriteriaError(format!("element header at bit {h} not readable")))?;
        let (new, type_byte) = if hd.is_group_member() {
            // Civil 3D's group-member form (S195): `02 0D` / `02 09`.
            crate::aecc_alignment::header_bytes(true, c.lock_parameters)
        } else {
            let rest = hd.flags.unwrap_or(0) & !ELEMENT_LOCKED;
            // A flag byte without bit 0x02 would be read as the type byte
            // (S126, S128: rejected), so unlocking drops the whole byte.
            (
                c.lock_parameters.then_some(rest | ELEMENT_LOCKED),
                hd.type_byte,
            )
        };
        w.copy(&data, at, h);
        encode_header(&mut w, new, type_byte);
        at = hd.next;
    }
    let new_data_end = data_end + w.len() - at;
    w.copy(&data, at, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    Ok((bytes, hb))
}
