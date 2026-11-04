# Future work

Records of work that is known to be needed but deliberately left for
later. IDs are `FRT-<n>`, never renumbered; an entry stays (marked done)
once it is finished.

## FRT-1: NSW Cubic Parabola spirals

**Status:** open (2026-11-04).

**What is known.**
- Spiral-type code 264 is NSW Cubic Parabola: S234 (S232 with three
  spirals set to 264, nothing else changed) opens clean in Civil 3D 2024
  and the properties panel shows the type as NSW Cubic Parabola (owner).
- Civil 3D 2024 draws **no curve** for these spirals — possibly a Civil
  3D 2024 bug (owner). The owner cannot create an NSW spiral in the GUI
  either, and the public API's `SpiralType` has no NSW member.
- Code 263 is not a spiral type: S233 (263 and 264) crashes Civil 3D on
  open.
- Autodesk's help ("About Spiral Definitions", names and meanings only):
  a modified cubic parabola meeting New South Wales standards, defined
  by Φ (between the final radial line and the perpendicular to the
  initial tangent), R, Xc and Yc (total X and Y).
- S232's "NSW Cubic Parabola" alignment is a Bi-Quadratic spiral whose
  type was changed in the settings afterwards; it still stores 262.

**What civil3drust does now.** `SpiralType::NswCubicParabola` (264) is
named and kept as read, but has no shape: it is drawn as a clothoid
(D-063's fallback).

**Needed.**
- A drawing with a real NSW spiral, drawn by Civil 3D (a later release,
  or 2024 if a way to create one turns up), to get its proxy graphics.
- Its shape from the NSW definition, checked against those graphics as
  the other types were (test `spiral_types`).
- How Civil 3D lays it out (stored length vs Xc) and whether it has the
  Cubic Parabola's deflection limit.

## FRT-2: RON → DWG (building a drawing from a dump)

**Status:** open (2026-11-04).

**What civil3drust does now.** `examples/dump` writes DWG → RON
(D-065); `dump::from_ron` reads it back into `dump::Drawing`. Nothing
writes a DWG from it.

**Needed.**
- Decide the base: edit an existing drawing from the dump (apply changed
  entities and alignments by handle, keep everything else) or rebuild a
  whole document (acadrust's own `serde` form for tables and objects,
  plus the `--raw` records).
- Alignments from the dump through the existing writers (`set_pis`,
  `aecc_create`), not raw bytes, so edits in the RON become model edits.
- Acceptance as D-049: DWG → RON → DWG → RON gives the same RON, and
  Civil 3D opens the rebuilt drawing with a clean AUDIT.
- Model-space entities only so far; blocks, paper space, layers and
  other tables are not in the dump.

