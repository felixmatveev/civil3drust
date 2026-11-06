//! Widening transition records of offset alignments (S256–S258;
//! read-only).
//!
//! An offset alignment with a widening holds, between the elements at the
//! nominal offset and at the widened offset, one transition element per
//! transition (in and out). Record forms (docs/alignment-object.md,
//! "Widenings"):
//!
//! ```text
//! prefix 22 (linear):
//!   params (B, BD, BD, BL n, n × BD), BL 3, BL id, BL before, BL after,
//!   BL m, m × { BL 0x1000, BL k, k × RD x, y }
//! prefix 34 (curve – line – curve):
//!   BL 0, BL id, BL before, BL after, BD curve 1 radius, BD curve 2 radius
//! prefix 36 (curve – reverse curve):
//!   BL 0, BL id, BL before, BL after, BD curve 1 radius
//! prefix 38 (curve – curve – reverse curve):
//!   BL 1, BL id, BL before, BL after, BD curve 1 radius, BD curve 2
//!   radius, BD curve 3 radius, BD transition length, B
//! ```
//!
//! A linear transition is a point list per part of the parent it crosses
//! (2 points over a tangent, more over a curve); Civil 3D's stations agree
//! with straight lines through them. The curved forms are placed between
//! the end of the element before and the start of the one after (D-072):
//! curve 1 turns toward the other offset, curve 2 (and 3) bring it back.

use crate::aecc_alignment::{self};
use crate::aecc_elements::{ArcGeometry, ParamBlock, Segment};
use crate::alignment::Point2;
use crate::raw;

/// Kind prefix of a linear widening transition (S256, S257).
pub const PREFIX_WIDENING: u16 = 22;
/// Kind prefix of a curve-line-curve widening transition (S257).
pub const PREFIX_WIDENING_CURVES: u16 = 34;
/// Kind prefix of a curve-reverse curve widening transition (S258).
pub const PREFIX_WIDENING_REVERSE: u16 = 36;
/// Kind prefix of a curve-curve-reverse curve widening transition (S258).
pub const PREFIX_WIDENING_COMPOUND_REVERSE: u16 = 38;
/// The BL after the parameter block of a linear transition.
pub const WIDENING_KIND: i32 = 3;
/// The BL before each point list of a linear transition.
pub const POINT_LIST_MARK: i32 = 0x1000;

/// A widening transition record.
#[derive(Debug, Clone, PartialEq)]
pub struct WideningRecord {
    pub header_bit: usize,
    pub end_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub def: WideningDef,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WideningDef {
    /// Prefix 22: a linear transition by its points, one list per part of
    /// the parent crossed, each with the BL before it (0x1000).
    Linear {
        params: ParamBlock,
        lists: Vec<(i32, Vec<Point2>)>,
    },
    /// Prefix 34: curve 1, a line, curve 2.
    CurveLineCurve {
        lead: i32,
        radius1: f64,
        radius2: f64,
    },
    /// Prefix 36: curve 1, then a reverse curve whose radius Civil 3D
    /// derives (not stored).
    CurveReverseCurve { lead: i32, radius1: f64 },
    /// Prefix 38: curve 1, curve 2 (compound, radius derived by Civil 3D
    /// and stored), reverse curve 3.
    CurveCurveReverseCurve {
        lead: i32,
        radius1: f64,
        radius2: f64,
        radius3: f64,
        length: f64,
        flag: bool,
    },
}

fn small(v: i32) -> bool {
    (0..100_000).contains(&v)
}

fn radius_ok(r: f64) -> bool {
    r.is_finite() && r > 0.0 && r < 1e7
}

/// The 11-bit kind prefix before the record at `bit` (0 when `bit` < 11).
pub fn prefix_at(data: &[u8], bit: usize) -> u16 {
    if bit < 11 {
        return 0;
    }
    (bit - 11..bit).fold(0, |v, b| (v << 1) | u16::from(raw::get_bit(data, b)))
}

/// The widening record at `bit` whose kind prefix is `prefix`.
pub fn widening_at(data: &[u8], bit: usize, prefix: u16) -> Option<WideningRecord> {
    let (_, k) = aecc_alignment::read_element_header(data, bit)?;
    let n = raw::bit_len(data);
    let point = |k: usize| {
        (k + 128 <= n).then(|| Point2::new(raw::get_f64(data, k), raw::get_f64(data, k + 64)))
    };
    let finite =
        |p: Point2| p.x.is_finite() && p.y.is_finite() && p.x.abs() < 1e9 && p.y.abs() < 1e9;
    let ids = |k: usize| -> Option<(i32, i32, i32, i32, usize)> {
        let (lead, k) = raw::read_bl(data, k)?;
        let (id, k) = raw::read_bl(data, k)?;
        let (before, k) = raw::read_bl(data, k)?;
        let (after, k) = raw::read_bl(data, k)?;
        (id >= 1 && small(before) && small(after)).then_some((lead, id, before, after, k))
    };
    let record = |id, before, after, end_bit, def| WideningRecord {
        header_bit: bit,
        end_bit,
        id,
        before,
        after,
        def,
    };
    match prefix {
        PREFIX_WIDENING => {
            let flag = raw::get_bit(data, k) == 1;
            let (p1, k) = raw::read_bd(data, k + 1)?;
            let (p2, k) = raw::read_bd(data, k)?;
            let (nv, mut k) = raw::read_bl(data, k)?;
            if !(1..=2).contains(&nv) {
                return None;
            }
            let mut values = Vec::new();
            for _ in 0..nv {
                let (v, q) = raw::read_bd(data, k)?;
                values.push(v);
                k = q;
            }
            let (kind, id, before, after, k) = ids(k)?;
            let (m, mut k) = raw::read_bl(data, k)?;
            if kind != WIDENING_KIND || !(1..=100).contains(&m) {
                return None;
            }
            let mut lists = Vec::new();
            for _ in 0..m {
                let (mark, q) = raw::read_bl(data, k)?;
                let (count, mut q) = raw::read_bl(data, q)?;
                if mark != POINT_LIST_MARK || !(1..10_000).contains(&count) {
                    return None;
                }
                let mut pts = Vec::new();
                for _ in 0..count {
                    let p = point(q)?;
                    if !finite(p) {
                        return None;
                    }
                    pts.push(p);
                    q += 128;
                }
                lists.push((mark, pts));
                k = q;
            }
            let params = ParamBlock {
                flag,
                p1,
                p2,
                values,
            };
            Some(record(
                id,
                before,
                after,
                k,
                WideningDef::Linear { params, lists },
            ))
        }
        PREFIX_WIDENING_CURVES => {
            let (lead, id, before, after, k) = ids(k)?;
            let (radius1, k) = raw::read_bd(data, k)?;
            let (radius2, k) = raw::read_bd(data, k)?;
            (lead == 0 && radius_ok(radius1) && radius_ok(radius2)).then(|| {
                let def = WideningDef::CurveLineCurve {
                    lead,
                    radius1,
                    radius2,
                };
                record(id, before, after, k, def)
            })
        }
        PREFIX_WIDENING_REVERSE => {
            let (lead, id, before, after, k) = ids(k)?;
            let (radius1, k) = raw::read_bd(data, k)?;
            (lead == 0 && radius_ok(radius1)).then(|| {
                let def = WideningDef::CurveReverseCurve { lead, radius1 };
                record(id, before, after, k, def)
            })
        }
        PREFIX_WIDENING_COMPOUND_REVERSE => {
            let (lead, id, before, after, k) = ids(k)?;
            let (radius1, k) = raw::read_bd(data, k)?;
            let (radius2, k) = raw::read_bd(data, k)?;
            let (radius3, k) = raw::read_bd(data, k)?;
            let (length, k) = raw::read_bd(data, k)?;
            if k >= n {
                return None;
            }
            let flag = raw::get_bit(data, k) == 1;
            let ok = lead == 1
                && [radius1, radius2, radius3].iter().all(|r| radius_ok(*r))
                && length.is_finite()
                && length > 0.0;
            ok.then(|| {
                let def = WideningDef::CurveCurveReverseCurve {
                    lead,
                    radius1,
                    radius2,
                    radius3,
                    length,
                    flag,
                };
                record(id, before, after, k + 1, def)
            })
        }
        _ => None,
    }
}

/// The kind prefix of a widening record.
pub fn kind_prefix(def: &WideningDef) -> u16 {
    match def {
        WideningDef::Linear { .. } => PREFIX_WIDENING,
        WideningDef::CurveLineCurve { .. } => PREFIX_WIDENING_CURVES,
        WideningDef::CurveReverseCurve { .. } => PREFIX_WIDENING_REVERSE,
        WideningDef::CurveCurveReverseCurve { .. } => PREFIX_WIDENING_COMPOUND_REVERSE,
    }
}

/// Write the record after its header (the inverse of [`widening_at`]).
pub fn encode_widening(w: &mut crate::aecc_build::Bits, r: &WideningRecord) {
    let ids = |w: &mut crate::aecc_build::Bits, lead: i32| {
        w.bl(lead);
        w.bl(r.id);
        w.bl(r.before);
        w.bl(r.after);
    };
    match &r.def {
        WideningDef::Linear { params, lists } => {
            w.bit(params.flag as u8);
            w.bd(params.p1);
            w.bd(params.p2);
            w.bl(params.values.len() as i32);
            params.values.iter().for_each(|v| w.bd(*v));
            ids(w, WIDENING_KIND);
            w.bl(lists.len() as i32);
            for (mark, pts) in lists {
                w.bl(*mark);
                w.bl(pts.len() as i32);
                for p in pts {
                    w.rd(p.x);
                    w.rd(p.y);
                }
            }
        }
        WideningDef::CurveLineCurve {
            lead,
            radius1,
            radius2,
        } => {
            ids(w, *lead);
            w.bd(*radius1);
            w.bd(*radius2);
        }
        WideningDef::CurveReverseCurve { lead, radius1 } => {
            ids(w, *lead);
            w.bd(*radius1);
        }
        WideningDef::CurveCurveReverseCurve {
            lead,
            radius1,
            radius2,
            radius3,
            length,
            flag,
        } => {
            ids(w, *lead);
            w.bd(*radius1);
            w.bd(*radius2);
            w.bd(*radius3);
            w.bd(*length);
            w.bit(*flag as u8);
        }
    }
}

/// The segments of a linear transition: lines through its points.
pub fn linear_segments(r: &WideningRecord) -> Vec<Segment> {
    let WideningDef::Linear { lists, .. } = &r.def else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut last: Option<Point2> = None;
    for p in lists.iter().flat_map(|(_, pts)| pts) {
        if let Some(l) = last {
            if (p.x - l.x).hypot(p.y - l.y) > 1e-9 {
                out.push(Segment::Line { start: l, end: *p });
            } else {
                continue;
            }
        }
        last = Some(*p);
    }
    out
}

fn normal(d: (f64, f64)) -> (f64, f64) {
    (-d.1, d.0)
}

fn add(p: Point2, d: (f64, f64), s: f64) -> Point2 {
    Point2::new(p.x + s * d.0, p.y + s * d.1)
}

/// The arc on the circle `center`/`radius` from `a` to `b`, turning left
/// when `left`.
fn arc(center: Point2, radius: f64, left: bool, a: Point2, b: Point2) -> Segment {
    let a0 = (a.y - center.y).atan2(a.x - center.x);
    let a1 = (b.y - center.y).atan2(b.x - center.x);
    let tau = std::f64::consts::TAU;
    let mut sweep = if left { a1 - a0 } else { a0 - a1 };
    sweep = sweep.rem_euclid(tau);
    Segment::Arc {
        start: a,
        end: b,
        geometry: ArcGeometry {
            center,
            radius,
            counter_clockwise: left,
            sweep,
            length: radius * sweep,
        },
    }
}

/// The side to turn first: +1 left, −1 right — toward the other end,
/// judged from both ends.
fn first_turn(p0: Point2, d0: (f64, f64), p1: Point2, d1: (f64, f64)) -> f64 {
    let (dx, dy) = (p1.x - p0.x, p1.y - p0.y);
    let a = d0.0 * dy - d0.1 * dx;
    let b = d1.0 * dy - d1.1 * dx;
    if a + b >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// Curve 1 from `p0` (direction `d0`), a line, curve 2 into `p1`
/// (direction `d1`); curve 1 turns toward the other end, curve 2 back.
pub fn curve_line_curve(
    p0: Point2,
    d0: (f64, f64),
    p1: Point2,
    d1: (f64, f64),
    radius1: f64,
    radius2: f64,
) -> Option<Vec<Segment>> {
    let s = first_turn(p0, d0, p1, d1);
    let c1 = add(p0, normal(d0), s * radius1);
    let c2 = add(p1, normal(d1), -s * radius2);
    // A tangent point on a circle travelled with orientation o (+1 left)
    // in direction u is C − o·R·n(u). The line T1 → T2 runs along u:
    // w = C2 − C1 = L·u + k·n(u) with k = o2·R2 − o1·R1.
    let (o1, o2) = (s, -s);
    let w = (c2.x - c1.x, c2.y - c1.y);
    let k = o2 * radius2 - o1 * radius1;
    let ww = w.0 * w.0 + w.1 * w.1;
    if ww <= k * k {
        return None;
    }
    let l = (ww - k * k).sqrt();
    // u = (L·w − k·n(w)) / |w|² rotated: solve w = L u + k n(u).
    let u = ((l * w.0 + k * w.1) / ww, (l * w.1 - k * w.0) / ww);
    let nu = normal(u);
    let t1 = add(c1, nu, -o1 * radius1);
    let t2 = add(c2, nu, -o2 * radius2);
    let mut out = vec![arc(c1, radius1, s > 0.0, p0, t1)];
    if l > 1e-9 {
        out.push(Segment::Line { start: t1, end: t2 });
    }
    out.push(arc(c2, radius2, s < 0.0, t2, p1));
    Some(out)
}

/// The radius of the reverse curve of a curve – reverse curve transition:
/// the circle tangent at `p1` (direction `d1`) that touches curve 1.
pub fn reverse_radius(
    p0: Point2,
    d0: (f64, f64),
    p1: Point2,
    d1: (f64, f64),
    radius1: f64,
) -> Option<f64> {
    let s = first_turn(p0, d0, p1, d1);
    let c1 = add(p0, normal(d0), s * radius1);
    let d = (p1.x - c1.x, p1.y - c1.y);
    let n1 = normal(d1);
    // |D − s·R2·n1| = R1 + R2.
    let dd = d.0 * d.0 + d.1 * d.1;
    let dn = d.0 * n1.0 + d.1 * n1.1;
    let den = 2.0 * (radius1 + s * dn);
    let r2 = (dd - radius1 * radius1) / den;
    (den.abs() > 1e-12 && r2 > 0.0 && r2.is_finite()).then_some(r2)
}

/// Curve 1 from `p0`, then a reverse curve into `p1` with the radius that
/// makes them touch ([`reverse_radius`]).
pub fn curve_reverse_curve(
    p0: Point2,
    d0: (f64, f64),
    p1: Point2,
    d1: (f64, f64),
    radius1: f64,
) -> Option<Vec<Segment>> {
    let s = first_turn(p0, d0, p1, d1);
    let r2 = reverse_radius(p0, d0, p1, d1, radius1)?;
    let c1 = add(p0, normal(d0), s * radius1);
    let c2 = add(p1, normal(d1), -s * r2);
    let (dx, dy) = (c2.x - c1.x, c2.y - c1.y);
    let l = dx.hypot(dy);
    let t = Point2::new(c1.x + radius1 * dx / l, c1.y + radius1 * dy / l);
    Some(vec![
        arc(c1, radius1, s > 0.0, p0, t),
        arc(c2, r2, s < 0.0, t, p1),
    ])
}

/// Curve 1 from `p0`, curve 2 (same turn, the stored radius), then a
/// reverse curve 3 into `p1`. Of the two placements of curve 2, the one
/// with the shorter path.
pub fn curve_curve_reverse_curve(
    p0: Point2,
    d0: (f64, f64),
    p1: Point2,
    d1: (f64, f64),
    radius1: f64,
    radius2: f64,
    radius3: f64,
) -> Option<Vec<Segment>> {
    let s = first_turn(p0, d0, p1, d1);
    let c1 = add(p0, normal(d0), s * radius1);
    let c3 = add(p1, normal(d1), -s * radius3);
    // Curve 2's centre: |C2 − C1| = |R2 − R1| (touching curve 1 inside),
    // |C2 − C3| = R2 + R3 (touching curve 3 outside).
    let (ra, rb) = ((radius2 - radius1).abs(), radius2 + radius3);
    let (dx, dy) = (c3.x - c1.x, c3.y - c1.y);
    let d = dx.hypot(dy);
    if d < 1e-12 || d > ra + rb || d < (ra - rb).abs() {
        return None;
    }
    let a = (ra * ra - rb * rb + d * d) / (2.0 * d);
    let h = (ra * ra - a * a).max(0.0).sqrt();
    let m = Point2::new(c1.x + a * dx / d, c1.y + a * dy / d);
    let mut best: Option<(f64, Vec<Segment>)> = None;
    for sign in [1.0, -1.0] {
        let c2 = Point2::new(m.x - sign * h * dy / d, m.y + sign * h * dx / d);
        let (ux, uy) = ((c2.x - c1.x) / ra, (c2.y - c1.y) / ra);
        // Where curves 1 and 2 touch: on the line through both centres.
        let t12 = if radius2 >= radius1 {
            Point2::new(c2.x - radius2 * ux, c2.y - radius2 * uy)
        } else {
            Point2::new(c2.x + radius2 * ux, c2.y + radius2 * uy)
        };
        let t23 = Point2::new(
            c2.x + radius2 * (c3.x - c2.x) / rb,
            c2.y + radius2 * (c3.y - c2.y) / rb,
        );
        let parts = vec![
            arc(c1, radius1, s > 0.0, p0, t12),
            arc(c2, radius2, s > 0.0, t12, t23),
            arc(c3, radius3, s < 0.0, t23, p1),
        ];
        let len: f64 = parts.iter().map(Segment::length).sum();
        if best.as_ref().is_none_or(|(b, _)| len < *b) {
            best = Some((len, parts));
        }
    }
    best.map(|(_, p)| p)
}

/// The segments of a curved transition placed between the end of the
/// element before (`p0`, `d0`) and the start of the one after (`p1`,
/// `d1`).
pub fn curved_segments(
    def: &WideningDef,
    p0: Point2,
    d0: (f64, f64),
    p1: Point2,
    d1: (f64, f64),
) -> Option<Vec<Segment>> {
    match *def {
        WideningDef::Linear { .. } => None,
        WideningDef::CurveLineCurve {
            radius1, radius2, ..
        } => curve_line_curve(p0, d0, p1, d1, radius1, radius2),
        WideningDef::CurveReverseCurve { radius1, .. } => {
            curve_reverse_curve(p0, d0, p1, d1, radius1)
        }
        WideningDef::CurveCurveReverseCurve {
            radius1,
            radius2,
            radius3,
            ..
        } => curve_curve_reverse_curve(p0, d0, p1, d1, radius1, radius2, radius3),
    }
}
