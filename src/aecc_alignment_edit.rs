//! Writing changed tangent geometry back into an `AECC_ALIGNMENT` record.
//!
//! Edits are same-size, in-place patches of doubles whose meaning is known
//! (docs/alignment-object.md), so the record's size, strings and handles
//! never change:
//!
//! 1. **Tangent element records**: start/end coordinates of every tangent.
//! 2. **Station-reference cache**: each record repeats a tangent's
//!    coordinates followed by a point on it. The tangent is replaced and the
//!    point keeps its distance from the tangent start (clamped to the new
//!    length).
//! 3. **Display cache**: centerline polylines and circle markers at PIs are
//!    moved; for a placeholder image the extents box is moved. The direction
//!    arrow is left alone; Civil 3D redraws the display cache.
//!
//! Not touched: the unidentified 64-bit field in the record header that
//! changes on every Civil 3D save, and label-group records. A file edited
//! without them has been opened, used and re-saved by Civil 3D 2024.

use crate::aecc_alignment::{DecodedAlignment, TangentRecord};
use crate::aecc_arc_edit::{elements_end_bit, find_station_copies, station_of};
use crate::aecc_elements::{resolve, ArcDef, DecodedElements, Element, Segment};
use crate::alignment::{Alignment, Point2};
use crate::proxy_graphics as pg;
use crate::raw;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// The new geometry must have the same number of PIs.
    PiCountChanged { old: usize, new: usize },
    /// Arcs by three points are fixed in place; editing around them is not
    /// supported yet.
    FixedArcs,
    /// Consecutive tangents don't share a PI.
    TangentsNotConnected,
    /// The edit leaves a free arc without room on a tangent.
    DoesNotFit(String),
    /// Invalid segment number or length.
    Geometry(String),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PiCountChanged { old, new } => write!(
                f,
                "new geometry has {new} PIs, record has {old}; adding or removing tangents is not supported"
            ),
            Self::FixedArcs => write!(
                f,
                "alignment has arcs defined by three points; only tangents and free arcs can be edited"
            ),
            Self::TangentsNotConnected => write!(f, "consecutive tangents don't share a PI"),
            Self::DoesNotFit(why) => write!(f, "edit doesn't fit: {why}"),
            Self::Geometry(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for EditError {}

/// What an edit changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditReport {
    pub tangent_records: usize,
    pub cache_records: usize,
    pub graphics_vertices: usize,
    pub graphics_markers: usize,
    pub placeholder_box: bool,
}

fn same(a: Point2, b: Point2) -> bool {
    a.distance_to(b) <= 1e-9 * (1.0 + a.x.abs().max(a.y.abs()))
}

fn write_tangent(data: &mut [u8], bit: usize, a: Point2, b: Point2) {
    raw::set_f64(data, bit, a.x);
    raw::set_f64(data, bit + 64, a.y);
    raw::set_f64(data, bit + 128, b.x);
    raw::set_f64(data, bit + 192, b.y);
}

/// Patch `data` (the record `old` was decoded from) so it stores `new`.
pub fn apply(
    data: &mut [u8],
    old: &DecodedAlignment,
    new: &Alignment,
) -> Result<EditReport, EditError> {
    let old_pis = old.alignment.pis().to_vec();
    let new_pis = new.pis().to_vec();
    if old_pis.len() != new_pis.len() {
        return Err(EditError::PiCountChanged {
            old: old_pis.len(),
            new: new_pis.len(),
        });
    }
    let mut rep = EditReport::default();
    let element_coords: Vec<usize> = old.tangents.iter().map(|t| t.coords_bit).collect();

    for (i, t) in old.tangents.iter().enumerate() {
        let (oa, ob) = (old_pis[i], old_pis[i + 1]);
        let (na, nb) = (new_pis[i], new_pis[i + 1]);
        if same(oa, na) && same(ob, nb) {
            continue;
        }
        // 1. The element record itself.
        write_tangent(data, t.coords_bit, na, nb);
        rep.tangent_records += 1;

        // 2. Cache records: same four doubles elsewhere, then a point.
        let hits = raw::find_f64s(data, &[oa.x, oa.y, ob.x, ob.y], 0, raw::bit_len(data));
        let old_len = oa.distance_to(ob);
        let new_len = na.distance_to(nb);
        for bit in hits.into_iter().filter(|b| !element_coords.contains(b)) {
            write_tangent(data, bit, na, nb);
            rep.cache_records += 1;
            let pb = bit + 256;
            if pb + 128 > raw::bit_len(data) {
                continue;
            }
            let p = Point2::new(raw::get_f64(data, pb), raw::get_f64(data, pb + 64));
            let (dx, dy) = ((ob.x - oa.x) / old_len, (ob.y - oa.y) / old_len);
            let (wx, wy) = (p.x - oa.x, p.y - oa.y);
            let along = wx * dx + wy * dy;
            let off = wx * dy - wy * dx;
            let tol = 1e-6 * (1.0 + old_len);
            if p.x.is_finite()
                && p.y.is_finite()
                && off.abs() < tol
                && along > -tol
                && along < old_len + tol
            {
                let s = along.clamp(0.0, new_len);
                let (ux, uy) = ((nb.x - na.x) / new_len, (nb.y - na.y) / new_len);
                raw::set_f64(data, pb, na.x + ux * s);
                raw::set_f64(data, pb + 64, na.y + uy * s);
            }
        }
    }

    // 3. Display cache.
    if let Some(g) = pg::find(data, 4096) {
        if g.is_placeholder() {
            rep.placeholder_box = patch_placeholder_box(data, &g, &old_pis, &new_pis);
        } else {
            for c in &g.chunks {
                match c.opcode {
                    pg::OP_POLYLINE => {
                        for (bit, v) in pg::poly_vertices(data, c) {
                            if let Some(i) = old_pis
                                .iter()
                                .position(|p| same(*p, Point2::new(v[0], v[1])))
                            {
                                if !same(old_pis[i], new_pis[i]) {
                                    raw::set_f64(data, bit, new_pis[i].x);
                                    raw::set_f64(data, bit + 64, new_pis[i].y);
                                    rep.graphics_vertices += 1;
                                }
                            }
                        }
                    }
                    pg::OP_CIRCLE => {
                        let (bit, v) = pg::circle_center(data, c);
                        if let Some(i) = old_pis
                            .iter()
                            .position(|p| same(*p, Point2::new(v[0], v[1])))
                        {
                            if !same(old_pis[i], new_pis[i]) {
                                raw::set_f64(data, bit, new_pis[i].x);
                                raw::set_f64(data, bit + 64, new_pis[i].y);
                                rep.graphics_markers += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(rep)
}

/// What an edit on an alignment with free arcs changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ElementsEditReport {
    pub patches: EditReport,
    /// Free arcs whose cached PC station moved, and copies updated.
    pub arcs_restationed: usize,
    pub station_copies: usize,
}

/// Change the PI-to-PI length of tangent `segment` (0-based, in chain
/// order) of an alignment made of tangents and free arcs (fillets).
///
/// Free arcs store only a radius and their tangents keep PI-to-PI
/// coordinates (docs/alignment-object.md), so the edit is the same PI edit
/// as for a tangent-only alignment (D-003): the tangent records, cache
/// records and display cache are patched by [`apply`]. In addition the
/// cached PC station of every free arc whose station changes is updated.
/// The edit is refused if a free arc would no longer fit.
pub fn set_tangent_length(
    data: &mut [u8],
    decoded: &DecodedElements,
    segment: usize,
    new_length: f64,
) -> Result<ElementsEditReport, EditError> {
    let mut target = pi_alignment(decoded)?;
    target
        .set_segment_length(segment, new_length)
        .map_err(|e| EditError::Geometry(e.to_string()))?;
    set_pis(data, decoded, target.pis())
}

/// The PI polyline of an alignment made of tangents and free arcs: the
/// tangent records' PI-to-PI coordinates, in chain order.
pub fn pi_alignment(decoded: &DecodedElements) -> Result<Alignment, EditError> {
    Ok(pi_polyline(decoded)?.1)
}

fn pi_polyline(decoded: &DecodedElements) -> Result<(Vec<TangentRecord>, Alignment), EditError> {
    if decoded
        .elements
        .iter()
        .any(|e| matches!(&e.record, Element::Arc(a) if matches!(a.def, ArcDef::ThreePoint { .. })))
    {
        return Err(EditError::FixedArcs);
    }
    let tangents: Vec<TangentRecord> = decoded
        .elements
        .iter()
        .filter_map(|e| match &e.record {
            Element::Tangent(t) => Some(t.clone()),
            Element::Arc(_) => None,
        })
        .collect();
    if tangents.windows(2).any(|w| !same(w[0].end, w[1].start)) {
        return Err(EditError::TangentsNotConnected);
    }
    let mut pis: Vec<Point2> = tangents.iter().map(|t| t.start).collect();
    pis.push(tangents.last().ok_or(EditError::TangentsNotConnected)?.end);
    let start = decoded.stored_start_station.unwrap_or(0.0);
    let name = decoded.name.clone().unwrap_or_default();
    let old_alignment =
        Alignment::new(name, start, pis).map_err(|e| EditError::Geometry(e.to_string()))?;
    Ok((tangents, old_alignment))
}

/// Move every PI of an alignment made of tangents and free arcs to
/// `new_pis` (same count). Tangent records, cache records and the display
/// cache are patched by [`apply`]; free arcs keep their radius and follow
/// their tangents, and their cached PC stations are updated. The edit is
/// refused if a curve would no longer fit.
///
/// This is how a new alignment is produced from a template record of the
/// same shape (same number of tangents and curves in the same places).
pub fn set_pis(
    data: &mut [u8],
    decoded: &DecodedElements,
    new_pis: &[Point2],
) -> Result<ElementsEditReport, EditError> {
    let (tangents, old_alignment) = pi_polyline(decoded)?;
    let start = old_alignment.start_station;
    if new_pis.len() != old_alignment.pis().len() {
        return Err(EditError::PiCountChanged {
            old: old_alignment.pis().len(),
            new: new_pis.len(),
        });
    }
    let new_alignment = Alignment::new(old_alignment.name.clone(), start, new_pis.to_vec())
        .map_err(|e| EditError::Geometry(e.to_string()))?;

    // Resolve the edited chain and check every tangent still has room.
    let new_pis = new_alignment.pis();
    let mut k = 0;
    let records: Vec<Element> = decoded
        .elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => {
                let mut t = t.clone();
                t.start = new_pis[k];
                t.end = new_pis[k + 1];
                k += 1;
                Element::Tangent(t)
            }
            other => other.clone(),
        })
        .collect();
    let resolved = resolve(&records).map_err(|e| EditError::DoesNotFit(e.to_string()))?;
    for (rec, seg) in records.iter().zip(&resolved) {
        if let (Element::Tangent(t), Segment::Line { start: s0, end: e0 }) = (rec, seg.segment) {
            let (dx, dy) = (t.end.x - t.start.x, t.end.y - t.start.y);
            if (e0.x - s0.x) * dx + (e0.y - s0.y) * dy < -1e-9 * (dx * dx + dy * dy) {
                return Err(EditError::DoesNotFit(format!(
                    "tangent {} would be shorter than its curves need",
                    t.id
                )));
            }
        }
    }

    // Stations of free arcs before and after; find all copies first so one
    // arc's new value is never mistaken for another's old one.
    let from = elements_end_bit(decoded);
    let mut station_patches = Vec::new();
    let mut arcs_restationed = 0;
    for (j, e) in decoded.elements.iter().enumerate() {
        if matches!(&e.record, Element::Arc(a) if matches!(a.def, ArcDef::Free { .. })) {
            let old = station_of(&decoded.elements, j, start);
            let new = station_of(&resolved, j, start);
            if (old - new).abs() > 1e-12 * old.abs().max(1.0) {
                arcs_restationed += 1;
                for b in find_station_copies(data, from, old) {
                    station_patches.push((b, new));
                }
            }
        }
    }

    // The station at the end of the first element is cached as the first
    // bit-coded double after the element records (S5: 100, S24: 88.197,
    // S25: the length of a single-tangent alignment). It doubles as the
    // first PC station when element 1 is a free arc, so skip it if the arc
    // patches already cover it.
    if !decoded.elements.is_empty() {
        let old_end0 = station_of(&decoded.elements, 1, start);
        let new_end0 = station_of(&resolved, 1, start);
        if (old_end0 - new_end0).abs() > 1e-12 * old_end0.abs().max(1.0) {
            if let Some(&b) = find_station_copies(data, from, old_end0).first() {
                if !station_patches.iter().any(|&(p, _)| p == b) {
                    station_patches.push((b, new_end0));
                }
            }
        }
    }

    let old = DecodedAlignment {
        handle: decoded.handle,
        tangents,
        stored_start_station: decoded.stored_start_station,
        alignment: old_alignment,
    };
    let patches = apply(data, &old, &new_alignment)?;
    for &(b, v) in &station_patches {
        raw::set_f64(data, b, v);
    }
    Ok(ElementsEditReport {
        patches,
        arcs_restationed,
        station_copies: station_patches.len(),
    })
}

fn extents(pts: &[Point2]) -> (f64, f64, f64, f64) {
    pts.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(a, b, c, d), p| (a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)),
    )
}

/// Move the corners of the placeholder's extents box to the new extents.
fn patch_placeholder_box(
    data: &mut [u8],
    g: &pg::Graphics,
    old: &[Point2],
    new: &[Point2],
) -> bool {
    let (ox0, oy0, ox1, oy1) = extents(old);
    let (nx0, ny0, nx1, ny1) = extents(new);
    let map = |v: f64, lo: f64, hi: f64, nlo: f64, nhi: f64| {
        if v == lo {
            Some(nlo)
        } else if v == hi {
            Some(nhi)
        } else {
            None
        }
    };
    let mut changed = false;
    for c in g.chunks.iter().filter(|c| c.opcode == pg::OP_POLYLINE) {
        let verts = pg::poly_vertices(data, c);
        // Only a box whose corners are all on the old extents.
        let mapped: Option<Vec<(usize, f64, f64)>> = verts
            .iter()
            .map(|(bit, v)| {
                Some((
                    *bit,
                    map(v[0], ox0, ox1, nx0, nx1)?,
                    map(v[1], oy0, oy1, ny0, ny1)?,
                ))
            })
            .collect();
        if let Some(m) = mapped {
            for (bit, x, y) in m {
                raw::set_f64(data, bit, x);
                raw::set_f64(data, bit + 64, y);
            }
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aecc_alignment::{decode, TANGENT_HEADER};
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
        fn tangent(&mut self, id: i32, before: i32, after: i32, a: (f64, f64), b: (f64, f64)) {
            for v in [TANGENT_HEADER, 0, id, before, after] {
                self.bl(v);
            }
            for v in [a.0, a.1, b.0, b.1] {
                self.rd(v);
            }
        }
        fn bd(&mut self, v: f64) {
            self.bit(0);
            self.bit(0);
            self.rd(v);
        }
        fn free_arc(&mut self, ids: (i32, i32, i32), radius: f64) {
            self.bl(TANGENT_HEADER);
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

    /// S11-like: S5's tangents with a free R=50 arc at PI (100,0) and two
    /// cached PC-station copies.
    fn s11_like() -> RawRecord {
        let mut w = Bits(Vec::new(), 0);
        w.tangent(1, 0, 4, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 4, 3, (100.0, 0.0), (200.0, 50.0));
        w.tangent(3, 2, 0, (200.0, 50.0), (300.0, 50.0));
        w.free_arc((4, 1, 2), 50.0);
        w.bit(1);
        w.bd(88.1966011250105);
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
    fn s15_edit_downstream_of_fillet_keeps_pc_station() {
        use crate::aecc_elements::decode_elements;
        let mut rec = s11_like();
        let before = decode_elements(&rec).unwrap();
        let rep = set_tangent_length(&mut rec.data, &before, 2, 200.0).unwrap();
        assert_eq!(rep.patches.tangent_records, 1);
        assert_eq!((rep.arcs_restationed, rep.station_copies), (0, 0));
        let after = decode_elements(&rec).unwrap();
        assert!((after.length() - 411.378981575051).abs() < 1e-9);
        assert_eq!(after.elements[3].segment.end(), Point2::new(400.0, 50.0));
    }

    #[test]
    fn edit_upstream_of_fillet_restations_it() {
        use crate::aecc_elements::decode_elements;
        let mut rec = s11_like();
        let before = decode_elements(&rec).unwrap();
        let rep = set_tangent_length(&mut rec.data, &before, 0, 120.0).unwrap();
        assert_eq!((rep.arcs_restationed, rep.station_copies), (1, 2));
        let t = 50.0 * (0.5f64.atan() / 2.0).tan();
        let new_pc = 120.0 - t;
        assert_eq!(find_station_copies(&rec.data, 0, new_pc).len(), 2);
        let after = decode_elements(&rec).unwrap();
        assert!((after.elements[0].segment.length() - new_pc).abs() < 1e-9);
    }

    /// Template use: S11's shape rewritten as 0,0 -> 100,0 -> 100,75 ->
    /// 100,100 keeps the R=50 fillet, now a 90° turn.
    #[test]
    fn set_pis_rewrites_s11_into_a_right_angle() {
        use crate::aecc_elements::{decode_elements, Segment};
        let mut rec = s11_like();
        let before = decode_elements(&rec).unwrap();
        let pis = [
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(100.0, 75.0),
            Point2::new(100.0, 100.0),
        ];
        let rep = set_pis(&mut rec.data, &before, &pis).unwrap();
        assert_eq!(rep.patches.tangent_records, 2); // tangent 1 unchanged
        assert_eq!((rep.arcs_restationed, rep.station_copies), (1, 2));
        let after = decode_elements(&rec).unwrap();
        // Search after the element records: the radius (also 50) is inside them.
        let from = elements_end_bit(&after);
        assert_eq!(find_station_copies(&rec.data, from, 50.0).len(), 2);
        let Segment::Arc {
            start,
            end,
            geometry: g,
        } = after.elements[1].segment
        else {
            panic!()
        };
        assert!(start.distance_to(Point2::new(50.0, 0.0)) < 1e-9);
        assert!(end.distance_to(Point2::new(100.0, 50.0)) < 1e-9);
        assert!(g.center.distance_to(Point2::new(50.0, 50.0)) < 1e-9);
        assert!((g.sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((after.length() - (100.0 + 25.0 * std::f64::consts::PI)).abs() < 1e-9);
        assert_eq!(
            set_pis(&mut rec.data, &before, &pis[..3]),
            Err(EditError::PiCountChanged { old: 4, new: 3 })
        );
    }

    #[test]
    fn edit_that_squeezes_a_fillet_is_refused() {
        use crate::aecc_elements::decode_elements;
        let mut rec = s11_like();
        let before = decode_elements(&rec).unwrap();
        // T = 11.80 needs tangent 1 to be at least that long.
        assert!(matches!(
            set_tangent_length(&mut rec.data, &before, 0, 5.0),
            Err(EditError::DoesNotFit(_))
        ));
        assert_eq!(rec.data, s11_like().data);
    }

    /// S5-like record: three tangents plus one cache record on tangent 2
    /// whose point is 50 along it.
    fn sample() -> RawRecord {
        let mut w = Bits(Vec::new(), 0);
        w.bit(1);
        w.tangent(1, 0, 2, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 1, 3, (100.0, 0.0), (200.0, 0.0));
        w.tangent(3, 2, 0, (200.0, 0.0), (300.0, 50.0));
        w.bit(1);
        w.bit(1);
        for v in [100.0, 0.0, 200.0, 0.0, 150.0, 0.0] {
            w.rd(v);
        }
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
    fn lengthening_segment_2_moves_downstream_and_cache() {
        let mut rec = sample();
        let before = decode(&rec).unwrap();
        let mut target = before.alignment.clone();
        target.set_segment_length(1, 110.0).unwrap();
        let rep = apply(&mut rec.data, &before, &target).unwrap();
        assert_eq!(rep.tangent_records, 2);
        assert_eq!(rep.cache_records, 1);

        let after = decode(&rec).unwrap();
        assert_eq!(
            after.alignment.pis(),
            &[
                Point2::new(0.0, 0.0),
                Point2::new(100.0, 0.0),
                Point2::new(210.0, 0.0),
                Point2::new(310.0, 50.0)
            ]
        );
        // Cache record now holds the new tangent 2 and the same point (50 along).
        let hits = raw::find_f64s(
            &rec.data,
            &[100.0, 0.0, 210.0, 0.0, 150.0, 0.0],
            0,
            raw::bit_len(&rec.data),
        );
        assert_eq!(hits.len(), 1);
        // Size unchanged.
        assert_eq!(rec.data.len(), sample().data.len());
    }

    #[test]
    fn unchanged_geometry_is_a_no_op() {
        let mut rec = sample();
        let before = decode(&rec).unwrap();
        let rep = apply(&mut rec.data, &before, &before.alignment).unwrap();
        assert_eq!(rep, EditReport::default());
        assert_eq!(rec.data, sample().data);
    }

    #[test]
    fn rejects_pi_count_change() {
        let mut rec = sample();
        let before = decode(&rec).unwrap();
        let fewer =
            Alignment::new("x", 0.0, vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)]).unwrap();
        assert_eq!(
            apply(&mut rec.data, &before, &fewer),
            Err(EditError::PiCountChanged { old: 4, new: 2 })
        );
    }

    /// Two tangents, then (like S5/S25) the end station of element 0 as
    /// the first BD after the elements, then a BD 100.0 that is not a
    /// station (must stay).
    #[test]
    fn set_pis_updates_first_element_end_station() {
        let mut w = Bits(Vec::new(), 0);
        w.tangent(1, 0, 2, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 1, 0, (100.0, 0.0), (200.0, 0.0));
        w.bit(1);
        w.bd(99.99999999999999);
        w.byte(0x41);
        w.bd(100.0);
        w.byte(0);
        let mut rec = RawRecord {
            handle: 1,
            handle_bits: 0,
            class: "AECC_ALIGNMENT".into(),
            is_entity: true,
            data: w.0,
        };
        let before = crate::aecc_elements::decode_elements(&rec).unwrap();
        let from = elements_end_bit(&before);
        set_pis(
            &mut rec.data,
            &before,
            &[
                Point2::new(0.0, 0.0),
                Point2::new(150.0, 0.0),
                Point2::new(250.0, 0.0),
            ],
        )
        .unwrap();
        let copies = find_station_copies(&rec.data, from, 150.0);
        assert_eq!(copies.len(), 1);
        assert_eq!(find_station_copies(&rec.data, from, 100.0).len(), 1);
    }
}
