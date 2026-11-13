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


## FRT-3: Station equations with profiles

**Status:** open (2026-11-11, owner).

**What civil3drust does now.** Profile entities are stored in the
alignment's stations (S299–S312, none with station equations).
`aecc_profile::sample_3d` (the workbench's 3D profile layer) and
`DecodedElements::point_at_station` treat a station as the start station
plus the distance along the alignment: station equations are not
applied.

**Needed.**
- A sample: an alignment with station equations (increasing and
  decreasing) and a layout profile across them, with `C3DDUMP`, to see
  whether profile records keep raw (distance) stations or equation
  stations, and what the API's `Station` / `RawStation` report.
- Apply the answer in `point_at_station` / `sample_3d` and in a profile
  writer.

## FRT-4: Sites — crossing members, parcels, feature lines

**Status:** open (2026-11-12, round close of the site study).

**What civil3drust does now.** `aecc_site_edit` creates, renames and
deletes empty sites and moves an alignment that crosses nothing into a
site and back (D-079). It refuses a move where the alignment would cross
or touch a member, any move into a site with parcels, alignments with
spirals, arcs when the site already has members, and deleting a site
with members.

**Needed.**
- The planar arrangement: crossing nodes (vertex flags 1072/1136),
  splitting the faces, the loop selector k, and parcel records for closed
  regions (S325, S326 are the references; MIST23 for the counters — a
  sample with the Numbering counters set to distinct values).
- Feature lines as members (S327, S329: edge flags, elevation points in
  `after`, the hard null references).
- An arc–segment crossing test, and spirals as chords or true curves in
  the container (not sampled: a spiral alignment in a site).
- Deleting a site with its members, as Civil 3D does (S331 deleted the
  feature line with the site).
- More built-in name IDs (MIST22) when a site reads as unnamed.
