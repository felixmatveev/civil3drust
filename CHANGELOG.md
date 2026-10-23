# Changelog

All notable changes to this project are recorded here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Floating tangents (O5, S89): element kind 4 with only the end point;
  `TangentRecord::kind`, `end_bit()`. Speeds on curves are written
  without an interval; speed tables without intervals are read. Test
  drawing S90, accepted by Civil 3D (S91).
- Interval block decoded (one interval per design speed: GUID, station,
  distance, length to element end, next element, element and point
  coordinates). `set_design_speeds` writes it from the geometry, adds
  speeds to alignments without any, and reproduces Civil 3D's records
  (S79, S84). Test drawing S87, verified in Civil 3D (S88). O4 done.
- O4 findings (S82): start station confirmed; alignments without design
  speeds have no interval block, which shows that the two "identity GUID"
  copies belong to the first design-speed interval and entry.
- `aecc_station` (start station read/write), `clear_design_speeds`,
  reading alignments without design speeds, `examples/set_start`,
  experiment S83.
- D-031: changing the start station shifts the design speeds instead of
  removing them (`move_start_station`, `shift_design_speeds`,
  `first_interval_station`); test drawing S85.
- Design criteria (O3, S79): `aecc_criteria` reads and sets the three
  criteria flags; `examples/exp_criteria` and experiment S80. S81: the
  generated full criteria set works in Civil 3D. `aecc_criteria::Criteria`
  names the four check boxes (criteria-based design, criteria file, check
  set, tangency check), confirmed by C1's dialog.
- Design speeds (O2, S74): `aecc_design_speed` reads and rewrites the
  design-speed table (station, speed, GUID per entry) at the start of the
  post-cache block; `examples/set_speeds`; test drawing S75.
- Creating alignments in Civil 3D drawings that never had one (S65 → S66):
  `aecc_registry` parses the empty registry, and `aecc_create::ensure_class`
  adds a missing class entry with the next positional number.
- `examples/exp_bare.rs` and S67: an alignment record alone in a plain
  AutoCAD drawing (S64), style and companion null. Civil 3D opens it and
  creates its settings, but selecting the alignment crashes until AUDIT
  assigns the Standard style: the style reference is required.
- Samples S64 (empty AutoCAD drawing) and S65 (its Civil 3D save).

- `exp_bare` optional style argument and S70: the alignment plus a copy of
  `TEST STYLE` in a plain AutoCAD drawing. Civil 3D dropped the ownerless
  style (S71/S72), so `exp_bare` now gives it an owner dictionary in the
  named object dictionary (S73). Civil 3D ignored that too; minimal
  drawings postponed (D-030, alignment-map §6).

### Fixed
- Interval distance is measured from the element's start, not the
  alignment's (S89).
- Start-station reader failed on alignments with Design Criteria bits set
  (S79–S82).
- `alignment_type` on records with several design speeds (S79).
- `raw::object_name` with design-criteria strings after the name; it no
  longer gives up when reading past the end of a short string stream
  (the previous commit broke `string_stream::tests::description_round_trip`).
- `set_design_speeds` now adds one (empty) comment string per design-speed
  entry; without it Civil 3D showed garbage as the comment (S76). Test
  drawing S77.
- `set_design_speeds` on Civil 3D's multi-speed tables: the identity GUID
  is read from its first copy instead of searched by value (entry GUIDs
  also occur outside the table); old entry GUIDs are reused. Sample S78.
  O2 verified in Civil 3D (S77, S78).
- `create_alignment` wrote the donor's class numbers into the new records;
  it now writes the target's (`set_object_type`), so donor and target can
  come from different templates.
- Alignment type (O1, samples S58/S59): a BL in the post-cache block after
  GUID copy 2, 1 = Centerline, 4 = Miscellaneous.
  `aecc_build::alignment_type` / `set_alignment_type`,
  `CreateOptions::alignment_type`, `create_alignment --type`. Test drawings
  S60 (generated types 1 and 4) and S61 (types 2, 3, 5).
- Samples S62/S63: Civil 3D accepted generated Centerline and Miscellaneous
  alignments (S60 → S62) and removed alignments with type values 2, 3, 5
  on RECOVER (S61 → S63).
- `docs/alignment-map.md`: region-by-region map of the alignment record and
  the records created with it — decoded / constant / varying / optional,
  and what a new alignment needs. Built by comparing every Civil 3D
  alignment record in S2–S55: outside the geometry almost everything is
  constant across both templates.
- docs/alignment-map.md §5: hypothesis that the constant blocks hold
  alignment options and element constraints at their defaults, with a
  one-option-per-sample plan (O1–O8).
- D-026: alignments are modelled as minimal parameters; caches are
  derived when writing, kept in separate writers if needed.
- `examples/exp_minimal.rs` and sample S56: six alignments, each with one
  part emptied (section-2 field, graphics, GUID, label group, all). All
  accepted by Civil 3D (S57): graphics, the 66-bit field and the label group
  are caches/optional; the GUID should be random.
- `aecc_registry` module: parse the companion registry (keys, companions,
  ID strings, sorted by ID string) and rebuild it with an entry added or
  removed; matches Civil 3D's records (S24→S25→S44→S48). D-025.
- `create_alignment` no longer needs the donor to be the target plus one
  alignment: any drawing with a Civil 3D alignment supplies the template;
  a registry entry (new key, ID string) is added to the target. Options
  `style` and `template` (`--style`, `--template`). Own handles and handle
  streams are rewritten at any byte length (`handle_refs::set_own_handle`,
  `rewrite_refs`). Samples S52 (S44 + D), S53 (S24 + two alignments):
  AUDIT 0 errors; re-saves S54/S55 (Civil 3D re-keys generated entries
  from a session counter).
- `create_alignment` takes `CreateOptions { label_group, parcel_entry }`;
  without the parcel entry no companion/registry entry is written and the
  alignment's companion references are nulled (`handle_refs::null_refs`
  rebuilds a handle stream, the first record-size change). Example flag
  `--no-parcel-entry`. Sample S30: AUDIT reports 2 errors without the
  parcel entry (S31 re-save), so it stays on by default.
- `aecc_create::create_alignment`: adds a new alignment to a drawing by
  transplanting the records of a donor drawing (target + one Civil 3D
  alignment) with new handles, a new ID string, a new name and PIs; the
  label group is optional. `handle_refs::retarget_any` retargets relative
  references too. `examples/create_alignment.rs`. Samples S26/S27
  (generated, with/without label group). D-021. Both open in Civil 3D
  with AUDIT 0 errors; re-saves S28/S29 show the label group is optional.
  D-022: no imitation of Autodesk's "TrustedDWG" marker.
- docs: "The companion object's content" — it stores points of the
  alignment's geometry; S28 shows Civil 3D accepted stale points.
- Sample S24 (`ZERO_CIVIL`): the owner's simplified, purged base drawing,
  read without failsafe mode. docs: "Base drawing: S24".
- Sample S25: S24 plus an alignment created in Civil 3D (creation sample
  A). docs: "Creating an alignment: S24 → S25" — the objects creation
  adds (alignment, companion, registry entry, station-equation label
  group), their references, and which other changes are save noise.

- `examples/swap_companions.rs` and sample S34 (experiment: two
  alignments' parcel companions swapped). D-023: parcel companion set
  aside (S32 re-audit: 0 errors). S35: Civil 3D accepts the swap and
  regenerates each companion's points from its new alignment.

- `string_stream` module: parses a record's string stream (flag, 16-bit
  size, bit-coded length + UTF-16 strings) and replaces a string of any
  length (`set_name`). `examples/set_name.rs` renames an alignment;
  `create_alignment` accepts any non-empty name. Samples S36–S38 (AUDIT
  0 errors in Civil 3D; re-saves S39–S41).
- `string_stream::set_description` / `description`: the description is
  stored twice right after the class string. `set_name --description`.
  S42 (Civil 3D) vs S43 (generated): identical alignment records.

- `aecc_build` module: rewrite an alignment's geometry with any number of
  tangents and free curves (graphics size + chunks, element count, kind
  prefixes, records, element-0 end station and cache), and give it a fresh
  identity value. `create_alignment` takes PIs of any count plus radii;
  `examples/create_alignment` accepts `x,y,r` for curves. Samples S44
  (Civil 3D two-tangent alignment), S45 (generated, equals S44's record
  apart from the identity), S46 (five PIs, two curves).

- `CreateOptions::graphics` / `--no-graphics` and `aecc_build::strip_graphics`:
  create an alignment without proxy graphics (Civil 3D regenerates them,
  S57). D-027.

### Fixed
- Created records could end with a stray zero byte after the handle
  stream (padding kept through several rewrites, S56); writers trim it
  (`handle_refs::trim_padding`) and `of_record` ignores trailing padding.
- Created alignments no longer reuse the donor alignment's identity value;
  the new value is a proper version-4 GUID (S45's had version nibble 5).
  New companion ID strings carry the '4' at position 13 like Civil 3D's.
- Alignments with free curves: each arc needs three empty strings in the
  string stream and an attribute-chunk triple in the graphics; S46 lacked
  both and didn't open. `set_geometry` now writes them
  (`string_stream::set_string_count`). S48/S49 (Civil 3D reference), S50
  (regenerated; matches S49 apart from GUID, a 62-bit field and rounding)
  opens with AUDIT 0 errors (re-save S51).
- `raw::object_name` reads the name from the string stream structure
  (class string, description twice, optional user name, name), so short
  names like `A` are found and the user name `felix` in S9/S22 is no
  longer reported as the name.
- `set_pis` / `set_tangent_length` / `create_alignment` now update the
  cached station at the end of the first element (first BD after the
  element records) when no free arc follows it. S26/S27/S30 kept the
  donor's value; AUDIT's fix in S32 showed the field. Sample S33.

### Changed
- `aecc_style`: alignment styles have 17 display components, not 16; the
  marker is now sixteen 1 bits + `01`, which also finds the last block
  (it was missed before) and the template default style of S2/S24.
- `raw::style_name` returns `None` for a style that stores no name (only
  the user-name pair) instead of the user name; `alignment_styles` lists
  it as "(no name stored)" and no longer drops unnamed styles.
- `aecc_style` module: locates the 16 display-component blocks of an
  alignment style and reads/sets each component's visible flag (component
  0 = line, from S21/S22). `examples/style_components.rs` lists flags or
  toggles one. Samples S21/S22 (line off/on) and S23 (generated; matches
  S22 except the modified time). D-020.
- docs: "Alignment style: display components"; S20 confirmed in Civil 3D
  (AUDIT 0 errors).
- `handle_refs` module: decodes a record's handle stream (code/counter
  references, relative codes resolved) and retargets an absolute reference
  in place. `RawRecord` gains `handle_bits`; `raw::class_of` names the
  object behind a handle; `raw::style_name` reads a style record's name.
- `aecc_alignment_refs` module: the alignment's layer, companion-object and
  alignment-style references; `alignment_styles` lists a drawing's styles;
  `set_alignment_style` changes an alignment's style (same-size only,
  D-019).
- `examples/refs.rs` (list handle references with target classes) and
  `examples/set_style.rs` (list styles / change an alignment's style).
- Samples S18/S19 (owner's own `NO STYLE` / `TEST STYLE` alignment styles)
  and S20 (S18 restyled by civil3drust; alignment record identical to
  S19's). docs: "Handle stream" section.
- `SIMILAR_PROJECTS.md`: related open-source projects to reuse or cross-check,
  starting with CivilCraft (Apache-2.0): what it covers (LandXML I/O,
  alignment/profile math, raw passthrough of undecoded objects; no AECC
  decoding), and how ported code is attributed. Decision D-018.
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
