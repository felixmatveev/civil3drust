//! Offset alignments (type 2; S255–S257): the offset parameters stored after
//! the alignment type (read-only).
//!
//! An offset alignment's elements are ordinary element records holding the
//! offset geometry: tangents moved sideways, and spirals stored as the
//! parent's spiral plus the offset (`aecc_elements::SPIRAL_OFFSET`). After
//! `BL type` (2) a centerline's `BL 2, 2, 2` and masking list are replaced
//! by this block (docs/alignment-object.md, "Offset alignments"):
//!
//! ```text
//! BL 1, BL 2, BL 1             (2, 2, 2 once the parent is deleted; MIST19)
//! BD nominal offset            (negative to the left)
//! BL side                      (1 left, 2 right)
//! 2 bits, BL 1                 (MIST19)
//! BL n regions                 (stretches at one offset, S257)
//! n × { BD offset, BD parent start, BD parent end, BD start, BD end,
//!       BL kind (2 nominal, 1 widening), BL first element, BL last
//!       element, BL 0 × 5, BL 3 }
//! BL m transitions             (S257)
//! m × { BL kind, BD length, BL element,
//!       kind 1 (linear): BD 0, BD 0, BL 2, BD 0
//!       kind 2 (curve-line-curve): BD 0, BD curve 1 radius, BD curve 2 radius }
//! BL k, k × {BL i (from 1), BL −1, BL count, count × BL element}  (MIST19)
//! …                            (not decoded: BL 2, BD, GUID, BD, GUID, …)
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

/// A stretch of an offset alignment at one offset (S257): the nominal
/// offset, or a widening.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetRegion {
    /// Negative to the left, like [`OffsetInfo::offset`].
    pub offset: f64,
    /// 2 at the nominal offset, 1 for a widening (S257).
    pub kind: i32,
    pub parent_start: f64,
    pub parent_end: f64,
    pub start: f64,
    pub end: f64,
    /// The elements of the stretch, as made (not renumbered, S256).
    pub first_element: i32,
    pub last_element: i32,
}

/// A widening transition (S257): `kind` 1 linear, 2 curve-line-curve.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetTransition {
    pub kind: i32,
    pub length: f64,
    /// The transition element.
    pub element: i32,
    /// Linear: BD 0, BD 0, BL 2, BD 0 (as BDs, the BL as a number);
    /// curve-line-curve: BD 0, curve 1 radius, curve 2 radius.
    pub values: Vec<f64>,
}

/// The decoded part of an offset alignment's parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetInfo {
    /// The block's first three BLs: `1, 2, 1` with a parent, `2, 2, 2`
    /// after the parent was deleted (S256; MIST19).
    pub head: [i32; 3],
    /// The nominal offset as stored: negative to the left of the parent.
    pub offset: f64,
    pub side: Side,
    /// The parent's station range the offset follows (first region's
    /// start to last region's end).
    pub parent_start: f64,
    pub parent_end: f64,
    /// The offset alignment's own station range.
    pub start: f64,
    pub end: f64,
    pub first_element: i32,
    pub last_element: i32,
    /// The stretches at one offset, in order (one without widenings).
    pub regions: Vec<OffsetRegion>,
    /// The widening transitions; `None` when a kind is not known.
    pub transitions: Option<Vec<OffsetTransition>>,
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
    let (n, mut q) = bl(q)?;
    if !(1..=1000).contains(&n) || !offset.is_finite() {
        return Err(bad("values out of shape"));
    }
    let mut regions = Vec::new();
    for _ in 0..n {
        let mut v = [0.0; 5];
        for x in &mut v {
            let (y, r) = bd(q)?;
            *x = y;
            q = r;
        }
        let (kind, r) = bl(q)?;
        let (first_element, r) = bl(r)?;
        let (last_element, mut r) = bl(r)?;
        // BL 0 × 5, BL 3.
        for want in [0, 0, 0, 0, 0, 3] {
            let (x, t) = bl(r)?;
            if x != want {
                return Err(bad("region tail"));
            }
            r = t;
        }
        q = r;
        if !v.iter().all(|x| x.is_finite()) {
            return Err(bad("values out of shape"));
        }
        regions.push(OffsetRegion {
            offset: v[0],
            kind,
            parent_start: v[1],
            parent_end: v[2],
            start: v[3],
            end: v[4],
            first_element,
            last_element,
        });
    }
    let (nt, mut q) = bl(q)?;
    let mut transitions = Some(Vec::new());
    for _ in 0..nt {
        let (kind, r) = bl(q)?;
        let (length, r) = bd(r)?;
        let (element, mut r) = bl(r)?;
        let mut values = Vec::new();
        match kind {
            1 => {
                for is_bl in [false, false, true, false] {
                    if is_bl {
                        let (x, t) = bl(r)?;
                        values.push(f64::from(x));
                        r = t;
                    } else {
                        let (x, t) = bd(r)?;
                        values.push(x);
                        r = t;
                    }
                }
            }
            2 => {
                for _ in 0..3 {
                    let (x, t) = bd(r)?;
                    values.push(x);
                    r = t;
                }
            }
            _ => {
                transitions = None;
                break;
            }
        }
        q = r;
        if let Some(t) = transitions.as_mut() {
            t.push(OffsetTransition {
                kind,
                length,
                element,
                values,
            });
        }
    }
    let (first, last) = (&regions[0], &regions[regions.len() - 1]);
    Ok(Some(OffsetInfo {
        head,
        offset,
        side,
        parent_start: first.parent_start,
        parent_end: last.parent_end,
        start: first.start,
        end: last.end,
        first_element: first.first_element,
        last_element: last.last_element,
        regions: regions.clone(),
        transitions,
    }))
}
