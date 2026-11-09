# AECC_ALIGNMENT record

Status: **decoded for everything sampled.** Every line, curve and
spiral layout, design speeds, start station, criteria and constraint
options, type, style, names, companion and registry are read and written,
and Civil 3D accepts civil3drust's alignments (D-049 acceptance from S188
on). The current record map is [`alignment-map.md`](alignment-map.md);
open fields are [`mysteries.md`](mysteries.md).

This file is the lab notebook behind those summaries, in the order the
samples were made. Where a later sample corrected an earlier reading, the
earlier text now says so (**Later:** …) or has been replaced; names used
early on ("volatile field", "identity GUID", "station-reference cache",
"open question n") are mapped to their current names where they occur.

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

Later samples (S58 on) are listed only in
[`samples/README.md`](../samples/README.md).

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

**Later:** sections 0, 2 and 6 are decoded (see "Handle stream",
"Section 2 revisited" and `alignment-map.md`); what this table calls the
station-reference cache is the design-speed interval block ("The interval
block", S82–S84).

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
header           code 00, 0x0008, flag byte 0x02 when this element's
                 parameters are locked (absent when not; S164, S172),
                 type byte 0x04 (0x0D: first spiral of a group stored
                 apart, S177) — read as the BL 0x04020008 in the locked
                 form (MIST7)
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
same 11 bits (`00000000010`): the kind prefix of the next record (2 =
line, 4 = curve, …; "Element list, cache and graphics"). After the last
record comes the speed-interval block ("The interval block").

In S2 the alignment runs (100,0)→(0,0), i.e. opposite to the drawn polyline;
Civil 3D chose the direction when the alignment was created.

**Check against a production drawing** (not shared, not in the repo):
`examples/alignments` decoded all 21 of its tangent-only alignments,
including ones with 3, 7, 11, 12 and 262 tangents, with the same geometry as
an independent extraction from the display cache. Alignments containing arcs
fail cleanly (a tangent's `after` points to an element with no tangent
record, i.e. the arc uses a different record type), and an offset alignment
has no tangent element records at all. (At the time; curves and spirals
are decoded since, offset alignments are still not sampled.)

### 4. Station-reference cache

In S2 one record: the tangent again, followed by the point (100,0) — the
alignment start. In S5 one record: tangent 1, followed by (0,0) — again the
alignment start. S1 has none. These records have no element header, so the
decoder never mistakes them for tangents.

**Later (S74, S82, S84):** these are not a station cache but the
**design-speed intervals**, one per design speed: the element the speed
starts on, its start and end, and the point at the speed's station
("The interval block"). S1 (and S9) have no design speed, hence none.
The hundreds of records on some production alignments are their many
design-speed entries' intervals or another block; not checked.

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
structure. S3 opened in Civil 3D with this field left stale, so it is not
checked on load. **Later:** this is MIST1, Civil 3D's geometry fingerprint
(checked only for alignments with a JSON layout, S214), and the 44-bit
growth in S4 is MIST2, the field Civil 3D adds on re-save (`mysteries.md`).

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
  record: 2 bits at bit 2677 and the volatile field, which grew from 62 to
  106 bits (record 706 → 711 bytes), the same +44-bit growth as S3 → S4.
  Everything after it is unchanged apart from the shift. (**Later:** the
  2 bits are the MIST2 format marker, `0x1148` → `0x1188`, and the 44 bits
  are MIST2 appended after MIST1; MIST5.)
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
| Flag area near bit 2676 (the section-2 form BL: Alignment Properties form, MIST4) | 5 bits | 39 bits |
| Volatile field (MIST1, then the date-modified block MIST3 and MIST2) | 65 bits | 163 bits |
| **Start station** (bit-coded double, BD) | bit 2995: code `10` = 0.0 (2 bits) | bit 3127: code `00` + raw double **100.0** (66 bits) |
| Speed interval and the speed table after it (100, 100, 2.25, 60) | present (one interval, tangent 1, point (0,0)) | **gone** (a start-station change wipes the design speeds, S82) |
| String stream | `AeccImpAlignment`, `Alignment - (2)` | `AeccImpAlignment`, **`felix`**, `Alignment - (2)` |

Tangent element records are unchanged apart from the shift.

**Location of the start station.** The BD ends exactly 41 bits before the
first tangent's header in every sample: S1, S2, S4–S9, and 24 of the 25
tangent alignments in the production drawing. Two of those read 1000.0 and
2000.0, matching their 10+00 and 20+00 stationing; the rest read 0.0. The
first 17 of the 41 bits (`0000?111000100000`, `?` varies) are identical in
all of them, so `aecc_alignment::start_station` checks them before trusting
the value (bit 16 excepted since S256's offset alignments set it). The exception is an alignment made from a feature line with 262
tangents, where the gap differs; there the station is reported as unknown.
The rest of the 41 bits holds the design-criteria and constraint-editing
flags (the last 20 bits of section 2; "Design criteria"), then the
element count BL (10 bits) and the first record's 11-bit kind prefix.

**Speeds wiped.** Civil 3D dropped the interval record when the stationing
changed: changing the start station removes the design speeds (S82).

**User string.** After the edit, the user's login name appears in the
string stream after the descriptions, before the alignment name. It
belongs to the Alignment Properties form, which Civil 3D writes after any
edit in that dialog ("The user string of the Alignment Properties form",
S227); the name reader follows the section-2 form (`raw::section2_form`).

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
not stored as such. (157.0796 appears later in the record as the speed
interval's length to the element's end.)

**Parameters.** The five values between the header and the IDs were
identical (60, −1, 1, −1, 2) in S10 and in all six arcs of a production
drawing. **Later:** p1 is the design speed at the curve (60, the
template's default; −1 until solved), p2 and the n1 BDs are criteria
values (−1 without criteria, MIST9), and the last BL (2) is the curve's
constraint code n2 (three points; `element-methods.md`).

**Check against a production drawing** (local only): `examples/alignments`
decoded all four alignments with curves — three tangent–arc–tangent curb
returns (element 2 is the arc, linked 1 ↔ 2 ↔ 3) and a three-arc
cul-de-sac (35 / 40.01 / 35 ft). Radii, deltas, lengths and stationing
match the earlier extraction from the display cache. One offset alignment
still has no element records.

**Tangents vs arcs.** Both start with the same header. After it, a tangent
has its kind BL (0 here) followed directly by its IDs; an arc has the
parameter block. The 11-bit kind prefix before the header (2 line, 4 arc)
tells them apart ("Element list, cache and graphics").

## Free arcs (fillets): S5 → S11

Adding a 50 ft curve at PI (100,0) in Civil 3D:

- **Tangents keep their PI-to-PI coordinates.** Tangent 1 is still
  (0,0)→(100,0) and tangent 2 still (100,0)→(200,50); only their links
  change (tangent 1 `after` 2 → 4, tangent 2 `before` 1 → 4).
- **The arc is element 4**, stored *after* tangent 3 but linked 1 → 4 → 2.
- **Its record is a second arc kind** — same fields up to the links, but:

  | Field | Arc by 3 points (S10) | Free arc (S11) |
  |-------|----------------------|----------------|
  | B after header | 1 | 0 (1 after a Civil 3D re-save, S14: MIST8, set on load) |
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
described in Autodesk's public Civil 3D documentation. n2 is the curve's
constraint code (2 three points, 29 free fillet by radius; all codes in
`element-methods.md`); the flag is MIST8.

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

The 2.0 / 3.0 / 4.0 block is the element-ID list Civil 3D adds after
geometry edits (S51: the IDs after the first); optional.

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
volatile field to the same value as in S12 — derived from the content
(MIST1, the geometry fingerprint).

**The bit after the arc header is not the arc kind.** It is 1 in S10 (arc by
three points) and in S14 (still a free arc: p1 = −1, n2 = 29, radius only),
and 0 in S11–S13. It is set by Civil 3D on curves it read from the file
(MIST8, S207–S210); the layout comes from n2 (D-012, D-035).

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
station 0; p1 = 60 in arcs by points is that design speed (MIST9).

**Generated edit (S16).** `examples/set_length S11 S16 8C5C 3 200` now
handles alignments with free arcs: the tangent records form a PI-to-PI
polyline, so the PI edit from D-003 applies unchanged; free arcs follow,
and cached PC stations are updated when they move (none here). It patched
tangent 3 and the placeholder box. `rawdiff S16 S15 AECC_ALIGNMENT` leaves
only the volatile field and the arc's flag bit. Re-running the S5 edit with
the new code reproduces S7 byte for byte.

## Generating an alignment from a template: S17

At this point civil3drust could not create an `AECC_ALIGNMENT` record
(creation came with S26, any element list with S45 and S92 on). It could
rewrite a record whose *shape* matches: the same number of tangents with
free curves in the same places.
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

An exact three-PI version needed a template with two tangents and one
free curve, or element removal; both exist since (`set_geometry`, S45;
"Adding and removing elements").

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
drawing (8C5B, and 37FD in S11/S18). Creating an alignment means creating
a companion and adding it to that list (confirmed by S25).

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
parsing or style pairs that share a handle length *and* name length.
(Solved differently: display components are found by a marker run, next
section, D-020.)

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

So creating an alignment needs the alignment record, its companion and
the registry entry; the label group turned out to be optional and the
name counter and settings can stay untouched (S26–S29).

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
  own alignments on every re-save — the MIST2 format marker and MIST2
  itself (the "2-bit flag" and "+44-bit volatile field") — plus
  regenerated display graphics. `Alignment - (1)` got the identical treatment.
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
those values were not refreshed, and AUDIT was clean. (civil3drust now
rebuilds the companion from the alignment's geometry, D-040,
`aecc_topology::refresh_companion`.)

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
twice** (empty unless set), the **name**, then empty strings (their count
follows D-056's rule). In S9 and S22, after edits made in Civil 3D, an
extra string — the user name `felix` — sits before the name: the
Alignment Properties form's user string (S227); with a JSON layout the
JSON comes before the name too.

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
- **After the last record:** 16 bits, then a version-4 GUID in
  little-endian GUID layout (version nibble 4 in byte 7, variant `10` in
  byte 8 in every Civil 3D value); the same value is stored again ~717
  bits later; stable through edits and most re-saves, but in S47 Civil 3D
  gave **both** alignments new GUIDs (S45's generated value had version
  nibble 5 — fixed since) — then bits up to 161 after the records, the BD
  station at the end of element 0, and the **element-0 cache**: BL 1,
  BL 0, BL id of the element after element 0, then six raw doubles —
  element 0's start, its end (the PC when a curve follows), and the start
  again. **Later (S82, S84, S115):** this is the first design-speed
  interval, not an identity: the GUID is the speed's (shared with its
  table entry), the "station at the end of element 0" is the interval's
  length to the element's end, and the BLs are the containing element's
  ID, before and after IDs ("The interval block").

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
  stream. (The full rule, from S184–S186, is D-056: 9 + 3 per speed with
  an interval + 1 per speed without + 3 per non-line element record + 1
  for a JSON layout.)
- in the graphics, after each arc Civil 3D repeats the attribute triple
  (opcodes 16, 18, 14 — copies of leading chunks 4–6) even though the
  lines-only style draws no arc (also in S24's `Alignment - (1)`).

`set_geometry` then set the string count to `donor − 3·donor arcs + 3·new
arcs` (since replaced by D-056's rule) and writes the triple after each
arc. S50 (S46 regenerated) equals S49's record in elements, strings and
graphics, except the speed GUID, MIST1 (then copied from the donor; zeroed
since, D-046) and the
last bit of the PC station (Civil 3D 88.1966011250105, us
88.19660112501052 — S24's own alignment, saved by Civil 3D, holds the
latter, so it isn't validated). Test `curves_match_civil3d_s49`.

**Result (S51).** S50 opens with AUDIT 0 errors; re-saved, `C` keeps its
six elements, radii, stationing and its generated speed GUID. Civil 3D
changed the usual re-save fields (the MIST2 marker, MIST2, the curve flag
bit MIST8) and **added a block of BDs 2, 3, 4, 5, 6 — the element IDs
after the first**. This is the "2.0, 3.0, 4.0" block of S12 (IDs 2–4 of a
four-element alignment): an ID list Civil 3D adds after geometry edits;
optional, as S50 shows.

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
| n × BD | — | — | 1.0, 2.0, …, n (the element-ID list, S51) |
| **BL type** | **1** | **4** | 1 in every other record |
| BL, BL, BL | 2, 2, 2 | 2, 2, 2 | 2, 2, 2 |
| BL, BL | 0, 1 | 0, 3 | 0, 1 (S2–S15) or 0, 3 (S24 template) |

So **type 1 = Centerline, 4 = Miscellaneous**. This is the only difference
in the post-cache block. Values 1–255 are stored as `01` + one byte, so the
type can be changed in place (`aecc_build::set_alignment_type`;
`create_alignment --type`). Values 2, 3 and 5 are presumably the other
types Civil 3D offers (Offset, Curb Return, Rail), but no sample confirms
them yet (S61).

**Other S58 → S59 differences** (side effects of editing in Alignment
Properties):

- section 2 grew 413 → 631 bits: MIST2 (44) and the Alignment Properties
  form with its date-modified block (MIST3, MIST4).
- the Properties form's user string ("felix") appears before the name.
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
- Each entry has its own GUID, shared with its interval after the
  elements (S82; the "GUID copy 1/2" of earlier sections).
- With two speeds, Civil 3D also changed the region right after the
  element records: the interval count is 2 (`…0010` instead of `…0001`),
  and the second interval holds 200 and the remaining length 311.76 —
  one interval per speed ("The interval block", decoded with S84).
- S1 (LandXML import) and S9 (start station 1+00) have no speeds and so
  the empty form (S82); S57 X5 has the zero-GUID form (MIST13).

**Code.** `aecc_design_speed::{design_speeds, set_design_speeds}`
(rewrites the table and, since S84, the intervals; the record grows or
shrinks), `examples/set_speeds`. Rewriting the same speeds reproduces
Civil 3D's bits exactly (test).

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
adds or removes one empty string per entry. The comments are the last
strings of the stream (S79). Civil 3D left the interval block at one
interval; it rebuilds the block when the alignment is edited (S79), and
civil3drust writes it since S84.

S77 = S75 regenerated with the fix, plus `Speeds 50-40-30` ((0, 50),
(150, 40), (300, 30)).
Owner: S77 looks correct in Civil 3D.

**S78** = S74 with Civil 3D's own `mixed speed` (two entries) rewritten to
(0, 50), (150, 40), (300, 30). The first attempt failed to read back:
`mixed speed`'s entry GUIDs also occur in the interval block after the
elements, so finding a GUID by value picked the wrong entry.
`set_design_speeds` then read the first interval's GUID (16 bits after
the elements), gave it to the first entry, and reused the other old GUIDs
in order before generating new ones. Owner: S78 works in Civil 3D.

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
  The rest of section 2 differs only in MIST1 and the date-modified
  stamp of the Properties form (MIST3). Which bit is which check box was
  separated by S81 (below).
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
- The criteria strings come after the name: the criteria path's
  `<DATADIR>` form marks the name's position (`raw::object_name` uses it
  first, then the section-2 form: string 3 in the plain form, 4 in the
  Properties form).
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
  criteria file strings and no check set. The owner's screenshot of C1's
  Design Criteria tab (criteria-based design checked; design criteria
  file, design check set and check for tangency unchecked) gives:

  | bits before section-2 end | meaning | default |
  |---|---|---|
  | 19 | use criteria-based design | 0 |
  | 18 | use design criteria file | 0 |
  | 17 | use design check set | 0 |
  | 14 | check for tangency between elements | 1 |

  `aecc_criteria::Criteria` names the four bits.

  Two Constraint Editing options sit between them (MIST15, solved):

  | bits before section-2 end | meaning | default | samples |
  |---|---|---|---|
  | 16 | always perform implied tangency constraint swapping | 0 in S24, 1 in S11 | S163 |
  | 15 | lock all parameter constraints: the lock new elements get; turning it off unlocks every element, locking elements one by one leaves it off (each element's own lock is the `0x02` header byte) | 1 | S164, S172 |

  `aecc_criteria::constraint_editing` reads both and
  `set_constraint_editing` writes them (also adding or removing the lock
  byte in every element header). `create_alignment` writes both off
  (D-052).
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
      13 bits 1010000001010 (B 1, BL kind 2 = speed, BD 0; S243),
      BD station, BD distance from the element's start, BD length to the element end,
      BL ID of the containing element, BL its before-ID, BL its after-ID,
      RD element start x, y, element end x, y, point x, y },
```

then the design-speed table. (The three BLs were first read as `1, 0,
next`, true for speeds on element 1; S115 showed they are the containing
element's ID and links.) Curves have the same form (S113). Civil 3D
orders the intervals by creation (S74: 2+00 first).
Each speed with an interval has three strings (comment + two), a speed
without an interval one. **Later (S243):** the block also holds the
intervals of station equations (kind 3) and masking regions (kind 4);
see "Station-anchored data".

`set_design_speeds` now writes the whole block from the geometry (one
interval per speed, table order), so it also adds speeds to alignments
that have none. It reproduces Civil 3D's records bit for bit for S79
`DESING OFF` (two intervals) and S84; S24 differs only by Civil 3D's
last-digit rounding at a fillet tangent point. Speeds outside the
alignment are refused; speeds on curves are written since S113.

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
  to the element's end would be 35.72); still unexplained (MIST11; every
  later curve interval follows the rule, S113).
- civil3drust at first wrote intervals only for speeds on tangents
  (speeds on curves got a table entry and one string; a table without any
  interval is interval count 0 followed by the entries). Curve intervals
  are written since S113.

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
  after each arc header): 0 → 1 for both arcs (MIST8). Our
  exact coordinates (220, 0), (300, −80), … are kept in the records.
- **Derived points recomputed in floating point.** Civil 3D recomputes
  the tangent end points from the fillets and writes its own rounding
  wherever a derived copy is stored: in the proxy graphics (T1 end
  220 → 220.00000000000003, 0 → 2.842e−14; T2 start y −79.99999999999999
  → −79.99999999999997, end y −220 → −219.99999999999997), in the station
  at the end of element 0 (220 → 220.00000000000003) and in the element-0
  cache (its end point, same two values). These are ulp differences and
  confirm the copies are derived.
- **Section 2**: MIST1 rewritten and MIST2 added (as in S8, S51, S57).
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
  bits before that BD differ as well (`00110` → `10010`: the first-element
  ID, below).
- **Interval block**: both GUID copies (interval and speed entry) get new
  values, but the interval itself is left stale — station 0, length to
  the element end 220, next element 4, points (0, 0) → (220, 0): it still
  describes the deleted tangent.
- The 16 bits `1010100100000001` before the interval GUID are the
  interval block's own header: `BL 0, BL 0, BL 0, BL n` with n = 1.
- Graphics: T1's polyline and one attribute triple removed (348 → 252
  bytes); MIST1 rewritten; no element-ID list added.

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
- MIST1 rewritten; graphics regenerated (252 → 316 bytes, 12 → 10
  chunks; derived).
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
- **MIST1 equals S94's** bit for bit, although it changed in S94 → S95:
  it is computed from the geometry, not a random or per-save value (as
  S12 = S14 suggested).

**Rules for element edits** (S94–S96): keep every other record and its
ID; the storage order is creation order (new records go last); a new
element takes an unused ID (highest + 1 is safe); relink the neighbours;
element count and string count (3 per arc) follow; a free curve that
loses a tangent becomes a floating curve (code 15) with its old PC as
start point; deleting the first element keeps stationing (start station
= old station of the new start). Civil 3D leaves the design-speed
interval stale; civil3drust rebuilds it (below).

**First-element ID in section 2.** Comparing the 60 bits around the
start station in every sample: the ID of the path's first element is
stored **twice** in section 2 — as a one-byte BL followed by `10` right
before the start-station BD, and as the BL that **ends** section 2
(the last 10 bits). Both are 1 in every sample (tangent 1 first) except
S94–S96, where both are 4 after T1 was deleted.
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
writing the site's topology graph (nodes and edges decoded since, faces
only partly: "Topology nodes", "Site containers with crossings"). civil3drust therefore creates **siteless**
alignments only, also in drawings that have sites (S107, D-038); moving
an alignment into a site stays a Civil 3D operation for now. Reading
sites and membership works for every sample (tests `site_samples`).

S107 = S104 + a siteless alignment `SITELESS NEW` made by
`create_alignment`, to confirm that sites in the target don't disturb
alignment creation.

**Result (S107) and the graph check.** AUDIT 0 errors. The owner's
LandXML export lists the site's two alignments under `<Alignments
name="TEST SITE">`, `SITELESS NEW` under the unnamed group, and **no
parcels**. Searching the containers for the exported coordinates:

- the topology stores one **node per element end point** (start, PC/PT,
  end): `TEST` 5 nodes for 4 elements, `CREATED to TEST SITE` 3 nodes for
  2 elements; S104's container holds all **8** (the two alignments don't
  touch, so no node is shared), and the export has no parcels because no
  closed loop exists — consistent with nodes + edges (elements) + faces.
- each node is `BD x, BD y, BD z (0)` followed by 13 bits (`01 00100000`
  + 3 bits counting 2, 3, 4, 5 along `TEST`; the last node differs);
  node coordinates carry their own rounding (88.19660112494603 vs the
  alignment's 88.196601125011).
- the siteless companion has the same body (S106's new companion 108E
  equals S102's container body, 32 bits later), so a companion is the
  one-alignment version of the same graph.
- **S107's companion is stale:** `create_alignment` copied the donor's
  (TEST's) nodes, so 107C holds TEST's five points instead of `SITELESS
  NEW`'s. Civil 3D accepts it (AUDIT clean, as S28) and the alignment
  itself is right. civil3drust now writes the companion's nodes from the
  geometry (next section, D-040).

## Topology nodes (S25–S49, S102–S106)

Companions and site containers start the same way after the object
header and the key: a constant run, then the **node list**:

```text
BL next free node ID, BL node count n, BL ID of the first node,
n × { BD x, BD y, BD z, 3 bits 010, BL ID of the next node (0 = last) }
```

| record | nodes | IDs (list order) | next free |
|---|---|---|---|
| S25 FB7 (1 tangent) | 2 | 1, 2 | 3 |
| S44 FF2 (2 tangents) | 3 | 1, 2, 3 | 4 |
| S106 108E (`TEST`, 4 elements) | 5 | 1–5 | 6 |
| S48 102A (`C`, 4 tangents) | 5 | 40, 41, 42, 43, 39 | 44 |
| S49 102A (`C` + 2 curves) | 7 | 16, 17, 18, 12–15 | 19 |

- The nodes are exactly the element end points (start, PC/PT, end), one
  per point, z = 0; a site container holds the nodes of all its
  alignments (S104: 8). Test `nodes_are_the_element_end_points` checks
  this for every alignment of S25, S44, S49, S102, S104, S106.
- IDs come from a counter that keeps growing within the record (S48's
  39–43 after earlier edits); after Civil 3D rebuilt C's graph for the two
  curves (S49) they restart lower. The list follows the next-IDs, not the
  path order.
- After the node list comes the **edge list**: `BL 2, BL 1, BL 0, BL 1`
  (next free edge ID, edge count, 0, first edge ID — a siteless alignment
  is **one edge**), then the edge: `BL flags, BL vertex count n, BL 0`
  and n vertices `BL kind, BL vertex flags, BL node ID, BL a, BL b`, an
  arc vertex followed by `BD bulge, BL 0`. The edge is a polyline through
  the nodes in **path order** (S48: 39, 41, 42, 43, 40 = (0,−200) …
  (400,−250)):

  | kind | meaning |
  |---|---|
  | 0 | a tangent starts at this vertex |
  | 1 | an arc starts here; bulge = tan(Δ/4), + left (S49: +0.11643 for the 26.57° left curve, −0.19891 for the 45° right one) |
  | 2 | last vertex |

  Edge flags: 0x10, + 0x800 in S49 and S106 (not "has arcs": S98 F9D has
  an arc with 0x10, below), + 0x20 in S25/S44 (geometry taken from an
  object, MIST12). Vertex flags 10 with a = b = 0 in S48/S49; 2, 50 or 178
  with a = b = 1 in S25, S44, S106 and S104 (a, b are face IDs, S112;
  the flags are MIST12).
- The companion ends with `BL 0 ×4, BL 2`, a face-like entry count
  (1 with `BL 1, 1, 7` + two BLs that follow the vertex a/b; 0 in S48,
  whose handle stream then lacks two null references), `BL 0 ×2`, six
  `BL 1, BL 100` pairs and six `BL 0`, then the ID string.
- A site container holds one edge per alignment (S104: header `3, 2, 0,
  1`; the second edge, `CREATED`, runs 6 → 8 → 7). The separator between
  edges and the container's tail are not decoded.

**Writing (D-040).** `aecc_topology::rebuild_companion` rewrites a
siteless companion's nodes and edge from the alignment's elements
(node IDs 1… in path order, vertex flags 10, a = b = 0); the object
header, key, edge flags, tail, strings and handle stream are kept (see
S109 below). Rebuilding S49's companion from `C` reproduces Civil 3D's record
size, flags, vertex kinds, bulges (to 1e−9) and everything after the edge
bit for bit (test `rebuilt_companion_matches_civil3d_s49`; only node IDs
and order differ). `create_alignment` now rebuilds the companion (S107's
was stale), and the geometry-editing examples (`edit_elements`,
`set_radius`, `set_length`, `set_pis`) refresh it
(`refresh_companion`). S108 has three alignments made this way.

**S108 → S109: rejected, and why.** Civil 3D's RECOVER discarded all
three rebuilt companions with `Error 34 (eWrongObjectType)` (6 errors
fixed, 3 objects erased; the alignments were kept with dangling
references). The template companion S98 F9D has **no face-like entry**
in its tail and a **three-reference** handle stream, but the first
rebuild wrote S49's tail with one entry. Across the samples the entry
count goes with the handle stream: 3 references without it, 5 with one
(test `tail_entry_matches_reference_count`), so Civil 3D read two
references past the end. F9D also shows that edge flag 0x800 is not
"has arcs": it has an arc with flags 0x10.

Fix: the rebuild keeps the template's **edge flags and tail** verbatim
and rewrites only nodes and the edge's vertices; companions whose
vertices are not in the uniform form (flags 10, a = b = 0) are left
unchanged. The handle stream is trimmed to its references. S110 = S108
regenerated this way (test `rebuilt_companion_keeps_the_template_tail`).

**Result (S111).** S110 opens without RECOVER, AUDIT 0 errors. On save
Civil 3D rewrote all three companions with **the same graph**: same
points, vertex kinds and flags, edge flags, tail and bulges (to the last
digit); only the node IDs are new, allocated from our next free ID (FB2:
7–12, next free 13) and listed in its own order. So the rebuilt
companions are accepted as they are (test
`civil3d_keeps_rebuilt_companions_s111`).

A misaligned start (a 4-byte BL read from the constant run) passed the
first node-list check in S49; the reader now bounds the IDs.

`aecc_topology::node_list` reads the list.

## Site containers with crossings (S112)

S112 = S104 + `test 3`, a U-shaped alignment created in `TEST SITE` that
crosses `TEST` (at (43.23, 0) and (131.11, 15.56)) and `CREATED to TEST
SITE` (at (43.23, −56.52) and (131.11, −47.74)). Civil 3D created two
parcel records (10AF, 10B0, class `AECC_PARCEL`, 660 bytes each). The
container 100E (523 bytes) references the site's three alignments (soft
pointer + null each), then the site and the two parcels (soft pointer +
null each), then 851.

- **Nodes:** 16 = the 12 element end points + the **4 crossings**.
- **Edges:** 3, one per alignment, in the order of the container's
  alignment references; header `4, 3, 0, 1`. Each edge ends with
  `BL 0 ×4`; between edges come `BL 0` and the next edge's ID (also in
  S104). Edge flags 2192 or 2224.
- **Crossings are vertices of every edge through them**, with vertex
  flags **1072**; `TEST` runs 1 → 13 → 2 → 3 → 15 → 4 → 5 (13 and 15 its
  crossings).
- **Vertex fields a, b are face IDs** on the two sides of the segment
  that starts at the vertex: 1 = the site's outer face, 2 and 3 the two
  parcels, 0 at the last vertex. `TEST` has (1, 3) from 13 to 15 — the
  stretch that bounds parcel 3 — and (1, 1) elsewhere; `test 3` has
  (3, 1), (2, 1), (2, 1), (2, 1), (3, 1) along its U. So a siteless
  companion's a = b = 1 (S25, S44, S106) is its single outer face, and
  a = b = 0 (S48, S49, S98) means no face information.
- **Faces** follow the edges: `BL next free face ID, BL count, BL 1, BL
  first face ID` (S112: `4, 3, 1, 1`; S102/S104 and the companions:
  `2, 1, 1, 1`), then per face flags and boundary references: S102
  `14, 1, (1, 0)`; S104 `14, 2, (1, 0), (6, 0)` — the outer face lists one
  start node per separate alignment (1 for `TEST`, 6 for `CREATED`);
  S112 `6, 1, 16, 1, 2`, `2?, 14, 1, 16, 3, 3`, `3, 14, 1, 15, 3, 1`, …
  The exact face record and what follows (S112: `1 3 1 1 …`, S102:
  six `1`) are not decoded.

`aecc_topology::graph` reads nodes and edges of companions and site
containers (tests `site_container_with_crossings_s112`,
`site_containers_parse_s102_s104`).

**Consequence for writing.** Putting an alignment into a site means
computing the planar arrangement of all its alignments: crossing nodes,
split segments with face IDs, faces with their boundaries, and a parcel
record per closed region. D-038 stands: civil3drust creates siteless
alignments; site membership is read-only. A narrower case — an
alignment that crosses nothing in the site (S104's form: one more edge,
one more start node in the outer face) — could be written once the face
record is decoded, if it becomes worth it.

## Section 2 revisited: common entity data and variants (S1–S112)

Dumping section 2 of all 249 alignment records in the samples and
diffing each against S24's (413 bits):

- **The first 37 bits are the common entity data** of the ODA DWG
  specification (R2010+): BB mode, BL reactor count, B no-xdictionary,
  B has-DS-data, BS colour, BD linetype scale, BB linetype flags, BB
  plot-style flags, BB material flags, RC shadow flags, 3 × B visual
  style, BS invisibility, RC lineweight. 242 records have colour 256 and
  lineweight 29 (BYLAYER), linetype BYLAYER, scale 1; S18–S23's alignment
  has colour 0, linetype flags 1 and lineweight 30 (BYBLOCK); S1's has an
  extension dictionary (the no-xdictionary bit is 0, matching the extra
  hard-owner reference at the front of its handle stream). The class's
  own data then starts with `BL 80, BL 0, BL 80, BL 80, BL 0` in every
  record. `aecc_entity::entity_common` reads it (test
  `every_alignment_starts_section_2_with_common_entity_data`).
- **The object header is not fixed-length.** S68's alignment (class
  number 522, from the class table AutoCAD made in S64) has a 10-bit
  object type instead of 18, so its EED field is at bit 34, not 42.
  `aecc_entity::eed_bit` computes it from the object type and handle
  length, and `aecc_build` now uses it.
- **Lengths seen:** 413 (plain), 457 (+44: the extra field after the
  66-bit cache, written by Civil 3D re-saves), 477/521 (+64: a nonzero
  start station), 465, 545, 589, 631, 653.
- **A +28-bit field after the class header** (section-2 bit 119, after
  `BL 1` instead of the plain form's 32-bit value) appears in exactly the
  alignments whose properties were changed in Civil 3D's Alignment
  Properties dialog or made with the layout tools: S9 (start station),
  S22's alignment (style), S59 (type), S74/S78 (design speeds), S79–S88
  (criteria, start station), S89 (layout tools), S98–S100 `Remove first`
  (its S80 check set). Plain re-saves, polyline-created and generated
  alignments don't have it. **Later:** it is part of the Alignment
  Properties header (MIST4), which announces the date-modified block
  (MIST3); civil3drust writes the plain form instead (D-057).
- The 66-bit cache field varies per record and is computed from the
  geometry (S94 = S96); the algorithm was not found.
- **Post-type block** `BL 2, 2, 2, 0, x`: x is the implied PI display
  option (Alignment Properties → Point of Intersection): 1 by change in
  alignment direction, 2 by individual curves and curve group, 3 do not
  display. Per alignment, defaulted from the drawing (S5–S23: 1, S24: 3).
  Settled by S122 (three copies set to 1, 2, 3); the drawing scale does
  not affect it. Read/set with `aecc_build::pi_display` (MIST6).

The other variable fields were identified later: the 31-bit field is
inside the date-modified stamp (MIST3), the +44 field is MIST2, and the
post-type value is the implied PI display (above).

## Line methods and speeds on curves (S113)

S113 (owner): three alignments whose last line was made with a specific
layout method, plus an alignment with no elements.

- **Fixed Line (from curve end, length)** is stored as a plain fixed line
  (kind 0, start and end point); the length isn't kept.
- **Floating Line (from curve end, length)** is **kind 5**: after the
  links only `BD length` (100). It starts at the previous element's end
  in that element's end direction.
- **Free Line (between two curves)** is **kind 9**: no data after the
  links. It is tangent to both neighbouring curves (here three-point
  curves, which keep their stored points) and trims them: the first curve
  ends and the last starts at the tangent points. The owner drew the two
  curves as separate pieces; the free line joined them into one chain.
  Decoded stations match Civil 3D's labels (PT 0+22.18, PC 3+10.06, EP
  3+89.12; test `line_methods_s113`).
- The curves before the length lines are n2 = 13 (radius + end point,
  tangent to the line before), the code S99 showed after a deletion —
  Floating Curve (from entity, radius, through point), confirmed by S117.
- `EMPTY`: element count 0, no records, both first-element IDs 0, no
  speed intervals, one table entry (60 at 0+00). civil3drust reports it
  as "no element records".

**Speed interval on a curve.** `fixed line by length`
has 60 at 0+00 and 30 at 1+18, inside the curve (1+03.90 – 1+33.55). The
curve's interval has the same form as a tangent's (the `2, 1` where
tangent intervals had `1, 0` turned out to be element IDs, see S115): station 118, distance from the element start 14.102, then
**the length to the element end 15.548** (= 29.650 − 14.102, the same
rule as on tangents), the next element's ID, and the curve's start, end
and the point at the station on the arc. S89's curve interval stores
an end point 169.264 along the arc, the curve's length when the speed was
set; the curve was shortened later and the interval is stale (MIST11,
solved by S235–S237).
`set_design_speeds` now writes curve intervals; for S113 its intervals
equal Civil 3D's to the last digit (test
`speed_interval_on_a_curve_s113`). Civil 3D itself wrote **no** intervals
for the other two alignments (one speed at 0+00), so the block is
optional, as S90/S91 showed.

S114 = S113 + speeds starting on curves, written by civil3drust
(`free line` 0:60 10:50 200:40 350:30; `floating line by length` 0:60
120:35).

**Result (S115).** S114 opened without errors; the speeds show and can
be edited (owner). On save Civil 3D kept every interval's station,
distances and points (last-digit rounding) but **rewrote the three BLs
after the distances**: our `1, 0, next` and `2, 1, next` became the
**containing element's ID, its before-ID and its after-ID** — e.g.
`4, 1, 3` for the speed at 2+00 on the free line (element 4 between 1 and
3), `3, 4, 0` on the last curve, `1, 0, 4` on the first curve. Every
earlier sample fits: their tangent speeds were on element 1 (`1, 0,
next`) and S113's curve speed on element 2 (`2, 1, 3`). civil3drust now
writes them this way and matches S115's intervals (test
`interval_links_match_civil3d_s115`). Civil 3D also reordered the
intervals (0, 200, 350, then 10); the order follows neither station nor
table order and doesn't matter for reading.

## Curve methods (S117)

S116's 14 empty alignments, each given one curve by the owner with one
Civil 3D curve method (inputs in the descriptions), saved as S117. Every
geometry below was checked against the lines and arcs Civil 3D wrote into
the record's proxy graphics. Full method → code table:
`docs/element-methods.md`.

**Parameter block.** `B flag, BD p1, BD p2, BL n1, n1 × BD, BL n2,
B closed, BL id, before, after`. `n1` counts the BDs that follow (1
everywhere except FC2, where it is 2 with values −1, −1). The bit before
the IDs, always 0 before, is 1 on the three whole circles.

**Fixed curves are normalised.** Two points and direction (first or
second point), two points and radius, from entity end through point:
all stored as three-point arcs (code 2) — the method is not kept.
Through point + direction + radius is stored as centre and radius.

**Whole circles (codes 0, 1).** A fixed curve by its centre with nothing
attached becomes a whole circle: length 2πR, drawn as a circle, start
marker at centre + (R, 0). Code 0: `RD centre, BD radius, B clockwise`;
code 1: `RD centre, RD through point, B clockwise`. FC5's description
says "through point at 0,0" but the record and the drawn marker have
(200, 0) (radius 100).

**Floating curves.** Code 8 (from entity end, through point): `RD end`;
code 21 (from entity end, radius, length): `BD radius, BD length,
B clockwise` (1 for FD1, drawn clockwise); code 11 (from entity, through
point, direction at point): `RD end, RD unit direction` — the curve is
tangent to the previous line, and that line's stored end (100, 1100)
is drawn extended to the tangent point (139.50, 1100); code 13 (from
entity, radius, through point) confirmed: `BD radius, RD end`, the line
trimmed.

**Free curve through a point (code 28).** `RD point`; the fillet between
the two tangents whose circle passes through it.

**Direction bit and open centre curves (S118).** `CCW` (code 21 with
the bit 0) is drawn counter-clockwise: the bit is 1 = clockwise. In
`Center and Radius` the two centre curves have the closed bit 0 and store
nothing but centre, radius and direction; their ends come from the lines
next to them. The free line between them touches both circles; the
floating line after the second leaves its circle tangentially toward its
stored end point. The first curve, with nothing before it, is a half
circle (180.0000°) ending where the free line leaves it — its start is
not stored. Civil 3D turns commas in alignment names into `_`. The same
holds the other way round (S242 `C EAST`): a line constrained by the
next element (kind 3, start point only) runs into the circle
tangentially and the curve, with nothing after it, is a half circle from
there; S242 `C NORTH` repeats S118's case with the radius picked
elsewhere (MIST14).

**Line-spiral group (S119).** The owner edited S118's first circle;
Civil 3D added element 7 before it: a line from (438, 2124) and a
clothoid into the circle, drawn as a line to TS (254.4271, 2098.2015) and
a spiral polyline to SC (233.0160, 2097.0713). It is **one record** with
a new kind prefix (`…01010`, value 10; tangents 2, arcs 4):

```text
BL 0x04020008, B flag 0, BD −1, BD −1, BL n1 1, BD −1,
BL code 162, BL id 7, BL before 0, BL after 2,     (no bit before the IDs)
BL 256 (spiral type: clothoid, MIST10), RD start 438, 2124, BD spiral length 21.5058
```

Geometry: the clothoid (A² = R·L) has length 21.5058 and ends at the
circle's radius (41.2311); the line from the stored point is tangent to
the circle shifted out by the spiral's offset `p = y − R(1 − cos θ)`,
and TS lies `k = x − R sin θ` before the foot of the perpendicular from
the centre. The centre curve now starts at SC. Computed TS and SC match
the drawing to 4 decimals. The group's line and spiral are separate parts
of one element (D-042).

## Spirals (S120 → S121 …)

S120's 21 empty alignments are filled one spiral method at a time.

**Fixed spirals (S121, S162).** S162 is S121 plus five alignments
`SPIRAL A`–`E`, each a line (x0,0 → x0+100,0) and one spiral after it.
Every spiral is one record of kind 6 (prefix `…00110`; lines 2, curves 4,
line-spiral groups 10):

```text
BL 0x04020008, B flag, BD −1, BD −1, BL n1 1, BD −1,
BL kind (0 inward, 1 outward, 2 compound; "type" in the table below),
BL id, BL before, BL after, BL spiral type (256 clothoid, MIST10), B clockwise,
inward:   RD start X, Y, RD unit direction, BD length, BD end radius
outward:  BD start radius, BD length, RD start X, Y, RD unit direction
compound: RD start X, Y, RD unit direction, BD start radius, BD length,
          BD end radius
BD 0, BD 0, BD 1 (MIST16)
```

| alignment | description (owner) | stored | drawn end = computed end |
|---|---|---|---|
| FD7 `Fixed Spiral` | inward, clockwise, R=100, L=200 | type 0, cw, L 200, R 200 | (295.0575, −32.7428) |
| 1071 `SPIRAL A` | after a line, inward, ccw, L=150, R=80 | type 0, ccw, L 120, R 300 | (619.5209, 7.9772) |
| 109F `SPIRAL B` | outward, clockwise, L=120, R=300 | type 1, cw, R 300, L 120 | (918.7232, −15.9270) |
| 10A1 `SPIRAL C` | compound, cw, L=200, R-start=150, R-end=300 | type 2, cw, 200, 200, 300 | (1275.3496, −83.5756) |
| 10A3 `SPIRAL D` | incurve, point (1400, 45) | type 0, ccw, L 118.1325, R 45.8811 | (1400, 45) |
| 10AE `SPIRAL E` | outcurve, point (1400, 45) | type 1, ccw, R 88.1522, L 111.6352 | (1400, 45) |

Each computed end (linear curvature from the start radius to the end
radius over the length, integrated numerically) equals the last point of
the spiral polyline in the record's proxy graphics to 4 decimals; the
start point and direction are the line's end. Notes:

- The values are what Civil 3D stored, not always what the descriptions
  say: A holds B's numbers (L 120, R 300) and C's start radius is 200, not
  150; FD7's radius is 200. The drawing confirms the stored values.
- A spiral laid out through a point (D, E) is stored by its computed
  length and radius; the point is its end and is not stored.
- The clockwise bit: 1 for FD7, B, C (clockwise), 0 for A, D, E.
- Compound order settled by S165: the owner changed C to L=800,
  R-end=250 (R-start 300 typed, 200 kept); it stores 200, 800, 250, and
  only start radius 200 → length 800 → end radius 250 ends where the
  drawing does (965.1484, −411.2572).
- FD7's header flag is 1 (MIST8).

**Free spirals between two lines (S165).** `Free Spiral (Between two
entities)` (FDA, incurve) and `… (1)` (10C5, outcurve), each L=800 between
two fixed lines. The spiral is a new record kind 8 (prefix `…01000`) that
stores only its length:

```text
BL 0x04020008, B flag, BD −1, BD −1, BL n1 1, BD −1,
BL code (0x000A000A incurve, 0x000A000C outcurve), BL id, BL before,
BL after, BL 256 (MIST10), BD length
```

Geometry: the spiral turns by the angle Δ between the lines (here
clockwise), so its radius at the curved end is L / 2Δ = 250.6310; it is
slid along the first line until its end lies on the second. Both lines are
extended or trimmed to it (FDA: line 1 drawn to (3047.9642, −422.1033)
beyond its stored end (3023.5, −439.5); line 2 from (3757.5827, −352.0680)
instead of (3816.5, −439.5)). Computed start and end equal the drawn
points to 4 decimals for both samples.


**Spiral groups (S173).** One record holds several parts; only lengths,
radii and (sometimes) a point are stored, and the position comes from the
neighbours. Records of kind 8 (codes 0, 2, 6; free spirals use the same
kind with codes 0xA000A/0xA000C) and kind 10 (codes 161, 162):

```text
BL 0x04020008, B flag, BD −1, BD −1, BL n1 1, BD −1,
BL code, BL id, BL before, BL after, BL 256 (MIST10), data:
  8/0   BD L1, BD R, BD L2            Free Spiral-Curve-Spiral (between two entities)
  8/2   BD L, BD R, RD point X, Y     Floating Curve with Spiral (from entity, radius, through point)
  8/6   BD L, BD R, BD L curve, B cw  Floating Curve with Spiral (from entity end, radius, length)
  10/161 BD L spiral, BD L line       Floating Line with Spiral (from curve end, length)
  10/162 RD start, BD L spiral        line with spiral into the next curve (S119)
  8/8   BD L1, BD L2                  Free Compound Spiral-Spiral (between two tangents)
  10/4  BD A1/A2                      Free Compound / Reverse Spiral-Spiral (between two curves)
  10/0  BD L1, BD L2                  Free Compound / Reverse Spiral-Line-Spiral (between two curves, spiral lengths)
  10/1  BD line length                Free Compound / Reverse Spiral-Line-Spiral (between two curves, line length)
  20/1  BD L1, BD L2, BD R, RD point  Floating Reverse Curve with Spirals (from curve, radius, through point)
  20/2  BD L1, BD L2, RD P1, RD P2    Floating Reverse Curve with Spirals (from curve, two points)
```

| alignment | stored | drawn joints = computed |
|---|---|---|
| FC5 | 150, 400, 200 between lines meeting at (1347.5, 1590.5) | TS (1029.5984, 1291.2202), SC (1144.8417, 1386.8694), CS (1535.4430, 1409.0588), ST (1697.6567, 1293.0199) |
| FB9 | 400, 200, 600, cw, after line → (1113.5, 2176.5) | SC (1494.4754, 2142.3444), curve end (1241.0600, 1834.1561) |
| FBC | 400, 200, (2400, 1800), after line (1809.5, 1752.5) → (2272, 1946) | line trimmed to TS (1971.3346, 1820.2081), SC (2353.0101, 1845.3614) |
| FB6 | 200, 400 after a three-point curve ending at (3051.5, 1554) | ST (3122.3887, 1740.4594), line end (3233.9661, 2124.5824) |

Geometry: spiral-curve-spiral uses the unsymmetrical spiral formulas
(shift p = y − R(1 − cos θ), k = x − R sin θ for each spiral; T1 = (R + p1)
tan(Δ/2) + k1 − (p1 − p2)/sin Δ, T2 = (R + p2) tan(Δ/2) + k2 + (p1 − p2)/sin
Δ from the PI). The through-point group slides the spiral along the line
until its curve passes through the point; of the two positions, Civil 3D
uses the one with the shorter curve to the point. The other two continue
from the previous element's end (the spiral-line group starts at the
previous curve's radius).

**Record ends corrected (S173).** The block after the last record starts
with five `BL 0` in alignments copied from S113's `EMPTY` (S117, S120)
and three in S24's: so fixed spirals end with three BDs `0, 0, 1`, not
four, and free spirals have no trailing bits after the length.

**Reverse curves with spirals (S174).** Kind 20 (prefix `…10100`).
Both start at a point on the previous curve, which Civil 3D trims there,
leave it with a spiral of length L1 (curvature 1/R1 → 0), and enter the
reverse curve with a spiral of length L2 (0 → 1/R2, turning the other
way). The start point is not stored; it is solved so that the second
curve passes through the stored point(s):

| alignment | stored | solved (= drawn) |
|---|---|---|
| FBF | L1 1000, L2 500, R 200, point (3200, 2900) after a three-point curve R 322.582 | start (2234.7204, 2224.9383), spirals' joint (2823.7016, 2901.8291), SC (3095.1080, 3280.8012) |
| FC2 | L1 750, L2 650, P1 (4700, 2700), P2 (5900, 3100) after a three-point curve R 300.007 | start (3817.0330, 2103.6556), joint (4442.6995, 2415.7895), SC (4898.6665, 2876.5096), R2 1302.9691 |

The second curve ends at the (last) point; with two points, P1 only
defines the circle (it lies before the SC in FC2). The equations have
more than one solution (FC2: another start at angle −0.394 rad with
R2 1721.16); Civil 3D's is the one with the shortest second curve to the
end point (0.81 rad against 4.68), the same rule as the through-point
curve with spiral (S173 FBC). civil3drust searches the start along the
previous curve (and the radius between L2/8 and 10⁶) with Newton's method
from a grid of starting values.

**Spiral-spiral groups (S175).**

- *Between two tangents* (FE3, code 8/8: L1 500, L2 600): two spirals
  meeting without a curve; the radius at the joint is (L1 + L2) / 2Δ =
  348.4002 and the position follows the spiral-curve-spiral formulas with
  a curve of zero length: TS (7355.6223, 1256.1543), joint (7760.0423,
  1530.4250), ST (8280.3228, 1273.0499), exactly as drawn.
- *Between two curves* (FDD compound, FE0 reverse; code 10/4): only the
  ratio A1/A2 (1.0) is stored. A spiral leaves the first curve (1/R1 → 0)
  and a spiral enters the second (0 → 1/R2), turning the same way
  (compound) or the other way (reverse), with A1² = R1·L1 and A2² =
  R2·L2. The start on the first curve and A are solved so that the second
  spiral ends tangent to the second curve; both curves are trimmed or
  extended to the spirals (FE0's first curve is drawn 282° long, past its
  stored end at 237°). One solution in each sample: FDD L1 1585.27, L2
  1116.99 (points within 0.0003 of the drawing); FE0 L1 8312.75, L2
  3940.27 (within 0.005; increasing the integration accuracy doesn't
  change civil3drust's points, so Civil 3D's very long spirals are drawn
  slightly off the exact clothoid).

**Spiral-line-spiral groups (S176).** Between two three-point curves: a
spiral out of the first (1/R1 → 0), a line, a spiral into the second
(0 → 1/R2), compound or reverse by the curves' senses. Both curves are
trimmed to the group.

| alignment | stored | solved (= drawn) |
|---|---|---|
| FE6 compound, spiral lengths (10/0) | L1 400, L2 200 | line 7364.5283 |
| FE9 reverse, spiral lengths (10/0) | L1 200, L2 400 | line 5819.3099 |
| FEC compound, line length (10/1) | line 600 | L1 6529.05, L2 12057.80 |
| FEF reverse, line length (10/1) | line 500 | L1 4556.89, L2 3885.79 |

With the line length stored, the spirals have **equal A** (A² = R1·L1 =
R2·L2: FEC 22.51 million, FEF 9.65 million); that, the start on the
first curve and the tangency to the second curve fix the group. One
solution in each sample. Points match the drawing to 4 decimals except
FEC (within 0.02; its second spiral is 12 058 long and turns 3.2 rad, like
S175 FE0).

**Long spiral groups (S177).**

*Between two tangents: group records with a stored TS.* Kinds 24 and 26
have no code field after the parameter block:

```text
… BL id, BL before, BL after, BL 256, BL n (2 / 1), B first curve left (?), B 0,
kind 24 (FC8, Free Compound Spiral-Curve-Spiral-Curve-Spiral):  BD L1, R1, L2, R2, L3, RD TS X, Y
kind 26 (FCB, Free Reverse Spiral-Curve-Spiral-Spiral-Curve-Spiral): BD L1, R1, L2, L3, R2, L4, RD TS X, Y
```

The TS is the stored start on the first line (FC8 (868.5, 0), FCB (1000,
0)). From there the pieces follow in order (compound: the middle spiral
goes from 1/R1 to 1/R2; reverse: R1 → ∞ → R2 the other way); the two curve
lengths are solved so that the last spiral ends on the second line in its
direction (FC8 71.6145 and 91.4175; FCB 200.0000 and 258.3333). All joints
equal the drawing.

*Stored as separate elements, with a JSON layout.* (Confirmed by S178:
the owner first suspected a copy-paste artifact of the remote desktop and
made the three methods again without copy and paste, with new values;
Civil 3D stored them the same way, and the JSON holds the newly typed
values.) Free Reverse
Spiral-Spiral-Curve-Spiral-Spiral (between two curves, FCE), Free Reverse
Spiral-Curve-Spiral-Curve (from curve, end entity, FD1) and Free Reverse
Spiral-Curve-Spiral-Spiral (from entity, end curve, FD4) are not group
records. Civil 3D stores their pieces as ordinary spirals (kind 6) and
three-point curves with fresh IDs (4, 5, …), so they resolve with no
solving (all joints equal the drawing), and:

- the first spiral has the element header `0x0D020008` (MIST7 type byte
  `0D`) and 128 more bits: the group's start point (MIST17, below);
- the string stream has an extra string after the description: a JSON
  description of the layout, e.g. for FCE

  ```json
  {"JsonVersion": 1, "FirstId": 1, "RGObjects": [
    {"RGObjectType": 1, "Id": 1, "PreviousId": 0, "NextId": 3, "ConstraintLocked": 2,
     "HasAnchorPoint": false, "ConstraintType": 2,
     "ConstraintParameters": {"PassThroughPoint1": [0, 0], …}},
    …,
    {"RGObjectType": 7, "Id": 3, "PreviousId": 1, "NextId": 2, "ConstraintLocked": 2,
     "ConstraintType": 2569, "SpiralType": 256,
     "ConstraintParameters": {"Spiral1Length": 150, "Spiral2Length": 150,
       "ArcRadius": 600, "Spiral3Length": 150, "Spiral4Length": 150}}]}
  ```

  It keeps the method as a group (ID 3, the IDs of the binary records
  being 4, 5, …) with its parameters, and names fields the binary records
  hold: `ConstraintType` 2 = three-point curve (n2 = 2) and 0 = fixed
  line, `ConstraintLocked` 2 (the lock byte 0x02 of the element headers,
  MIST7) and `"SpiralType": 256` (the BL 256 of every spiral record,
  MIST10). The group constraint types are 2569 (0xA09, FCE), 2561 (0xA01,
  FD1) and 2565 (0xA05, FD4); they are not in the binary records.
  `string_stream::layout_json` returns it. The name and description
  readers skip it (it shifted the name by one string; in FD4 the first
  description copy is also empty).

**S178 (re-sample).** 11D4 Reverse SSCSS between two curves (Ls 120,
180, R 500, Ls 140, 200), 11DE Reverse SCSC from curve to line (Ls 110,
170, R 450, Ls 190), 11F8 Reverse SCSS from line to curve (Ls 130, R
550, Ls 160, 210): again separate spirals (kind 6, the first with header
`0x0D020008`) and three-point curves, plus the JSON string with exactly
these values; every joint equals the drawing. The spirals keep their
typed lengths; the curves between them get the typed radius, and the end
curve is drawn as stored (its radius as built by the owner).

**What the JSON does (owner, S178 in Civil 3D; experiment S179/S180).**
In Alignment Properties a stored-apart group shows as one entity with
sub-entities (11F8: entity 2 "Spiral-Curve-Rev-Spiral-Spiral", 2.1–2.4,
parameter constraint "Spiral1 Length Curve Radius and Reverse…", locked;
tangency "Constrained on Both Sides (Free)"), between the fixed line 1 and
the fixed curve 3 — the JSON's numbering, not the binary records' (4, 5,
…). The group has no controls of its own beyond the entity list; it
follows small changes of its neighbours and disappears when they change
too much. Civil 3D's LandXML export (S178) lists the pieces as plain
curves and clothoid spirals with the same coordinates as the binary
records.

Results:

- **S179** was meant to remove the JSON but was overwritten by mistake
  with an unmodified acadrust round trip of S178 (JSON kept). It opens
  cleanly and the groups re-solve when a neighbour is nudged; saved as
  **S181**, Civil 3D rewrites the JSON with the moved neighbours' points
  (11D4's end curve point 3 (951, 821.5) → (1075.5, 874.5), 11DE's line
  start, 11F8's line end). So the JSON is maintained live.
- **S180** (JSON emptied): opens cleanly, AUDIT clean, but nudging a
  neighbour no longer re-solves the group, before and after save and
  reopen. In the saved **S182** the group is cut from the element after
  it (that element and the rest become a detached piece). An empty JSON
  string is read as a layout with no groups: the pieces are plain
  elements and the links Civil 3D re-solves are lost.
- **S183**: the JSON string really removed (string count one lower).
  Owner: RECOVER, and the alignments come back empty, without even a
  name. The strings are read by position: the JSON string is a fixed
  slot of these alignments' string stream (after the description, before
  the created-by user and the name), so removing it shifts the name and
  everything after it. (The re-save sent back was a copy of S181, so
  there is no recovered sample.)

**Conclusion.** For these three group types the JSON string is the live
definition: Civil 3D reads it to build the group, keeps it current on
every edit, and treats an empty one as "no groups". The binary spirals
and curves are the solved snapshot. To edit an alignment that has one,
civil3drust must keep the JSON consistent (or empty it on purpose, which
makes the pieces fixed). **Later:** the JSON slot is announced by
section-2 flag 0x02 (S206), MIST17 is the group's start point (below),
and Civil 3D solves a group from the JSON alone, so civil3drust writes
only the JSON object (S216 → S217, D-059, D-061).

**Format of the JSON string (S177, S178, S181).** Tab indentation, CRLF
line ends, no trailing newline, `"key": value`, arrays of numbers on one
line as `[ x, y ]`, every number written as C `%.17g` (so `1596.536`
appears as `1596.5360000000001`, `47.0` as `47`). Objects are listed by
ID. Fixed elements: `RGObjectType` 0 (line, `ConstraintType` 0, two
`PassThroughPoint`s) or 1 (curve, `ConstraintType` 2, three points),
`HasAnchorPoint` false; the points are the ones the element was defined
with (typed), not the trimmed ends of the binary record — any points on
the same line or circle describe the same element. The group:
`RGObjectType` 7, the next ID after the fixed elements (the pieces are
numbered after it), `ConstraintType` 2569/2561/2565, `SpiralType` 256 and
its parameters in the order Spiral1, Spiral2, ArcRadius, Spiral3, Spiral4
(the spirals before the arc are 1 and 2, those after it 3 and 4; the
methods leave out the ones they lack). `aecc_layout::Json` parses and
writes this format byte for byte (test `layout_json`), and
`aecc_layout::compose` builds it from the element records.

**MIST17 solved: the group's start point.** The "128 bits after the
flag" of a member spiral begin one bit earlier, at the flag's position:
they are two RDs, the group's start point, equal to the spiral's start in
all 15 member spirals of S177, S178, S180–S182; the parameter flag comes
after them. The decoder reads `SpiralRecord::member` as that point
(encode round trip still bit-exact).

## Generation test (S184, S185)

`examples/exp_chains` writes one new alignment per source alignment:
`create_alignment` from S24's F9E as template, then `replace_elements`
with the source's element records re-encoded from their decoded fields
(`aecc_encode`, D-054), the string count after the name copied from the
source (D-055), and the companion rebuilt.

- **S184**: 43 alignments covering lines (fixed, floating, free, every
  S113 method), curves of every S117/S118 method, fixed and free spirals,
  every spiral group of S176, SCSCS and SCSSCS (S177 FC8, FCB). All read
  back with the source's element count and length. S119 1061 lost its
  design speeds (a speed on a spiral group is not written yet).
- **S185**: the six JSON-type group alignments, with their spirals as
  group members (`0x0D`) but no JSON string.

**Result (owner, S186, S187).** Both needed RECOVER.

- S184: 13 alignments discarded with "Error 151 (eOutOfRange)": FB9, FC2,
  FC5, FC8, FCB, FCE, FE6, FF2, FFB, FFE, 1016, 1019, 102E. The other 30
  opened. The discarded ones share no element kind: FBF and FC2 are both
  a single three-point curve (n2 = 2) with identical parameters, yet FBF
  opened and FC2 was discarded.
- What they share is the **string count**: copied from the source
  (D-055), it ignored the template's design speed (3 strings), so every
  alignment except the S176 spiral A–E ones had 2–3 strings too few.
  Civil 3D reads past the end of the stream; whether that fails depends
  on the bits that follow. In the re-save S186, 101C and 1031 keep
  garbage strings and 100D and 1010 have string streams civil3drust
  cannot parse: what was read past the end, written back.
- S186 also gives the rule. Civil 3D rewrote every remaining alignment's
  string count to exactly **9 + 3 per speed with an interval + 1 per
  speed without + 3 per record that is not a line + 1 for a JSON string**
  (all 29 that parse). Curves, spirals, free spirals and every group
  record count the same, 3; a group counts once however many pieces it
  has. This extends S46's "3 per arc". Civil 3D's own drawings often have
  more (leftovers of edits), which it accepts (D-056).
- In S186 the SCSSCS group (1031, S177 FCB) survived with its group
  record; Civil 3D set its parameter flag to 1 (solved), as on the other
  groups. The SCSCS group (102E) was one of the discarded.
- S185: all six discarded with "Error 34 (eWrongObjectType)". They were
  also two strings short. (**Later:** the cause was a member header
  Civil 3D never writes, `08 00 0D`; S205/S206.)

**Second round (S188–S190).** The same alignments with the string count
from the rule (`write_elements` now adds 3 per non-line record):

- **S188**: S184's 43 alignments.
- **S189**: S185's six (member spirals, no JSON).
- **S190**: the same six with the members written as ordinary spirals
  (header type `0x04`, no 128-bit block; `exp_chains --plain-members`).
  If S189 is rejected and S190 opens, the member header needs the JSON
  (MIST17); if both open, it does not.

**Result (owner).**

- **S188**: opens without RECOVER, no apparent issues, AUDIT clean. All
  43 generated alignments: lines (fixed, floating, free, by length),
  curves of every method, fixed and free spirals, every spiral group,
  SCSCS and SCSSCS. Re-saved as **S195**, which reads back to the same
  model (every element: ID, links, start, length; test
  `generated_resave`). Civil 3D only set the parameter flag of curves,
  spirals and groups from 0 to 1 (solved) and changed a few radii in the
  last bits (1e-14).
- **S189**: RECOVER, all six discarded, "Error 34 (eWrongObjectType)",
  with a correct string count. So S185's rejection was the member
  header, not the strings.
- **S190**: opens cleanly, no errors. Re-saved as **S196**: same model,
  flags set to 1 as in S195. The six long reverse assemblies are then
  plain fixed spirals and curves, without group behaviour.

**Conclusion.** civil3drust writes every element kind it reads, and Civil
3D accepts them (the owner changed the style to show arcs and spirals for
the check). Written as ordinary spirals (`0x04`) the group pieces are
accepted as fixed elements. (S189's rejection was first read as "a member
spiral needs the JSON"; **later** it turned out to be the member header
civil3drust wrote, `08 00 0D`, which Civil 3D never writes — S206 accepts
correct member headers without a JSON.)


**JSON composition test (S197, S198).** The six JSON-type alignments of
S177 and S178 generated again in S24 (as S189), with the member spirals
and a layout JSON composed by civil3drust from the element records
(`exp_chains --json`), inserted after the two description strings
(string count = rule + 1, D-056):

- **S197**: the JSON as composed; its fixed elements use the binary
  records' points (the trimmed ends and the curve's stored middle point,
  not the owner's typed points).
- **S198**: the same with each group's `Spiral1Length` 20 longer in the
  JSON only (the binary pieces keep the old length). Shows whether Civil
  3D re-solves from the JSON on opening, on a nudge, or not at all.

**Result (owner).** Both RECOVER, all six alignments discarded,
"Error 34 (eWrongObjectType)" — as S189 without the JSON (the bad member
header again, found with S205).

**The JSON flag in section 2.** Every Civil 3D record with a JSON string
is in the Alignment Properties form, and its section-2 BLs from bit 115
differ from those of records without one:

| record | section-2 BLs from bit 115 |
|---|---|
| S24 F9E, all generated (plain form) | `0x1148`, `0` |
| S177 FC8, FCB (Properties form, no JSON) | `1`, `0x08010004`, … |
| S178 11D4, 11DE, 11F8; S180 (JSON emptied) | `3`, `0x01010034`, `0x08010004`, … |
| S177 FCE, FD1, FD4; S181, S182 (with MIST2) | `0x43`, `0x01010034`, `0x08010004`, … |

So in the Properties form the first BL is a set of flags — `0x01` the
Alignment Properties header (`0x08010004` …), `0x02` the JSON layout
(announced by `BL 0x01010034`), `0x40` MIST2 — and each set flag adds its
tagged block. The JSON slot in the string stream follows the `0x02` flag:
with the flag and no string the strings shift (S183); with the string and
no flag the JSON is read as the name (S206). The Properties form also has
one more string after the JSON (the user who edited it, "felix"). The
plain form cannot announce a JSON string (S206).

**Next test (S199, S200).**

- **S199**: S178 itself as target, its alignment 11F8 as template, so the
  six generated alignments inherit Civil 3D's own JSON form (flags 3,
  `0x01010034`, the user string, the date-modified block); their JSON is
  replaced by the composed one (`exp_chains --json`). If they open, the
  composed JSON and member spirals are right and only the form was
  missing.
- **S200**: S197 with the section-2 form changed (`exp_json_form`): FB3
  and FBF get S178 11F8's Properties-form region and date-modified block
  (copied) and a user string "civil3drust" after the JSON (variant A);
  FB6 and FC2 the plain form with `0x114A` (flag `0x02` set, variant B);
  FB9 and FBC `0x114A` followed by `BL 0x01010034` (variant C).

**Result (owner).** Both RECOVER; every generated alignment discarded
with eWrongObjectType, in all variants. In S199 (re-save after RECOVER:
**S203**) S178's three own alignments survive; nudging rebuilds their
curves and the group sub-entities are there. So the Properties form and
the JSON flag are not enough either: something else in the generated
records is wrong (the member header, below).

**What differs (S178 11DE compared with itself after `replace_elements`
with its own elements).** Besides the zeroed MIST1, the removed
element-ID list (`[1]`) and the rebuilt proxy graphics, `finish` rewrote
the design speed: S178 stores it **without an interval** (zero GUID, the
form Civil 3D uses for layout-tool alignments), and civil3drust wrote it
with an interval block. Every Civil 3D record with a JSON string has its
speed without an interval (S177, S178), and every rejected one had an
interval (S189, S197–S200: from S24's template or written by `finish`).
`finish` now keeps interval-less speeds as they are when they don't
change. (S206 later opened with an interval, so this was not the cause
either.)

**Next test (S204, S205).**

- **S204** (bisect, in place on S178, `exp_json_bisect`): 11D4 gets the
  composed JSON (nothing else changed); 11DE goes through
  `replace_elements` with its own elements (speed now kept without an
  interval); 11F8 unchanged (control).
- **S205**: S199 generated again with the speed fix: six alignments from
  S178's 11F8, speeds without intervals.

**A wrong member header in every earlier generated group (found while
regenerating S205).** `create_alignment` turns Constraint Editing off
(D-052), so the generated elements are unlocked, and `write_elements`
wrote the group-member spiral as `08 00 0D`: the locked type byte without
the `02` byte, a form Civil 3D never writes. With the lock off Civil 3D
writes `08 00 02 09` (S202, the other session's MIST7 test), and the JSON
then says `"ConstraintLocked": 0` (S202 FCE). S185, S189 and S197–S200
all had the bad header, so their eWrongObjectType does not show that the
JSON flag or the speed form matter. `aecc_alignment::header_bytes` (from
S202) now gives `02 09`, and `compose` takes the lock state and writes 0
or 2. S204 is unaffected (edited in place, locked).

- **S205** (regenerated): Properties + JSON form from S178's 11F8,
  speeds without intervals, header `02 09`, `ConstraintLocked` 0.
- **S206**: S197 regenerated the same way: plain form, no JSON flag,
  speed with an interval. If S206 opens, neither the flag nor the speed
  form is needed; if only S205 opens, one of them is.

**Result (owner; re-saves S211–S213).** All three open without RECOVER,
AUDIT clean. The wrong member header was what Civil 3D had rejected in
every earlier generated group.

- **S204**: clean; nudges update all curves. Event Viewer, on opening:
  *Warning — "The alignment '…' contains new constraint curve group
  modified unexpectedly in old version. The modification won't be kept
  in new release."* for 11D4 (composed JSON) and 11DE (re-encoded), not
  for the untouched 11F8.
- **S205**: clean; nudges update all curves; the same warning for all six
  generated alignments, none for S178's own three. Generated alignments
  with a group and a JSON layout are accepted and behave as groups.
- **S206** (plain form, no JSON flag): opens clean, AUDIT clean, but the
  JSON string is read as the **name** (Prospector lists the alignments by
  their JSON text) and the GUI breaks when one is selected. The string
  slots follow the section-2 flags: without flag `0x02` there is no JSON
  slot, and string 3 is the name. The `02 09` member spirals themselves
  were accepted without the flag, so a member header does not require the
  JSON (the earlier "member needs JSON" reading, S189, came from the bad
  header). The re-save S213 keeps the plain form (`0x1148` → `0x1188`, MIST2
  added) and the JSON text as the name.
- **Re-saves.** S211 11D4 and all six generated alignments in S212 lost
  the member header (pieces now ordinary `04` spirals); S211 11D4's pieces
  were renumbered 4–8 (Civil 3D's own S178 record numbered them 5–9). The
  JSON strings are kept as written. S211 11DE (warned) kept its member
  header, and its MIST1 came back to S178's original value (zeroed in
  S204).

**What the warning looks at.** The warned records changed either the
JSON text (11D4, MIST1 untouched) or MIST1 and the derived parts (11DE,
JSON untouched); the untouched 11F8 was not warned. A stored value
depending on both — most likely MIST1, Civil 3D's geometry fingerprint
(MIST1), if for these alignments it also covers the JSON — would explain
both. **S214** (in place on S178, `exp_json_bisect`): 11D4 only MIST1
zeroed; 11DE re-encoded with MIST1 set back to its original value;
11F8 only the JSON's `ArcRadius` changed in its last digits (550 →
550.0000000000011, the radius measured from the curve piece).

**Result (owner; re-save S215).** Clean opening; the warning only for
11D4 (MIST1 zeroed); grouping kept on all three. In S215 Civil 3D
rebuilt 11D4: its pieces renumbered 4–8 (from 5–9), the member header
kept, and MIST1 back to exactly S178's value; same geometry and length.
11DE (re-encoded, MIST1 restored) and 11F8 (ArcRadius last digits) were
not warned and are unchanged.

**Conclusion.**

- For an alignment with a JSON layout, Civil 3D checks MIST1 (its
  geometry fingerprint) on opening. A mismatch logs "modified
  unexpectedly in old version" and Civil 3D **rebuilds the group's
  pieces from the JSON** (fresh piece IDs) — that is what "the
  modification won't be kept" means. Alignments without a JSON accept
  a zero MIST1 silently (S56).
- The fingerprint does not see the last digits of `ArcRadius` (11F8),
  but it did see S204's composed JSON for 11D4 (other pass-through points
  of the fixed curves, MIST1 untouched → warned): it covers the JSON's
  geometry, probably rounded. Its algorithm is unknown (MIST1).
- civil3drust cannot compute MIST1, so every generated or edited group
  alignment gets the warning once; Civil 3D then re-solves the group from
  the composed JSON and keeps it as a group (S205 → S212, S214 → S215).
  The geometry is the same, only the piece IDs change.

**Groups from typed parameters (S216).** `aecc_layout::layout` writes the
JSON from the fixed elements and a `Group` (method, neighbours, typed
parameters); the group object it writes for S178 11F8 from 130, 550,
160, 210 is Civil 3D's line for line (test `layout_json`). S216 (S178,
template 11F8, `exp_group_params` and `exp_chains --json-tweak`):

| alignment | element records | JSON group |
|---|---|---|
| 1208 `G direct … 11F8` | fixed line and curve only, linked to each other | 130, 550, 160, 210 (as typed) |
| 120B `G group … 11F8` | the same, linked to the group's ID 3 (no record has it) | as typed |
| 120E `G direct … 11DE` | fixed curve and line, linked to each other | **150**, 170, **500**, 190 (typed 110, 170, 450, 190) |
| 1211 `G direct … 11D4` | the two fixed curves | as typed |
| 1214, 1217 `S178 11DE/11F8 …` | all pieces (as S205) | `Spiral1Length` + 20 in the JSON only |

civil3drust's reader then refused the first four (the chain has a gap or
a missing ID), so they were written by cutting the pieces out of a full
record (reading them came with D-060 and D-061, below). Questions: does Civil 3D build the pieces from the JSON when
there are none (1208–1211), does it use the JSON's parameters over the
pieces' (120E, 1214, 1217), and does it accept either linking.

**Result (owner; re-save S217).** Clean opening, AUDIT clean, the
"modified unexpectedly" warning for all six new alignments. In S217
every group is solved **from the JSON**:

- 1208, 120B, 1211 (no pieces written, both linkings): pieces built,
  identical to S178's (same IDs 4…, points, lengths 130/550/160/210 and
  120/180/500/140/200); the fixed elements trimmed to the tangent points.
- 120E (Spiral1 150, R 500 instead of the typed 110, 450): a different
  group with exactly those values (spirals 150, 170, 190, curve R 500),
  the fixed curve trimmed to the new tangent point; length 4656.50
  instead of 4728.53 — the drawing the owner saw differ.
- 1214, 1217 (pieces written, `Spiral1Length` + 20 in the JSON): spiral 1
  is now 130 (was 110) and 150 (was 130): the JSON wins over the pieces.
- Member headers after the save: `09` in 120E, 1214, 1217; none in 1208,
  120B, 1211 (Civil 3D's own three keep `0D`). Both forms reopen.

**Conclusion (D-059).** A spiral group of these methods is written as
its JSON object (method and typed parameters) between two fixed
elements; Civil 3D solves the pieces on opening. The fixed elements may
be linked to each other or to the group ID. The element records need
not contain the pieces. (civil3drust then lacked a reader for such
records and a direct writer; both follow.)

**Reading a JSON group in place of the gap (D-060, superseded by D-061:
such groups are now solved, next paragraph; the type is `LayoutGroup` in
`DecodedElements::layout_groups`).** `decode_elements`
now looks for JSON groups whose pieces are not stored: the fixed element
before the group links straight to the one after it, or to the group's
own ID, which no record has. The chain is cut there; the group becomes
an `UnsolvedGroup` (ID, `ConstraintType`, neighbours, typed parameters)
in `DecodedElements::unsolved`, and the elements after it its
`following` chain (positions known, stations not, since civil3drust does
not solve the group). `elements` (and so `length()` and the stations)
ends at the first unsolved group; `all()` includes the following
elements, so the proxy graphics and the companion cover them;
`is_complete()` is false until Civil 3D has solved and saved the group.
The records keep their stored links.

With that, `replace_elements` writes the fixed elements and the JSON
directly (`exp_group_params` no longer cuts pieces out of a full
record): **S218** is S216's four parameter-only alignments written this
way — the same JSON and strings, smaller records because the proxy
graphics now hold only the fixed elements, not the stale pieces. Owner: clean opening, no visual issues, AUDIT clean (the usual
"modified unexpectedly" warning).

**civil3drust solves JSON groups when reading (D-061, owner's decision).**
A JSON group whose pieces are not stored is solved on reading, as Civil 3D
does on opening: `aecc_layout_solve::solve_group` places the pieces of the
method (SSCSS 2569, SSCS 2561, SCSS 2565) from the typed lengths and
radius, the two unknowns — where the group leaves A and the middle
curve's length — fixed by the conditions that the last piece ends on B's
line or circle heading along it (Newton from a grid of starts; of several
solutions the one leaving A nearest to A's stored end). The middle curve
turns against A (reverse from a curve) or against B (from a line). A and
B are trimmed to the solved points, the pieces go into `elements` as
parts of a `GroupDef::Layout` group, and stations run to the end of the
alignment. Against Civil 3D's solutions of the same groups (S217, from
S216; including 120E with new parameters) every element matches within
4e-7 and the lengths within 1e-6 (test `layout_json`). A group that
cannot be solved is still reported (`LayoutGroup::solved` false) with the
elements after it in `following`.

On writing, `replace_elements` drops the `Layout` group: the record keeps
the fixed elements and the JSON object only (test
`solved_json_group_is_written_back_as_json`). When the pieces are
stored as well, the JSON wins too (owner's decision; Civil 3D does the
same, S216 1214/1217 → S217): the stored pieces are set aside
(`DecodedElements::set_aside`) and the group is solved from the JSON;
for S177 and S178 the solution matches Civil 3D's stored pieces within
1e-4 (test `curve_methods`). `DecodeOptions::stored_group_pieces` reads
the stored pieces instead (recovering damaged files). Owner: these three
are the only methods kept apart with a JSON. A group at an end of the
alignment: next.

**Edge case: a group at an end (S223).** S178 plus six JSON-only groups
(`exp_group_params end|start`): for each of 11F8 (SCSS), 11DE (SSCS) and
11D4 (SSCSS) once without the fixed element after the group (`NextId` 0;
the element before links to the group's ID) and once without the one
before it (`PreviousId` 0, `FirstId` the group; the remaining element is
first in the records, before 0). These methods are defined "between" or
"from … to" two entities, so with one missing nothing fixes the group's
position. civil3drust reads them with the group unsolved (stations end
at the remaining element). Question: what does Civil 3D do — reject, keep
the group unsolved, drop it, or place it somehow?

**Result (owner; re-save S224).** Clean opening; the group does not
appear anywhere. In S224 Civil 3D has **emptied the JSON string** of all
six (slot and section-2 JSON flag kept; flags `0x43` after the save) and
kept only the remaining fixed element: the group is dropped silently.
S178's own three are unchanged. So a group of these methods at an end of
the alignment, with nothing on that side, is not valid, and Civil 3D
cannot make one (owner, S225 below); `aecc_layout::layout` refuses it.

**Reader fix.** With the JSON string empty, the name reader took the
user string ("felix") as the name. `raw::structured_name` now drops the
string slot announced by the section-2 JSON flag
(`raw::layout_json_flag`) by position, empty or not.

**Owner (S225): an end entity is required.** Civil 3D does not let these
methods be created without an end entity. Deleting the entity after a
long group (S225: S177 with the element after each of its five long
groups deleted, FC8 and FCB included) silently turns the group into its
pieces as separate elements, the JSON string emptied (slot and flag
kept). The pieces are new record forms:

- spirals of kind **6** (after a line) and **8** (after a curve or a
  spiral): `BL kind, BL id, BL before, BL after, BL 256, B clockwise,
  BD end radius, BD length, BD 0, BD 1` — no position: they start where
  the element before ends, kind 8 with its radius
  (`SPIRAL_AFTER_LINE`, `SPIRAL_AFTER_CURVE`);
- the curves become floating curves by radius and length (code 21,
  already decoded);
- the first piece keeps its group-member header and start point
  (MIST17); that point, not the stored end of the element before, is
  where it starts: FC8's fixed tangent still stores (0, 0)–(1000, 0)
  while the pieces start at 868.5, and the tangent is cut there.

Decoded this way, every joint of the five alignments lies on Civil 3D's
proxy graphics (test `s225_group_pieces_after_end_entity_deleted`), and
the records re-encode bit for bit (`encode_roundtrip`).

**Parameter lock and the JSON (S177 → S202).** Turning "Lock all
parameter constraints" off changes, besides the element headers
(`02 0D` → `02 09` on the group member, `02` dropped elsewhere), only
`ConstraintLocked` 2 → 0 on every object of the JSON. 
`aecc_criteria::set_constraint_editing` now rewrites both; for S177 FCE
its result has Civil 3D's S202 JSON byte for byte, and locking again
gives S177's (test `group_member_header_unlocked_s177_s202`).

**The group's own lock (S226, owner).** A group can only be locked or
unlocked as a whole. S226: S178 11D4 with "Lock all" off, then the group
locked. The element headers are all unlocked (the member `02 09`, the
others without `02`); only the JSON's group object says
`ConstraintLocked` 2, the fixed elements 0. So `ConstraintLocked` is per
object, and a group's lock lives only in the JSON. (Civil 3D also
re-solved the group on that edit: pieces renumbered 4–8.) The reader
reports it as `LayoutGroup::locked`; `aecc_layout::Group::locked` sets it
when writing (test `group_lock_is_its_own`).

## Open questions and next samples

Open fields are tracked as MIST1 … in [`mysteries.md`](mysteries.md),
each with its location, the samples that show it and a test to settle
it; its last section lists what has not been sampled. Deferred work is in
[`FUTURE.md`](../FUTURE.md).

## Template note

S1 and S5 (and S6, S11, S12, S15, S89) contain a `3DSOLID` in Civil 3D's
internal block `PRESSUREPARTERRORBLOCK` (handle 7F72) that acadrust 0.6.5
could not parse (failsafe mode, solid dropped on save). acadrust 0.6.7
reads it, and those samples now load normally and keep it after a save
(`docs/acadrust-0.6.7.md`).

## Provenance

Every offset and value above was produced with this repository's tools on
acadrust's record bytes. An earlier exploratory pass (before this repository
existed) used LibreDWG's `dwgread` purely as a byte dumper; LibreDWG has no
knowledge of AECC classes, no LibreDWG code or documentation was used, and
all findings here were re-derived with acadrust-based tools.

## Proxy graphics setting (S140, S141)

The same drawing saved by Civil 3D 2024 with PROXYGRAPHICS 1 (S140) and 0
(S141). With 0 Civil 3D still writes a graphics stream, but its
placeholder: a text chunk and polylines (684 bytes for an empty alignment,
312 bytes for one with a spiral), instead of the real display (60 bytes of
attribute chunks for an empty alignment, 2688 bytes for the spiral one).
Everything after the graphics and the handle stream are bit-identical, so
no field of the record depends on the setting (MIST1, MIST2). S123/S124
(Civil 3D 2027) also have PROXYGRAPHICS 0, hence their placeholders.

## Derived values not stored (S24, owner's Civil 3D listing)

Civil 3D's Entities and PIs listing of S24's alignment was searched for in
the record (64- and 32-bit floats at every bit offset). Stored: tangent
end points, PI coordinates, radius, the station after element 0, the
speed-interval point and the design speed. Not stored anywhere: chord
length, delta, start/end directions, external tangent and secant,
mid-ordinate, mid and radial points, the arc's end coordinates, the total
length and the degree of curvature — Civil 3D derives them (D-026).

## MIST1: geometry cache (partially solved, S142–S150)

The 66-bit value in section 2 (bit 165 in the plain form, 199 in the
Alignment Properties form, 241 in S59) is computed by Civil 3D from the
alignment's geometry: unchanged by saves, renaming and the proxy-graphics
setting; a length or radius change rewrites most of it, a move or rotation
changes 28 of its bits and keeps 38 (bytes 1–4 and the low 6 bits of byte
5). Civil 3D accepts zero bits and recomputes it on save (on alignments
with a JSON layout after a warning and a re-solve of the groups, S214 →
S215). civil3drust
treats it as a cache: it is not decoded, and every created or edited
alignment gets zeros (D-046, `aecc_build::geometry_key_bit`). Details and
the full table: `mysteries.md`, MIST1.


## The user string of the Alignment Properties form (S227)

The Alignment Properties form has one more string after the two
descriptions: the user who edited it ("felix" in S144, S177, S178). The
plain form has none and reads string 3 as the name. `to_plain_form`
(D-057) kept the string, so S193 and every alignment created from a
Properties-form template (S227) showed "felix" as its name in Civil 3D
(owner, S227 screenshot); S194, Civil 3D's re-save of S193, keeps it as
the name. `to_plain_form` now removes it, and `create_alignment` also
clears the template's descriptions. The name reader follows the
section-2 form as Civil 3D does (`raw::section2_form`: string 3 in the
plain form, 4 in the Properties form), so it now reads S193, S194 and
S227 the way Civil 3D shows them. S228 is the template made again.

## Deleting an alignment (S228 → S229)

The owner deleted `Alignment - (1)` (F9E) from S228 in Civil 3D and
saved (S229). Record by record (`hexview --diff`), besides the usual
re-save changes (styles, settings, MIST2 added, XRECORDs and class
nodes renumbered):

- **removed:** the alignment F9E, its station-equation label group FA3
  and its design-speed label group FA4 (both point at F9E with a soft
  pointer; the design-speed group is one Civil 3D made itself, F9E being
  S24's own alignment);
- **companion F9D kept but emptied:** 231 → 117 bytes, its points and its
  soft pointer to F9E (with three null references) removed, key, ID and
  owner (registry) kept;
- **registry entry kept:** the registry's list still holds F9D (order
  changed, as on any save, S54/S55);
- nothing else refers to F9E (the alignment style keeps no back
  reference).

So a deletion is: drop the alignment entity and every label-group
entity pointing at it, and empty its companion. A site member (not in
this sample) would also have to leave the site's collection
(`AECC_PARCEL_NODE`, S101–S112).

**The emptied companion.** The node list keeps its first BL (next free
ID, 10 in F9D) and gets no nodes; the edge list, the edge, the `BL 0 ×4`
and the tail entry `2, 1, 1, 1, 7` are replaced by `BL 0, 0, 2, 1, 2,
2`; the run before the strings and the strings stay; the handle stream
keeps only its first reference (the registry, owner).

**civil3drust (D-062).** `aecc_delete::delete_alignment(doc, handle)`
does the same on a loaded drawing: removes the alignment and every
`…_LABEL_GROUP` entity that refers to it, and empties its companion
(`empty_companion`); the registry entry stays. Alignments in a site are
refused (postponed to the site work, owner). On S228 it gives S229's
records: the same alignments left, the companion identical to Civil 3D's
apart from object-header bits a save rewrites and handle-stream padding
(test `delete_alignment`). `examples/delete_alignment` is the command
line form; a program that edits the model calls the same function.
S230: S228 with `Alignment - (1)` deleted this way. Owner: S230 opens with no issues, AUDIT clean;
its re-save S231 reads back to the same seven alignments, the companion
still empty (D-049 acceptance).

## Station-anchored data: a speed on a spiral, masking, a station equation (S243)

S243 (owner, S24-based): three alignments, each a 100' tangent, an
outward spiral (R 300 → ∞, L 200) and a floating line, the last two also
a floating curve R 100; design speeds 60 at 0+00 and 30 at 2+00.

- `Spline speed change` (FD7): the 30 mph speed at 2+00 lies on the
  spiral (1+00–3+00);
- `masking` (FDF): a masking region 2+90.30–3+34.11 (Alignment
  Properties → Masking);
- `station equation` (FE5): one station equation, back 1+90, ahead
  3+10.

**The interval block holds every station-anchored object.** Its 13
"constant" bits are `B 1, BL kind, BD 0`: kind 2 for a design speed (all
earlier samples), 3 for a station equation, 4 for a masking region. FDF
has four intervals (0, 2+90.30, 2+00, 3+34.11: the speeds and the
region's start and end), FE5 three (0, 2+00, and 1+90 for the equation).
The intervals of the region and the equation carry the stations Civil 3D
took from the cursor (190.2994155883789, 89.99996185302734: single
precision), the speeds' exact ones.

**Speed on a spiral (FD7).** The interval has the same form as on lines
and curves: station 200, distance 100 from the spiral's start, length 100
to its end, element 2 (before 1, after 3), the spiral's start and end,
and the point at 2+00 on the clothoid (198.7772, −13.8094). civil3drust's
point differs from Civil 3D's by 3e-10.

**Station equations (FE5).** The `BL 0` that came before the design
speeds' `BL 1, BL n` in every earlier record is the station-equation
count. FE5 has one:

```text
BL 1 (count),
{ 18 bits `00` + `BE 04` (BS 1214?), BD 190 (raw station), GUID (its
  interval's), BD 190 (back), BD 310 (ahead), B 1 },
BL 1, BL 2, … (design speeds as before)
```

**Masking (FDF).** The `0` of `BL 2, 2, 2, 0` before the implied-PI
display (MIST6) is the masking-region count. FDF has one:

```text
BL 1 (count),
{ BD 290.299391746521 (start), BD 334.1108031681572 (end), B 0,
  GUID (start interval), GUID (end interval) },
BL 3 (implied-PI display)
```

The unexplained bits (the equation's 18 bits and last bit, the region's
bit) are MIST18.

**Strings.** The counts fit `9 + 2 per interval + 1 per speed + 1 per
masking region + 2 per station equation + 3 per non-line element (+ 1
JSON, + 1 user string in the Properties form)`: FD7 19, FDF 27, FE5 26
(D-056 with intervals counted by kind). FE5's equation strings include a
`"1"`.

**civil3drust.** `aecc_station_refs` reads the intervals (with their
kind), the station equations and the masking list
(`aecc_build::masking_regions`); the design-speed reader, the type and
PI-display readers and `required_string_count` follow the new layout
(the old reader failed on FE5). `set_design_speeds` now writes speeds on
fixed spirals and keeps the other intervals, the equation list and the
masking list as they are; moving the start station is refused while an
alignment has equations or masking regions (not sampled). Rewriting the
same speeds gives FD7's record back except the last digits of the
spiral's end and point; FDF and FE5 keep their region and equation
(test `station_refs`).

**Masking check boxes (S246, owner).** The owner gave FDF four regions
(Masking tab: 1 locked to the start, 0+00–0+69.20, comment `R1`; 2
1+39.93–1+81.09 with Mask off; 3 2+90.30–3+34.11; 4 3+63.40–4+30.86
locked to the end). The region's bit is 1 only for region 2: it is the
inverted Mask check box (`MaskRegion::masked`). The locks are not stored
as flags: the locked regions keep stations 0 and 430.859 and ordinary
kind-4 intervals. All eight region intervals and the comment string fit
the string rule (42 strings). Test `s246_masking_regions`.

The lock is Civil 3D's reading of the stations: with Lock To Start
cleared but the start left at 0+00, it is back after save and reopen
(owner). civil3drust reads the locks the same way (first region at the
start, last at the end, within 1e-6) and then treats them as model data
(D-068): `MaskRegion::range` gives a locked side the alignment's own
start or end, an unlocked side its recorded station.

**Two station equations (S247, owner).** FE5 given a second equation
before the first: 1+10 back → 1+09 ahead, then the old one at raw 1+90,
now back 1+89 → 3+10 ahead (Station Control tab). The list holds both,
in station order, each with its interval (kind 3) and its strings (the
equation numbers `"1"`, `"2"`). Station Back of the second equation is
derived from the first (raw 190 − 110 + ahead 109 = 189), as the
Station Control help describes. The 18 leading bits are the same in both
(MIST18); the trailing bit is 1 in both, both Increasing (the direction
after the equation, so a lower ahead station is still Increasing). Test
`s247_two_station_equations`.

**Decreasing (S248, owner).** Increase/Decrease is a selection list in
the grid, not derived from the stations. With equation 2 set to
Decreasing its trailing bit is 0 (equation 1 keeps 1):
`StationEquation::increasing`. Comments typed for both equations show
the two strings per equation: the comment, then the number (`"First
record"`, `"1"`, `"Decreasing start"`, `"2"`). Test
`s248_decreasing_equation`.

**Stations with equations.** `aecc_station_refs::display_station`
gives the station Civil 3D shows at a raw station (start + distance):
past an equation's Raw Station Back the stations continue from its
Station Ahead, up or down by Increase/Decrease; Station Back is derived
(`station_back`). S247's end is 430.859 raw → 310 + 240.859 = 5+50.86,
the End the owner's Station Control tab shows; with equation 2 Decreasing
(S248) it is 310 − 240.859 = 0+69.14. Element stations in civil3drust
stay raw; `examples/alignments` prints both. Test
`stations_with_equations`.

**Moving the start station (S249, owner).** S248 with each alignment's
start station changed through the Station Control tab's reference point
(FD7 and FE5 to 0+05, FDF to 0+00.50). The record stores only the new
start station (the BD at the end of section 2); no reference-point
coordinates appear anywhere. Civil 3D:

- removes the design speeds (all three; as S82);
- moves every masking region by the same amount (0.5), locks kept;
- moves each equation's Raw Station Back with the start (110 → 115,
  190 → 195, last digits re-rounded) and keeps Station Back and Station
  Ahead as stored (110 / 109, 189 / 310), so the stored Station Back of
  the first equation no longer equals its raw station;
- moves the intervals of the regions and equations with them.

civil3drust still refuses start-station moves on alignments with
equations or masking regions (D-067); S249 shows what doing it the Civil
3D way takes. Test `s249_start_station_moved`.

**Writing equations and regions (D-070; S250 → S251, owner: clean open
and AUDIT).** S250 had civil3drust write equations and regions on all
three alignments. On FDF and FE5, Civil 3D's re-save S251 reads back to
the same model: Civil 3D gave the regions new GUIDs and re-rounded some
raw stations (119.9999999254942), nothing else. On FD7 the first region
had been given the same two GUIDs as the two equations (the example used
one seed for both edits): Civil 3D merged the anchors, keeping one
interval per GUID, and the region took the equations' stations
(1+50–3+20). Since then new anchors never reuse a GUID the record has
(speeds included). S252 is S250 made again that way. Tests
`s251_resave_and_unique_guids`.

**Acceptance (S252 → S254, owner).** S252 (S250's edits with unique
GUIDs) opens clean, AUDIT clean, stations and masks as written; its
re-save S254 reads back to the same equations, regions and speeds
(intervals reordered). Test `s254_resave_reads_back`.

**Inside a JSON group (S253, owner).** S178 11D4 with a masking region
and a station equation inside its JSON group (Free Reverse SSCSS).
Civil 3D anchors them to the **group as one element**: the intervals
name the group's ID and links (3, before 1, after 2), the group's start
and end points, the distance from the group's start and the length to
its end (the sum of its pieces); only the point lies on the piece that
holds the station. `write_interval` now treats elements sharing an ID as
one, for JSON groups (other groups still refused, unsampled); written
from civil3drust's own solution of the group, all three intervals match
Civil 3D's within 1e-6. After the edit 11D4 has 15 surplus empty strings
**after** the ones the string rule counts (56 for 41 + 1 user), so the
writers place the tail from the counted end, not the last string. S243
FD7's section-2 flag BL is 0x1C1 (flags 0x80 and 0x100 besides 0x41,
meanings unknown); the form reader now accepts any flag value with
0x01. Test `s253_refs_inside_a_json_group`.

**S244 → S245 (owner).** S244 (S243 with every alignment's speeds
rewritten by civil3drust: two on FD7's spiral, one in FDF's masking
region, one after FE5's equation) opens clean, AUDIT clean, "behaviour as
expected". Its re-save S245 reads back to the same speeds, region and
equation. Civil 3D recomputed the stations of some intervals from their
points (FDF's speed 320.00000000045003, FE5's equation 190.0000000372529,
also its raw station) and reordered them; the equation's back and ahead
stations and the region's stations are kept.

## Production drawing check (2026-11-05)

A Civil 3D 2024–schema production drawing with 26 alignments (local
only, not stored; described generically) was read with every reader:

- **The interval block is shared with many more objects.** Alignments
  used by labels, profiles and other objects carry hundreds of intervals
  (up to 1,203), with kinds far beyond 2–4 (values up to ~700, and one
  near `0x7FFF` per alignment); every interval has the usual form and two
  strings, so the string rule still holds (1,908 strings for 949
  intervals, …). The three BLs that open the block are small counters
  there (`1–3, 0–1, 0`), 0 in every purpose-made sample. Both are now
  accepted (`aecc_station_refs::intervals_at`), and speed rewrites keep
  the counters.
- **Element records are contiguous.** The element scan used to search the
  whole record and found chance matches inside the large interval blocks
  (two alignments failed as "pieces hold 1 of 2 elements"). It now stops
  at the first gap: each record starts right after the previous one and
  its 11-bit kind prefix (D-069).
- **Types 2 and 3.** One offset alignment (type 2) and three curb returns
  (type 3) — the API's Offset and CurbReturn values, as named. Their data
  after `BL type` is not `2, 2, 2` and a masking list; the offset form
  was decoded later from S255 ("Offset alignments"); the curb returns'
  is not.
- **Alignments from feature lines** have a section-2 form starting `BL
  0x40, BL 0, …` without the MIST1 anchor, and two more strings per
  element (262 tangents → 524).

Result: all 26 decode their elements; type, speeds, station equations,
masking regions and PI display read on 21 of 26.

## Spiral types (S232)

The owner filled S228 (one named 100' tangent per type) with an outward
spiral after each tangent (R 200 at the tangent end → ∞, L 200,
clockwise), each laid out with a different type in Curve and Spiral
Settings (S232). The BL after a spiral record's IDs (MIST10) is the type;
the records differ in nothing else.

| code | type (Civil 3D's list) | shape fitted to the proxy graphics |
|------|------|-----------|
| 256 | Clothoid | curvature k(s) = k0 + (k1 − k0)·t, t = s/L |
| 257 | Sine Half-Wave Diminishing Tangent | y'' = (1 − cos(πx/X))/2R, X = L − c·L³/R², c = (1/3 − 3/2π²)/8 |
| 258 | Cubic (JP) | y = x³/6RX, X + X³/40R² = L |
| 259 | Bloss | k(s) = k0 + (k1 − k0)(3t² − 2t³) |
| 260 | Cubic Parabola | y = x³/6RL, arc length to X equal to L |
| 261 | Sinusoidal Curves | k(s) = k0 + (k1 − k0)(t − sin(2πt)/2π) |
| 262 | Bi-Quadratic | k(s) = k0 + (k1 − k0)·2t² (t ≤ ½), 1 − 2(1 − t)² |
| 264 | NSW Cubic Parabola | not known: S234 opens and lists the type, but Civil 3D 2024 draws no curve (FUTURE.md, FRT-1) |

x and y are measured from the straight end along and across the tangent
(X the spiral's extent along it); an outward spiral is the same curve
walked from its curved end. The shapes are the textbook definitions of
these curves; the X rules for 257 and 258 were found by fitting the
spiral's end (S232) and come from one L/R pair only — the end matches
within 3e-5, every graphics vertex lies on the curve (the test's
sampling, 0.005). For 257 and 258 the true arc length is not L (199.57,
199.86 here); civil3drust takes stations along them proportional to the
true arc length, which only the ends confirm.

**Autodesk's help, "About Spiral Definitions"** (public user docs, read
for names and meanings; its equations are images and were not used):
the types are Clothoid, Bloss, Sinusoidal Curves, Sine Half-Wavelength
Diminishing Tangent Curve, Cubic Parabola, Cubic (JP), NSW Cubic
Parabola and Bi-Quadratic (Schramm). Meanings that bear on the shapes
above: Sine Half-Wave and Cubic (JP) are written in x along the initial
tangent and X, the total x at the transition's end (as fitted); Cubic
(JP) is an approximation of the clothoid for small deflections; the
Cubic Parabola's radius reaches a minimum near 24°05′41″, so it is not
usable past about 24° of deflection; NSW Cubic Parabola is a modified
cubic parabola defined by Φ (between the final radial line and the
perpendicular to the initial tangent), R, Xc and Yc (total X and Y) —
still to be sampled; Bi-Quadratic is two second-degree parabolas, with
its own formulas for the compound case (civil3drust's compound
Bi-Quadratic, the same rise between two curvatures, is unchecked);
Sinusoidal Curves cover deflections from 0 to 90°.

Code 263 crashes Civil 3D on open (S233).

The copied "NSW Cubic Parabola" alignment kept 262 and the Bi-Quadratic
shape (MIST10): changing the type in the settings does not change an
existing spiral.

**civil3drust.** `spiral_type::SpiralType` (codes, names);
`SpiralGeometry::spiral_type`, set from the record, picks the shape in
`point_at` (`spiral_type::local_point`). Placements civil3drust computes
itself — spiral groups, free spirals, JSON groups, line-spiral groups —
stay clothoids (their records also carry the code, but no other type has
been sampled there); unknown codes are drawn as clothoids. Compound
spirals (two finite radii) of the arc-length types use the same rise;
the tangent-defined types have no compound form and fall back to the
clothoid. Test `spiral_types`.

## Offset alignments (S255–S258)

**Sample.** S255 (owner, Civil 3D 2024): "Spline speed change" (FD7:
tangent (0,0)→(100,0), outward clothoid R 300 → ∞ L 200 clockwise,
tangent; start station 5) with two offset alignments over its whole
length, 10' to the left (115F, "…-Left-10.000") and 10' to the right
(1170, "…-Right-10.000"). Compared: the two offsets with each other (they
differ only in the offset's sign, the side, the end station and GUIDs)
and with the parent.

**Elements.** Ordinary element records with the offset geometry, IDs 7–9
(the parent has 1–3):

- Tangents: kind 0 (fixed) with both points moved sideways by 10.
- The spiral: kind prefix 6, a new spiral kind **5** after the parameter
  block (`B 0, BD −1, BD −1, BL 1, BD −1`):

  ```text
  BL 5, BL id, BL before, BL after, BL 256 (type: clothoid), B clockwise,
  RD x, y      the parent spiral's straight (∞) end: here its END
               (294.1157, −43.8826); its start for an inward spiral (S256)
  RD dx, dy    the parent's direction of travel there
  BD 300       start radius
  BD 200       length (the parent's)
  BD inf       end radius
  BD −10       offset: negative to the LEFT (+10 on the right offset)
  BD 0, BD 1   the part of the parent followed, as fractions of its
               length (S256: 0 to 0.8416 when the offset stops at a
               parent station inside the spiral)
  ```

  The element is the parent spiral moved sideways: running the parent
  back from the stored end gives its start (100, 0); every point moved by
  10 along the left normal gives the offset curve, which meets both
  offset tangents (within 3e-10) and matches the proxy graphics (every
  vertex of the curve polylines within 0.01; the one vertex off is on a
  two-vertex tick mark, as on the parent). Its length is the parent's changed by the
  offset times the turn (1/3 rad here): 203.333 outside the turn, 196.667
  inside — the end stations Civil 3D stores (408.33, 401.67) agree.
  S256's inward spiral settled the stored point: the straight end. A
  compound spiral (no straight end) is not sampled; civil3drust reads
  the point as its start there.

**After the type** (`BL 2`). A centerline's `BL 2, 2, 2`, masking list
and PI display are replaced by:

```text
BL 1, BL 2, BL 1                 update mode (1 dynamic, 2 static,
                                  S265), 2 (MIST19), 1 (2 once the parent
                                  is deleted, S256, S262)
BD −10                           offset, negative to the left
BL 1                             side: 1 left, 2 right
B, B                             start / end station locked to the
                                  parent's start / end (S264; the first
                                  is the start, S267)
BL 1                             lock mode: 1 Geometry Locking, 2
                                  Station Locking (S270)
BL 1                             number of regions (3 with a widening,
                                  S257: "Widenings" below); per region:
BD −10                             its offset
BD 5, BD 405                       the parent's station range followed
BD 5, BD 408.33                    the offset alignment's own stations
BL 2, BL 7, BL 9                   kind (2 nominal), first and last
                                    element ID as made (not renumbered
                                    later, S256)
BL 0 × 5, BL 3                     (MIST19)
BL 0                             number of widening transitions
BL n,
  n × {BL i, BL −1, BL 1, BL id} i = 1…n, one per element (S256's 11B1
                                  lists 11, 13, 14, 12, 15: tangents,
                                  then the arc, then the spiral)
…                                (MIST19: BL 2, BD 5, GUID, BD 405, GUID,
                                  then likely BL 0 (masking), BL 3 (PI
                                  display), as on a centerline)
```

**Handle stream and associations.** The soft pointers at the start of
the handle streams are reactors (the common object header's reactor
count goes 0 → 1 on the parent in S260 and back in S261): the offset
alignment has one, its own `AECC_ASSOC_DEPENDENCY`; the parent has one
per offset, the dependency on it. The offset's stream ends with one
more reference after the check set: a soft pointer to the parent. Each
offset has an `AECC_ASSOC_ACTION` (owned by the model space's
associative network) that hard-owns an
`AECC_ASSOC_OFFSET_ALIGNMENT_ACTION_BODY` (14 bytes: a hard pointer to
the offset alignment) and two `AECC_ASSOC_DEPENDENCY` records (22 bytes:
one with a soft pointer to the parent, one to the offset; with several
offsets the parent's dependencies are chained to each other), each
hard-owning an 11-byte `AECC_ASSOC_DEPENDENCY_BODY`. The offset
parameters live in the alignment record, not in these.

**Created and deleted** (S259 → S260 → S261, owner: one single-tangent
centerline, an offset 10' left added, then deleted; object inventories
compared). Adding the offset created, besides the offset alignment
(8D71) and its companion (8D70, registered like any alignment's):

| handle | object | owner / role |
|---|---|---|
| 8D72 | dictionary `ACAD_ASSOCNETWORK` → 8D73 | the model space's extension dictionary (8D72 is added to it) |
| 8D73 | `AcDbAssocNetwork` | the model space's network; holds action 8D76 (owned) |
| 8D74 | dictionary `ACAD_ASSOCNETWORK` → 8D75 | the named objects dictionary (entry added) |
| 8D75 | `AcDbAssocNetwork` | the drawing's top network; lists 8D73 (not owned) |
| 8D76 | `AECC_ASSOC_ACTION` | owned by 8D73; owns 8D77, 8D78, 8D7A |
| 8D77 | `AECC_ASSOC_OFFSET_ALIGNMENT_ACTION_BODY` | hard pointer → 8D71 |
| 8D78 / 8D79 | dependency on the parent / its body | parent's reactor |
| 8D7A / 8D7B | dependency on the offset / its body | offset's reactor |

Also changed: the parent's reactor list; a new `AECC_SETTINGS_NODE`
holding `AeccCreateOffsetAlignment` (the command's remembered dialog
values, kept after the delete — not model data); the parent record
regenerated from the plain form S259 had (as any edited alignment,
D-057); the save-time records (`AECC_LASTSAVED_APPVERSION` and another
xrecord) rewritten. The two networks were the drawing's first, so this
is what an offset adds to a drawing without associations.

Deleting the offset removed the alignment, the action with its body,
dependencies and their bodies, both networks and both dictionaries
(entries gone from the model space's extension dictionary and the named
objects dictionary), and the parent's reactor — S261's parent differs
from S260's only there. The companion stays, orphaned and still
registered, as for any deleted alignment (D-062).

**Parameters edited** (S260 → S263 → S264 → S265, owner, one step per
save in the Offset Parameters tab):

- **Nominal offset 10 → 20 left** (S263): the nominal offset, the
  region's offset and the element (now at y = 20) change; nothing else
  in the offset. The parent's station anchors for the range are
  rewritten with fresh GUIDs.
- **Stations 1+00–9+00, locks cleared** (S264): the two lock bits clear;
  the region's parent range becomes 100–900 and the element is rebuilt
  over it; the offset keeps its own stationing from 0 (0–800; S256's
  offsets were made over 100–900 and start at 100). The range is
  anchored on the parent: two parent intervals at the range's parent
  stations whose GUIDs are the two in the offset's block (`BD parent
  start, GUID, BD parent end, GUID` after the transitions). Their
  `kind` field is 6 for set stations; with the locks set it was 0 at
  the start and 90 at the end (S260, S263; S255's parent: 0 and 237;
  S267 settles which is which).
  The parent also holds a second start/end pair (0 / 90) no other
  record names.
- **Dynamic → static** (S265): the update mode becomes 2 and the
  offset's element is locked (element header flag 0x02); the parent,
  the association objects and the rest of the record are unchanged.
  Civil 3D no longer rebuilds the elements from the parent, which the
  lock records.

**Parent edited** (S260 → S266, owner: the parent's tangent shortened
from 1000 to 500, the offset dynamic with both stations locked). Civil
3D rebuilt the offset and saved the result: element 0–500, region and
own stations 0–500, fresh anchor GUIDs on the parent (start kind 0 at 0,
end kind 90 at 500). The parent's second start/end pair kept its end at
1000, past the parent's new end — it follows nothing. Whether Civil 3D
also rebuilds an offset saved stale (elements not matching the parent)
was experiment S268 (the S266 offset's tangent set back to 1000):
Civil 3D opens it cleanly and keeps the stale line — drawn, and saved
unchanged (S272); AUDIT rebuilds it to 500, with the caches and fresh
range anchors on the parent (S273). So the elements are model data a
writer must compute; only AUDIT or an edit makes Civil 3D redo them.

**One lock cleared** (S266 → S267, owner): "Start station – lock to
start of parent" kept, the end's lock cleared at 5+00. The first lock
bit stays set, the second clears; the parent's start anchor keeps kind
0, the end anchor becomes kind 6 at 500. So kind 0 = locked to the
parent's start, 90 = locked to its end (237 in S255), 6 = a station.
Edits in the Properties dialog also switch the record to the Properties
form with the author and a date (S264, S267; MIST3), as for any
alignment.

**Update mode, lock mode, superelevation type** (S267 → S269 → S270 →
S271, owner):

- **Static** again (S269; end station 4+00): update mode 2, the element
  locked — as S265.
- **Station Locking** (S270; the offset dynamic again): the BL after the
  lock bits goes 1 → 2, so it is the lock mode. The parent's start
  anchor turns from kind 0 to kind 6 although "lock to start of parent"
  stays set: with station locking both ends are anchored as stations
  (kinds 0 / 90 belong to geometry locking).
- **Cant** (S271): the lock mode reads 1 again and the start anchor kind
  0 again (whether the owner set Geometry Locking back is to be
  confirmed). The offset record's section 2 changes: a BD just after
  the value 2.25 goes from 0 to 4.92125 (1.5 m — a track gauge), and 42
  more bits appear earlier in section 2 (not decoded). Superelevation
  versus cant is stored there, not in the offset block.

**Parent deleted, offset kept** (S260 → S262, owner). Removed: the
parent with its label groups (its companion stays, D-062), and the
whole association — action, body, both dependencies and bodies, both
networks and both `ACAD_ASSOCNETWORK` dictionaries — exactly as when the
offset itself is deleted (S261). The offset alignment keeps its size and
geometry; only its block head turns from `1, 2, 1` to `2, 2, 2`, its
reactor goes and the parent slot after the check set becomes a null
soft pointer (as in S256). Its region list, offset, side, stations and
element IDs are unchanged (S256's renumbering came from something else
done in that file). Also replaced: an `AECC_CLASS_NODE` and its
`AECC_USER_DEFINED_ATTRIBUTE_CLASSIFICATION` (same content, new
handles) and the save-time xrecords.

**Production check.** The production drawing's offset alignment (one
tangent, 14 to the left) reads the same way: offset, side and stations
as its name and geometry say. Its handle stream has two dependency
pointers first, and soft pointers to a profile and a profile view
between the check set and the parent.

**S256** (owner's file; steps read from the drawing). S255 with the
parent FD7 deleted, plus TEST (119F: tangent, free arc R 236.03,
tangent, tangent, inward clothoid ∞ → R 100, L 200) with three offsets:

- **Parent deleted** (115F, 1170): still type 2 with the same geometry;
  the block head reads `2, 2, 2`, the dependency pointer is gone, and
  the parent slot after the check set is a null soft pointer. The
  elements were renumbered 1–3, the block's first/last IDs still say 7, 9.
- **Over part of the parent** (11B1 25 left, 11C2 10 right, parent
  stations 100–900; the offset alignment starts at station 100): the
  arc's offset is an ordinary fixed three-point arc (R 261.03 / 226.03,
  same center); the spiral record stores the parent spiral's start and
  the fractions 0 to 0.8416 (station 900). Decoded lengths plus 100
  match the stored end stations to 1e-9. These records have bit 16 of
  the 41-bit gap before the first element set (centerlines and S255:
  0); the start-station check no longer tests it.
- **An offset of an offset with a widening** (11D9, "…-Left-0.000",
  parent 11B1, offset 0): its elements are offsets of TEST (the spiral
  record holds TEST's spiral with offset −25 and fractions
  0.1758–0.8416) with a widening of 30 more to the left and linear
  transitions (S257 read the rest; "Widenings" below).

**Widenings** (S257, S258; owner's screenshots of the Offset Alignment
Parameters). S257: two single-tangent centerlines, each with an offset
at 0 and one user-specified widening, 26 to the left over parent
stations 112.17–260.54, with transitions of length 75 in and out:
linear (1218) and Curve – Line – Curve with curve 1 radius 45 and
curve 2 radius 25 (1234). Each element list holds tangent 3 (nominal),
transition in, tangent 4 (widened), transition out, tangent 5 (nominal).
S258: centerlines with a tangent, an arc and a free spiral, offset 10.28
left, widened to 57.055 left from 1+12.17: onto the arc (1297, the
widened stretch is tangent + arc), ending just after the arc's start
(12A6, transition out onto the arc), out over the spiral (12B4, linear),
and the Curve – Curve – Reverse Curve (12C2) and Curve – Reverse Curve
(12D0) types.

- **Regions.** The block after the type keeps one entry per stretch at
  one offset; S255/S256 have one, a widening makes three (nominal,
  widening, nominal):

  ```text
  BL update mode, BL 2, BL 1, BD nominal offset, BL side, B lock start,
  B lock end, BL lock mode,
  BL n regions,
  n × { BD offset, BD parent start, BD parent end, BD start, BD end,
        BL kind (2 nominal, 1 widening), BL first, BL last element,
        BL 0 × 5, BL 3 }
  BL m transitions,
  m × { BL type, BD length (along the parent), BL element,
        1 linear:                       BD 0, BD 0, BL 2, BD 0
        2 curve – line – curve:         BD 0, BD R1, BD R2
        4 curve – curve – reverse curve: BD 0, BD R1, BD R2, BD R3
        5 curve – reverse curve:        BD 0, B 0, BD R1 }
  ```

  1234: regions (0, 0–37.17, own 0–37.17, element 3), (−26,
  112.17–260.54, own 117.12–265.49, element 4), (0, 335.54–415.40, own
  345.44–425.29, element 5); transitions (2, 75, element 8, 0, 45, 25)
  and (2, 75, element 9, 0, 45, 25). The own stations are those of the
  drawn curve, so they differ between the transition types (116.55
  linear, 117.12 curve – line – curve, 130.14 / 130.32 for S258's
  reverse-curve types). The curve 2 radius of type 4 is the one Civil 3D
  shows greyed (derived: 60.033 in, 71.891 out); type 5 stores no
  reverse radius (Civil 3D shows 38.516 / 126.914). Type 3 is not
  sampled. A transition out that ends on the arc measures 74.988 (the
  75 the owner entered, along the parent; MIST19).
- **Linear transition records** (prefix **22**):

  ```text
  params (B, BD −1, BD −1, BL 1, BD −1)
  BL 3, BL id, BL before, BL after,
  BL m, m × { BL 0x1000, BL k, k × RD x, y }
  ```

  One point list per part of the parent crossed: 2 points over a
  tangent (S257's 6 and 7; S256's 17), more over a spiral (S256's 18:
  2 over the tangent, then 3 over the spiral; S258's 12B4: 3 over the
  spiral, then 2 over the tangent). Consecutive lists share their
  joining point. Civil 3D's stations agree, to 1e-9, with straight lines
  through the points, and so do its graphics.
- **Curved transition records** — only the radii; the curve is placed
  between the end of the element before and the start of the one after
  (on a tangent, an arc or a spiral). Curve 1 starts there and turns
  toward the other offset:

  | prefix | type | record after the header | shape |
  |---|---|---|---|
  | 34 | curve – line – curve | `BL 0, id, before, after, BD R1, BD R2` | curve 1, the line touching both circles, curve 2 (reverse) |
  | 36 | curve – reverse curve | `BL 0, id, before, after, BD R1` | curve 1, then the reverse curve tangent at the end whose circle touches curve 1 — its radius is derived (38.516 / 126.914, as Civil 3D shows) |
  | 38 | curve – curve – reverse curve | `BL 1, id, before, after, BD R1, BD R2, BD R3, BD length, B` | curve 1, curve 2 (same turn, touching curve 1 inside), reverse curve 3 tangent at the end; of the two placements of curve 2, the shorter path |

  The B ending the type-38 record was 1 on the transition in and 0 on
  the transition out (which was also locked, flag byte 0x02). Every
  region's stored start and end station agrees with the placed curves
  to 1e-6, in all S257/S258 samples, and every vertex of the graphics'
  curve polylines lies within 0.01 of them (two-vertex polylines are
  tick marks and are not compared).

**Not sampled.** A compound spiral in the parent; transition type 3; a
static offset; the parent edited after the offsets were made.

**civil3drust.** Read-only. `SPIRAL_OFFSET` records decode and re-encode
bit for bit; `SpiralGeometry::offset` (positive to the left) moves the
parent spiral, and `point_at` / `curve_length` work along the drawn
curve (D-071), over the part of the parent given by the fractions.
Widening transitions are `Element::Widening` (`aecc_widening`; D-072),
read and re-encoded bit for bit; not editable.
`aecc_offset::offset_info` reads the regions and transitions;
`aecc_alignment_refs::offset_parent` is the parent (none after it is
deleted). Test `offset_alignments`.

## MIST2: re-save field (partially solved, S151–S157)

`BL 0x0D00D773, BL n` (44 bits) right after MIST1, or 98 bits after it in
the Alignment Properties form. Civil 3D adds it on the first re-save and
switches the full BL at section-2 bit 115 from `0x1148` to `0x1188` in the
same save. The tag is checked (zeroing it is "eWrongObjectType", S151);
n is 2 for centerlines and 1 for Miscellaneous alignments and is kept as
stored (S156). Removing the field while the marker says `0x1188` is
rejected (S154); removing both is accepted (S155), and Civil 3D restores
them on its next save (S157). civil3drust removes the field and sets the
marker back when it creates an alignment or writes geometry (D-047,
`aecc_build::remove_resave_field`, `drop_caches`).
