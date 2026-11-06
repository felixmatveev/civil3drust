//! Encoding element records from their decoded fields (the inverse of
//! [`crate::aecc_elements::element_records`]).
//!
//! Every kind of element record civil3drust reads can be written back from
//! its fields alone: lines, curves of every constraint code, fixed spirals
//! (including the group members of S177/S178), free spirals and every
//! spiral group. [`encode_element`] writes the 11-bit kind prefix, the
//! element header and the record; test `encode_roundtrip` checks that this
//! reproduces Civil 3D's records bit for bit in the samples.
//!
//! The layouts are documented with each record type in `aecc_elements` and
//! in `docs/alignment-object.md`.

use crate::aecc_alignment::{
    ElementHeader, TangentRecord, KIND_FIXED, KIND_FLOATING, KIND_FLOATING_LENGTH,
    KIND_FLOATING_NEXT, KIND_FREE,
};
use crate::aecc_build::{encode_header, Bits, PREFIX_ARC, PREFIX_TANGENT};
use crate::aecc_elements::{
    ArcDef, ArcParams, ArcRecord, Element, FreeSpiralRecord, GroupDef, GroupRecord,
    LineSpiralRecord, ParamBlock, SpiralRecord, GROUP_LINE_SPIRAL, PREFIX_GROUP, PREFIX_LINE_GROUP,
    PREFIX_REVERSE_GROUP, PREFIX_SCSCS, PREFIX_SCSSCS, PREFIX_SPIRAL, SPIRAL_AFTER_CURVE,
    SPIRAL_AFTER_LINE, SPIRAL_COMPOUND, SPIRAL_INWARD, SPIRAL_OFFSET, SPIRAL_OUTWARD,
};
use crate::alignment::Point2;

/// The 11-bit kind prefix written before `el`.
pub fn kind_prefix(el: &Element) -> u16 {
    match el {
        Element::Tangent(_) => PREFIX_TANGENT,
        Element::Arc(_) => PREFIX_ARC,
        Element::LineSpiral(_) => PREFIX_LINE_GROUP,
        Element::Spiral(_) => PREFIX_SPIRAL,
        Element::FreeSpiral(_) => PREFIX_GROUP,
        Element::Group(g) => group_prefix(&g.def),
        Element::Widening(w) => match w.def {
            crate::aecc_widening::WideningDef::Linear { .. } => {
                crate::aecc_widening::PREFIX_WIDENING
            }
            crate::aecc_widening::WideningDef::CurveLineCurve { .. } => {
                crate::aecc_widening::PREFIX_WIDENING_CURVES
            }
        },
    }
}

fn group_prefix(def: &GroupDef) -> u16 {
    match def {
        GroupDef::SpiralCurveSpiral { .. }
        | GroupDef::SpiralCurveThrough { .. }
        | GroupDef::SpiralCurveLength { .. }
        | GroupDef::SpiralSpiralLines { .. } => PREFIX_GROUP,
        GroupDef::SpiralLine { .. }
        | GroupDef::SpiralSpiralCurves { .. }
        | GroupDef::SpiralLineSpiralLengths { .. }
        | GroupDef::SpiralLineSpiralLine { .. } => PREFIX_LINE_GROUP,
        GroupDef::ReverseThrough { .. } | GroupDef::ReverseTwoPoints { .. } => PREFIX_REVERSE_GROUP,
        GroupDef::Scscs { .. } => PREFIX_SCSCS,
        // Written as the JSON object only; `replace_elements` drops it.
        GroupDef::Layout { .. } => PREFIX_GROUP,
        GroupDef::Scsscs { .. } => PREFIX_SCSSCS,
    }
}

/// Write `el` with its kind prefix and `header` (flag byte and type byte
/// as read with [`crate::aecc_alignment::read_header`], or chosen for a new
/// element: `flags: Some(ELEMENT_LOCKED)` for a locked element).
pub fn encode_element(w: &mut Bits, header: &ElementHeader, el: &Element) {
    w.bits(u64::from(kind_prefix(el)), 11);
    encode_header(w, header.flags, header.type_byte);
    match el {
        Element::Tangent(t) => tangent(w, t),
        Element::Arc(a) => arc(w, a),
        Element::LineSpiral(g) => line_spiral(w, g),
        Element::Spiral(s) => spiral(w, s, header.type_byte),
        Element::FreeSpiral(s) => free_spiral(w, s),
        Element::Group(g) => group(w, g),
        Element::Widening(r) => crate::aecc_widening::encode_widening(w, r),
    }
}

fn point(w: &mut Bits, p: Point2) {
    w.rd(p.x);
    w.rd(p.y);
}

fn ids(w: &mut Bits, id: i32, before: i32, after: i32) {
    w.bl(id);
    w.bl(before);
    w.bl(after);
}

fn params(w: &mut Bits, p: &ParamBlock) {
    w.bit(p.flag as u8);
    params_after_flag(w, p);
}

fn params_after_flag(w: &mut Bits, p: &ParamBlock) {
    w.bd(p.p1);
    w.bd(p.p2);
    w.bl(p.values.len() as i32);
    p.values.iter().for_each(|v| w.bd(*v));
}

fn tangent(w: &mut Bits, t: &TangentRecord) {
    w.bl(t.kind);
    ids(w, t.id, t.before, t.after);
    match t.kind {
        KIND_FIXED => {
            point(w, t.start);
            point(w, t.end);
        }
        KIND_FLOATING => point(w, t.end),
        KIND_FLOATING_NEXT => point(w, t.start),
        KIND_FLOATING_LENGTH => w.bd(t.length.unwrap_or(0.0)),
        KIND_FREE => {}
        _ => {}
    }
}

fn arc(w: &mut Bits, a: &ArcRecord) {
    let ArcParams {
        p1,
        p2,
        n1,
        p3,
        p4,
        n2,
        closed,
    } = a.params;
    w.bit(a.flag as u8);
    w.bd(p1);
    w.bd(p2);
    w.bl(n1);
    w.bd(p3);
    if let Some(v) = p4 {
        w.bd(v);
    }
    w.bl(n2);
    w.bit(closed as u8);
    ids(w, a.id, a.before, a.after);
    match a.def {
        ArcDef::ThreePoint {
            start, mid, end, ..
        } => {
            point(w, start);
            point(w, mid);
            point(w, end);
        }
        ArcDef::Free { radius, .. } => w.bd(radius),
        ArcDef::Floating { radius, start, .. } => {
            w.bd(radius);
            point(w, start);
        }
        ArcDef::FloatingEnd { radius, end, .. } => {
            w.bd(radius);
            point(w, end);
        }
        ArcDef::CenterRadius {
            center,
            radius,
            clockwise,
            ..
        } => {
            point(w, center);
            w.bd(radius);
            w.bit(clockwise as u8);
        }
        ArcDef::CenterThrough {
            center,
            through,
            clockwise,
            ..
        } => {
            point(w, center);
            point(w, through);
            w.bit(clockwise as u8);
        }
        ArcDef::FloatingThrough { end, .. } => point(w, end),
        ArcDef::FloatingLength {
            radius,
            length,
            clockwise,
            ..
        } => {
            w.bd(radius);
            w.bd(length);
            w.bit(clockwise as u8);
        }
        ArcDef::FloatingDirection { end, direction, .. } => {
            point(w, end);
            w.rd(direction.0);
            w.rd(direction.1);
        }
        ArcDef::FreeThrough { through, .. } => point(w, through),
    }
}

fn line_spiral(w: &mut Bits, g: &LineSpiralRecord) {
    w.bit(g.flag as u8);
    w.bd(g.p1);
    w.bd(g.p2);
    w.bl(g.extra.len() as i32);
    g.extra.iter().for_each(|v| w.bd(*v));
    w.bl(GROUP_LINE_SPIRAL);
    ids(w, g.id, g.before, g.after);
    w.bl(g.mode);
    point(w, g.start);
    w.bd(g.spiral_length);
}

fn spiral(w: &mut Bits, s: &SpiralRecord, type_byte: u8) {
    if type_byte & 0x08 != 0 {
        // The group's start point (MIST17); the spiral's start if unset.
        point(w, s.member.unwrap_or(s.start));
    }
    params(w, &s.params);
    w.bl(s.kind);
    ids(w, s.id, s.before, s.after);
    w.bl(s.mode);
    w.bit(s.clockwise as u8);
    let dir = Point2::new(s.direction.0, s.direction.1);
    match s.kind {
        SPIRAL_INWARD => {
            point(w, s.start);
            point(w, dir);
            w.bd(s.length);
            w.bd(s.end_radius);
        }
        SPIRAL_COMPOUND => {
            point(w, s.start);
            point(w, dir);
            w.bd(s.start_radius);
            w.bd(s.length);
            w.bd(s.end_radius);
        }
        SPIRAL_OUTWARD => {
            w.bd(s.start_radius);
            w.bd(s.length);
            point(w, s.start);
            point(w, dir);
        }
        SPIRAL_OFFSET => {
            point(w, s.start);
            point(w, dir);
            w.bd(s.start_radius);
            w.bd(s.length);
            w.bd(s.end_radius);
            w.bd(s.offset);
            s.tail.iter().take(2).for_each(|v| w.bd(*v));
            return;
        }
        SPIRAL_AFTER_LINE | SPIRAL_AFTER_CURVE => {
            w.bd(s.end_radius);
            w.bd(s.length);
            s.tail.iter().take(2).for_each(|v| w.bd(*v));
            return;
        }
        _ => {}
    }
    s.tail.iter().for_each(|v| w.bd(*v));
}

fn free_spiral(w: &mut Bits, s: &FreeSpiralRecord) {
    params(w, &s.params);
    w.bl(s.code);
    ids(w, s.id, s.before, s.after);
    w.bl(s.mode);
    w.bd(s.length);
}

fn group(w: &mut Bits, g: &GroupRecord) {
    params(w, &g.params);
    let long = matches!(g.def, GroupDef::Scscs { .. } | GroupDef::Scsscs { .. });
    if !long {
        w.bl(g.code);
    }
    ids(w, g.id, g.before, g.after);
    w.bl(g.mode);
    let (count, second) = g.long_fields.unwrap_or((0, false));
    match g.def {
        GroupDef::Layout { .. } => {}
        GroupDef::SpiralCurveSpiral { l1, radius, l2 } => {
            [l1, radius, l2].iter().for_each(|v| w.bd(*v))
        }
        GroupDef::SpiralCurveThrough {
            spiral,
            radius,
            point: p,
        } => {
            w.bd(spiral);
            w.bd(radius);
            point(w, p);
        }
        GroupDef::SpiralCurveLength {
            spiral,
            radius,
            curve_length,
            clockwise,
        } => {
            w.bd(spiral);
            w.bd(radius);
            w.bd(curve_length);
            w.bit(clockwise as u8);
        }
        GroupDef::SpiralSpiralLines { l1, l2 } => {
            w.bd(l1);
            w.bd(l2);
        }
        GroupDef::SpiralSpiralCurves { ratio } => w.bd(ratio),
        GroupDef::SpiralLineSpiralLengths { l1, l2 } => {
            w.bd(l1);
            w.bd(l2);
        }
        GroupDef::SpiralLineSpiralLine { line } => w.bd(line),
        GroupDef::SpiralLine { spiral, line } => {
            w.bd(spiral);
            w.bd(line);
        }
        GroupDef::ReverseThrough {
            l1,
            l2,
            radius,
            point: p,
        } => {
            w.bd(l1);
            w.bd(l2);
            w.bd(radius);
            point(w, p);
        }
        GroupDef::ReverseTwoPoints { l1, l2, p1, p2 } => {
            w.bd(l1);
            w.bd(l2);
            point(w, p1);
            point(w, p2);
        }
        GroupDef::Scscs {
            spirals,
            r1,
            r2,
            first_left,
            ts,
        } => {
            w.bl(count);
            w.bit(first_left as u8);
            w.bit(second as u8);
            [spirals[0], r1, spirals[1], r2, spirals[2]]
                .iter()
                .for_each(|v| w.bd(*v));
            point(w, ts);
        }
        GroupDef::Scsscs {
            spirals,
            r1,
            r2,
            first_left,
            ts,
        } => {
            w.bl(count);
            w.bit(first_left as u8);
            w.bit(second as u8);
            [spirals[0], r1, spirals[1], spirals[2], r2, spirals[3]]
                .iter()
                .for_each(|v| w.bd(*v));
            point(w, ts);
        }
    }
}
