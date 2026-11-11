//! Profiles (`AECC_PROFILE`, class name `AeccImpVAlignment`) and their links
//! to alignments and profile views — read-only, first look (S298–S305;
//! docs/profile-object.md).
//!
//! A layout profile keeps its geometry in element records shaped like an
//! alignment's: an 11-bit kind prefix (2 tangent, 6 vertical curve), a
//! 23-bit header (`00`, `0x08`, `0x00`, five 0 bits), then `BL kind, BL
//! id, BL before, BL after`. Points are (station, elevation).
//!
//! ```text
//! tangent (prefix 2): RD station, RD elevation (start), RD station, RD
//!                     elevation (end) — the points it was drawn through,
//!                     not clipped to the alignment (S299: −100 … 1900)
//! curve (prefix 6):   BD 0, BD length (the only value Civil 3D asks for:
//!                     S306 typed 150), BL 0x1001, BL 0, 3 bits, BD −1 × 3,
//!                     BL 0
//! ```

use acadrust::document::CadDocument;

use crate::alignment::Point2;
use crate::handle_refs;
use crate::raw::{self, RawRecord};

/// Kind prefix of a profile tangent record.
pub const PREFIX_TANGENT: u32 = 2;
/// Kind prefix of a profile vertical-curve record.
pub const PREFIX_CURVE: u32 = 6;
/// The BL after a vertical curve's two BDs (S300).
pub const CURVE_MARK: i32 = 0x1001;

/// A profile tangent: `x` is the station, `y` the elevation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileTangent {
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub start: Point2,
    pub end: Point2,
}

/// A vertical curve between two tangents (S300, S306), placed from them by
/// its length.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileCurve {
    pub id: i32,
    pub before: i32,
    pub after: i32,
    /// The BD after the links: 0 in every sample.
    pub lead: f64,
    /// The curve length (S306: typed 150; S300 488.7677).
    pub length: f64,
    /// The three BDs after the mark: −1 in every sample.
    pub tail: [f64; 3],
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

/// The element after the header at `h` (prefix already checked).
fn element_at(data: &[u8], h: usize, prefix: u32) -> Option<(ProfileElement, usize)> {
    let (kind, k) = raw::read_bl(data, h + 23)?;
    let (id, k) = raw::read_bl(data, k)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let small = |v: i32| (0..100_000).contains(&v);
    if kind != 0 || id < 1 || !small(id) || !small(before) || !small(after) {
        return None;
    }
    match prefix {
        PREFIX_TANGENT => {
            if k + 256 > raw::bit_len(data) {
                return None;
            }
            let v: Vec<f64> = (0..4).map(|i| raw::get_f64(data, k + 64 * i)).collect();
            if !v.iter().all(|x| x.is_finite() && x.abs() < 1e9) {
                return None;
            }
            let t = ProfileTangent {
                id,
                before,
                after,
                start: Point2::new(v[0], v[1]),
                end: Point2::new(v[2], v[3]),
            };
            Some((ProfileElement::Tangent(t), k + 256))
        }
        PREFIX_CURVE => {
            let (a, k) = raw::read_bd(data, k)?;
            let (b, k) = raw::read_bd(data, k)?;
            let (mark, k) = raw::read_bl(data, k)?;
            let (_, k) = raw::read_bl(data, k)?;
            let mut k = k + 3;
            let mut tail = [0.0; 3];
            for t in &mut tail {
                let (v, n) = raw::read_bd(data, k)?;
                *t = v;
                k = n;
            }
            let (_, k) = raw::read_bl(data, k)?;
            if mark != CURVE_MARK || !a.is_finite() || !b.is_finite() {
                return None;
            }
            let c = ProfileCurve {
                id,
                before,
                after,
                lead: a,
                length: b,
                tail,
            };
            Some((ProfileElement::Curve(c), k))
        }
        _ => None,
    }
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

/// A symmetric parabola placed at a PVI (S300, S306).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedCurve {
    pub id: i32,
    /// The PVI: station, elevation.
    pub pvi: Point2,
    pub length: f64,
    pub grade_in: f64,
    pub grade_out: f64,
}

impl PlacedCurve {
    /// Start (BVC) station and elevation.
    pub fn start(&self) -> Point2 {
        let h = self.length / 2.0;
        Point2::new(self.pvi.x - h, self.pvi.y - self.grade_in * h)
    }
    /// End (EVC) station and elevation.
    pub fn end(&self) -> Point2 {
        let h = self.length / 2.0;
        Point2::new(self.pvi.x + h, self.pvi.y + self.grade_out * h)
    }
    /// Grade change (out − in).
    pub fn grade_change(&self) -> f64 {
        self.grade_out - self.grade_in
    }
    /// K: length per unit (percent) of grade change, as Civil 3D shows it.
    pub fn k(&self) -> f64 {
        self.length / (100.0 * self.grade_change().abs())
    }
    /// Elevation at `station` on the curve.
    pub fn elevation_at(&self, station: f64) -> f64 {
        let s = self.start();
        let x = station - s.x;
        s.y + self.grade_in * x + self.grade_change() / (2.0 * self.length) * x * x
    }
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

/// Follow the links from the first tangent: each tangent gives a PVI, a
/// curve between two tangents sits on their shared point. `None` when the
/// chain is broken or a tangent does not start where the previous one ends.
pub fn layout(elements: &[ProfileElement]) -> Option<ProfileLayout> {
    let by_id = |id: i32| elements.iter().find(|e| e.id() == id);
    let first = elements.iter().find(|e| match e {
        ProfileElement::Tangent(t) => t.before == 0,
        _ => false,
    })?;
    let grade = |t: &ProfileTangent| (t.end.y - t.start.y) / (t.end.x - t.start.x);
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
                            curves.push(PlacedCurve {
                                id: c.id,
                                pvi: t.start,
                                length: c.length,
                                grade_in: grade(p),
                                grade_out: grade(t),
                            });
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
