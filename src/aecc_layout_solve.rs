//! Solving the spiral groups held only by the JSON layout (D-061).
//!
//! A group of the sampled reverse-spiral methods sits between two fixed
//! elements, A before and B after (fixed lines or curves). Its typed
//! parameters give every piece's length and curvature except the length
//! of the middle curve; the point where it leaves A is free too. Those two
//! unknowns are fixed by the two conditions at the end: the last piece
//! ends on B's line or circle, heading along B. Newton's method from a
//! grid of starts; of the solutions, the one leaving A nearest to A's
//! stored end is taken (the previous solution, which Civil 3D also keeps
//! when it re-solves). Checked against Civil 3D's own solutions (S178,
//! S217).

use crate::aecc_elements::{ArcGeometry, Segment, SpiralGeometry};
use crate::alignment::Point2;

/// A fixed element's carrier, travelled in the alignment's direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Carrier {
    /// Through `point`, unit direction `dir`.
    Line { point: Point2, dir: (f64, f64) },
    Circle {
        center: Point2,
        radius: f64,
        counter_clockwise: bool,
    },
}

impl Carrier {
    /// The carrier of a resolved line or arc segment.
    pub fn of(seg: &Segment) -> Option<Carrier> {
        match *seg {
            Segment::Line { start, end } => {
                let l = start.distance_to(end);
                (l > 0.0).then(|| Carrier::Line {
                    point: start,
                    dir: ((end.x - start.x) / l, (end.y - start.y) / l),
                })
            }
            Segment::Arc { geometry: g, .. } => Some(Carrier::Circle {
                center: g.center,
                radius: g.radius,
                counter_clockwise: g.counter_clockwise,
            }),
            Segment::Spiral { .. } => None,
        }
    }

    /// Signed curvature (+ left).
    fn curvature(&self) -> f64 {
        match *self {
            Carrier::Line { .. } => 0.0,
            Carrier::Circle {
                radius,
                counter_clockwise,
                ..
            } => sign(counter_clockwise) / radius,
        }
    }

    /// Point and heading (radians) after `s` along the carrier from `from`.
    fn walk(&self, from: Point2, s: f64) -> (Point2, f64) {
        match *self {
            Carrier::Line { dir, .. } => (
                Point2::new(from.x + dir.0 * s, from.y + dir.1 * s),
                dir.1.atan2(dir.0),
            ),
            Carrier::Circle {
                center,
                radius,
                counter_clockwise,
            } => {
                let a0 = (from.y - center.y).atan2(from.x - center.x);
                let a = a0 + sign(counter_clockwise) * s / radius;
                (
                    Point2::new(center.x + radius * a.cos(), center.y + radius * a.sin()),
                    a + sign(counter_clockwise) * std::f64::consts::FRAC_PI_2,
                )
            }
        }
    }

    /// The two end conditions for a point and heading: distance off the
    /// carrier and the angle to its direction there.
    fn miss(&self, p: Point2, heading: f64) -> (f64, f64) {
        match *self {
            Carrier::Line { point, dir } => (
                dir.0 * (p.y - point.y) - dir.1 * (p.x - point.x),
                wrap(heading - dir.1.atan2(dir.0)),
            ),
            Carrier::Circle {
                center,
                radius,
                counter_clockwise,
            } => {
                let a = (p.y - center.y).atan2(p.x - center.x);
                let tangent = a + sign(counter_clockwise) * std::f64::consts::FRAC_PI_2;
                (p.distance_to(center) - radius, wrap(heading - tangent))
            }
        }
    }

    /// Distance along the carrier from `from` to the foot of `p`, in the
    /// travel direction (for a circle in [0, 2πR)).
    fn along(&self, from: Point2, p: Point2) -> f64 {
        match *self {
            Carrier::Line { dir, .. } => dir.0 * (p.x - from.x) + dir.1 * (p.y - from.y),
            Carrier::Circle {
                center,
                radius,
                counter_clockwise,
            } => {
                let a0 = (from.y - center.y).atan2(from.x - center.x);
                let a = (p.y - center.y).atan2(p.x - center.x);
                let d = sign(counter_clockwise) * (a - a0);
                d.rem_euclid(std::f64::consts::TAU) * radius
            }
        }
    }

    /// The segment from `a` to `b` along the carrier.
    fn segment(&self, a: Point2, b: Point2) -> Segment {
        match *self {
            Carrier::Line { .. } => Segment::Line { start: a, end: b },
            Carrier::Circle {
                center,
                radius,
                counter_clockwise,
            } => {
                let len = self.along(a, b);
                Segment::Arc {
                    start: a,
                    end: b,
                    geometry: ArcGeometry {
                        center,
                        radius,
                        counter_clockwise,
                        sweep: len / radius,
                        length: len,
                    },
                }
            }
        }
    }
}

fn sign(ccw: bool) -> f64 {
    if ccw {
        1.0
    } else {
        -1.0
    }
}

fn wrap(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    (a + std::f64::consts::PI).rem_euclid(t) - std::f64::consts::PI
}

/// One piece of a group: length and signed curvature at its start and end
/// (a spiral when they differ, the middle curve when they are equal).
#[derive(Debug, Clone, Copy)]
struct Piece {
    length: f64,
    k0: f64,
    k1: f64,
}

/// The pieces of `method` (`"SSCSS"`, `"SSCS"`, `"SCSS"`) with `params`
/// in the JSON order, the middle curve `lc` long, its sense `sigma`.
fn pieces(method: &str, p: &[f64], ka: f64, kb: f64, sigma: f64, lc: f64) -> Option<Vec<Piece>> {
    let piece = |length, k0, k1| Piece { length, k0, k1 };
    Some(match method {
        "SSCSS" => {
            let k = sigma / p[2];
            vec![
                piece(p[0], ka, 0.0),
                piece(p[1], 0.0, k),
                piece(lc, k, k),
                piece(p[3], k, 0.0),
                piece(p[4], 0.0, kb),
            ]
        }
        "SSCS" => {
            let k = sigma / p[2];
            vec![
                piece(p[0], ka, 0.0),
                piece(p[1], 0.0, k),
                piece(lc, k, k),
                piece(p[3], k, 0.0),
            ]
        }
        "SCSS" => {
            let k = sigma / p[1];
            vec![
                piece(p[0], ka, k),
                piece(lc, k, k),
                piece(p[2], k, 0.0),
                piece(p[3], 0.0, kb),
            ]
        }
        _ => return None,
    })
}

/// The segment of one piece from `p` with `heading`.
fn place(pc: &Piece, p: Point2, heading: f64) -> Segment {
    let dir = (heading.cos(), heading.sin());
    if pc.k0 == pc.k1 {
        let r = 1.0 / pc.k0.abs();
        let ccw = pc.k0 > 0.0;
        let n = if ccw {
            (-dir.1, dir.0)
        } else {
            (dir.1, -dir.0)
        };
        let center = Point2::new(p.x + n.0 * r, p.y + n.1 * r);
        let a0 = (p.y - center.y).atan2(p.x - center.x);
        let a1 = a0 + sign(ccw) * pc.length / r;
        let end = Point2::new(center.x + r * a1.cos(), center.y + r * a1.sin());
        return Segment::Arc {
            start: p,
            end,
            geometry: ArcGeometry {
                center,
                radius: r,
                counter_clockwise: ccw,
                sweep: pc.length / r,
                length: pc.length,
            },
        };
    }
    let k = if pc.k0 != 0.0 { pc.k0 } else { pc.k1 };
    let radius = |k: f64| {
        if k == 0.0 {
            f64::INFINITY
        } else {
            1.0 / k.abs()
        }
    };
    let geometry = SpiralGeometry {
        start: p,
        direction: dir,
        length: pc.length,
        radius: radius(pc.k1),
        start_radius: radius(pc.k0),
        counter_clockwise: k > 0.0,
    };
    let (end, _) = geometry.point_at(pc.length);
    Segment::Spiral {
        start: p,
        end,
        geometry,
    }
}

/// End point and heading of a placed piece (heading by its curvature, so
/// it does not depend on the integrator's direction output).
fn after(pc: &Piece, seg: &Segment, heading: f64) -> (Point2, f64) {
    (seg.end(), heading + (pc.k0 + pc.k1) / 2.0 * pc.length)
}

/// A solved group: the point where it leaves A, its pieces in order and
/// the point where it meets B.
#[derive(Debug, Clone, PartialEq)]
pub struct SolvedGroup {
    pub leave_a: Point2,
    pub pieces: Vec<Segment>,
    pub meet_b: Point2,
}

/// Lay the pieces from the point `s` along A (from `a_start`).
fn lay(
    a: &Carrier,
    a_start: Point2,
    s: f64,
    list: &[Piece],
) -> (Point2, Vec<Segment>, Point2, f64) {
    let (p0, h0) = a.walk(a_start, s);
    let (mut p, mut h) = (p0, h0);
    let mut segs = Vec::with_capacity(list.len());
    for pc in list {
        let seg = place(pc, p, h);
        (p, h) = after(pc, &seg, h);
        segs.push(seg);
    }
    (p0, segs, p, h)
}

/// Solve a group of `method` with `params` (JSON order) between A
/// (carrier `a`, its segment starting at `a_start`, stored end `a_end`)
/// and B (carrier `b`). `None` when no solution is found.
pub fn solve_group(
    method: &str,
    params: &[f64],
    a: &Carrier,
    a_start: Point2,
    a_end: Point2,
    b: &Carrier,
) -> Option<SolvedGroup> {
    let (ka, kb) = (a.curvature(), b.curvature());
    // The middle curve turns against A (reverse from a curve) or against
    // B (from a line); try both senses if neither tells.
    let sigmas: Vec<f64> = match method {
        "SSCSS" | "SSCS" if ka != 0.0 => vec![-ka.signum()],
        "SCSS" if kb != 0.0 => vec![-kb.signum()],
        _ => vec![1.0, -1.0],
    };
    let radius = match method {
        "SCSS" => *params.get(1)?,
        _ => *params.get(2)?,
    };
    let stored = a.along(a_start, a_end);
    let span = match a {
        Carrier::Circle { radius, .. } => std::f64::consts::TAU * radius,
        Carrier::Line { .. } => stored.max(1.0) * 2.0,
    };
    let residual = |sigma: f64, s: f64, lc: f64| -> Option<(f64, f64)> {
        let list = pieces(method, params, ka, kb, sigma, lc)?;
        let (_, _, p, h) = lay(a, a_start, s, &list);
        Some(b.miss(p, h))
    };
    let mut best: Option<(f64, f64, f64)> = None; // (sigma, s, lc)
    let scale = radius.max(1.0);
    for &sigma in &sigmas {
        for i in 0..24 {
            for j in 0..12 {
                let mut s = span * i as f64 / 24.0;
                let mut lc = std::f64::consts::TAU * radius * j as f64 / 12.0;
                let mut ok = false;
                for _ in 0..60 {
                    let (f1, f2) = residual(sigma, s, lc)?;
                    if f1.abs() < 1e-9 * scale && f2.abs() < 1e-12 {
                        ok = true;
                        break;
                    }
                    let h1 = 1e-6 * scale;
                    let (a1, a2) = residual(sigma, s + h1, lc)?;
                    let (b1, b2) = residual(sigma, s, lc + h1)?;
                    let (j11, j21) = ((a1 - f1) / h1, (a2 - f2) / h1);
                    let (j12, j22) = ((b1 - f1) / h1, (b2 - f2) / h1);
                    let det = j11 * j22 - j12 * j21;
                    if det.abs() < 1e-18 {
                        break;
                    }
                    let ds = (f1 * j22 - f2 * j12) / det;
                    let dl = (j11 * f2 - j21 * f1) / det;
                    let lim = scale;
                    s -= ds.clamp(-lim, lim);
                    lc -= dl.clamp(-lim, lim);
                    if !(s.is_finite() && lc.is_finite()) {
                        break;
                    }
                }
                if !ok || lc < 0.0 || s < 0.0 {
                    continue;
                }
                if let Carrier::Circle { .. } = a {
                    s = s.rem_euclid(span);
                }
                let key = (s - stored).abs();
                if best.is_none_or(|(_, bs, bl)| {
                    let bk = (bs - stored).abs();
                    key < bk - 1e-6 || ((key - bk).abs() <= 1e-6 && lc < bl)
                }) {
                    best = Some((sigma, s, lc));
                }
            }
        }
    }
    let (sigma, s, lc) = best?;
    let list = pieces(method, params, ka, kb, sigma, lc)?;
    let (leave_a, pieces, meet_b, _) = lay(a, a_start, s, &list);
    Some(SolvedGroup {
        leave_a,
        pieces,
        meet_b,
    })
}

/// A's segment cut at the point where the group leaves it.
pub fn trim_before(a: &Carrier, seg: &Segment, at: Point2) -> Segment {
    a.segment(seg.start(), at)
}

/// B's segment starting where the group meets it.
pub fn trim_after(b: &Carrier, seg: &Segment, at: Point2) -> Segment {
    b.segment(at, seg.end())
}
