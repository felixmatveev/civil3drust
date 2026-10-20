# civil3drust

Cleanroom implementation of Civil 3D (AECC) DWG objects in Rust, built on
[acadrust](https://github.com/hakanaktt/acadrust).

> **Status:** early skeleton. Not affiliated with or endorsed by Autodesk.
> "Civil 3D" and "AutoCAD" are trademarks of Autodesk, Inc., used here only
> to describe file compatibility.

## Goal

Read and write Civil 3D objects (alignments, profiles, surfaces, pipe
networks, …) in DWG files from Rust, and exchange them with LandXML, without
needing Civil 3D installed.

### First milestone

Tangent-only horizontal alignments (no curves, no spirals):

1. Read a Civil 3D DWG.
2. Change the length of one alignment segment.
3. Save it back.
4. Open the result in Civil 3D with no errors and the change visible.

## What's here now

| Module | Purpose |
|---|---|
| `alignment` | Format-independent tangent-alignment geometry: stationing, point at station, editing a segment length. Fully unit-tested. |
| `inventory` | Lists the AECC classes in a DWG's class table via acadrust. Starting point for studying object layouts. |

Try the inventory on a drawing:

```
cargo run --example inventory -- path/to/drawing.dwg
```

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
