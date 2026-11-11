# Profile layout methods → element records

The entity methods of Civil 3D 2024's Profile Layout Tools (owner's
screenshots, 2026-11-11), like `element-methods.md` for alignments. Each
row is filled in from a sample and its `C3DDUMP` output.

## Tangents

| method | record | status |
|---|---|---|
| Fixed Tangent (Two points) | prefix 2, kind 0: through points (station, elevation) × 2; API `Fixed`, `PassThroughPt1AndPt2` | known, S299–S306 |
| Fixed Tangent - Best Fit | | |
| Float Tangent (Grade, length) | prefix 2, kind 4: RD end point only — the same record as "through point" (the grade and length are not kept: S308 `float tangemt grade length`, 2 % over 100, stores its end 283.635 / −35.864); API `FloatOnPrev`, `Constraint2` 534 | known, S308 |
| Float Tangent (Through point) | prefix 2, kind 4: RD end point; tangent to the element before (its tangency point solved); API `FloatOnPrev`, `PassThroughPt2` | known, S308 |
| Float Tangent - Best Fit | | |
| Free Tangent | prefix 2, kind 9: no data; touches the curves before and after (common tangent solved); API `Free`, `NoConstraints` | known, S308 |
| Solve Tangent Intersection | a command: the result is two fixed tangents sharing their point (S308 `tangent intersection`) | known, S308 |

## Vertical curves

| method | record | status |
|---|---|---|
| Fixed Vertical Curve (Three points) | prefix 6, kind 11: BD 0, three points (start, on the curve, end); the parabola through them, trimmed by dependent neighbours; API `Fixed`, `ThreePassPoints` | known, S308 |
| Fixed Vertical Curve (Two points, parameter) | prefix 6, kind 13: BD 0, start point, end point, BL 0x1000 sag / 0x1001 crest, BL 0x2000 (K), BD K; API `Fixed`, `TwoPassPointsAndParameter` | known (K), S308 |
| Fixed Vertical Curve (Entity end, through point) | prefix 6, kind 12 (as "grade at start point"): the start is the element before's end, the grade there that element's grade; API `TwoPassPointsAndGrade` | known, S309 |
| Fixed Vertical Curve (Two points, grade at start point) | prefix 6, kind 12: BD 0, start, end, BL 0, BD grade in percent (0.2 = 0.002); API `TwoPassPointsAndGrade`, `GradeAtThroughPoint1` | known, S309 |
| Fixed Vertical Curve (Two points, grade at end point) | prefix 6, kind 12: …, BL 1, BD grade in percent; API `GradeAtThroughPoint2` | known, S309 |
| Fixed Vertical Curve (Parabolic, high/low point, parameter, length) | stored as kind 13 (two points and a parameter): start, end, crest, BL 0x2002 (radius), BD 500 — Civil 3D reports `HighLowPoints` | known, S309 |
| Fixed Vertical Curve (Circular, high/low point, parameter, length) | S309's profile has **no element record** (its API reports a half circle R 50 from 850 to 950 with invalid grades, `Constraint2` 533): not readable from the file | open, S309 |
| Fixed Vertical Curve - Best Fit | | |
| Floating Vertical Curve (Parameter, through point) | prefix 6, kind 32: BD 0, end point, BL −1, BL parameter (0x2000 K, 0x2002 radius — S309), BD value; hangs from the tangent before (its start solved); API `FloatOnPrev`, `PreviousSubEntPassPtAndParameter` | known (K), S308 |
| Floating Vertical Curve (Through point, grade) | prefix 6, kind 34: BD 0, end point, BL 1, BD grade in percent there; hangs from the tangent before; API `PreviousSubEntPassPtWithGrade` | known, S309 |
| Floating Vertical Curve - Best Fit | | |
| Free Vertical Curve (Parabola) | | |
| Free Vertical Curve (Circular) | | |
| Free Vertical Parabola (PVI based) | see the curve table below | known, S300, S306, S307 |
| Free Asymmetrical Parabola (PVI based) | prefix 8, kind 10, as the asymmetric curve drawn with the tangents | known, S309 |
| Free Circular Curve (PVI based) | prefix 4, kind 9 (radius), as drawn with the tangents | known, S309 |
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

## Fixed, floating and free entities (S308)

S308 (owner, with `C3DDUMP`): one profile per method. The fixed entities
stand on their stored points; a floating one hangs from the element
before; a free one touches both neighbours. `aecc_profile::segments`
resolves every entity, and each start and end agrees with Civil 3D's to
1e-6 (test `every_entity_matches_civil3d_api`, also over S306 and S307):

- floating parabola: elevation = tangent line + a (s − s₀)², a = ±1 /
  (200 K) (sign: the side of the line the point is on), s₀ from the
  point;
- floating tangent: the station where the curve's tangent passes through
  the point;
- free tangent: the line touching both curves.

The curve code (0x1000 sag, 0x1001 crest) is the BL after a free
parabola's value too: every free parabola sampled was a crest.

S309 (owner, with `C3DDUMP`) adds the PVI-based free curves, the grade
methods and the parameter code 0x2002 (radius; a parabola's radius is
100 K). Grades are stored **in percent**. Every entity of S309 agrees
with the API to 1e-6, except the circular high/low point curve, which
has no record.

Not seen yet: the length parameter (0x2001?), the Best Fit methods,
fixed circular curves by other methods.
