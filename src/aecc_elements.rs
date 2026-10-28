//! Alignment elements of every decoded kind: tangents and arcs (read-only).
//!
//! Two kinds of **arc element record** have been seen. Both share one
//! layout up to the links:
//!
//! ```text
//! BL 0x04020008   header, same as tangents
//! B  flag         meaning unknown: 1 in S10 and S14, 0 in S11–S13
//! BD p1           60.0 (by points) / -1.0 (free)
//! BD p2           -1.0
//! BL n1           1
//! BD p3           -1.0
//! BL n2           2 (by points) / 29 (free)
//! B  0
//! BL id, BL before, BL after      as for tangents
//! ```
//!
//! then, for an arc **by three points** (sample S10; production arcs made
//! from polylines): `RD start X, Y, mid X, Y, end X, Y`, the midpoint being
//! the point halfway along the arc; and for a **free arc** (sample S11, a
//! fillet added between two tangents): `BD radius`. A free arc's position
//! comes from its neighbours: the tangents on either side keep their PI to
//! PI coordinates, and the arc is tangent to both with the stored radius.
//!
//! (B = bit, BL = bit-coded long, BD = bit-coded double, RD = raw double,
//! per the public ODA DWG specification.) p1–p3, n1, n2 are kept as read;
//! their meaning is not known. The kind is chosen from them (n2 = 29 or
//! p1 = −1 → free; n2 = 2 or p1 = 60 → by points), not from the flag bit,
//! which Civil 3D set on a free arc when re-saving S13 as S14. Unknown
//! parameter values fall back to trying both layouts.
//!
//! [`decode_elements`] chains the records and resolves them into
//! [`Segment`]s: arcs get centre, radius, direction and length; tangents
//! next to a free arc are trimmed to its PC or PT.

use crate::aecc_alignment::{self, DecodeError, Linked, TangentRecord, ELEMENT_HEADER};
use crate::alignment::Point2;
use crate::raw::{self, RawRecord};

/// Parameters stored before an arc's IDs, meaning not yet known.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcParams {
    pub p1: f64,
    pub p2: f64,
    /// Count of the BDs that follow (1 in most samples, 2 in S117's
    /// centre-and-radius circle).
    pub n1: i32,
    /// The first of the `n1` BDs.
    pub p3: f64,
    /// The second of the `n1` BDs when `n1` is 2.
    pub p4: Option<f64>,
    pub n2: i32,
    /// The bit before the IDs: 1 for the full circles of S117 (fixed curves
    /// by centre; the alignment is the whole circle).
    pub closed: bool,
}

/// How an arc record defines its geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArcDef {
    /// Start, midpoint and end stored as raw doubles from `coords_bit`.
    ThreePoint {
        coords_bit: usize,
        start: Point2,
        mid: Point2,
        end: Point2,
    },
    /// Only the radius (a BD at `radius_bit`); placed between the
    /// neighbouring tangents.
    Free { radius_bit: usize, radius: f64 },
    /// Constraint code 15 (S94): the radius (BD at `radius_bit`) and the
    /// start point (two raw doubles at `start_bit`); the curve ends tangent
    /// to the next element. Seen when the tangent before a free curve was
    /// deleted in Civil 3D.
    Floating {
        radius_bit: usize,
        radius: f64,
        start_bit: usize,
        start: Point2,
    },
    /// Constraint code 13 (S99, S100): the radius and the **end** point;
    /// the curve starts tangent to the previous element. Seen when the
    /// tangent after a free curve was deleted in Civil 3D.
    FloatingEnd {
        radius_bit: usize,
        radius: f64,
        end_bit: usize,
        end: Point2,
    },
    /// Code 0 (S117): centre and radius, and the turn direction (a bit
    /// after the data, 1 = clockwise). Fixed Curve (center point and
    /// radius), and Fixed Curve (through point, direction at point and
    /// radius), which Civil 3D stores by its centre.
    CenterRadius {
        center_bit: usize,
        center: Point2,
        radius: f64,
        clockwise: bool,
    },
    /// Code 1 (S117): centre and a point on the circle, and the turn
    /// direction. Fixed Curve (center point, through point).
    CenterThrough {
        center_bit: usize,
        center: Point2,
        through: Point2,
        clockwise: bool,
    },
    /// Code 8 (S117): starts at the previous element's end, tangent to it,
    /// and ends at `end`. Floating Curve (from entity end, through point).
    FloatingThrough { end_bit: usize, end: Point2 },
    /// Code 21 (S117): starts at the previous element's end, tangent to
    /// it, with this radius, length and turn direction. Floating Curve
    /// (from entity end, radius, length).
    FloatingLength {
        radius_bit: usize,
        radius: f64,
        length: f64,
        clockwise: bool,
    },
    /// Code 11 (S117): tangent to the previous line (whose end moves to
    /// the tangent point, extending or trimming it), ending at `end` with
    /// the unit direction `direction`. Floating Curve (from entity, through
    /// point, direction at point).
    FloatingDirection {
        end_bit: usize,
        end: Point2,
        direction: (f64, f64),
    },
    /// Code 28 (S117): a fillet between two tangents passing through
    /// `through`. Free Curve Fillet (between two entities, through point).
    FreeThrough { through_bit: usize, through: Point2 },
}

impl ArcDef {
    /// A short description of how the curve is constrained.
    pub fn describe(&self) -> &'static str {
        match self {
            Self::ThreePoint { .. } => "fixed, by 3 points",
            Self::Free { .. } => "free, by radius",
            Self::Floating { .. } => "floating, start point and radius",
            Self::FloatingEnd { .. } => "floating, radius and end point",
            Self::CenterRadius { .. } => "fixed, centre and radius",
            Self::CenterThrough { .. } => "fixed, centre and through point",
            Self::FloatingThrough { .. } => "floating, from end through point",
            Self::FloatingLength { .. } => "floating, from end by radius and length",
            Self::FloatingDirection { .. } => "floating, through point with direction",
            Self::FreeThrough { .. } => "free, through point",
        }
    }
}

/// Arc constraint codes (the BL `n2` in [`ArcParams`]), from samples.
pub const ARC_THREE_POINT: i32 = 2;
/// A free curve between two tangents (fillet).
pub const ARC_FREE: i32 = 29;
/// A curve with a fixed start point, tangent to the next element (S94).
pub const ARC_FLOATING_START: i32 = 15;
/// A curve with a fixed end point, tangent to the previous element (S99).
pub const ARC_FLOATING_END: i32 = 13;
/// Centre and radius (S117).
pub const ARC_CENTER_RADIUS: i32 = 0;
/// Centre and a point on the circle (S117).
pub const ARC_CENTER_THROUGH: i32 = 1;
/// From the previous element's end through a point (S117).
pub const ARC_FLOATING_THROUGH: i32 = 8;
/// Tangent to the previous line, ending at a point with a direction (S117).
pub const ARC_FLOATING_DIRECTION: i32 = 11;
/// From the previous element's end by radius and length (S117).
pub const ARC_FLOATING_LENGTH: i32 = 21;
/// Fillet through a point (S117).
pub const ARC_FREE_THROUGH: i32 = 28;

/// One arc element record.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcRecord {
    pub header_bit: usize,
    /// Bit offset just after the record's last field.
    pub end_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    /// The bit after the header (meaning unknown).
    pub flag: bool,
    pub params: ArcParams,
    pub def: ArcDef,
}

/// Circle geometry of an arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcGeometry {
    pub center: Point2,
    pub radius: f64,
    /// True if the arc turns left (counter-clockwise) from start to end.
    pub counter_clockwise: bool,
    /// Central angle in radians, always positive.
    pub sweep: f64,
    pub length: f64,
}

/// Circle through three points, travelling start → mid → end.
pub fn arc_through(a: Point2, m: Point2, b: Point2) -> Option<ArcGeometry> {
    let d = 2.0 * (a.x * (m.y - b.y) + m.x * (b.y - a.y) + b.x * (a.y - m.y));
    let scale = a.distance_to(b).max(a.distance_to(m)).max(1.0);
    if d.abs() <= 1e-12 * scale * scale {
        return None;
    }
    let sq = |p: Point2| p.x * p.x + p.y * p.y;
    let cx = (sq(a) * (m.y - b.y) + sq(m) * (b.y - a.y) + sq(b) * (a.y - m.y)) / d;
    let cy = (sq(a) * (b.x - m.x) + sq(m) * (a.x - b.x) + sq(b) * (m.x - a.x)) / d;
    let center = Point2::new(cx, cy);
    let radius = center.distance_to(a);
    let ccw = (m.x - a.x) * (b.y - a.y) - (m.y - a.y) * (b.x - a.x) > 0.0;
    let ang = |p: Point2| (p.y - cy).atan2(p.x - cx);
    let tau = std::f64::consts::TAU;
    let sweep = if ccw {
        (ang(b) - ang(a)).rem_euclid(tau)
    } else {
        (ang(a) - ang(b)).rem_euclid(tau)
    };
    Some(ArcGeometry {
        center,
        radius,
        counter_clockwise: ccw,
        sweep,
        length: radius * sweep,
    })
}

/// A curve of radius `radius` tangent to the line `pc_dir` arriving at
/// `pi` and the line `pt_dir` leaving it: returns (PC, PT, geometry).
/// `None` for parallel or degenerate directions.
pub fn fillet(
    pi: Point2,
    in_dir: (f64, f64),
    out_dir: (f64, f64),
    radius: f64,
) -> Option<(Point2, Point2, ArcGeometry)> {
    let norm = |v: (f64, f64)| {
        let l = v.0.hypot(v.1);
        (l > 0.0).then(|| (v.0 / l, v.1 / l))
    };
    let (u1, u2) = (norm(in_dir)?, norm(out_dir)?);
    let cross = u1.0 * u2.1 - u1.1 * u2.0;
    let dot = (u1.0 * u2.0 + u1.1 * u2.1).clamp(-1.0, 1.0);
    let delta = cross.atan2(dot).abs();
    if delta <= 1e-12 || delta >= std::f64::consts::PI - 1e-12 || radius.is_nan() || radius <= 0.0 {
        return None;
    }
    let t = radius * (delta / 2.0).tan();
    let pc = Point2::new(pi.x - u1.0 * t, pi.y - u1.1 * t);
    let pt = Point2::new(pi.x + u2.0 * t, pi.y + u2.1 * t);
    let ccw = cross > 0.0;
    // Centre is to the left of the incoming direction for a left turn.
    let (nx, ny) = if ccw { (-u1.1, u1.0) } else { (u1.1, -u1.0) };
    let center = Point2::new(pc.x + nx * radius, pc.y + ny * radius);
    Some((
        pc,
        pt,
        ArcGeometry {
            center,
            radius,
            counter_clockwise: ccw,
            sweep: delta,
            length: radius * delta,
        },
    ))
}

/// A curve of radius `radius` from `start` that ends tangent to the line
/// through `on_line` with direction `dir`, travelling along `dir`: returns
/// (start, tangent point, geometry). Of the possible curves the one with
/// the smallest central angle is taken (the record stores no direction;
/// S94 has one candidate of 90° and one of 270°).
pub fn curve_into_line(
    start: Point2,
    dir: (f64, f64),
    on_line: Point2,
    radius: f64,
) -> Option<(Point2, Point2, ArcGeometry)> {
    let l = dir.0.hypot(dir.1);
    if l.is_nan() || l <= 0.0 || radius.is_nan() || radius <= 0.0 {
        return None;
    }
    let u = (dir.0 / l, dir.1 / l);
    let n = (-u.1, u.0); // left normal
    let tau = std::f64::consts::TAU;
    let mut best: Option<(Point2, Point2, ArcGeometry)> = None;
    for side in [1.0, -1.0] {
        // Centre on the line shifted by side·R along n, at distance R from start.
        let o = Point2::new(
            on_line.x + side * radius * n.0,
            on_line.y + side * radius * n.1,
        );
        let t0 = (start.x - o.x) * u.0 + (start.y - o.y) * u.1;
        let foot = Point2::new(o.x + t0 * u.0, o.y + t0 * u.1);
        let h2 = radius * radius - foot.distance_to(start).powi(2);
        if h2 < -1e-9 * radius * radius {
            continue;
        }
        let h = h2.max(0.0).sqrt();
        for sgn in [1.0, -1.0] {
            let c = Point2::new(foot.x + sgn * h * u.0, foot.y + sgn * h * u.1);
            let tp = Point2::new(c.x - side * radius * n.0, c.y - side * radius * n.1);
            let ccw = side > 0.0; // centre left of the travel direction
            let ang = |p: Point2| (p.y - c.y).atan2(p.x - c.x);
            let sweep = if ccw {
                (ang(tp) - ang(start)).rem_euclid(tau)
            } else {
                (ang(start) - ang(tp)).rem_euclid(tau)
            };
            if sweep <= 1e-12 {
                continue;
            }
            let g = ArcGeometry {
                center: c,
                radius,
                counter_clockwise: ccw,
                sweep,
                length: radius * sweep,
            };
            if best.is_none_or(|(_, _, b)| sweep < b.sweep) {
                best = Some((start, tp, g));
            }
        }
    }
    best
}

/// A decoded element record of either kind.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Tangent(TangentRecord),
    Arc(ArcRecord),
}

impl Element {
    pub fn header_bit(&self) -> usize {
        match self {
            Self::Tangent(t) => t.header_bit,
            Self::Arc(a) => a.header_bit,
        }
    }
}

impl Linked for Element {
    fn id(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.id,
            Self::Arc(a) => a.id,
        }
    }
    fn before(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.before,
            Self::Arc(a) => a.before,
        }
    }
    fn after(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.after,
            Self::Arc(a) => a.after,
        }
    }
}

fn arc_at(data: &[u8], bit: usize) -> Option<ArcRecord> {
    if raw::get_bit(data, bit) != 0
        || raw::get_bit(data, bit + 1) != 0
        || raw::get_byte(data, bit + 2) != 0x08
    {
        return None;
    }
    let (h, k) = raw::read_bl(data, bit)?;
    if h != ELEMENT_HEADER {
        return None;
    }
    let flag = raw::get_bit(data, k) == 1;
    let (p1, k) = raw::read_bd(data, k + 1)?;
    let (p2, k) = raw::read_bd(data, k)?;
    let (n1, k) = raw::read_bl(data, k)?;
    if !(1..=2).contains(&n1) {
        return None;
    }
    let (p3, k) = raw::read_bd(data, k)?;
    let (p4, k) = if n1 == 2 {
        let (v, k) = raw::read_bd(data, k)?;
        (Some(v), k)
    } else {
        (None, k)
    };
    let (n2, k) = raw::read_bl(data, k)?;
    let closed = raw::get_bit(data, k) == 1;
    let (id, k) = raw::read_bl(data, k + 1)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let small = |v: i32| (0..100_000).contains(&v);
    if id < 1 || !small(id) || !small(before) || !small(after) || !small(n1) || !small(n2) {
        return None;
    }
    if ![p1, p2, p3, p4.unwrap_or(0.0)]
        .iter()
        .all(|v| v.is_finite())
    {
        return None;
    }
    let params = ArcParams {
        p1,
        p2,
        n1,
        p3,
        p4,
        n2,
        closed,
    };
    let point = |b: usize| -> Option<Point2> {
        if b + 128 > raw::bit_len(data) {
            return None;
        }
        let p = Point2::new(raw::get_f64(data, b), raw::get_f64(data, b + 64));
        (p.x.is_finite() && p.y.is_finite()).then_some(p)
    };
    let radius_ok = |r: f64| r.is_finite() && r > 1e-9 && r < 1e9;
    let bit_at = |b: usize| (b < raw::bit_len(data)).then(|| raw::get_bit(data, b) == 1);
    let three_point = || -> Option<(ArcDef, usize)> {
        if k + 384 > raw::bit_len(data) {
            return None;
        }
        let c: Vec<f64> = (0..6).map(|i| raw::get_f64(data, k + 64 * i)).collect();
        if c.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let (start, mid, end) = (
            Point2::new(c[0], c[1]),
            Point2::new(c[2], c[3]),
            Point2::new(c[4], c[5]),
        );
        arc_through(start, mid, end)?; // three distinct, non-collinear points
        Some((
            ArcDef::ThreePoint {
                coords_bit: k,
                start,
                mid,
                end,
            },
            k + 384,
        ))
    };
    let free = || -> Option<(ArcDef, usize)> {
        let (radius, k2) = raw::read_bd(data, k)?;
        (radius.is_finite() && radius > 1e-9 && radius < 1e9).then_some((
            ArcDef::Free {
                radius_bit: k,
                radius,
            },
            k2,
        ))
    };
    let floating = || -> Option<(ArcDef, usize)> {
        let (ArcDef::Free { radius_bit, radius }, k2) = free()? else {
            return None;
        };
        if k2 + 128 > raw::bit_len(data) {
            return None;
        }
        let start = Point2::new(raw::get_f64(data, k2), raw::get_f64(data, k2 + 64));
        (start.x.is_finite() && start.y.is_finite()).then_some((
            ArcDef::Floating {
                radius_bit,
                radius,
                start_bit: k2,
                start,
            },
            k2 + 128,
        ))
    };
    let floating_end = || -> Option<(ArcDef, usize)> {
        let (
            ArcDef::Floating {
                radius_bit,
                radius,
                start_bit,
                start,
            },
            k2,
        ) = floating()?
        else {
            return None;
        };
        Some((
            ArcDef::FloatingEnd {
                radius_bit,
                radius,
                end_bit: start_bit,
                end: start,
            },
            k2,
        ))
    };
    // The layout follows the constraint code n2. p1 is not a layout
    // marker: it is −1 or the design speed at the curve (S89, S94).
    let (def, end_bit) = match n2 {
        ARC_FREE => free()?,
        ARC_FLOATING_START => floating()?,
        ARC_FLOATING_END => floating_end()?,
        ARC_THREE_POINT => three_point()?,
        ARC_CENTER_RADIUS => {
            let center = point(k)?;
            let (radius, k2) = raw::read_bd(data, k + 128)?;
            if !radius_ok(radius) {
                return None;
            }
            let clockwise = bit_at(k2)?;
            (
                ArcDef::CenterRadius {
                    center_bit: k,
                    center,
                    radius,
                    clockwise,
                },
                k2 + 1,
            )
        }
        ARC_CENTER_THROUGH => {
            let (center, through) = (point(k)?, point(k + 128)?);
            let clockwise = bit_at(k + 256)?;
            (
                ArcDef::CenterThrough {
                    center_bit: k,
                    center,
                    through,
                    clockwise,
                },
                k + 257,
            )
        }
        ARC_FLOATING_THROUGH => (
            ArcDef::FloatingThrough {
                end_bit: k,
                end: point(k)?,
            },
            k + 128,
        ),
        ARC_FLOATING_LENGTH => {
            let (radius, k2) = raw::read_bd(data, k)?;
            let (length, k3) = raw::read_bd(data, k2)?;
            if !radius_ok(radius) || !radius_ok(length) {
                return None;
            }
            let clockwise = bit_at(k3)?;
            (
                ArcDef::FloatingLength {
                    radius_bit: k,
                    radius,
                    length,
                    clockwise,
                },
                k3 + 1,
            )
        }
        ARC_FLOATING_DIRECTION => {
            let (end, d) = (point(k)?, point(k + 128)?);
            (
                ArcDef::FloatingDirection {
                    end_bit: k,
                    end,
                    direction: (d.x, d.y),
                },
                k + 256,
            )
        }
        ARC_FREE_THROUGH => (
            ArcDef::FreeThrough {
                through_bit: k,
                through: point(k)?,
            },
            k + 128,
        ),
        _ => free().or_else(three_point)?,
    };
    Some(ArcRecord {
        header_bit: bit,
        end_bit,
        flag,
        id,
        before,
        after,
        params,
        def,
    })
}

/// Every tangent or arc element record in `data`, in storage order.
pub fn element_records(data: &[u8]) -> Vec<Element> {
    let mut out = Vec::new();
    let mut bit = 0;
    let n = raw::bit_len(data);
    while bit + 34 <= n {
        if let Some(t) = aecc_alignment::record_at(data, bit) {
            bit = t.end_bit();
            out.push(Element::Tangent(t));
        } else if let Some(a) = arc_at(data, bit) {
            bit = a.end_bit;
            out.push(Element::Arc(a));
        } else {
            bit += 1;
        }
    }
    out
}

/// The geometry an element actually occupies along the alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Line {
        start: Point2,
        end: Point2,
    },
    Arc {
        start: Point2,
        end: Point2,
        geometry: ArcGeometry,
    },
}

impl Segment {
    pub fn start(&self) -> Point2 {
        match *self {
            Self::Line { start, .. } | Self::Arc { start, .. } => start,
        }
    }
    pub fn end(&self) -> Point2 {
        match *self {
            Self::Line { end, .. } | Self::Arc { end, .. } => end,
        }
    }
    pub fn length(&self) -> f64 {
        match self {
            Self::Line { start, end } => start.distance_to(*end),
            Self::Arc { geometry, .. } => geometry.length,
        }
    }
}

/// An element record together with its resolved geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedElement {
    pub record: Element,
    pub segment: Segment,
}

impl Linked for ResolvedElement {
    fn id(&self) -> i32 {
        self.record.id()
    }
    fn before(&self) -> i32 {
        self.record.before()
    }
    fn after(&self) -> i32 {
        self.record.after()
    }
}

/// Resolve chained records into segments. Free arcs need tangents on both
/// sides that meet at a PI; those tangents are trimmed to the PC / PT.
pub fn resolve(chain: &[Element]) -> Result<Vec<ResolvedElement>, DecodeError> {
    resolve_with(chain, &vec![None; chain.len()])
}

/// (start, end, geometry) of an arc placed before [`resolve_with`].
type Placed = Option<(Point2, Point2, ArcGeometry)>;

/// [`resolve`] with some arcs already placed (`pre`, by chain index): the
/// arcs that continue from the element before them (S117).
fn resolve_with(chain: &[Element], pre: &[Placed]) -> Result<Vec<ResolvedElement>, DecodeError> {
    let unsupported = |m: String| DecodeError::BrokenChain(m);
    // Free arcs first: (PC, PT, geometry) by chain index.
    let mut free: Vec<Placed> = pre.to_vec();
    for (i, e) in chain.iter().enumerate() {
        if let Element::Arc(a) = e {
            let circle = match a.def {
                ArcDef::CenterRadius {
                    center,
                    radius,
                    clockwise,
                    ..
                } => Some((center, radius, clockwise)),
                ArcDef::CenterThrough {
                    center,
                    through,
                    clockwise,
                    ..
                } => Some((center, center.distance_to(through), clockwise)),
                _ => None,
            };
            if let Some((center, radius, clockwise)) = circle {
                if !a.params.closed || chain.len() != 1 {
                    return Err(unsupported(format!(
                        "arc {}: a curve by its centre that is not a whole circle \
                         (no sample yet)",
                        a.id
                    )));
                }
                free[i] = Some(full_circle(center, radius, !clockwise));
            }
            if let ArcDef::FreeThrough { through, .. } = a.def {
                let prev = i.checked_sub(1).and_then(|j| chain.get(j));
                let (Some(Element::Tangent(ta)), Some(Element::Tangent(tb))) =
                    (prev, chain.get(i + 1))
                else {
                    return Err(unsupported(format!(
                        "free arc {} is not between two tangents",
                        a.id
                    )));
                };
                let f = fillet_through(
                    ta.end,
                    (ta.end.x - ta.start.x, ta.end.y - ta.start.y),
                    (tb.end.x - tb.start.x, tb.end.y - tb.start.y),
                    through,
                )
                .ok_or_else(|| unsupported(format!("free arc {} has no valid fillet", a.id)))?;
                free[i] = Some(f);
            }
        }
        if let Element::Arc(ArcRecord {
            def: ArcDef::Free { radius, .. },
            id,
            ..
        }) = e
        {
            let prev = i.checked_sub(1).and_then(|j| chain.get(j));
            let next = chain.get(i + 1);
            let (Some(Element::Tangent(a)), Some(Element::Tangent(b))) = (prev, next) else {
                return Err(unsupported(format!(
                    "free arc {id} is not between two tangents"
                )));
            };
            if a.end.distance_to(b.start) > 1e-6 * (1.0 + a.end.x.abs().max(a.end.y.abs())) {
                return Err(unsupported(format!(
                    "tangents around free arc {id} don't meet at a PI"
                )));
            }
            let f = fillet(
                a.end,
                (a.end.x - a.start.x, a.end.y - a.start.y),
                (b.end.x - b.start.x, b.end.y - b.start.y),
                *radius,
            )
            .ok_or_else(|| unsupported(format!("free arc {id} has no valid fillet")))?;
            free[i] = Some(f);
        }
        if let Element::Arc(ArcRecord {
            def: ArcDef::Floating { radius, start, .. },
            id,
            ..
        }) = e
        {
            let Some(Element::Tangent(b)) = chain.get(i + 1) else {
                return Err(unsupported(format!(
                    "floating arc {id} is not followed by a tangent"
                )));
            };
            let f = curve_into_line(
                *start,
                (b.end.x - b.start.x, b.end.y - b.start.y),
                b.start,
                *radius,
            )
            .ok_or_else(|| unsupported(format!("floating arc {id} cannot reach its tangent")))?;
            free[i] = Some(f);
        }
        if let Element::Arc(ArcRecord {
            def: ArcDef::FloatingEnd { radius, end, .. },
            id,
            ..
        }) = e
        {
            let prev = i.checked_sub(1).and_then(|j| chain.get(j));
            let Some(Element::Tangent(a)) = prev else {
                return Err(unsupported(format!(
                    "floating arc {id} does not follow a tangent"
                )));
            };
            // The same curve travelled backwards: from the end point into
            // the previous tangent's line, against its direction.
            let (e_pt, tp, g) = curve_into_line(
                *end,
                (a.start.x - a.end.x, a.start.y - a.end.y),
                a.end,
                *radius,
            )
            .ok_or_else(|| unsupported(format!("floating arc {id} cannot leave its tangent")))?;
            let g = ArcGeometry {
                counter_clockwise: !g.counter_clockwise,
                ..g
            };
            free[i] = Some((tp, e_pt, g));
        }
    }
    let mut out = Vec::with_capacity(chain.len());
    for (i, e) in chain.iter().enumerate() {
        let segment = match e {
            Element::Tangent(t) => {
                let start = i
                    .checked_sub(1)
                    .and_then(|j| free[j])
                    .map_or(t.start, |(_, pt, _)| pt);
                let end = free
                    .get(i + 1)
                    .copied()
                    .flatten()
                    .map_or(t.end, |(pc, _, _)| pc);
                Segment::Line { start, end }
            }
            Element::Arc(a) => match a.def {
                ArcDef::ThreePoint {
                    start, mid, end, ..
                } => {
                    let g = arc_through(start, mid, end)
                        .ok_or_else(|| unsupported(format!("arc {} is degenerate", a.id)))?;
                    // A free line (kind 9) next to it trims it to the
                    // tangent point (S113).
                    let free = |e: Option<&Element>| match e {
                        Some(Element::Tangent(t)) if t.kind == crate::aecc_alignment::KIND_FREE => {
                            Some(t.clone())
                        }
                        _ => None,
                    };
                    let s = i
                        .checked_sub(1)
                        .and_then(|j| free(chain.get(j)))
                        .map_or(start, |t| t.end);
                    let e = free(chain.get(i + 1)).map_or(end, |t| t.start);
                    if s == start && e == end {
                        Segment::Arc {
                            start,
                            end,
                            geometry: g,
                        }
                    } else {
                        Segment::Arc {
                            start: s,
                            end: e,
                            geometry: arc_on(&g, s, e),
                        }
                    }
                }
                _ => {
                    let (pc, pt, g) = free[i].ok_or_else(|| {
                        unsupported(format!(
                            "arc {} continues from an element that doesn't resolve",
                            a.id
                        ))
                    })?;
                    Segment::Arc {
                        start: pc,
                        end: pt,
                        geometry: g,
                    }
                }
            },
        };
        out.push(ResolvedElement {
            record: e.clone(),
            segment,
        });
    }
    Ok(out)
}

/// An alignment decoded into resolved elements.
#[derive(Debug, Clone)]
pub struct DecodedElements {
    pub handle: u64,
    pub name: Option<String>,
    pub stored_start_station: Option<f64>,
    /// The alignment proper: the chain starting at the first-element ID,
    /// in path order. Stations, length and design speeds refer to it only.
    pub elements: Vec<ResolvedElement>,
    /// Disconnected pieces left by deleting an interior tangent (S100).
    /// Civil 3D draws them but ignores them otherwise: no stations, not
    /// part of the alignment until joined again (owner, S100).
    pub detached: Vec<Vec<ResolvedElement>>,
}

impl DecodedElements {
    /// Length of the alignment proper (detached pieces excluded).
    pub fn length(&self) -> f64 {
        self.elements.iter().map(|e| e.segment.length()).sum()
    }
    /// Every element record: the alignment, then the detached pieces.
    pub fn all(&self) -> impl Iterator<Item = &ResolvedElement> {
        self.elements.iter().chain(self.detached.iter().flatten())
    }
    /// Number of element records stored (the element count).
    pub fn record_count(&self) -> usize {
        self.elements.len() + self.detached.iter().map(Vec::len).sum::<usize>()
    }
    /// Bit offset of the first record's header in storage order.
    pub fn first_record_bit(&self) -> Option<usize> {
        self.all().map(|e| e.record.header_bit()).min()
    }
    /// Bit offset just after the last record in storage order.
    pub fn records_end_bit(&self) -> Option<usize> {
        self.all()
            .map(|e| match &e.record {
                Element::Tangent(t) => t.end_bit(),
                Element::Arc(a) => a.end_bit,
            })
            .max()
    }
}

/// Decode an alignment of tangents and arcs (read-only).
pub fn decode_elements(rec: &RawRecord) -> Result<DecodedElements, DecodeError> {
    let records = element_records(&rec.data);
    // The path's first element ID ends section 2 (BL right before the
    // element count, S94); it names the alignment's chain when there are
    // detached pieces (S100).
    let head = records
        .iter()
        .map(Element::header_bit)
        .min()
        .and_then(|b| b.checked_sub(31))
        .filter(|&b| raw::get_bit(&rec.data, b) == 0 && raw::get_bit(&rec.data, b + 1) == 1)
        .map(|b| raw::get_byte(&rec.data, b + 2) as i32);
    let mut pieces = aecc_alignment::chain_pieces(&records, head)?.into_iter();
    let elements = resolve_piece(pieces.next().expect("at least one piece"))?;
    let detached = pieces.map(resolve_piece).collect::<Result<Vec<_>, _>>()?;
    // The station precedes the first record in storage order, which is
    // not the first in the chain once element 1 is deleted (S94).
    let first_stored = records.iter().map(Element::header_bit).min().unwrap_or(0);
    let stored_start_station = aecc_alignment::start_station(&rec.data, first_stored);
    Ok(DecodedElements {
        handle: rec.handle,
        name: aecc_alignment::record_name(&rec.data),
        stored_start_station,
        elements,
        detached,
    })
}

/// End point and unit travel direction at the end of a segment.
fn end_state(seg: &Segment) -> (Point2, (f64, f64)) {
    match *seg {
        Segment::Line { start, end } => {
            let l = start.distance_to(end).max(f64::MIN_POSITIVE);
            (end, ((end.x - start.x) / l, (end.y - start.y) / l))
        }
        Segment::Arc {
            end, geometry: g, ..
        } => {
            let (rx, ry) = (
                (end.x - g.center.x) / g.radius,
                (end.y - g.center.y) / g.radius,
            );
            let d = if g.counter_clockwise {
                (-ry, rx)
            } else {
                (ry, -rx)
            };
            (end, d)
        }
    }
}

/// A whole circle travelled from its east point (S117: the start marker
/// of a circle alignment is at centre + (R, 0)).
fn full_circle(center: Point2, radius: f64, ccw: bool) -> (Point2, Point2, ArcGeometry) {
    let s = Point2::new(center.x + radius, center.y);
    let tau = std::f64::consts::TAU;
    (
        s,
        s,
        ArcGeometry {
            center,
            radius,
            counter_clockwise: ccw,
            sweep: tau,
            length: radius * tau,
        },
    )
}

fn unit(v: (f64, f64)) -> Option<(f64, f64)> {
    let l = v.0.hypot(v.1);
    (l.is_finite() && l > 0.0).then(|| (v.0 / l, v.1 / l))
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn circle(center: Point2, radius: f64, ccw: bool) -> ArcGeometry {
    ArcGeometry {
        center,
        radius,
        counter_clockwise: ccw,
        sweep: 0.0,
        length: 0.0,
    }
}

/// Code 8: the curve leaving `p` in direction `dir`, tangent there, that
/// passes through `end`.
pub fn curve_through(p: Point2, dir: (f64, f64), end: Point2) -> Placed {
    let u = unit(dir)?;
    let n = (-u.1, u.0);
    let d = (end.x - p.x, end.y - p.y);
    let denom = 2.0 * (d.0 * n.0 + d.1 * n.1);
    if denom.abs() <= 1e-12 * (1.0 + d.0.hypot(d.1)) {
        return None;
    }
    let s = (d.0 * d.0 + d.1 * d.1) / denom;
    let c = Point2::new(p.x + s * n.0, p.y + s * n.1);
    Some((p, end, arc_on(&circle(c, s.abs(), s > 0.0), p, end)))
}

/// Code 21: the curve leaving `p` in direction `dir` with this radius,
/// length and sense.
pub fn curve_by_length(
    p: Point2,
    dir: (f64, f64),
    radius: f64,
    length: f64,
    clockwise: bool,
) -> Placed {
    let u = unit(dir)?;
    let s = if clockwise { -radius } else { radius };
    let c = Point2::new(p.x - s * u.1, p.y + s * u.0);
    let sweep = length / radius;
    let a = (p.y - c.y).atan2(p.x - c.x) + if clockwise { -sweep } else { sweep };
    let end = Point2::new(c.x + radius * a.cos(), c.y + radius * a.sin());
    let g = ArcGeometry {
        sweep,
        length,
        ..circle(c, radius, !clockwise)
    };
    Some((p, end, g))
}

/// Code 11: the curve tangent to the line through `p0` with direction
/// `line_dir` that reaches `end` travelling in `dir`. Returns (tangent
/// point on the line, end, geometry).
pub fn curve_from_line_to(
    p0: Point2,
    line_dir: (f64, f64),
    end: Point2,
    dir: (f64, f64),
) -> Placed {
    let (u, v) = (unit(line_dir)?, unit(dir)?);
    let m = (-v.1, v.0);
    let k = 1.0 - cross(u, m);
    if k.abs() <= 1e-12 {
        return None;
    }
    // Signed radius s (> 0: centre left of travel) with the centre at
    // end + s·m and at signed distance s from the line.
    let s = cross(u, (end.x - p0.x, end.y - p0.y)) / k;
    let c = Point2::new(end.x + s * m.0, end.y + s * m.1);
    let tp = Point2::new(c.x + s * u.1, c.y - s * u.0);
    Some((tp, end, arc_on(&circle(c, s.abs(), s > 0.0), tp, end)))
}

/// Code 28: the fillet between the line arriving at `pi` along `in_dir`
/// and the line leaving along `out_dir` that passes through `x`.
pub fn fillet_through(pi: Point2, in_dir: (f64, f64), out_dir: (f64, f64), x: Point2) -> Placed {
    let (u1, u2) = (unit(in_dir)?, unit(out_dir)?);
    let b = unit((u2.0 - u1.0, u2.1 - u1.1))?;
    let delta = cross(u1, u2).atan2(u1.0 * u2.0 + u1.1 * u2.1).abs();
    let (sh, ch) = ((delta / 2.0).sin(), (delta / 2.0).cos());
    let v = (x.x - pi.x, x.y - pi.y);
    // Centre at pi + k·b, radius k·cos(δ/2), |centre − x| = radius.
    let (qa, qb, qc) = (
        sh * sh,
        -2.0 * (b.0 * v.0 + b.1 * v.1),
        v.0 * v.0 + v.1 * v.1,
    );
    let disc = qb * qb - 4.0 * qa * qc;
    if qa <= 1e-15 || disc < 0.0 {
        return None;
    }
    let mut best: Placed = None;
    for k in [
        (-qb - disc.sqrt()) / (2.0 * qa),
        (-qb + disc.sqrt()) / (2.0 * qa),
    ] {
        if k <= 0.0 {
            continue;
        }
        let Some((pc, pt, g)) = fillet(pi, u1, u2, k * ch) else {
            continue;
        };
        if arc_on(&g, pc, x).sweep <= g.sweep + 1e-9 && best.is_none() {
            best = Some((pc, pt, g));
        }
    }
    best
}

/// The line tangent to circle `c1` (travelled in its sense) leaving it
/// and to circle `c3` entering it in its sense: (point on c1, point on
/// c3). With the left normal n of the line, a counter-clockwise circle
/// has its centre at p + r·n, a clockwise one at p − r·n.
fn tangent_between(c1: &ArcGeometry, c3: &ArcGeometry) -> Option<(Point2, Point2)> {
    let s1 = if c1.counter_clockwise { 1.0 } else { -1.0 };
    let s3 = if c3.counter_clockwise { 1.0 } else { -1.0 };
    let (dx, dy) = (c3.center.x - c1.center.x, c3.center.y - c1.center.y);
    let dist = dx.hypot(dy);
    if dist <= 0.0 {
        return None;
    }
    let k = (s3 * c3.radius - s1 * c1.radius) / dist;
    if k.abs() > 1.0 {
        return None;
    }
    let theta = dy.atan2(dx);
    let mut best: Option<(Point2, Point2, f64)> = None;
    for sgn in [1.0, -1.0] {
        let a = theta + sgn * k.acos();
        let n = (a.cos(), a.sin());
        let p1 = Point2::new(
            c1.center.x - s1 * c1.radius * n.0,
            c1.center.y - s1 * c1.radius * n.1,
        );
        let p3 = Point2::new(
            c3.center.x - s3 * c3.radius * n.0,
            c3.center.y - s3 * c3.radius * n.1,
        );
        let d = (n.1, -n.0); // n is the left normal of d
        let along = (p3.x - p1.x) * d.0 + (p3.y - p1.y) * d.1;
        if along > 0.0 && best.is_none_or(|b| along < b.2) {
            best = Some((p1, p3, along));
        }
    }
    best.map(|(a, b, _)| (a, b))
}

/// An arc on the circle of `g` from `start` to `end` in `g`'s sense.
fn arc_on(g: &ArcGeometry, start: Point2, end: Point2) -> ArcGeometry {
    let tau = std::f64::consts::TAU;
    let ang = |p: Point2| (p.y - g.center.y).atan2(p.x - g.center.x);
    let sweep = if g.counter_clockwise {
        (ang(end) - ang(start)).rem_euclid(tau)
    } else {
        (ang(start) - ang(end)).rem_euclid(tau)
    };
    ArcGeometry {
        sweep,
        length: g.radius * sweep,
        ..*g
    }
}

/// Resolve one connected piece of the element chain.
fn resolve_piece(mut chained: Vec<Element>) -> Result<Vec<ResolvedElement>, DecodeError> {
    // Lines that depend on their neighbours are placed first, left to
    // right: floating lines (kind 4: end point, S89; kind 5: length, S113)
    // continue from the resolved end of the elements before them; free
    // lines (kind 9, S113) touch the circles of the curves on both sides.
    let mut pre: Vec<Placed> = vec![None; chained.len()];
    for i in 1..chained.len() {
        if let Element::Arc(a) = &chained[i] {
            let (id, def) = (a.id, a.def);
            if !matches!(
                def,
                ArcDef::FloatingThrough { .. }
                    | ArcDef::FloatingLength { .. }
                    | ArcDef::FloatingDirection { .. }
            ) {
                continue;
            }
            let unsupported = |why: &str| DecodeError::BrokenChain(format!("arc {id}: {why}"));
            let prefix = resolve_with(&chained[..i], &pre[..i])
                .map_err(|_| unsupported("elements before it don't resolve"))?;
            let last = prefix.last().expect("i >= 1");
            let (p, dir) = end_state(&last.segment);
            pre[i] = match def {
                ArcDef::FloatingThrough { end, .. } => curve_through(p, dir, end),
                ArcDef::FloatingLength {
                    radius,
                    length,
                    clockwise,
                    ..
                } => curve_by_length(p, dir, radius, length, clockwise),
                ArcDef::FloatingDirection { end, direction, .. } => match last.segment {
                    Segment::Line { start, .. } => curve_from_line_to(start, dir, end, direction),
                    _ => return Err(unsupported("not after a line (no sample yet)")),
                },
                _ => unreachable!(),
            };
            if pre[i].is_none() {
                return Err(unsupported("no curve fits"));
            }
            continue;
        }
        let Element::Tangent(t) = &chained[i] else {
            continue;
        };
        let (kind, id) = (t.kind, t.id);
        let float =
            kind == aecc_alignment::KIND_FLOATING || kind == aecc_alignment::KIND_FLOATING_LENGTH;
        if !float && kind != aecc_alignment::KIND_FREE {
            continue;
        }
        let unsupported =
            |why: &str| DecodeError::BrokenChain(format!("tangent {id} (kind {kind}): {why}"));
        let prefix = resolve_with(&chained[..i], &pre[..i])
            .map_err(|_| unsupported("elements before it don't resolve"))?;
        let last = prefix.last().expect("i >= 1");
        let (p0, dir) = end_state(&last.segment);
        if float {
            let Element::Tangent(t) = &mut chained[i] else {
                unreachable!()
            };
            t.start = p0;
            if kind == aecc_alignment::KIND_FLOATING_LENGTH {
                let l = t.length.unwrap_or(0.0);
                t.end = Point2::new(p0.x + dir.0 * l, p0.y + dir.1 * l);
            }
            continue;
        }
        // Free line: both neighbours must be arcs with known circles.
        let c1 = match last.segment {
            Segment::Arc { geometry, .. } => geometry,
            _ => return Err(unsupported("not after an arc")),
        };
        let c3 = match chained.get(i + 1) {
            Some(Element::Arc(ArcRecord {
                def: ArcDef::ThreePoint {
                    start, mid, end, ..
                },
                ..
            })) => {
                arc_through(*start, *mid, *end).ok_or_else(|| unsupported("next arc degenerate"))?
            }
            _ => return Err(unsupported("not before a three-point arc")),
        };
        let (a, b) = tangent_between(&c1, &c3).ok_or_else(|| unsupported("no common tangent"))?;
        let Element::Tangent(t) = &mut chained[i] else {
            unreachable!()
        };
        t.start = a;
        t.end = b;
    }
    let elements = resolve_with(&chained, &pre)?;
    for w in elements.windows(2) {
        let (e, s) = (w[0].segment.end(), w[1].segment.start());
        if e.distance_to(s) > 1e-6 * (1.0 + e.x.abs().max(e.y.abs())) {
            return Err(DecodeError::Discontinuous {
                after_id: w[0].id(),
            });
        }
    }
    Ok(elements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aecc_alignment::decode;

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
        fn arc_head(&mut self, by_points: bool, ids: (i32, i32, i32)) {
            self.arc_head_flag(by_points, by_points, ids);
        }
        fn arc_head_flag(&mut self, by_points: bool, flag: bool, ids: (i32, i32, i32)) {
            self.bl(ELEMENT_HEADER);
            self.bit(flag as u8);
            self.bd(if by_points { 60.0 } else { -1.0 });
            self.bd(-1.0);
            self.bl(1);
            self.bd(-1.0);
            self.bl(if by_points { 2 } else { 29 });
            self.bit(0);
            self.bl(ids.0);
            self.bl(ids.1);
            self.bl(ids.2);
        }
        /// Arc by three points, as in S10.
        fn arc(&mut self, ids: (i32, i32, i32), pts: [(f64, f64); 3]) {
            self.arc_head(true, ids);
            for p in pts {
                self.rd(p.0);
                self.rd(p.1);
            }
        }
        /// Free arc by radius, as in S11.
        fn free_arc(&mut self, ids: (i32, i32, i32), radius: f64) {
            self.arc_head(false, ids);
            self.bd(radius);
        }
    }

    fn record(data: Vec<u8>) -> RawRecord {
        RawRecord {
            handle: 7,
            handle_bits: 0,
            class: "AECC_ALIGNMENT".into(),
            is_entity: true,
            data,
        }
    }

    const H: f64 = std::f64::consts::FRAC_1_SQRT_2 * 100.0; // 70.71…

    fn kinds(d: &DecodedElements) -> Vec<&'static str> {
        d.elements
            .iter()
            .map(|e| match e.segment {
                Segment::Line { .. } => "T",
                Segment::Arc { .. } => "A",
            })
            .collect()
    }

    #[test]
    fn decodes_s10_quarter_arc() {
        let mut w = Bits(Vec::new(), 0);
        w.byte(0xA5);
        w.arc((1, 0, 0), [(0.0, 0.0), (H, 100.0 - H), (100.0, 100.0)]);
        let d = decode_elements(&record(w.0)).unwrap();
        let Element::Arc(a) = &d.elements[0].record else {
            panic!("not an arc")
        };
        assert_eq!(
            a.params,
            ArcParams {
                p1: 60.0,
                p2: -1.0,
                n1: 1,
                p3: -1.0,
                p4: None,
                n2: 2,
                closed: false
            }
        );
        let Segment::Arc { geometry: g, .. } = d.elements[0].segment else {
            panic!()
        };
        assert!(g.center.distance_to(Point2::new(0.0, 100.0)) < 1e-9);
        assert!((g.radius - 100.0).abs() < 1e-9);
        assert!(g.counter_clockwise);
        assert!((g.length - 50.0 * std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn decodes_tangent_arc_tangent_chain() {
        let mut w = Bits(Vec::new(), 0);
        w.tangent(3, 2, 0, (200.0, 100.0), (200.0, 150.0));
        w.arc(
            (2, 1, 3),
            [(100.0, 0.0), (100.0 + H, 100.0 - H), (200.0, 100.0)],
        );
        w.tangent(1, 0, 2, (0.0, 0.0), (100.0, 0.0));
        let rec = record(w.0);
        let d = decode_elements(&rec).unwrap();
        assert_eq!(kinds(&d), ["T", "A", "T"]);
        assert!((d.length() - (150.0 + 50.0 * std::f64::consts::PI)).abs() < 1e-9);
        assert_eq!(
            decode(&rec).unwrap_err(),
            DecodeError::HasCurves { arcs: 1 }
        );
    }

    /// S11: S5's tangents kept PI to PI, a free R=50 arc at PI (100,0).
    #[test]
    fn resolves_free_fillet_like_s11() {
        let mut w = Bits(Vec::new(), 0);
        w.tangent(1, 0, 4, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 4, 3, (100.0, 0.0), (200.0, 50.0));
        w.tangent(3, 2, 0, (200.0, 50.0), (300.0, 50.0));
        w.free_arc((4, 1, 2), 50.0);
        let rec = record(w.0);
        let d = decode_elements(&rec).unwrap();
        assert_eq!(kinds(&d), ["T", "A", "T", "T"]);
        let delta = 0.5f64.atan(); // direction change 26.565°
        let t = 50.0 * (delta / 2.0).tan(); // 11.8034
        let pc = Point2::new(100.0 - t, 0.0);
        let pt = Point2::new(100.0 + t * delta.cos(), t * delta.sin());
        assert!(d.elements[0].segment.end().distance_to(pc) < 1e-9);
        assert!(d.elements[2].segment.start().distance_to(pt) < 1e-9);
        let Segment::Arc { geometry: g, .. } = d.elements[1].segment else {
            panic!()
        };
        assert!(g.counter_clockwise);
        assert!(g.center.distance_to(Point2::new(100.0 - t, 50.0)) < 1e-9);
        assert!((g.length - 50.0 * delta).abs() < 1e-9);
        let total = (100.0 - t) + 50.0 * delta + (125f64.sqrt() * 10.0 - t) + 100.0;
        assert!((d.length() - total).abs() < 1e-9);
    }

    /// S14: Civil 3D set the flag bit on the free arc when re-saving S13.
    #[test]
    fn free_arc_with_flag_set_like_s14() {
        let mut w = Bits(Vec::new(), 0);
        w.tangent(1, 0, 4, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 4, 3, (100.0, 0.0), (200.0, 50.0));
        w.tangent(3, 2, 0, (200.0, 50.0), (300.0, 50.0));
        w.arc_head_flag(false, true, (4, 1, 2));
        w.bd(75.0);
        let d = decode_elements(&record(w.0)).unwrap();
        let Element::Arc(a) = &d.elements[1].record else {
            panic!()
        };
        assert!(a.flag);
        assert!(matches!(a.def, ArcDef::Free { radius, .. } if radius == 75.0));
    }

    #[test]
    fn free_arc_without_tangents_is_rejected() {
        let mut w = Bits(Vec::new(), 0);
        w.free_arc((1, 0, 0), 50.0);
        assert!(decode_elements(&record(w.0)).is_err());
    }

    #[test]
    fn clockwise_arc_and_collinear_rejection() {
        let g = arc_through(
            Point2::new(0.0, 0.0),
            Point2::new(H, H - 100.0),
            Point2::new(100.0, -100.0),
        )
        .unwrap();
        assert!(!g.counter_clockwise);
        assert!(g.center.distance_to(Point2::new(0.0, -100.0)) < 1e-9);
        assert!(arc_through(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 2.0)
        )
        .is_none());
        // Right-turn fillet: east then south-east.
        let (pc, _pt, g) = fillet(Point2::new(100.0, 0.0), (1.0, 0.0), (1.0, -1.0), 10.0).unwrap();
        assert!(!g.counter_clockwise);
        assert!(g.center.distance_to(Point2::new(pc.x, -10.0)) < 1e-9);
    }

    #[test]
    fn floating_curve_into_line_takes_the_short_way_s94() {
        // S94: start (220, 0), R 80, ending tangent to x = 300 heading −y.
        let (s, t, g) = curve_into_line(
            Point2::new(220.0, 0.0),
            (0.0, -300.0),
            Point2::new(300.0, 0.0),
            80.0,
        )
        .unwrap();
        assert_eq!(s, Point2::new(220.0, 0.0));
        assert!(t.distance_to(Point2::new(300.0, -80.0)) < 1e-9);
        assert!(g.center.distance_to(Point2::new(220.0, -80.0)) < 1e-9);
        assert!(!g.counter_clockwise);
        assert!((g.sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }
}
