# Alignment layout methods → element records

Civil 3D's Alignment Layout Tools offer the methods below (owner's
screenshots of the toolbar menus, Civil 3D 2024). Each method is expected
to store its own constraint: the BL after the element header for lines
(`kind`), the BL `n2` in the parameter block for curves. This table maps
methods to what the samples show; the empty rows are the sample plan.

Status: **known** = decoded from a sample; **probable** = fits a sample but the method used isn't recorded; **planned** = sample requested.

## Lines

| method | constraint | status | samples |
|---|---|---|---|
| Fixed Line (two points) | kind 0: start and end point | known | S2, S5, … |
| Fixed Line (from curve end, length) | kind 0: stored as two points (the length is not kept) | known | S113 |
| Fixed Line – Best Fit | | later | |
| Floating Line (from curve, through point) | kind 4: end point only | known | S89 |
| Floating Line (from curve end, length) | kind 5: `BD length`; starts at the previous element's end, in its end direction | known | S113 |
| Floating Line – Best Fit | | later | |
| Free Line (between two curves) | kind 9: no data; tangent to both neighbouring curves, which it trims | known | S113 |

## Curves

| method | constraint | status | samples |
|---|---|---|---|
| Fixed Curve (three point) | n2 = 2: start, mid, end points | known | S10, S89 |
| Fixed Curve (two points and direction at first point) | | planned | S117 |
| Fixed Curve (two points and direction at second point) | | planned | S117 |
| Fixed Curve (two points and radius) | | planned | S117 |
| Fixed Curve (from entity end, through point) | | planned | S117 |
| Fixed Curve (center point and radius) | | planned | S117 |
| Fixed Curve (center point, through point) | | planned | S117 |
| Fixed Curve (through point, direction at point and radius) | | planned | S117 |
| Fixed Curve – Best Fit | | later | |
| Floating Curve (from entity, radius, through point) | probably n2 = 13 (radius + end point; S113's curves, method to be confirmed) | probable | S113 |
| Floating Curve (from entity end, through point) | | planned | S117 |
| Floating Curve (from entity end, radius, length) | | planned | S117 |
| Floating Curve (from entity, through point, direction at point) | | planned | S117 |
| Floating Curve – Best Fit | | later | |
| Free Curve Fillet (between two entities, radius) | n2 = 29: radius | known | S11, S89 |
| Free Curve Fillet (between two entities, through point) | | planned | S117 |
| Free Curve – Best Fit | | later | |
| (made by Civil 3D when the tangent before a free curve is deleted) | n2 = 15: radius + start point | known | S94 |
| (made by Civil 3D when the tangent after a free curve is deleted) | n2 = 13: radius + end point | known | S99, S100 |

`p1` in the curve parameter block is the design speed at the curve (−1
until Civil 3D solves it, S94).

## Lines and curves with spirals

| method | status | samples |
|---|---|---|
| Floating Line with Spiral (from curve, through point) | planned | S118 |
| Floating Line with Spiral (from curve end, length) | planned | S118 |
| Floating Curve with Spiral (from entity end, radius, length) | planned | S118 |
| Floating Curve with Spiral (from entity, radius, through point) | planned | S118 |
| Floating Reverse Curve with Spirals (from curve, radius, through point) | planned | S118 |
| Floating Reverse Curve with Spirals (from curve, two points) | planned | S118 |
| Free Spiral-Curve-Spiral (between two entities) | planned, first | S118 |
| Free Compound Spiral-Curve-Spiral-Curve-Spiral (between two tangents) | planned | S119 |
| Free Reverse Spiral-Curve-Spiral-Spiral-Curve-Spiral (between two tangents) | planned | S119 |
| Free Reverse Spiral-Curve-Spiral-Curve (from entity, end curve) | planned | S119 |
| Free Reverse Spiral-Spiral-Curve (from curve, end entity) | planned | S119 |
| Free Reverse Spiral-Spiral-Curve-Spiral-Spiral (between two curves) | planned | S119 |

## Spirals

| method | status | samples |
|---|---|---|
| Fixed Spiral | planned | S120 |
| Free Spiral (between two entities) | planned | S120 |
| Free Compound Spiral-Spiral (between two curves) | planned | S120 |
| Free Reverse Spiral-Spiral (between two curves) | planned | S120 |
| Free Compound Spiral-Spiral (between two tangents) | planned | S120 |
| Free Compound Spiral-Line-Spiral (between two curves, spiral lengths) | planned | S120 |
| Free Reverse Spiral-Line-Spiral (between two curves, spiral lengths) | planned | S120 |
| Free Compound Spiral-Line-Spiral (between two curves, line length) | planned | S120 |
| Free Reverse Spiral-Line-Spiral (between two curves, line length) | planned | S120 |

## How the samples are made

S116 is prepared by civil3drust: S24 plus 14 empty alignments named after
the 14 curve methods (owner's highlighted list, which adds Fixed Curve
(Three point) and Free Curve Fillet (radius) as controls). The owner lays
out one curve with its method in each and saves the result as S117.



Each drawing is a copy of S24 with one alignment per method, named after
the method (e.g. `FL length`, `FC 2pt dir1`, `SCS free`). Where a method
needs something to attach to, the alignment starts with the simplest
fixed element (a two-point line or a three-point curve), so the new
element is the only unknown in the record. Spiral type (clothoid) and
default lengths as Civil 3D proposes them; the values typed are noted in
the sample's README row.
