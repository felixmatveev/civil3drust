# civil3drust

Cleanroom implementation of Civil 3D (AECC) DWG objects in Rust, built on
[acadrust](https://github.com/hakanaktt/acadrust).

> **Status:** horizontal alignments — read, edit and create in existing
> Civil 3D drawings; spirals being decoded. Not affiliated with or endorsed
> by Autodesk. "Civil 3D" and "AutoCAD" are trademarks of Autodesk, Inc.,
> used here only to describe file compatibility.

## Goal

Read and write Civil 3D objects (alignments, profiles, surfaces, pipe
networks, …) in DWG files from Rust, and exchange them with LandXML, without
needing Civil 3D installed.

**Current focus: creating and editing alignments in existing Civil 3D
drawings.** LandXML export is low priority for now.

## Where things stand

Every result below was checked by opening civil3drust's output in Civil
3D 2024 (AUDIT, re-save, comparison with Civil 3D's own records). The
sample drawings (S1–S120) and the evidence for each finding are in
[`docs/alignment-object.md`](docs/alignment-object.md); the record map is
[`docs/alignment-map.md`](docs/alignment-map.md).

### Alignments: what civil3drust can do

| | read | write |
|---|---|---|
| Lines: fixed, floating (end point, length), free (between curves) | ✓ | fixed lines |
| Curves: every Civil 3D curve method (three point, centre, floating, free fillet by radius or through point), whole circles | ✓ | free curves (fillets) |
| Spirals: line-with-spiral group | ✓ (first sample) | — |
| Other spiral groups (SCS, compound, reverse …) | decoding (S120 → S121) | — |
| Create an alignment (PIs + fillet radii, or empty) | | ✓ — style, label set, site-less, type, registry and companion |
| Remove elements, add a free curve, change a length / radius / PIs | | ✓ |
| Name, description, style, display components | ✓ | ✓ |
| Start station, design speeds (incl. on curves), design criteria flags | ✓ | ✓ |
| Alignment type (centerline, miscellaneous) | ✓ | ✓ |
| Implied PI display option | ✓ | ✓ (`set_pi_display`) |
| Sites and parcels | ✓ membership | read-only for now (D-038) |
| Detached pieces after deleting an interior element | ✓ | kept as placeholders (D-037) |

### Still unexplained

The open fields and behaviours are tracked as **MIST1 … MIST15** in
[`docs/mysteries.md`](docs/mysteries.md): section-2 caches (MIST1–MIST5),
element header, flag and
parameter fields (MIST7–MIST10), one speed-interval distance (MIST11),
topology flags (MIST12), a zero-GUID block (MIST13) and the half-circle
rule (MIST14) and a section-2 flag (MIST15). MIST6 (implied PI display) is
solved. None of them stops Civil 3D from opening our files.

### Milestones reached

- 2026-10-20 — tangent-only alignment edited (segment length) and
  accepted by Civil 3D with zero AUDIT errors.
- Free-curve radius edits, alignments generated from PIs and fillets,
  new alignments in existing drawings, design speeds, start stations,
  criteria flags, styles and types — each accepted by Civil 3D.
- Element add/remove matching Civil 3D's own edits (S94–S100), companion
  topology rebuilt and accepted (S111).
- All 14 curve layout methods decoded (S117, S118); first spiral group
  (S119).

civil3drust uses acadrust 0.6.7, which reads every `AEC*` class (Civil 3D's
included) into its Architecture types; `raw::open` turns those back into
raw records so they are saved byte for byte (D-033,
[`docs/acadrust-0.6.7.md`](docs/acadrust-0.6.7.md)).

## Modules

| Module | Purpose |
|---|---|
| `raw`, `handle_refs`, `string_stream` | Raw AECC records (opened with `raw::open`), bit helpers, handle and string streams. |
| `aecc_entity` | Common entity data (layer colour, linetype, …) at the start of the record. |
| `aecc_alignment`, `aecc_elements` | Element records (lines, curves, spiral groups) decoded and resolved into geometry with stationing. |
| `aecc_build`, `aecc_create` | Writing element lists and graphics; creating alignments from a template alignment. |
| `aecc_element_edit`, `aecc_arc_edit`, `aecc_alignment_edit` | Adding and removing elements, radius and length edits. |
| `aecc_station`, `aecc_design_speed`, `aecc_criteria` | Start station, design speeds, design criteria. |
| `aecc_style`, `aecc_alignment_refs` | Style references and display components; name, description, layer. |
| `aecc_registry`, `aecc_topology`, `aecc_site` | Companion registry, topology graph, sites (read-only). |
| `proxy_graphics` | The display cache embedded in AECC entity records. |
| `alignment`, `inventory` | Format-independent alignment geometry; AECC class inventory. |

The command-line examples (`alignments`, `create_alignment`,
`edit_elements`, `set_pis`, `set_speeds`, `set_style`, …) are listed with
their arguments in [`CLAUDE.md`](CLAUDE.md). For example:

```
cargo run --release --example alignments -- drawing.dwg
cargo run --release --example create_alignment -- donor.dwg target.dwg out.dwg "New" "0,0 100,0,50 100,100"
cargo run --release --example edit_elements -- in.dwg out.dwg "New" remove 1
```

Tools: [`hexview/`](hexview/README.md) (hex view and record-by-record bit
diff of two drawings) and [`viewer/`](viewer/README.md) (2D/3D viewer for
checking decoded objects against Civil 3D's graphics).

## Cleanroom rules

This project only uses information obtained legitimately. It is built from:

- **Our own sample files**: DWGs created in Civil 3D for this purpose, with
  controlled, documented changes between versions (e.g. "same alignment,
  segment 2 lengthened by 10 ft") so the byte-level differences can be
  studied.
- **Public specifications**: the LandXML schema, the Open Design Alliance's
  publicly released *Open Design Specification for .dwg files*, and
  acadrust's own source.
- **Public documentation**: Autodesk's published Civil 3D user and API docs
  (for names and meanings of properties, not implementation).
- **Observed behavior**: what Civil 3D does when it opens a file we wrote.

It is **not** built from:

- Decompiled or disassembled Autodesk binaries (Civil 3D, ObjectARX/ObjectDBX
  DLLs, RealDWG).
- ObjectARX or RealDWG SDK headers, samples, or anything under an Autodesk
  license or NDA.
- Code or documentation from the Open Design Alliance's Teigha/ODA SDK
  beyond its public specification.
- Copied code from other projects with incompatible licenses.

Contributors must follow the same rules. Every format finding should be
recorded in `docs/` with the sample files and steps used to reach it.

## Change records

- [`CHANGELOG.md`](CHANGELOG.md) — what changed, per version.
- [`DECISIONS.md`](DECISIONS.md) — why things are built the way they are.
- Commit messages describe each individual change.

## License

[MPL-2.0](LICENSE), the same as acadrust.
