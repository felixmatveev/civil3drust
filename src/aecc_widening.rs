//! Widening transition records of offset alignments (S256, S257;
//! read-only).
//!
//! An offset alignment with a widening holds, between the elements at the
//! nominal offset and at the widened offset, one transition element per
//! transition (in and out). Two record forms are known:
//!
//! ```text
//! prefix 22 (linear transition):
//!   params (B, BD, BD, BL n, n × BD), BL 3, BL id, BL before, BL after,
//!   BL form (1 | 2), BL 0x1000, BL 2, RD start x, y, RD end x, y
//!   form 2 adds: BL 0x1000, BL n, n × RD x, y
//! prefix 34 (curve-line-curve transition):
//!   BL 0, BL id, BL before, BL after, BD curve 1 radius, BD curve 2 radius
//! ```
//!
//! Form 1 is a straight line from start to end (a linear transition along
//! a straight part of the parent, S257). Form 2 (S256) crosses from a
//! straight part of the parent onto its spiral: start to end over the
//! straight part, then the listed points (the first is the end) over the
//! spiral; civil3drust joins them with lines. The curve-line-curve form
//! stores only the radii: the transition is placed between the elements
//! before and after it (docs/alignment-object.md, "Widenings").

use crate::aecc_alignment::{self};
use crate::aecc_elements::{ArcGeometry, ParamBlock, Segment};
use crate::alignment::Point2;
use crate::raw;

/// Kind prefix of a linear widening transition (S256, S257).
pub const PREFIX_WIDENING: u16 = 22;
/// Kind prefix of a curve-line-curve widening transition (S257).
pub const PREFIX_WIDENING_CURVES: u16 = 34;
/// The BL after the parameter block of a linear transition.
pub const WIDENING_KIND: i32 = 3;

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
    /// Prefix 22: a linear transition by its points.
    Linear {
        params: ParamBlock,
        /// 1: start and end only; 2: points follow.
        form: i32,
        /// The two BLs after the form (0x1000, 2 in every sample).
        a: i32,
        b: i32,
        start: Point2,
        end: Point2,
        /// Form 2: the BL before the count (0x1000) and the points.
        points: Option<(i32, Vec<Point2>)>,
    },
    /// Prefix 34: curve 1, a line, curve 2.
    CurveLineCurve {
        lead: i32,
        radius1: f64,
        radius2: f64,
    },
}

fn small(v: i32) -> bool {
    (0..100_000).contains(&v)
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
            let (kind, k) = raw::read_bl(data, k)?;
            let (id, k) = raw::read_bl(data, k)?;
            let (before, k) = raw::read_bl(data, k)?;
            let (after, k) = raw::read_bl(data, k)?;
            let (form, k) = raw::read_bl(data, k)?;
            let (a, k) = raw::read_bl(data, k)?;
            let (b, k) = raw::read_bl(data, k)?;
            if kind != WIDENING_KIND || id < 1 || !small(before) || !small(after) {
                return None;
            }
            if !(1..=2).contains(&form) {
                return None;
            }
            let (start, end) = (point(k)?, point(k + 128)?);
            if !finite(start) || !finite(end) {
                return None;
            }
            let mut k = k + 256;
            let points = if form == 2 {
                let (c, q) = raw::read_bl(data, k)?;
                let (m, mut q) = raw::read_bl(data, q)?;
                if !(1..10_000).contains(&m) {
                    return None;
                }
                let mut pts = Vec::new();
                for _ in 0..m {
                    let p = point(q)?;
                    if !finite(p) {
                        return None;
                    }
                    pts.push(p);
                    q += 128;
                }
                k = q;
                Some((c, pts))
            } else {
                None
            };
            Some(WideningRecord {
                header_bit: bit,
                end_bit: k,
                id,
                before,
                after,
                def: WideningDef::Linear {
                    params: ParamBlock {
                        flag,
                        p1,
                        p2,
                        values,
                    },
                    form,
                    a,
                    b,
                    start,
                    end,
                    points,
                },
            })
        }
        PREFIX_WIDENING_CURVES => {
            let (lead, k) = raw::read_bl(data, k)?;
            let (id, k) = raw::read_bl(data, k)?;
            let (before, k) = raw::read_bl(data, k)?;
            let (after, k) = raw::read_bl(data, k)?;
            let (radius1, k) = raw::read_bd(data, k)?;
            let (radius2, k) = raw::read_bd(data, k)?;
            let ok = |r: f64| r.is_finite() && r > 0.0 && r < 1e7;
            if lead != 0
                || id < 1
                || !small(before)
                || !small(after)
                || !ok(radius1)
                || !ok(radius2)
            {
                return None;
            }
            Some(WideningRecord {
                header_bit: bit,
                end_bit: k,
                id,
                before,
                after,
                def: WideningDef::CurveLineCurve {
                    lead,
                    radius1,
                    radius2,
                },
            })
        }
        _ => None,
    }
}

/// Write the record after its header (the inverse of [`widening_at`]).
pub fn encode_widening(w: &mut crate::aecc_build::Bits, r: &WideningRecord) {
    match &r.def {
        WideningDef::Linear {
            params,
            form,
            a,
            b,
            start,
            end,
            points,
        } => {
            w.bit(params.flag as u8);
            w.bd(params.p1);
            w.bd(params.p2);
            w.bl(params.values.len() as i32);
            params.values.iter().for_each(|v| w.bd(*v));
            w.bl(WIDENING_KIND);
            w.bl(r.id);
            w.bl(r.before);
            w.bl(r.after);
            w.bl(*form);
            w.bl(*a);
            w.bl(*b);
            for p in [start, end] {
                w.rd(p.x);
                w.rd(p.y);
            }
            if let Some((c, pts)) = points {
                w.bl(*c);
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
            w.bl(*lead);
            w.bl(r.id);
            w.bl(r.before);
            w.bl(r.after);
            w.bd(*radius1);
            w.bd(*radius2);
        }
    }
}

/// The segments of a linear transition: start to end, then lines through
/// the listed points.
pub fn linear_segments(r: &WideningRecord) -> Vec<Segment> {
    let WideningDef::Linear {
        start, end, points, ..
    } = &r.def
    else {
        return Vec::new();
    };
    let mut out = vec![Segment::Line {
        start: *start,
        end: *end,
    }];
    if let Some((_, pts)) = points {
        let mut last = *end;
        for p in pts {
            if (p.x - last.x).hypot(p.y - last.y) > 1e-9 {
                out.push(Segment::Line {
                    start: last,
                    end: *p,
                });
                last = *p;
            }
        }
    }
    out
}

fn arc_from(start: Point2, dir: (f64, f64), radius: f64, left: bool, sweep: f64) -> Segment {
    let s = if left { 1.0 } else { -1.0 };
    let center = Point2::new(start.x - s * radius * dir.1, start.y + s * radius * dir.0);
    let a0 = (start.y - center.y).atan2(start.x - center.x);
    let a1 = a0 + s * sweep;
    let end = Point2::new(center.x + radius * a1.cos(), center.y + radius * a1.sin());
    Segment::Arc {
        start,
        end,
        geometry: ArcGeometry {
            center,
            radius,
            counter_clockwise: left,
            sweep,
            length: radius * sweep,
        },
    }
}

/// A curve-line-curve transition from `p0` to `p1`, both travelling along
/// `dir` (the nominal and the widened lines are parallel): curve 1 turns
/// toward the other line, the line crosses, curve 2 turns back. `None`
/// when the radii don't fit.
pub fn curve_line_curve(
    p0: Point2,
    p1: Point2,
    dir: (f64, f64),
    radius1: f64,
    radius2: f64,
) -> Option<Vec<Segment>> {
    let (dx, dy) = (p1.x - p0.x, p1.y - p0.y);
    let along = dx * dir.0 + dy * dir.1;
    let side = dir.0 * dy - dir.1 * dx;
    let h = side.abs();
    let rs = radius1 + radius2;
    if along <= 0.0 || h < 1e-12 {
        return None;
    }
    // h = rs (1 − cos θ) + Ls sin θ with Ls = (along − rs sin θ) / cos θ;
    // increasing in θ while Ls ≥ 0.
    let f = |t: f64| rs * (1.0 - t.cos()) + (along - rs * t.sin()) * t.tan() - h;
    let hi0 = if along < rs {
        (along / rs).asin()
    } else {
        std::f64::consts::FRAC_PI_2 - 1e-12
    };
    if f(hi0) < 0.0 {
        return None;
    }
    let (mut lo, mut hi) = (0.0, hi0);
    for _ in 0..200 {
        let m = 0.5 * (lo + hi);
        if f(m) < 0.0 {
            lo = m;
        } else {
            hi = m;
        }
    }
    let t = 0.5 * (lo + hi);
    let ls = (along - rs * t.sin()) / t.cos();
    let left = side > 0.0;
    let a1 = arc_from(p0, dir, radius1, left, t);
    let s = if left { 1.0 } else { -1.0 };
    let d1 = {
        let (c, sn) = (t.cos(), s * t.sin());
        (dir.0 * c - dir.1 * sn, dir.0 * sn + dir.1 * c)
    };
    let l0 = a1.end();
    let l1 = Point2::new(l0.x + ls * d1.0, l0.y + ls * d1.1);
    let a2 = arc_from(l1, d1, radius2, !left, t);
    let mut out = vec![a1];
    if ls > 1e-9 {
        out.push(Segment::Line { start: l0, end: l1 });
    }
    out.push(a2);
    Some(out)
}
