# Changelog

All notable changes to this project are recorded here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Site study, first batch (docs and samples only): S321–S331, the
  owner's eleven-step site series (empty site, alignment moved in,
  created in the site, crossings, a parcel, a feature line, rename,
  second site, move out, site deleted). `docs/site-object.md` maps the
  site record's tail, the site list, the parcels' tail and the whole
  container/companion record (nodes, edge slots with free slots, faces
  with loops, counters and lists), checked on all 105 container records
  of S101–S112 and S313–S331. New open questions MIST22 (sites without
  a name string) and MIST23 (parcel counters). Reader gaps found are
  listed there; the library is unchanged until the round closes.
- `examples/exp_site.rs` and experiments S332–S340: first site writers
  (rename/describe, name an unnamed site, create by cloning, delete an
  empty site, move a non-crossing alignment into a site and back),
  awaiting Civil 3D's verdict. The move-in output equals Civil 3D's S323
  apart from caches and history flags (`docs/site-object.md`, "Write
  experiments").
- S332 (alignment moved into a site by `exp_site`) accepted by
  Civil 3D 2024: no RECOVER, clean AUDIT, the alignment listed under the
  site (owner; API dump added). MIST22 narrowed: the series' sites are
  named `A` and `C` after all — names already in the drawing's name pool
  are stored as a pool ID with no string.
- S333 (alignment moved out of a site, an emptied companion refilled)
  accepted by Civil 3D 2024 and re-saved as S341: the refilled companion
  is kept; the container is rewritten without the free slot and the
  loop-less face (`docs/civil3d-rebuilds.md`).
- S342, Civil 3D's re-save of S332: the moved alignment stays in the
  site, its record equal to Civil 3D's own S323; container graph and
  site graphics rebuilt by Civil 3D. Move-in and move-out of a
  non-crossing alignment now meet the D-049 acceptance (no RECOVER,
  clean AUDIT, re-save reads back to the same model).
- S334 accepted by Civil 3D; its re-save S343 shows that a container's
  edge entries carry their own ID (a rewrite drops freed entries and
  keeps the IDs): layout in `docs/site-object.md` and `exp_site`
  corrected.
- New field: an alignment record holds the ID of its edge in its
  companion or site container (133 bits after MIST1/MIST2; it matches
  in 573 sample alignments). Found through S335, where Civil 3D's AUDIT reported
  one error because the field was stale (re-save S344: the alignment
  added to the container twice). `exp_site` now writes it; corrected
  experiment S345 (`docs/site-object.md`, `docs/civil3d-rejects.md`).
- S336–S340 accepted by Civil 3D (owner): rename with description, a
  literal name over a pooled one, deleting the only site, creating a
  site from an empty one and from one with members. Re-saves S346–S348:
  Civil 3D draws the missing site graphics, re-keys written sites and
  keeps the site list in key order (key mod 8, then key), which
  `exp_site` now writes.

### Changed
- `viewer/` and `hexview/` moved to `archive/` (D-075), unmaintained.
  The workbench took over what they did: hexview's window is its
  **File (hex)** tab (sections, record overlay and list, inspector,
  detail windows, Compare with… and the copyable report; `in file` from
  the hex pane jumps there), and the command line gained `--dump`,
  `--diff` (hexview), `--objects` (viewer `--dump`), the viewer's `--svg`
  options and `--compare other.dwg`. `--dump`/`--objects` no longer panic
  on a closed pipe.

### Fixed
- The `C3DDUMP` outputs of S306–S312 were never committed (`*.api.txt`
  was ignored everywhere), so `every_entity_matches_civil3d_api` failed
  on a fresh clone; `.gitignore` now keeps `samples/*.api.txt`.
- `tools/c3d_api_dump`: `Commands.cs` did not build against Civil 3D 2024
  (CS0104: `DBObject` exists in both `Autodesk.AutoCAD.DatabaseServices`
  and `Autodesk.Civil.DatabaseServices`); an alias picks AutoCAD's. Built
  by the owner with .NET SDK 8.0.428.

### Changed
- `tools/c3d_api_dump`: two commands, `C3DDUMP` and `C3DDUMPCONFIG`
  (replacing `C3DDUMP`'s fixed sections and `C3DDUMPALL`). The
  configuration (classes: `control`/`all`/patterns, getters, display
  representations, echo) lasts the session and is saved to
  `%APPDATA%\civil3drust\c3ddump.cfg`. Output: alignments in the detailed
  form, then `== Objects` in handle order (D-077).
- Working rule: library changes in batches after a sample round
  (CLAUDE.md).

### Added
- `docs/briefs/site-study.md`: hand-off brief for the site study
  (knowns, code entry points, samples to request, rules, done criteria).
- Workbench draws feature lines decoded, as 3D polylines (arcs from
  bulges), instead of their bounding-box proxy graphics;
  `aecc_feature_line::bulge_arc` and `sample_3d`.
- S316 `C3DDUMP`: `GetPoints` per point type matches the decoded
  vertices (test `points_match_civil3d_api_s316`; two dependent
  elevations differ by 2e-10, recomputed by Civil 3D on open). The
  add-in also calls `Get…(int)` methods per index.
- S316/S319 `C3DDUMPALL` outputs: feature line counts, site and
  elevations agree with the decoded vertices (test
  `counts_match_civil3d_api_s316_s319`); `AECC_PARCEL` is the API's
  `Site`; the site's `felix` string is the user name, not a description.
  `C3DDUMPALL control` leaves out display representations
  (`AECC_DISP_REP_*`).
- `C3DDUMPALL` asks for classes (`control` default, `all`, or DXF name
  patterns); both commands leave out per-object bookkeeping properties.
  S312's new dumps: profiles have companions in the alignment registry
  (the first profile shares its alignment's), docs/profile-object.md.
- `tools/c3d_api_dump`: `C3DDUMP` now also dumps sites, feature lines
  and every other `AECC_*` entity (simple properties plus each `Get…()`
  and `Get…(enum)` result, e.g. a feature line's points per point type);
  new `C3DDUMPALL` writes every Civil 3D object in the database (owner
  and simple properties) to `<drawing>.api-all.txt`. Not built here (no
  Civil 3D); the owner builds it. The profile test's dump parser ends an
  entity at any new section.
- `docs/open-blocks.md`: the unread blocks of alignments, profiles,
  feature lines and the objects around them, with a proposed order for a
  typed skeleton. MIST21 parked (a Civil 3D copy-paste problem, owner).
- Feature lines (S313–S318, docs/feature-line-object.md):
  `aecc_feature_line` reads name, description, site container and the
  vertices (x, y, elevation, arc bulges, elevation points) from the site
  topology's edge. `aecc_topology::graph` reads the list after each edge
  (`Edge::after`, non-empty with an elevation point). Experiment
  `exp_site_name` (S318) for the pasted copy that Civil 3D will not edit
  (MIST21). S319/S320 (owner's re-save and explode-and-rebuild) rule
  out the site name and the null references.
- FUTURE.md FRT-3: station equations with profiles (owner).
- Workbench: profiles drawn in 3D over their alignments (sampled every
  unit and at entity boundaries; x, y from the alignment, z from the
  profile) with a vertical exaggeration control (Z ×, `--z-scale`).
  Library: `Segment::point_at`, `DecodedElements::point_at_station`,
  `aecc_profile::{elevation_at, sample_3d, profile_name}`.
- Profile Design Criteria settings (S312, owner): three bits before the
  first element record, the criteria file and Minimum K Table strings,
  and a pointer to the profile design check set.
  `aecc_profile::profile_criteria`, `first_element_bit`.
- Profile Locking settings (S310, owner): anchor to alignment geometry
  points or start, modify or delete affected entities, notify, and the
  list of entities waiting to be reported (S302) — kept in the profile's
  `AECC_NOTIFICATIONINFOS` object. `aecc_profile::profile_locking`.
  S311: after an alignment edit, the "delete affected entities" profile
  lost its entities; the others kept theirs.
- Profile curves by grade and by radius (S309, owner, with `C3DDUMP`):
  fixed parabola by two points and a grade at either end (kind 12,
  grades in percent), floating parabola by a point and its grade (kind
  34), parameter 0x2002 = radius. Every S309 entity agrees with the API
  to 1e-6; the circular high/low point curve has no record in the file.
- Profile entities of the fixed, floating and free methods (S308, owner,
  with `C3DDUMP`): fixed parabola by three points or two points and K,
  floating parabola by a point and K, floating tangent to a point (also
  what "grade, length" stores), free tangent between two curves.
  `aecc_profile::segments` resolves every entity's start and end; all
  entities of S306–S308 agree with Civil 3D's API to 1e-6.
- Profile vertical curves of every free type (S307, owner, with
  `C3DDUMP`): symmetric parabola by length or K (prefix 6, kinds 0, 2),
  circular by length, radius or through point (prefix 4, kinds 0, 9, 7),
  asymmetric parabola (prefix 8, kind 10). `aecc_profile::CurveDef`,
  `CurveShape`; `layout` places them and reproduces Civil 3D's BVC, EVC,
  length, K, radius and high/low point to 1e-7.
- Profiles, first look (S298–S305, owner): `aecc_profile` reads a
  layout profile's tangent and vertical-curve records (alignment-like
  element records with (station, elevation) points) and the links
  between alignment, profiles and profile views
  (`pointers_to`, `parent_alignment`); `docs/profile-object.md`; test
  `profiles`. S306: the vertical curve's BD is its length (typed 150).
  `aecc_profile::layout` resolves PVIs and symmetric parabolas (BVC,
  EVC, K, elevation along it); checked to 1e-9 against the first
  `C3DDUMP` profile output (S306).
- `C3DDUMP` also writes each alignment's profiles (every simple property,
  each PVI and entity) and profile views, read by reflection, for the
  profile samples.
- Stale experiment results (S296, S297, owner): S295 opens cleanly;
  saving without AUDIT rebuilt only one connected alignment (on its own,
  owner), AUDIT (clean)
  rebuilt every offset and dynamic connected alignment exactly as Civil
  3D's own move (S290). A writer editing an input should rebuild them
  (civil3d-rebuilds.md). Test `s296_s297_stale_connected_rebuilt_by_audit`.
- Stale experiment S295 (civil3drust): S287 with only NORTH-SOUTH moved
  20' east, its offsets and the connected alignments left stale, to see
  whether Civil 3D rebuilds connected alignments on open or on AUDIT.
  S292's Spiral-curve-spiral disappeared in Civil 3D (owner).
- Connected alignments after their inputs or parameters change (S290–
  S294, owner): moved input (rebuilt), input start station (block
  stations follow), input curve reaching the connection (a fixed curve
  replaces the first tangent; the spiral – curve – spiral is left
  without elements), offset and overlaps edited, AUDIT. A free curve
  between a three-point curve and a line now decodes (S292's fillet).
  Tests `s290_…` to `s294_…`.
- Connected Reverse Spiral-Spiral-Curve-Spiral-Spiral (S289, owner):
  decodes as the centerline group 2569; curve group types 2–7 and the
  curve-group JSON's length names mapped to the layout group's. Test
  `s289_reverse_spiral_spiral_curve_spiral_spiral`; S289 in
  `encode_roundtrip`.
- Connected alignments with reverse spirals (S288, owner): their JSON
  layout groups 2562 (reverse spiral – spiral – curve – spiral) and 2566
  (spiral – curve – spiral – reverse spiral) are solved with the pieces of
  2561 and 2565, matching the alignment graphics; the name is read after
  the curve-group JSON when there is no user string. Test
  `s288_reverse_spiral_groups`; S288 in `encode_roundtrip`.
- Connected alignment converted to static (S287, owner): the two BLs
  after the type go 1, 1 → 2, 2 (`aecc_offset::connected_head`); the
  associations stay; the last tangent is renumbered; section 2 takes the
  Alignment Properties form. Test `s287_converted_to_static`.
- Three Centered Arc in connected alignments (S286, owner's dialog
  values): the prefix-22 record of kind 1 reads and re-encodes bit for
  bit (`WideningDef::ThreeCenteredArc`: radius 1, length 1, radius 2,
  length 2, radius 3, length 3) and is placed between its tangents
  (`aecc_widening::three_centered`, arc 2 taking the rest of the turn),
  so all four S286 alignments decode. S283–S286 added to the
  `encode_roundtrip` test.
- Connected alignment curve groups (S286, owner): Spiral-Curve-Spiral
  below and above 180° decode; the block's curve group type code (2
  fillet, 3 three centered arc, 4 spiral-curve-spiral) and the < 180°
  bit located; the Three Centered Arc record (prefix 22) documented, not
  decoded.
- Connected alignment with a circular fillet between centerlines (S285,
  owner): a free arc between two tangents stored already trimmed (they
  don't meet at the PI) now decodes, the PI taken where the tangents'
  lines cross. The connected block's per-connection head and the
  curve-group part after the connections partly read (MIST20).
- Connected (curb return, type 3) alignments, first look (S282, S283,
  owner): elements decode; `aecc_alignment_refs::connected_inputs` (the
  two inputs after the check set); the objects Civil 3D adds (a curb
  return action body, three chained dependencies); the block after the
  type partly read (MIST20). `offset_parent` now answers only for
  offset alignments. Test `connected_alignments`. S284 (radius changed):
  each connection holds four regions in the offset region form; the
  curve and spiral values live only in the elements.
- Widening added, edited, deleted and doubled (S276–S281, owner): all
  decode, stations match Civil 3D's to 1e-6; deleting a widening
  restores the offset's elements with their old IDs; widenings add only
  parent anchors. Test `s276_s281_widening_steps`; the graphics check
  skips closed frame polylines.
- The rest of the offset block: `OffsetInfo::element_map` (offset
  elements per parent element), `anchors` (the parent's station anchors
  for the range and every transition end, by GUID) and the three BLs
  after them; checked on every offset sample (test
  `offset_element_map_and_anchors`). acadrust 0.6.7 re-encodes the
  associative networks and dictionaries unchanged (S260 round trip);
  Civil 3D accepts the result and keeps the offset associative (S274,
  re-saved S275).
- Experiment S268 (a stale offset) with the owner's re-saves S272 (no
  AUDIT: the stale geometry kept) and S273 (AUDIT: rebuilt): offset
  elements must be written from the parent. civil3d-rebuilds.md row.
- Offset lock mode and superelevation type (S269–S271, owner):
  `OffsetInfo::lock_mode` (1 Geometry Locking, 2 Station Locking); with
  station locking both ends are anchored as stations (kind 6). Cant
  shows in section 2 (a BD 0 → 4.92125, 1.5 m), not in the offset block.
- Parent edited and one lock cleared (S266, S267, owner): Civil 3D saves
  a dynamic offset rebuilt after its parent changes; the first lock bit
  is the start's; anchor kinds 0 (locked start), 90 (locked end), 6 (a
  station). Experiment S268 prepared (a stale offset). Test
  `s266_s267_parent_edit_and_locks`.
- Offset parameters edited (S263–S265, owner): `OffsetInfo` gains the
  start/end lock bits; the block head's first value is the update mode
  (1 dynamic, 2 static; static also locks the elements). The station
  range is anchored by two parent intervals whose GUIDs the offset
  holds. Test `s263_s265_offset_parameters`.
- What creating and deleting an offset alignment adds and removes
  (S259–S261, owner): the association objects (action, body, two
  dependencies and bodies), the model-space and top associative networks
  with their `ACAD_ASSOCNETWORK` dictionaries, the reactors on parent and
  offset; Civil 3D removes all of them on delete, keeping the companion.
  Documented in alignment-object.md; test `s259_s261_offset_objects`.
  S262: the parent deleted instead — the same association objects go,
  the offset stays with head `2, 2, 2`, no reactor, a null parent slot.
- `workbench/` (D-073): DWG research app replacing `viewer/` and
  `hexview/` over time. Renders the drawing (viewer code, scene rebuilt
  from the document after each change); one selection drives a hex pane
  (the record, the records it references and that reference it, current
  bytes with changed bits in red, field overlay, values at any bit), a
  RON pane (Model: decoded alignment, Apply runs the equivalent commands;
  Raw: record bytes, Apply replaces them), a command form generated from
  the registry with handle/point picking, and a console. Undo/redo, Save
  As with a read-back comparison of every raw record, optional external
  `c3dr` for commands from a newer library build. Headless `--script`,
  `--related`, `--svg`. Edits run on a copy under `catch_unwind`.
- Command registry (`commands`, D-074): every edit as a named command
  with an argument spec — `set_length`, `set_radius`, `set_pis`, `curve`,
  `remove`, `set_name`, `set_start`, `set_speeds`, `set_style`,
  `style_component`, `equations`, `masking`, `create`, `delete` — plus
  reports (`alignments`, `elements`, `styles`, `refs`, `help`).
  `run_line` runs a command line on an open document; `spec()` gives the
  list as data. New `c3dr` binary runs them from the shell
  (`c3dr <command> in.dwg [out.dwg] args…`, `c3dr help`, `c3dr --spec`).
  Edits match the examples' output (identical files for `set_length`,
  `set_radius`, `set_pis`, `remove`, `equations`, `masking`, `set_style`,
  `style_component`).
- `dump`: per-object helpers for the workbench — `raw_record`,
  `raw_records_of`, `alignment_of`, `alignment_ron`/`alignment_from_ron`,
  `record_of`, `record_ron`/`record_from_ron` (bytes 16 per line inside
  the RON string), `record_bytes`.
- Design sketch for `workbench/`, a DWG research app that will replace
  `viewer/` and `hexview/` (`docs/workbench.md`, D-073): render, select,
  per-object hex and RON panes, command-line edits, raw and model RON
  apply, Save As with a reopen check. No code yet.
- Widenings on curves and the reverse-curve transition types (S258,
  owner): curve – reverse curve (prefix 36) and curve – curve – reverse
  curve (prefix 38) records; curved transitions are placed between any
  neighbours (tangent, arc, spiral) by tangent circles (D-072 updated);
  linear transitions hold one point list per part of the parent crossed.
  `offset_info` reads transition types 4 and 5. Every region's stations
  match Civil 3D's to 1e-6 and the curve graphics to 0.01; the derived
  reverse radii match the values Civil 3D shows.
- Widenings on offset alignments (S257, owner): transition records of
  two new kinds — linear (prefix 22, by points, also crossing onto a
  spiral) and curve-line-curve (prefix 34, by its two radii, placed
  between its neighbours; D-072) — decode and re-encode bit for bit.
  `offset_info` reads the regions (nominal and widened stretches with
  parent and own station ranges) and the transitions (kind, length,
  element, radii). S256's offset of an offset now decodes; all element
  stations match Civil 3D's to 1e-6 and the graphics to 0.01. The form-2
  record has no four-BD tail (that was the interval head).
- Offset alignments (S255, read-only): the offset spiral record (kind 5:
  the parent spiral, by its end point, plus the offset; D-071), drawn
  along the offset curve with its own length; tangents are ordinary
  records. `aecc_offset::offset_info` reads the offset, side, the parent's
  and the alignment's station ranges and the element IDs after the type
  (MIST19 for the rest); `aecc_alignment_refs::offset_parent` is the
  parent (a soft pointer after the check set; none once the parent is
  deleted, S256). S256: offsets over part of an alignment with an arc
  and an inward spiral decode (the spiral record stores the parent's
  straight end and the fractions followed; stations match Civil 3D's to
  1e-9); orphaned offsets read (block head `2, 2, 2`); the start-station
  check ignores bit 16 of the gap. `alignment_refs` accepts
  soft pointers after the check set (the parent; profiles and profile
  views in a production drawing), which made it fail before. `alignments` prints them, `dump` shows
  the spiral's offset. Test `offset_alignments`; S255 re-encodes bit for
  bit.
- Station anchors inside JSON spiral groups (S253): intervals name the
  group as one element (ID, links, start, end, distance), matching Civil
  3D within 1e-6; equations and masking regions can be written on JSON
  alignments. The writers place their strings from the end of the
  counted strings (Civil 3D leaves surplus ones after them). The
  section-2 form reader accepts flag values above 0xFF (S243 FD7: 0x1C1).
  S254: Civil 3D's re-save of S252 reads back the same (acceptance).
- S251 (Civil 3D re-save of S250: equations and regions read back, except a
  region that shared GUIDs with equations and was merged onto them); new
  anchors (speeds, equations, regions) never reuse a GUID the record has.
  S252: S250 made again.
- Writing station equations and masking regions (D-070):
  `aecc_station_refs::set_station_equations` and `set_masking_regions`
  (intervals, lists, derived Station Back, locks, comments and numbers;
  other intervals kept), `examples/set_station_refs`. `write_interval`
  is shared with the design speeds. Tests: S248's equations and S246's
  regions written again match Civil 3D's; equations and regions added to
  and removed from an alignment. Sample S250 for Civil 3D to check.
- Production-drawing check (local only): the interval block's opening
  BLs may be counters and its intervals may have any 16-bit kind
  (labels and other objects); both are accepted and speed rewrites keep
  the counters. Element records are read as one contiguous run (D-069),
  which fixes two production alignments. 26 of 26 decode their elements.
- `aecc_station_refs::display_station` and `station_back`: stations with
  station equations applied (S247: end 5+50.86 as Civil 3D shows it);
  `examples/alignments` prints equations and both stations. Sample S249
  (start station moved via the reference point: only the start station
  is stored; speeds removed, regions and raw stations moved). Tests
  `stations_with_equations`, `s249_start_station_moved`.
- Station equations: the bit after Station Ahead is Increase/Decrease
  (`StationEquation::increasing`, was `flag`; S248); each equation's
  strings are its comment and number. Test `s248_decreasing_equation`.
- Sample S247 (two station equations): Station Back of later equations is
  derived from the one before; the 18 leading bits are the same in both;
  equation numbers are strings. Test `s247_two_station_equations`.
- Masking regions' Mask check box (S246): the region's bit is the inverted
  Mask (`MaskRegion::masked`, was `flag`); the Lock To Start / End boxes
  are not stored as flags; civil3drust derives them on reading and then
  keeps them as model data (`lock_to_start`, `lock_to_end`,
  `MaskRegion::range`; D-068). Test `s246_masking_regions`.
- Station-anchored data (S243): `aecc_station_refs` reads the interval
  block with each interval's kind (2 design speed, 3 station equation,
  4 masking region), the station-equation list and (with
  `aecc_build::masking_regions`) the masking list. Design speeds, type
  and PI display are read on alignments with equations or masking
  regions; `set_design_speeds` writes speeds on fixed spirals and keeps
  equations, masking regions and their intervals; start-station moves
  are refused while an alignment has them. `required_string_count`
  counts 2 per interval, 1 per speed and region, 2 per equation. Sample
  S243, test `station_refs`, MIST18. S244: S243's speeds rewritten this
  way; Civil 3D accepts it and its re-save S245 reads back the same.
- MIST14 solved: an open centre curve with nothing on one side is a half
  circle from the line on the other side, either way round; a kind-3
  line into such a curve is resolved (S242, test
  `open_centre_curve_half_circle_s242`).
- `COMPATIBILITY.md`: every Civil 3D 2024 Prospector item and the full
  Settings tree (one table per feature),
  with the code that handles it and its status (supported, partial,
  read-only, referenced, passthrough, rejected).
- `docs/modularity.md` (D-066, deferred until more objects are sampled): review of how hard the crate is
  to extend piece by piece — 11 weak spots (dependency cycle across 16
  modules, no single record map, trial-scan element dispatch that drops
  unknown kinds, catch-all matches, scattered refusals and cache
  handling, mixed write strategies, grab-bag modules), a layered target
  shape and a 10-step migration order. Documentation only.
- `dump` module and `examples/dump` (D-065): a plain-text RON view of a
  DWG — model-space entities in compact form (`Line(h: "2A1", layer:
  "0", start: (…), end: (…))`), decoded alignments (elements, style,
  start station, detached pieces, JSON groups), and a count of raw
  records by class; `--raw` lists every raw record with its bytes in hex.
  `dump::from_ron` reads a dump back. Tests `dump_samples` (S24 in
  detail; every sample's dump reads back to the same value).
- Dependencies `serde`, `ron`, and acadrust's `serde` feature.

### Changed
- Companions are always written in the rebuilt form (edge `0x10`,
  vertices (10, 0, 0)), for templates in any form (D-064, MIST12
  handled; S238–S241, test `rebuilt_form_accepted_s238_s241`).
  `companion_graph` skips a chance node-list match before the real one
  (S240).
- `raw::object_name`'s doc comment describes the current name rule
  (JSON slot, user string, `<DATADIR>` anchor, section-2 form).
- Documentation revised against the current state: readings that later
  samples corrected are replaced or marked **Later:** in
  `docs/alignment-object.md` (the "identity GUID" and "station cache" are
  the design-speed interval, the "volatile field" is MIST1/MIST2, S189's
  "member needs the JSON" was the bad header, …); `docs/mysteries.md`
  rewritten with a status table and without settled hypotheses and plans
  (MIST17 corrected); `docs/alignment-map.md` updated (section 2, interval
  block, string rule, what a new alignment needs); README status, module
  list and capabilities; `element-methods.md`, `class-catalog.md`,
  `civil3d-rejects.md` (pre-write checks for group members and spiral
  codes), `civil3d-rebuilds.md`, `acadrust-0.6.7.md` brought up to date.

### Added
- MIST12: companion flags surveyed (session history: built in the
  session, loaded, from an object). `aecc_topology::rebuild_companion_with`
  and `exp_companion` write the rebuilt form (edge `0x10`, vertices
  (10, 0, 0)) for experiments S238, S239.
- MIST11 solved: S89's odd curve interval is stale (the curve's earlier
  length); samples S235–S237 follow the rule and show Civil 3D refreshing
  intervals on a grip edit (test `speed_interval_third_distance_s89_s235_s237`).
- MIST13 solved from existing samples: Civil 3D's extra block in S57 X5
  is a duplicate design-speed interval with a new GUID beside the
  zero-GUID one (test `zero_guid_interval_duplicated_s57_x5`).
- `FUTURE.md`, a record of deferred work; FRT-1: NSW Cubic Parabola
  spirals. `SpiralType::NswCubicParabola` names code 264 (S234: Civil 3D
  lists the type but draws no curve); drawn as a clothoid until sampled.
  Code 263 is what crashed S233.
- `examples/exp_spiral_code` and sample S233: spiral-type codes 263 and
  264 written into S232's spirals, for Civil 3D to judge. S233 crashes
  Civil 3D on open (owner); S234 has code 264 only.
- Spiral types (S232, MIST10 solved): `spiral_type::SpiralType` names
  the code after a spiral record's IDs (256 Clothoid … 262 Bi-Quadratic),
  and `SpiralGeometry::spiral_type` draws fixed spirals in their own
  shape (Bloss, Sinusoidal, Bi-Quadratic, Sine Half-Wave, Cubic (JP),
  Cubic Parabola), matching Civil 3D's graphics. Sample S232, test
  `spiral_types`. Meanings from Autodesk's "About Spiral Definitions"
  noted in `docs/alignment-object.md` (limits, NSW parameters).
- Sample S231: Civil 3D accepts civil3drust's deletion (S230 re-saved).
- `aecc_delete::delete_alignment` (and `empty_companion`): deletes a
  siteless alignment as Civil 3D does (S229) — alignment and label groups
  removed, companion emptied, registry entry kept. `examples/
  delete_alignment`; test `delete_alignment`; sample S230.
- Sample S229 (owner: an alignment deleted in Civil 3D) and its analysis.
- Sample S227: template for the spiral-type sample (MIST10, MIST16).
- A group's own lock: `aecc_layout::Group::locked` and
  `LayoutGroup::locked` (`ConstraintLocked` per object; the element
  headers don't show it, S226). Sample S226.
- `aecc_layout::layout` refuses a group at an end of the alignment
  (Civil 3D drops it, S224, and cannot create one).
- Spiral records placed from the element before: kinds 6 (after a line)
  and 8 (after a curve or spiral), end radius and length
  (`SPIRAL_AFTER_LINE`, `SPIRAL_AFTER_CURVE`), decoded and encoded; a
  stored group start point cuts the element before. What Civil 3D
  leaves when a long group's end entity is deleted (sample S225); joints
  match its proxy graphics.
- Sample S224 (Civil 3D drops a JSON group at an alignment end).
- `exp_group_params` modes `end` / `start` (group at an end of the
  alignment); sample S223.
- The reader prefers the JSON when a group's pieces are stored too (as
  Civil 3D does, S216 → S217): the stored pieces are set aside
  (`DecodedElements::set_aside`, `LayoutGroup::stored_pieces`) and the
  group is solved from the JSON. `decode_elements_with` with
  `DecodeOptions { stored_group_pieces: true }` reads the stored pieces
  instead (damaged-file recovery); `set_constraint_editing` uses it.
  `stored_records()` lists the records actually stored (record count and
  bit range include set-aside pieces).
- MIST9: curve parameters with criteria-based design on are the design
  speed, the minimum radius and a third criteria value (S219–S222, test
  `curve_criteria_values_s219_s222`).
- `aecc_layout_solve`: solves the sampled reverse-spiral JSON groups
  (SSCSS, SSCS, SCSS) between two fixed lines or curves. `decode_elements`
  solves JSON groups without stored pieces (D-061): pieces as parts of a
  `GroupDef::Layout` group, neighbours trimmed, stations to the end;
  matches Civil 3D's solutions (S217) within 4e-7. `DecodedElements::
  unsolved`/`UnsolvedGroup` renamed `layout_groups`/`LayoutGroup` (with
  `solved`). `replace_elements` writes `Layout` groups as JSON only.
- `decode_elements` reads a JSON group without pieces in place of the
  gap: `DecodedElements::unsolved` (`UnsolvedGroup`: ID, constraint type,
  neighbours, typed parameters, the elements after it),
  `is_complete()`; `all()` includes the elements after unsolved groups;
  `Element::set_links`. `replace_elements` can now write parameter-only
  groups directly; `exp_group_params` uses it. `alignments` prints
  unsolved groups. Sample S218; test `json_group_reads_in_place_of_the_gap`.
- Civil 3D solves spiral groups from the JSON alone (S216 → S217): no
  pieces needed, the JSON's parameters win over written pieces.
  Sample S217.
- `aecc_layout::layout` and `Group`: the layout JSON from fixed elements
  and groups given by method and typed parameters (`compose` now builds
  on it); `group_parameters`. `examples/exp_group_params`; sample S216
  (groups without pieces, for Civil 3D to solve).
- MIST1 is checked for JSON-layout alignments (S214 → S215): a mismatch
  brings the "modified unexpectedly" warning and Civil 3D rebuilds the
  group's pieces from the JSON; a zeroed MIST1 is otherwise harmless.
  Sample S215.
- Generated spiral groups with a composed JSON layout open in Civil 3D
  (S205: no RECOVER, AUDIT clean, groups re-solve on nudges; an Event
  Viewer warning "modified unexpectedly in old version" remains). S206:
  without the section-2 JSON flag the JSON is read as the name. Samples
  S211–S213 (owner's re-saves), S214 (warning bisect);
  `exp_json_bisect` ops `reencode-mist1`, `zero-mist1`, `json-radius`.
- MIST8 solved: the curve flag is set by Civil 3D for curves read from
  the file (S207–S210, test `curve_flag_set_on_load_s207_s210`).
- Tangent kind 3, "Constrained by Next (Floating)": start point only
  (`KIND_FLOATING_NEXT`, S201). Group-member spirals unlocked: type byte
  `0x09` with the `02` byte kept (S202); `header_bytes`,
  `ElementHeader::is_group_member` / `locked`, and
  `set_constraint_editing` writes Civil 3D's form (S177 → S202 element
  records bit for bit). Samples S201, S202; MIST7 owner tests recorded.
- Section-2 JSON flag found: Civil 3D records with a layout JSON are in
  the Properties form with flags `3` (or `0x43`) and `BL 0x01010034`
  (S177, S178, S180–S182); S197/S198 (plain form + JSON) were rejected.
  `examples/exp_json_form` (variants A–C); samples S199 (generated in
  S178's own JSON form), S200 (form variants).
- `aecc_layout`: the JSON layout string — `Json` parses and writes it in
  Civil 3D's format byte for byte (tabs, CRLF, `%.17g`), `compose` builds
  it from element records (fixed lines, three-point curves, the three
  reverse-spiral groups), `set_layout` writes it into the record;
  `string_stream::insert_empty`. MIST17 solved: a member spiral stores
  the group's start point (two RDs) before its parameter flag;
  `SpiralRecord::member` is now that point. `exp_chains --json` /
  `--json-tweak`; samples S197, S198; test `layout_json`.
- Generated alignments accepted by Civil 3D: S188 (43 alignments, every
  element kind) opens without RECOVER, AUDIT clean, and its re-save S195
  reads back to the same model; so does S190 → S196. Test
  `generated_resave`. S189 confirms that group-member spirals need the
  JSON layout string (MIST17; `docs/civil3d-rejects.md`);
  `replace_elements` now refuses them in an alignment without one.
- `aecc_build::to_plain_form`: records civil3drust writes drop the
  Alignment Properties header and its date-modified block (MIST3, MIST4)
  for the plain form (D-057). Samples S191–S194 (S191/S192 rejected,
  S193 accepted and re-saved plain as S194); test
  `date_modified_block_dropped_plain_form`.
- `exp_mist3`: drop the date-modified block (MIST3) of an alignment in
  the Alignment Properties form, alone, with MIST2, or by turning the
  record into the plain form (experiments S191–S193, to be opened in
  Civil 3D).
- `aecc_element_edit::required_string_count` and
  `aecc_design_speed::interval_count`; test `string_count` (Civil 3D's
  re-save S186 and the generated S188–S190 follow the rule).
  `exp_chains --plain-members`. Samples S186, S187 (Civil 3D's recovered
  re-saves of S184, S185) and S188–S190 (regenerated).
- `replace_elements` writes any decoded element (spirals, free spirals,
  every group) through `aecc_encode`, not only lines and free curves.
  `examples/exp_chains`: generate one alignment per source alignment of
  other drawings. Generated samples S184 (43 alignments, every element
  kind) and S185 (the JSON-type groups' members without a JSON string),
  for opening in Civil 3D.
- `aecc_encode::encode_element`: writes any element record (kind prefix,
  header, record) from its decoded fields. Spiral, free-spiral and group
  records now keep their parameter block (`ParamBlock`), group members
  their 128-bit block, the long groups their two flag fields. Test
  `encode_roundtrip`: all 332 element records of 22 samples (every kind)
  re-encode bit for bit.
- `string_stream::remove` (take a string out of the stream) and
  `examples/exp_json` (drop the JSON layout string: `remove` or `empty`).
  Experiment samples S179 (meant as removed, but overwritten with a plain
  round trip of S178 by mistake), S180 (emptied), their Civil 3D re-saves
  S181 and S182, and S183 (removed, checked; RECOVER, alignments emptied).
  `docs/civil3d-rejects.md`: removed and emptied JSON layout strings.
- Spiral groups (S173): `Element::Group` with `GroupDef` — spiral-curve-spiral,
  spiral-curve by length or through a point, spiral-line from a curve —
  placed from their neighbours into several parts. Sample S173.
- Reverse curves with spirals (group kind 20, codes 1 and 2): the start on
  the previous curve (and the second radius) solved; the previous curve
  is trimmed. Sample S174.
- Spiral-spiral groups (S175): between two lines (kind 8 code 8) and
  between two curves (kind 10 code 4, compound or reverse, A1/A2 stored;
  start and A solved, curves trimmed or extended).
- Spiral-line-spiral groups between two curves (S176): kind 10 code 0 (by
  spiral lengths, line solved) and code 1 (by line length, spirals with
  equal A solved), compound or reverse.
- Long spiral groups (S177): SCSCS and SCSSCS between tangents (kinds 24,
  26, from a stored TS, curve lengths solved); spirals with element header
  0x0D020008; Civil 3D's JSON layout string (`string_stream::layout_json`),
  skipped by the name and description readers. MIST17; MIST10 named
  (`SpiralType`). Confirmed by S178 (made without copy and paste): the
  three long reverse methods are stored as separate elements plus the
  JSON layout string.

- Free spirals between two lines (record kind 8): `Element::FreeSpiral`,
  `free_spiral_between`; the lines are extended or trimmed to the spiral.
  Compound spirals read as start radius, length, end radius. Sample S165.
- Element headers without the lock byte (S164): `read_element_header`
  reads both forms for tangents, arcs and spirals, and the writers follow
  the alignment's form (`encode_element_header`, `element_header_flags`;
  D-051). Tests `unlocked_element_headers_read_s164` and
  `edits_keep_the_unlocked_form_s164`.
- Samples S166–S169: element edits of the unlocked S164 and their
  Civil 3D re-saves; accepted, AUDIT clean, read back the same (test
  `unlocked_edits_survive_civil3d_resave_s166_s169`).
- Samples S170–S172: an alignment created unlocked, its Civil 3D re-save,
  and the same with one element locked by the owner. The lock byte is per
  element; bit 15 is "Lock all", the lock new elements get (locking each
  element by hand leaves it off). Test `lock_is_per_element_s170_s172`.
- New alignments are written with both Constraint Editing options off
  (no flag byte per element); `aecc_criteria::set_constraint_editing`
  switches them on existing alignments (D-052). Tests
  `created_alignment_is_unlocked`, `set_constraint_editing_matches_civil3d_s163_s164`.
- `aecc_criteria::constraint_editing`: the Constraint Editing options
  "Always perform implied tangency constraint swapping" (bit 16 before the
  section-2 end) and "Lock all parameter constraints" (bit 15).
- Fixed spiral records (kind 6): `Element::Spiral`, `SpiralRecord`
  (inward, outward, compound), resolved with linear curvature;
  `SpiralGeometry::start_radius`. Sample S162 and test
  `s162_spirals_match_civil3d_drawing`; MIST16.

### Fixed
- `set_length` / `set_pis` also refuse offset and connected alignments
  (their block after the type keeps stations and element numbers, and
  Civil 3D rebuilds a dynamic offset from its parent, S268) and
  alignments with a spiral group held by the layout JSON (Civil 3D
  follows the JSON's points; an unsolved group, S223, hid the rest of
  the chain and the edit reported a wrong length). `DecodedElements`
  carries the alignment type (`alignment_type`), read without decoding
  twice (`aecc_design_speed::locate_with`).
- Section-2 offsets are counted from the class data
  (`aecc_entity::section2_base`), so `section2_form`, the JSON flag and
  the plain-form conversion work on alignments with reactors (offsets,
  their parents, connected alignments' inputs), which have an 8-bit
  longer reactor count; the curve-group JSON of a connected alignment is
  not taken for its name (S287).
- `set_length` and `set_pis` refuse alignments that hold anything but
  fixed tangents and free arcs (`EditError::NotPiLayout`). Before, only
  three-point arcs were refused, so on a tangent followed by a fixed
  spiral (S162 FD7), a floating curve (S117 FCB) or a line constrained
  by the next element (S242 FEA) the tangent moved, the other element
  stayed, the command reported success and the written alignment no
  longer decoded (or, for the floating curve, did not change). Test
  `pi_edits_refuse_alignments_with_other_elements`.
- New alignments made from a template in the Alignment Properties form
  were named "felix" in Civil 3D: `to_plain_form` kept the form's user
  string, which the plain form reads as the name (S193, S227). It is now
  removed; `create_alignment` also clears the template's descriptions.
  The name reader follows the section-2 form (`raw::section2_form`) as
  Civil 3D does, so it shows these names as Civil 3D shows them. Sample
  S228 (the spiral-type template again).
- `set_constraint_editing` also sets the JSON layout's `ConstraintLocked`
  (2 locked, 0 unlocked) on every object; S177 → S202 reproduced byte
  for byte, and back.
- Object names of alignments whose JSON string Civil 3D emptied: the
  slot announced by the section-2 JSON flag is skipped by position
  (`raw::layout_json_flag`); the user string was read as the name
  (S224).
- `aecc_layout::compose` writes `ConstraintLocked` 0 for unlocked
  alignments (S202), 2 for locked; generated group members now get
  Civil 3D's unlocked header `08 00 02 09` (`header_bytes`), not
  `08 00 0D`, which every earlier generated group had (S185, S189,
  S197–S200). Samples S205 (regenerated), S206. The interval-less speed
  rule is limited to alignments with a JSON layout (keeps
  `civil3d_2027` passing).
- Element edits no longer rewrite design speeds stored without intervals
  in alignments with a JSON layout (the only form seen there) when the
  speeds don't change; they used to gain an interval block.
  `examples/exp_json_bisect`; samples S203–S205.
- Element edits counted 3 strings only per curve; spirals and groups need
  3 as well (S184 → S186, D-056). Generated alignments were short, and
  Civil 3D discarded 13 of S184's 43 on open. `exp_chains` no longer
  copies the source's count (D-055 superseded).
- Spirals with the group-member header (type byte `0x0D`, S177) are now
  read without the lock byte too: `aecc_alignment::read_header` returns
  flags and type byte for both forms; `set_constraint_editing` keeps the
  type byte. MIST7 survey of all samples written up.
- Re-linking a kept arc in element edits read the header as a fixed
  32-bit value and broke unlocked alignments with arcs (S171, S172).
- Fixed spiral records end with three BDs (not four) and free spirals
  with their length; the extra bits belong to the block after the
  records.

### Changed
- README and `docs/mysteries.md`: widening (offset alignments with widening
  regions) and alignment groups (siteless and per site) listed as missing; spiral status
  updated.
- `docs/element-methods.md`: sample descriptions are hints; stored records
  checked against Civil 3D's drawing are the ground truth.
- MIST15: the Constraint Editing option "Always perform implied tangency
  constraint swapping" (confirmed by S163).
- MIST7: "Lock all parameter constraints" off removes the `0x02` byte from
  every element header (S164) and clears bit 15 before the section-2 end.
- MIST15 marked SOLVED (S163). The start-station check no longer requires
  the lock bit, so S164's start station is found.
- DECISIONS.md: D-049 records the end goal (files civil3drust and Civil 3D
  read the same; model data plus required plumbing only; Civil 3D
  rebuilds its own caches) and the acceptance test; status notes added to
  the decisions it affects (D-008, D-014, D-025, D-026, D-027, D-030,
  D-039, D-040), and missing Date/Status lines filled in (D-028–D-048).
- MIST2 is now also removed in the Alignment Properties form, with its
  own marker at section-2 bits 115–124 (`0100000001` → `0011000001`);
  accepted by Civil 3D (S159, re-saved S160). D-047 updated.
- MIST3 identified as the time of the last Alignment Properties edit: a
  Windows FILETIME after `BL 0x1600101F` in the Properties form.
  Plan: drop the block (to be tested).
- MIST2 (the re-save field) is removed with its format marker set back
  to `0x1148` instead of being copied, when civil3drust creates an
  alignment or writes its geometry (D-047): `aecc_build::
  remove_resave_field`, `drop_caches`, `drop_record_caches`; the
  `set_length`, `set_radius` and `set_pis` tools apply it. Not yet for the
  Alignment Properties form (test S158). MIST2 marked partially solved.
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
  and bit 0x04 dropped. A family of `0xTT0M0008` headers found in many
  classes, unchanged between the 11.0 and 13.0 class schemas; the element
  header is also unchanged in a 2020 drawing. Gaps noted from that drawing
  (alignments that do not decode, type field not found).
- First acadrust 0.6.7-written drawing confirmed in Civil 3D (S125 → S131).
- S149/S150 (radius change, rotation in place): the radius feeds the
  MIST1 shape bits; position and direction mix in the placement bits.
- S147/S148 (move west, rotate): MIST1 splits into 38 bits that follow
  the shape only and 28 bits that follow the placement.
- S145/S146: MIST1 depends on shape (a length change rewrites it) and on
  position (a move changes bytes 0 and 5–7, keeps 1–4).
- S144 (rename): MIST1 does not depend on the name; the rename adds
  MIST4 and the 589-bit section 2 form.
- MIST1 is content-derived (S142/S143: same drawing saved minutes apart,
  identical records; unchanged since S24). MIST5 changes with MIST2's
  appearance on the first re-save (S24 → S131).
- MIST1 holds none of the values Civil 3D lists for S24's alignment;
  derived values that are never stored are recorded.
- Samples S140/S141 (S120 saved with PROXYGRAPHICS 1 and 0): MIST1/MIST2
  are not tied to proxy graphics; with PROXYGRAPHICS 0 Civil 3D writes
  placeholder graphics and nothing else in the record changes.
- MIST7: the header value also occurs in other Civil 3D classes (S119);
  likely a generic sub-object tag, not text.
- Priorities: Civil 3D 2027 support is low priority; `hexview` and
  `viewer` are frozen (CLAUDE.md).
- MIST7: drawing units and Civil 3D release ruled out (S123 feet, S124
  meters, both Civil 3D 2027: same element header).

### Changed
- MIST1 marked partially solved: a geometry cache, ignored and zeroed.
- MIST2 read as `BL 0x0D00D773, BL n`: constant first value; n is 2 for
  centerlines and 1 for Miscellaneous alignments. Experiment
  `examples/exp_mist2.rs`, samples S151–S154 (zeroed and removed):
  the first value is checked, the second is kept as stored (S156), and
  removing the field is rejected while the format marker at section-2
  bit 115 still says `0x1188` (MIST5). S155 (removed, marker back to
  `0x1148`) is accepted and Civil 3D restores both on save (S157).
- MIST1 (the 66-bit geometry key) is zeroed instead of copied when
  civil3drust creates an alignment or writes its geometry (D-046);
  `aecc_build::geometry_key_bit`, `clear_geometry_key`.

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
- `docs/civil3d-rejects.md`: every condition Civil 3D was seen to reject
  (RECOVER), crash on, report in AUDIT or quietly change, with samples
  and how the writer guards against it; ends in the pre-write check list.
- `docs/civil3d-rebuilds.md`: what Civil 3D accepts missing, zeroed or
  stale and rebuilds on save — the parts civil3drust leaves to it (D-049).
- `tools/c3d_api_dump/`: a read-only Civil 3D add-in (`C3DDUMP`) that
  writes each alignment's public API values (`CreationMode`,
  `FingerPrint`, design speeds, entity constraints, …) to a text file,
  to compare with what civil3drust decodes.
- MIST12: companion edge flag `0x20` (with vertex a = b = 1) marks an
  alignment whose geometry came from an object; kept by re-saves, cleared
  by layout-tool edits. A candidate for the API's `CreationMode`.
- Sample S161 (alignments created from a polyline and with the layout
  tools): the creation method is not stored in section 2 (not MIST4);
  layout-tool alignments store their design speed without an interval
  and with a zero GUID; the 617-bit difference matches MIST13's block;
  MIST1 matches S123's for the same geometry.
- MIST2 in the Alignment Properties form: plain removal rejected (S158);
  its marker found at section-2 bits 115–124 (`0100000001` with MIST2,
  `0011000001` without). `exp_mist2 … remove-marker` handles both forms;
  test sample S159.
- `docs/class-catalog.md`: every decoded alignment field mapped to the
  public Civil 3D API class and property it stores, or classified as
  derived, cache, serialization plumbing or open (D-048); the record seen
  as a C++ class hierarchy with typed sub-objects. MIST1 is probably
  `Entity.FingerPrint`; MIST3's `0x1600101F` + FILETIME block is shared by
  82 classes.
- `examples/tags.rs`: candidate sub-object tags (full-form BLs reused
  across classes).
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
- Sample S121 (fixed spiral): record layout documented, not yet decoded
  in code (length/radius order open).
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
