# Alignment record map — what is known, what is needed

A region-by-region map of everything Civil 3D stores for an alignment,
with how well each part is understood and whether a new alignment needs
it. First built from the Civil 3D–written alignment records of S2–S55
(two templates, 1–6 elements, tangents and free curves), compared region
by region, and corrected with the later samples. Details and evidence for
each finding are in [`alignment-object.md`](alignment-object.md); the
open fields are in [`mysteries.md`](mysteries.md).

Design direction (D-026, D-049): an alignment is modelled by its
meaningful parameters only (name, description, style, start station,
elements, speeds, options); the regions marked *derived* are computed
when writing or left for Civil 3D to rebuild.

Status legend:

- **decoded** — meaning known; civil3drust computes it.
- **constant** — identical in every Civil 3D record in the samples; written
  as a documented constant.
- **varies (unknown)** — differs per alignment; meaning unknown.
- **optional** — present only in some records; a new alignment can omit it
  (tested) or is expected to (untested).

## 1. The `AECC_ALIGNMENT` entity record

Bit offsets for S44's `Alignment - (3)` (FF3, two tangents, 566 bytes);
other records shift by the sizes of the variable parts.

| bits (FF3) | region | status | needed for a new alignment |
|---|---|---|---|
| 0–17 | object type (class number, `10` + RS; `01` + byte, 10 bits, for classes 500–751, e.g. the companion in S123/S124) | per drawing (positional class table) | yes — the target's class number; the class entry is added when missing (S65/S66) |
| 18–41 | own handle (`0x0n` + n bytes) | decoded | yes — `handle_refs::set_own_handle` |
| 42–43 | EED size `10` (no extended data) | constant | yes |
| 44 | has-graphics flag | decoded | yes (0 = no graphics, accepted: S57) |
| 45–55 | graphics size (BLL, bytes) | decoded | only with graphics |
| 56–1783 | **proxy graphics** (display cache): 7 attribute chunks, one polyline per tangent, an attribute triple after each arc; with PROXYGRAPHICS 0 Civil 3D writes its placeholder instead (S141) | decoded; derived from geometry | **no** — Civil 3D regenerates it (S57); kept by default for programs without Civil 3D |
| 1784–2196 | **section 2** (413 bits in the plain form), see 1.1 | decoded except MIST1–MIST4 contents | yes |
| 2197–2206 | element count (BL) | decoded | yes |
| 2207–2856 | **element records**, each with an 11-bit kind prefix (2 line, 4 curve, 6 spiral — kind 5 within it is an offset alignment's spiral, S255 —, 8 / 10 / 20 / 24 / 26 spiral groups and free spirals; 22 / 34 / 36 / 38 an offset alignment's linear / curve – line – curve / curve – reverse curve / curve – curve – reverse curve widening transition, S256–S258) | decoded (`aecc_elements`, `aecc_encode`; every kind re-encodes bit for bit) | yes — the geometry |
| 2857–2872 | interval block head: `BL 0, BL 0, BL 0, BL n` (n speed intervals) | decoded | yes |
| 2873–3000 | first **interval**'s GUID (= its design-speed entry's GUID; not an alignment identity, S82; equations and masking regions have intervals too, S243) | decoded (random v4) | with speeds — a fresh GUID (a zero GUID changes how Civil 3D saves, S57 X5, MIST13) |
| 3001–3017 | `B 1, BL kind (2 speed, 3 station equation, 4 masking), BD 0`, BD station (0), BD distance from the element start (0) | decoded | with speeds |
| 3018–3083 | BD length to the element's end (here: the station at the end of element 0) | decoded; derived | with speeds |
| 3084–3489 | rest of the interval: containing element's ID, before and after IDs (S115); element start, end, point at the station (6 RD) — alignment-object.md "The interval block" | decoded; derived from speeds + geometry | written by `set_design_speeds` |
| (after the intervals) | station-equation list: `BL n`, n × {18 bits, BD raw station, GUID, BD back, BD ahead, B} (S243; the `BL 0` before the speed table) | read (MIST18 bits open) | empty list |
| 3490–3858 | **post-cache block**, see 1.2 | decoded (speed table, type, masking, PI display) | yes |
| 3859–4394 | **string stream**: class name, description ×2, [user, Properties form], [JSON layout], name, empty strings: 9 + 3 per speed with an interval + 1 per speed without + 3 per non-line element record + 1 with a JSON layout (D-056) | decoded | yes |
| 4395–4411 | string-stream size (16 bits) + flag | decoded | yes |
| 4412–4527 | **handle stream**: layer, null, companion ×2, null, style, null | decoded | layer and **style required** (null style: invisible, crash on select, S67–S69); companion refs may be null (§2) |

### 1.1 Section 2 (graphics end → element count)

Offsets are counted from the class data's start less 37 bits (the usual common entity data, `aecc_entity::section2_base`): an entity with reactors has a 10-bit reactor count and everything sits 8 bits later (S287).

| bits (relative) | content | status |
|---|---|---|
| 0–114 | class header `BL 80, 0, 80, 80, 0` and common entity data (layer colour, linetype …; `aecc_entity`) | decoded |
| 115–164 | form BL: plain `BL 0x1148` (`0x1188` with MIST2), `BL 0`; Alignment Properties form: flag BL (0x01 Properties header, 0x02 JSON layout + `BL 0x01010034`, 0x40 MIST2), `BL 0x08010004`, 26 bits (MIST4) | decoded / handled (D-057) |
| 165–230 | MIST1, 66 bits (at 199 in the Properties form, 241 in S59) | **geometry fingerprint**, computed by Civil 3D; civil3drust writes zeros (D-046); checked for JSON-layout alignments (warning, groups re-solved) |
| (Properties form) | date-modified block `BL 0x1600101F` + FILETIME (MIST3) | dropped with the plain form (D-057) |
| +44 bits | MIST2 `BL 0x0D00D773, BL n` | **optional**, added by Civil 3D re-saves; must match the form BL; civil3drust removes it (D-047) |
| 231–396 | almost all zero bits | constant |
| last 22 bits | from the end: 13 unknown, 14 check for tangency, 15 lock all parameter constraints, 16 implied tangency swapping, 17 check set, 18 criteria file, 19 criteria-based design; the start station BD ends 20 bits before the end | decoded (`aecc_criteria`, `aecc_station`) |

### 1.2 Post-cache block

| content | status |
|---|---|
| design-speed table: BL 0, BL 1, BL n, n × {BL 1, BD station, BD speed, GUID} (the GUID shared with the speed's interval) | decoded (O2, S74) |
| BD 100, BL 0, BL n, n BDs (element-ID list, optional: Civil 3D adds it after geometry edits, S12, S51), **BL alignment type** (1 Centerline, 2 Offset, 4 Miscellaneous), BL 2, 2, 2 (an offset alignment has its offset parameters here instead, up to the PI display: alignment-object.md "Offset alignments", MIST19), **masking list** `BL n`, n × {BD start, BD end, B, GUID, GUID} (S243), **BL implied-PI display** (1 by direction, 2 by curves, 3 none) | decoded (O1, S58/S59; S122; S243) |
| 1 bit (block bit 340) | 0 in all samples |
| station-equation data (S9: +196 bits in section 2, more after the cache) | the list after the intervals is read (S243); S9's section-2 bits not decoded |

## 2. Other records created with an alignment

| record | content | status | needed |
|---|---|---|---|
| companion `AECC_FEATURE_STYLE` | own handle; key (`0x8000_0000 \| n`); points of the alignment; 32-hex ID string; refs: own−off → registry, soft pointer → alignment | key, ID, refs decoded; points regenerated by Civil 3D (S35) | **yes** — without it AUDIT reports 2 errors (S30); after AUDIT's fix null references are accepted (S32) |
| registry entry | key, soft-owner ref, ID string; sorted by ID | decoded (`aecc_registry`, also the empty registry of a drawing without alignments, S65); keys re-assigned by Civil 3D on save (S54/S55) | yes, with the companion |
| `AECC_ALIGNMENT_STAEQU_LABEL_GROUP` | refs: layer, alignment, label style | refs decoded; body copied | **no** (S27 → S29) |
| design-speed label group | — | — | no (never created for new alignments) |
| name-template counter, command settings | — | — | no (bookkeeping; untouched in S26–S55) |

## 3. What a new alignment needs

Computed by civil3drust: own handle, graphics (optional — X2), section 2
in the plain form with MIST1 zeroed and no MIST2 / MIST3, element count
and records, speed intervals and table, strings by D-056's rule, the
handle stream (layer, companion twice, style), the companion (key, ID
string, refs, topology rebuilt in the uniform form, D-040) and the
registry entry.

Taken from a template alignment (any Civil 3D alignment, D-039): the
constant bits listed above and the companion's body. Not needed: label
groups, the element-ID list, MIST1's value, MIST2, the date-modified
block (all rebuilt or ignored by Civil 3D; `civil3d-rebuilds.md`).

## 4. Experiments (S56)

S56 is S24 plus six alignments, each with one part emptied
(`examples/exp_minimal.rs`):

| alignment | change | question |
|---|---|---|
| X0 control | none | baseline (as S53) |
| X1 zero-hash | section-2 66-bit field all zero | is the field checked? |
| X2 no-graphics | has-graphics 0, no proxy graphics | does Civil 3D regenerate the display cache? |
| X3 zero-guid | both GUID copies zero | is the identity required / reassigned? |
| X4 no-label | no label group | (known optional) |
| X5 minimal | X1 + X2 + X3 + X4 | all together |

If Civil 3D rejects a change, RECOVER removes only that alignment.

**Results (S57).** The file opens with AUDIT 0 errors and **all six
alignments are kept**, including X5 (everything emptied). On save Civil 3D
rebuilds what was removed:

| alignment | after the Civil 3D save |
|---|---|
| all | the section-2 66-bit field is rewritten with a new value in every alignment (Civil 3D's own included), so it is a cache; the +44-bit section-2 field is added to every alignment |
| X0, X1, X3, X4 | new identity GUID (also replacing our generated ones), as in S47 |
| X2 no-graphics | proxy graphics regenerated (13 chunks, same as the others) |
| X3 zero-guid | new GUID assigned |
| X5 minimal | graphics regenerated, but the GUID **stays zero** and Civil 3D adds a 617-bit block and 2 strings after the cache that no other record has |

Conclusions: proxy graphics, the 66-bit field and the label group are not
needed; the GUID should be a real random value (a zero GUID is accepted but
sends Civil 3D down a different path). Separately, a parsing slip in S56's
generated records — one stray zero byte after the handle stream, from
padding kept through several rewrite steps — was ignored by Civil 3D; the
writers now trim it (`handle_refs::trim_padding`).

Civil 3D saving to DXF writes no AECC entities at all (the DXF saved with
S57 has an empty ENTITIES section), so DXF cannot carry alignments.

## 5. The "constant" blocks were default options

Most regions that looked constant in S2–S55 turned out to be alignment
settings and element constraints at their default values, identical
because every early alignment was made the same way (from a polyline, a
centerline, no criteria). Each option below was sampled from S24 with one
change (O1–O8); the element kinds and constraints are in
[`element-methods.md`](element-methods.md). Not sampled yet: O8
superelevation, station equations (S9 shows their data), offset
alignments.

| option | result |
|---|---|
| O6 floating curves, and every other layout method | **decoded** (S113, S117–S121, S162–S178): every line, curve and spiral method; adding and removing elements (S92–S100); `element-methods.md`. |
| O7 site | **partly decoded** (S101–S103): a site is an `AECC_PARCEL` face owning a container; an alignment in a site points both companion references at that container, which takes the companion's body; the old companion is emptied. One container per site holds all its alignments as a topology graph (S104–S106, not decoded); moving out creates a new companion. Read-only in civil3drust (`aecc_site`); new alignments are siteless (D-038). |
| O5 layout tools | **decoded** (S89, S113–S115): floating tangent = kind 4 with end point only; the BL after the element header is the line kind. Speed intervals on curves have the tangent form (S113); civil3drust writes them and Civil 3D keeps them (S114 → S115). S89's one curve interval is unexplained (MIST11). A table with no intervals at all is accepted (S90 → S91). |
| O4 start station | **decoded** (S82, S9): BD ending 20 bits before the section-2 end. With a nonzero start the alignments had no design speeds: interval count 0, empty speed table, 3 strings fewer — so "GUID copy 1/2" are the first speed interval's GUIDs, not an alignment identity. Interval fields: GUID, 13 bits, BD station, BD distance, BD length to element end. civil3drust shifts speeds with the start (D-031, S85). Verified: shifted speeds (S86) and speeds added to speedless alignments (S87 → S88) accepted. |
| O3 design criteria | **decoded** (S79, S81): three flags at the end of section 2, five strings (criteria file, superelevation table, lanes, attainment method), check-set reference (set on save regardless). Verified: flags + strings + check set reproduce criteria-on (S80 C3 → S81). Bits (from the section-2 end): 19 criteria-based design, 18 criteria file, 17 check set, 14 tangency check (default on); confirmed by C1's dialog. |
| O2 design speed | **decoded** (S74): the table at the start of the post-cache block; the "BD 60" constant was the default entry. Each speed has an interval after the elements (decoded since, S84, S113–S115). Verified: generated one-, two- and three-speed tables (S77) and an edited Civil 3D table (S78) work in Civil 3D; each entry needs a comment string (S76). |
| O1 alignment type | **decoded** (S58 Centerline, S59 Miscellaneous): one BL in the post-cache block, 1 = Centerline, 4 = Miscellaneous; the "constant" block was this option at its default. Verified: generated types 1 and 4 accepted and kept (S60 → S62); values 2, 3, 5 on a plain alignment are rejected and removed by RECOVER (S61 → S63), so they need a parent or other data. |

## 6. Minimal drawings (S64–S73, postponed)

| finding | sample |
|---|---|
| Civil 3D builds its infrastructure (settings, styles, registry) when it opens or saves a plain AutoCAD drawing | S64 → S65, S67 → S68/S69 |
| an alignment record alone opens; the companion may stay null | S67 |
| null style: invisible, crash on select; AUDIT assigns the default style | S67 → S68 |
| a style without an owner is dropped; an owned style outside Civil 3D's collection is ignored; both fall back to the default style | S70 → S71/S72, S73 |

Next step when resumed: write the alignment style into a minimal copy of
Civil 3D's style collection (tree node → collection → style).
