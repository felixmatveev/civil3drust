//! Design-criteria settings of an alignment (O3, S79).
//!
//! Three flag bits near the end of section 2 (the bits between the proxy
//! graphics and the element list), 19, 18 and 17 bits before its end:
//! all three are 1 for an alignment with design criteria switched on
//! (S79 `DESIGN ON`) and 0 otherwise (every other sample). Their individual
//! meaning is not separated yet; Civil 3D's Design Criteria tab has three
//! check boxes (criteria-based design, design criteria file, design check
//! set), which they probably are, in some order.
//!
//! With criteria on, Civil 3D also stores in the string stream (strings 2–6
//! after the name): the criteria file path, the same path with
//! `<DATADIR>`, the superelevation rate table, the number of lanes and the
//! attainment method; and the alignment's last handle reference points at
//! an `AECC_ALIGNMENT_DESIGN_CHECK_SET` (S74, S79; null in alignments
//! created without one). See `docs/alignment-object.md`, "Design criteria".

use crate::aecc_elements::decode_elements;
use crate::raw::{self, RawRecord};
use std::fmt;

/// Distance of the first flag from the end of section 2.
pub const FLAGS_FROM_END: usize = 19;

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
    d.elements
        .iter()
        .map(|e| e.record.header_bit())
        .min()
        .and_then(|b| b.checked_sub(21))
        .ok_or_else(|| CriteriaError("no elements".into()))
}

/// The three flags.
pub fn criteria_flags(rec: &RawRecord) -> Result<[bool; 3], CriteriaError> {
    let e = section2_end(rec)?;
    let b = e - FLAGS_FROM_END;
    Ok([0, 1, 2].map(|i| raw::get_bit(&rec.data, b + i) == 1))
}

/// Set the three flags in `data` (laid out like `rec`) in place.
pub fn set_criteria_flags(
    rec: &RawRecord,
    data: &mut [u8],
    flags: [bool; 3],
) -> Result<(), CriteriaError> {
    let e = section2_end(rec)?;
    let b = e - FLAGS_FROM_END;
    for (i, f) in flags.iter().enumerate() {
        let (byte, m) = ((b + i) / 8, 1u8 << (7 - (b + i) % 8));
        if *f {
            data[byte] |= m;
        } else {
            data[byte] &= !m;
        }
    }
    Ok(())
}
