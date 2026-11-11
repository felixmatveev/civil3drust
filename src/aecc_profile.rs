//! Profiles (`AECC_PROFILE`, class name `AeccImpVAlignment`) and their links
//! to alignments and profile views — read-only (S298–S307;
//! docs/profile-object.md, docs/profile-methods.md).
//!
//! A layout profile keeps its geometry in element records shaped like an
//! alignment's: an 11-bit kind prefix, a 23-bit header (`00`, `0x08`,
//! `0x00`, five 0 bits), then `BL kind, BL id, BL before, BL after`. Points
//! are (station, elevation). The prefix gives the entity type, the kind
//! how it is constrained:
//!
//! ```text
//! prefix 2 tangent, kind 0 (two points): RD station, RD elevation × 2
//! prefix 6 symmetric parabola:  BD 0, BD value — kind 0 length, 2 K;
//!                               then BL 0x1001, BL 0, 3 bits, BD −1 × 3, BL 0
//! prefix 4 circular:            kind 0 BD length, 9 BD radius,
//!                               7 RD station, RD elevation (through point)
//! prefix 8 asymmetric parabola: kind 10 BD length 1, BD length 2
//! prefix 6 fixed parabola:      kind 11 BD 0, 3 × (RD station, RD elevation);
//!                               kind 13 BD 0, 2 points, BL curve (0x1000 sag,
//!                               0x1001 crest), BL parameter (0x2000 K), BD value
//!                               kind 12 BD 0, 2 points, BL end (0 start, 1
//!                               end), BD grade (%) there
//! prefix 6 floating parabola:   kind 32 BD 0, point, BL −1, BL parameter
//!                               (0x2000 K, 0x2002 radius), BD value;
//!                               kind 34 BD 0, point, BL 1, BD grade (%) there
//! prefix 2 floating tangent:    kind 4 end point; free tangent: kind 9, nothing
//! ```

use acadrust::document::CadDocument;

use crate::alignment::Point2;
use crate::handle_refs;
use crate::raw::{self, RawRecord};

/// Kind prefix of a profile tangent record.
pub const PREFIX_TANGENT: u32 = 2;
/// Kind prefix of a circular vertical curve (S307).
pub const PREFIX_CIRCULAR: u32 = 4;
/// Kind prefix of a symmetric parabola (S300, S306, S307).
pub const PREFIX_PARABOLA: u32 = 6;
/// Kind prefix of an asymmetric parabola (S307).
pub const PREFIX_ASYMMETRIC: u32 = 8;
/// The BL after a free symmetric parabola's two BDs: [`CURVE_SAG`] + 1,
/// a crest, in every sample (S300, S306, S307).
pub const PARABOLA_MARK: i32 = 0x1001;

/// A profile tangent: `x` is the station, `y` the elevation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileTangent {
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub start: Point2,
    pub end: Point2,
}

/// How a free vertical curve between two tangents is given (the API's
/// `Constraint2`, S306, S307).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveDef {
    /// Symmetric parabola by length (kind 0, `CurveLength`).
    ParabolaLength(f64),
    /// Symmetric parabola by K (kind 2, `KValue`).
    ParabolaK(f64),
    /// Circular curve by (horizontal) length (kind 0, `CurveLength`).
    CircularLength(f64),
    /// Circular curve by radius (kind 9, `Radius`).
    CircularRadius(f64),
    /// Circular curve through a point (kind 7, `PassThroughPt`).
    CircularThrough(Point2),
    /// Asymmetric parabola by its two lengths (kind 10).
    Asymmetric { length1: f64, length2: f64 },
    /// Fixed parabola through three points: start, a point on it, end
    /// (prefix 6, kind 11, `ThreePassPoints`, S308).
    ParabolaThreePoints([Point2; 3]),
    /// Fixed parabola from a start to an end point with a parameter
    /// (kind 13, `TwoPassPointsAndParameter`, S308): `curve` is 0x1000 for a
    /// sag, 0x1001 for a crest; `param` 0x2000 is K.
    ParabolaTwoPoints {
        start: Point2,
        end: Point2,
        curve: i32,
        param: i32,
        value: f64,
    },
    /// Floating parabola tangent to the element before, ending at a point,
    /// with a parameter (kind 32, `PreviousSubEntPassPtAndParameter`, S308):
    /// `curve` −1 (not given), `param` 0x2000 is K.
    ParabolaFloating {
        through: Point2,
        curve: i32,
        param: i32,
        value: f64,
    },
    /// Fixed parabola from a start to an end point with the grade at one
    /// end, in percent (kind 12, `TwoPassPointsAndGrade`, S309): `at_end`
    /// false for the start ("grade at start point", "entity end, through
    /// point" — the grade of the element before), true for the end.
    ParabolaTwoPointsGrade {
        start: Point2,
        end: Point2,
        at_end: bool,
        grade_percent: f64,
    },
    /// Floating parabola tangent to the element before, ending at a point
    /// with a grade there, in percent (kind 34,
    /// `PreviousSubEntPassPtWithGrade`, S309).
    ParabolaFloatingGrade { through: Point2, grade_percent: f64 },
}

/// The parameter code of K (S308).
pub const PARAM_K: i32 = 0x2000;
/// The parameter code of the radius (S309: R = 100 K for a parabola).
pub const PARAM_RADIUS: i32 = 0x2002;
/// The curve code of a sag (crest is this + 1, S300, S308).
pub const CURVE_SAG: i32 = 0x1000;

/// A vertical curve between two tangents.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileCurve {
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub def: CurveDef,
}

/// A tangent that is not fixed by two points (S308).
#[derive(Debug, Clone, PartialEq)]
pub struct DependentTangent {
    pub id: i32,
    pub before: i32,
    pub after: i32,
    /// Floating (kind 4): tangent to the element before, ending at this
    /// point (`FloatOnPrev`: Civil 3D's "through point" and "grade, length"
    /// methods both store it). `None` for a free tangent (kind 9, between
    /// two curves, `NoConstraints`).
    pub through: Option<Point2>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileElement {
    Tangent(ProfileTangent),
    Curve(ProfileCurve),
    Dependent(DependentTangent),
}

impl ProfileElement {
    pub fn id(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.id,
            Self::Curve(c) => c.id,
            Self::Dependent(t) => t.id,
        }
    }
    pub fn before(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.before,
            Self::Curve(c) => c.before,
            Self::Dependent(t) => t.before,
        }
    }
    pub fn after(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.after,
            Self::Curve(c) => c.after,
            Self::Dependent(t) => t.after,
        }
    }
}

fn bits(data: &[u8], bit: usize, n: usize) -> u32 {
    (0..n).fold(0, |v, k| v << 1 | raw::get_bit(data, bit + k) as u32)
}

/// The 23-bit element header at `bit`, if there is one.
fn header_at(data: &[u8], bit: usize) -> bool {
    bit + 23 <= raw::bit_len(data)
        && bits(data, bit, 2) == 0
        && raw::get_byte(data, bit + 2) == 0x08
        && raw::get_byte(data, bit + 10) == 0x00
        && bits(data, bit + 18, 5) == 0
}

fn rd_point(data: &[u8], k: usize) -> Option<(Point2, usize)> {
    if k + 128 > raw::bit_len(data) {
        return None;
    }
    let p = Point2::new(raw::get_f64(data, k), raw::get_f64(data, k + 64));
    (p.x.is_finite() && p.y.is_finite() && p.x.abs() < 1e9 && p.y.abs() < 1e9)
        .then_some((p, k + 128))
}

/// The element after the header at `h`.
fn element_at(data: &[u8], h: usize, prefix: u32) -> Option<(ProfileElement, usize)> {
    let (kind, k) = raw::read_bl(data, h + 23)?;
    let (id, k) = raw::read_bl(data, k)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let small = |v: i32| (0..100_000).contains(&v);
    if !(0..64).contains(&kind) || id < 1 || !small(id) || !small(before) || !small(after) {
        return None;
    }
    let positive = |v: f64| (v.is_finite() && v > 0.0).then_some(v);
    let (def, end) = match (prefix, kind) {
        (PREFIX_TANGENT, 0) => {
            let (start, k) = rd_point(data, k)?;
            let (end, k) = rd_point(data, k)?;
            let t = ProfileTangent {
                id,
                before,
                after,
                start,
                end,
            };
            return Some((ProfileElement::Tangent(t), k));
        }
        (PREFIX_TANGENT, 4 | 9) => {
            let (through, k) = if kind == 4 {
                let (p, k) = rd_point(data, k)?;
                (Some(p), k)
            } else {
                (None, k)
            };
            let t = DependentTangent {
                id,
                before,
                after,
                through,
            };
            return Some((ProfileElement::Dependent(t), k));
        }
        (PREFIX_PARABOLA, 11) => {
            let (lead, k) = raw::read_bd(data, k)?;
            let (a, k) = rd_point(data, k)?;
            let (b, k) = rd_point(data, k)?;
            let (c, k) = rd_point(data, k)?;
            if lead != 0.0 {
                return None;
            }
            (CurveDef::ParabolaThreePoints([a, b, c]), k)
        }
        (PREFIX_PARABOLA, 13) => {
            let (lead, k) = raw::read_bd(data, k)?;
            if lead != 0.0 {
                return None;
            }
            let (start, k) = rd_point(data, k)?;
            let (end, k) = rd_point(data, k)?;
            let (curve, k) = raw::read_bl(data, k)?;
            let (param, k) = raw::read_bl(data, k)?;
            let (value, k) = raw::read_bd(data, k)?;
            let def = CurveDef::ParabolaTwoPoints {
                start,
                end,
                curve,
                param,
                value: positive(value)?,
            };
            (def, k)
        }
        (PREFIX_PARABOLA, 12) => {
            let (lead, k) = raw::read_bd(data, k)?;
            let (start, k) = rd_point(data, k)?;
            let (end, k) = rd_point(data, k)?;
            let (which, k) = raw::read_bl(data, k)?;
            let (grade_percent, k) = raw::read_bd(data, k)?;
            if lead != 0.0 || !(0..=1).contains(&which) || !grade_percent.is_finite() {
                return None;
            }
            let def = CurveDef::ParabolaTwoPointsGrade {
                start,
                end,
                at_end: which == 1,
                grade_percent,
            };
            (def, k)
        }
        (PREFIX_PARABOLA, 34) => {
            let (lead, k) = raw::read_bd(data, k)?;
            let (through, k) = rd_point(data, k)?;
            let (which, k) = raw::read_bl(data, k)?;
            let (grade_percent, k) = raw::read_bd(data, k)?;
            if lead != 0.0 || which != 1 || !grade_percent.is_finite() {
                return None;
            }
            (
                CurveDef::ParabolaFloatingGrade {
                    through,
                    grade_percent,
                },
                k,
            )
        }
        (PREFIX_PARABOLA, 32) => {
            let (lead, k) = raw::read_bd(data, k)?;
            if lead != 0.0 {
                return None;
            }
            let (through, k) = rd_point(data, k)?;
            let (curve, k) = raw::read_bl(data, k)?;
            let (param, k) = raw::read_bl(data, k)?;
            let (value, k) = raw::read_bd(data, k)?;
            let def = CurveDef::ParabolaFloating {
                through,
                curve,
                param,
                value: positive(value)?,
            };
            (def, k)
        }
        (PREFIX_PARABOLA, 0 | 2) => {
            let (lead, k) = raw::read_bd(data, k)?;
            let (v, k) = raw::read_bd(data, k)?;
            let (mark, k) = raw::read_bl(data, k)?;
            if lead != 0.0 || !(CURVE_SAG..=CURVE_SAG + 1).contains(&mark) {
                return None;
            }
            let v = positive(v)?;
            let def = if kind == 0 {
                CurveDef::ParabolaLength(v)
            } else {
                CurveDef::ParabolaK(v)
            };
            (def, k)
        }
        (PREFIX_CIRCULAR, 0 | 9) => {
            let (v, k) = raw::read_bd(data, k)?;
            let v = positive(v)?;
            let def = if kind == 0 {
                CurveDef::CircularLength(v)
            } else {
                CurveDef::CircularRadius(v)
            };
            (def, k)
        }
        (PREFIX_CIRCULAR, 7) => {
            let (p, k) = rd_point(data, k)?;
            (CurveDef::CircularThrough(p), k)
        }
        (PREFIX_ASYMMETRIC, 10) => {
            let (l1, k) = raw::read_bd(data, k)?;
            let (l2, k) = raw::read_bd(data, k)?;
            let def = CurveDef::Asymmetric {
                length1: positive(l1)?,
                length2: positive(l2)?,
            };
            (def, k)
        }
        _ => return None,
    };
    let c = ProfileCurve {
        id,
        before,
        after,
        def,
    };
    Some((ProfileElement::Curve(c), end))
}

/// The element records of a profile record's data, in record order.
pub fn profile_elements(data: &[u8]) -> Vec<ProfileElement> {
    let mut out = Vec::new();
    let n = raw::bit_len(data);
    let mut bit = 11;
    while bit + 23 <= n {
        if header_at(data, bit) {
            let prefix = bits(data, bit - 11, 11);
            if let Some((e, end)) = element_at(data, bit, prefix) {
                out.push(e);
                bit = end;
                continue;
            }
        }
        bit += 1;
    }
    out
}

/// The handles `rec` points to whose class is `class`, in stream order
/// (`code` 4 soft, 5 hard pointers; `None` for any).
pub fn pointers_to(doc: &CadDocument, rec: &RawRecord, class: &str, code: Option<u8>) -> Vec<u64> {
    handle_refs::of_record(rec)
        .into_iter()
        .filter(|r| r.handle != 0 && code.is_none_or(|c| c == r.code))
        .filter(|r| raw::class_of(doc, r.handle).as_deref() == Some(class))
        .map(|r| r.handle)
        .collect()
}

/// The alignment a profile (or profile view) belongs to: its first hard
/// pointer to an `AECC_ALIGNMENT` (S299–S305).
pub fn parent_alignment(doc: &CadDocument, rec: &RawRecord) -> Option<u64> {
    pointers_to(doc, rec, "AECC_ALIGNMENT", Some(5))
        .first()
        .copied()
}

/// The shape of a placed vertical curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveShape {
    /// Symmetric parabola of (horizontal) length.
    Parabola { length: f64 },
    /// Two parabolas meeting under (over) the PVI (S307).
    Asymmetric { length1: f64, length2: f64 },
    /// A circular arc tangent to both tangents: centre and radius.
    Circular { center: Point2, radius: f64 },
}

/// A vertical curve placed at its PVI (S300, S306, S307).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedCurve {
    pub id: i32,
    /// The PVI: station, elevation.
    pub pvi: Point2,
    pub grade_in: f64,
    pub grade_out: f64,
    pub shape: CurveShape,
}

/// Unit direction of a grade.
fn dir(g: f64) -> (f64, f64) {
    let l = (1.0 + g * g).sqrt();
    (1.0 / l, g / l)
}

impl PlacedCurve {
    /// Grade change (out − in).
    pub fn grade_change(&self) -> f64 {
        self.grade_out - self.grade_in
    }
    /// 1 for a sag curve (centre above), −1 for a crest.
    fn turn(&self) -> f64 {
        if self.grade_change() >= 0.0 {
            1.0
        } else {
            -1.0
        }
    }
    /// The circular curve's tangent points (on the incoming and outgoing
    /// tangent).
    fn circle_ends(&self, center: Point2, radius: f64) -> (Point2, Point2) {
        let s = self.turn();
        let (u1, u2) = (dir(self.grade_in), dir(self.grade_out));
        // The inside normal (left for a sag) from the centre back.
        let n1 = (-u1.1 * s, u1.0 * s);
        let n2 = (-u2.1 * s, u2.0 * s);
        (
            Point2::new(center.x - radius * n1.0, center.y - radius * n1.1),
            Point2::new(center.x - radius * n2.0, center.y - radius * n2.1),
        )
    }
    /// Start (BVC) station and elevation.
    pub fn start(&self) -> Point2 {
        match self.shape {
            CurveShape::Parabola { length } => {
                let h = length / 2.0;
                Point2::new(self.pvi.x - h, self.pvi.y - self.grade_in * h)
            }
            CurveShape::Asymmetric { length1, .. } => {
                Point2::new(self.pvi.x - length1, self.pvi.y - self.grade_in * length1)
            }
            CurveShape::Circular { center, radius } => self.circle_ends(center, radius).0,
        }
    }
    /// End (EVC) station and elevation.
    pub fn end(&self) -> Point2 {
        match self.shape {
            CurveShape::Parabola { length } => {
                let h = length / 2.0;
                Point2::new(self.pvi.x + h, self.pvi.y + self.grade_out * h)
            }
            CurveShape::Asymmetric { length2, .. } => {
                Point2::new(self.pvi.x + length2, self.pvi.y + self.grade_out * length2)
            }
            CurveShape::Circular { center, radius } => self.circle_ends(center, radius).1,
        }
    }
    /// Horizontal length (EVC − BVC station).
    pub fn length(&self) -> f64 {
        self.end().x - self.start().x
    }
    /// K: length per percent of grade change, as Civil 3D shows it.
    pub fn k(&self) -> f64 {
        self.length() / (100.0 * self.grade_change().abs())
    }
    /// Elevation at `station` on the curve.
    pub fn elevation_at(&self, station: f64) -> f64 {
        let a = self.grade_change();
        match self.shape {
            CurveShape::Parabola { length } => {
                let s = self.start();
                let x = station - s.x;
                s.y + self.grade_in * x + a / (2.0 * length) * x * x
            }
            CurveShape::Asymmetric { length1, length2 } => {
                // Offset at the PVI: A·L1·L2 / (2(L1 + L2)).
                let e = a * length1 * length2 / (2.0 * (length1 + length2));
                if station <= self.pvi.x {
                    let s = self.start();
                    let x = station - s.x;
                    s.y + self.grade_in * x + e / (length1 * length1) * x * x
                } else {
                    let s = self.end();
                    let x = s.x - station;
                    s.y - self.grade_out * x + e / (length2 * length2) * x * x
                }
            }
            CurveShape::Circular { center, radius } => {
                let dx = station - center.x;
                center.y - self.turn() * (radius * radius - dx * dx).max(0.0).sqrt()
            }
        }
    }
    /// The high (crest) or low (sag) point, where the grade is 0 — the
    /// end with the lower or higher elevation if the grade keeps its sign.
    pub fn high_low_point(&self) -> Point2 {
        let (s, e) = (self.start(), self.end());
        let at = |x: f64| Point2::new(x, self.elevation_at(x));
        if self.grade_in * self.grade_out > 0.0 {
            let pick = if (e.y > s.y) == (self.turn() < 0.0) {
                e
            } else {
                s
            };
            return pick;
        }
        let x = match self.shape {
            CurveShape::Parabola { length } => s.x - self.grade_in * length / self.grade_change(),
            CurveShape::Circular { center, .. } => center.x,
            CurveShape::Asymmetric { length1, length2 } => {
                let e = self.grade_change() * length1 * length2 / (2.0 * (length1 + length2));
                let x1 = s.x - self.grade_in * length1 * length1 / (2.0 * e);
                if x1 <= self.pvi.x {
                    x1
                } else {
                    self.end().x + self.grade_out * length2 * length2 / (2.0 * e)
                }
            }
        };
        at(x)
    }
}

/// A circular curve tangent to both grades at the PVI: centre and radius
/// from its definition. The length given is the horizontal one (EVC − BVC
/// station, S307: Civil 3D's `Length`).
fn circle(pvi: Point2, g1: f64, g2: f64, def: CurveDef) -> Option<(Point2, f64)> {
    let s = if g2 >= g1 { 1.0 } else { -1.0 };
    let (u1, u2) = (dir(g1), dir(g2));
    // The angle between the tangents, and the centre's offset per unit
    // radius: back along tangent 1 by tan(Δ/2), then across.
    let delta = (u1.0 * u2.1 - u1.1 * u2.0).abs().asin();
    let t = (delta / 2.0).tan();
    let w = (-t * u1.0 - s * u1.1, -t * u1.1 + s * u1.0);
    let span = (u1.1 - u2.1).abs(); // horizontal length per unit radius
    let r = match def {
        CurveDef::CircularRadius(r) => r,
        CurveDef::CircularLength(l) => l / span,
        CurveDef::CircularThrough(p) => {
            // |PVI + R·w − P| = R: (|w|² − 1)R² − 2(w·d)R + |d|² = 0.
            let d = (p.x - pvi.x, p.y - pvi.y);
            let a = w.0 * w.0 + w.1 * w.1 - 1.0;
            let b = -2.0 * (w.0 * d.0 + w.1 * d.1);
            let c = d.0 * d.0 + d.1 * d.1;
            let disc = (b * b - 4.0 * a * c).max(0.0).sqrt();
            let roots = [(-b + disc) / (2.0 * a), (-b - disc) / (2.0 * a)];
            // The radius whose arc spans the point's station: the tangent
            // points are at PVI + R·(w + s·(u.y, −u.x)) for each tangent.
            *roots.iter().filter(|r| **r > 0.0).find(|r| {
                let x1 = pvi.x + **r * (w.0 + s * u1.1);
                let x2 = pvi.x + **r * (w.0 + s * u2.1);
                (x1.min(x2) - 1e-9..=x1.max(x2) + 1e-9).contains(&p.x)
            })?
        }
        _ => return None,
    };
    (r.is_finite() && r > 0.0).then(|| (Point2::new(pvi.x + r * w.0, pvi.y + r * w.1), r))
}

/// A layout profile resolved from its element records: PVIs in order (the
/// tangents' through points) and the curves placed at them.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileLayout {
    pub pvis: Vec<Point2>,
    pub curves: Vec<PlacedCurve>,
}

impl ProfileLayout {
    /// Start and end station (first and last PVI).
    pub fn station_range(&self) -> (f64, f64) {
        (self.pvis[0].x, self.pvis[self.pvis.len() - 1].x)
    }
}

/// Place `c` at the PVI between tangents `a` and `b`.
fn place(
    c: &ProfileCurve,
    pvi: Point2,
    a: &ProfileTangent,
    b: &ProfileTangent,
) -> Option<PlacedCurve> {
    let grade = |t: &ProfileTangent| (t.end.y - t.start.y) / (t.end.x - t.start.x);
    let (g1, g2) = (grade(a), grade(b));
    let shape = match c.def {
        CurveDef::ParabolaLength(length) => CurveShape::Parabola { length },
        CurveDef::ParabolaK(k) => CurveShape::Parabola {
            length: k * 100.0 * (g2 - g1).abs(),
        },
        CurveDef::Asymmetric { length1, length2 } => CurveShape::Asymmetric { length1, length2 },
        def => {
            let (center, radius) = circle(pvi, g1, g2, def)?;
            CurveShape::Circular { center, radius }
        }
    };
    Some(PlacedCurve {
        id: c.id,
        pvi,
        grade_in: g1,
        grade_out: g2,
        shape,
    })
}

/// Follow the links from the first tangent: each tangent gives a PVI, a
/// curve between two tangents sits on their shared point. `None` when the
/// chain is broken or a tangent does not start where the previous one ends.
pub fn layout(elements: &[ProfileElement]) -> Option<ProfileLayout> {
    let by_id = |id: i32| elements.iter().find(|e| e.id() == id);
    let first = elements.iter().find(|e| match e {
        ProfileElement::Tangent(t) => t.before == 0,
        _ => false,
    })?;
    let mut pvis = Vec::new();
    let mut curves = Vec::new();
    let mut prev: Option<&ProfileTangent> = None;
    let mut pending: Option<&ProfileCurve> = None;
    let mut cur = Some(first);
    let mut guard = 0;
    while let Some(e) = cur {
        guard += 1;
        if guard > elements.len() {
            return None;
        }
        let next = match e {
            ProfileElement::Tangent(t) => {
                match prev {
                    None => pvis.push(t.start),
                    Some(p) => {
                        let tol = 1e-6 * (1.0 + t.start.x.abs());
                        if (p.end.x - t.start.x).abs() > tol || (p.end.y - t.start.y).abs() > tol {
                            return None;
                        }
                        if let Some(c) = pending.take() {
                            curves.push(place(c, t.start, p, t)?);
                        }
                    }
                }
                pvis.push(t.end);
                prev = Some(t);
                t.after
            }
            ProfileElement::Curve(c) => {
                pending = Some(c);
                c.after
            }
            // Only PVI layouts (fixed tangents, free curves); see
            // [`segments`] for the rest.
            ProfileElement::Dependent(_) => return None,
        };
        cur = if next == 0 { None } else { Some(by_id(next)?) };
    }
    (pending.is_none() && !pvis.is_empty()).then_some(ProfileLayout { pvis, curves })
}

/// The shape of a resolved profile segment (S308).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentShape {
    /// A straight grade line: elevation = `y0 + grade · (station − x0)`.
    Line { x0: f64, y0: f64, grade: f64 },
    /// A parabola: elevation = `a · s² + b · s + c` (s = station).
    Parabola { a: f64, b: f64, c: f64 },
    /// A free curve placed at its PVI (any type).
    Placed(PlacedCurve),
}

impl SegmentShape {
    pub fn elevation_at(&self, x: f64) -> f64 {
        match *self {
            Self::Line { x0, y0, grade } => y0 + grade * (x - x0),
            Self::Parabola { a, b, c } => (a * x + b) * x + c,
            Self::Placed(p) => p.elevation_at(x),
        }
    }
    pub fn grade_at(&self, x: f64) -> f64 {
        match *self {
            Self::Line { grade, .. } => grade,
            Self::Parabola { a, b, .. } => 2.0 * a * x + b,
            Self::Placed(p) => {
                let h = 1e-6;
                (p.elevation_at(x + h) - p.elevation_at(x - h)) / (2.0 * h)
            }
        }
    }
    fn line(p: Point2, q: Point2) -> Self {
        Self::Line {
            x0: p.x,
            y0: p.y,
            grade: (q.y - p.y) / (q.x - p.x),
        }
    }
    fn tangent_at(&self, x: f64) -> Self {
        Self::Line {
            x0: x,
            y0: self.elevation_at(x),
            grade: self.grade_at(x),
        }
    }
}

/// One resolved profile entity: from `start` to `end` (station, elevation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileSegment {
    pub id: i32,
    pub start: Point2,
    pub end: Point2,
    pub shape: SegmentShape,
}

/// The parabola through three points (stations distinct).
fn parabola_3(p: [Point2; 3]) -> Option<SegmentShape> {
    let [p0, p1, p2] = p;
    let d = (p0.x - p1.x) * (p0.x - p2.x) * (p1.x - p2.x);
    if d == 0.0 {
        return None;
    }
    let a = (p2.x * (p1.y - p0.y) + p1.x * (p0.y - p2.y) + p0.x * (p2.y - p1.y)) / d;
    let b =
        (p2.x * p2.x * (p0.y - p1.y) + p1.x * p1.x * (p2.y - p0.y) + p0.x * p0.x * (p1.y - p2.y))
            / d;
    let c = p0.y - (a * p0.x + b) * p0.x;
    Some(SegmentShape::Parabola { a, b, c })
}

/// The parabola's `a` (elevation'' = 2a) from a parameter: K or radius
/// (R = 100 K), `sag` choosing the sign.
fn curvature(param: i32, value: f64, sag: bool) -> Option<f64> {
    let k = match param {
        PARAM_K => value,
        PARAM_RADIUS => value / 100.0,
        _ => return None,
    };
    Some(if sag { 1.0 } else { -1.0 } / (200.0 * k))
}

/// The parabola with curvature `a` (elevation'' = 2a) through `p` with
/// grade `g` at `p`.
fn parabola_at(p: Point2, g: f64, a: f64) -> SegmentShape {
    let b = g - 2.0 * a * p.x;
    let c = p.y - (a * p.x + b) * p.x;
    SegmentShape::Parabola { a, b, c }
}

/// The station where a tangent from `curve` passes through `p`, searched
/// on `[lo, hi]` (S308's floating tangent).
fn tangent_from(curve: &SegmentShape, p: Point2, lo: f64, hi: f64) -> Option<f64> {
    let f = |t: f64| curve.elevation_at(t) + curve.grade_at(t) * (p.x - t) - p.y;
    root(f, lo, hi)
}

/// A root of `f` on `[lo, hi]`: scan for a sign change, then bisect.
fn root(f: impl Fn(f64) -> f64, lo: f64, hi: f64) -> Option<f64> {
    let n = 400;
    let mut prev = (lo, f(lo));
    for i in 1..=n {
        let x = lo + (hi - lo) * i as f64 / n as f64;
        let cur = (x, f(x));
        if prev.1 == 0.0 {
            return Some(prev.0);
        }
        if prev.1.is_finite() && cur.1.is_finite() && prev.1.signum() != cur.1.signum() {
            let (mut a, mut b) = (prev.0, cur.0);
            let fa = prev.1;
            for _ in 0..200 {
                let m = 0.5 * (a + b);
                let fm = f(m);
                if !fm.is_finite() {
                    break;
                }
                if (fm > 0.0) == (fa > 0.0) {
                    a = m;
                } else {
                    b = m;
                }
            }
            return Some(0.5 * (a + b));
        }
        prev = cur;
    }
    None
}

/// The stations where one line touches both `c1` (on `[lo1, hi1]`) and
/// `c2`: the free tangent between two curves (S308).
fn common_tangent(
    c1: &SegmentShape,
    (lo1, hi1): (f64, f64),
    c2: &SegmentShape,
    (lo2, hi2): (f64, f64),
) -> Option<(f64, f64)> {
    // For t1 on curve 1, the tangent line touches curve 2 where curve 2's
    // grade equals it; solve that point's offset from the line.
    let touch = |t1: f64| -> Option<(f64, f64)> {
        let g = c1.grade_at(t1);
        let t2 = root(|t| c2.grade_at(t) - g, lo2, hi2)?;
        let gap = c2.elevation_at(t2) - (c1.elevation_at(t1) + g * (t2 - t1));
        Some((t2, gap))
    };
    let t1 = root(|t| touch(t).map_or(f64::NAN, |v| v.1), lo1, hi1)?;
    Some((t1, touch(t1)?.0))
}

/// Resolve every entity of a profile into a segment with its start and end
/// (S299–S308): fixed tangents and fixed curves stand on their own points,
/// free curves sit at their PVI, a floating curve hangs from the tangent
/// before it, a floating tangent leaves the curve before it towards its
/// point, a free tangent touches the curves on both sides. `None` when the
/// chain cannot be followed or a constraint has no solution.
pub fn segments(elements: &[ProfileElement]) -> Option<Vec<ProfileSegment>> {
    // The chain in link order.
    let mut chain = Vec::new();
    let mut cur = elements.iter().find(|e| e.before() == 0)?;
    loop {
        chain.push(cur);
        if chain.len() > elements.len() {
            return None;
        }
        match cur.after() {
            0 => break,
            n => cur = elements.iter().find(|e| e.id() == n)?,
        }
    }
    let n = chain.len();
    // Shapes and natural ends of the elements that stand on their own.
    let mut shape: Vec<Option<SegmentShape>> = vec![None; n];
    let mut start: Vec<Option<Point2>> = vec![None; n];
    let mut end: Vec<Option<Point2>> = vec![None; n];
    for (i, e) in chain.iter().enumerate() {
        match e {
            ProfileElement::Tangent(t) => {
                shape[i] = Some(SegmentShape::line(t.start, t.end));
                start[i] = Some(t.start);
                end[i] = Some(t.end);
            }
            ProfileElement::Curve(c) => match c.def {
                CurveDef::ParabolaThreePoints(p) => {
                    shape[i] = Some(parabola_3(p)?);
                    start[i] = Some(p[0]);
                    end[i] = Some(p[2]);
                }
                CurveDef::ParabolaTwoPoints {
                    start: p,
                    end: q,
                    curve,
                    param,
                    value,
                } => {
                    let l = q.x - p.x;
                    let a = curvature(param, value, curve == CURVE_SAG)?;
                    // Chord grade = grade at the middle.
                    let g = (q.y - p.y) / l - a * l;
                    shape[i] = Some(parabola_at(p, g, a));
                    start[i] = Some(p);
                    end[i] = Some(q);
                }
                CurveDef::ParabolaTwoPointsGrade {
                    start: p,
                    end: q,
                    at_end,
                    grade_percent,
                } => {
                    let (l, g) = (q.x - p.x, grade_percent / 100.0);
                    let chord = (q.y - p.y) / l;
                    let a = if at_end {
                        (g - chord) / l
                    } else {
                        (chord - g) / l
                    };
                    let g0 = if at_end { g - 2.0 * a * l } else { g };
                    shape[i] = Some(parabola_at(p, g0, a));
                    start[i] = Some(p);
                    end[i] = Some(q);
                }
                _ => {}
            },
            ProfileElement::Dependent(t) => end[i] = t.through,
        }
    }
    // Elements that depend on their neighbours, in chain order.
    for i in 0..n {
        let prev = i.checked_sub(1);
        match chain[i] {
            ProfileElement::Curve(c) => match c.def {
                CurveDef::ParabolaFloating {
                    through,
                    param,
                    value,
                    ..
                } => {
                    let line = shape[prev?]?;
                    // Hangs from the line: elevation = line + a (s − s0)²,
                    // through the point; a's sign from which side it lies.
                    let off = through.y - line.elevation_at(through.x);
                    let a = curvature(param, value, off > 0.0)?;
                    let s0 = through.x - (off / a).sqrt();
                    let p0 = Point2::new(s0, line.elevation_at(s0));
                    shape[i] = Some(parabola_at(p0, line.grade_at(s0), a));
                    start[i] = Some(p0);
                    end[i] = Some(through);
                    end[prev?] = Some(p0);
                }
                CurveDef::ParabolaFloatingGrade {
                    through,
                    grade_percent,
                } => {
                    // Hangs from the line, reaching the point with the
                    // grade: a = (gP − g) / (2 d), d = 2 (yP − line) / (gP − g).
                    let line = shape[prev?]?;
                    let (g, gp) = (line.grade_at(through.x), grade_percent / 100.0);
                    let d = 2.0 * (through.y - line.elevation_at(through.x)) / (gp - g);
                    let a = (gp - g) / (2.0 * d);
                    let s0 = through.x - d;
                    let p0 = Point2::new(s0, line.elevation_at(s0));
                    if !(d > 0.0 && a.is_finite()) {
                        return None;
                    }
                    shape[i] = Some(parabola_at(p0, g, a));
                    start[i] = Some(p0);
                    end[i] = Some(through);
                    end[prev?] = Some(p0);
                }
                CurveDef::ParabolaThreePoints(_)
                | CurveDef::ParabolaTwoPoints { .. }
                | CurveDef::ParabolaTwoPointsGrade { .. } => {}
                _ => {
                    // A free curve between two fixed tangents.
                    let (ProfileElement::Tangent(a), Some(ProfileElement::Tangent(b))) =
                        (chain[prev?], chain.get(i + 1))
                    else {
                        return None;
                    };
                    let p = place(c, a.end, a, b)?;
                    shape[i] = Some(SegmentShape::Placed(p));
                    start[i] = Some(p.start());
                    end[i] = Some(p.end());
                    end[prev?] = start[i];
                    start[i + 1] = end[i];
                }
            },
            ProfileElement::Dependent(t) => {
                let before = shape[prev?]?;
                match t.through {
                    Some(p) => {
                        let lo = start[prev?]?.x;
                        let s = tangent_from(&before, p, lo, p.x)?;
                        let q = Point2::new(s, before.elevation_at(s));
                        shape[i] = Some(SegmentShape::line(q, p));
                        start[i] = Some(q);
                        end[prev?] = Some(q);
                    }
                    None => {
                        let after = shape.get(i + 1).copied().flatten()?;
                        let r1 = (start[prev?]?.x, end[prev?]?.x);
                        let r2 = (start[i + 1]?.x, end[i + 1]?.x);
                        let (t1, t2) = common_tangent(&before, r1, &after, r2)?;
                        let p = Point2::new(t1, before.elevation_at(t1));
                        let q = Point2::new(t2, after.elevation_at(t2));
                        shape[i] = Some(before.tangent_at(t1));
                        (start[i], end[i]) = (Some(p), Some(q));
                        end[prev?] = Some(p);
                        start[i + 1] = Some(q);
                    }
                }
            }
            ProfileElement::Tangent(_) => {}
        }
    }
    (0..n)
        .map(|i| {
            Some(ProfileSegment {
                id: chain[i].id(),
                start: start[i]?,
                end: end[i]?,
                shape: shape[i]?,
            })
        })
        .collect()
}

/// A profile's Profile Locking settings and pending notifications, kept in
/// its `AECC_NOTIFICATIONINFOS` object (S310; S302 for the list).
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileLocking {
    /// "Anchor profile geometry points to alignment geometry points" (the
    /// default); false: "Anchor profile to alignment start".
    pub anchor_to_geometry: bool,
    /// "Modify affected entities" (the default); false: "Delete affected
    /// entities".
    pub modify_affected: bool,
    /// "Notify which entities are affected" (the default).
    pub notify: bool,
    /// Entities waiting to be reported as affected (S302: 3, 1, 2 after the
    /// alignment edit), by ID.
    pub affected: Vec<i32>,
}

/// The locking settings of profile `rec`, from the
/// `AECC_NOTIFICATIONINFOS` object it owns. After the common object data
/// (EED, reactors, two bits): 18 bits (the same in every sample), `BL n`,
/// `n × BL entity ID`, `B anchor to geometry`, `B modify`, `B notify`.
pub fn profile_locking(doc: &CadDocument, rec: &RawRecord) -> Option<ProfileLocking> {
    let h = *pointers_to(doc, rec, "AECC_NOTIFICATIONINFOS", Some(3)).first()?;
    let n = raw::records(doc, "AECC_NOTIFICATIONINFOS")
        .into_iter()
        .find(|r| r.handle == h)?;
    let data = &n.data;
    let eed = crate::aecc_entity::eed_bit(data)?;
    // EED size 0, no reactors (both BS / BL `10`).
    if bits(data, eed, 4) != 0b1010 {
        return None;
    }
    let (count, mut k) = raw::read_bl(data, eed + 24)?;
    if !(0..10_000).contains(&count) {
        return None;
    }
    let mut affected = Vec::new();
    for _ in 0..count {
        let (id, q) = raw::read_bl(data, k)?;
        affected.push(id);
        k = q;
    }
    if k + 3 > raw::bit_len(data) - n.handle_bits {
        return None;
    }
    Some(ProfileLocking {
        anchor_to_geometry: raw::get_bit(data, k) == 1,
        modify_affected: raw::get_bit(data, k + 1) == 1,
        notify: raw::get_bit(data, k + 2) == 1,
        affected,
    })
}

/// Bit of the first element record's header (after its 11-bit prefix).
pub fn first_element_bit(data: &[u8]) -> Option<usize> {
    let n = raw::bit_len(data);
    (11..n.saturating_sub(23)).find(|&bit| {
        header_at(data, bit) && element_at(data, bit, bits(data, bit - 11, 11)).is_some()
    })
}

/// A profile's Design Criteria tab (S312).
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileCriteria {
    /// "Use criteria-based design", "Use design criteria file", "Use design
    /// check set": the three bits 24, 23 and 22 before the first element
    /// record's header (all three on or off together in S312; the order is
    /// assumed from the alignment's, `aecc_criteria`).
    pub criteria_based: bool,
    pub criteria_file: bool,
    pub check_set: bool,
    /// The `AECC_PROFILE_DESIGN_CHECK_SET` pointed to (a hard pointer after
    /// the alignment's; null when unused).
    pub check_set_handle: Option<u64>,
}

/// The Design Criteria settings of profile `rec`. With criteria on, the
/// string stream also holds the criteria file (its path, the same with
/// `<DATADIR>`) and the Minimum K Table name twice, after the name.
pub fn profile_criteria(doc: &CadDocument, rec: &RawRecord) -> Option<ProfileCriteria> {
    let h = first_element_bit(&rec.data)?;
    let b = |k: usize| raw::get_bit(&rec.data, h - k) == 1;
    Some(ProfileCriteria {
        criteria_based: b(24),
        criteria_file: b(23),
        check_set: b(22),
        check_set_handle: pointers_to(doc, rec, "AECC_PROFILE_DESIGN_CHECK_SET", Some(5))
            .first()
            .copied(),
    })
}

/// The elevation at `station` on a resolved profile, `None` outside it.
pub fn elevation_at(segments: &[ProfileSegment], station: f64) -> Option<f64> {
    segments
        .iter()
        .find(|s| s.start.x - 1e-9 <= station && station <= s.end.x + 1e-9)
        .map(|s| s.shape.elevation_at(station))
}

/// The profile as a 3D polyline over its alignment: a point every `step`
/// (and at every entity boundary) where both exist, `[x, y, elevation]`
/// (x, y from the alignment at the station, elevation from the profile).
pub fn sample_3d(
    alignment: &crate::aecc_elements::DecodedElements,
    segments: &[ProfileSegment],
    step: f64,
) -> Vec<[f64; 3]> {
    let (Some(first), Some(last)) = (segments.first(), segments.last()) else {
        return Vec::new();
    };
    let a0 = alignment.stored_start_station.unwrap_or(0.0);
    let (lo, hi) = (
        first.start.x.max(a0),
        last.end.x.min(a0 + alignment.length()),
    );
    if step.is_nan() || step <= 0.0 || hi <= lo {
        return Vec::new();
    }
    let mut stations: Vec<f64> = Vec::new();
    let mut s = (lo / step).ceil() * step;
    stations.push(lo);
    while s < hi {
        stations.push(s);
        s += step;
    }
    stations.push(hi);
    stations.extend(
        segments
            .iter()
            .flat_map(|g| [g.start.x, g.end.x])
            .filter(|x| (lo..=hi).contains(x)),
    );
    stations.sort_by(f64::total_cmp);
    stations.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    stations
        .into_iter()
        .filter_map(|st| {
            let p = alignment.point_at_station(st)?;
            Some([p.x, p.y, elevation_at(segments, st)?])
        })
        .collect()
}

/// The profile's name: the string after the label point names, which end
/// with `PVI` (S299–S312; the generic name lookup finds a label name).
pub fn profile_name(rec: &RawRecord) -> Option<String> {
    let ss = crate::string_stream::parse(&rec.data, rec.handle_bits).ok()?;
    let i = ss.strings.iter().position(|s| s.text == "PVI")?;
    ss.strings.get(i + 1).map(|s| s.text.clone())
}
