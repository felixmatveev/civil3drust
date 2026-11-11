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
| Free Vertical Parabola (PVI based) | prefix 6, kind 0: BD 0, BD length, BL 0x1001, BL 0, 3 bits, BD −1 × 3, BL 0; API `ProfileParabolaSymmetric`, `Free`, `CurveLength` (the curve S300/S306 got — which of the free methods made it to confirm) | known, S300, S306 |
| Free Asymmetrical Parabola (PVI based) | | |
| Free Circular Curve (PVI based) | | |
| Free Vertical Curve - Best Fit | | |

The helper objects every profile owns name four of these families:
`AECC_DB_FLOAT_TANGENT_GRADELEN` (float tangent by grade and length),
`AECC_DB_FIXED_PARABOLA_HLPT`, `AECC_DB_FIXED_PARABOLA_HLPTPATSPEAK`
(fixed parabola by high/low point), `AECC_DB_FIXED_ARC_HLPT` (fixed
circular curve by high/low point).
