# AECC_ALIGNMENT record

Status: **partly decoded**. Enough is known to locate and change the geometry
of a tangent-only alignment; one change made this way has been opened,
used and re-saved by Civil 3D 2024 (see *Round trip* below).

All offsets are **bit offsets into the record bytes as acadrust 0.6.5 stores
them** (`raw::RawRecord::data`: the merged data/string/handle streams,
without the size prefix and CRC). They are specific to each sample; what
generalizes is the order and encoding of the sections, not the numbers.

## Samples

Made in Autodesk Civil 3D 2024 (build U.202, version 24.3) by the project
owner for this study. Not yet committed to `samples/`.

| ID | File | How it was made |
|----|------|-----------------|
| S1 | `TEST-ALIGN.dwg` | New drawing; Insert → LandXML of a single 100-unit tangent from (0,0) to (100,0) (hand-written LandXML 1.2); saved. |
| S2 | `TEST-LINE1.dwg` | New drawing (drawing units: meters); a 2-vertex polyline (0,0)–(100,0) brought in from DXF; *Create Alignment from Objects*; saved. |
| S3 | `TEST-LINE1_L150.dwg` | S2 with the tangent changed to 150 by the prototype editor (same-size patch of four doubles, written back through acadrust). |
| S4 | `TEST-LINE1_L150_civil.dwg` | S3 opened in Civil 3D, checked, saved. |

Tools: `examples/inventory.rs`, `examples/rawdiff.rs`,
`examples/dumprec.rs`.

## Record layout

Observed order of sections (S1, S2, S4):

| # | Section | Encoding | S2 (handle D90, 955 bytes) | S1 (handle 8C45, 508 bytes) |
|---|---------|----------|----------------------------|-----------------------------|
| 0 | Record prefix | not studied | bits 0–63 | bits 0–63 |
| 1 | Proxy graphics (display cache) | raw LE, see below | bits 64–5247 (648 bytes, 26 chunks) | bits 64–2559 (312 bytes, 3 chunks) |
| 2 | Entity common data and alignment fields | DWG bit codes; mostly undecoded | 5248–5731 | 2560–3043 |
| 3 | Design tangent | 4 raw doubles: start X, start Y, end X, end Y | **5732** | **3044** |
| 4 | Station-reference cache | per record: 4 raw doubles (tangent start/end) + 2 raw doubles (a point) | **6229**: tangent (100,0)→(0,0), point (100,0) | none |
| 5 | String stream | UTF-16LE | `AeccImpAlignment`, name `Alignment - (1)` | `AeccImpAlignment`, `TEST LINE` |
| 6 | Handle stream | not studied | | |

### 1. Proxy graphics

The display cache that AutoCAD draws when Civil 3D is not present, in the
proxy-entity graphics format described in the public ODA *Open Design
Specification for .dwg files*: `[i32 total bytes][i32 chunk count]`, then
chunks of `[i32 chunk bytes][i32 opcode][body]` (chunk bytes include the
8-byte header). Walking the chunks lands exactly on the declared total in
every sample.

Opcodes seen and their use in alignments:

| Opcode | Primitive | Use |
|--------|-----------|-----|
| 2 | circle: center, radius, normal | start/end markers (radius from the style; 1.27 in S2, a 0.05" marker in a metric drawing) |
| 6 | polyline: count + 3D points | centerline, one chunk per tangent; also the arrow shaft |
| 7 | polygon: count + 3D points | direction arrowhead at the alignment midpoint |
| 14, 16, 18, 20 | color, layer, linetype, fill | attribute changes between primitives |
| 21, 34 | not identified | |
| 36 | unicode text | S1 only: placeholder image (see below) |

**Placeholder image (S1).** An alignment imported from LandXML and saved
without being drawn has no real graphics: a text chunk reading
`AeccDbAlignment (AeccLand130)` (text height/width 0.1) and a closed
100 × 0.4 rectangle around the tangent. The design tangent (section 3) is
still present and correct.

### 3. Design tangent

The tangent's endpoints as four raw little-endian doubles at an arbitrary
bit offset. In S2 the alignment runs (100,0)→(0,0), i.e. opposite to the
drawn polyline; Civil 3D chose the direction when the alignment was created.

### 4. Station-reference cache

In S2 one record: the tangent again, followed by the point (100,0) — the
alignment start. S1 (nothing referencing the alignment) has none. A
production drawing (not shared) showed hundreds of records on alignments
used by corridors, profiles and labels, all with points on the tangent, which
suggests one record per referencing object or station. Not confirmed.

## Round trip: S2 → S3 → S4

`rawdiff S2 S4 AECC_ALIGNMENT` (bit-level Myers alignment):

| Change | Where (S2 → S4) | Old → new |
|--------|-----------------|-----------|
| End marker circle center X | 3936 | 0 → −50 |
| Centerline polyline end X | 5056 | 0 → −50 |
| Direction arrow (polygon + shaft) | 1920–2840 | x ≈ 49.8–50.2 → 24.8–25.2 (redrawn at the new midpoint) |
| **Unknown insertion** | 5463 | 24 bits → 68 bits (record grows 44 bits; total record +6 bytes after padding) |
| Design tangent end X | 5860 → 5904 | 0 → −50 |
| Cache record tangent end X | 6357 → 6401 | 0 → −50 (cache point (100,0) unchanged) |

So Civil 3D accepted the four patched doubles as the alignment's geometry,
regenerated the arrow, and wrote 44 extra bits in section 2 that the patched
file (S3) did not have. S3 itself opened without errors, so those bits are
not required for loading.

**Side effect of saving.** Opening and saving in Civil 3D also rewrote about
150 unrelated AECC records (styles, settings nodes, label styles) — see
`rawdiff S2 S4` summary. Controlled sample pairs must therefore both be saved
by Civil 3D in the same way, or style/settings noise will hide the real
differences.

## Open questions

1. The 44-bit insertion in section 2 (S4 bit 5463): what field, and why it
   appears after Civil 3D regenerates the alignment.
2. Where the start station is stored (a bit-coded double in section 2 is
   suspected; 0 encodes in 2 bits, so S1/S2 can't show it).
3. Whether lengths, end station, or extents are cached anywhere else (none
   found for single tangents: the only coordinate copies are sections 1, 3
   and 4).
4. Multi-tangent layout: expected one 4-double design record per tangent;
   to be confirmed with a purpose-made sample.

## Next sample pair

Two drawings, both created and saved in Civil 3D 2024 the same way:

- **A**: alignment from a 3-segment polyline, e.g. (0,0)–(100,0)–(200,50)–(300,50),
  start station 0+00, a profile view or station labels present.
- **B**: identical, but segment 2 is 10 ft longer (built from a polyline with
  that change, not edited in Civil 3D, to keep everything else equal).

Then `rawdiff A B` and `rawdiff A B AECC_ALIGNMENT`.

## Provenance

Every offset and value above was produced with this repository's tools on
acadrust's record bytes. An earlier exploratory pass (before this repository
existed) used LibreDWG's `dwgread` purely as a byte dumper; LibreDWG has no
knowledge of AECC classes, no LibreDWG code or documentation was used, and
all findings here were re-derived with acadrust-based tools.
