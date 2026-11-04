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

use crate::aecc_alignment::{self, DecodeError, Linked, TangentRecord};
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
    /// A line followed by a spiral, one record (S119).
    LineSpiral(LineSpiralRecord),
    /// A spiral with its own start point and direction (S121, S162).
    Spiral(SpiralRecord),
    /// A spiral between two lines, by length (S165).
    FreeSpiral(FreeSpiralRecord),
    /// A spiral group: spiral-curve-spiral, spiral-curve, spiral-line (S173).
    Group(GroupRecord),
}

impl Element {
    pub fn header_bit(&self) -> usize {
        match self {
            Self::Tangent(t) => t.header_bit,
            Self::Arc(a) => a.header_bit,
            Self::LineSpiral(g) => g.header_bit,
            Self::Spiral(g) => g.header_bit,
            Self::FreeSpiral(g) => g.header_bit,
            Self::Group(g) => g.header_bit,
        }
    }
    /// Bit offset just after the record.
    pub fn end_bit(&self) -> usize {
        match self {
            Self::Tangent(t) => t.end_bit(),
            Self::Arc(a) => a.end_bit,
            Self::LineSpiral(g) => g.end_bit,
            Self::Spiral(g) => g.end_bit,
            Self::FreeSpiral(g) => g.end_bit,
            Self::Group(g) => g.end_bit,
        }
    }
}

impl Linked for Element {
    fn id(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.id,
            Self::Arc(a) => a.id,
            Self::LineSpiral(g) => g.id,
            Self::Spiral(g) => g.id,
            Self::FreeSpiral(g) => g.id,
            Self::Group(g) => g.id,
        }
    }
    fn before(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.before,
            Self::Arc(a) => a.before,
            Self::LineSpiral(g) => g.before,
            Self::Spiral(g) => g.before,
            Self::FreeSpiral(g) => g.before,
            Self::Group(g) => g.before,
        }
    }
    fn after(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.after,
            Self::Arc(a) => a.after,
            Self::LineSpiral(g) => g.after,
            Self::Spiral(g) => g.after,
            Self::FreeSpiral(g) => g.after,
            Self::Group(g) => g.after,
        }
    }
}

impl Element {
    /// Set the before/after links.
    pub fn set_links(&mut self, before: i32, after: i32) {
        let (b, a) = match self {
            Self::Tangent(t) => (&mut t.before, &mut t.after),
            Self::Arc(x) => (&mut x.before, &mut x.after),
            Self::LineSpiral(x) => (&mut x.before, &mut x.after),
            Self::Spiral(x) => (&mut x.before, &mut x.after),
            Self::FreeSpiral(x) => (&mut x.before, &mut x.after),
            Self::Group(x) => (&mut x.before, &mut x.after),
        };
        *b = before;
        *a = after;
    }
}

/// Code of the line-spiral record (S119).
pub const GROUP_LINE_SPIRAL: i32 = 162;

/// A line from a fixed point followed by a clothoid into the next curve,
/// stored as one record (S119: Civil 3D's line-with-spiral layout from the
/// start of a curve). Layout:
///
/// ```text
/// BL 0x04020008, B flag, BD p1, BD p2, BL n1, n1 × BD,
/// BL code (162), BL id, BL before, BL after,
/// BL mode (256, meaning unknown), RD start X, Y, BD spiral length
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LineSpiralRecord {
    pub header_bit: usize,
    pub end_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub flag: bool,
    pub p1: f64,
    pub p2: f64,
    pub extra: Vec<f64>,
    pub mode: i32,
    /// The line's start (stored).
    pub start: Point2,
    pub spiral_length: f64,
    /// Where the line meets the spiral (TS) and the spiral meets the curve
    /// (SC); NaN until resolved.
    pub ts: Point2,
    pub sc: Point2,
}

fn line_spiral_at(data: &[u8], bit: usize) -> Option<LineSpiralRecord> {
    let (_, k) = aecc_alignment::read_element_header(data, bit)?;
    let flag = raw::get_bit(data, k) == 1;
    let (p1, k) = raw::read_bd(data, k + 1)?;
    let (p2, k) = raw::read_bd(data, k)?;
    let (n1, mut k) = raw::read_bl(data, k)?;
    if !(1..=2).contains(&n1) {
        return None;
    }
    let mut extra = Vec::new();
    for _ in 0..n1 {
        let (v, q) = raw::read_bd(data, k)?;
        extra.push(v);
        k = q;
    }
    let (code, k) = raw::read_bl(data, k)?;
    if code != GROUP_LINE_SPIRAL {
        return None;
    }
    let (id, k) = raw::read_bl(data, k)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let (mode, k) = raw::read_bl(data, k)?;
    if k + 128 > raw::bit_len(data) {
        return None;
    }
    let start = Point2::new(raw::get_f64(data, k), raw::get_f64(data, k + 64));
    let (spiral_length, end_bit) = raw::read_bd(data, k + 128)?;
    let small = |v: i32| (0..100_000).contains(&v);
    if id < 1
        || !small(before)
        || !small(after)
        || !start.x.is_finite()
        || !start.y.is_finite()
        || !(spiral_length.is_finite() && spiral_length > 0.0)
    {
        return None;
    }
    let nan = Point2::new(f64::NAN, f64::NAN);
    Some(LineSpiralRecord {
        header_bit: bit,
        end_bit,
        id,
        before,
        after,
        flag,
        p1,
        p2,
        extra,
        mode,
        start,
        spiral_length,
        ts: nan,
        sc: nan,
    })
}

/// Element header of the first spiral of a stored-apart group (S177), in
/// the locked form (see [`aecc_alignment::read_header`]).
pub const HEADER_GROUP_MEMBER: i32 = 0x0D02_0008;

/// The parameter block after an element header (curves, spirals, groups):
/// `B flag, BD p1, BD p2, BL n, n × BD`. p1 is the design speed at the
/// element (−1 until solved); the rest are −1 in every sample (MIST9).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamBlock {
    pub flag: bool,
    pub p1: f64,
    pub p2: f64,
    pub values: Vec<f64>,
}

impl ParamBlock {
    /// The block of every element Civil 3D writes fresh: −1, −1, one −1.
    pub fn unsolved() -> Self {
        ParamBlock {
            flag: false,
            p1: -1.0,
            p2: -1.0,
            values: vec![-1.0],
        }
    }
}

/// Read a parameter block whose flag bit is at `flag_bit` and whose BDs
/// start at `bd_bit` (the bit after the flag).
fn read_params(data: &[u8], flag_bit: usize, bd_bit: usize) -> Option<(ParamBlock, usize)> {
    let flag = raw::get_bit(data, flag_bit) == 1;
    let (p1, k) = raw::read_bd(data, bd_bit)?;
    let (p2, k) = raw::read_bd(data, k)?;
    let (n, mut k) = raw::read_bl(data, k)?;
    if !(1..=2).contains(&n) {
        return None;
    }
    let mut values = Vec::new();
    for _ in 0..n {
        let (v, q) = raw::read_bd(data, k)?;
        values.push(v);
        k = q;
    }
    Some((
        ParamBlock {
            flag,
            p1,
            p2,
            values,
        },
        k,
    ))
}

/// Spiral types (the code BL of a spiral record, S162).
pub const SPIRAL_INWARD: i32 = 0;
pub const SPIRAL_OUTWARD: i32 = 1;
pub const SPIRAL_COMPOUND: i32 = 2;
/// The kind prefix (11 bits before the header) of spiral records.
pub const PREFIX_SPIRAL: u16 = 6;

/// A spiral element record (kind prefix 6; S121, S162). Layout after the
/// parameter block (`B flag, BD p1, BD p2, BL n1, n1 × BD`):
///
/// ```text
/// BL type (0 inward, 1 outward, 2 compound), BL id, BL before, BL after,
/// BL 256 (MIST10), B clockwise,
/// inward:   RD start X, Y, RD unit direction, BD length, BD end radius
/// outward:  BD start radius, BD length, RD start X, Y, RD unit direction
/// compound: RD start X, Y, RD unit direction, BD start radius, BD length,
///           BD end radius
/// then BD × 3 (0, 0, 1 in every sample)
/// ```
///
/// Spirals laid out through a point (S162 D, E) store the computed length
/// and radius; the point is their end.
#[derive(Debug, Clone, PartialEq)]
pub struct SpiralRecord {
    pub header_bit: usize,
    pub end_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub flag: bool,
    pub kind: i32,
    pub mode: i32,
    pub clockwise: bool,
    pub start: Point2,
    pub direction: (f64, f64),
    pub length: f64,
    /// `f64::INFINITY` at the straight end of inward and outward spirals.
    pub start_radius: f64,
    pub end_radius: f64,
    /// The three BDs after the geometry (0, 0, 1 so far).
    pub tail: [f64; 3],
    pub params: ParamBlock,
    /// The group's start point, stored before the parameter block of the
    /// first spiral of a group kept apart with a JSON layout (header type
    /// byte 0x0D, S177; MIST17): two RDs, equal to this spiral's start.
    pub member: Option<Point2>,
}

impl SpiralRecord {
    pub fn geometry(&self) -> SpiralGeometry {
        SpiralGeometry {
            start: self.start,
            direction: self.direction,
            length: self.length,
            radius: self.end_radius,
            start_radius: self.start_radius,
            counter_clockwise: !self.clockwise,
        }
    }
}

/// The 11-bit kind prefix before an element header, if there is room.
fn kind_prefix(data: &[u8], bit: usize) -> Option<u16> {
    let start = bit.checked_sub(11)?;
    Some((0..11).fold(0u16, |v, i| {
        v << 1 | u16::from(raw::get_bit(data, start + i))
    }))
}

fn spiral_at(data: &[u8], bit: usize) -> Option<SpiralRecord> {
    if kind_prefix(data, bit)? != PREFIX_SPIRAL {
        return None;
    }
    // Header 0x0D020008 (S177): the first spiral of a group Civil 3D stored
    // as separate elements; the group's start point (RD x, RD y) comes
    // before the parameter flag (MIST17).
    let h = aecc_alignment::read_header(data, bit)?;
    let member_bits = h.is_group_member();
    let (member, flag_bit) = if member_bits {
        let x = raw::get_f64(data, h.next);
        let y = raw::get_f64(data, h.next + 64);
        (Some(Point2::new(x, y)), h.next + 128)
    } else {
        (None, h.next)
    };
    let (params, k) = read_params(data, flag_bit, flag_bit + 1)?;
    let flag = params.flag;
    let (kind, k) = raw::read_bl(data, k)?;
    let (id, k) = raw::read_bl(data, k)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let (mode, k) = raw::read_bl(data, k)?;
    let clockwise = raw::get_bit(data, k) == 1;
    let mut k = k + 1;
    let point = |b: usize| -> Option<Point2> {
        (b + 128 <= raw::bit_len(data))
            .then(|| Point2::new(raw::get_f64(data, b), raw::get_f64(data, b + 64)))
    };
    let bd = |k: &mut usize| -> Option<f64> {
        let (v, q) = raw::read_bd(data, *k)?;
        *k = q;
        Some(v)
    };
    let inf = f64::INFINITY;
    let (start, d, length, start_radius, end_radius);
    match kind {
        SPIRAL_INWARD | SPIRAL_COMPOUND => {
            start = point(k)?;
            d = point(k + 128)?;
            k += 256;
            // Compound: start radius first (S165 C: 200, 800, 250).
            start_radius = if kind == SPIRAL_COMPOUND {
                bd(&mut k)?
            } else {
                inf
            };
            length = bd(&mut k)?;
            end_radius = bd(&mut k)?;
        }
        SPIRAL_OUTWARD => {
            start_radius = bd(&mut k)?;
            length = bd(&mut k)?;
            start = point(k)?;
            d = point(k + 128)?;
            k += 256;
            end_radius = inf;
        }
        _ => return None,
    }
    let mut tail = [0.0; 3];
    for t in &mut tail {
        *t = bd(&mut k)?;
    }
    let ok = |v: f64| v.is_finite() && v > 0.0 && v < 1e9;
    let small = |v: i32| (0..100_000).contains(&v);
    if id < 1
        || !small(before)
        || !small(after)
        || !ok(length)
        || !(ok(start_radius) || start_radius == inf)
        || !(ok(end_radius) || end_radius == inf)
        || !(start.x.is_finite() && start.y.is_finite())
        || (d.x.hypot(d.y) - 1.0).abs() > 1e-6
    {
        return None;
    }
    Some(SpiralRecord {
        header_bit: bit,
        end_bit: k,
        id,
        before,
        after,
        flag,
        kind,
        mode,
        clockwise,
        start,
        direction: (d.x, d.y),
        length,
        start_radius,
        end_radius,
        tail,
        params,
        member,
    })
}

/// The kind prefix of free-spiral records (S165).
/// Free-spiral codes (S165): a spiral from the line before into the line
/// after, curvature rising (incurve) or falling (outcurve).
pub const FREE_SPIRAL_INCURVE: i32 = 0x000A_000A;
pub const FREE_SPIRAL_OUTCURVE: i32 = 0x000A_000C;

/// A free spiral between two lines (kind prefix 8; S165): only its length
/// is stored. Layout after the parameter block:
///
/// ```text
/// BL code (0xA000A incurve, 0xA000C outcurve), BL id, BL before,
/// BL after, BL 256 (MIST10), BD length
/// ```
///
/// It turns by the angle Δ between the two lines, so its radius at the
/// curved end is L / 2Δ; it is slid along the first line until it ends on
/// the second, which both lines are extended or trimmed to.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeSpiralRecord {
    pub header_bit: usize,
    pub end_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub flag: bool,
    pub code: i32,
    pub mode: i32,
    pub length: f64,
    pub params: ParamBlock,
}

fn free_spiral_at(data: &[u8], bit: usize) -> Option<FreeSpiralRecord> {
    if kind_prefix(data, bit)? != PREFIX_GROUP {
        return None;
    }
    let (_, k) = aecc_alignment::read_element_header(data, bit)?;
    let (params, k) = read_params(data, k, k + 1)?;
    let flag = params.flag;
    let (code, k) = raw::read_bl(data, k)?;
    if code != FREE_SPIRAL_INCURVE && code != FREE_SPIRAL_OUTCURVE {
        return None;
    }
    let (id, k) = raw::read_bl(data, k)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let (mode, k) = raw::read_bl(data, k)?;
    let (length, k) = raw::read_bd(data, k)?;
    if id < 1 || before < 0 || after < 0 || !(length.is_finite() && length > 0.0) {
        return None;
    }
    Some(FreeSpiralRecord {
        header_bit: bit,
        end_bit: k,
        id,
        before,
        after,
        flag,
        code,
        mode,
        length,
        params,
    })
}

/// Place a free spiral of length `l` between the line arriving along
/// `in_dir` through `p` and the line leaving along `out_dir` through `q`:
/// returns (start on the first line, end on the second, geometry).
pub fn free_spiral_between(
    p: Point2,
    in_dir: (f64, f64),
    q: Point2,
    out_dir: (f64, f64),
    l: f64,
    incurve: bool,
) -> Option<(Point2, Point2, SpiralGeometry)> {
    let (u1, u2) = (unit(in_dir)?, unit(out_dir)?);
    let c = cross(u1, u2);
    let delta = c.atan2(u1.0 * u2.0 + u1.1 * u2.1);
    if c.abs() <= 1e-12 || delta.abs() <= 1e-12 {
        return None;
    }
    let r = l / (2.0 * delta.abs());
    let inf = f64::INFINITY;
    let mut g = SpiralGeometry {
        start: Point2::new(0.0, 0.0),
        direction: u1,
        length: l,
        radius: if incurve { r } else { inf },
        start_radius: if incurve { inf } else { r },
        counter_clockwise: delta > 0.0,
    };
    let (e, _) = g.point_at(l);
    let w = (q.x - p.x - e.x, q.y - p.y - e.y);
    let t = cross(w, u2) / c;
    let ts = Point2::new(p.x + t * u1.0, p.y + t * u1.1);
    g.start = ts;
    Some((ts, Point2::new(ts.x + e.x, ts.y + e.y), g))
}

/// The kind prefix of spiral-group records with codes 0, 2, 6 (and of
/// free spirals); 10 for line-spiral groups (codes 161, 162).
pub const PREFIX_GROUP: u16 = 8;
pub const PREFIX_LINE_GROUP: u16 = 10;

/// What a group record stores (S173). Lengths and radii as typed; the
/// position comes from the neighbouring elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupDef {
    /// A group held only by the JSON layout (no record; D-061): its
    /// pieces are solved by civil3drust when read, and it is written back
    /// as the JSON object only. `constraint_type` as in the JSON.
    Layout { constraint_type: i32 },
    /// Code 0 (prefix 8): spiral, curve, spiral between two lines meeting
    /// at a PI. Free Spiral-Curve-Spiral (between two entities).
    SpiralCurveSpiral { l1: f64, radius: f64, l2: f64 },
    /// Code 2 (prefix 8): spiral and curve from the line before, the curve
    /// ending at `point`. Floating Curve with Spiral (from entity, radius,
    /// through point).
    SpiralCurveThrough {
        spiral: f64,
        radius: f64,
        point: Point2,
    },
    /// Code 6 (prefix 8): spiral and curve from the previous element's end,
    /// the curve `curve_length` long; a direction bit follows (1 =
    /// clockwise). Floating Curve with Spiral (from entity end, radius,
    /// length).
    SpiralCurveLength {
        spiral: f64,
        radius: f64,
        curve_length: f64,
        clockwise: bool,
    },
    /// Code 161 (prefix 10): spiral out of the previous curve, then a line.
    /// Floating Line with Spiral (from curve end, length).
    SpiralLine { spiral: f64, line: f64 },
    /// Code 8 (prefix 8): two spirals meeting without a curve between two
    /// lines (radius (L1 + L2) / 2Δ). Free Compound Spiral-Spiral (between
    /// two tangents).
    SpiralSpiralLines { l1: f64, l2: f64 },
    /// Code 4 (prefix 10): a spiral out of the previous curve and a spiral
    /// into the next one, with A1 / A2 = `ratio`. Free Compound / Reverse
    /// Spiral-Spiral (between two curves).
    SpiralSpiralCurves { ratio: f64 },
    /// Code 0 (prefix 10): spiral out of the previous curve, a line, a spiral
    /// into the next curve, by spiral lengths (the line's length solved).
    /// Free Compound / Reverse Spiral-Line-Spiral (between two curves,
    /// spiral lengths).
    SpiralLineSpiralLengths { l1: f64, l2: f64 },
    /// Code 1 (prefix 10): as code 0 by the line's length; the spirals have
    /// equal A. Free Compound / Reverse Spiral-Line-Spiral (between two
    /// curves, line length).
    SpiralLineSpiralLine { line: f64 },
    /// Kind 24 (no code): spiral, curve, spiral, curve, spiral between two
    /// lines, compound, from a stored TS. Free Compound
    /// Spiral-Curve-Spiral-Curve-Spiral (between two tangents).
    Scscs {
        spirals: [f64; 3],
        r1: f64,
        r2: f64,
        first_left: bool,
        ts: Point2,
    },
    /// Kind 26 (no code): spiral, curve, spiral, spiral, curve, spiral
    /// between two lines, reverse, from a stored TS. Free Reverse
    /// Spiral-Curve-Spiral-Spiral-Curve-Spiral (between two tangents).
    Scsscs {
        spirals: [f64; 4],
        r1: f64,
        r2: f64,
        first_left: bool,
        ts: Point2,
    },
    /// Code 1 (prefix 20): from a point on the previous curve, a spiral out
    /// of it, a spiral the other way into a curve of `radius` ending at
    /// `point`. Floating Reverse Curve with Spirals (from curve, radius,
    /// through point).
    ReverseThrough {
        l1: f64,
        l2: f64,
        radius: f64,
        point: Point2,
    },
    /// Code 2 (prefix 20): as code 1, the second curve through `p1` and
    /// ending at `p2` (its radius solved). Floating Reverse Curve with
    /// Spirals (from curve, two points).
    ReverseTwoPoints {
        l1: f64,
        l2: f64,
        p1: Point2,
        p2: Point2,
    },
}

/// A spiral group record (S173): one record holding several parts.
///
/// ```text
/// BL 0x04020008, B flag, BD p1, BD p2, BL n1, n1 × BD,
/// BL code, BL id, BL before, BL after, BL 256 (MIST10), data:
///   code 0:   BD L1, BD R, BD L2
///   code 2:   BD L, BD R, RD point X, Y
///   code 6:   BD L spiral, BD R, BD L curve, B clockwise
///   code 161: BD L spiral, BD L line
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRecord {
    pub header_bit: usize,
    pub end_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub flag: bool,
    pub code: i32,
    pub mode: i32,
    pub def: GroupDef,
    pub params: ParamBlock,
    /// Kinds 24 and 26: the BL (2 / 1) and the second bit before the
    /// lengths (meanings unknown).
    pub long_fields: Option<(i32, bool)>,
}

fn group_at(data: &[u8], bit: usize) -> Option<GroupRecord> {
    let prefix = kind_prefix(data, bit)?;
    if ![
        PREFIX_GROUP,
        PREFIX_LINE_GROUP,
        PREFIX_REVERSE_GROUP,
        PREFIX_SCSCS,
        PREFIX_SCSSCS,
    ]
    .contains(&prefix)
    {
        return None;
    }
    let (_, k) = aecc_alignment::read_element_header(data, bit)?;
    let (params, k) = read_params(data, k, k + 1)?;
    let flag = params.flag;
    // The long groups (kinds 24, 26) have no code field.
    let long = prefix == PREFIX_SCSCS || prefix == PREFIX_SCSSCS;
    let (code, k) = if long { (0, k) } else { raw::read_bl(data, k)? };
    let (id, k) = raw::read_bl(data, k)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let (mode, mut k) = raw::read_bl(data, k)?;
    let mut long_fields = None;
    let bd = |k: &mut usize| -> Option<f64> {
        let (v, q) = raw::read_bd(data, *k)?;
        *k = q;
        (v.is_finite() && v > 0.0 && v < 1e9).then_some(v)
    };
    let def = match (prefix, code) {
        (PREFIX_SCSCS | PREFIX_SCSSCS, _) => {
            // BL (2 / 1), B first curve left (?), B, lengths and radii, RD TS.
            let (count, q) = raw::read_bl(data, k)?;
            let first_left = raw::get_bit(data, q) == 1;
            long_fields = Some((count, raw::get_bit(data, q + 1) == 1));
            k = q + 2;
            let scscs = prefix == PREFIX_SCSCS;
            let v: Vec<f64> = (0..if scscs { 5 } else { 6 })
                .map(|_| bd(&mut k))
                .collect::<Option<_>>()?;
            if k + 128 > raw::bit_len(data) {
                return None;
            }
            let ts = Point2::new(raw::get_f64(data, k), raw::get_f64(data, k + 64));
            k += 128;
            if scscs {
                GroupDef::Scscs {
                    spirals: [v[0], v[2], v[4]],
                    r1: v[1],
                    r2: v[3],
                    first_left,
                    ts,
                }
            } else {
                GroupDef::Scsscs {
                    spirals: [v[0], v[2], v[3], v[5]],
                    r1: v[1],
                    r2: v[4],
                    first_left,
                    ts,
                }
            }
        }
        (PREFIX_GROUP, 0) => GroupDef::SpiralCurveSpiral {
            l1: bd(&mut k)?,
            radius: bd(&mut k)?,
            l2: bd(&mut k)?,
        },
        (PREFIX_GROUP, 2) => {
            let (spiral, radius) = (bd(&mut k)?, bd(&mut k)?);
            if k + 128 > raw::bit_len(data) {
                return None;
            }
            let point = Point2::new(raw::get_f64(data, k), raw::get_f64(data, k + 64));
            k += 128;
            GroupDef::SpiralCurveThrough {
                spiral,
                radius,
                point,
            }
        }
        (PREFIX_GROUP, 6) => {
            let (spiral, radius, curve_length) = (bd(&mut k)?, bd(&mut k)?, bd(&mut k)?);
            let clockwise = raw::get_bit(data, k) == 1;
            k += 1;
            GroupDef::SpiralCurveLength {
                spiral,
                radius,
                curve_length,
                clockwise,
            }
        }
        (PREFIX_GROUP, 8) => GroupDef::SpiralSpiralLines {
            l1: bd(&mut k)?,
            l2: bd(&mut k)?,
        },
        (PREFIX_LINE_GROUP, 0) => GroupDef::SpiralLineSpiralLengths {
            l1: bd(&mut k)?,
            l2: bd(&mut k)?,
        },
        (PREFIX_LINE_GROUP, 1) => GroupDef::SpiralLineSpiralLine { line: bd(&mut k)? },
        (PREFIX_LINE_GROUP, 4) => GroupDef::SpiralSpiralCurves { ratio: bd(&mut k)? },
        (PREFIX_LINE_GROUP, 161) => GroupDef::SpiralLine {
            spiral: bd(&mut k)?,
            line: bd(&mut k)?,
        },
        (PREFIX_REVERSE_GROUP, 1 | 2) => {
            let (l1, l2) = (bd(&mut k)?, bd(&mut k)?);
            let radius = if code == 1 { Some(bd(&mut k)?) } else { None };
            let n = if code == 1 { 1 } else { 2 };
            if k + 128 * n > raw::bit_len(data) {
                return None;
            }
            let pt = |b: usize| Point2::new(raw::get_f64(data, b), raw::get_f64(data, b + 64));
            let p1 = pt(k);
            let def = match radius {
                Some(radius) => GroupDef::ReverseThrough {
                    l1,
                    l2,
                    radius,
                    point: p1,
                },
                None => GroupDef::ReverseTwoPoints {
                    l1,
                    l2,
                    p1,
                    p2: pt(k + 128),
                },
            };
            k += 128 * n;
            def
        }
        _ => return None,
    };
    if id < 1 || before < 0 || after < 0 {
        return None;
    }
    Some(GroupRecord {
        header_bit: bit,
        end_bit: k,
        id,
        before,
        after,
        flag,
        code,
        mode,
        def,
        params,
        long_fields,
    })
}

/// Shift `p` and tangent offset `k` of a clothoid of length `l` entering
/// radius `r` (curve shifted out by p, centre k along the tangent from
/// the TS), with the local SC point (x, y) and the spiral angle.
fn spiral_offsets(r: f64, l: f64) -> (f64, f64, f64, f64, f64) {
    let (x, y) = clothoid(l, r * l);
    let th = l / (2.0 * r);
    (x, y, th, y - r * (1.0 - th.cos()), x - r * th.sin())
}

fn rotate(v: (f64, f64), a: f64) -> (f64, f64) {
    let (c, s) = (a.cos(), a.sin());
    (c * v.0 - s * v.1, s * v.0 + c * v.1)
}

fn inward(start: Point2, dir: (f64, f64), l: f64, r: f64, ccw: bool) -> SpiralGeometry {
    SpiralGeometry {
        start,
        direction: dir,
        length: l,
        radius: r,
        start_radius: f64::INFINITY,
        counter_clockwise: ccw,
    }
}

fn spiral_segment(g: SpiralGeometry) -> Segment {
    Segment::Spiral {
        start: g.start,
        end: g.point_at(g.length).0,
        geometry: g,
    }
}

/// The parts of a group record placed between its neighbours (S173):
/// `before` is the resolved element before it (end state), `prev_line`
/// and `next_line` the stored lines around it.
fn place_group(
    def: GroupDef,
    before: Option<&Segment>,
    prev_line: Option<&TangentRecord>,
    next_line: Option<&TangentRecord>,
    next_curve: Option<&ArcGeometry>,
) -> Option<Vec<Segment>> {
    match def {
        // Never stored as a record (solved in `solve_layout_group`).
        GroupDef::Layout { .. } => None,
        GroupDef::SpiralSpiralLines { l1, l2 } => {
            let (a, b) = (prev_line?, next_line?);
            let u1 = unit((a.end.x - a.start.x, a.end.y - a.start.y))?;
            let u2 = unit((b.end.x - b.start.x, b.end.y - b.start.y))?;
            let delta = cross(u1, u2).atan2(u1.0 * u2.0 + u1.1 * u2.1).abs();
            if delta <= 1e-9 {
                return None;
            }
            let radius = (l1 + l2) / (2.0 * delta);
            let mut parts = place_group(
                GroupDef::SpiralCurveSpiral { l1, radius, l2 },
                before,
                prev_line,
                next_line,
                next_curve,
            )?;
            parts.remove(1); // the curve between them has no length
            Some(parts)
        }
        GroupDef::Scscs {
            spirals,
            r1,
            r2,
            first_left,
            ts,
        } => place_long_group(
            ts,
            prev_line?,
            next_line?,
            &spirals,
            (r1, r2),
            first_left,
            false,
        ),
        GroupDef::Scsscs {
            spirals,
            r1,
            r2,
            first_left,
            ts,
        } => place_long_group(
            ts,
            prev_line?,
            next_line?,
            &spirals,
            (r1, r2),
            first_left,
            true,
        ),
        GroupDef::SpiralLineSpiralLengths { l1, l2 } => {
            let Some(&Segment::Arc { geometry, .. }) = before else {
                return None;
            };
            place_spiral_line_spiral(&geometry, next_curve?, Some((l1, l2)), None)
        }
        GroupDef::SpiralLineSpiralLine { line } => {
            let Some(&Segment::Arc { geometry, .. }) = before else {
                return None;
            };
            place_spiral_line_spiral(&geometry, next_curve?, None, Some(line))
        }
        GroupDef::SpiralSpiralCurves { ratio } => {
            let Some(&Segment::Arc {
                start, geometry, ..
            }) = before
            else {
                return None;
            };
            place_spiral_spiral(&geometry, start, next_curve?, ratio)
        }
        GroupDef::SpiralCurveSpiral { l1, radius, l2 } => {
            let (a, b) = (prev_line?, next_line?);
            let pi = a.end;
            let u1 = unit((a.end.x - a.start.x, a.end.y - a.start.y))?;
            let u2 = unit((b.end.x - b.start.x, b.end.y - b.start.y))?;
            let d = cross(u1, u2).atan2(u1.0 * u2.0 + u1.1 * u2.1);
            let ccw = d > 0.0;
            let delta = d.abs();
            if delta <= 1e-9 {
                return None;
            }
            let (_, _, _, p1, k1) = spiral_offsets(radius, l1);
            let (x2, y2, th2, p2, k2) = spiral_offsets(radius, l2);
            let t1 = (radius + p1) * (delta / 2.0).tan() + k1 - (p1 - p2) / delta.sin();
            let t2 = (radius + p2) * (delta / 2.0).tan() + k2 + (p1 - p2) / delta.sin();
            let ts = Point2::new(pi.x - t1 * u1.0, pi.y - t1 * u1.1);
            let st = Point2::new(pi.x + t2 * u2.0, pi.y + t2 * u2.1);
            let side = if ccw { 1.0 } else { -1.0 };
            let s1 = inward(ts, u1, l1, radius, ccw);
            let (sc, _) = s1.point_at(l1);
            let n2 = (-u2.1 * side, u2.0 * side);
            let cs = Point2::new(st.x - x2 * u2.0 + y2 * n2.0, st.y - x2 * u2.1 + y2 * n2.1);
            let n1 = (-u1.1 * side, u1.0 * side);
            let c = Point2::new(
                ts.x + k1 * u1.0 + (radius + p1) * n1.0,
                ts.y + k1 * u1.1 + (radius + p1) * n1.1,
            );
            let arc = arc_on(&circle(c, radius, ccw), sc, cs);
            let s2 = SpiralGeometry {
                start: cs,
                direction: rotate(u2, -side * th2),
                length: l2,
                radius: f64::INFINITY,
                start_radius: radius,
                counter_clockwise: ccw,
            };
            Some(vec![
                spiral_segment(s1),
                Segment::Arc {
                    start: sc,
                    end: cs,
                    geometry: arc,
                },
                Segment::Spiral {
                    start: cs,
                    end: st,
                    geometry: s2,
                },
            ])
        }
        GroupDef::SpiralCurveThrough {
            spiral,
            radius,
            point,
        } => {
            let a = prev_line?;
            let u = unit((a.end.x - a.start.x, a.end.y - a.start.y))?;
            let mut best: Option<(f64, Vec<Segment>)> = None;
            for ccw in [true, false] {
                let side = if ccw { 1.0 } else { -1.0 };
                let n = (-u.1 * side, u.0 * side);
                let (_, _, _, p, k) = spiral_offsets(radius, spiral);
                let off = (
                    a.start.x + k * u.0 + (radius + p) * n.0 - point.x,
                    a.start.y + k * u.1 + (radius + p) * n.1 - point.y,
                );
                let b = off.0 * u.0 + off.1 * u.1;
                let disc = b * b - (off.0 * off.0 + off.1 * off.1 - radius * radius);
                if disc < 0.0 {
                    continue;
                }
                for t in [-b - disc.sqrt(), -b + disc.sqrt()] {
                    let ts = Point2::new(a.start.x + t * u.0, a.start.y + t * u.1);
                    let s = inward(ts, u, spiral, radius, ccw);
                    let (sc, _) = s.point_at(spiral);
                    let o = t_c(k, u, p + radius, n);
                    let c = Point2::new(ts.x + o.0, ts.y + o.1);
                    let arc = arc_on(&circle(c, radius, ccw), sc, point);
                    if best.as_ref().is_none_or(|(w, _)| arc.sweep < *w) {
                        best = Some((
                            arc.sweep,
                            vec![
                                spiral_segment(s),
                                Segment::Arc {
                                    start: sc,
                                    end: point,
                                    geometry: arc,
                                },
                            ],
                        ));
                    }
                }
            }
            best.map(|(_, v)| v)
        }
        GroupDef::SpiralCurveLength {
            spiral,
            radius,
            curve_length,
            clockwise,
        } => {
            let (p, dir) = end_state(before?);
            let s = inward(p, dir, spiral, radius, !clockwise);
            let (sc, d) = s.point_at(spiral);
            let (_, end, arc) = curve_by_length(sc, d, radius, curve_length, clockwise)?;
            Some(vec![
                spiral_segment(s),
                Segment::Arc {
                    start: sc,
                    end,
                    geometry: arc,
                },
            ])
        }
        GroupDef::ReverseThrough {
            l1,
            l2,
            radius,
            point,
        } => {
            let Some(&Segment::Arc {
                start, geometry, ..
            }) = before
            else {
                return None;
            };
            place_reverse(&geometry, start, l1, l2, Some(radius), &[point])
        }
        GroupDef::ReverseTwoPoints { l1, l2, p1, p2 } => {
            let Some(&Segment::Arc {
                start, geometry, ..
            }) = before
            else {
                return None;
            };
            place_reverse(&geometry, start, l1, l2, None, &[p1, p2])
        }
        GroupDef::SpiralLine { spiral, line } => {
            let prev = before?;
            let Segment::Arc { geometry: g, .. } = prev else {
                return None;
            };
            let (p, dir) = end_state(prev);
            let s = SpiralGeometry {
                start: p,
                direction: dir,
                length: spiral,
                radius: f64::INFINITY,
                start_radius: g.radius,
                counter_clockwise: g.counter_clockwise,
            };
            let (st, d) = s.point_at(spiral);
            Some(vec![
                spiral_segment(s),
                Segment::Line {
                    start: st,
                    end: Point2::new(st.x + line * d.0, st.y + line * d.1),
                },
            ])
        }
    }
}

/// The kind prefix of reverse-curve-with-spirals groups (S174).
pub const PREFIX_REVERSE_GROUP: u16 = 20;

/// Spiral out of `c1` at angle `phi`, a spiral the other way into radius
/// `r2`: (start on c1, the two spirals, SC, centre of the second curve).
fn reverse_chain(
    c1: &ArcGeometry,
    phi: f64,
    l1: f64,
    l2: f64,
    r2: f64,
) -> (Point2, SpiralGeometry, SpiralGeometry, Point2, Point2) {
    let s = Point2::new(
        c1.center.x + c1.radius * phi.cos(),
        c1.center.y + c1.radius * phi.sin(),
    );
    let dir = if c1.counter_clockwise {
        (-phi.sin(), phi.cos())
    } else {
        (phi.sin(), -phi.cos())
    };
    let s1 = SpiralGeometry {
        start: s,
        direction: dir,
        length: l1,
        radius: f64::INFINITY,
        start_radius: c1.radius,
        counter_clockwise: c1.counter_clockwise,
    };
    let (m, d1) = s1.point_at(l1);
    let s2 = SpiralGeometry {
        start: m,
        direction: d1,
        length: l2,
        radius: r2,
        start_radius: f64::INFINITY,
        counter_clockwise: !c1.counter_clockwise,
    };
    let (sc, d2) = s2.point_at(l2);
    let side = if s2.counter_clockwise { 1.0 } else { -1.0 };
    let c2 = Point2::new(sc.x - side * r2 * d2.1, sc.y + side * r2 * d2.0);
    (s, s1, s2, sc, c2)
}

/// Floating reverse curve with spirals (S174): solve where the group
/// leaves the previous curve `c1` (between `from` and `to` on it) and, for
/// the two-point method, the second radius, so that the second curve
/// passes through `points` (its end is the last one). Of several
/// solutions the one with the shortest second curve is taken, as Civil 3D
/// does (S174 FC2).
fn place_reverse(
    c1: &ArcGeometry,
    from: Point2,
    l1: f64,
    l2: f64,
    radius: Option<f64>,
    points: &[Point2],
) -> Option<Vec<Segment>> {
    let ang = |p: Point2| (p.y - c1.center.y).atan2(p.x - c1.center.x);
    let a0 = ang(from);
    let sense = if c1.counter_clockwise { 1.0 } else { -1.0 };
    let span = c1.sweep;
    let phi_at = |t: f64| a0 + sense * t;
    let end = *points.last()?;
    // Unknowns: t along c1 and log r2 (only t when the radius is given).
    let resid = |x: &[f64; 2]| -> [f64; 2] {
        let r2 = radius.unwrap_or_else(|| x[1].exp());
        let (.., c2) = reverse_chain(c1, phi_at(x[0]), l1, l2, r2);
        let f0 = c2.distance_to(points[0]) - r2;
        let f1 = points.get(1).map_or(0.0, |p| c2.distance_to(*p) - r2);
        [f0, f1]
    };
    let two = radius.is_none();
    let mut roots: Vec<[f64; 2]> = Vec::new();
    let (nt, nr) = (24, if two { 12 } else { 1 });
    for i in 0..=nt {
        for j in 0..nr {
            let mut x = [
                span * i as f64 / nt as f64,
                (l2 / 4.0).ln() + (1e6f64 / (l2 / 4.0)).ln() * j as f64 / nr as f64,
            ];
            for _ in 0..40 {
                let f = resid(&x);
                if f[0].abs().max(f[1].abs()) < 1e-9 {
                    break;
                }
                let h = [1e-7, 1e-6];
                let mut jac = [[0.0; 2]; 2];
                for (k, hk) in h.iter().enumerate().take(if two { 2 } else { 1 }) {
                    let mut y = x;
                    y[k] += hk;
                    let g = resid(&y);
                    jac[0][k] = (g[0] - f[0]) / hk;
                    jac[1][k] = (g[1] - f[1]) / hk;
                }
                if two {
                    let det = jac[0][0] * jac[1][1] - jac[0][1] * jac[1][0];
                    if det.abs() < 1e-14 {
                        break;
                    }
                    x[0] -= (jac[1][1] * f[0] - jac[0][1] * f[1]) / det;
                    x[1] -= (-jac[1][0] * f[0] + jac[0][0] * f[1]) / det;
                } else {
                    if jac[0][0].abs() < 1e-14 {
                        break;
                    }
                    x[0] -= f[0] / jac[0][0];
                }
                if !x[0].is_finite() || !x[1].is_finite() {
                    break;
                }
            }
            let f = resid(&x);
            let inside = x[0] >= -1e-9
                && x[0] <= span + 1e-9
                && (!two || (x[1] > (l2 / 8.0).ln() && x[1] < 1e6f64.ln()));
            if inside && f[0].abs().max(f[1].abs()) < 1e-6 {
                roots.push(x);
            }
        }
    }
    let mut best: Option<(f64, Vec<Segment>)> = None;
    for x in roots {
        let r2 = radius.unwrap_or_else(|| x[1].exp());
        let (s, s1, s2, sc, c2) = reverse_chain(c1, phi_at(x[0]), l1, l2, r2);
        let arc = arc_on(&circle(c2, r2, s2.counter_clockwise), sc, end);
        if best.as_ref().is_none_or(|(w, _)| arc.sweep < *w - 1e-9) {
            let _ = s;
            best = Some((
                arc.sweep,
                vec![
                    spiral_segment(s1),
                    spiral_segment(s2),
                    Segment::Arc {
                        start: sc,
                        end,
                        geometry: arc,
                    },
                ],
            ));
        }
    }
    best.map(|(_, v)| v)
}

/// Spiral out of `c1` at angle `phi` (A1 = `ratio` · A2) and spiral into a
/// curve of radius `r2` in sense `ccw2`: (start, spirals, SC, centre).
fn spiral_spiral_chain(
    c1: &ArcGeometry,
    phi: f64,
    a2: f64,
    ratio: f64,
    r2: f64,
    ccw2: bool,
) -> (SpiralGeometry, SpiralGeometry, Point2, Point2) {
    let a1 = ratio * a2;
    let (l1, l2) = (a1 * a1 / c1.radius, a2 * a2 / r2);
    let s = Point2::new(
        c1.center.x + c1.radius * phi.cos(),
        c1.center.y + c1.radius * phi.sin(),
    );
    let dir = if c1.counter_clockwise {
        (-phi.sin(), phi.cos())
    } else {
        (phi.sin(), -phi.cos())
    };
    let s1 = SpiralGeometry {
        start: s,
        direction: dir,
        length: l1,
        radius: f64::INFINITY,
        start_radius: c1.radius,
        counter_clockwise: c1.counter_clockwise,
    };
    let (m, d1) = s1.point_at(l1);
    let s2 = SpiralGeometry {
        start: m,
        direction: d1,
        length: l2,
        radius: r2,
        start_radius: f64::INFINITY,
        counter_clockwise: ccw2,
    };
    let (sc, d2) = s2.point_at(l2);
    let side = if ccw2 { 1.0 } else { -1.0 };
    let c2 = Point2::new(sc.x - side * r2 * d2.1, sc.y + side * r2 * d2.0);
    (s1, s2, sc, c2)
}

/// Spiral-spiral between two curves (S175): solve where the spirals leave
/// `c1` and their parameter A so that the second spiral ends tangent to
/// `c2`. Compound or reverse follows from the curves' senses.
fn place_spiral_spiral(
    c1: &ArcGeometry,
    from: Point2,
    c2: &ArcGeometry,
    ratio: f64,
) -> Option<Vec<Segment>> {
    let a0 = (from.y - c1.center.y).atan2(from.x - c1.center.x);
    let sense = if c1.counter_clockwise { 1.0 } else { -1.0 };
    let resid = |x: &[f64; 2]| -> [f64; 2] {
        let (.., c) = spiral_spiral_chain(
            c1,
            a0 + sense * x[0],
            x[1].exp(),
            ratio,
            c2.radius,
            c2.counter_clockwise,
        );
        [c.x - c2.center.x, c.y - c2.center.y]
    };
    let scale = c1.radius.min(c2.radius);
    let mut best: Option<(f64, [f64; 2])> = None;
    let nt = 24;
    for i in 0..=nt {
        for f in [0.5, 1.0, 2.0, 5.0, 20.0] {
            // The start may lie past the curve's stored end, which is then
            // extended (S175 FE0): search the whole circle.
            let mut x = [
                std::f64::consts::TAU * i as f64 / nt as f64,
                (scale * f).ln(),
            ];
            for _ in 0..40 {
                let r = resid(&x);
                if r[0].abs().max(r[1].abs()) < 1e-9 * (1.0 + scale) {
                    break;
                }
                let mut jac = [[0.0; 2]; 2];
                for k in 0..2 {
                    let mut y = x;
                    y[k] += 1e-7;
                    let g = resid(&y);
                    jac[0][k] = (g[0] - r[0]) / 1e-7;
                    jac[1][k] = (g[1] - r[1]) / 1e-7;
                }
                let det = jac[0][0] * jac[1][1] - jac[0][1] * jac[1][0];
                if det.abs() < 1e-14 {
                    break;
                }
                x[0] -= (jac[1][1] * r[0] - jac[0][1] * r[1]) / det;
                x[1] -= (-jac[1][0] * r[0] + jac[0][0] * r[1]) / det;
                if !(x[1] > -5.0 && x[1] < 30.0) || !x[0].is_finite() {
                    break;
                }
            }
            let r = resid(&x);
            let ok =
                x[1] > -5.0 && x[1] < 30.0 && r[0].abs().max(r[1].abs()) < 1e-6 * (1.0 + scale);
            let x = [x[0].rem_euclid(std::f64::consts::TAU), x[1]];
            // Of several solutions, the shortest spirals.
            let a = x[1].exp();
            if ok && best.is_none_or(|(b, _)| a < b - 1e-9) {
                best = Some((a, x));
            }
        }
    }
    let (_, x) = best?;
    let (s1, s2, ..) = spiral_spiral_chain(
        c1,
        a0 + sense * x[0],
        x[1].exp(),
        ratio,
        c2.radius,
        c2.counter_clockwise,
    );
    Some(vec![spiral_segment(s1), spiral_segment(s2)])
}

/// Spiral out of `c1` at `phi` (length `l1`), a line of length `t`, a
/// spiral of length `l2` into a curve of radius `c2.radius` turning as
/// `c2`: (spirals, line ends, centre reached).
fn sls_chain(
    c1: &ArcGeometry,
    phi: f64,
    l1: f64,
    t: f64,
    l2: f64,
    c2: &ArcGeometry,
) -> (SpiralGeometry, (Point2, Point2), SpiralGeometry, Point2) {
    let s = Point2::new(
        c1.center.x + c1.radius * phi.cos(),
        c1.center.y + c1.radius * phi.sin(),
    );
    let dir = if c1.counter_clockwise {
        (-phi.sin(), phi.cos())
    } else {
        (phi.sin(), -phi.cos())
    };
    let s1 = SpiralGeometry {
        start: s,
        direction: dir,
        length: l1,
        radius: f64::INFINITY,
        start_radius: c1.radius,
        counter_clockwise: c1.counter_clockwise,
    };
    let (m, d) = s1.point_at(l1);
    let n = Point2::new(m.x + t * d.0, m.y + t * d.1);
    let s2 = SpiralGeometry {
        start: n,
        direction: d,
        length: l2,
        radius: c2.radius,
        start_radius: f64::INFINITY,
        counter_clockwise: c2.counter_clockwise,
    };
    let (sc, d2) = s2.point_at(l2);
    let side = if c2.counter_clockwise { 1.0 } else { -1.0 };
    let c = Point2::new(
        sc.x - side * c2.radius * d2.1,
        sc.y + side * c2.radius * d2.0,
    );
    (s1, (m, n), s2, c)
}

/// Spiral-line-spiral between two curves (S176): either the spiral lengths
/// are stored and the line length is solved, or the line length is stored
/// and the spirals (with A1 = A2) are solved; in both cases also where the
/// group leaves the first curve. Of several solutions the shortest group.
fn place_spiral_line_spiral(
    c1: &ArcGeometry,
    c2: &ArcGeometry,
    lengths: Option<(f64, f64)>,
    line: Option<f64>,
) -> Option<Vec<Segment>> {
    // x = [phi, v]: v is the line length (lengths stored) or ln A.
    let parts = |x: &[f64; 2]| -> (f64, f64, f64) {
        match (lengths, line) {
            (Some((l1, l2)), _) => (l1, x[1], l2),
            (None, Some(t)) => {
                let a2 = (2.0 * x[1]).exp();
                (a2 / c1.radius, t, a2 / c2.radius)
            }
            _ => (f64::NAN, f64::NAN, f64::NAN),
        }
    };
    let resid = |x: &[f64; 2]| -> [f64; 2] {
        let (l1, t, l2) = parts(x);
        let (.., c) = sls_chain(c1, x[0], l1, t, l2, c2);
        [c.x - c2.center.x, c.y - c2.center.y]
    };
    let scale = c1
        .radius
        .max(c2.radius)
        .max(c1.center.distance_to(c2.center));
    let starts: Vec<f64> = if lengths.is_some() {
        vec![0.1 * scale, 0.5 * scale, scale, 2.0 * scale]
    } else {
        [0.1, 0.3, 1.0, 3.0]
            .iter()
            .map(|f| (f * scale).ln())
            .collect()
    };
    let valid = |x: &[f64; 2]| {
        x[0].is_finite()
            && if lengths.is_some() {
                x[1] >= 0.0 && x[1] < 1e7
            } else {
                x[1] > -5.0 && x[1] < 20.0
            }
    };
    let mut best: Option<(f64, [f64; 2])> = None;
    let nt = 24;
    for i in 0..nt {
        for &v in &starts {
            let mut x = [std::f64::consts::TAU * i as f64 / nt as f64, v];
            for _ in 0..50 {
                let r = resid(&x);
                if r[0].abs().max(r[1].abs()) < 1e-9 * scale {
                    break;
                }
                let mut jac = [[0.0; 2]; 2];
                for k in 0..2 {
                    let h = 1e-7 * x[k].abs().max(1.0);
                    let mut y = x;
                    y[k] += h;
                    let g = resid(&y);
                    jac[0][k] = (g[0] - r[0]) / h;
                    jac[1][k] = (g[1] - r[1]) / h;
                }
                let det = jac[0][0] * jac[1][1] - jac[0][1] * jac[1][0];
                if det.abs() < 1e-14 {
                    break;
                }
                x[0] -= (jac[1][1] * r[0] - jac[0][1] * r[1]) / det;
                x[1] -= (-jac[1][0] * r[0] + jac[0][0] * r[1]) / det;
                if !valid(&x) {
                    break;
                }
            }
            if !valid(&x) {
                continue;
            }
            let r = resid(&x);
            if r[0].abs().max(r[1].abs()) > 1e-6 * scale {
                continue;
            }
            let (l1, t, l2) = parts(&x);
            let total = l1 + t + l2;
            if best.is_none_or(|(b, _)| total < b - 1e-6) {
                best = Some((total, [x[0].rem_euclid(std::f64::consts::TAU), x[1]]));
            }
        }
    }
    let (_, x) = best?;
    let (l1, t, l2) = parts(&x);
    let (s1, (m, n), s2, _) = sls_chain(c1, x[0], l1, t, l2, c2);
    Some(vec![
        spiral_segment(s1),
        Segment::Line { start: m, end: n },
        spiral_segment(s2),
    ])
}

/// Kind prefixes of the long spiral groups between two lines (S177).
pub const PREFIX_SCSCS: u16 = 24;
pub const PREFIX_SCSSCS: u16 = 26;

/// One piece of a long group: a spiral from curvature `k0` to `k1` (signed,
/// + = left) or a curve (k0 = k1) of length `len`.
#[derive(Clone, Copy)]
struct Piece {
    len: f64,
    k0: f64,
    k1: f64,
}

/// Segments for `pieces` laid out from `p` heading along `dir`.
fn lay_pieces(
    p: Point2,
    dir: (f64, f64),
    pieces: &[Piece],
) -> Option<(Vec<Segment>, Point2, (f64, f64))> {
    let (mut p, mut d) = (p, dir);
    let mut out = Vec::new();
    for pc in pieces {
        if pc.len <= 0.0 {
            continue;
        }
        if (pc.k0 - pc.k1).abs() < 1e-15 {
            let (_, end, arc) = curve_by_length(p, d, 1.0 / pc.k0.abs(), pc.len, pc.k0 < 0.0)?;
            let (_, nd) = end_state(&Segment::Arc {
                start: p,
                end,
                geometry: arc,
            });
            out.push(Segment::Arc {
                start: p,
                end,
                geometry: arc,
            });
            p = end;
            d = nd;
        } else {
            let r = |k: f64| {
                if k == 0.0 {
                    f64::INFINITY
                } else {
                    1.0 / k.abs()
                }
            };
            let ccw = pc.k0 + pc.k1 > 0.0;
            let g = SpiralGeometry {
                start: p,
                direction: d,
                length: pc.len,
                radius: r(pc.k1),
                start_radius: r(pc.k0),
                counter_clockwise: ccw,
            };
            let (end, nd) = g.point_at(pc.len);
            out.push(Segment::Spiral {
                start: p,
                end,
                geometry: g,
            });
            p = end;
            d = nd;
        }
    }
    Some((out, p, d))
}

/// SCSCS (compound) or SCSSCS (reverse) between two lines, from a stored
/// TS on the first line (S177): the two curve lengths are solved so that
/// the last spiral ends on the second line in its direction.
fn place_long_group(
    ts: Point2,
    a: &TangentRecord,
    b: &TangentRecord,
    spirals: &[f64],
    (r1, r2): (f64, f64),
    first_left: bool,
    reverse: bool,
) -> Option<Vec<Segment>> {
    let u1 = unit((a.end.x - a.start.x, a.end.y - a.start.y))?;
    let u2 = unit((b.end.x - b.start.x, b.end.y - b.start.y))?;
    let delta = cross(u1, u2).atan2(u1.0 * u2.0 + u1.1 * u2.1);
    let s1 = if first_left { 1.0 } else { -1.0 };
    let s2 = if reverse { -s1 } else { s1 };
    let (k1, k2) = (s1 / r1, s2 / r2);
    let last = *spirals.last()?;
    let pieces = |a1: f64, a2: f64| -> Vec<Piece> {
        let mut v = vec![
            Piece {
                len: spirals[0],
                k0: 0.0,
                k1,
            },
            Piece {
                len: a1,
                k0: k1,
                k1,
            },
        ];
        if reverse {
            v.push(Piece {
                len: spirals[1],
                k0: k1,
                k1: 0.0,
            });
            v.push(Piece {
                len: spirals[2],
                k0: 0.0,
                k1: k2,
            });
        } else {
            v.push(Piece {
                len: spirals[1],
                k0: k1,
                k1: k2,
            });
        }
        v.push(Piece {
            len: a2,
            k0: k2,
            k1: k2,
        });
        v.push(Piece {
            len: last,
            k0: k2,
            k1: 0.0,
        });
        v
    };
    // Heading: Δ = Σ (k0 + k1)/2 · len; gives a2 from a1.
    let turn_without = |a1: f64| -> f64 {
        pieces(a1, 0.0)
            .iter()
            .map(|p| (p.k0 + p.k1) / 2.0 * p.len)
            .sum()
    };
    let a2_of = |a1: f64| (delta - turn_without(a1)) / k2;
    let off = |a1: f64| -> Option<f64> {
        let a2 = a2_of(a1);
        if a2 < -1e-9 {
            return None;
        }
        let (_, end, _) = lay_pieces(ts, u1, &pieces(a1, a2.max(0.0)))?;
        Some(cross(u2, (end.x - b.start.x, end.y - b.start.y)))
    };
    let span = std::f64::consts::TAU * r1;
    let n = 400;
    let mut prev: Option<(f64, f64)> = None;
    let mut root = None;
    for i in 0..=n {
        let x = span * i as f64 / n as f64;
        let Some(f) = off(x) else {
            prev = None;
            continue;
        };
        if let Some((px, pf)) = prev {
            if pf == 0.0 || pf * f < 0.0 {
                let (mut lo, mut hi, mut flo) = (px, x, pf);
                for _ in 0..80 {
                    let m = (lo + hi) / 2.0;
                    let fm = off(m)?;
                    if (fm < 0.0) == (flo < 0.0) {
                        lo = m;
                        flo = fm;
                    } else {
                        hi = m;
                    }
                }
                root = Some((lo + hi) / 2.0);
                break;
            }
        }
        prev = Some((x, f));
    }
    let a1 = root?;
    let (segs, ..) = lay_pieces(ts, u1, &pieces(a1, a2_of(a1).max(0.0)))?;
    Some(segs)
}

/// Centre offset from the TS: k along the tangent, `dist` along the normal.
fn t_c(k: f64, u: (f64, f64), dist: f64, n: (f64, f64)) -> (f64, f64) {
    (k * u.0 + dist * n.0, k * u.1 + dist * n.1)
}

fn arc_at(data: &[u8], bit: usize) -> Option<ArcRecord> {
    let (_, k) = aecc_alignment::read_element_header(data, bit)?;
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
    while bit + 26 <= n {
        if let Some(t) = aecc_alignment::record_at(data, bit) {
            bit = t.end_bit();
            out.push(Element::Tangent(t));
        } else if let Some(g) = group_at(data, bit) {
            bit = g.end_bit;
            out.push(Element::Group(g));
        } else if let Some(sp) = free_spiral_at(data, bit) {
            bit = sp.end_bit;
            out.push(Element::FreeSpiral(sp));
        } else if let Some(sp) = spiral_at(data, bit) {
            bit = sp.end_bit;
            out.push(Element::Spiral(sp));
        } else if let Some(a) = arc_at(data, bit) {
            bit = a.end_bit;
            out.push(Element::Arc(a));
        } else if let Some(g) = line_spiral_at(data, bit) {
            bit = g.end_bit;
            out.push(Element::LineSpiral(g));
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
    /// A clothoid from a line (infinite radius) into a curve.
    Spiral {
        start: Point2,
        end: Point2,
        geometry: SpiralGeometry,
    },
}

impl Segment {
    pub fn start(&self) -> Point2 {
        match *self {
            Self::Line { start, .. } | Self::Arc { start, .. } | Self::Spiral { start, .. } => {
                start
            }
        }
    }
    pub fn end(&self) -> Point2 {
        match *self {
            Self::Line { end, .. } | Self::Arc { end, .. } | Self::Spiral { end, .. } => end,
        }
    }
    pub fn length(&self) -> f64 {
        match self {
            Self::Line { start, end } => start.distance_to(*end),
            Self::Arc { geometry, .. } => geometry.length,
            Self::Spiral { geometry, .. } => geometry.length,
        }
    }
}

/// A clothoid starting tangent to a line: curvature grows linearly from
/// 0 at `start` to 1/`radius` after `length`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiralGeometry {
    pub start: Point2,
    /// Unit direction at the start.
    pub direction: (f64, f64),
    pub length: f64,
    /// Radius at the end (`f64::INFINITY` when the spiral ends on a line).
    pub radius: f64,
    /// Radius at the start (`f64::INFINITY` when it starts from a line).
    pub start_radius: f64,
    pub counter_clockwise: bool,
}

impl SpiralGeometry {
    /// Point and unit direction at distance `s` from the start.
    pub fn point_at(&self, s: f64) -> (Point2, (f64, f64)) {
        if self.start_radius.is_finite() || !self.radius.is_finite() {
            return self.point_at_general(s);
        }
        let a2 = self.radius * self.length;
        let (x, y) = clothoid(s, a2);
        let t = s * s / (2.0 * a2);
        let side = if self.counter_clockwise { 1.0 } else { -1.0 };
        let (u, n) = (
            self.direction,
            (-self.direction.1 * side, self.direction.0 * side),
        );
        let p = Point2::new(
            self.start.x + x * u.0 + y * n.0,
            self.start.y + x * u.1 + y * n.1,
        );
        let (c, sn) = (t.cos(), t.sin());
        (p, (c * u.0 + sn * n.0, c * u.1 + sn * n.1))
    }

    /// Curvature at distance `s` (positive, whatever the sense).
    fn curvature(&self, s: f64) -> f64 {
        let k = |r: f64| if r.is_finite() { 1.0 / r } else { 0.0 };
        let (k0, k1) = (k(self.start_radius), k(self.radius));
        k0 + (k1 - k0) * s / self.length
    }

    /// Any linear-curvature spiral (outward, compound): the heading is
    /// integrated with Simpson's rule.
    fn point_at_general(&self, s: f64) -> (Point2, (f64, f64)) {
        let side = if self.counter_clockwise { 1.0 } else { -1.0 };
        let a0 = self.direction.1.atan2(self.direction.0);
        let k0 = self.curvature(0.0);
        let heading = |t: f64| a0 + side * (k0 * t + (self.curvature(t) - k0) * t / 2.0);
        let n = 512;
        let h = s / n as f64;
        let (mut x, mut y) = (0.0, 0.0);
        for i in 0..=n {
            let w = if i == 0 || i == n {
                1.0
            } else if i % 2 == 1 {
                4.0
            } else {
                2.0
            };
            let a = heading(i as f64 * h);
            x += w * a.cos();
            y += w * a.sin();
        }
        let a = heading(s);
        (
            Point2::new(self.start.x + x * h / 3.0, self.start.y + y * h / 3.0),
            (a.cos(), a.sin()),
        )
    }
}

/// Local coordinates of the clothoid point at arc length `s` (A² = `a2`),
/// turning left: x = ∫cos(t²/2A²), y = ∫sin(t²/2A²) (Simpson's rule).
fn clothoid(s: f64, a2: f64) -> (f64, f64) {
    let n = 256;
    let h = s / n as f64;
    let (mut x, mut y) = (0.0, 0.0);
    for i in 0..=n {
        let t = i as f64 * h;
        let w = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        let a = t * t / (2.0 * a2);
        x += w * a.cos();
        y += w * a.sin();
    }
    (x * h / 3.0, y * h / 3.0)
}

/// Place a line from `p` plus a clothoid of length `l` entering the
/// circle `c` (in its sense): returns (TS, SC, spiral).
pub fn line_spiral_into(
    p: Point2,
    l: f64,
    c: &ArcGeometry,
) -> Option<(Point2, Point2, SpiralGeometry)> {
    let r = c.radius;
    let (xs, ys) = clothoid(l, r * l);
    let theta = l / (2.0 * r);
    let shift = ys - r * (1.0 - theta.cos());
    let k = xs - r * theta.sin();
    let offset = ArcGeometry {
        radius: r + shift,
        ..*c
    };
    let point = ArcGeometry {
        center: p,
        radius: 0.0,
        ..*c
    };
    let (_, foot) = tangent_between(&point, &offset)?;
    let u = unit((foot.x - p.x, foot.y - p.y))?;
    let ts = Point2::new(foot.x - k * u.0, foot.y - k * u.1);
    let g = SpiralGeometry {
        start: ts,
        direction: u,
        length: l,
        radius: r,
        start_radius: f64::INFINITY,
        counter_clockwise: c.counter_clockwise,
    };
    let (sc, _) = g.point_at(l);
    Some((ts, sc, g))
}

/// An element record together with its resolved geometry. A group record
/// (line and spiral) resolves into one entry per part, `part` 0, 1, ….
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedElement {
    pub record: Element,
    pub segment: Segment,
    pub part: usize,
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
    resolve_with(chain, &vec![None; chain.len()], &vec![None; chain.len()])
}

/// Parts of group records placed before [`resolve_with`], by chain index.
type Parts = Option<Vec<Segment>>;

/// (start, end, geometry) of an arc placed before [`resolve_with`].
type Placed = Option<(Point2, Point2, ArcGeometry)>;

/// [`resolve`] with some arcs already placed (`pre`, by chain index): the
/// arcs that continue from the element before them (S117).
fn resolve_with(
    chain: &[Element],
    pre: &[Placed],
    groups: &[Parts],
) -> Result<Vec<ResolvedElement>, DecodeError> {
    let unsupported = |m: String| DecodeError::BrokenChain(m);
    // Free arcs first: (PC, PT, geometry) by chain index.
    let mut free: Vec<Placed> = pre.to_vec();
    // Free spirals between two lines (S165): (start, end, geometry).
    let mut spirals: Vec<Option<(Point2, Point2, SpiralGeometry)>> = vec![None; chain.len()];
    for (i, e) in chain.iter().enumerate() {
        if let Element::FreeSpiral(sp) = e {
            let prev = i.checked_sub(1).and_then(|j| chain.get(j));
            let (Some(Element::Tangent(a)), Some(Element::Tangent(b))) = (prev, chain.get(i + 1))
            else {
                return Err(unsupported(format!(
                    "free spiral {} is not between two lines (no sample yet)",
                    sp.id
                )));
            };
            spirals[i] = Some(
                free_spiral_between(
                    a.start,
                    (a.end.x - a.start.x, a.end.y - a.start.y),
                    b.start,
                    (b.end.x - b.start.x, b.end.y - b.start.y),
                    sp.length,
                    sp.code == FREE_SPIRAL_INCURVE,
                )
                .ok_or_else(|| unsupported(format!("free spiral {} doesn't fit", sp.id)))?,
            );
        }
    }
    for (i, e) in chain.iter().enumerate() {
        if let Element::Arc(a) = e {
            let by_centre = match a.def {
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
            if let Some((center, radius, clockwise)) = by_centre {
                if a.params.closed {
                    if chain.len() != 1 {
                        return Err(unsupported(format!(
                            "arc {}: a whole circle with other elements (no sample yet)",
                            a.id
                        )));
                    }
                    free[i] = Some(full_circle(center, radius, !clockwise));
                } else {
                    // An open curve by its centre runs between the points
                    // where the lines next to it touch its circle (S118).
                    let g = circle(center, radius, !clockwise);
                    let touch = |e: Option<&Element>, end: bool| match e {
                        Some(Element::Tangent(t)) => Some(if end { t.end } else { t.start }),
                        Some(Element::LineSpiral(g)) if end => Some(g.sc),
                        _ => None,
                    };
                    let prev = i.checked_sub(1).and_then(|j| chain.get(j));
                    let Some(end) = touch(chain.get(i + 1), false) else {
                        return Err(unsupported(format!(
                            "arc {}: a curve by its centre not followed by a line \
                             (no sample yet)",
                            a.id
                        )));
                    };
                    // With nothing before it, the curve is a half circle
                    // ending there (S118).
                    let start = match prev {
                        None => Point2::new(2.0 * center.x - end.x, 2.0 * center.y - end.y),
                        _ => touch(prev, true).ok_or_else(|| {
                            unsupported(format!(
                                "arc {}: a curve by its centre after a curve (no sample yet)",
                                a.id
                            ))
                        })?,
                    };
                    let g = arc_on(&g, start, end);
                    let g = if g.sweep <= 1e-12 {
                        ArcGeometry {
                            sweep: std::f64::consts::TAU,
                            length: radius * std::f64::consts::TAU,
                            ..g
                        }
                    } else {
                        g
                    };
                    free[i] = Some((start, end, g));
                }
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
        if let Element::Group(g) = e {
            let parts = groups[i].clone().ok_or_else(|| {
                unsupported(format!("group {}: its neighbours don't place it", g.id))
            })?;
            for (part, segment) in parts.into_iter().enumerate() {
                out.push(ResolvedElement {
                    record: e.clone(),
                    segment,
                    part,
                });
            }
            continue;
        }
        let segment = match e {
            Element::Group(_) => unreachable!(),
            Element::FreeSpiral(_) => {
                let (start, end, geometry) = spirals[i].expect("placed above");
                Segment::Spiral {
                    start,
                    end,
                    geometry,
                }
            }
            Element::Spiral(sp) => {
                let g = sp.geometry();
                Segment::Spiral {
                    start: sp.start,
                    end: g.point_at(sp.length).0,
                    geometry: g,
                }
            }
            Element::LineSpiral(g) => Segment::Line {
                start: g.start,
                end: g.ts,
            },
            Element::Tangent(t) => {
                let group_end =
                    |j: usize| groups[j].as_ref().and_then(|v| v.last()).map(Segment::end);
                let group_start = |j: usize| {
                    groups
                        .get(j)?
                        .as_ref()
                        .and_then(|v| v.first())
                        .map(Segment::start)
                };
                let start = i
                    .checked_sub(1)
                    .and_then(|j| {
                        free[j]
                            .map(|f| f.1)
                            .or(spirals[j].map(|s| s.1))
                            .or(group_end(j))
                    })
                    .unwrap_or(t.start);
                let end = free
                    .get(i + 1)
                    .copied()
                    .flatten()
                    .map(|f| f.0)
                    .or(spirals.get(i + 1).copied().flatten().map(|s| s.0))
                    .or(group_start(i + 1))
                    .unwrap_or(t.end);
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
                        .and_then(|j| {
                            free(chain.get(j)).map(|t| t.end).or_else(|| {
                                // A group ending on the curve trims it (S175).
                                groups.get(j)?.as_ref()?.last().map(Segment::end)
                            })
                        })
                        .unwrap_or(start);
                    let e = free(chain.get(i + 1))
                        .map(|t| t.start)
                        .or_else(|| {
                            // A group starting on the curve trims it (S174).
                            groups.get(i + 1)?.as_ref()?.first().map(Segment::start)
                        })
                        .unwrap_or(end);
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
            part: 0,
        });
        if let Element::LineSpiral(g) = e {
            let c = match chain.get(i + 1) {
                Some(Element::Arc(_)) => open_centre_circle(chain.get(i + 1)),
                _ => None,
            }
            .ok_or_else(|| unsupported(format!("line-spiral {}: no curve after it", g.id)))?;
            let (_, _, sp) = line_spiral_into(g.start, g.spiral_length, &c)
                .ok_or_else(|| unsupported(format!("line-spiral {} doesn't fit", g.id)))?;
            out.push(ResolvedElement {
                record: e.clone(),
                segment: Segment::Spiral {
                    start: g.ts,
                    end: g.sc,
                    geometry: sp,
                },
                part: 1,
            });
        }
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
    /// Spiral groups held only by the JSON layout, without stored pieces
    /// (S216; D-059). civil3drust solves them when reading (D-061): their
    /// pieces are in `elements` as parts of a [`GroupDef::Layout`] group,
    /// and the fixed elements around them are trimmed, so stations run to
    /// the end. A group that cannot be solved keeps the elements after it
    /// in its `following` (stations unknown) and `elements` ends there.
    pub layout_groups: Vec<LayoutGroup>,
    /// Piece records stored for JSON groups and set aside for the
    /// solution from the JSON (D-061); they are still in the record.
    pub set_aside: Vec<Element>,
}

/// A group of the JSON layout whose pieces are not stored (D-059, D-061).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutGroup {
    /// The group's ID in the JSON.
    pub id: i32,
    /// Its JSON `ConstraintType` (2569, 2561, 2565 for the sampled
    /// reverse-spiral methods).
    pub constraint_type: i32,
    /// The fixed elements before and after it (0 at an end).
    pub previous: i32,
    pub next: i32,
    /// Typed parameters, as named in the JSON.
    pub params: Vec<(String, f64)>,
    /// True when civil3drust solved it (its pieces are in `elements`).
    pub solved: bool,
    /// IDs of the piece records stored for it, set aside in favour of the
    /// JSON (empty when none were stored).
    pub stored_pieces: Vec<i32>,
    /// When not solved: the chain after the group, in path order.
    pub following: Vec<ResolvedElement>,
}

impl DecodedElements {
    /// Length of the alignment proper (detached pieces excluded).
    pub fn length(&self) -> f64 {
        self.elements.iter().map(|e| e.segment.length()).sum()
    }
    /// Every element record: the alignment, the elements after unsolved
    /// groups, then the detached pieces.
    pub fn all(&self) -> impl Iterator<Item = &ResolvedElement> {
        self.elements
            .iter()
            .chain(self.layout_groups.iter().flat_map(|g| g.following.iter()))
            .chain(self.detached.iter().flatten())
    }
    /// True when every part of the alignment is resolved (no JSON group
    /// left unsolved), so stations and length are known.
    pub fn is_complete(&self) -> bool {
        self.layout_groups.iter().all(|g| g.solved)
    }
    /// The element records stored in the record, in storage order: those
    /// in use and the pieces set aside for a JSON group's solution, not
    /// the solved pieces (which have no record).
    pub fn stored_records(&self) -> Vec<&Element> {
        let mut v: Vec<&Element> = self
            .all()
            .map(|e| &e.record)
            .filter(|r| !matches!(r, Element::Group(g) if matches!(g.def, GroupDef::Layout { .. })))
            .chain(self.set_aside.iter())
            .collect();
        v.sort_by_key(|r| r.header_bit());
        v.dedup_by_key(|r| r.header_bit());
        v
    }
    /// Number of element records stored (the element count).
    pub fn record_count(&self) -> usize {
        self.stored_records().len()
    }
    /// True if a record holds several parts (spiral groups), which the
    /// editing functions don't handle yet.
    pub fn has_groups(&self) -> bool {
        self.all().any(|e| e.part > 0) || !self.set_aside.is_empty()
    }
    /// Bit offset of the first record's header in storage order.
    pub fn first_record_bit(&self) -> Option<usize> {
        self.stored_records().first().map(|r| r.header_bit())
    }
    /// Bit offset just after the last record in storage order.
    pub fn records_end_bit(&self) -> Option<usize> {
        self.stored_records().iter().map(|r| r.end_bit()).max()
    }
}

/// Decode an alignment of tangents and arcs (read-only).
pub fn decode_elements(rec: &RawRecord) -> Result<DecodedElements, DecodeError> {
    decode_elements_with(rec, DecodeOptions::default())
}

/// How [`decode_elements_with`] reads an alignment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DecodeOptions {
    /// Read the pieces stored for a JSON group instead of solving the
    /// group from the JSON (by default the JSON wins, as in Civil 3D,
    /// S216 → S217). For recovering damaged files.
    pub stored_group_pieces: bool,
}

/// [`decode_elements`] with options.
pub fn decode_elements_with(
    rec: &RawRecord,
    options: DecodeOptions,
) -> Result<DecodedElements, DecodeError> {
    let records = element_records(&rec.data);
    // The station precedes the first record in storage order, which is
    // not the first in the chain once element 1 is deleted (S94).
    let first_stored = records.iter().map(Element::header_bit).min().unwrap_or(0);
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
    // Groups of the JSON layout: the chain is cut at them and their stored
    // pieces, if any, are set aside for the solution from the JSON (D-059,
    // D-061).
    let mut unsolved = layout_groups(rec, &records, options.stored_group_pieces);
    let (set_aside, kept): (Vec<Element>, Vec<Element>) = records
        .into_iter()
        .partition(|e| unsolved.iter().any(|g| g.stored_pieces.contains(&e.id())));
    let mut records = kept;
    for g in &unsolved {
        for e in records.iter_mut() {
            if e.id() == g.previous {
                let b = e.before();
                e.set_links(b, 0);
            } else if e.id() == g.next {
                let a = e.after();
                e.set_links(0, a);
            }
        }
    }
    let mut pieces = aecc_alignment::chain_pieces(&records, head)?;
    let mut elements = resolve_piece(pieces.remove(0))?;
    for g in unsolved.iter_mut() {
        if let Some(i) = pieces.iter().position(|p| p[0].id() == g.next) {
            g.following = resolve_piece(pieces.remove(i))?;
        }
    }
    let mut detached = pieces
        .into_iter()
        .map(resolve_piece)
        .collect::<Result<Vec<_>, _>>()?;
    // Solve the JSON groups along the chain (D-061).
    while let Some(g) = unsolved.iter_mut().find(|g| {
        !g.solved
            && !g.following.is_empty()
            && elements.last().map(|e| e.record.id()) == Some(g.previous)
    }) {
        if !solve_layout_group(&mut elements, g) {
            break;
        }
    }
    // The records keep their stored links (cut only for chaining).
    if !unsolved.is_empty() {
        let stored = element_records(&rec.data);
        let fix = |e: &mut ResolvedElement| {
            if let Some(o) = stored.iter().find(|o| o.id() == e.record.id()) {
                e.record.set_links(o.before(), o.after());
            }
        };
        elements.iter_mut().for_each(fix);
        unsolved
            .iter_mut()
            .flat_map(|g| g.following.iter_mut())
            .for_each(fix);
        detached.iter_mut().flatten().for_each(fix);
    }
    let stored_start_station = aecc_alignment::start_station(&rec.data, first_stored);
    Ok(DecodedElements {
        handle: rec.handle,
        name: aecc_alignment::record_name(&rec.data),
        stored_start_station,
        elements,
        detached,
        layout_groups: unsolved,
        set_aside,
    })
}

/// Solve `g` between the last element of `elements` (A) and the first of
/// its `following` chain (B): trim A, add the pieces as parts of a
/// [`GroupDef::Layout`] group, trim B and move the chain into `elements`.
fn solve_layout_group(elements: &mut Vec<ResolvedElement>, g: &mut LayoutGroup) -> bool {
    use crate::aecc_layout_solve::{solve_group, trim_after, trim_before, Carrier};
    let method = match g.constraint_type {
        2569 => "SSCSS",
        2561 => "SSCS",
        2565 => "SCSS",
        _ => return false,
    };
    let (Some(a), Some(b)) = (elements.last(), g.following.first()) else {
        return false;
    };
    let (Some(ca), Some(cb)) = (Carrier::of(&a.segment), Carrier::of(&b.segment)) else {
        return false;
    };
    let params: Vec<f64> = g.params.iter().map(|p| p.1).collect();
    let Some(sol) = solve_group(
        method,
        &params,
        &ca,
        a.segment.start(),
        a.segment.end(),
        &cb,
    ) else {
        return false;
    };
    let a_seg = trim_before(&ca, &a.segment, sol.leave_a);
    let b_seg = trim_after(&cb, &b.segment, sol.meet_b);
    elements.last_mut().unwrap().segment = a_seg;
    let record = Element::Group(GroupRecord {
        header_bit: usize::MAX,
        end_bit: 0,
        id: g.id,
        before: g.previous,
        after: g.next,
        flag: false,
        code: 0,
        mode: 256,
        def: GroupDef::Layout {
            constraint_type: g.constraint_type,
        },
        params: ParamBlock::unsolved(),
        long_fields: None,
    });
    for (i, seg) in sol.pieces.into_iter().enumerate() {
        elements.push(ResolvedElement {
            record: record.clone(),
            segment: seg,
            part: i + 1,
        });
    }
    let mut following = std::mem::take(&mut g.following);
    following[0].segment = b_seg;
    elements.extend(following);
    g.solved = true;
    true
}

/// The JSON layout's groups whose pieces are not stored: the fixed
/// element before the group links straight to the one after it, or to the
/// group's own ID, which no record has (S216).
fn layout_groups(rec: &RawRecord, records: &[Element], keep_stored: bool) -> Vec<LayoutGroup> {
    use crate::aecc_layout::Json;
    let Some(text) = crate::string_stream::layout_json(&rec.data, rec.handle_bits) else {
        return Vec::new();
    };
    let Ok(j) = Json::parse(&text) else {
        return Vec::new();
    };
    let Some(Json::Arr(objs)) = j.get("RGObjects") else {
        return Vec::new();
    };
    let num = |o: &Json, k: &str| match o.get(k) {
        Some(Json::Num(v)) => Some(*v),
        _ => None,
    };
    objs.iter()
        .filter(|o| num(o, "RGObjectType") == Some(7.0))
        .filter_map(|o| {
            let id = num(o, "Id")? as i32;
            let previous = num(o, "PreviousId")? as i32;
            let next = num(o, "NextId")? as i32;
            let prev = records.iter().find(|e| e.id() == previous);
            let gap = match prev {
                Some(p) => p.after() == next || p.after() == id,
                None => records
                    .iter()
                    .find(|e| e.id() == next)
                    .is_some_and(|n| n.before() == 0 || n.before() == id),
            };
            // Stored pieces: the chain from the element before up to the
            // one after.
            let mut stored_pieces = Vec::new();
            if !gap {
                if keep_stored {
                    return None;
                }
                let mut cur = prev?.after();
                while cur != next && cur != 0 && stored_pieces.len() < 16 {
                    let e = records.iter().find(|e| e.id() == cur)?;
                    stored_pieces.push(cur);
                    cur = e.after();
                }
                if cur != next || stored_pieces.is_empty() {
                    return None;
                }
            }
            let params = match o.get("ConstraintParameters") {
                Some(Json::Obj(kv)) => kv
                    .iter()
                    .filter_map(|(k, v)| match v {
                        Json::Num(x) => Some((k.clone(), *x)),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            Some(LayoutGroup {
                id,
                constraint_type: num(o, "ConstraintType")? as i32,
                previous,
                next,
                params,
                solved: false,
                stored_pieces,
                following: Vec::new(),
            })
        })
        .collect()
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
        Segment::Spiral {
            end, geometry: g, ..
        } => (end, g.point_at(g.length).1),
    }
}

/// The circle of an open curve defined by its centre (codes 0 and 1 with
/// the closed bit clear, S118), in its sense.
fn open_centre_circle(e: Option<&Element>) -> Option<ArcGeometry> {
    let Some(Element::Arc(a)) = e else {
        return None;
    };
    if a.params.closed {
        return None;
    }
    match a.def {
        ArcDef::CenterRadius {
            center,
            radius,
            clockwise,
            ..
        } => Some(circle(center, radius, !clockwise)),
        ArcDef::CenterThrough {
            center,
            through,
            clockwise,
            ..
        } => Some(circle(center, center.distance_to(through), !clockwise)),
        _ => None,
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
    let mut groups: Vec<Parts> = vec![None; chained.len()];
    // Line-spiral groups into an open centre curve (S119).
    for i in 0..chained.len() {
        let Element::LineSpiral(g) = &chained[i] else {
            continue;
        };
        let id = g.id;
        let unsupported = |why: &str| DecodeError::BrokenChain(format!("line-spiral {id}: {why}"));
        let c = open_centre_circle(chained.get(i + 1))
            .ok_or_else(|| unsupported("not before a curve by its centre (no sample yet)"))?;
        let (ts, sc, _) = line_spiral_into(g.start, g.spiral_length, &c)
            .ok_or_else(|| unsupported("start point inside the curve"))?;
        let Element::LineSpiral(g) = &mut chained[i] else {
            unreachable!()
        };
        g.ts = ts;
        g.sc = sc;
    }
    for i in 0..chained.len() {
        let Element::Group(g) = &chained[i] else {
            continue;
        };
        let (id, def) = (g.id, g.def);
        let unsupported = |why: &str| DecodeError::BrokenChain(format!("group {id}: {why}"));
        let line = |j: Option<usize>| match j.and_then(|j| chained.get(j)) {
            Some(Element::Tangent(t)) if t.kind == aecc_alignment::KIND_FIXED => Some(t.clone()),
            _ => None,
        };
        let (prev_line, next_line) = (line(i.checked_sub(1)), line(Some(i + 1)));
        let before = if i > 0 {
            resolve_with(&chained[..i], &pre[..i], &groups[..i])
                .ok()
                .and_then(|v| v.last().map(|e| e.segment))
        } else {
            None
        };
        let next_curve = match chained.get(i + 1) {
            Some(Element::Arc(ArcRecord {
                def: ArcDef::ThreePoint {
                    start, mid, end, ..
                },
                ..
            })) => arc_through(*start, *mid, *end),
            _ => None,
        };
        groups[i] = Some(
            place_group(
                def,
                before.as_ref(),
                prev_line.as_ref(),
                next_line.as_ref(),
                next_curve.as_ref(),
            )
            .ok_or_else(|| unsupported("its neighbours don't place it (no sample yet)"))?,
        );
    }
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
            let prefix = resolve_with(&chained[..i], &pre[..i], &groups[..i])
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
        let next_circle = |chained: &[Element]| -> Result<ArcGeometry, DecodeError> {
            match chained.get(i + 1) {
                Some(Element::Arc(ArcRecord {
                    def:
                        ArcDef::ThreePoint {
                            start, mid, end, ..
                        },
                    ..
                })) => arc_through(*start, *mid, *end)
                    .ok_or_else(|| unsupported("next arc degenerate")),
                e => open_centre_circle(e)
                    .ok_or_else(|| unsupported("not before a three-point or centre curve")),
            }
        };
        // After an open curve by its centre, whose end is not known yet,
        // the line is placed from the circle alone (S118).
        if let Some(c1) = open_centre_circle(chained.get(i - 1)) {
            let point = |p: Point2| ArcGeometry {
                radius: 0.0,
                ..circle(p, 0.0, true)
            };
            let (a, b) = match kind {
                aecc_alignment::KIND_FREE => {
                    let c3 = next_circle(&chained)?;
                    tangent_between(&c1, &c3).ok_or_else(|| unsupported("no common tangent"))?
                }
                aecc_alignment::KIND_FLOATING => {
                    let end = t.end;
                    let (a, _) = tangent_between(&c1, &point(end))
                        .ok_or_else(|| unsupported("end point inside the circle"))?;
                    (a, end)
                }
                _ => return Err(unsupported("after a centre curve (no sample yet)")),
            };
            let Element::Tangent(t) = &mut chained[i] else {
                unreachable!()
            };
            t.start = a;
            t.end = b;
            continue;
        }
        let prefix = resolve_with(&chained[..i], &pre[..i], &groups[..i])
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
        let c3 = next_circle(&chained)?;
        let (a, b) = tangent_between(&c1, &c3).ok_or_else(|| unsupported("no common tangent"))?;
        let Element::Tangent(t) = &mut chained[i] else {
            unreachable!()
        };
        t.start = a;
        t.end = b;
    }
    let elements = resolve_with(&chained, &pre, &groups)?;
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
    use crate::aecc_alignment::{decode, ELEMENT_HEADER};

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
                Segment::Spiral { .. } => "S",
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
