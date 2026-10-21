# Changelog

All notable changes to this project are recorded here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- `viewer/`: civil3drust-viewer, a 2D plan / 3D orbit viewer (egui) for the
  Civil 3D objects in a DWG. Draws decoded alignments (path, PIs, station
  ticks and labels), the proxy graphics stored in each AECC entity (lines,
  arcs, circles, text, colors; Civil 3D placeholders hidden by default),
  and plain AutoCAD entities for context. Object list with per-class
  filtering, click-to-select details, adjustable proxy lift in 3D. Also
  `--dump` (text listing) and `--svg` (headless render) modes.
- Cargo project depending on `acadrust` 0.6.5.
- `alignment` module: tangent-only horizontal alignment with stationing,
  `point_at_station`, and `set_segment_length` (downstream PIs translate
  with the edited segment).
- `inventory` module: lists AECC classes from a DWG's class table.
- `inventory` example CLI (`cargo run --example inventory -- file.dwg`).
- README with project goal, first milestone, and cleanroom rules.
- `DECISIONS.md` design log and `docs/` folder for format findings.
- `raw` module: stored bytes of AECC records via acadrust (`records`,
  `open` with failsafe fallback) and bit-level helpers (`get_f64`,
  `set_f64`, `find_f64s`, `utf16_strings`, `changed_bit_ranges`).
- `rawdiff` example: compares AECC records between two DWGs (summary of all
  classes, or per-class detail with changed doubles; bit-level Myers
  alignment for records that changed size).
- `dumprec` example: writes the raw bytes of one class's records to files.
- `docs/alignment-object.md`: first findings on the `AECC_ALIGNMENT` record
  (section order, proxy graphics, design tangent, station-reference cache,
  Civil 3D round trip of a lengthened tangent).
- Decisions D-005 (study via acadrust raw bytes) and D-006 (controlled pairs
  both saved by Civil 3D).
- Dev-dependency `similar` 2 (MIT/Apache-2.0) for the bit-level diff.
- `raw::get_bit` and `raw::read_bl` (bit-coded long, ODA specification).
- `aecc_alignment` module: finds tangent element records
  (`tangent_records`), orders them by before/after links (`chain`), and
  decodes a tangent-only `AECC_ALIGNMENT` into an `Alignment` (`decode`).
  Start station is not decoded yet and is set to 0. Five unit tests on
  synthetic records.
- `alignments` example: lists decoded alignments with element IDs, links,
  coordinates and bit offsets.
- `docs/alignment-object.md`: sample S5 (three tangents), tangent element
  record layout and linkage, template `3DSOLID` note; decision D-007.
- `proxy_graphics` module: finds and walks the display-cache chunk stream
  (polyline/polygon vertices, circle centers, placeholder detection).
- `raw::record_data_mut`: mutable record bytes by handle.
- `aecc_alignment_edit` module: `apply` writes a changed `Alignment` back
  into the record as same-size patches (tangent element records,
  station-reference cache records, display-cache polylines/markers or
  placeholder box). Three unit tests.
- `set_length` example: change one tangent's length, save, read back and
  verify.
- `docs/alignment-object.md`: controlled pair S5/S6 (segment 2 + 10 ft),
  volatile header field, and S7 (civil3drust's edit of S5) matching
  Civil 3D's S6 bit for bit apart from that field; decision D-008.
- First milestone reached: S7 opened in Civil 3D 2024 with zero AUDIT errors
  and was re-saved (S8) with the edited geometry unchanged; findings in
  `docs/alignment-object.md` and README.

- Start station decoding: `aecc_alignment::start_station` reads the
  bit-coded double 41 bits before the first tangent header (checked against
  a fixed 17-bit prefix); `DecodedAlignment::stored_start_station` and the
  `Alignment` stationing use it. `alignments` example shows station ranges.
- `raw::strings_after_impl` and `raw::object_name`: names are the last
  plain string after `AeccImp…`, skipping the optional user-name string and
  text-like binary noise.
- `docs/alignment-object.md`: sample S9 (start station 1+00), station field,
  cache rebuilt by Civil 3D, optional user string, arc-element lead.

- `samples/`: the nine study drawings S1–S9 with a manifest
  (`samples/README.md`: how each was made, size, SHA-256).
- `CLAUDE.md`: working rules for this repository (commit straight to
  `main`, checks before commit, cleanroom and sample rules).

- `aecc_elements` module (read-only): arc element records (`ArcRecord`,
  `ArcParams`), circle through three points (`arc_through`: centre,
  radius, direction, sweep, length), `Element` enum, `element_records`,
  `decode_elements`. Three unit tests.
- `aecc_alignment`: `ELEMENT_HEADER` (alias `TANGENT_HEADER`), `Linked`
  trait, `chain` is generic over element kinds, `DecodeError::HasCurves`.
- `raw::read_bd` (bit-coded double).
- `alignments` example lists tangents and arcs (radius, direction, delta,
  centre) with stationing.
- Sample S10 (single R = 100 arc) and its findings; decision D-009.
- Free arcs (fillets) in `aecc_elements`: `ArcDef::{ThreePoint, Free}`,
  `fillet` (PC, PT, centre from a PI, two directions and a radius),
  `resolve` → `Segment`/`ResolvedElement` with tangents trimmed to
  PC/PT; `DecodedElements` now holds resolved elements. Two new tests.
- `alignments` example shows the arc kind.
- Sample S11 (S5 with a 50 ft fillet at PI 1) and its findings; decision
  D-010.
- `aecc_arc_edit` module: `set_free_arc_radius` patches a free arc's
  radius and its cached PC-station copies, checking the curve still fits
  (two unit tests). `set_radius` example.
- Samples S12 (Civil 3D, radius 50 → 75) and S13 (same edit by
  civil3drust, matching S12 apart from the volatile field, a 2/3/4 block
  and 3×10⁻¹⁴ in the PC station); decision D-011.

- Sample S14 (S13 re-saved by Civil 3D: AUDIT 0 errors, differs from
  Civil 3D's own S12 by one flag bit); `ArcRecord::flag`; decision D-012.

- `aecc_alignment_edit::set_tangent_length`: tangent-length edits on
  alignments with free arcs (fillets), updating cached PC stations and
  refusing edits that squeeze a curve (three unit tests).
  `aecc_arc_edit` station helpers shared. `set_length` example now uses it
  and prints the resolved alignment.
- Samples S15 (Civil 3D: tangent 3 +100 ft past a fillet, with Civil 3D's
  LandXML export, which matches the decoder to 12 decimals) and S16 (same
  edit by civil3drust, differing from S15 only in the volatile field and the
  arc flag bit); decision D-013.

- `aecc_alignment_edit::set_pis` (move every PI of an alignment of
  tangents and free arcs; used to generate alignments from a template of
  the same shape) and `pi_alignment`; `set_tangent_length` now builds on
  them. One new test. `set_pis` example.
- Sample S17: 0,0 → 100,0 → 100,100 with an R = 50 fillet generated from
  S11 (with an extra zero-deflection PI at 100,75); decision D-014.
  Opened in Civil 3D with zero AUDIT errors; Civil 3D's LandXML export of
  it (added to `samples/`) matches the intended geometry exactly.

- Decision D-015 (no Autodesk-provided styles in civil3drust; reference or
  copy from the user's drawing). CLAUDE.md priorities: LandXML export low,
  focus on alignment creation.
- Decision D-016 (decode styles from the owner's own minimal styles, one
  option per sample); sample plan in `docs/alignment-object.md`.

### Fixed
- Free arcs whose flag bit is set (S14) failed to decode; the arc layout is
  now chosen from the parameter block.
- Alignment names were wrong when the string stream holds a user name
  before the name (S9) or text-like noise after it.

### Known issues
- acadrust 0.6.5 cannot parse the `3DSOLID` in Civil 3D's
  `PRESSUREPARTERRORBLOCK`; failsafe reading drops it and saved copies lack
  it (Civil 3D accepted such a file without AUDIT errors).
