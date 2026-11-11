# Profile layout methods → element records

The entity methods of Civil 3D 2024's Profile Layout Tools (owner's
screenshots, 2026-11-11), like `element-methods.md` for alignments. Each
row is filled in from a sample and its `C3DDUMP` output.

## Tangents

| method | record | status |
|---|---|---|
| Fixed Tangent (Two points) | prefix 2, kind 0: through points (station, elevation) × 2; API `Fixed`, `PassThroughPt1AndPt2` | known, S299–S306 |
| Fixed Tangent - Best Fit | | |
| Float Tangent (Grade, length) | | |
| Float Tangent (Through point) | | |
| Float Tangent - Best Fit | | |
| Free Tangent | | |
| Solve Tangent Intersection (a command, not a method?) | | |

## Vertical curves

| method | record | status |
|---|---|---|
| Fixed Vertical Curve (Three points) | | |
| Fixed Vertical Curve (Two points, parameter) | | |
| Fixed Vertical Curve (Entity end, through point) | | |
| Fixed Vertical Curve (Two points, grade at start point) | | |
| Fixed Vertical Curve (Two points, grade at end point) | | |
| Fixed Vertical Curve (Parabolic, high/low point, parameter, length) | | |
| Fixed Vertical Curve (Circular, high/low point, parameter, length) | | |
| Fixed Vertical Curve - Best Fit | | |
| Floating Vertical Curve (Parameter, through point) | | |
| Floating Vertical Curve (Through point, grade) | | |
| Floating Vertical Curve - Best Fit | | |
| Free Vertical Curve (Parabola) | | |
| Free Vertical Curve (Circular) | | |
| Free Vertical Parabola (PVI based) | see the curve table below | known, S300, S306, S307 |
| Free Asymmetrical Parabola (PVI based) | | |
| Free Circular Curve (PVI based) | | |
| Free Vertical Curve - Best Fit | | |

The helper objects every profile owns name four of these families:
`AECC_DB_FLOAT_TANGENT_GRADELEN` (float tangent by grade and length),
`AECC_DB_FIXED_PARABOLA_HLPT`, `AECC_DB_FIXED_PARABOLA_HLPTPATSPEAK`
(fixed parabola by high/low point), `AECC_DB_FIXED_ARC_HLPT` (fixed
circular curve by high/low point).

## Vertical Curve Settings (curves drawn with the tangents)

The owner's Vertical Curve Settings dialog (used when tangents are drawn
with curves), with its defaults: curve type **Parabolic**, **Circular**
or **Asymmetric**; separately for crest and sag curves:

| type | parameters (defaults) |
|---|---|
| Parabolic | Length (150) or K value (50) |
| Circular | Length (200) or Default Radius (50) |
| Asymmetric | Length1 (100), Length2 (100) |

S306's curve is a symmetric parabola by length (API `Constraint2
CurveLength`). The vertical curve record's first BD (0) and its three
bits are the likely place for the type and for length versus K / radius.

## Free vertical curves as stored (S306, S307)

S307 (owner): one profile per curve type and parameter, each tangent –
curve – tangent, drawn with the tangents; `C3DDUMP` next to it. The
kind prefix gives the curve type, the element kind its constraint (the
API's `Constraint2`); every curve is `Constraint1 Free`, placed at the
PVI between its two tangents:

| curve | prefix | kind | data after the links | API |
|---|---|---|---|---|
| symmetric parabola by length | 6 | 0 | BD 0, BD length, BL 0x1001, BL 0, 3 bits, BD −1 × 3, BL 0 | `ProfileParabolaSymmetric`, `CurveLength` |
| symmetric parabola by K | 6 | 2 | BD 0, BD K, (as above) | `ProfileParabolaSymmetric`, `KValue` |
| circular by length | 4 | 0 | BD length (horizontal) | `ProfileCircular`, `CurveLength` |
| circular by radius | 4 | 9 | BD radius | `ProfileCircular`, `Radius` |
| circular through a point | 4 | 7 | RD station, RD elevation | `ProfileCircular`, `PassThroughPt` |
| asymmetric parabola | 8 | 10 | BD length 1, BD length 2 | `ProfileParabolaAsymmetric` (`Constraint2` throws) |

Everything else is derived, and `aecc_profile::layout` reproduces
Civil 3D's values to 1e-7 (test `curve_types_match_civil3d_api`):

- parabola: length = K · 100 · |A| (A = grade out − grade in); BVC/EVC at
  PVI ∓ L/2;
- circular: an arc tangent to both grade lines; its **length is
  horizontal** (EVC − BVC station) = R · |sin θ₁ − sin θ₂| (θ = atan
  grade); the through point gives R from |PVI + R·w − P| = R;
- asymmetric: BVC = PVI − L1, EVC = PVI + L2, offset at the PVI
  A · L1 · L2 / (2 (L1 + L2)), two parabolas meeting there;
- K = horizontal length / (100 |A|) for all three; the high/low point
  where the grade is 0.

Not seen yet: the fixed and floating curve methods, the other tangent
methods.
