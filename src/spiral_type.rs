//! Spiral types (S232): the BL after a spiral record's IDs (MIST10) names
//! the transition curve Civil 3D lays out, and each type has its own shape.
//!
//! Codes from the owner's sample (one alignment per type, named after the
//! type in Civil 3D's list); shapes from fitting each type's proxy graphics
//! (S232, outward spirals R 200, L 200) against the textbook definitions of
//! these curves. Four types vary the curvature along the arc length `s`
//! (t = s / L):
//!
//! | code | type | curvature rise f(t) |
//! |------|------|---------------------|
//! | 256 | Clothoid | t |
//! | 259 | Bloss | 3t² − 2t³ |
//! | 261 | Sinusoidal | t − sin(2πt) / 2π |
//! | 262 | Bi-Quadratic | 2t² (t ≤ ½), 1 − 2(1 − t)² |
//!
//! Three are defined along the tangent `x` from the straight end, as
//! offsets y(x) with total tangent length X:
//!
//! | code | type | y'' | X |
//! |------|------|-----|---|
//! | 257 | Sine Half-Wave Diminishing Tangent | (1 − cos(πx/X)) / 2R | L − c L³/R², c = (1/3 − 3/2π²)/8 |
//! | 258 | Cubic (JP) | x / RX | X + X³/40R² = L |
//! | 260 | Cubic Parabola | x / RL | arc length to X is L |
//!
//! The X rules are fitted from one L / R pair (S232: within 3e-5 at the
//! end); a second pair would confirm them. Along these three, `s` is taken
//! proportional to the true arc length (only the ends are proven).
//! S232's "NSW Cubic Parabola" alignment is a copy of the Bi-Quadratic one
//! whose type was changed in the settings afterwards: it kept code 262 and
//! its shape, so its own code is not known yet.

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiralType {
    Clothoid,
    SineHalfWave,
    JapaneseCubic,
    Bloss,
    CubicParabola,
    Sinusoidal,
    Biquadratic,
}

impl SpiralType {
    pub const ALL: [SpiralType; 7] = [
        Self::Clothoid,
        Self::SineHalfWave,
        Self::JapaneseCubic,
        Self::Bloss,
        Self::CubicParabola,
        Self::Sinusoidal,
        Self::Biquadratic,
    ];

    /// The type stored as `code` (S232); `None` for codes not sampled.
    pub fn from_code(code: i32) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.code() == code)
    }

    pub fn code(self) -> i32 {
        match self {
            Self::Clothoid => 256,
            Self::SineHalfWave => 257,
            Self::JapaneseCubic => 258,
            Self::Bloss => 259,
            Self::CubicParabola => 260,
            Self::Sinusoidal => 261,
            Self::Biquadratic => 262,
        }
    }

    /// Civil 3D's name for the type.
    pub fn name(self) -> &'static str {
        match self {
            Self::Clothoid => "Clothoid",
            Self::SineHalfWave => "Sine Half-Wave Diminishing Tangent",
            Self::JapaneseCubic => "Cubic (JP)",
            Self::Bloss => "Bloss",
            Self::CubicParabola => "Cubic Parabola",
            Self::Sinusoidal => "Sinusoidal Curves",
            Self::Biquadratic => "Bi-Quadratic",
        }
    }

    /// Types defined along the arc length (curvature k0 + (k1 − k0) f(t)).
    pub fn by_arc_length(self) -> bool {
        matches!(
            self,
            Self::Clothoid | Self::Bloss | Self::Sinusoidal | Self::Biquadratic
        )
    }

    /// ∫₀ᵗ f for the arc-length types.
    fn rise_integral(self, t: f64) -> f64 {
        match self {
            Self::Bloss => t.powi(3) - t.powi(4) / 2.0,
            Self::Sinusoidal => t * t / 2.0 + ((2.0 * PI * t).cos() - 1.0) / (4.0 * PI * PI),
            Self::Biquadratic if t <= 0.5 => 2.0 * t.powi(3) / 3.0,
            Self::Biquadratic => t - 0.5 + 2.0 * (1.0 - t).powi(3) / 3.0,
            _ => t * t / 2.0,
        }
    }
}

/// Local point and heading at distance `s` along a spiral of type `ty`,
/// length `l`, curvature `k0` → `k1` (magnitudes, 0 on a line), turning
/// left from the origin along +x: `(x, y, heading)`. `None` when the type
/// has no shape for these curvatures (the tangent-defined types need one
/// straight end).
pub fn local_point(ty: SpiralType, l: f64, k0: f64, k1: f64, s: f64) -> Option<(f64, f64, f64)> {
    if ty.by_arc_length() {
        let heading = |u: f64| k0 * u + (k1 - k0) * l * ty.rise_integral(u / l);
        let (x, y) = simpson(s, |u| {
            let a = heading(u);
            (a.cos(), a.sin())
        });
        return Some((x, y, heading(s)));
    }
    // Tangent-defined: from the straight end, k = 1/R at the curved one.
    let (inward, k) = match (k0 == 0.0, k1 == 0.0) {
        (true, false) => (true, k1),
        (false, true) => (false, k0),
        _ => return None,
    };
    let c = TangentCurve::new(ty, l, k)?;
    let total = c.arc(c.x_end);
    if inward {
        let x = c.x_at_arc(s / l * total);
        return Some((x, c.y(x), c.slope(x).atan()));
    }
    // Outward: the canonical curve walked back from its curved end.
    let (xe, ye, he) = (c.x_end, c.y(c.x_end), c.slope(c.x_end).atan());
    let x = c.x_at_arc((l - s) / l * total);
    let (dx, dy) = (x - xe, c.y(x) - ye);
    // Rotate so the reversed heading at the curved end is +x.
    let a = -(he + PI);
    let (ca, sa) = (a.cos(), a.sin());
    let (px, py) = (dx * ca - dy * sa, dx * sa + dy * ca);
    // Walked backwards, the left-turning curve turns right: mirror it so
    // the result turns left like the others.
    Some((px, -py, he - c.slope(x).atan()))
}

/// Simpson's rule for ∫₀ˢ of a 2-vector.
fn simpson(s: f64, f: impl Fn(f64) -> (f64, f64)) -> (f64, f64) {
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
        let (a, b) = f(i as f64 * h);
        x += w * a;
        y += w * b;
    }
    (x * h / 3.0, y * h / 3.0)
}

/// A tangent-defined spiral y(x), 0 ≤ x ≤ `x_end`, turning left.
struct TangentCurve {
    ty: SpiralType,
    k: f64,
    l: f64,
    x_end: f64,
}

impl TangentCurve {
    fn new(ty: SpiralType, l: f64, k: f64) -> Option<Self> {
        let mut c = TangentCurve { ty, k, l, x_end: l };
        c.x_end = match ty {
            SpiralType::SineHalfWave => {
                let coef = (1.0 / 3.0 - 1.5 / (PI * PI)) / 8.0;
                l - coef * l.powi(3) * k * k
            }
            SpiralType::JapaneseCubic => {
                let mut x = l;
                for _ in 0..100 {
                    x = l - k * k * x.powi(3) / 40.0;
                }
                x
            }
            SpiralType::CubicParabola => {
                // Newton on arc(x) = L.
                let mut x = l;
                for _ in 0..50 {
                    let step = (c.arc(x) - l) / c.slope(x).hypot(1.0);
                    x -= step;
                    if step.abs() < 1e-13 * l {
                        break;
                    }
                }
                x
            }
            _ => return None,
        };
        (c.x_end.is_finite() && c.x_end > 0.0).then_some(c)
    }

    fn slope(&self, x: f64) -> f64 {
        let (k, xe) = (self.k, self.x_end);
        match self.ty {
            SpiralType::SineHalfWave => k / 2.0 * (x - xe / PI * (PI * x / xe).sin()),
            SpiralType::JapaneseCubic => k * x * x / (2.0 * xe),
            _ => k * x * x / (2.0 * self.l),
        }
    }

    fn y(&self, x: f64) -> f64 {
        let (k, xe) = (self.k, self.x_end);
        match self.ty {
            SpiralType::SineHalfWave => {
                let w = xe / PI;
                k / 2.0 * (x * x / 2.0 + w * w * ((x / w).cos() - 1.0))
            }
            SpiralType::JapaneseCubic => k * x.powi(3) / (6.0 * xe),
            _ => k * x.powi(3) / (6.0 * self.l),
        }
    }

    fn arc(&self, x: f64) -> f64 {
        simpson(x, |u| (self.slope(u).hypot(1.0), 0.0)).0
    }

    fn x_at_arc(&self, s: f64) -> f64 {
        let mut x = s.min(self.x_end);
        for _ in 0..50 {
            let step = (self.arc(x) - s) / self.slope(x).hypot(1.0);
            x -= step;
            if step.abs() < 1e-12 * self.l.max(1.0) {
                break;
            }
        }
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for t in SpiralType::ALL {
            assert_eq!(SpiralType::from_code(t.code()), Some(t));
        }
        assert_eq!(SpiralType::from_code(263), None);
    }

    #[test]
    fn arc_length_types_turn_by_half_the_end_curvature() {
        // Every rise f here is symmetric, so the total turn is k L / 2.
        for t in SpiralType::ALL.into_iter().filter(|t| t.by_arc_length()) {
            let (_, _, h) = local_point(t, 200.0, 0.0, 1.0 / 200.0, 200.0).unwrap();
            assert!((h - 0.5).abs() < 1e-12, "{t:?} {h}");
        }
    }

    #[test]
    fn tangent_types_start_and_end() {
        for t in [
            SpiralType::SineHalfWave,
            SpiralType::JapaneseCubic,
            SpiralType::CubicParabola,
        ] {
            let (x, y, h) = local_point(t, 200.0, 0.0, 0.005, 0.0).unwrap();
            assert!(x.abs() < 1e-9 && y.abs() < 1e-9 && h.abs() < 1e-12);
            let (x, y, _) = local_point(t, 200.0, 0.005, 0.0, 0.0).unwrap();
            assert!(x.abs() < 1e-9 && y.abs() < 1e-9, "{t:?} {x} {y}");
            assert!(local_point(t, 200.0, 0.004, 0.005, 10.0).is_none());
        }
    }
}
