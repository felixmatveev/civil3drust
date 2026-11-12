# Workbench: a DWG research app

Status: **built, steps 1–8** (D-073, D-074, D-075; how to use it:
[`workbench/README.md`](../workbench/README.md)). Replaces `viewer/` and `hexview/` (both frozen) when it can
do everything they do; until then all three exist.

## Purpose

One window to study and test civil3drust on real drawings:

1. open a DWG and render it (plain AutoCAD context + AECC objects);
2. select one object and see it as geometry, as hex and as RON;
3. edit it with commands or by editing its RON;
4. save a copy and open it in Civil 3D (or another CAD program) to check it.

Not a CAD program. Ugly is fine; slow with many objects is fine. Correct and
easy to extend is what matters.

## Layout

```
+-----------------------------------------------------------------------------+
| [Open] [Save As] [Reload] [Undo] [Redo]  file.dwg  AC1032  *modified*       |
+---------------+-------------------------------------------+-----------------+
| OBJECTS       | VIEW (plan / orbit)                       | RON             |
| filter: ____  |                                           |  (selected obj) |
| > AECC_ALIGN. |      selected object highlighted,         |  Alignment(     |
|   #2A3 "CL-1" |      the rest dimmed                      |   h: "2A3",     |
|   #2B0 "CL-2" |                                           |   name: ..      |
| > AECC_SITE   |      click = pick handle / point          |   elements: [.. |
| > (context)   |                                           |  [Model|Raw]    |
|               |                                           |  [Apply][Revert]|
+---------------+-------------------------------------------+-----------------+
| HEX (selected object + related records only)                                |
| -- #2A3 AECC_ALIGNMENT  (own record, 1 840 B)  ---------------------------- |
| 0000  4E 02 ...  fields underlined, changed bits since last edit in red     |
| -- #2A4 companion (referenced by #2A3) --------------------- [collapsed] -- |
| -- #1F0 style (referenced by #2A3) ------------------------- [collapsed] -- |
| -- #120 registry (references #2A3) ------------------------- [collapsed] -- |
+-----------------------------------------------------------------------------+
| > set_length CL-1 2 150.0                                                    |
| ok: segment 2 150.000 -> 150.000, 1 record changed (42 bits)                 |
+-----------------------------------------------------------------------------+
```

Every pane follows one **selection** (a handle). Selecting in the list, the
view, the hex pane or a handle link in RON selects everywhere.

## State

```rust
struct Workbench {
    path: PathBuf,
    doc: Opened,            // raw::open — the only source of truth
    scene: Scene,           // derived: rebuilt from doc after every change
    index: RecordIndex,     // derived: handle -> class, bytes, refs, back-refs
    selected: Option<u64>,
    undo: Vec<Snapshot>,    // whole changed records (bytes + handle bits)
    redo: Vec<Snapshot>,
    baseline: HashMap<u64, Vec<u8>>, // record bytes as opened, for the diff
    log: Vec<String>,       // command-line history and results
}
```

One rule: **nothing edits `scene` or `index` directly.** Every change goes
`doc` → rebuild `scene` and `index` → redraw. With small test drawings the
full rebuild is instant; no incremental updates to get wrong.

`Scene::load` today opens the file and drops the document. It gets split into
`Scene::from_doc(&Opened)` so the workbench can rebuild without re-reading
the file.

## Hex pane: only what belongs to the object

An AECC object is spread over several records (the entity record, its
companions, style, registry entry, site, …) and those sit at unrelated
places in the objects section. The pane does not show the file; it shows a
list of **record blocks**:

- the selected record (expanded);
- records it references (`handle_refs::of_record`), labelled with the
  reference kind where known;
- records that reference it (back-references from `RecordIndex`);
- a depth toggle (0 = own record, 1 = default, 2 = one more hop) and a
  class filter, so a style used by 40 alignments doesn't flood the list.

Each block reuses hexview's code: record framing, field overlay
(`model::Field` with bit offsets, click a field = jump to it), handle links.
Bytes come from the record data in `doc`, not from the file, so the pane
shows **edited** bytes before saving. Changed bits against `baseline` are
coloured (`raw::changed_bit_ranges`).

File-offset view of the decompressed section (hexview's current main pane)
stays available as a secondary tab for whole-file browsing, read from the
file on disk.

## RON pane: two modes

- **Model** — the object as `dump::Alignment` (decoded geometry, name,
  style, start station). This is the readable one.
- **Raw** — the record as `dump::Record` (class, handle bits, bytes as hex).
  Ugly, but it can express *any* change, including to classes civil3drust
  doesn't decode yet.

`Apply`:

- Raw: parse, check handle and class are unchanged, `raw::set_record`.
  Always works; for experiments by hand.
- Model: parse, compare with the current decode, and translate the
  difference into existing edit calls: element list →
  `aecc_element_edit::replace_elements`, name → the set_name path, start
  station → the set_start path. Anything the translation can't express
  is refused with a message naming the field (no silent drops). This is
  the first slice of FRT-2 (RON → DWG), for one object at a time.

Parse errors show RON's span (line:col) in the pane.

## Command line

Commands mirror the existing examples' arguments minus the file names,
since the file is already open. Missing arguments are taken from the
selection and from clicks in the view (handle or point), so
`set_pis` + clicking points works as well as typing them.

```
set_length  <alignment> <seg> <len>
set_radius  <alignment> <arc id> <radius>
set_pis     <alignment> "x,y x,y ..."
curve       <alignment> <tangent id> <radius>
remove      <alignment> <element id>
set_name    <alignment> <name|-> [--description "text"]
set_start   <alignment> <station> [--clear-speeds]
set_speeds  <alignment> "station:speed ..."
create      <name> "x,y x,y,r ..." [--style ..] [--template ..]
delete      <alignment>
select      <name|#handle>
diff        [other.dwg]     # bit diff vs baseline or another file
```

Implementation: one `match` arm per command, each calling a **library**
function. Where an example still holds the logic (some `set_*` examples do
their own patching), that logic moves into `src/` first, so the example,
the workbench and the tests share one copy.

Every command runs inside `std::panic::catch_unwind` on a clone of the
records it touches; a panic or error leaves `doc` unchanged and prints the
message. A bad file can't kill the app.

## Save

- **Save As only.** Never overwrites the opened file or anything in
  `samples/`.
- After writing (`DwgWriter::write_to_file`), reopen the saved file with
  `raw::open` and compare its records to the in-memory ones; show the
  result in the log ("saved, 312 records, all match" or the list that
  doesn't). Every save is also a round-trip test.
- Warn before saving if the drawing was opened in failsafe mode (records
  acadrust dropped would be missing from the copy).

## Crate

```
workbench/
  Cargo.toml        # own [workspace], like viewer/ and hexview/
  src/main.rs       # args: [file.dwg] [--goto #handle]
  src/app.rs        # window, panes, selection
  src/scene.rs      # from viewer, split into from_doc()
  src/view.rs       # from viewer (camera, picking)
  src/hex.rs        # from hexview model.rs, per-record blocks
  src/ron_pane.rs
  src/commands.rs   # parse + dispatch to library functions
  src/state.rs      # Workbench, undo, rebuild
```

Starts as a copy of `viewer/` + `hexview/` code, not a dependency on them,
so the frozen crates stay untouched.

## Build order

1. Open, render, select; `Scene::from_doc`; object list + view
   (viewer parity minus SVG export).
2. Hex pane with per-object record blocks and refs/back-refs.
3. RON pane, read-only (Model and Raw).
4. Save As + reopen check.
5. Command line with `set_length`, `set_pis`, `set_radius`; undo.
6. Raw RON apply.
7. Model RON apply (element list, name, start station).
8. Remaining commands; then hexview parity (file-offset tab, two-file
   compare) and retire `viewer/` and `hexview/`.

Steps 1–5 cover the current testing needs: open, look, edit, save, check in
Civil 3D.

## Build time

Each library change relinks the workbench (eframe/wgpu make that link the
slow part). Mitigations, set once:

- `mold` (Linux) or `lld` as the linker;
- `[profile.dev.package."*"] opt-level = 2` and run the workbench in dev
  profile: dependencies are built optimised once, our code builds fast;
- `cargo check` while editing.

## As built (2026-11-07)

Differences from the sketch above, and what it settled:

- **Commands come from the library registry** (`src/commands.rs`,
  D-074), not from a match in the workbench. The same list runs in
  `c3dr`, the console and the command form; `help` lists it. Arguments
  are the examples' minus file names; `create` takes the donor DWG as its
  first argument. `@` in a command stands for the selected handle.
  `select` and `diff` from the sketch are not commands: selection is the
  list, the view, the hex pane's handle links and "go to handle"; the
  diff is always on (red bytes, ✎ in the list).
- **Command form** (right panel): a form generated from each command's
  argument spec, with "pick" buttons that take a handle or points from
  clicks in the view. In the console, Ctrl+click inserts `x,y` and
  Alt+click a handle. Points are free clicks (3 decimals), no snapping.
- **External c3dr**: the form can load another `c3dr`'s command list
  (`c3dr --spec`) and run commands through it on a temporary copy of the
  drawing (written, edited by `c3dr`, read back). New library commands
  are then usable without rebuilding the workbench.
- **Hex pane**: depth 1 and back-references on by default; class filter
  to hide e.g. layers. Raw records show the document's bytes; records
  acadrust parses show bytes from the file map (hexview's loader, run in
  the background after opening, and again after Save As). Reference
  kinds are labelled by their handle-reference code only
  (`Ref code N`); names for them are future work. A byte click decodes
  RC/RS/RL/RD/BD/BL and the next 16 bits at that bit; the bit can be
  stepped one at a time for unaligned fields. When a record changes size,
  the bytes between the common start and common end are marked changed.
- **RON pane**: Raw is `dump::record_ron` (bytes 16 per line in the RON
  string). Model Apply diffs against the current decode and runs
  `set_name`, `set_style`, `set_start`, `set_pis` (PIs from the moved
  lines: first start, intersections of consecutive lines, last end) and
  `set_radius` as one undo step; Preview lists them. Element count,
  kinds, IDs, spirals, groups and derived values are refused by name.
- **Undo** keeps whole-document snapshots (50 steps).
- **Headless modes** for checks without a display: `--script` (commands
  and Save As through the same state code), `--related` (the hex pane's
  record list), `--svg` (the view).

Step 8 (2026-11-10): hexview's window became the **File (hex)** tab next
to the drawing (`fileview.rs`: sections, record outlines and list,
inspector, detail windows with acadrust's view, Compare with… and the
copyable report); it follows the open file and reloads after Save As. The
hex pane's `in file` button jumps there. Its `--dump` and `--diff` and
the viewer's `--dump` (now `--objects`) and `--svg` options moved to the
workbench's command line; `--compare other.dwg` opens the comparison at
start. `viewer/` and `hexview/` are in `archive/` (D-075).

## Profiles in 3D (2026-11-11)

Profiles (`AECC_PROFILE`) are drawn in the **Decoded** layer as 3D
polylines over their alignments: the profile's entities resolved by
`aecc_profile::segments`, sampled every 1 drawing unit (1 ft in US
units) and at every entity boundary by `aecc_profile::sample_3d` —
x, y from the alignment at the station, z the elevation — with a marker
at each entity's start and end. Only the stretch where both the profile
and the alignment exist is drawn (a layout profile can run past the
alignment's ends). Station equations are not applied yet (FRT-3).

Elevations are small next to plan distances, so the toolbar's **Z ×**
(and `--z-scale <n>` for `--svg`) exaggerates every z in the 3D view.
The object list shows the profile's name, entity count, stations and
elevation range; `--objects` prints the same. Drawing profiles in their
profile views (2D) comes later, once the view's fields are mapped.

## Feature lines in 3D (2026-11-11)

Feature lines (`AECC_FEATURE_LINE`) used to show only their proxy
graphics, a bounding box drawn as a placeholder. They are now decoded
(◆): their vertices from the site's topology (`aecc_feature_line::vertices`),
drawn by `aecc_feature_line::sample_3d` as a 3D polyline — straight
between vertices, arcs from their bulges sampled every 1 unit, z linear
along each segment — with a marker at every vertex. The object list and
`--objects` show the site, the vertex, elevation point and arc counts
and the elevation range.

## Open questions

- Names for reference kinds (companion, style, label set, site,
  registry) instead of codes.
- Model RON for classes other than `AECC_ALIGNMENT`: Raw only until they
  have a decoder.
- Snapping picked points to PIs and element ends.
