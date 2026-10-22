# Similar projects

Other open-source projects working on Civil 3D data, LandXML or civil design geometry. Listed for reuse,
cross-checking and credit. Code ported from any of them keeps its original license header and is recorded in
`THIRD_PARTY.md`.

---

## CivilCraft

- **Repo:** https://github.com/getcadcraft/civilcraft (reviewed at commit `b57d9a6`, 2026-10-20)
- **What it is:** Browser/desktop civil & site design app (JavaScript, ES modules) with TIN surfaces, alignments,
  profiles, corridors, grading, pipe networks, and LandXML in/out. Part of the "CadCraft" app family.
- **License:** Apache-2.0, © 2026 The Data Design Group, Inc. Vendored libraries are all permissive (ISC, MIT,
  Unlicense, OFL). Their PROVENANCE.md says the code was written clean-room from the ODA DWG spec 5.4.1, the
  Autodesk DXF reference and the LandXML 1.2 schema, with GPL tools (LibreDWG, libdxfrw) used only as black-box test
  oracles.
- **Trademarks:** "CadCraft", "CivilCraft" and the other app names are trademarks. We use them only to identify the
  source, never as a name or brand.

### Relevance to civil3drust

**It does not decode Civil 3D (AECC) objects.** It has no AECC handling at all. Custom/AEC classes are kept as raw
bits and written back with remapped handles (`app/dwg/passthrough.js`). Its DWG "surface" code covers ACIS `SURFACE`
entities, not TIN surfaces. Its civil objects live only in its own project format and in LandXML. The AECC-in-DWG
problem therefore remains ours to solve.

| Module | Size | Use for civil3drust |
|---|---|---|
| `app/landxml.js` | 478 lines | **Main candidate to port.** LandXML 1.2 read/write: CgPoints, TIN surfaces (breaklines, boundary), alignments (Line/Curve CoordGeom), profiles (PVI/ParaCurve), pipe networks, plan features, coordinate system. |
| `app/dwg/passthrough.js` | 294 lines | Approach worth copying: preserve undecoded objects (e.g. partially decoded AECC) with handle remapping on save. Check acadrust first. |
| `app/alignment.js` | 160 lines | PI-based alignments with circular curves, station/offset, no spirals. Use as a source of test vectors (overlapping curves, short tangents). |
| `app/profile.js` | 92 lines | Parabolic vertical curves, K values, high/low points. Use as a source of test vectors. |
| `app/tin.js` | 755 lines | Constrained Delaunay with breaklines. Don't port; use the Rust `spade` crate instead. |
| `app/hydro.js`, `app/sewer.js` | — | Rational method, Manning, HGL/EGL, sanitary flows. Not for this crate; relevant to network_core. |
| "Spec gap" comments in `app/dwg/*` | — | Method to adopt for AECC reverse engineering: fixtures with a distinctive value in every field, bits located one at a time, results cross-checked against ODA File Converter output. |

### How we use it

- **Linking:** not practical (JS vs Rust, and their package is `"private": true`).
- **Porting** (e.g. LandXML I/O): ported files keep `SPDX-License-Identifier: Apache-2.0` plus the upstream path and
  commit, the copyright line and a "modified" note. `THIRD_PARTY.md` carries their NOTICE and the Apache-2.0 text.
  The crate license becomes `MPL-2.0 AND Apache-2.0`. Apache-2.0 → MPL-2.0 is a compatible direction.
- **Cross-checking:** a Node oracle (`tests/oracle/`) pinned to the same commit runs their `readLandXml` /
  `writeLandXml` on our fixtures and diffs the results against civil3drust. Black-box use creates no license
  obligations; behaviour checked this way gets credited in the docs.
- **Clean room:** their code is permissive and itself clean-room, and the README's rules only exclude code with
  incompatible licenses, so porting their LandXML/geometry code is allowed. It is **not** a source for DWG/AECC format
  findings: those still come only from the sources listed in the README (it has no AECC knowledge anyway). We still
  don't read GPL sources (e.g. LibreDWG).
- **Tracking upstream:** to pick up their fixes, diff their files from the recorded commit.

### First step when LandXML work starts

LandXML is currently low priority (alignment creation in DWG comes first). When it starts: port the alignment and profile parts of `landxml.js` (this matches the tangent-only alignment milestone) and add the
Node oracle test.
