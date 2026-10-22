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

Component 0 is the line (S22). The others suggest 1–2 are curve and
spiral, but only component 0 is confirmed; the names of the rest need one
sample each.

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

## Open questions

1. The volatile field in section 2 (S5/S6 bits 2730–2789): variable length
   (62 bits in S5–S7, 106 after a Civil 3D re-save in S8; +44 bits also in
   S3→S4); what it holds.
9. The 2-bit flag at bit 2677 that Civil 3D changed when re-saving S7.
10. The block of BDs (2.0, 3.0, 4.0) Civil 3D added in S12 after the radius
    change (and again in S14).
11. The flag bit after the arc header (1 in S10, S14, S15; 0 in S11–S13).
    Civil 3D set it after geometry edits it saved (S14, S15) but not after
    its own radius edit (S12).
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
line linetype, then the curve and spiral components visible (to confirm
components 1 and 2), each from the previous file.

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

## Provenance

Every offset and value above was produced with this repository's tools on
acadrust's record bytes. An earlier exploratory pass (before this repository
existed) used LibreDWG's `dwgread` purely as a byte dumper; LibreDWG has no
knowledge of AECC classes, no LibreDWG code or documentation was used, and
all findings here were re-derived with acadrust-based tools.
