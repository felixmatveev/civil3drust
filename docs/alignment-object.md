# AECC_ALIGNMENT record

Status: **partly decoded; first milestone reached.** Tangent-only geometry
decodes from its own element records (`aecc_alignment::decode`),
independent of the display cache. A segment-length edit written by
civil3drust (S7) opened in Civil 3D 2024 with zero AUDIT errors, showed the
change, and was re-saved by Civil 3D with civil3drust's geometry unchanged
(S8, below).

All offsets are **bit offsets into the record bytes as acadrust 0.6.5 stores
them** (the same bytes under 0.6.7 after `raw::open`, D-033) (`raw::RawRecord::data`: the merged data/string/handle streams,
without the size prefix and CRC). They are specific to each sample; what
generalizes is the order and encoding of the sections, not the numbers.

## Samples

Made in Autodesk Civil 3D 2024 (build U.202, version 24.3) by the project
owner for this study, or generated from them. All are in
[`samples/`](../samples/) as `S<n>_<name>.dwg`; see
[`samples/README.md`](../samples/README.md) for sizes and checksums.

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
| S10 | `arc_civ3d.dwg` | Alignment `Alignment - (3)`: a single arc, R = 100 ft, from (0,0) to (100,100); saved before being drawn. |
| S11 | `100-200-300_r50.dwg` | S5 with a 50 ft free curve (fillet) added at the first PI (100,0) in Civil 3D; saved before being drawn. |
| S12 | `100-200-300_r75.dwg` | S11 with the curve radius changed to 75 ft in Civil 3D; saved. |
| S13 | `S11_r75_generated.dwg` (generated) | S11 with the radius set to 75 by `examples/set_radius`. |
| S14 | `S13_civ3d_resaved.dwg` | S13 opened in Civil 3D (AUDIT 0 errors, radius 75 and stationing correct), saved. |
| S15 | `100-200-300_r50_seg3plus100.dwg` + `.xml` | S11 with tangent 3 lengthened by 100 ft in Civil 3D; saved, and exported to LandXML 1.2 by Civil 3D. |
| S16 | `S11_seg3plus100_generated.dwg` (generated) | S11 with tangent 3 set to 200 by `examples/set_length`. |
| S17 | `generated_L90_r50.dwg` (generated) + `.xml` | S11 as a template, PIs set to (0,0), (100,0), (100,75), (100,100) by `examples/set_pis`; opened in Civil 3D (AUDIT 0 errors) and exported to LandXML. |
| S18 | `r50_NO_STYLE.dwg` | The S15 alignment with the owner's own alignment style `NO STYLE` (made with *New*; every component off); drawing purged. |
| S19 | `r50_TEST_STYLE.dwg` | As S18, but the alignment uses the owner's style `TEST STYLE` (made with *New*; only the line component drawn). Both styles exist in both files. |
| S20 | `S18_TEST_STYLE_generated.dwg` (generated) | S18 with the alignment's style set to `TEST STYLE` by `examples/set_style`; opened in Civil 3D, AUDIT 0 errors. |
| S21 | `TEST_STYLE_line_off.dwg` | As S19, with `TEST STYLE`'s line component turned off (nothing drawn); saved. |
| S22 | `TEST_STYLE_line_on.dwg` | S21 with only `TEST STYLE`'s line component turned back on in Civil 3D; saved. |
| S23 | `S21_line_on_generated.dwg` (generated) | S21 with component 0 of `TEST STYLE` turned on by `examples/style_components`. |
| S24 | `ZERO_CIVIL.dwg` | Owner's simplified, purged base: one alignment (S15 geometry) using `TEST STYLE`. |
| S25 | `ZERO_CIVIL_second_alignment.dwg` | S24 plus a second alignment, `Alignment - (2)`, created in Civil 3D from a polyline drawn (100,100) → (0,100), style `TEST STYLE`; saved. Creation sample A. |
| S26 | `S24_created_alignment.dwg` (generated) | S24 plus `civil3drust - 1`, a tangent (0,−100) → (200,−100), created by `examples/create_alignment` with S25 as donor (companion FB2, alignment FB3, label group FB4). |
| S27 | `S24_created_alignment_no_label_group.dwg` (generated) | As S26 but without the station-equation label group. |
| S28 | `S26_civ3d_resaved.dwg` | S26 opened in Civil 3D (AUDIT 0 errors), saved. |
| S29 | `S27_civ3d_resaved.dwg` | S27 opened in Civil 3D (AUDIT 0 errors), saved. |
| S30 | `S24_created_alignment_no_parcel_entry.dwg` (generated) | S24 plus `civil3drust - 1` with no label group and no parcel entry (no companion, registry untouched, companion references nulled). |
| S31 | `S30_civ3d_resaved.dwg` | S30 opened in Civil 3D (AUDIT: 2 errors found, not fixed), saved. |
| S32 | `S30_audit_fixed.dwg` | S30 opened in Civil 3D, AUDIT with fixing, saved. |
| S33 | `S24_created_alignment_v2.dwg` (generated) | As S26, generated after the end-station fix. |
| S34 | `S25_companions_swapped.dwg` (generated) | S25 with the two alignments' parcel companions swapped. |
| S35 | `S34_civ3d_resaved.dwg` | S34 opened in Civil 3D (AUDIT 0 errors), saved. |
| S36 | `S24_renamed_A.dwg` (generated) | S24 with the alignment renamed `A`. |
| S37 | `S24_renamed_long.dwg` (generated) | S24 with the alignment renamed to a 65-character name. |
| S38 | `S24_created_B.dwg` (generated) | S24 plus a created alignment named `B`. |
| S39–S41 | `S36/S37/S38_civ3d_resaved.dwg` | S36–S38 opened in Civil 3D (AUDIT 0 errors each), saved. |
| S42 | `S41_descriptions.dwg` | S41 with descriptions entered in Civil 3D for both alignments. |
| S43 | `S41_descriptions_generated.dwg` (generated) | S41 with the same descriptions set by civil3drust. |
| S44 | `S25_plus_two_tangent_alignment.dwg` | S25 plus `Alignment - (3)` from a 3-vertex polyline (two tangents). |
| S45 | `S24_created_two_tangents.dwg` (generated) | S24 plus `Two tangents`, same PIs as S44's, by civil3drust. |
| S46 | `S24_created_five_pis_two_curves.dwg` (generated) | S24 plus `C`: five PIs, R50 and R30 free curves. Does not open in Civil 3D; RECOVER removes `C`. |
| S47 | `S45_civ3d_resaved.dwg` | S45 opened in Civil 3D (AUDIT 0 errors), saved. |
| S48 | `S25_plus_C_no_curves.dwg` | S25 plus `C`, five PIs, no curves (Civil 3D). |
| S49 | `S48_C_with_curves.dwg` | S48 with R50 / R30 free curves added in Civil 3D. |
| S50 | `S24_created_C_v2.dwg` (generated) | S46 regenerated after the arc-string fix. |
| S51 | `S50_civ3d_resaved.dwg` | S50 opened in Civil 3D (AUDIT 0 errors), saved. |
| S52 | `S44_created_D.dwg` (generated) | S44 plus `D` (template from S25; registry entry added). |
| S53 | `S24_created_E1_E2.dwg` (generated) | S24 plus two created alignments. |
| S54 | `S52_civ3d_resaved.dwg` | S52 opened in Civil 3D (AUDIT 0 errors), saved. |
| S55 | `S53_civ3d_resaved.dwg` | S53 opened in Civil 3D (AUDIT 0 errors), saved after S54. |
| S56 | `S24_minimal_experiments.dwg` (generated) | Experiments X0–X5: which parts can be empty ([alignment-map.md](alignment-map.md)). |
| S57 | `S56_civ3d_resaved.dwg` | S56 opened in Civil 3D (AUDIT 0 errors), saved; results in [alignment-map.md](alignment-map.md) §4. |

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

## Arc element records: S10

S10's record has one element record, header at bit 2994 (same place as in
S5). The display cache is a placeholder (box (0,0)–(100,100)).

```text
BL 0x04020008   header, same as tangents
B  1
BD 60.0
BD -1.0
BL 1
BD -1.0
BL 2
B  0
BL id = 1, BL before = 0, BL after = 0
RD start X, RD start Y      (0, 0)
RD mid X,   RD mid Y        (70.7107, 29.2893)
RD end X,   RD end Y        (100, 100)
```

The middle point is the arc's midpoint: the circle through the three points
has centre (0, 100), radius 100, turns left (counter-clockwise), sweeps 90°,
and is 157.0796 long, matching how the arc was drawn. Centre and radius are
not stored as such. (157.0796 does appear later in the record, outside the
element record; not decoded.)

**Unknown parameters.** The five values between the header and the IDs were
identical (60, −1, 1, −1, 2) in S10 and in all six arcs of a production
drawing (radii 25–40 ft, single arcs and arcs between tangents). Not
geometry; perhaps defaults such as a design speed. Kept as read
(`ArcParams`).

**Check against a production drawing** (local only): `examples/alignments`
decoded all four alignments with curves — three tangent–arc–tangent curb
returns (element 2 is the arc, linked 1 ↔ 2 ↔ 3) and a three-arc
cul-de-sac (35 / 40.01 / 35 ft). Radii, deltas, lengths and stationing
match the earlier extraction from the display cache. One offset alignment
still has no element records.

**Tangents vs arcs.** Both start with the same header. After it, a tangent
has `10` (read as BL 0) followed directly by its IDs; an arc has the bit `1`
and the parameter block. No separate type field has been found; the decoder
tries both layouts and validates each (link IDs in range, finite
coordinates, three non-collinear points for an arc).

## Free arcs (fillets): S5 → S11

Adding a 50 ft curve at PI (100,0) in Civil 3D:

- **Tangents keep their PI-to-PI coordinates.** Tangent 1 is still
  (0,0)→(100,0) and tangent 2 still (100,0)→(200,50); only their links
  change (tangent 1 `after` 2 → 4, tangent 2 `before` 1 → 4).
- **The arc is element 4**, stored *after* tangent 3 but linked 1 → 4 → 2.
- **Its record is a second arc kind** — same fields up to the links, but:

  | Field | Arc by 3 points (S10) | Free arc (S11) |
  |-------|----------------------|----------------|
  | B after header | 1 | 0 (but 1 after a Civil 3D re-save, S14 — not the kind) |
  | p1 | 60.0 | **−1.0** |
  | p2, n1, p3 | −1, 1, −1 | −1, 1, −1 |
  | n2 | 2 | **29** |
  | after the links | 6 raw doubles: start, mid, end | **BD radius** (50.0) |

- **Geometry is derived**, not stored: the arc is tangent to both
  neighbouring tangents with the stored radius. For S11: deflection
  26.5651°, T = 11.8034, PC (88.1966, 0), PT (110.5573, 5.2786), centre
  (88.1966, 50), length 23.1824, alignment length 311.3790.
  `decode_elements` computes exactly these and trims the tangents to the
  PC/PT. The only related value stored is 88.1966 (the PC station) twice
  further on, outside the element record, together with the cache data.

So "fixed" curves (made from polyline arcs, or drawn by points) store their
points, while "free" curves (fillets between tangents) store only a radius
and depend on their neighbours, matching the fixed/free entity kinds
described in Autodesk's public Civil 3D documentation. The meaning of the
flag and of n2 (2 vs 29) is inferred from these two samples only.

## Changing a free arc's radius: S11 → S12, and S13

`rawdiff S11 S12`: only the alignment, the minor-station label group, a
feature style and the catalog node differ. In the alignment record:

| Change | S11 | S12 |
|--------|-----|-----|
| Radius (BD after the arc's links, bit 4307) | 50.0 | 75.0 |
| Cached PC station, two copies (BDs at bits 4534 and 4748) | 88.1966011250105 | 82.29490168751575 |
| Volatile header field | — | changed |
| Block near bit 5300 | — | **new**: three BDs 2.0, 3.0, 4.0 (+211 bits) |

Tangent records, links and the placeholder box are unchanged, confirming
that a free arc depends only on its radius and its neighbours. The decoded
S12 geometry: T = 17.7051, PC (82.2949, 0), PT (115.8359, 7.9180), length
34.7736, alignment length 311.1668, all as computed by hand.

The 2.0 / 3.0 / 4.0 block looks like element IDs (the arc and its
neighbours, or the elements whose stations changed); not decoded.

**Generated edit (S13).** `examples/set_radius S11 S13 8C5C 4 75` patched
the radius and both PC-station copies. `rawdiff S13 S12 AECC_ALIGNMENT`
leaves: the volatile field; the last bits of the PC station
(82.29490168751578 vs Civil 3D's ...575, a 3×10⁻¹⁴ difference from
floating-point order); and the 2/3/4 block, which civil3drust does not
write. The radius is bit-identical.

**Round trip (S14).** S13 opened in Civil 3D 2024 with zero AUDIT errors and
the curve at R = 75 with correct stationing; re-saved as S14.
`rawdiff S14 S12 AECC_ALIGNMENT`: **one bit** differs — the flag bit right
after the arc's header (bit 4055). Everything else, including the volatile
header field, the 2/3/4 block and the PC station (now Civil 3D's own
82.29490168751575), is identical to Civil 3D's own edit (S12). So on load
and save Civil 3D recomputed the station, wrote the 2/3/4 block, and set the
volatile field to the same value as in S12 — which suggests that field is
derived from the content rather than random.

**The bit after the arc header is not the arc kind.** It is 1 in S10 (arc by
three points) and in S14 (still a free arc: p1 = −1, n2 = 29, radius only),
and 0 in S11–S13. Its meaning is unknown; the decoder now picks the layout
from p1/n2 (D-012).

## Tangent edit past a fillet: S11 → S15, LandXML check, and S16

**What Civil 3D changed** (`rawdiff S11 S15 AECC_ALIGNMENT`, record size
unchanged): tangent 3's end X (300 → 400), two corners of the placeholder
box (x 300 → 400), the volatile header field, and the arc's flag bit
(0 → 1, as in S14). Nothing else: the edit is downstream of the curve, so
the arc and its cached PC station are untouched.

**Decoder vs Civil 3D's LandXML.** S15's LandXML export lists the same
alignment. Every value matches the decoder's output to all 12 decimals
LandXML gives: length 411.378981575051; tangent 1 length 88.196601125011;
curve start (88.196601125011, 0), centre (88.196601125011, 50), end
(110.557280900008, 5.278640450004), PI (100, 0), radius 50, delta
26.565051177078°, length 23.18238045004, rotation ccw; tangents 2 and 3
lengths 100 and 200. (LandXML lists coordinates as northing, easting.)
LandXML also carries `<Feature name="SpeedStation">` with speed 60 at
station 0; whether p1 = 60 in arcs by points is a design speed is not
established (free arcs have p1 = −1 in the same drawing).

**Generated edit (S16).** `examples/set_length S11 S16 8C5C 3 200` now
handles alignments with free arcs: the tangent records form a PI-to-PI
polyline, so the PI edit from D-003 applies unchanged; free arcs follow,
and cached PC stations are updated when they move (none here). It patched
tangent 3 and the placeholder box. `rawdiff S16 S15 AECC_ALIGNMENT` leaves
only the volatile field and the arc's flag bit. Re-running the S5 edit with
the new code reproduces S7 byte for byte.

## Generating an alignment from a template: S17

civil3drust cannot create an `AECC_ALIGNMENT` record from nothing: the
record prefix, most of section 2 and the handle stream (style, site, label
links) are not decoded. It can rewrite a record whose *shape* matches: the
same number of tangents with free curves in the same places.
`aecc_alignment_edit::set_pis` moves every PI, lets free curves follow,
updates their cached PC stations and the placeholder box, and refuses
geometry where a curve doesn't fit.

S17 was asked for as 0,0 → 100,0 → 100,100 with a 50 ft fillet. The nearest
template, S11, has four PIs, so the PIs were set to (0,0), (100,0),
(100,75), (100,100): the requested geometry with an extra zero-deflection
PI on the second leg. Result: tangent 50.0000, R = 50 left 90° curve
(PC (50,0), PT (100,50), centre (50,50), length 78.5398), tangents 25 + 25;
total 178.5398. Changes against S11: tangent records 2 and 3, the
placeholder box corners (300 → 100, 50 → 100), and both PC-station copies
(88.1966 → 50.00000000000001; the last digit is floating-point rounding
of T = R·tan 45°).

**Verified in Civil 3D.** S17 opened with zero AUDIT errors. Civil 3D's
LandXML export of it lists exactly the intended alignment: line 50; curve
radius 50, ccw, delta 90, start (50,0), centre (50,50), end (100,50), PI
(100,0), length 78.539816339745; lines 25 and 25 at 90°; length
178.539816339745. The zero-deflection PI was kept as two separate lines,
so Civil 3D accepts it.

An exact three-PI version needs a template with two tangents and one free
curve, or support for removing elements (which changes the record size and
probably element counts in section 2).

## Handle stream: S1–S19

Each record's handle stream is its last `dwg_handle_bits` bits (acadrust
reports the count; `RawRecord::handle_bits`). It is a run of handle
references, each a byte `CODE(4) | COUNTER(4)` followed by `COUNTER`
handle bytes, most significant first (public ODA specification; acadrust
`read_handle_reference`). Decoded by `handle_refs`; listed by
`examples/refs.rs`.

Every alignment in S1–S19 ends with the same seven references:

| # (from end) | code | S11 | S18 | target |
|---:|---|---|---|---|
| 7 | 5 hard ptr | 3800 | 10 | the entity's layer (`C-ROAD`, `0`) |
| 6 | 4 soft ptr | 0 | 0 | null |
| 5 | 5 hard ptr | 8C5B | 8C5B | an `AECC_FEATURE_STYLE` object, handle = alignment − 1 in every sample |
| 4 | 5 hard ptr | 8C5B | 8C5B | the same object again |
| 3 | 4 soft ptr | 0 | 0 | null |
| 2 | 5 hard ptr | 1F64 | 8DE7 | **the alignment style** (`AECC_ALIGNMENT_STYLE`) |
| 1 | 5 hard ptr | 0 | 0 | null |

S1 has one more in front: code 3 (hard owner) to a dictionary, probably its
extension dictionary.

**Style reference (S18 vs S19).** The two alignment records are the same
size (756 bytes) and differ only in bits 6031–6036: the low byte of
reference 2, `5.2 8D E7` (`NO STYLE`) vs `5.2 8D FE` (`TEST STYLE`). No
other alignment field depends on the style. S20, made from S18 by rewriting
only that byte (`aecc_alignment_refs::set_alignment_style`), has an
alignment record byte-identical to S19's. The only other record that
differs between S18 and S19 is one `AECC_NETWORK_PART_CATALOG_DEF_NODE`
(not related; not studied).

Changing the style this way needs the new handle to take as many bytes as
the old one, so the record size stays the same (decision D-019).

**Companion object.** The `AECC_FEATURE_STYLE` object at alignment − 1
(8C5B in S5–S19) soft-points back to the alignment (its second reference)
and to object 12F9, which soft-owns (code 2) every such companion in the
drawing (8C5B, and 37FD in S11/S18). Creating an alignment will likely mean
creating a companion and adding it to that list; to be confirmed by
creation sample A.

**Alignment style records.** All styles, and a collection record of the
same class (9B3) that soft-owns each of them, are `AECC_ALIGNMENT_STYLE`.
A style record starts with a soft pointer to the collection (or `own−1`
when it is the next handle), then about 70 references: mostly null, plus
for each display component a triple (layer, linetype, null), e.g.
(`0`, `ByBlock`, null), with `DASHED2` and `ByLayer` in a few. `NO STYLE` and
`TEST STYLE` have identical handle-stream contents (both reference block
`_None`, 8DFB, as an arrow block). Their names are the first plain string
in the string stream (`raw::style_name`), followed by the description and
created-by / modified-by user names.

The data streams of `NO STYLE` and `TEST STYLE` differ from bit 29 on: the
object's own handle is stored early (8DE7 vs 8DFE), and the varying
encoding lengths after it shift everything, so a plain bit diff doesn't
show which bits are component visibility. Decoding needs either field-by-field
parsing or style pairs that share a handle length *and* name length (see
Next samples).

## Alignment style: display components (S21 → S22, S23)

S21 and S22 were saved from the same drawing; between them only the line
component of `TEST STYLE` (8DFE) was switched on. In the style record
(603 bytes in both) exactly two things changed:

1. **One bit** at 1428 (0 → 1): the line component's visible flag.
2. **A 64-bit Windows FILETIME** (100 ns ticks since 1601, UTC), stored
   raw at bit 801: 2026-10-22 19:39:33 → 19:40:03 UTC, i.e. the style's
   modified time. A second FILETIME at bit 1307 (19:15:01 UTC, both
   files) is probably the creation time. (Positions are for this record;
   they move with the encoding before them.)

The flag belongs to a run of **17 display-component blocks**: 16 spaced
123 bits apart (one gap is 125) and a last, longer one (135 bits in S24).
Each block contains a run of sixteen 1 bits followed by `01`; the visible
flag is 47 bits before it. (A first version matched `…0101`, which misses
the 17th block whenever its next bits differ, as in S2 and S5–S23; S24
showed all 17.) Reading that bit in every style in S1–S24 gives sensible
results:

| style | flags (component 0 first) |
|---|---|
| `NO STYLE` (owner's, all off) | `00000000000000000` |
| `TEST STYLE`, S21 | `00000000000000000` |
| `TEST STYLE`, S22 | `10000000000000000` |
| template default style, no name stored (S2 C2, S24 48C) | `11111101000000001` |
| Basic | `11100001000000001` |
| Proposed | `11101101111000001` |
| Existing | `11100001111000001` |
| Layout | `11101111111000001` |
| Offsets | `11100000000000000` |

**Names (owner's screenshots of the Display tab, S98).** The 17 blocks
are the three view directions in Civil 3D's order: **Plan** 0–7 — Line,
Curve, Spiral, Arrow, Line Extensions, Curve Extensions, Tangent
Extensions, Warning Symbol; **Model** 8–15 — the same eight; **Section**
16 — Line Marker in Section (the longer last block). Confirmed bit by bit:
0 (S21 → S22) and 1 (S97 → S98: plan Curve on; the screenshots show plan
Line and Curve on, everything in Model and Section off = `11000000 00000000
0`). Every style above reads sensibly this way, e.g. Basic = plan Line,
Curve, Spiral, Warning Symbol + section marker; Proposed adds the line and
curve extensions in plan and Line, Curve, Spiral in model; Offsets = plan
Line, Curve, Spiral only.

The default style of S2's and S24's template stores no name: its only
strings are the created-by / modified-by pair (`raw::style_name` returns
`None`). Civil 3D lists it as **Standard** (owner, S24), so the name of
the default style is implied, not stored.

S23 (S21 with that one bit set by civil3drust) has a style record equal to
S22's except for the modified time, which civil3drust leaves alone.

In the alignment record the S21 → S22 change only added display graphics
(+28 bytes, proxy graphics now hold the drawn line); civil3drust does not
write those, and Civil 3D redraws on open.

The S19 → S21 re-save changed every style record (and many other style
classes), so style comparisons are only valid between consecutive saves
of one session, as here.

## Base drawing: S24

From S24 on, samples start from `ZERO_CIVIL.dwg` (S24), the owner's
further simplified and purged drawing (473 KB, 3008 AECC records in 317
classes). acadrust reads it without failsafe mode: the
`PRESSUREPARTERRORBLOCK` 3DSOLID of S1/S5–S19 is gone, so files written
from it lose nothing.

It holds one alignment, `Alignment - (1)` [F9E], with the S15 geometry
(0,0 → 100,0 → 200,50 → 400,50, free R = 50 curve at PI 1; 411.379 ft),
and two alignment styles besides the collection (48B): the template
default [48C] (shown as Standard, no name stored) and the owner's `TEST STYLE` [FA9], line
component on, used by the alignment. The alignment's handle stream has the
usual seven references (layer `0`, companion F9D = F9E − 1, style FA9).
The companion registry is BC0, which soft-owns only F9D.

`set_length`, `set_radius`, `set_pis` and `style_components` all work on
it unchanged (read-back verified).

## Creating an alignment: S24 → S25 (sample A)

Objects added (every other handle is unchanged; handles FB9, FBA, FBC,
FBD were used and freed, presumably by the source polyline, which Civil 3D
erased):

| handle | class | what it is |
|---|---|---|
| FB7 | `AECC_FEATURE_STYLE` | the companion object (alignment − 1, as predicted from S1–S24) |
| FB8 | `AECC_ALIGNMENT` | `Alignment - (2)`, 463 bytes, one tangent (100,100) → (0,100), real display graphics |
| FBB | `AECC_ALIGNMENT_STAEQU_LABEL_GROUP` | station-equation label group, entity in model space |

References between them:

- **FB8** (alignment): the usual seven — layer `0`, FB7 twice, style FA9.
  It does not point at its label group.
- **FB7** (companion): first reference `own−off` → BC0 (the registry),
  then a soft pointer → FB8.
- **BC0** (registry, `AECC_FEATURE_STYLE`, owned via `own−1` by BBF
  `AECC_ALIGNMENT_PARCEL_NODE`): gains a soft-owner reference to FB7 in
  front of the existing one to F9D (34 → 62 handle bits; 118 → 191 bytes,
  so it carries per-entry data too).
- **FBB** (label group): layer `0`, null, null, soft pointer → FB8, hard
  pointer → label style 49F (`AECC_LABEL_COLLECTOR_STYLE`). Byte size and
  layout equal to the first alignment's group FA3, which points at F9E.

The first alignment also has an `AECC_ALIGNMENT_DESIGNSPEED_LABEL_GROUP`
(FA4); the new one got none, so it is not required.

Model space entity order after the save: F9E, FA3, FA4, FB8, FBB.

Both alignment records store their own handle at bit 26 of the data, so a
copied record must have it patched.

Other differences, all save noise or "last used" settings:

- `AECC_SETTINGS_NODE` 994 (holds the string `AeccCreateAlignmentEntities`,
  the Create Alignment command settings): +7 bytes and a new hard pointer to
  `TEST STYLE` — the last style used by the command.
- `AECC_NAME_TEMPLATES_NODE` FA0 (holds `Alignment - (<[…]>)`): one bit,
  112, changed 0 → 1; probably the name counter's next value (not yet
  decoded).
- Two XRecords re-created with new handles and identical content
  (`AECC_LASTSAVED_APPVERSION` 13.6.1963 and the Map `_CurrQry`), the
  first alignment's design-speed group (a −2.0 double lost its rounding
  error), and the network-part catalog (changes on every save).

So creating an alignment needs at minimum: the alignment record, its
companion, the registry entry, and (probably) the station-equation label
group. Whether Civil 3D accepts a file without the label group, and with
the name counter and settings untouched, is the next experiment.

## First generated alignment: S26, S27

`aecc_create::create_alignment` (D-021) adds an alignment to S24 by
copying S25's four creation records (companion, alignment, label group,
registry) and turning them into a new alignment:

1. New handles from the target's handle seed, in Civil 3D's order:
   companion FB2, alignment FB3, label group FB4. Each record's own handle
   (`0x02` + 2 bytes at bit 18) is patched.
2. References between them retargeted in place: alignment → companion
   (twice), companion → registry (`own−off`, offset recomputed) and →
   alignment, label group → alignment, registry soft-owner → companion.
3. A new 32-hex-digit ID string written over the donor's in the companion
   and in the registry (the only two places S25 stores it).
4. Name `civil3drust - 1` written over `Alignment - (2)` (same length) and
   the tangent moved to (0,−100) → (200,−100) with `set_pis`.
5. The companion inserted as a raw object owned by the registry, the
   alignment and label group added to model space, and the registry's
   bytes replaced by the patched two-entry version.

Every record keeps its size. Read back, S26 has both alignments decoding,
and all references resolve. S27 is the same without the label group.

**Civil 3D result (S28, S29).** Both files open with AUDIT 0 errors;
Civil 3D shows its "Non Autodesk DWG" notice on open (see D-022). Re-saved:

- `civil3drust - 1` keeps its name, style, geometry and handles
  (FB2/FB3/FB4); the registry still owns FB2 then F9D; the label group
  FB4 (S28) is kept unchanged.
- **The label group is optional**: S29 has none, and Civil 3D did not add
  one.
- In the generated alignment Civil 3D changed only what it changes in its
  own alignments on every re-save — the 2-bit flag (open question 9) and
  the +44-bit volatile field (open question 1) — plus regenerated display
  graphics. `Alignment - (1)` got the identical treatment.
- Many style records were re-encoded (as in the S19 → S21 re-save); not
  caused by the generated objects.

Not touched (S25 showed them changing, but they look like bookkeeping):
the name counter in `AECC_NAME_TEMPLATES_NODE` and the Create Alignment
command's last-used settings.

## The companion object's content (S1–S28)

The companion is class `AECC_FEATURE_STYLE` (C++ `AeccDbFeatureStyle`,
class number 1056 in S25; its registry BC0 is the same class, owned by
BBF `AECC_ALIGNMENT_PARCEL_NODE`, `AeccDbAlignmentParcelNode`, 1055). In
S25:

- F9D (for `Alignment - (1)`), 231 bytes: object header, then bit-coded
  doubles **200, 50, 0** (bits 356–489), a BD 1.0, about 30 bits, then
  **400, 50, …** (from bit 522); further data; the ID string
  `b4c9ce06…` at bit 1240; handle stream: `own−off` → BC0, soft pointer →
  F9E, three nulls.
- FB7 (for `Alignment - (2)`), 174 bytes: **100, 100, 0**, 1.0, …, then
  a point with x = 0 (2-bit BD) and **100**; ID string `07ce2462…` at bit
  785; handle stream `own−off` → BC0, soft pointer → FB8, three nulls.

Full doubles found in the companion across samples (zeros are 2-bit BDs
and don't show):

| sample | alignment PIs | companion |
|---|---|---|
| S1, S2 | (0,0) → (100,0) | 100 |
| S10 | arc (0,0) → (100,100) | 100, 100 |
| S5 | (0,0) (100,0) (200,50) (300,50) | 300, 50, 100, 200, 50 |
| S11 | S5 + R50 at PI 1 | 200, 50, 300, 50 |
| S12 | S11 with R75 | 300, 50, 200, 50 |
| S15 | S11, tangent 3 to (400,50) | 400, 50, 200, 50 |
| S24/S25 F9D | as S15 | 200, 50, 400, 50 |
| S25 FB7 | (100,100) → (0,100) | 100, 100, 100 |

So the companion stores points of the alignment's geometry — the
endpoints of the last tangent (or of the whole alignment when it has one
element), order varying — and Civil 3D updates them when it edits the
alignment. Its class name and its registry's owner suggest it is the
alignment's entry in the parcel/site topology.

**S28:** after Civil 3D opened and re-saved S26, the generated
alignment's companion FB2 still holds the donor's points (100,100) /
(0,100), not (0,−100) / (200,−100); it changed size (174 → 177 bytes) but
those values were not refreshed, and AUDIT was clean. `create_alignment`
should write the new geometry there too once the layout is pinned down.

## Without the parcel entry: S30

The owner identified the companion as the alignment's parcel (site
topology) data. S30 tests whether an alignment needs it: the new
alignment FB3 is written alone — no companion object, no registry entry
(BC0 identical to S24), no label group. Its handle stream becomes layer,
null, **null, null** (were the companion), null, style, null: 113 → 81 bits
and 463 → 459 bytes, rebuilt by `handle_refs::null_refs`; the data stream
is unchanged. S30 differs from S24 only by the added alignment record.

**Civil 3D result.** AUDIT reports **2 errors** (not fixed; the summary
names no objects). S26/S27 (with the parcel entry) audit clean, so the
errors come from the missing parcel entry — most likely the alignment's
two null companion references. Re-saved without fixing (S31), Civil 3D
keeps the alignment with null references and does not rebuild a
companion or registry entry. The other objects it adds (a dictionary with
a `1" = 1'` scale, class-node/attribute-classification objects, save
history and app-version XRecords) are the same as in the clean S28/S29
re-saves, so they are routine for files written by acadrust.

Conclusion: the parcel entry is expected for every alignment;
`create_alignment` writes it by default.

**AUDIT with fixing (S32).** Compared with S31 (same file, not fixed):

- The alignment's companion references stay **null**; no companion or
  registry entry is created. So AUDIT's fix does not rebuild the parcel
  entry.
- In `civil3drust - 1` exactly one value changed: the BD at bit 2259
  (2207 before the re-save shift), **99.99999999999999 → 200**.
- `Alignment - (1)`'s companion F9D got new points, (110.557, 5.279) and
  (200, 50) — the middle tangent — replacing what was there: AUDIT
  recomputes companion geometry.

## Station at the end of the first element

The value AUDIT corrected is the first bit-coded double after the element
records. It is the station at the end of element 0: 100 in S5–S7 (first
tangent), 88.197 in S24 (first tangent up to the curve), the total length
for a single-tangent alignment (S25: 99.99999999999999 — Civil 3D's own
rounding of 100). When element 1 is a free arc it is also the first of the
two PC-station copies, which is why the radius and tangent edits already
updated it in the curve samples.

`set_pis` (and so `set_tangent_length` and `create_alignment`) now patches
the first BD copy of the old end-of-element-0 station after the element
records. Before the fix, S26/S27/S30 kept the donor's 99.99999999999999;
S26 still audited clean, so a stale value there is not one of S30's two
errors. S33 is S26 regenerated with the fix.

**Re-audit of S32:** 0 errors. After AUDIT's fix, an alignment with null
companion references and no parcel entry is accepted. The owner decided to
set the parcel companion aside (D-023).

## Experiment: swapped companions (S34)

S25 with `Alignment - (1)` (F9E) pointed at FB7 and `Alignment - (2)`
(FB8) at F9D, both references each, and the companions' back-pointers
swapped to match (`examples/swap_companions`). Only those four records
change, all same-size. Each parcel entry now holds the other alignment's
points and ID string.

**Result (S35).** AUDIT 0 errors. Civil 3D keeps the swapped links and, on
save, **regenerates each companion's points from its new alignment**: F9D
(now `Alignment - (2)`'s) goes from 200, 50, 400, 50 to 100, 100, 100
(231 → 162 bytes); FB7 (now `Alignment - (1)`'s) from 100, 100, 100 to
200, 50, 400, 50 (174 → 228 bytes). Each keeps its own ID string and
registry position; the alignments are untouched. So a companion is
interchangeable: its geometry is derived from whichever alignment points
at it, and only the links and the ID string matter. That also explains
why the donor's stale points in our generated companions (S26, S33) were
harmless.

## Names and the string stream (S1–S38)

The alignment record's strings sit in an R2007+-style string stream at
the end of the data stream (public ODA layout, confirmed in every sample):

- the data stream ends with a **flag bit** (1) at `data_end − 1`, preceded
  by a **16-bit little-endian size** at `data_end − 17` — the string stream
  length in bits (S25 FB8: 536 bits, stream 3038–3574);
- each string is a bit-coded short **length** (`01` + 1 byte for 1–255
  code units; `10` for empty; `00` + 16 bits for longer) followed by the
  **UTF-16LE code units**.

Alignment strings, in order: `AeccImpAlignment`, the **description
twice** (empty unless set), the **name**, then 5–11 empty strings (count
varies by record). In S9 and S22, after edits made in Civil 3D, an extra
string — the user name `felix` — sits before the name (`raw::object_name`
takes the fifth string when it is non-empty).

**Description (S41 → S42).** Typing a description in Civil 3D changes only
strings 1 and 2 (identical copies; line breaks stored as `\r\n`) and the
stream size: every bit before the string stream and the whole handle stream
are unchanged. `string_stream::set_description` writes both copies; S43
(S41 + the same descriptions by civil3drust) has both alignment records
byte-identical to S42's.

`string_stream::set_name` rewrites the name's length and code units,
recomputes the size field, and moves the rest of the stream, the size,
the flag and the handle stream as a block; the record grows or shrinks.
`examples/set_name.rs` renames an alignment; `create_alignment` now accepts
any non-empty name. S36 (`A`), S37 (65 characters) and S38 (created `B`)
all open with AUDIT 0 errors and keep their names through a Civil 3D save
(S39–S41).

## Element list, cache and graphics (S44 and S1–S15)

Comparing the one-tangent `Alignment - (2)` (FB8) with the two-tangent
`Alignment - (3)` (FF3) in S44, and the curve samples:

- **Graphics size.** After the object header: bits 42–43 EED size `10`
  (none), bit 44 has-graphics `1`, then from bit 45 a bit-coded long long
  (3-bit byte count + little-endian bytes) — the proxy graphics size in
  bytes (FB8 156, FF3 216, `Alignment - (1)` 312). With `TEST STYLE` the
  graphics are seven attribute chunks followed by one 60-byte, 2-point
  polyline chunk (opcode 6) per tangent; arcs aren't drawn by that style.
- **Element count.** A BL (`01` + byte) right before the first element:
  1 (S1, S2, FB8, S10), 2 (FF3), 3 (S5), 4 (S11, S24).
- **Kind prefix.** Each element record is preceded by 11 bits:
  `00000000010` for a tangent, `00000000100` for an arc (S10, S11).
  Records are stored in ID order; tangents get IDs 1…n, arcs the next IDs
  (S11: T1 T2 T3 A4, path T1 A4 T2 T3).
- **After the last record:** 16 bits, the alignment's **identity value**
  — a version-4 GUID in little-endian GUID layout (version nibble 4 in
  byte 7, variant `10` in byte 8 in every Civil 3D value); the same value
  is stored again ~717 bits later; distinct per alignment, found nowhere
  else; stable through edits and most re-saves (S5/S6/S8/S11/S12,
  S24→S44), but in S47 Civil 3D gave **both** alignments new GUIDs (S45's
  generated value had version nibble 5 — fixed since) — then bits up to
  161 after the records, the BD station at the end of element 0, and the **element-0
  cache**: BL 1, BL 0, BL id of the element after element 0, then six raw
  doubles — element 0's start, its end (the PC when a curve follows), and
  the start again. Nothing else in the record depends on the element
  count; the rest is copied unchanged.

`aecc_build::set_geometry` rebuilds the graphics (donor's attribute
chunks + one polyline per tangent), the count, the element records and the
cache from PIs and optional radii; `set_identity` writes a fresh identity
value (both copies). From S24 + S25's one-tangent donor, S45 reproduces
S44's two-tangent record exactly from the count to the string stream
(test `two_tangents_match_civil3d_s44`) apart from the identity value; the
graphics are identical too. Re-encoding S5 with R = 50 at PI 1 reproduces
S11's four element records bit for bit.

`create_alignment` now takes any number of PIs and free curves at interior
PIs (`x,y,r` in `examples/create_alignment`).

**Civil 3D results.** S45 (two tangents) opens with AUDIT 0 errors and
keeps its geometry through a save (S47). S46 (five PIs, two free curves)
did not open; RECOVER deleted the alignment.

**Why (S49).** Comparing S46's `C` with the same alignment made in Civil
3D: element records identical, but

- **each free arc reads three empty strings** from the string stream:
  tangent-only records have 12 strings (S2, S5, S25), one arc 15 (S10,
  S11), two arcs 18 (S49). S46 had 12, so Civil 3D ran off the end of the
  stream. (S12 and S48 carry 8 more, together with other extra data —
  optional, not needed.)
- in the graphics, after each arc Civil 3D repeats the attribute triple
  (opcodes 16, 18, 14 — copies of leading chunks 4–6) even though the
  lines-only style draws no arc (also in S24's `Alignment - (1)`).

`set_geometry` now sets the string count to `donor − 3·donor arcs + 3·new
arcs` (`string_stream::set_string_count`) and writes the triple after each
arc. S50 (S46 regenerated) equals S49's record in elements, strings and
graphics, except the identity GUID, the 62-bit section-2 field (copied
from the donor; it differs in every record and isn't a timestamp) and the
last bit of the PC station (Civil 3D 88.1966011250105, us
88.19660112501052 — S24's own alignment, saved by Civil 3D, holds the
latter, so it isn't validated). Test `curves_match_civil3d_s49`.

**Result (S51).** S50 opens with AUDIT 0 errors; re-saved, `C` keeps its
six elements, radii, stationing and its generated identity GUID. Civil 3D
changed the usual re-save fields (2-bit flag, +44-bit volatile field, the
arc flag bit — open questions 9, 1, 11) and **added a block of BDs 2, 3,
4, 5, 6 — the element IDs after the first**. This is the "2.0, 3.0, 4.0"
block of S12 (IDs 2–4 of a four-element alignment), answering open
question 10: an ID list Civil 3D adds after geometry edits; optional, as
S50 shows.

## The companion registry (S24, S25, S44, S48, S11)

The registry (BC0 in S24–S53, 12F9 in S5–S19; class `AECC_FEATURE_STYLE`,
owned by the `AECC_ALIGNMENT_PARCEL_NODE` through an `own−1` reference)
lists every alignment companion. With one to four entries:

| sample | entries | keys | string stream | handle stream |
|---|---|---|---|---|
| S24 | 1 | C76 | `""`, `b4c9…` | own−1, F9D |
| S25 | 2 | C7A, C76 | `""`, `07ce…`, `b4c9…` | own−1, FB7, F9D |
| S44 | 3 | C7A, 29F5, C76 | + `475c…` | + FF2 |
| S48 | 4 | C7A, 29FA, 29F5, C76 | + `37d6…` | + 102A |

(keys shown without their high bit `0x8000_0000`.)

- **Data stream:** after the object header, a BL 0, a BL **count**, then
  one key per entry, always as a full RL (`00` + 32 bits, high bit set) —
  34 bits per entry, the whole growth of the data stream.
- **Keys:** the same value is stored in the companion (first `00` + RL with
  the high bit set, bit 66 in S48's companions). Values come in runs per
  editing session (C76, C7A; 29F5, 29FA) and appear nowhere else, so a new
  entry uses the largest key + 1.
- **Strings:** an empty string, then each companion's 32-hex-digit ID
  string.
- **Handles:** `own−1` to the parcel node, then a soft-owner reference to
  each companion.
- All three lists are in the same order: **sorted by ID string** (S48:
  `07ce`, `37d6`, `475c`, `b4c9`).

`aecc_registry::Registry` parses this and rebuilds it with an entry added
or removed. Adding S25's, S44's and S48's new entries to their
predecessors reproduces Civil 3D's records exactly, except the padding
after the handle stream (Civil 3D pads with ones, we with zeros); removing
gives back the predecessor (test `registry_entries_match_civil3d`).

With this, `create_alignment` (D-025) takes its template alignment,
companion and label group from **any** donor drawing with a Civil 3D
alignment, gives the companion a new key and ID string, and adds it to the
target's own registry. Own handles and references are rewritten at any
byte length (`handle_refs::set_own_handle`, `rewrite_refs`). The style can
be chosen (`--style`); the label group is written only if the target has
its label style. S52 adds `D` to S44 (three existing entries); S53 adds
two alignments to S24 in a row.

**Civil 3D result (S54, S55).** Both open with AUDIT 0 errors; re-saved,
every alignment, companion and registry entry is kept, in ID-string order.
Civil 3D **re-keys the generated entries** and leaves its own alone: `D`
29F6 → 29FC (S54), `E1` C77 → 29FD and `E2` C78 → 29FE (S55, saved next in
the same session). The new keys continue one counter across the two files,
so keys come from a counter of the running Civil 3D session, and the key
civil3drust writes only has to be unique until the next save (it also
regenerates companions whose stored points are stale, cf. S35).

## Alignment type (O1: S58, S59)

**Samples.** S58 and S59 are the same drawing with one alignment (1089,
named "Centerline", one tangent (0,0)→(500,0)) created from a polyline;
in S58 its type is *Centerline*, in S59 *Miscellaneous*. Which file had
which type was not stated; it follows from the value below (S58's value 1
matches every other Civil 3D alignment in the samples, all created with
the default type Centerline).

**Steps.** Region-by-region comparison of record 1089 (graphics, section 2,
elements, post-cache block, strings), then a field walk (BD/BL) of the
post-cache block from GUID copy 2.

**Finding.** After GUID copy 2 the post-cache block reads

| field | S58 | S59 | others |
|---|---|---|---|
| BD | 100.0 | 100.0 | 100.0 in all |
| BL | 0 | 0 | 0 in all |
| BL n | 0 | 0 | 4 or 6 in S12, S14, S48 `C`, S51 `C` |
| n × BD | — | — | 1.0, 2.0, …, n (element IDs; open question 10) |
| **BL type** | **1** | **4** | 1 in every other record |
| BL, BL, BL | 2, 2, 2 | 2, 2, 2 | 2, 2, 2 |
| BL, BL | 0, 1 | 0, 3 | 0, 1 (S2–S15) or 0, 3 (S24 template) |

So **type 1 = Centerline, 4 = Miscellaneous**. This is the only difference
in the post-cache block. Values 1–255 are stored as `01` + one byte, so the
type can be changed in place (`aecc_build::set_alignment_type`;
`create_alignment --type`). Values 2, 3 and 5 are presumably the other
types Civil 3D offers (Offset, Curb Return, Rail), but no sample confirms
them yet (S61).

**Other S58 → S59 differences** (not yet attributed; probably side effects
of the type change being saved by Civil 3D, cf. S57):

- section 2 grew 413 → 631 bits; 44 of them are the optional +44-bit field
  Civil 3D adds on every save; the rest resembles the property data S9
  inserted with a start station.
- the user-name string ("felix") appears before the name, as after other
  Civil 3D edits.
- the companion 1088 grew 3 bytes (a field at bit 336 grew from 10 to 34
  bits); companions are regenerated by Civil 3D (S35).

**Test.** S60: S24 + "Generated centerline" (type 1) and "Generated misc"
(type 4), both written by civil3drust. Civil 3D opened it with no issues
and listed the two types as Centerline and Miscellaneous; its re-save
(S62) keeps the values 1 and 4. So the type value alone sets the type.
In S62 both records have the usual 457-bit section 2 and are identical
outside the type and the name (the 96-bit size difference is the 6 extra
UTF-16 characters of "centerline"). The extra section-2 data in S59 is
therefore not part of the Miscellaneous type; it comes from the
properties edit.

S61: S24 + values 2, 3 and 5. Civil 3D reported errors, and RECOVER removed
all three alignments (S63 has only the original). So these values are
valid only together with data a plain alignment lacks, probably a parent
alignment for offset and curb-return types; which value is which type
stays open until a sample with a real offset or curb-return alignment.
`create_alignment --type` therefore accepts only 1 and 4 by name; other
numbers are an experiment option.

## Drawings without alignments (S64–S67)

**Samples.** S64 is an empty AutoCAD drawing (32 KB, 18 classes, no AECC
classes). S65 is S64 opened in Civil 3D and saved (470 KB): Civil 3D adds
its drawing infrastructure on save — 593 classes, about 300 of them AECC
(settings, styles, label sets, parcel nodes, the companion registry) — but
no alignment classes. Compared with S24 (which has an alignment) S65 lacks
the class entries `AECC_ALIGNMENT`, the four alignment label-group classes
and `AECC_NAME_TEMPLATES_NODE`/`_ROOT`; it has one alignment style (48C, no
name stored) and an **empty registry** (BC0, 45 bytes).

**Empty registry.** The key list is `10 10` (BL 0, count 0) at bit 74; the
249 bits after it are the same as in every registry in S24–S63 (1–7
entries), and the string stream holds only `""`. The handle stream has
only the own−1 reference to the parcel node. `aecc_registry` now parses
it and adds the first soft-owner reference after the existing references.

**Class table.** DWG class numbers are positional (500 + index; all
classes in S24, S64, S65 follow this), and every AECC record starts with
its class number as object type (`10` + RS, bits 0–17). Different
templates number the same class differently (S2: `AECC_ALIGNMENT` 1092;
S24: 1093). `create_alignment` therefore looks the class up in the target,
appends the donor's class entry when it is missing (S65 → number
500 + 593 = 1093), and writes the target's number into every record it
adds. (Before, the donor's number was copied, which only worked because
the samples used matching templates.)

**S66** = S65 + one alignment ("Minimal", one tangent, style 48C, no label
group), template S58 1089: the minimal Civil 3D drawing with an alignment
built entirely from Civil 3D's own default infrastructure.

**S67** = S64 + one alignment record (experiment `exp_bare`): only the
`AECC_ALIGNMENT` class entry (number 518) and the record, with layer 0
and all other references (companion, style) null; 28 KB. It asks whether
Civil 3D builds its settings on open and keeps an alignment that has no
style and no parcel entry.

**Results.**

- S66 (and S65): opened with no issues.
- S67: opens without a RECOVER prompt; the alignment is listed in
  Prospector but not drawn. **Selecting it crashes Civil 3D** (unhandled
  access violation reading 0x0098), so a null style reference is not safe.
- AUDIT on S67: 3 errors found, 3 fixed. The one it names is
  `Alignment (Alignment Style) eNullObjectId → Standard`. After the fix the
  alignment shows as a dot until it is moved (and moved back), then draws
  normally. Saved as S68.
- S67 saved without AUDIT: S69.
- AutoCAD (without Civil 3D) and OpenCADStudio open S67 and draw the
  alignment from its proxy graphics.

**What the re-saves show.**

| | S67 (ours) | S69 (no AUDIT) | S68 (AUDIT) |
|---|---|---|---|
| file | 28 KB | 469 KB | 467 KB |
| record | 435 B | 345 B | 936 B |
| proxy graphics | 8 chunks (one polyline) | 4 chunks, nothing drawn | 26 chunks |
| style reference | null | null | 257 (Civil 3D's default style, no name stored) |
| companion references | null | null | null |
| registry | none | created, empty (BCA) | created, empty (BCA) |
| class number | 518 | 518 | 518 |

- Civil 3D builds its whole drawing infrastructure when it opens a plain
  drawing (both re-saves grew to the size of S65, with one alignment style,
  257).
- It regenerates the proxy graphics from the style on save. With no style
  there is nothing to draw (S69: 4 attribute chunks, no geometry), which is
  why the alignment was invisible. Civil 3D draws from the style and
  geometry, not from the stored graphics.
- AUDIT does not create a companion or registry entry; the companion
  references stay null, as after S30's AUDIT (S32). The two unnamed AUDIT
  errors are probably the two companion references, matching S30's two
  errors.

Conclusions: of all the references, **only the style is required**. The
companion/registry entry, the label group and Civil 3D's settings are
optional (Civil 3D creates the settings itself; AUDIT accepts null
companions). A truly minimal file therefore needs the alignment record and
one alignment style. The style could be our own minimal style record
(not Autodesk's template content, D-015) — not yet tried.

**S70** = S64 + `TEST STYLE` (the owner's style from S24, FA9, as 248) +
the alignment pointing at it (`exp_bare … FA9`, 29 KB). Classes added:
`AECC_ALIGNMENT_STYLE` (518) and `AECC_ALIGNMENT` (519). The style's
references were mapped to S64: layer 0 (10) and ByBlock (14) kept; its
two `DASHED2` linetype references (488, not in S64) → Continuous (16); its
owner (the style collection 48B) and the soft pointer to the default style
(48C) null, since S64 has neither. Question: does Civil 3D accept a style
without a collection (adopt it, AUDIT it, or remove it), and does the
alignment then draw and select normally?

**Result (S71 with AUDIT, S72 without).** S70 opens with no RECOVER prompt
and no visible errors, but the alignment shows the default style
(Standard, not TEST STYLE). AUDIT: 2 errors found, 2 fixed, none named. In
both re-saves **our style 248 is gone**; Civil 3D built its own collection
(257) with one style (258, the default):

- S72 (no AUDIT): the alignment still points at 248, which no longer exists.
- S71 (AUDIT): the alignment points at 258.

So Civil 3D did not keep an alignment style that has no owner. It
probably never loaded it, since nothing owns or reaches it. When the
reference turned out to be broken, Civil 3D fell back to the default
style. The two AUDIT errors are probably the companion references (as in
S30, S67), since the style fix is not reported as an error here.

**S73** = S70 with the style given an owner: a new dictionary
`CIVIL3DRUST_STYLES` (249) in the named object dictionary holds the
entry `TEST STYLE` → 248, and the style's owner reference points at 249.
Question: is an owned style kept and used, even though it is outside
Civil 3D's own style collection?

**Result.** S73 opens normally; AUDIT finds and fixes 2 errors (as in S70);
the alignment is shown with Standard, not TEST STYLE. So an owned style
outside Civil 3D's collection is not used either: Civil 3D accepts only
alignment styles from its own style collection (under its tree nodes).
(No re-save of S73 was kept.)

**Status: postponed (D-030).** A minimal drawing would need our style in
Civil 3D's own style collection, which means writing that part of Civil 3D's
infrastructure. Civil 3D adds its full infrastructure (~440 KB) on the
first save anyway, so for now new alignments go into drawings Civil 3D has
saved (S65/S66 approach).

**Results.**

- S66 (and S65): opened with no issues.
- S67: opens without a RECOVER prompt. The alignment is listed in
  Prospector but not drawn in the viewport. **Selecting it crashes Civil 3D**
  (unhandled access violation reading 0x0098), so a null style reference is
  not safe.
- AUDIT on S67 reports 3 errors and fixes 3; the one it names is
  `Alignment (Alignment Style) eNullObjectId → Standard`: it assigns a style
  called Standard, which means Civil 3D built its default settings and
  styles when it opened the plain drawing. After the fix the alignment shows
  as a dot until it is moved (and moved back); then it draws normally. So
  Civil 3D does not draw from the record's proxy graphics. It recomputes the
  display when the alignment changes, and the AUDIT fix alone does not
  trigger that.
- AutoCAD (without Civil 3D) and OpenCADStudio open S67 and draw the
  alignment from its proxy graphics.

Conclusions: the style reference is required (the only reference AUDIT
had to repair). The companion/registry entry and the rest of Civil 3D's
infrastructure are not required to open the file: Civil 3D creates the
infrastructure itself. The other two AUDIT errors are not named in its
output; the resaved files (with and without AUDIT) will show them.

## Design speeds (O2: S74, S75)

**Sample.** S74 (owner, Civil 3D 2024): three one-tangent alignments with
design speeds set in Civil 3D: `60MPH` (1063, 60 mph), `30MPH` (106A,
30 mph), `mixed speed` (1074: 60 mph from 0+00, 30 mph from 2+00).

**Steps.** Search of every bit offset for the doubles 60, 30, 200 (and
the metric equivalents); region comparison with S58 (same template, no
speed set); field walk (BL/BD) around the hits.

**Finding.** The design speeds are the start of the post-cache block:

```text
BL 0, BL 1, BL n,
n × { BL 1, BD start station, BD speed, 16-byte GUID },
BD 100, BL 0, BL m, m × BD (element IDs), BL alignment type, …
```

- Speeds are stored in the drawing's unit (mph here), as entered.
- Every alignment has at least one entry. `60MPH` is bit-identical to
  S58's record outside the geometry, caches and GUID: **60 at station 0 is
  Civil 3D's default in this template**, which is why the "BD 60" looked
  constant in S24–S73. The S1–S19 template's default is 100 (S2–S4).
- One entry carries the alignment's identity GUID: that's "GUID copy 2". In
  `mixed speed` it is the entry at 2+00, so it is probably the entry that
  existed first; the new entry at 0+00 got its own random v4 GUID.
- With two speeds, Civil 3D also changed the region right after the
  element records: the 16-bit field before GUID copy 1 ends `…0010`
  instead of `…0001` (probably BL 2 vs 1), and the fields up to the cache
  hold 200 and the remaining length 311.76, so the station cache is split
  at the speed change. This looks derived (interval cache) and is not
  decoded. The cache parser of `exp_minimal`/`create_alignment` does not
  read this form.
- S1 (LandXML import), S9 (station equation) and S57 X5 (zero GUID) have a
  different layout here; the reader does not find their table.

**Code.** `aecc_design_speed::{design_speeds, set_design_speeds}`
(rewrites the table, keeping the identity GUID on the first entry; the
record grows or shrinks), `examples/set_speeds`. Rewriting the same
speeds reproduces Civil 3D's bits exactly (test).

**Test.** S75 = S24 + `Speed 30` (one entry, 30) and `Speeds 60-30`
((0, 60), (200, 30)), created by `create_alignment` and edited by
`set_speeds`. The interval-cache region is left in its one-interval form,
so this also asks whether Civil 3D recomputes it.

**Result (S76).** No errors, and both speed lists are kept. But the new
two-speed alignment showed a garbage comment ("ڠ₢ꒀ樟ᾤ聪ᾤꁒ쀏") in the speed
grid, and the re-saved record has 13 strings, the last one that text.
**Each design-speed entry has a comment string** in the string stream.
Our record still had the one-entry count (12), so Civil 3D read the
missing comment from the bits before the stream. `set_design_speeds` now
adds or removes one empty string per entry. That works because all the
strings around them are empty; where the comments sit among the empty
strings is not known yet. Civil 3D left the interval field after the
elements at the one-interval value. So that field (BL 2 and 2 extra
strings in S74 `mixed speed`) is not caused by the speeds; it came from
some other edit in S74 and is still open.

S77 = S75 regenerated with the fix, plus `Speeds 50-40-30` ((0, 50),
(150, 40), (300, 30)).
Owner: S77 looks correct in Civil 3D.

**S78** = S74 with Civil 3D's own `mixed speed` (two entries) rewritten to
(0, 50), (150, 40), (300, 30). The first attempt failed to read back:
`mixed speed`'s entry GUIDs also occur in the interval block after the
elements, so finding the identity GUID by value picked the wrong entry.
`set_design_speeds` now reads the identity from copy 1 (16 bits after the
elements), gives it to the first entry, and reuses the other old GUIDs in
order before generating new ones. Owner: S78 works in Civil 3D.

## Design criteria (O3: S79, S80)

**Sample.** S79 (owner): S77 opened in Civil 3D, its two alignments
renamed `DESIGN ON` (FB3, was `Speed 30`; design criteria switched on)
and `DESING OFF` (FB6, was `Speeds 60-30`; criteria off, a comment
`COMMENT` typed in a speed row), saved. So S77 → S79 is a clean pair for
both records.

**Steps.** Region comparison against the same records in S77, the
section-2 bits of ON vs OFF, handle streams, string streams.

**Findings.**

- **Three flag bits** 19, 18 and 17 bits before the end of section 2:
  `111` in `DESIGN ON`, `000` in `DESING OFF` and every other sample (the
  bits after the start station BD: S9 has its own flag in the next bit).
  The rest of section 2 differs only in the 66-bit cache field and a
  31-bit field (bits 300–330 of the 589-bit form), both probably caches.
  Which bit is which check box is not separated (one sample with all on).
- **Strings**: with criteria on, five of the eight empty strings after the
  name are filled: the criteria file path, the same path as
  `<DATADIR>…`, and the three default criteria shown in the Design
  Criteria tab (S81 screenshot): Minimum Radius Table
  (`AASHTO 2011 US Customary eMax 4%`), Transition Length Table
  (`2 Lane`) and Attainment Method (`AASHTO 2011 Crowned Roadway`). These are names of
  Civil 3D's standard data and appear here as text only.
- **Design check set**: the last handle reference (null in alignments
  created from the S24 template) points at `AECC_ALIGNMENT_DESIGN_CHECK_SET`
  4D3 in both records, also in OFF. Civil 3D sets it on save, as in S74
  (`30MPH`, `mixed speed`), so it does not mean criteria are on.
- Both records got the edited section-2 form (413 → 589 bits) and the
  user-name string, as after other Civil 3D property edits.
- **Speed comments**: `DESING OFF` has 16 strings, the last one `COMMENT`:
  the speed comments are at the end of the string stream. Civil 3D also
  filled the interval block after the elements (625 → 1362 bits, the S74
  `mixed speed` form, +2 strings), so that block is derived from the
  speed table after all. Civil 3D accepted S77's records without it and
  rebuilt it when the alignment was edited.
- With criteria strings present, the name is no longer always string 3 or
  4 by the earlier rule; `raw::object_name` now uses the `<DATADIR>`
  string as an anchor.
- `aecc_build::alignment_type` failed on records with several design
  speeds (it read after the GUID copy 2 entry); it now reads after the
  whole table.

**Code.** `aecc_criteria::{criteria_flags, set_criteria_flags}`.

**Experiment S80** (`examples/exp_criteria`): S24 + four created
alignments: `C0 control`, `C1 flags` (flags only), `C2 checkset` (flags +
check set 4D3), `C3 full` (flags + check set + the five strings copied
from S79). Question: which of these Civil 3D shows as criteria-based
design, and whether flags without the strings cause errors.

**Result (S81, owner's re-save; `C0`, `C1`, `C3` were opened in
Alignment Properties, which adds the user-name string).**

- `C3 full` shows exactly S79's settings (screenshot): criteria-based
  design on, design criteria file on with the file and the three default
  criteria, design check set on (`Standard`), check for tangency on.
  **Flags + strings + check-set reference reproduce criteria-on.**
- `C2 checkset` (not opened): kept as written, flags `111`.
- `C1 flags` (opened): Civil 3D changed its bits from `111 0111` to
  `100 0101` (19…13 bits before the section-2 end): flag 1 kept, flags 2
  and 3 cleared, and the bit 14 before the end cleared too. C1 had no
  criteria file strings and no check set, so the likely mapping is:

  | bits before section-2 end | probable meaning | default |
  |---|---|---|
  | 19 | use criteria-based design | 0 |
  | 18 | use design criteria file | 0 |
  | 17 | use design check set | 0 |
  | 14 | check for tangency between elements | 1 |

  ("probable" → confirmed, see below.)

  **Confirmed** by the owner's screenshot of C1's Design Criteria tab:
  criteria-based design checked; design criteria file, design check set
  and check for tangency unchecked. `aecc_criteria::Criteria` names the
  four bits.
- `C0 control`: unchanged apart from the edited-record form; every
  alignment now references check set 4D3 (set on save).

## Start station (O4: S82)

**Sample.** S82 (owner): S81 edited in Civil 3D, C3 deleted, the others
renamed: `ZERO START` (FB3, was C0; start 0+00), `100 PLUS` (FB6, was
C1; reference point at the start (0, −200), reference station −1+00,
owner's screenshot) and `100 MINUS` (FB9, was C2; start +1+00). The
criteria bits of FB6/FB9 are the ones left from S80/S81 (C1, C2), not
part of O4.

**Findings.**

- The start station is the BD that ends 20 bits before the end of
  section 2, as found with S9: −100 in FB6, +100 in FB9 (section 2 grows
  by 64 bits from the `10` of 0.0). The reference point equal to the start
  point adds nothing else to section 2. The station reader required the
  criteria bits after the BD to be 0, so it failed on S79–S82; fixed.
- With a nonzero start station **both have no design speeds and no
  interval block**: after the element records come `BL 0, BL 0, BL 0,
  BL 0` (interval count 0, so no GUID copy 1 and no cache), then the
  design-speed table with `n = 0`, then `BD 100 …`. The string stream has
  3 strings fewer (5 empty strings after the name instead of 8). S9
  (start 1+00) and S1 (LandXML import) have exactly the same form, which
  explains why their layouts looked different.
- So the "identity GUID" is not an alignment identity at all: **copy 1 is
  the GUID of the first interval in the interval block, and copy 2 the
  GUID of the matching design-speed entry.** Each design speed has an
  interval (GUID, stations, cache) after the elements and an entry in the
  table, with the same GUID (S74 `mixed speed`: two intervals, two
  entries, GUIDs shared). The three strings are the speed's comment and
  the interval's two strings (S79 `DESING OFF`: 2 speeds → 8 + 3).
- **Changing the start station wipes the design speeds** (owner checked
  in Civil 3D: no speeds for both, in either direction).

**Code.** `aecc_station::{start_station, set_start_station}`;
`aecc_design_speed::clear_design_speeds` (writes the no-speed form);
`design_speeds` reads the empty form; `set_design_speeds` refuses to add
speeds to an alignment without any (the interval block is not written
yet); `examples/set_start` (`--clear-speeds`).

**Experiment S83**: S24 + `Start 100 speeds kept` (start 1+00, the
template's speed entry and interval left in place), `Start 100 no
speeds` and `Start -100 no speeds` (speeds cleared as Civil 3D does).
Question: are both forms accepted, and what does Civil 3D show for the
kept speed?

**Result (S84).** AUDIT: no errors. Civil 3D kept the speed of
`Start 100 speeds kept` and moved it to 1+00 (table 0:60 → 100:60), and
rewrote the first interval: its fields after the 13 constant bits are
three BDs, **station, distance from the start, length to the end of the
element** — S84: (100, 0, 500); S24: (0, 0, 88.20); S74 `mixed speed`:
(200, 200, 311.76). The other two kept their empty form.

**civil3drust behaviour (D-031):** changing the start station keeps the
design speeds and shifts their stations (and the first interval's
station) by the same amount; alignments without speeds stay without.
`aecc_station::move_start_station`, `set_start` without
`--clear-speeds`. S85: S24 + `Shift +100` (60 at 1+00), `Shift -100 two
speeds` (60 at −1+00, 30 at 1+00) and `Shift +250 three speeds` (50 at
2+50, 40 at 4+00, 30 at 5+50).
Result (S86): no AUDIT errors; Civil 3D kept all shifted speeds and did
not rewrite the intervals on a plain save.

### The interval block (S24, S74, S79, S84)

After the element records:

```text
BL 0, BL 0, BL 0, BL n,
n × { 16-byte GUID (the matching design-speed entry's),
      13 bits 1010000001010,
      BD station, BD distance from the element's start, BD length to the element end,
      BL 1, BL 0, BL ID of the next element (after-link),
      RD element start x, y, element end x, y, point x, y },
```

then the design-speed table. Every interval in the samples starts on a
tangent; `BL 1, BL 0` may encode the element kind, so curves need a
sample. Civil 3D orders the intervals by creation (S74: 2+00 first).
Each speed with an interval has three strings (comment + two), a speed
without an interval one.

`set_design_speeds` now writes the whole block from the geometry (one
interval per speed, table order), so it also adds speeds to alignments
that have none. It reproduces Civil 3D's records bit for bit for S79
`DESING OFF` (two intervals) and S84; S24 differs only by Civil 3D's
last-digit rounding at a fillet tangent point. Speeds on curves and
outside the alignment are refused.

S87 = S82 with speeds added: `ZERO START` 0:60 100:50 250:40 400:30
(had one), `100 PLUS` −100:40 150:25 and `100 MINUS` 100:55 (had none).
Result (S88): no AUDIT errors, all speeds shown as written. O4 done.

## Layout tools (O5: S89)

**Sample.** S89 (owner): an alignment drawn with the layout tools in the
S1–S19 template (read in failsafe mode: its `PRESSUREPARTERRORBLOCK`
3DSOLID is dropped by acadrust 0.6.5 — saving such a file loses it). Five
elements: 1 fixed tangent (two points), 2 fixed curve (three points),
3 a tangent from the curve's end to a point, 5 a free curve (radius),
4 a tangent; design speeds 60 at 0+00 and 30 at 2+85.63 (on curve 2).

**Findings.**

- **Floating tangent** (element 3): same 11-bit prefix and header as a
  tangent, but the BL after the header is **4** (fixed tangents: 0) and
  only the **end point** follows (2 RD). It starts where the previous
  element ends (tangent to curve 2). `TangentRecord::kind`
  (`KIND_FIXED` 0, `KIND_FLOATING` 4); the decoder fills in the start from
  the previous element. The BL after the header is thus the element's
  constraint/creation kind.
- Records are stored in ID order (1, 2, 3, 4, 5) even when the path order
  differs (1, 2, 3, 5, 4).
- **Interval on a curve** (the 2+85.63 speed): `BD 285.63, BD 84.6364
  (distance from the element's start — so for tangents too the second
  distance is from the element start, not the alignment start), BD
  84.628, BL 2, BL 1, BL 3 (next element)`, then 6 RD: the curve's start,
  a point on the circle 84.628 further on, and the point at the station.
  The third distance on a curve does not follow the tangent rule (length
  to the element's end would be 35.72); not decoded.
- civil3drust therefore writes intervals only for speeds on tangents;
  speeds on curves get a table entry and one string. A speed table
  without any interval (only curve speeds) is written as interval
  count 0 followed by the entries.

S90 = S24 + three alignments with a 90° curve of radius 80 (curve
2+20–3+45.66): `Curve speed` 0:60 260:30 (the second on the curve, no
interval), `Tangent speeds` 0:60 400:30 (control), `Only curve speed`
260:40 (no intervals at all).
Result (S91): no AUDIT errors, all speeds shown as written; on a plain
save Civil 3D keeps the records as they are (no intervals added).

## Adding and removing elements (O6: S92 → S93 …)

**Baseline (S93).** S92's generated `Edit base` (FB3: T1 A4 T2 A5 T3,
two free curves R 80) opened in Civil 3D, AUDIT, saved unchanged.
`hexview --diff S92 S93 --filter FB3`: the record grows 820 → 826 bytes,
ten differing regions, all explained:

- **Element records unchanged** apart from the arc flag bit (34 bits
  after each arc header): 0 → 1 for both arcs (open question 11). Our
  exact coordinates (220, 0), (300, −80), … are kept in the records.
- **Derived points recomputed in floating point.** Civil 3D recomputes
  the tangent end points from the fillets and writes its own rounding
  wherever a derived copy is stored: in the proxy graphics (T1 end
  220 → 220.00000000000003, 0 → 2.842e−14; T2 start y −79.99999999999999
  → −79.99999999999997, end y −220 → −219.99999999999997), in the station
  at the end of element 0 (220 → 220.00000000000003) and in the element-0
  cache (its end point, same two values). These are ulp differences and
  confirm the copies are derived.
- **Section 2**: the 66-bit cache field rewritten and the 44-bit extra
  field added (open question 1, as in S8, S51, S57).
- **Handle stream**: same references; Civil 3D pads the last byte with
  1-bits (we pad with 0).
- **Unchanged**: the interval GUID, the speed table, the string stream;
  no element-ID list was added (S51 got one after a geometry edit, so it
  marks edited geometry, not every save).

So S93 is a clean baseline: the edits S94–S97 are compared with S93, not
S92.

**S94: first tangent deleted.** S93 with the first element (T1,
0,0 → 220,0) deleted in Civil 3D. (The plan was to delete the first
curve; T1 went instead, which is step S96 of the plan done early.) Read
from `hexview --diff S93 S94 --filter FB3`: the record was rebuilt from
S93 plus the diff's regions (711 bytes, as Civil 3D wrote it) and decoded.
826 → 711 bytes:

- **The record is removed, nothing is renumbered.** Element count BL
  5 → 4; T1's record is gone; the others keep their IDs (2, 3, 4, 5) and
  storage order (T2 T3 A4 A5), so the first stored record is no longer
  the first in the chain and ID 1 is simply unused.
- **Only the neighbour's link changes**: A4 before 1 → 0.
- **The orphaned free curve becomes a floating curve.** A4's constraint
  code `n2` goes 29 → 15 and the record grows by two raw doubles after the
  radius BD: the curve's **start point** (220.00000000000003,
  2.842e−14 — its old PC). Code 15 = fixed start point + radius, ending
  tangent to the next element. The record stores no direction; of the two
  possible curves (90° clockwise and 270°) civil3drust takes the shorter
  (`curve_into_line`), which is the one Civil 3D shows.
- **`p1` is the design speed at the curve, not a layout marker.** It
  went −1 → 60 on both arcs (the alignment's 60 mph). S89 confirms:
  curve 2 has 60 (speed 60 from 0+00), curve 5 has **30** (after the
  30 mph change at 2+85.63); S10's 60 is its template's default speed.
  −1 = not computed (curves made from a polyline, S11–S93); Civil 3D
  fills it in when it re-solves the curves after an edit.
- **Stationing is kept.** The start station becomes 220.00000000000003
  (`00` + raw double, 20 bits before the end of section 2), so the
  remaining elements keep their stations (the new start is 2+20). Five
  bits before that BD differ as well (`00110` → `10010`; not decoded).
- **Interval block**: both GUID copies (interval and speed entry) get new
  values, but the interval itself is left stale — station 0, length to
  the element end 220, next element 4, points (0, 0) → (220, 0): it still
  describes the deleted tangent.
- The 16 bits `1010100100000001` before GUID copy 1 are the interval
  block's own header: `BL 0, BL 0, BL 0, BL n` with n = 1 (open issue 6
  answered; already listed as the block's first fields).
- Graphics: T1's polyline and one attribute triple removed (348 → 252
  bytes); section-2 cache field rewritten; no element-ID list added.

**In Civil 3D (owner, S94).** Alignment Properties shows the start
station 2+20 and the 60 mph design speed still at 0+00, before the start.
Editing that speed fails validation (“0+00.01 out of range — must be
between 2+20.00 and 8+31.33”; 8+31.33 = 220 + 611.33 confirms the decoded
length) and leaves Civil 3D unstable; left alone, the drawing works.
So the stale speed and interval are a Civil 3D inconsistency, not a form
to copy: when civil3drust removes elements it should move speeds that
fall before the new start onto the start and rebuild the intervals
(`set_design_speeds` already writes them from the geometry). The record
rebuilt from S93 + the diff is byte-identical to S94's (test
`s94_first_tangent_deleted`).

**Decoder.** `aecc_elements` now chooses the arc layout by `n2` (29
free, 15 floating with start point, 2 three-point), no longer by `p1`
(S94's free arc A5 with `p1` 60 would have been read as a three-point
arc), resolves floating curves (`ArcDef::Floating`, `curve_into_line`),
and reads the start station before the first record in **storage**
order. S94's `Edit base` decodes as A4 (floating) T2 A5 T3, 611.33 long,
start station 220.

**S95: a curve between two tangents deleted.** S94 with A5 (between
T2 and T3) deleted. Read from the diff (record rebuilt from S94 + the
report; 288 bits of the new graphics were cut off by the report, so only
the graphics are incomplete), 711 → 730 bytes:

- A5's record is removed (count 4 → 3); T2 after 5 → 3 and T3 before
  5 → 2 — the two tangents are linked directly and **keep their stored
  end points**, which still meet at the PI (300, −300). No tangent is
  rewritten or merged; nothing is renumbered.
- The string stream loses the arc's three empty strings (18 → 15).
- A4 (floating) is untouched, `p1` stays 60; the interval block and the
  speed table are unchanged (still the stale 0+00 interval).
- Section-2 cache rewritten; graphics regenerated (252 → 316 bytes,
  12 → 10 chunks; not decoded, derived).
- Decodes as A4 T2 T3, length 645.66 (= 125.66 + 220 + 300).

So removing an element is: drop its record and its strings, relink the
neighbours, fix the count — exactly the reverse of what `set_geometry`
writes for a fillet.

The record rebuilt from the report matches S95's except for the 288
graphics bits the old report cut off (bytes 168–199).

**S96: free curve added back.** S95 with a free curve R 80 added at the
PI (300, −300). The record is S94's again apart from:

- the new curve gets **ID 5** and is stored **last** (T2 T3 A4 A5), its
  neighbours relinked to it (T2 after 3 → 5, T3 before 2 → 5), count
  3 → 4, three empty strings back (15 → 18). ID 5 is both the deleted
  ID and the highest existing ID + 1 (IDs 2, 3, 4), so the sample can't
  tell those apart; it is not the lowest free ID (1).
- the new arc has flag bit 0 and `p1` −1, as Civil 3D's own fillets
  made from a polyline (S11): flag and design speed are filled in only
  when Civil 3D re-solves the curve (S93, S94).
- graphics regenerated (416 bytes, 11 chunks; derived).
- **the section-2 66-bit field equals S94's** bit for bit, although it
  changed in S94 → S95: it is computed from the geometry, not a random or
  per-save value (consistent with open question 12, S12 = S14).

**Rules for element edits** (S94–S96): keep every other record and its
ID; the storage order is creation order (new records go last); a new
element takes an unused ID (highest + 1 is safe); relink the neighbours;
element count and string count (3 per arc) follow; a free curve that
loses a tangent becomes a floating curve (code 15) with its old PC as
start point; deleting the first element keeps stationing (start station
= old station of the new start). Civil 3D leaves the design-speed
interval stale; civil3drust should rebuild it.

**First-element ID in section 2.** Comparing the 60 bits around the
start station in every sample: the ID of the path's first element is
stored **twice** in section 2 — as a one-byte BL followed by `10` right
before the start-station BD, and as the BL that **ends** section 2
(the last 10 bits). Both are 1 in every sample (tangent 1 first) except
S94–S96, where both are 4 after T1 was deleted. (The "bit 4 after the
start-station field" of open issue 5 is the low bit of this BL.)
Test `first_element_fields_name_the_chain_head`.

**civil3drust: `aecc_element_edit` (D-036).** `remove_element` and
`add_free_curve` edit the element list in place following the rules
above, keeping the other records bit for bit (only links and, where
needed, a tangent's start/end point or a curve's constraint change),
then rebuild the derived parts: graphics, start station and both
first-element IDs, design-speed intervals (speeds before the new start
move onto it, speeds past the end are dropped), and they drop the
optional element-ID list. Replayed on Civil 3D's own files:

- S93 − T1 gives S94's chain, floating curve, start station and
  first-element IDs (records differ only in `p1`, which Civil 3D sets to
  60 when it re-solves, and last-digit rounding);
- S94 − A5 gives **S95's element list bit for bit**;
- S95 + free curve after T2 gives **S96's element list bit for bit**
  (ID 5, stored last, flag 0, `p1` −1).

Supported: removing the first element, the last element (when the one
before is a tangent, a three-point curve or a free curve — S99), a free
curve between two tangents, and a tangent between two free curves (S100);
adding a free curve between two tangents that meet at a stored PI.
Refused: other interior tangents and removals that leave a floating
curve without the element it is tangent to (no sample of the form).

S97 = S24 + four alignments edited this way (`Remove first` with speeds
0:60 300:40, `Remove curve`, `Add curve`, `Chain` = S94–S96 replayed).

**Result (S98).** The owner opened S97, ran AUDIT and checked all four
alignments: geometry as written, start stations as written (2+20 for
`Remove first` and `Chain`), design speeds shown as written (220:60 300:40,
0:60, 0:60, 220:60) and **editable** — unlike Civil 3D's own S94, whose
stale speed failed validation. On save Civil 3D kept every element record
apart from the arc flag bit (0 → 1 on all our curves, as in S92 → S93);
`p1` stayed −1 (no re-solve on a plain save) and the speeds were kept.
The owner also turned on `TEST STYLE`'s curve component: the style's
component flags go `10000…` → `11000…`, so **component 1 is the curve
line** (component 0, the tangent line, from S21 → S22).

**S99: last tangent after a free curve deleted** (S98, `Remove first`,
T3). The free curve A5 becomes constraint **13**: radius plus its **end
point** (two raw doubles after the radius, here its old PT (380, −300)),
starting tangent to the previous element — the mirror of code 15. Its
after-link becomes 0; flag and `p1` are kept; count 4 → 3, the
string stream unchanged in arcs. Speeds (220:60 300:40, both on curves,
no intervals) unchanged.

**S100: tangent between two free curves deleted** (S98, `Add curve`,
T2). Both curves become one-sided: A4 → code 13 with its old PT
(300, −880), after 0; A5 → code 15 with its old PC (300, −1020), before
0. Nothing joins them: the alignment now has **two pieces with a gap**
(T1 A4 | A5 T3), two records with no predecessor. The first-element IDs
in section 2 stay 1. The interval and speed (0:60 on T1) are unchanged.
**Civil 3D treats only the chain from the first element as the
alignment** (owner, S100): stations run 0+00 to 3+45.66 on T1 A4 and stop
there; A5 T3 is drawn but has no stations and is ignored by every
operation — a placeholder until it is joined again.

**Decoder.** Code 13 is read as `ArcDef::FloatingEnd` and resolved as
the curve leaving the previous tangent's line and ending at the point.
`chain_pieces` orders the records into connected pieces; the one starting
at the first-element ID is the alignment (`DecodedElements::elements`:
length, stations, speeds), the others are kept as
`DecodedElements::detached` — read, drawn and preserved, but not part of
the alignment, as in Civil 3D.

**`remove_element`** now also removes the last tangent after a free
curve (S99 form) and an interior tangent between two free curves (S100
form: the rest becomes a detached piece; speeds past the new end are
dropped). Both replays reproduce Civil 3D's element lists **bit for bit**
(tests `s99_…`, `s100_…`). On an alignment with detached pieces the
alignment itself can still be edited and the detached records are kept
as stored; elements of a detached piece can't be edited.

## Sites (O7: S101–S103)

**Samples** (owner): S101 = S98 with the four generated alignments
deleted, `Alignment - (1)` renamed `TEST`, and an empty site `TEST SITE`
(description on two lines); S102 = `TEST` moved into the site; S103 = a
second, empty site `TEST2` added. Compared with `hexview --diff`.

**Creating a site (S98 → S101)** adds two records (plus bookkeeping
xrecords and class/attribute nodes):

- the **site**, class `AECC_PARCEL` (C++ `AeccDbFace`; first string
  `AeccImpFace`): strings `AeccImpFace`, `""`, name, description,
  created-by; handle stream: four hard pointers to layer 0 (10), a null,
  the parcel node BBC, a **hard owner → its container**, then the parcel
  style 8DD ×3 with a null. So a site is a *face* of Civil 3D's topology
  model, the same class as parcels.
- the site's **container**, class `AECC_FEATURE_STYLE` like the alignment
  companions (48 bytes empty): `own−1` → the site, hard pointer → 851.

The **sites collection** is `AECC_PARCEL_NODE` (BBC): data `BL 0, BL n,
n × RL key` (high bit set, as in the alignment registry; S103 keys
`…BDA` for TEST2, `…BD7` for TEST SITE), strings `""` + the names, handle
stream: tree node 252, the `n` sites in the order of the names, one more
site pointer (the most recently created site in both samples), then the
three label-collector styles. Names are ordered `TEST2`, `TEST SITE`
(S103): either by name ignoring spaces or by key, descending; not decided.

**Deleting alignments (S98 → S101)** keeps their companions in the
alignment registry BC0, emptied to 117 bytes: the object header, a short
body, the ID string and only the `own−off` reference to the registry.

**Moving an alignment into a site (S101 → S102):**

- the alignment record's data is unchanged; only its two companion
  references (handle stream 2 and 3) change from the companion F9D to the
  site's container 100E;
- the old companion F9D is **emptied** (224 → 117 bytes, the form of a
  deleted alignment's companion) but stays in the registry with its ID
  string and key;
- the container 100E grows 48 → 176 bytes: the companion's body (the
  geometry points, matched run for run against F9D's old data) is
  inserted, and its handle stream gains soft pointers to the alignment,
  a null, the site, a null (before the 851 pointer).
- the site record gains 180 bits after its graphics (not decoded).

S103's new site changes nothing in the existing ones except the
collection (BBC: new key, name, pointer; the "most recent" pointer
moves to TEST2).

**civil3drust (read-only).** `aecc_site::sites` lists the sites (handle,
name, description, container) from the collection;
`aecc_site::membership` tells whether an alignment is siteless (its
companion) or in a site (site and container) by following its companion
references; `examples/sites` prints both. `create_alignment` no longer
takes a donor alignment that is in a site (its references lead to a
container, not a companion of its own).

**Several alignments, moving between sites (S104–S106).**

- **One container per site holds all its alignments** (S104: TEST and
  `CREATED to TEST SITE` both point at 100E). Its handle stream is
  `own−1` site, then per alignment `soft → alignment, null`, then
  `soft → site, null`, then the hard pointer 851. Its data grows in three
  places per alignment (+637 bits for the two-tangent alignment): after
  the header (+147 bits), a body with the geometry (+294) and a tail
  section (+196), so it is a list-of-lists structure — consistent with a
  topology graph (nodes, edges, faces), which is what a site is: the
  alignments in it split it into parcels.
- **Moving `TEST` to `TEST 2` (S105):** its data leaves 100E (back to the
  145-byte one-alignment form, now for `CREATED`) and appears in 1069
  (48 → 176 bytes, the same form as 100E in S102).
- **Moving it back to `<None>` (S106):** Civil 3D creates a **new
  companion** (108E, new key `…BDD`, new ID string, a new registry entry
  in BC0) and does not reuse the emptied F9D, which stays as it is. The
  `TEST 2` container does not return to its empty 48-byte form: it keeps
  `soft → site, null` and 11 more bytes (59 bytes).
- The emptied companions shrank again 117 → 115 bytes between S103 and
  S104 (a re-save; not decoded).

**Conclusion for writing.** Putting an alignment into a site means
writing the site's topology graph, which is not decoded (only the
geometry points in it are). civil3drust therefore creates **siteless**
alignments only, also in drawings that have sites (S107, D-038); moving
an alignment into a site stays a Civil 3D operation for now. Reading
sites and membership works for every sample (tests `site_samples`).

S107 = S104 + a siteless alignment `SITELESS NEW` made by
`create_alignment`, to confirm that sites in the target don't disturb
alignment creation.

## Open questions

1. The volatile field in section 2 (S5/S6 bits 2730–2789): variable length
   (62 bits in S5–S7, 106 after a Civil 3D re-save in S8; +44 bits also in
   S3→S4); what it holds.
9. The 2-bit flag at bit 2677 that Civil 3D changed when re-saving S7.
10. ~~The block of BDs (2.0, 3.0, 4.0) in S12~~ — answered by S51: the
    element IDs after the first, added by Civil 3D on save; optional.
11. The flag bit after the arc header (1 in S10, S14, S15; 0 in S11–S13).
    Civil 3D set it after geometry edits it saved (S14, S15) but not after
    its own radius edit (S12); it also sets it when re-saving our
    generated free arcs unchanged (S92 → S93).
12. The volatile header field: identical in S12 and S14 (same content,
    different save sessions), so probably computed from the content.
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
7. ~~Arc element records~~ — answered by S10 (arc by three points) and S11
   (free arc by radius between tangents). Still open: the meaning of the
   parameter block (p1 60 vs −1, n2 2 vs 29), other free/floating variants
   (by length, through a point), and spirals.
8. How offset alignments store their geometry (no tangent records).

## Next samples

**O6: adding and removing elements (current).** Each step is a Save As
from the previous file in Civil 3D 2024, compared with `hexview --diff`:

| Sample | From | Change in Civil 3D |
|---|---|---|
| S92 | S24 | generated: `Edit base` (FB3), 0,0 → 300,0 (R 80) → 300,−300 (R 80) → 600,−300; T1 A4 T2 A5 T3 |
| S93 | S92 | opened (AUDIT), saved unchanged — the baseline |
| S94 | S93 | first tangent (T1) deleted — done |
| S95 | S94 | second curve (A5) deleted — done |
| S96 | S95 | free curve R 80 added back at that PI — done |
| S97 | S24 | generated: civil3drust's element edits (see below) |
| S98 | S97 | opened in Civil 3D (AUDIT), saved — done, all accepted |
| later | S96 | floating curve added at the end, from the last tangent |

Questions: new element IDs (reused or max + 1), the element-0 cache when
element 0 goes, the ID-list block, the three strings per arc, and the
floating-curve kind and parameters (open issue 2).

For alignment creation (each a copy of S24 with one change, saved the
same way):

- **A**: ~~S24 plus a second alignment from a polyline~~ — done, S25.
- **B**: ~~as A, but with a different alignment style~~ — the style
  reference is answered by S18/S19.
- **C**: as A, but with the "No Labels" label set — whether label-group
  objects are required.

For style records: pairs saved from the **same** style edited in place
(one property changed, Save As in between), so a bit diff isolates the
property. Done: line visible (S21 → S22). Next: line colour, line layer,
line linetype, then the spiral component visible (to confirm component 2;
1 is confirmed by S98), each from the previous file.

For styles (D-015): the owner creates **own** minimal styles in Civil 3D with
*New* (not *Copy*), own names, generic values — e.g. alignment style
`CR-Empty` with every component off, label set `CR-NoLabels` with no
labels. These are the owner's output and the baseline for decoding style
records. Then one change per sample (line component on, its colour, its
layer, arc component on, …), each compared with the previous. Sample B can
use `CR-Empty` as its "different style", which also locates the style
handle in the alignment record.

- A template with two tangents and one free curve (e.g. 0,0 → 100,0 →
  100,100, R = 50), for exact generation of that shape.

- S10 with the radius changed (e.g. 100 → 150) in Civil 3D, to see which
  fields an arc edit touches.

## Template note

S1 and S5 contain a `3DSOLID` in Civil 3D's internal block
`PRESSUREPARTERRORBLOCK` (handle 7F72 in both) that acadrust 0.6.5 cannot
parse. Reading falls back to failsafe mode and the solid is dropped, so a
copy written by acadrust would lose it. S2 (a different, smaller drawing)
does not have it. A lossless round trip of drawings from this template needs
that acadrust issue fixed first.

Update: acadrust 0.6.7 reads this solid; all seven affected samples (S1, S5,
S6, S11, S12, S15, S89) now load without failsafe mode and keep it after a
save (docs/acadrust-0.6.7.md).

## Provenance

Every offset and value above was produced with this repository's tools on
acadrust's record bytes. An earlier exploratory pass (before this repository
existed) used LibreDWG's `dwgread` purely as a byte dumper; LibreDWG has no
knowledge of AECC classes, no LibreDWG code or documentation was used, and
all findings here were re-derived with acadrust-based tools.
