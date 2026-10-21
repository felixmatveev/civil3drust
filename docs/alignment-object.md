# AECC_ALIGNMENT record

Status: **partly decoded; first milestone reached.** Tangent-only geometry
decodes from its own element records (`aecc_alignment::decode`),
independent of the display cache. A segment-length edit written by
civil3drust (S7) opened in Civil 3D 2024 with zero AUDIT errors, showed the
change, and was re-saved by Civil 3D with civil3drust's geometry unchanged
(S8, below).

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
| S5 | `100-200-300.dwg` | Alignment `Alignment - (2)` with three tangents (0,0)–(100,0)–(200,50)–(300,50); saved before Civil 3D drew it (placeholder graphics). Sample A of the 10 ft pair. |
| S6 | `100-200-30010.dwg` | Same as S5 but segment 2 is 10 ft longer (121.8034 instead of 111.8034); later PIs moved with it: (208.9443, 54.4721), (308.9443, 54.4721). Saved the same way as S5. Sample B of the pair. |
| S7 | `A_seg2plus10.dwg` (generated) | S5 with segment 2 set to 121.80339887498948 by `examples/set_length` (civil3drust). |
| S8 | `A_seg2plus10_civ3d_resaved.dwg` | S7 opened in Civil 3D 2024 (AUDIT: 0 errors; geometry correct), then saved. |
| S9 | `A_seg2plus10_civ3d_resaved_sta100.dwg` | S8 with the alignment's start station changed from 0+00 to 1+00 in Civil 3D, saved. |

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

## Controlled pair: S5 → S6 (segment 2 + 10 ft, both from Civil 3D)

`rawdiff S5 S6`: only six AECC classes differ.

| Class | What changed |
|-------|--------------|
| `AECC_ALIGNMENT` | Same size (706 bytes). Placeholder box corners (300→308.9443, 50→54.4721); tangent 2 end; tangent 3 start and end; and bits 2730–2789 (see below). Tangent 1, the cache record (it refers to tangent 1) and everything else identical. |
| `AECC_ALIGNMENT_STATION_LABEL_GROUP`, `…_MINOR_STATION_LABEL_GROUP`, `…_GEOMPOINT_LABEL_GROUP` | Only the corners of each label group's own placeholder box (label extents, e.g. 307.24, 59 → 316.56, 63.47). Depends on label text size, so not derivable from the geometry alone. |
| `AECC_FEATURE_STYLE` (one record) | Size changed; holds values 300/200/50. Not understood; treated as save-time noise for now. |
| `AECC_NETWORK_PART_CATALOG_DEF_NODE` | Values around 526–542 and −6.5 → −6.75; catalog metadata, unrelated to the alignment. |

No length, end station, or midpoint is stored: searching both records for
the old and new total length (311.80/321.80), segment-2 length
(111.80/121.80) and midpoint found nothing. Only coordinates are stored.

**Volatile field (bits 2730–2789 in S5/S6, section 2).** About 60 bits that
differ between every pair of samples compared (S1, S5, S6) with no visible
structure. Not a double, not a bit-coded long long, not any length or
coordinate. Probably an ID, hash or modification stamp. The 44-bit growth
Civil 3D wrote into S4 (bit 5463) is likely the same field. S3 opened in
Civil 3D with this field left stale, so it is not checked on load.

## Generated vs Civil 3D: S7 vs S6

`examples/set_length S5 S7 8C5C 2 121.80339887498948` patched two tangent
records and the placeholder box (the cache record is on tangent 1, which
did not change). `rawdiff S7 S6 AECC_ALIGNMENT`:

```
[8C5C] Alignment - (2): 706 bytes, 1 changed bit range(s)
    bits    2730..2789     (no plausible double)
```

So civil3drust's edit reproduces Civil 3D's own record **bit for bit**,
except the volatile field. The coordinates civil3drust computed are
identical to the bit to Civil 3D's. The whole-file comparison leaves only
the three label groups, the feature style and the catalog node, the same
records that differ between S5 and S6.

## Civil 3D re-save of the generated file: S8

S7 opened in Civil 3D 2024; AUDIT reported zero errors and the alignment
showed the new length. Saved as S8.

- `alignments S8`: same three tangents, coordinates bit-identical to S6
  (Civil 3D's own B) and S7.
- `rawdiff S7 S8 AECC_ALIGNMENT`: Civil 3D changed only two things in the
  record: 2 bits at bit 2677 (a flag; meaning unknown) and the volatile
  field, which grew from 62 to 106 bits (record 706 → 711 bytes). That is
  the same +44-bit growth seen when S3 was re-saved as S4, so the volatile
  field is variable-length. Everything after it is unchanged apart from the
  shift.
- `rawdiff S8 S6`: tangent geometry identical; many style and settings
  records differ because S6 was saved in a different Civil 3D session (save
  noise, see D-006).
- `PRESSUREPARTERRORBLOCK`: acadrust dropped its unparseable `3DSOLID`
  (7F72) when writing S7. Civil 3D did not recreate it and AUDIT did not
  flag it; the block still holds its two hatches and two polylines, and S8
  reads in acadrust without failsafe mode.

## Start station: S8 → S9

`rawdiff S8 S9 AECC_ALIGNMENT` (711 → 641 bytes):

| Change | S8 | S9 |
|--------|----|----|
| Flag area near bit 2676 | 5 bits | 39 bits |
| Volatile field | 65 bits | 163 bits |
| **Start station** (bit-coded double, BD) | bit 2995: code `10` = 0.0 (2 bits) | bit 3127: code `00` + raw double **100.0** (66 bits) |
| Station-reference cache and the label values after it (100, 100, 2.25, 60) | present (one record, tangent 1, point (0,0)) | **gone** |
| String stream | `AeccImpAlignment`, `Alignment - (2)` | `AeccImpAlignment`, **`felix`**, `Alignment - (2)` |

Tangent element records are unchanged apart from the shift.

**Location of the start station.** The BD ends exactly 41 bits before the
first tangent's header in every sample: S1, S2, S4–S9, and 24 of the 25
tangent alignments in the production drawing. Two of those read 1000.0 and
2000.0, matching their 10+00 and 20+00 stationing; the rest read 0.0. The
first 17 of the 41 bits (`0000?111000100000`, `?` varies) are identical in
all of them, so `aecc_alignment::start_station` checks them before trusting
the value. The exception is an alignment made from a feature line with 262
tangents, where the gap differs; there the station is reported as unknown.
The rest of the 41 bits is not decoded; a few bits vary between samples
(possibly counts or flags).

**Cache is derived.** Civil 3D dropped the station-reference cache record
when the stationing changed, confirming it is rebuilt data rather than part
of the design.

**Optional user string.** After the edit, the user's login name appears in
the string stream between the class name and the alignment name. A
production drawing has another user's name in the same position. Probably a
"last modified by" field. Names are therefore read as the *last* plain
string after `AeccImpAlignment` (`raw::object_name`), not the first.

**Arc elements (lead for later).** In alignments with curves, the arc
elements also start with the `0x04020008` header, but the fields after it
are laid out differently (the BL that is 0 for tangents is not 0), so the
tangent decoder rejects them and reports a broken chain. A sample with one
curve is needed.

## Open questions

1. The volatile field in section 2 (S5/S6 bits 2730–2789): variable length
   (62 bits in S5–S7, 106 after a Civil 3D re-save in S8; +44 bits also in
   S3→S4); what it holds.
9. The 2-bit flag at bit 2677 that Civil 3D changed when re-saving S7.
2. ~~Where the start station is stored~~ — answered by S9: a BD ending 41
   bits before the first tangent header. Editing it in place is only
   possible when the encoding length doesn't change (both values not 0 or
   1); otherwise the record must grow or shrink.
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

## Next samples

- One arc: S5 with a curve added at PI 2, to find the arc element record.

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
