//! Offset alignments (type 2; S255): the offset parameters stored after
//! the alignment type (read-only).
//!
//! An offset alignment's elements are ordinary element records holding the
//! offset geometry: tangents moved sideways, and spirals stored as the
//! parent's spiral plus the offset (`aecc_elements::SPIRAL_OFFSET`). After
//! `BL type` (2) a centerline's `BL 2, 2, 2` and masking list are replaced
//! by this block (docs/alignment-object.md, "Offset alignments"):
//!
//! ```text
//! BL 1, BL 2, BL 1                       (2, 2, 2 once the parent is
//!                                         deleted; MIST19)
//! BD offset                              (negative to the left)
//! BL side                                (1 left, 2 right)
//! 2 bits, BL 1, BL 1 | 3                 (MIST19)
//! BD offset                              (again)
//! BD parent start station, BD parent end station
//! BD start station, BD end station       (of the offset alignment)
//! BL 2, BL first element ID, BL last element ID
//! BL 0 × 5, BL 3, BL 0, BL n, n × {BL i (from 1), BL −1, BL 1, BL id}
//! …                                      (not decoded: BL 2, BD, GUID,
//!                                         BD, GUID, …)
//! ```
//!
//! The parent is a soft pointer after the check set
//! ([`crate::aecc_alignment_refs::offset_parent`]).

use crate::aecc_build::{self, BuildError};
use crate::raw::{self, RawRecord};

/// Alignment type of an offset alignment.
pub const TYPE_OFFSET: i32 = 2;

/// Which side of the parent an offset alignment lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// The decoded part of an offset alignment's parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetInfo {
    /// The block's first three BLs: `1, 2, 1` with a parent, `2, 2, 2`
    /// after the parent was deleted (S256; MIST19).
    pub head: [i32; 3],
    /// The offset as stored: negative to the left of the parent.
    pub offset: f64,
    pub side: Side,
    /// The parent's station range the offset follows.
    pub parent_start: f64,
    pub parent_end: f64,
    /// The offset alignment's own station range.
    pub start: f64,
    pub end: f64,
    pub first_element: i32,
    pub last_element: i32,
}

impl OffsetInfo {
    /// The offset as a distance to the left (negative to the right).
    pub fn left_offset(&self) -> f64 {
        -self.offset
    }
}

fn bad(what: &str) -> BuildError {
    BuildError::Layout(format!("offset parameters: {what}"))
}

/// The offset parameters of `rec`, or `None` when it is not an offset
/// alignment.
pub fn offset_info(rec: &RawRecord) -> Result<Option<OffsetInfo>, BuildError> {
    let data = &rec.data;
    let q = aecc_build::type_bit(rec, data)?;
    let bl = |q: usize| raw::read_bl(data, q).ok_or_else(|| bad("truncated"));
    let bd = |q: usize| raw::read_bd(data, q).ok_or_else(|| bad("truncated"));
    let (ty, q) = bl(q)?;
    if ty != TYPE_OFFSET {
        return Ok(None);
    }
    let mut head = [0; 3];
    let mut q = q;
    for (i, h) in head.iter_mut().enumerate() {
        let (v, n) = bl(q)?;
        if !(1..=2).contains(&v) || (i == 1 && v != 2) {
            return Err(bad("unknown block head"));
        }
        *h = v;
        q = n;
    }
    let (offset, q) = bd(q)?;
    let (side, q) = bl(q)?;
    let side = match side {
        1 => Side::Left,
        2 => Side::Right,
        _ => return Err(bad("unknown side")),
    };
    let (_, q) = bl(q + 2)?;
    let (_, q) = bl(q)?;
    let (again, q) = bd(q)?;
    let (parent_start, q) = bd(q)?;
    let (parent_end, q) = bd(q)?;
    let (start, q) = bd(q)?;
    let (end, q) = bd(q)?;
    let (_, q) = bl(q)?;
    let (first_element, q) = bl(q)?;
    let (last_element, _) = bl(q)?;
    if again != offset
        || !offset.is_finite()
        || ![parent_start, parent_end, start, end]
            .iter()
            .all(|v| v.is_finite())
    {
        return Err(bad("values out of shape"));
    }
    Ok(Some(OffsetInfo {
        head,
        offset,
        side,
        parent_start,
        parent_end,
        start,
        end,
        first_element,
        last_element,
    }))
}
