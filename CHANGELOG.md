# Changelog

All notable changes to this project are recorded here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed
- README rewritten around the current alignment status (read/write
  table, unexplained fields, milestones); open questions refreshed.
- Open questions moved to `docs/mysteries.md` as MIST1–MIST14.
- MIST6 solved: the post-type value is the implied PI display option.
- MIST7 experiment: `examples/exp_header.rs` and samples S125–S130
  (S24 with the element header changed, plus a control). Round 1
  results: top byte must be 4, third byte 2 or 3, low word 8 accepted and
  9 out of range; Civil 3D writes back what it read (S131, S132). Round 2
  (S133–S138, re-save S139): the low word must be exactly 8, the top byte
  exactly 4, and the third byte needs bit 0x02; bit 0x01 is kept on save
  and bit 0x04 dropped.
- First acadrust 0.6.7-written drawing confirmed in Civil 3D (S125 → S131).
- MIST7: the header value also occurs in other Civil 3D classes (S119);
  likely a generic sub-object tag, not text.
- Priorities: Civil 3D 2027 support is low priority; `hexview` and
  `viewer` are frozen (CLAUDE.md).
- MIST7: drawing units and Civil 3D release ruled out (S123 feet, S124
  meters, both Civil 3D 2027: same element header).

### Fixed
- Records whose class number is 500–751 store the object type in one byte
  (`01` + byte): own handles are now read and written after either form,
  and `set_object_type` writes the form for the number (it now returns
  new bytes). Using a Civil 3D 2027 drawing (S123/S124) as the donor
  failed before.
- `hexview` and `viewer` build again: both now handle spiral segments
  (`Segment::Spiral`, added with the line-spiral groups), shown as a
  field in hexview and drawn as a clothoid in the viewer.
- A design-speed entry with an all-zero GUID (Civil 3D 2027) no longer
  corrupts the record when speeds are rewritten: the identity's second
  copy is found through the table, and zero GUIDs get fresh ones.

### Added
- Samples S123/S124 (Civil 3D 2027, feet and meters), `docs/civil3d-2027.md`,
  `tests/civil3d_2027.rs`, `handle_refs::object_type_bits`,
  `aecc_design_speed::entry_guid_bits` and `ZERO_GUID`; MIST15 in
  `docs/mysteries.md`.
- `aecc_build::pi_display` / `set_pi_display` and `PI_DISPLAY_*`: the
  implied point-of-intersection display option (MIST6). Sample S122
  (three alignments set to 1, 2, 3) and `tests/create_samples.rs`
  `pi_display_field`.
- Curve constraint codes 0 (centre + radius), 1 (centre + through point),
  8 (from entity end through point), 11 (through point with direction),
  21 (from entity end by radius and length) and 28 (fillet through a
  point) decoded and resolved; whole circles; the curve parameter block
  reads `n1` BDs and the closed bit. `ArcDef::describe`. Sample S117 (one
  curve per Civil 3D curve method) and `tests/curve_methods.rs`.
- Open centre curves (closed bit 0) placed between the free or floating
  lines next to them, a half circle when nothing precedes them; direction
  bit 0 confirmed counter-clockwise. Sample S118.
- Line-spiral group records (code 162): `Element::LineSpiral`,
  `Segment::Spiral` (clothoid), resolved before a centre curve; a group
  resolves into one entry per part (`ResolvedElement::part`). Editing
  and speeds refuse alignments with groups for now. Sample S119.
- Sample S120: 21 empty alignments named after the spiral layout methods.
- `create_alignment` with no PIs copies an empty template alignment (S113
  `EMPTY`), giving its speed-table entry a fresh GUID;
  `aecc_design_speed::empty_alignment_table`. Sample S116: 14 empty
  alignments named after the curve methods.
- Line kinds 5 (floating line by length) and 9 (free line between two
  curves) decoded and resolved; free lines trim the three-point curves
  next to them (S113).
- `set_design_speeds` writes speed intervals on curves (`BL 2, BL 1`,
  point on the arc), matching Civil 3D's S113 interval. Samples S113,
  S114.
- `docs/element-methods.md`: Civil 3D's layout methods mapped to element
  constraints, with the sample plan for lines, curves and spirals (S113–S117).
- `aecc_entity::entity_common`: the ODA common entity data at the start
  of section 2 (colour, linetype scale and flags, lineweight,
  invisibility, extension dictionary flag); checked on all 249 alignment
  records.
- `aecc_topology::graph`: nodes and edges of site containers (several
  alignments, crossings as shared vertices, face IDs per segment). Sample
  S112 (alignment crossing two others in a site, two parcels).
- Sample S111: Civil 3D accepts and keeps the rebuilt companions (S110).
- Topology edges (`aecc_topology::companion_graph`): one edge per
  alignment, vertices in path order with bulges for arcs.
- `aecc_topology::rebuild_companion` / `refresh_companion` (D-040):
  companions are rewritten from the geometry; `create_alignment` and the
  geometry-editing examples use it (S107's companion was stale). Sample
  S108.
- `aecc_topology::node_list`: the topology node list of companions and
  site containers (one node per element end point); test against S25–S106.
- S107's LandXML export; topology findings: one node per element end
  point in companions and site containers (docs, "Sites").
- Samples S104–S106 (two alignments in a site, moved between sites and
  back to siteless) and S107 (siteless alignment created next to sites);
  site membership tests.
- Sites (O7, S101–S103): `aecc_site::sites` and `membership` read the
  drawing's sites and which site an alignment belongs to; example `sites`.
  `create_alignment` skips donor alignments that are in a site.
- Arc constraint 13 (`ArcDef::FloatingEnd`, S99): radius and end point,
  starting tangent to the previous element.
- Alignments with detached pieces decode (S100): only the chain from the
  first element is the alignment (`DecodedElements::elements`); the rest is
  kept in `DecodedElements::detached`, unstationed and preserved by edits,
  as Civil 3D does (D-037).
- `remove_element` removes the last tangent after a free curve (S99) and a
  tangent between two free curves (S100, the rest becomes a detached
  piece); both match Civil 3D bit for bit. Samples S99, S100.
- Alignment style components named by view (Plan 0–7, Model 8–15,
  Section 16; `aecc_style::component_name`, `component_index`);
  `style_components` takes `plan:curve`-style names and groups the flags.
- Sample S98: Civil 3D accepted all four element-edited alignments of S97
  (speeds editable); alignment style component 1 is the curve line.
- Element edits in place (`aecc_element_edit`, D-036): `remove_element`
  (first, last, or a free curve between tangents) and `add_free_curve`
  (at a PI between two tangents), keeping the other records; graphics,
  start station, first-element IDs and design-speed intervals rebuilt.
  Example `edit_elements`; sample S97. Reproduces S95's and S96's element
  lists bit for bit.
- `aecc_station::first_element_fields` / `set_first_element`: the path's
  first element ID, stored twice in section 2 (S94).
- `aecc_build::replace_graphics`, `graphics_bits`, `clear_element_id_list`;
  `handle_refs::keep_refs`.
- Samples S95 (curve between tangents deleted) and S96 (free curve added
  back) with tests; element-edit rules in `docs/alignment-object.md`.
- `tools/rebuild_from_report.py`: rebuild drawing B's record from A's
  record and a hexview diff report.
- Sample S94 (first tangent deleted in Civil 3D) with tests
  `tests/edit_samples.rs` (floating curve, kept IDs and stations, `p1` as
  the curve's design speed).
- Floating curves (arc constraint code 15, S94): start point and radius,
  ending tangent to the next element (`ArcDef::Floating`,
  `curve_into_line`). Example `scan_elements` decodes a raw record dump.
- Sample S92: base alignment for the O6 add/remove-elements series
  (`docs/alignment-object.md`, *Next samples*).
- Sample S93 (Civil 3D re-save of S92) and its findings: element records
  kept, derived points recomputed with Civil 3D's rounding.
- hexview: comparison of two drawings (**Compare with…**, `--compare`,
  `--diff a.dwg b.dwg`). Records are matched by handle; changed records
  get a bit-level diff of the data stream (regions as bit ranges with
  their bits and the decoded field before them), a separate handle-stream
  comparison and the decoded fields whose values changed. Filters by kind
  and by class/name/handle; a Copy button puts the report on the
  clipboard (D-034).

### Changed
- hexview diff reports print B's bits in full (A's still cut at 512) and
  list up to 200 regions per record, so A plus the report rebuilds B.
- acadrust 0.6.5 → 0.6.7 (library, viewer, hexview). 0.6.7 reads every
  `AEC*` class, all `AECC_*` included, into its Architecture types and
  re-encodes them on save; `raw::open` now restores them to raw records
  (`raw::restore_raw_records`) so they are still read, edited and saved
  byte for byte (D-033). All 91 samples: identical raw records on read and
  after a save round trip, identical `alignments` output, same test
  results (docs/acadrust-0.6.7.md).

### Fixed
- Speed intervals store the containing element's ID, before-ID and
  after-ID (S115); civil3drust wrote `1, 0, next` for every tangent, which
  Civil 3D corrected on save. Sample S115.
- The EED/graphics fields are located from the object header
  (`aecc_entity::eed_bit`) instead of a fixed bit 42, which is wrong for
  short class numbers (S68).
- `rebuild_companion` keeps the template's edge flags and tail: writing
  S49's tail into a three-reference companion made Civil 3D discard it
  (S108 → S109). Handle stream trimmed. Sample S110.
- The node-list reader accepted a misaligned start in S49 (IDs now
  bounded).
- `create_alignment` accepts any Civil 3D alignment as template: the
  last handle reference may be a design check set (kept if the target has
  it, else nulled and the criteria box cleared); templates with floating
  elements, edited element lists or a nonzero start station get their
  element list replaced in place (`aecc_element_edit::replace_elements`)
  and start at 0+00, speeds moved with the start.
- Arc records are read by their constraint code instead of `p1`, which is
  the design speed at the curve (a free arc with a speed was misread).
- The start station is read before the first record in storage order
  (wrong once the first element is deleted).
- The `3DSOLID` in `PRESSUREPARTERRORBLOCK` (S1, S5, S6, S11, S12, S15,
  S89) is read and kept on save; those drawings no longer need failsafe
  mode (acadrust 0.6.7).

### Added
- `raw::set_record` (replace a raw record's bytes and handle-stream size),
  `examples/fingerprint` (raw-record hashes before/after a save, for
  comparing acadrust versions), `tests/acadrust_compat.rs`.
- `hexview/`: egui hex viewer for DWG files. Offset / hex / decoded
  columns; every object record from the handle map is outlined and
  labelled (blue = acadrust, red = AECC via civil3drust, amber = opaque,
  gray = unmapped). Detail windows show frame offsets, CRC check,
  civil3drust's decoded fields with bit offsets, handle references and
  acadrust's `Debug` view. Bottom bar decodes the bytes at the cursor.
  Records are mapped in the decompressed `AcDb:AcDbObjects` section
  (and in the raw file for R2000). `--dump` for a text listing, `--goto`
  to open at an offset or handle. D-032.
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
