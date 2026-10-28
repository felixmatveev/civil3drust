//! Editing free arcs (fillets) in an `AECC_ALIGNMENT` record.
//!
//! A free arc stores only its radius; its position follows from the
//! neighbouring PI-to-PI tangents (docs/alignment-object.md, S11). Changing
//! the radius in Civil 3D (S11 → S12) changed, in the alignment record:
//!
//! 1. the radius (a bit-coded double after the arc's links), and
//! 2. two cached copies of the arc's PC station (bit-coded doubles after
//!    the element records),
//!
//! plus the volatile header field and a short block of bit-coded doubles
//! (2, 3, 4) that civil3drust does not write. Tangent records don't change.
//!
//! [`set_free_arc_radius`] patches (1) and (2) in place. The radius must
//! already be stored as a full double (code `00`), which is the case for
//! any radius other than 0 or 1.

use crate::aecc_alignment::{DecodeError, Linked};
use crate::aecc_elements::{resolve, ArcDef, DecodedElements, Element, Segment};
use crate::proxy_graphics as pg;
use crate::raw;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ArcEditError {
    NoSuchElement(i32),
    /// The element isn't a free arc (radius edits on arcs by three points
    /// need different patches, not yet studied).
    NotFreeArc(i32),
    InvalidRadius(f64),
    /// The stored radius isn't a full double, so it can't be patched in
    /// place.
    RadiusNotPatchable,
    /// The new radius needs more tangent than is available.
    DoesNotFit(String),
    Decode(DecodeError),
}

impl fmt::Display for ArcEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchElement(id) => write!(f, "no element with ID {id}"),
            Self::NotFreeArc(id) => write!(f, "element {id} is not a free arc (fillet)"),
            Self::InvalidRadius(r) => write!(f, "radius must be positive and finite, got {r}"),
            Self::RadiusNotPatchable => {
                write!(
                    f,
                    "stored radius is not a full double; can't patch in place"
                )
            }
            Self::DoesNotFit(why) => write!(f, "radius doesn't fit: {why}"),
            Self::Decode(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ArcEditError {}

/// What a radius edit changed.
#[derive(Debug, Clone, PartialEq)]
pub struct RadiusReport {
    pub old_radius: f64,
    pub new_radius: f64,
    pub old_pc_station: f64,
    pub new_pc_station: f64,
    /// Cached PC-station copies found and updated.
    pub station_copies: usize,
    /// True if the record has real (non-placeholder) display graphics,
    /// which are now out of date until Civil 3D redraws them.
    pub graphics_stale: bool,
}

/// First bit after the last element record.
pub(crate) fn elements_end_bit(decoded: &DecodedElements) -> usize {
    decoded
        .elements
        .iter()
        .map(|e| e.record.end_bit())
        .max()
        .unwrap_or(0)
}

/// Bit offsets of the raw doubles of cached station copies: bit-coded
/// doubles (code `00`) after `from` whose value equals `station` to 1e-9
/// relative (Civil 3D's value can differ from ours in the last bits).
pub(crate) fn find_station_copies(data: &[u8], from: usize, station: f64) -> Vec<usize> {
    let tol = 1e-9 * station.abs().max(1.0);
    let mut hits = Vec::new();
    let mut bit = from;
    while bit + 66 <= raw::bit_len(data) {
        if raw::get_bit(data, bit) == 0
            && raw::get_bit(data, bit + 1) == 0
            && (raw::get_f64(data, bit + 2) - station).abs() <= tol
        {
            hits.push(bit + 2);
            bit += 66;
        } else {
            bit += 1;
        }
    }
    hits
}

pub(crate) fn station_of(
    elements: &[crate::aecc_elements::ResolvedElement],
    idx: usize,
    start: f64,
) -> f64 {
    start
        + elements[..idx]
            .iter()
            .map(|e| e.segment.length())
            .sum::<f64>()
}

/// Set the radius of free arc `arc_id` in `data`, the record `decoded` was
/// decoded from.
pub fn set_free_arc_radius(
    data: &mut [u8],
    decoded: &DecodedElements,
    arc_id: i32,
    new_radius: f64,
) -> Result<RadiusReport, ArcEditError> {
    if !(new_radius.is_finite() && new_radius > 0.0) {
        return Err(ArcEditError::InvalidRadius(new_radius));
    }
    let idx = decoded
        .elements
        .iter()
        .position(|e| e.id() == arc_id)
        .ok_or(ArcEditError::NoSuchElement(arc_id))?;
    let Element::Arc(arc) = &decoded.elements[idx].record else {
        return Err(ArcEditError::NotFreeArc(arc_id));
    };
    let ArcDef::Free {
        radius_bit,
        radius: old_radius,
    } = arc.def
    else {
        return Err(ArcEditError::NotFreeArc(arc_id));
    };
    if raw::get_bit(data, radius_bit) != 0 || raw::get_bit(data, radius_bit + 1) != 0 {
        return Err(ArcEditError::RadiusNotPatchable);
    }

    // Resolve with the new radius and check every tangent still has room.
    let mut records: Vec<Element> = decoded.elements.iter().map(|e| e.record.clone()).collect();
    if let Element::Arc(a) = &mut records[idx] {
        a.def = ArcDef::Free {
            radius_bit,
            radius: new_radius,
        };
    }
    let resolved = resolve(&records).map_err(ArcEditError::Decode)?;
    for (old, new) in decoded.elements.iter().zip(&resolved) {
        if let (Element::Tangent(t), Segment::Line { start, end }) = (&old.record, new.segment) {
            let (dx, dy) = (t.end.x - t.start.x, t.end.y - t.start.y);
            let along = (end.x - start.x) * dx + (end.y - start.y) * dy;
            if along < -1e-9 * (dx * dx + dy * dy) {
                return Err(ArcEditError::DoesNotFit(format!(
                    "tangent {} would be shorter than zero",
                    t.id
                )));
            }
        }
    }

    let start = decoded.stored_start_station.unwrap_or(0.0);
    let old_pc_station = station_of(&decoded.elements, idx, start);
    let new_pc_station = station_of(&resolved, idx, start);

    // 1. The radius.
    raw::set_f64(data, radius_bit + 2, new_radius);

    // 2. Cached PC stations (see `find_station_copies`).
    let hits = find_station_copies(data, elements_end_bit(decoded), old_pc_station);
    for &b in &hits {
        raw::set_f64(data, b, new_pc_station);
    }
    let copies = hits.len();

    let graphics_stale = pg::find(data, 4096).is_some_and(|g| !g.is_placeholder());
    Ok(RadiusReport {
        old_radius,
        new_radius,
        old_pc_station,
        new_pc_station,
        station_copies: copies,
        graphics_stale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aecc_alignment::ELEMENT_HEADER;
    use crate::aecc_elements::decode_elements;
    use crate::raw::RawRecord;

    struct Bits(Vec<u8>, usize);
    impl Bits {
        fn bit(&mut self, b: u8) {
            if self.1.is_multiple_of(8) {
                self.0.push(0);
            }
            if b != 0 {
                let i = self.1 / 8;
                self.0[i] |= 1 << (7 - self.1 % 8);
            }
            self.1 += 1;
        }
        fn byte(&mut self, v: u8) {
            for i in (0..8).rev() {
                self.bit((v >> i) & 1);
            }
        }
        fn bl(&mut self, v: i32) {
            if v == 0 {
                self.bit(1);
                self.bit(0);
            } else if (1..256).contains(&v) {
                self.bit(0);
                self.bit(1);
                self.byte(v as u8);
            } else {
                self.bit(0);
                self.bit(0);
                v.to_le_bytes().iter().for_each(|b| self.byte(*b));
            }
        }
        fn rd(&mut self, v: f64) {
            v.to_le_bytes().iter().for_each(|b| self.byte(*b));
        }
        fn bd(&mut self, v: f64) {
            self.bit(0);
            self.bit(0);
            self.rd(v);
        }
        fn tangent(&mut self, id: i32, before: i32, after: i32, a: (f64, f64), b: (f64, f64)) {
            for v in [ELEMENT_HEADER, 0, id, before, after] {
                self.bl(v);
            }
            for v in [a.0, a.1, b.0, b.1] {
                self.rd(v);
            }
        }
        fn free_arc(&mut self, ids: (i32, i32, i32), radius: f64) {
            self.bl(ELEMENT_HEADER);
            self.bit(0);
            self.bd(-1.0);
            self.bd(-1.0);
            self.bl(1);
            self.bd(-1.0);
            self.bl(29);
            self.bit(0);
            self.bl(ids.0);
            self.bl(ids.1);
            self.bl(ids.2);
            self.bd(radius);
        }
    }

    /// S11-like record with two cached PC-station copies after the elements.
    fn s11_like() -> RawRecord {
        let mut w = Bits(Vec::new(), 0);
        w.tangent(1, 0, 4, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 4, 3, (100.0, 0.0), (200.0, 50.0));
        w.tangent(3, 2, 0, (200.0, 50.0), (300.0, 50.0));
        w.free_arc((4, 1, 2), 50.0);
        w.bit(1);
        w.bd(88.1966011250105); // Civil 3D's rounding of the PC station
        w.byte(0x3C);
        w.bd(88.1966011250105);
        w.byte(0);
        RawRecord {
            handle: 1,
            handle_bits: 0,
            class: "AECC_ALIGNMENT".into(),
            is_entity: true,
            data: w.0,
        }
    }

    #[test]
    fn radius_50_to_75_like_s12() {
        let mut rec = s11_like();
        let before = decode_elements(&rec).unwrap();
        let rep = set_free_arc_radius(&mut rec.data, &before, 4, 75.0).unwrap();
        assert_eq!(rep.old_radius, 50.0);
        assert_eq!(rep.station_copies, 2);
        let t75 = 75.0 * (0.5f64.atan() / 2.0).tan();
        assert!((rep.new_pc_station - (100.0 - t75)).abs() < 1e-9);

        let after = decode_elements(&rec).unwrap();
        let Segment::Arc {
            geometry: g, start, ..
        } = after.elements[1].segment
        else {
            panic!()
        };
        assert_eq!(g.radius, 75.0);
        assert!((start.x - (100.0 - t75)).abs() < 1e-9);
        // Tangent records untouched (still PI to PI); size unchanged.
        assert_eq!(rec.data.len(), s11_like().data.len());
        let tang: Vec<_> = crate::aecc_elements::element_records(&rec.data)
            .into_iter()
            .filter(|e| matches!(e, Element::Tangent(_)))
            .collect();
        let tang0: Vec<_> = crate::aecc_elements::element_records(&s11_like().data)
            .into_iter()
            .filter(|e| matches!(e, Element::Tangent(_)))
            .collect();
        assert_eq!(tang, tang0);
    }

    #[test]
    fn rejects_radius_that_does_not_fit_and_non_arcs() {
        let mut rec = s11_like();
        let before = decode_elements(&rec).unwrap();
        // T = R·tan(13.28°) must stay below 100 (tangent 1): R = 1000 fails.
        assert!(matches!(
            set_free_arc_radius(&mut rec.data, &before, 4, 1000.0),
            Err(ArcEditError::DoesNotFit(_))
        ));
        assert_eq!(
            set_free_arc_radius(&mut rec.data, &before, 1, 75.0),
            Err(ArcEditError::NotFreeArc(1))
        );
        assert_eq!(
            set_free_arc_radius(&mut rec.data, &before, 9, 75.0),
            Err(ArcEditError::NoSuchElement(9))
        );
        assert_eq!(rec.data, s11_like().data);
    }
}
