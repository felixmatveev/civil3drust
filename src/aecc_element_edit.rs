//! Adding and removing elements of an existing alignment in place (O6,
//! S92–S96; docs/alignment-object.md, "Adding and removing elements";
//! D-036).
//!
//! Unlike [`crate::aecc_build::set_geometry`], which rebuilds the whole
//! element list from PIs, these functions keep every other element record
//! as it is (ID, kind, constraint, parameters), following what Civil 3D
//! does:
//!
//! - a removed element's record is dropped and its neighbours are linked
//!   to each other; nothing is renumbered (S94, S95);
//! - a new element takes the highest ID + 1 and is stored last (S96);
//! - a free curve that loses the tangent before it becomes a floating
//!   curve (constraint 15) starting at its old PC (S94);
//! - removing the first element keeps the stationing: the start station
//!   becomes the old station of the new start, and both copies of the
//!   first element's ID in section 2 point at the new first element (S94);
//! - each arc has three empty strings in the string stream (S95, S96).
//!
//! Then the derived parts are rebuilt: the proxy graphics, the design-speed
//! intervals (Civil 3D leaves them stale, which breaks its own speed editor,
//! S94) and the optional element-ID list is dropped. Speeds that fall
//! before the new start move onto it; speeds past the new end are dropped.

use crate::aecc_alignment::{Linked, KIND_FIXED, KIND_FLOATING};
use crate::aecc_build::{
    clear_element_id_list, encode_free_arc, encode_tangent, replace_graphics, Bits,
    FREE_ARC_PARAMS, PREFIX_ARC,
};
use crate::aecc_design_speed::{design_speeds, set_design_speeds};
use crate::aecc_elements::{
    decode_elements, ArcDef, ArcParams, DecodedElements, Element, ResolvedElement, Segment,
    ARC_FLOATING_END, ARC_FLOATING_START, ARC_FREE,
};
use crate::alignment::Point2;
use crate::raw::{self, RawRecord};
use crate::string_stream;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementEditError(pub String);

impl fmt::Display for ElementEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "element edit: {}", self.0)
    }
}

impl std::error::Error for ElementEditError {}

fn err(m: impl Into<String>) -> ElementEditError {
    ElementEditError(m.into())
}

/// What an element edit did.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementEditReport {
    /// Elements after the edit.
    pub elements: usize,
    pub length: f64,
    pub start_station: f64,
    /// ID given to a new element.
    pub added_id: Option<i32>,
    /// Free curves turned into floating curves.
    pub floating: Vec<i32>,
    /// Design speeds after the edit, as written.
    pub speeds: Vec<(f64, f64)>,
    pub graphics_rebuilt: bool,
}

/// A change to one kept record.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Rewrite {
    /// New before/after links only.
    Links,
    /// A free curve whose previous tangent is gone: constraint 15, start
    /// point = `start` (S94).
    FloatingFrom(Point2),
    /// A free curve whose next tangent is gone: constraint 13, end point =
    /// `end` (S99, S100).
    FloatingTo(Point2),
    /// A tangent written as a fixed tangent with these points (keeps the
    /// same line; used when its neighbour on that side disappears).
    FixedTangent(Point2, Point2),
}

/// One record of the new element list, in storage order.
enum Planned<'a> {
    Kept {
        el: &'a Element,
        before: i32,
        after: i32,
        rewrite: Rewrite,
    },
    NewFreeArc {
        id: i32,
        before: i32,
        after: i32,
        radius: f64,
    },
    NewTangent {
        id: i32,
        before: i32,
        after: i32,
        start: Point2,
        end: Point2,
    },
}

/// Bit offset of the ID field of an arc record and the bit after its
/// after-link.
fn arc_id_bits(data: &[u8], header_bit: usize) -> Option<(usize, usize)> {
    let (_, k) = raw::read_bl(data, header_bit)?;
    let k = k + 1; // flag
    let (_, k) = raw::read_bd(data, k)?;
    let (_, k) = raw::read_bd(data, k)?;
    let (n1, mut k) = raw::read_bl(data, k)?;
    for _ in 0..n1 {
        k = raw::read_bd(data, k)?.1;
    }
    let (_, k) = raw::read_bl(data, k)?;
    let ids = k + 1;
    let (_, k) = raw::read_bl(data, ids)?;
    let (_, k) = raw::read_bl(data, k)?;
    let (_, k) = raw::read_bl(data, k)?;
    Some((ids, k))
}

fn encode_kept(
    data: &[u8],
    w: &mut Bits,
    el: &Element,
    before: i32,
    after: i32,
    rewrite: Rewrite,
) -> Result<(), ElementEditError> {
    match (el, rewrite) {
        (Element::Tangent(t), Rewrite::FixedTangent(a, b)) => {
            encode_tangent(w, t.id, before, after, a, b);
        }
        (Element::Tangent(t), Rewrite::Links) => {
            w.bits(0b00, 2);
            w.rl(crate::aecc_alignment::ELEMENT_HEADER);
            w.bl(t.kind);
            w.bl(t.id);
            w.bl(before);
            w.bl(after);
            w.copy(data, t.coords_bit, t.end_bit());
        }
        (Element::Arc(a), Rewrite::Links) => {
            let (ids, rest) =
                arc_id_bits(data, a.header_bit).ok_or_else(|| err("arc record layout"))?;
            w.copy(data, a.header_bit, ids);
            w.bl(a.id);
            w.bl(before);
            w.bl(after);
            w.copy(data, rest, a.end_bit);
        }
        (Element::Arc(a), Rewrite::FloatingFrom(start)) => {
            let ArcDef::Free { radius, .. } = a.def else {
                return Err(err(format!("arc {} is not a free curve", a.id)));
            };
            let params = ArcParams {
                n2: ARC_FLOATING_START,
                ..a.params
            };
            encode_free_arc(w, a.flag, &params, (a.id, before, after), radius);
            w.rd(start.x);
            w.rd(start.y);
        }
        (Element::Arc(a), Rewrite::FloatingTo(end)) => {
            let ArcDef::Free { radius, .. } = a.def else {
                return Err(err(format!("arc {} is not a free curve", a.id)));
            };
            let params = ArcParams {
                n2: ARC_FLOATING_END,
                ..a.params
            };
            encode_free_arc(w, a.flag, &params, (a.id, before, after), radius);
            w.rd(end.x);
            w.rd(end.y);
        }
        (Element::Arc(a), Rewrite::FixedTangent(..)) => {
            return Err(err(format!("arc {} cannot become a tangent", a.id)));
        }
        (Element::Tangent(t), Rewrite::FloatingFrom(_) | Rewrite::FloatingTo(_)) => {
            return Err(err(format!("tangent {} cannot become a curve", t.id)));
        }
    }
    Ok(())
}

/// Replace the element list (count + prefixed records) of `rec` with
/// `plan` and fix the string count for the change in arcs.
fn write_elements(
    rec: &RawRecord,
    d: &DecodedElements,
    plan: &[Planned],
) -> Result<RawRecord, ElementEditError> {
    let data = &rec.data;
    let first = d.first_record_bit().ok_or_else(|| err("no elements"))?;
    let last_end = d.records_end_bit().unwrap();
    let count_bit = first - 11 - 10;
    if raw::read_bl(data, count_bit) != Some((d.record_count() as i32, first - 11)) {
        return Err(err("element count not where expected"));
    }
    let mut w = Bits::default();
    w.copy(data, 0, count_bit);
    w.bl(plan.len() as i32);
    for p in plan {
        match p {
            Planned::Kept {
                el,
                before,
                after,
                rewrite,
            } => {
                let h = el.header_bit();
                w.copy(data, h - 11, h); // the record's own kind prefix
                encode_kept(data, &mut w, el, *before, *after, *rewrite)?;
            }
            Planned::NewTangent {
                id,
                before,
                after,
                start,
                end,
            } => {
                w.bits(crate::aecc_build::PREFIX_TANGENT as u64, 11);
                encode_tangent(&mut w, *id, *before, *after, *start, *end);
            }
            Planned::NewFreeArc {
                id,
                before,
                after,
                radius,
            } => {
                w.bits(PREFIX_ARC as u64, 11);
                // As Civil 3D writes a curve it has not solved yet: flag 0,
                // p1 (design speed) −1 (S96).
                encode_free_arc(
                    &mut w,
                    false,
                    &FREE_ARC_PARAMS,
                    (*id, *before, *after),
                    *radius,
                );
            }
        }
    }
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let new_data_end = data_end + w.len() - last_end;
    w.copy(data, last_end, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;

    let arcs_old = d
        .all()
        .filter(|e| matches!(e.record, Element::Arc(_)))
        .count();
    let arcs_new = plan
        .iter()
        .filter(|p| match p {
            Planned::Kept { el, .. } => matches!(el, Element::Arc(_)),
            Planned::NewFreeArc { .. } => true,
            Planned::NewTangent { .. } => false,
        })
        .count();
    let strings = string_stream::parse(&bytes, hb)
        .map_err(|e| err(e.to_string()))?
        .strings
        .len();
    let want = (strings + 3 * arcs_new)
        .checked_sub(3 * arcs_old)
        .ok_or_else(|| err("string stream too short"))?;
    let (bytes, hb) =
        string_stream::set_string_count(&bytes, hb, want).map_err(|e| err(e.to_string()))?;
    Ok(RawRecord {
        data: bytes,
        handle_bits: hb,
        ..rec.clone()
    })
}

/// Stations of each element's start, in chain order.
fn starts(d: &DecodedElements, start: f64) -> Vec<f64> {
    let mut acc = start;
    d.elements
        .iter()
        .map(|e| {
            let s = acc;
            acc += e.segment.length();
            s
        })
        .collect()
}

/// Speeds adjusted to `[start, end]`: those before the start collapse into
/// the one in effect at the start, moved onto it (unless a speed already
/// starts there); those past the end are dropped. Stored order is kept.
fn clamp_speeds(speeds: &[(f64, f64)], start: f64, end: f64) -> Vec<(f64, f64)> {
    let eps = 1e-6 * (1.0 + start.abs().max(end.abs()));
    let at_start = speeds.iter().any(|s| (s.0 - start).abs() <= eps);
    let in_effect = speeds
        .iter()
        .enumerate()
        .filter(|(_, s)| s.0 < start - eps)
        .max_by(|a, b| a.1 .0.total_cmp(&b.1 .0))
        .map(|(i, _)| i);
    speeds
        .iter()
        .enumerate()
        .filter_map(|(i, &(st, sp))| {
            if st < start - eps {
                (!at_start && Some(i) == in_effect).then_some((start, sp))
            } else if st > end + eps {
                None
            } else {
                Some((st, sp))
            }
        })
        .collect()
}

/// Rebuild the derived parts after the element list changed: check the
/// new geometry, graphics, start station, element-ID list, design speeds.
fn finish(
    old: &RawRecord,
    new: RawRecord,
    new_start: Option<f64>,
    new_first: Option<i32>,
    seed: u64,
) -> Result<(Vec<u8>, usize, ElementEditReport), ElementEditError> {
    let decoded = decode_elements(&new).map_err(|e| err(format!("result does not decode: {e}")))?;
    // Every tangent must keep its direction (curves must fit).
    for e in &decoded.elements {
        if let (Element::Tangent(t), Segment::Line { start, end }) = (&e.record, e.segment) {
            let (dx, dy) = (t.end.x - t.start.x, t.end.y - t.start.y);
            if (end.x - start.x) * dx + (end.y - start.y) * dy < -1e-9 * (dx * dx + dy * dy) {
                return Err(err(format!("curves don't fit on tangent {}", t.id)));
            }
        }
    }
    let (bytes, hb, graphics_rebuilt) =
        replace_graphics(&new, &decoded.all().cloned().collect::<Vec<_>>())
            .map_err(|e| err(e.to_string()))?;
    let mut r = RawRecord {
        data: bytes,
        handle_bits: hb,
        ..new
    };
    if let Some(s) = new_start {
        let (bytes, hb) = crate::aecc_station::set_start_station(&r, s)
            .ok_or_else(|| err("start station not found"))?;
        r = RawRecord {
            data: bytes,
            handle_bits: hb,
            ..r
        };
    }
    if let Some(id) = new_first {
        let mut data = r.data.clone();
        crate::aecc_station::set_first_element(&r, &mut data, id)
            .ok_or_else(|| err("first-element ID fields not found"))?;
        r = RawRecord { data, ..r };
    }
    if let Some((bytes, hb)) = clear_element_id_list(&r).map_err(|e| err(e.to_string()))? {
        r = RawRecord {
            data: bytes,
            handle_bits: hb,
            ..r
        };
    }
    let start = crate::aecc_station::start_station(&r).unwrap_or(0.0);
    let length = decoded.length();
    let old_speeds: Vec<(f64, f64)> = design_speeds(old)
        .map_err(|e| err(e.to_string()))?
        .iter()
        .map(|s| (s.station, s.speed))
        .collect();
    let speeds = clamp_speeds(&old_speeds, start, start + length);
    if !old_speeds.is_empty() {
        let (bytes, hb) = set_design_speeds(&r, &speeds, seed).map_err(|e| err(e.to_string()))?;
        r = RawRecord {
            data: bytes,
            handle_bits: hb,
            ..r
        };
    }
    // Same references as before; drop padding carried through the steps.
    let keep = {
        let (d, hb) = crate::handle_refs::trim_padding(&old.data, old.handle_bits, old.handle);
        let total = raw::bit_len(&d);
        crate::handle_refs::parse(&d, total - hb, total, old.handle).len()
    };
    let (data, handle_bits) = crate::handle_refs::keep_refs(&r.data, r.handle_bits, r.handle, keep)
        .ok_or_else(|| err("handle stream lost references"))?;
    Ok((
        data,
        handle_bits,
        ElementEditReport {
            elements: decoded.elements.len(),
            length,
            start_station: start,
            added_id: None,
            floating: Vec::new(),
            speeds,
            graphics_rebuilt,
        },
    ))
}

fn stored(d: &DecodedElements) -> Vec<&ResolvedElement> {
    let mut v: Vec<&ResolvedElement> = d.all().collect();
    v.sort_by_key(|e| e.record.header_bit());
    v
}

/// Remove element `id` from the alignment `rec`.
///
/// Supported (as Civil 3D does it, S94/S95/S99/S100):
/// - the first element: the next one becomes the first; a free curve
///   there becomes a floating curve from its PC (15), a tangent keeps its
///   line and starts where it started before; stationing is kept;
/// - the last element: a free curve before it becomes a floating curve to
///   its PT (13), a tangent keeps its line;
/// - a free curve between two tangents: they then meet at their PI;
/// - a tangent between two free curves: the curves become 13 and 15 and
///   everything after the gap becomes a detached piece, which Civil 3D
///   ignores (no stations, no speeds); design speeds past the new end are
///   dropped.
///
/// Anything else (other interior tangents, a floating curve losing the
/// element it is tangent to, elements of a detached piece) is refused.
/// Detached pieces are kept as stored. `seed` seeds new design-speed IDs.
/// Returns the new record bytes and handle-stream size.
pub fn remove_element(
    rec: &RawRecord,
    id: i32,
    seed: u64,
) -> Result<(Vec<u8>, usize, ElementEditReport), ElementEditError> {
    let d = decode_elements(rec).map_err(|e| err(e.to_string()))?;
    let n = d.elements.len();
    let idx = d
        .elements
        .iter()
        .position(|e| e.record.id() == id)
        .ok_or_else(|| err(format!("no element with ID {id}")))?;
    if n == 1 {
        return Err(err("cannot remove the only element"));
    }
    if d.detached.iter().flatten().any(|e| e.record.id() == id) {
        return Err(err(format!(
            "element {id} is in a detached piece, not part of the alignment"
        )));
    }
    let start = d.stored_start_station.unwrap_or(0.0);
    let mut rewrites: Vec<(i32, i32, i32, Rewrite)> = Vec::new(); // id, before, after
    let mut floating = Vec::new();
    let mut new_start = None;
    let mut new_first = None;
    if idx == 0 {
        let next = &d.elements[1];
        if let Element::Arc(a) = &next.record {
            if matches!(
                a.def,
                ArcDef::FloatingEnd { .. }
                    | ArcDef::FloatingThrough { .. }
                    | ArcDef::FloatingLength { .. }
                    | ArcDef::FloatingDirection { .. }
            ) {
                return Err(err(format!(
                    "curve {} would lose the element it is tangent to \
                     (no sample of that form yet)",
                    a.id
                )));
            }
        }
        let rw = match &next.record {
            Element::Arc(a) if matches!(a.def, ArcDef::Free { .. }) => {
                floating.push(a.id);
                Rewrite::FloatingFrom(next.segment.start())
            }
            Element::Tangent(t) if t.kind == KIND_FLOATING || t.kind == KIND_FIXED => {
                Rewrite::FixedTangent(next.segment.start(), t.end)
            }
            _ => Rewrite::Links,
        };
        rewrites.push((next.record.id(), 0, next.record.after(), rw));
        new_start = Some(starts(&d, start)[1]);
        new_first = Some(next.record.id());
    } else if idx == n - 1 {
        let prev = &d.elements[idx - 1];
        let rw = match &prev.record {
            Element::Tangent(t) if t.kind == KIND_FIXED => {
                Rewrite::FixedTangent(t.start, prev.segment.end())
            }
            Element::Tangent(_) => Rewrite::Links,
            Element::Arc(a) => match a.def {
                ArcDef::ThreePoint { .. }
                | ArcDef::FloatingEnd { .. }
                | ArcDef::FloatingThrough { .. }
                | ArcDef::FloatingLength { .. }
                | ArcDef::FloatingDirection { .. } => Rewrite::Links,
                // S99: the free curve keeps its end point (its old PT).
                ArcDef::Free { .. } => {
                    floating.push(a.id);
                    Rewrite::FloatingTo(prev.segment.end())
                }
                ArcDef::Floating { .. }
                | ArcDef::FreeThrough { .. }
                | ArcDef::CenterRadius { .. }
                | ArcDef::CenterThrough { .. } => {
                    return Err(err(format!(
                        "curve {} would lose the element it is tangent to \
                         (no sample of that form yet)",
                        a.id
                    )))
                }
            },
        };
        rewrites.push((prev.record.id(), prev.record.before(), 0, rw));
    } else {
        let (prev, next) = (&d.elements[idx - 1], &d.elements[idx + 1]);
        let free_arc = |e: &ResolvedElement| matches!(&e.record, Element::Arc(a) if matches!(a.def, ArcDef::Free { .. }));
        // S100: a tangent between two free curves. Both curves become
        // one-sided (13 keeps its end point, 15 its start point) and the
        // alignment splits into two pieces with a gap between them.
        if matches!(d.elements[idx].record, Element::Tangent(_)) && free_arc(prev) && free_arc(next)
        {
            floating.extend([prev.record.id(), next.record.id()]);
            rewrites.push((
                prev.record.id(),
                prev.record.before(),
                0,
                Rewrite::FloatingTo(prev.segment.end()),
            ));
            rewrites.push((
                next.record.id(),
                0,
                next.record.after(),
                Rewrite::FloatingFrom(next.segment.start()),
            ));
        } else {
            let free_between_tangents = matches!(
                &d.elements[idx].record,
                Element::Arc(a) if matches!(a.def, ArcDef::Free { .. })
            ) && matches!(prev.record, Element::Tangent(_))
                && matches!(next.record, Element::Tangent(_));
            if !free_between_tangents {
                return Err(err(format!(
                    "element {id} is inside the alignment: only free curves between \
                 two tangents can be removed there"
                )));
            }
            rewrites.push((
                prev.record.id(),
                prev.record.before(),
                next.record.id(),
                Rewrite::Links,
            ));
            rewrites.push((
                next.record.id(),
                prev.record.id(),
                next.record.after(),
                Rewrite::Links,
            ));
        }
    }
    // A tangent kept as it was (its old stored start) must still meet a
    // free curve after it: only the first-element case rewrites a tangent
    // start, and only to the point it already started at.
    let plan: Vec<Planned> = stored(&d)
        .into_iter()
        .filter(|e| e.record.id() != id)
        .map(|e| {
            let el = &e.record;
            match rewrites.iter().find(|r| r.0 == el.id()) {
                Some(&(_, before, after, rewrite)) => Planned::Kept {
                    el,
                    before,
                    after,
                    rewrite,
                },
                None => Planned::Kept {
                    el,
                    before: el.before(),
                    after: el.after(),
                    rewrite: Rewrite::Links,
                },
            }
        })
        .collect();
    let new = write_elements(rec, &d, &plan)?;
    let (bytes, hb, mut report) = finish(rec, new, new_start, new_first, seed)?;
    report.floating = floating;
    Ok((bytes, hb, report))
}

/// Add a free curve of `radius` at the PI between tangent `tangent_id`
/// and the tangent after it. The curve gets the highest ID + 1 and is
/// stored last, with flag 0 and design speed −1 as Civil 3D writes a new
/// curve (S96). The curve must fit on both tangents. `seed` seeds new
/// design-speed IDs. Returns the new record bytes and handle-stream size.
pub fn add_free_curve(
    rec: &RawRecord,
    tangent_id: i32,
    radius: f64,
    seed: u64,
) -> Result<(Vec<u8>, usize, ElementEditReport), ElementEditError> {
    if !(radius.is_finite() && radius > 0.0) {
        return Err(err(format!("bad radius {radius}")));
    }
    let d = decode_elements(rec).map_err(|e| err(e.to_string()))?;

    let idx = d
        .elements
        .iter()
        .position(|e| e.record.id() == tangent_id)
        .ok_or_else(|| err(format!("no element with ID {tangent_id}")))?;
    let (a, b) = match (
        &d.elements[idx].record,
        d.elements.get(idx + 1).map(|e| &e.record),
    ) {
        (Element::Tangent(a), Some(Element::Tangent(b))) => (a, b),
        _ => {
            return Err(err(format!(
                "element {tangent_id} is not a tangent followed by a tangent"
            )))
        }
    };
    // Free curves are resolved between the tangents' stored points, which
    // must meet at the PI (as in every free curve Civil 3D wrote).
    let (ea, sb) = (a.end, b.start);
    if !(ea.x.is_finite() && sb.x.is_finite())
        || ea.distance_to(sb) > 1e-6 * (1.0 + ea.x.abs().max(ea.y.abs()))
    {
        return Err(err(format!(
            "tangents {} and {} don't meet at a stored PI",
            a.id, b.id
        )));
    }
    let new_id = d.all().map(|e| e.record.id()).max().unwrap_or(0) + 1;
    let plan: Vec<Planned> = stored(&d)
        .into_iter()
        .map(|e| {
            let el = &e.record;
            let (before, after) = if el.id() == a.id {
                (el.before(), new_id)
            } else if el.id() == b.id {
                (new_id, el.after())
            } else {
                (el.before(), el.after())
            };
            Planned::Kept {
                el,
                before,
                after,
                rewrite: Rewrite::Links,
            }
        })
        .chain(std::iter::once(Planned::NewFreeArc {
            id: new_id,
            before: a.id,
            after: b.id,
            radius,
        }))
        .collect();
    let new = write_elements(rec, &d, &plan)?;
    let (bytes, hb, mut report) = finish(rec, new, None, None, seed)?;
    report.added_id = Some(new_id);
    Ok((bytes, hb, report))
}

/// Replace every element of `rec` with `els` (fixed tangents and free
/// arcs, as [`crate::aecc_build::plan`] makes them, stored in the given
/// order), keeping the rest of the record: used by `set_geometry` for
/// templates whose element list or stationing doesn't have the plain
/// layout (floating curves, a nonzero start station, S98 `Chain`). The
/// derived parts are rebuilt as for the other edits: graphics,
/// first-element IDs, element-ID list, design speeds (clamped to the new
/// length). Detached pieces are refused.
pub fn replace_elements(
    rec: &RawRecord,
    els: &[Element],
    seed: u64,
) -> Result<(Vec<u8>, usize, ElementEditReport), ElementEditError> {
    let d = decode_elements(rec).map_err(|e| err(e.to_string()))?;
    if !d.detached.is_empty() {
        return Err(err("the alignment has detached pieces"));
    }
    let mut plan = Vec::with_capacity(els.len());
    for e in els {
        plan.push(match e {
            Element::Tangent(t) if t.kind == KIND_FIXED => Planned::NewTangent {
                id: t.id,
                before: t.before,
                after: t.after,
                start: t.start,
                end: t.end,
            },
            Element::Arc(a) => match a.def {
                ArcDef::Free { radius, .. } => Planned::NewFreeArc {
                    id: a.id,
                    before: a.before,
                    after: a.after,
                    radius,
                },
                _ => return Err(err(format!("arc {}: only free arcs can be written", a.id))),
            },
            Element::Tangent(t) => {
                return Err(err(format!(
                    "tangent {}: only fixed tangents can be written",
                    t.id
                )))
            }
        });
    }
    let head = els
        .iter()
        .find(|e| e.before() == 0)
        .map(|e| e.id())
        .ok_or_else(|| err("no first element"))?;
    let new = write_elements(rec, &d, &plan)?;
    finish(rec, new, None, Some(head), seed)
}

/// Constraint code of a curve (for reports): 29 free, 15 floating, 2
/// three-point.
pub fn constraint_name(n2: i32) -> &'static str {
    match n2 {
        ARC_FREE => "free",
        ARC_FLOATING_START => "floating (start point)",
        ARC_FLOATING_END => "floating (end point)",
        crate::aecc_elements::ARC_THREE_POINT => "fixed (three points)",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds_before_the_start_move_onto_it() {
        assert_eq!(clamp_speeds(&[(0.0, 60.0)], 220.0, 831.33), [(220.0, 60.0)]);
        assert_eq!(
            clamp_speeds(&[(0.0, 60.0), (100.0, 50.0), (300.0, 30.0)], 220.0, 831.33),
            [(220.0, 50.0), (300.0, 30.0)]
        );
        // A speed already at the start wins; past the end is dropped.
        assert_eq!(
            clamp_speeds(&[(0.0, 60.0), (220.0, 40.0), (900.0, 20.0)], 220.0, 831.33),
            [(220.0, 40.0)]
        );
    }
}
