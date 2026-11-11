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
/// The BL after a symmetric parabola's two BDs (S300).
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
}

/// A vertical curve between two tangents.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileCurve {
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub def: CurveDef,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProfileElement {
    Tangent(ProfileTangent),
    Curve(ProfileCurve),
}

impl ProfileElement {
    pub fn id(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.id,
            Self::Curve(c) => c.id,
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
        (PREFIX_PARABOLA, 0 | 2) => {
            let (lead, k) = raw::read_bd(data, k)?;
            let (v, k) = raw::read_bd(data, k)?;
            let (mark, k) = raw::read_bl(data, k)?;
            if lead != 0.0 || mark != PARABOLA_MARK {
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
        };
        cur = if next == 0 { None } else { Some(by_id(next)?) };
    }
    (pending.is_none() && !pvis.is_empty()).then_some(ProfileLayout { pvis, curves })
}
