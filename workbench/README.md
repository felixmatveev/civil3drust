# civil3drust-workbench

A DWG research app (D-073): open a drawing, look at it, inspect any object
as hex and RON, edit it with commands or by editing its RON, save a copy
and open it in Civil 3D. It replaced `viewer/` and `hexview/` (now in
`archive/`, D-075). Ugly and unoptimised on purpose.

```
cargo run -- ../samples/S11_100-200-300_r50.dwg [--goto 8C5C] [--compare other.dwg]
cargo run -- --script in.dwg [--save out.dwg] "cmd" ...  # same edits, headless
cargo run -- --related in.dwg <handle> [depth]           # what the hex pane shows
cargo run -- --objects in.dwg                            # drawn objects (viewer --dump)
cargo run -- --dump in.dwg                               # sections and records (hexview --dump)
cargo run -- --diff a.dwg b.dwg [--show civil,opaque,unmapped,acad|all] [--filter text]
cargo run -- --changes a.dwg b.dwg [--brief] [--class pats] [--handle h,h] [--filter text]
             [--noise list] [--show kinds|all] [--api-a dump] [--api-b dump] [--out report.txt]
                                                         # records + Civil 3D API values that changed
cargo run -- --api dump|in.dwg [--class pats] [--handle h,h] [--filter text] [--out file]
cargo run -- --api-diff a b [--class pats] [--handle h,h] [--filter text] [--noise list] [--out file]
cargo run -- --api-history in.dwg [same options]        # c3dump/ next to the drawing, save by save
cargo run -- --make-noise list a b                      # two saves without a change → noise list
cargo run -- --svg out.svg in.dwg [--select <handle>] [--3d] [--no-proxy] [--no-decoded]
             [--placeholders] [--no-context] [--light] [--size WxH]
```

**What changed between two saves** (D-080, `docs/workbench.md`): with the
Civil 3D add-in's Auto on (`tools/c3d_api_dump`), every save leaves an
API dump in `c3dump\` next to the drawing. Save the drawing under a new
name per step (`SAVEAS`), then

```
civil3drust-workbench --changes STEP_01.dwg STEP_02.dwg --out changes.txt
```

writes, per object, the record bits that differ and the API values that
changed. That file is what to pass on; add `--brief` when it is long.
One drawing saved repeatedly under the same name has only its dump
history: `--api-history file.dwg`.

Dependencies are built with `opt-level = 2` in the dev profile (see
`Cargo.toml`), so the first build is slow and later ones only rebuild
civil3drust and the workbench. A faster linker (`mold`, `lld`) helps
further.

## Window

```
+--------------------------------------------------------------------------+
| Open  Save As  Reload  Undo  Redo | 2D 3D Fit | layers                    |
+-----------+----------------------------------------+---------------------+
| Objects   | [Drawing] [File (hex)]                 | Details|RON|Form|API|
| (✎ edited)|  click: select   Ctrl+click: x,y       |                     |
|           |  Alt+click: handle into command line   |                     |
+-----------+----------------------------------------+---------------------+
| Hex: selected record + records it references / that reference it        |
+--------------------------------------------------------------------------+
| Console: log + command line (help, @ = selection, ↑/↓ history)           |
+--------------------------------------------------------------------------+
```

- **Drawing / File (hex)** tabs in the middle. File (hex) is hexview's
  window: the saved file section by section (offset | hex | decoded),
  every record outlined and coloured by who decodes it (blue acadrust,
  red civil3drust, amber opaque, grey unmapped), a record list, the
  inspector bar (u8…f64 at the cursor), detail windows with acadrust's
  full view, and **Compare with…**: records of two drawings matched by
  handle, differing regions as bit ranges, a report to copy (the same as
  `--diff`). It shows the file on disk and reloads after Save As.
- **Objects**: model-space custom objects by class; "other raw records"
  lists styles, sites, companions and everything else acadrust keeps as
  raw bytes. ✎ marks records changed since the file was opened.
- **Hex**: one block per record — the selection, the records it
  references (depth 0–2) and, optionally, the records that reference it.
  Raw records show the document's current bytes (red = changed since
  opened, blue/underlined = a field starts here, grey = handle stream);
  records acadrust parses (layers, plain entities) show their bytes from
  the file on disk. Click a byte to see the values readable at that bit
  (RC, RS, RL, RD, BD, BL, the next 16 bits); step bit by bit. The field
  list jumps to a field's bit; `→` selects the record a handle points to;
  `in file` shows the record in the File (hex) tab.
- **RON**: *Model* is the decoded alignment (as in `examples/dump`);
  Apply turns your changes into commands (name, style, start station,
  moved lines → `set_pis`, arc radius → `set_radius`) and runs them as one
  undo step; Preview shows them. Anything else (element count, spirals,
  derived values) is refused with the reason. *Raw* is the record's bytes
  16 per line; Apply replaces them exactly as typed (any class).
- **Command form**: pick a command, fill its arguments (pick handles and
  points in the view), Run or copy it to the command line. The list comes
  from the library's command registry (D-074), so new commands appear
  here with no GUI work.
- **External c3dr**: point the form at another `c3dr` binary (a newer
  library build), load its command list and run commands through it: the
  drawing goes out as a temporary DWG and comes back read, no workbench
  rebuild needed.
- **Console**: every command, result and error. Commands run on a copy of
  the document under `catch_unwind`, so an error or panic changes nothing.

Save As never overwrites the open file and refuses `samples/`; after
writing it reads the file back and compares every raw record with memory.
