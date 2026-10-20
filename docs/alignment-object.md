# AECC_ALIGNMENT record

Status: **partly decoded**. Tangent-only geometry decodes from its own
element records (`aecc_alignment::decode`), independent of the display
cache. One change made by patching those values has been opened, used and
re-saved by Civil 3D 2024 (see *Round trip* below).

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
| S5 | `100-200-300.dwg` | Alignment `Alignment - (2)` with three tangents (0,0)–(100,0)–(200,50)–(300,50); saved before Civil 3D drew it (placeholder graphics). First of the planned 10 ft pair (sample A). |

Tools: `examples/inventory.rs`, `examples/rawdiff.rs`,
`examples/dumprec.rs`, `examples/alignments.rs`.

## Record layout

Observed order of sections (S1, S2, S4, S5):

| # | Section | Encoding | S2 (handle D90, 955 bytes) | S1 (handle 8C45, 508 bytes) | S5 (handle 8C5C, 706 bytes) |
|---|---------|----------|----------------------------|-----------------------------|-----------------------------|
| 0 | Record prefix | not studied | bits 0–63 | 0–63 | 0–63 |
| 1 | Proxy graphics (display cache) | raw LE, see below | 64–5247 (648 bytes, 26 chunks) | 64–2559 (312 bytes, 3 chunks) | 64–2559 (312 bytes, 3 chunks) |
| 2 | Entity common data and alignment fields | DWG bit codes; mostly undecoded | 5248–5681 | 2560–2993 | 2560–2993 |
| 3 | Tangent element records | see below | 1 record, header **5682**, coords 5732 | 1 record, header **2994**, coords 3044 | 3 records, headers **2994, 3319, 3652** |
| 4 | Station-reference cache | per record: 4 raw doubles (tangent start/end) + 2 raw doubles (a point) | **6229**: tangent (100,0)→(0,0), point (100,0) | none | **4215**: tangent 1, point (0,0) |
| 5 | String stream | UTF-16LE | `AeccImpAlignment`, name `Alignment - (1)` | `AeccImpAlignment`, `TEST LINE` | `AeccImpAlignment`, `Alignment - (2)` |
| 6 | Handle stream | not studied | | | |

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

**Placeholder image (S1, S5).** An alignment saved before Civil 3D has drawn
it has no real graphics: a text chunk reading `AeccDbAlignment (AeccLand130)`
(text height/width 0.1) and a closed rectangle around the alignment's
extents (100 × 0.4 in S1, 300 × 50 in S5). The tangent element records
(section 3) are still present and correct, so decoding must not depend on
the display cache.

### 3. Tangent element records

One record per tangent. In BL = bit-coded long and RD = raw double, as
defined in the public ODA specification:

```text
BL 0x04020008    header; identical in every record of every sample (meaning unknown)
BL 0
BL id            element ID, 1-based
BL before        ID of the previous element, 0 if none
BL after         ID of the next element, 0 if none
RD start X  RD start Y  RD end X  RD end Y
```

| Sample | Record | id, before, after | Start → end |
|--------|--------|-------------------|-------------|
| S1 | 1 | 1, 0, 0 | (0,0) → (100,0) |
| S2 | 1 | 1, 0, 0 | (100,0) → (0,0) |
| S4 | 1 | 1, 0, 0 | (100,0) → (−50,0) |
| S5 | 1 | 1, 0, 2 | (0,0) → (100,0) |
| S5 | 2 | 2, 1, 3 | (100,0) → (200,50) |
| S5 | 3 | 3, 2, 0 | (200,50) → (300,50) |

The before/after chain matches the `EntityBefore` / `EntityAfter`
properties that Autodesk's public Civil 3D API documentation describes for
alignment entities. In S5 the records follow each other, separated by the
same 11 bits (`00000000010`, not decoded). What follows the last record
differs between samples and is not decoded.

In S2 the alignment runs (100,0)→(0,0), i.e. opposite to the drawn polyline;
Civil 3D chose the direction when the alignment was created.

**Check against a production drawing** (not shared, not in the repo):
`examples/alignments` decoded all 21 of its tangent-only alignments,
including ones with 3, 7, 11, 12 and 262 tangents, with the same geometry as
an independent extraction from the display cache. Alignments containing arcs
fail cleanly (a tangent's `after` points to an element with no tangent
record, i.e. the arc uses a different record type), and an offset alignment
has no tangent element records at all.

### 4. Station-reference cache

In S2 one record: the tangent again, followed by the point (100,0) — the
alignment start. In S5 one record: tangent 1, followed by (0,0) — again the
alignment start. S1 (nothing referencing the alignment) has none. Cache
records have no element header, so the decoder never mistakes them for
tangents. A
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
4. ~~Multi-tangent layout~~ — answered by S5: one element record per
   tangent, linked by before/after IDs.
5. Meaning of the header value `0x04020008` and of the BL 0 after it
   (candidates: element type, constraint type, version).
6. The 11 bits between consecutive element records, and the data after the
   last one.
7. Arc and spiral element records: expected to share the id/before/after
   fields with a different header; needs a sample with one curve.
8. How offset alignments store their geometry (no tangent records).

## Next sample pair

Two drawings, both created and saved in Civil 3D 2024 the same way:

- **A**: alignment from a 3-segment polyline, e.g. (0,0)–(100,0)–(200,50)–(300,50),
  start station 0+00, a profile view or station labels present.
- **B**: identical, but segment 2 is 10 ft longer (built from a polyline with
  that change, not edited in Civil 3D, to keep everything else equal).

Then `rawdiff A B` and `rawdiff A B AECC_ALIGNMENT`. **A** is S5; **B** is
still needed. Save B the same way as S5 (S5 was saved before the alignment
was drawn, so B should be too, or both should be regenerated before saving).

## Template note

S1 and S5 contain a `3DSOLID` in Civil 3D's internal block
`PRESSUREPARTERRORBLOCK` (handle 7F72 in both) that acadrust 0.6.5 cannot
parse. Reading falls back to failsafe mode and the solid is dropped, so a
copy written by acadrust would lose it. S2 (a different, smaller drawing)
does not have it. A lossless round trip of drawings from this template needs
that acadrust issue fixed first.

## Provenance

Every offset and value above was produced with this repository's tools on
acadrust's record bytes. An earlier exploratory pass (before this repository
existed) used LibreDWG's `dwgread` purely as a byte dumper; LibreDWG has no
knowledge of AECC classes, no LibreDWG code or documentation was used, and
all findings here were re-derived with acadrust-based tools.
