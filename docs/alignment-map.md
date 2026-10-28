# Alignment record map — what is known, what is needed

A region-by-region map of everything Civil 3D stores for an alignment,
with how well each part is understood and whether a new alignment needs
it. Built from all Civil 3D–written alignment records in S2–S55 (two
templates, 1–6 elements, tangents and free curves), compared region by
region. Details and evidence for each finding are in
[`alignment-object.md`](alignment-object.md).

Design direction (D-026): an alignment is modelled by its meaningful
parameters only (name, description, style, start station, PIs, curves);
the regions marked *derived* below are caches computed when writing, and
are candidates for omission (S56).

Status legend:

- **decoded** — meaning known; civil3drust computes it.
- **constant** — identical in every Civil 3D record in the samples (both
  templates); can be written as a documented constant.
- **varies (unknown)** — differs per alignment; meaning unknown.
- **optional** — present only in some records; a new alignment can omit it
  (tested) or is expected to (untested).

## 1. The `AECC_ALIGNMENT` entity record

Bit offsets for S44's `Alignment - (3)` (FF3, two tangents, 566 bytes);
other records shift by the sizes of the variable parts.

| bits (FF3) | region | status | needed for a new alignment |
|---|---|---|---|
| 0–17 | object type (class number, `10` + RS) | per drawing (positional class table) | yes — the target's class number; the class entry is added when missing (S65/S66) |
| 18–41 | own handle (`0x0n` + n bytes) | decoded | yes — `handle_refs::set_own_handle` |
| 42–43 | EED size `10` (no extended data) | constant | yes |
| 44 | has-graphics flag | decoded | yes (0 = no graphics, accepted: S57) |
| 45–55 | graphics size (BLL, bytes) | decoded | only with graphics |
| 56–1783 | **proxy graphics** (display cache): 7 attribute chunks, one polyline per tangent, an attribute triple after each arc | decoded; derived from geometry | **no** — Civil 3D regenerates it (S57); kept by default for programs without Civil 3D |
| 1784–2196 | **section 2** (413 bits), see 1.1 | mostly constant | yes |
| 2197–2206 | element count (BL) | decoded | yes |
| 2207–2856 | **element records**, each with an 11-bit kind prefix (tangent `…010`, arc `…100`) | decoded | yes — the geometry |
| 2857–2872 | 16 bits `1010100100000001` | constant | yes (constant) |
| 2873–3000 | **identity GUID** (v4), copy 1 | decoded (random) | yes — write a random v4 GUID (a zero GUID is accepted but changes how Civil 3D saves the record, S57 X5) |
| 3001–3017 | 17 bits `10100000010101010` | constant | yes (constant) |
| 3018–3083 | station at the end of element 0 (BD) | decoded; derived | yes |
| 3084–3489 | rest of the **first design-speed interval**: BL 1, BL 0, BL next element; element start, element end, point (6 RD) — see alignment-object.md "The interval block" | decoded; derived from speeds + geometry | written by `set_design_speeds` |
| 3490–3858 | **post-cache block** (369 bits), see 1.2 | constant except GUID copy 2 | yes |
| 3859–4394 | **string stream**: class name, description ×2, name, empty strings (8 + 3 per arc) | decoded | yes |
| 4395–4411 | string-stream size (16 bits) + flag | decoded | yes |
| 4412–4527 | **handle stream**: layer, null, companion ×2, null, style, null | decoded | layer and **style required** (null style: invisible, crash on select, S67–S69); companion refs may be null (§2) |

### 1.1 Section 2 (graphics end → element count)

| bits (relative) | content | status |
|---|---|---|
| 0–164 | | constant (all 13 records compared) |
| 165–230 | 66-bit field | **cache / don't care**: zeros accepted (S56 X1) and Civil 3D writes a new value on every save (S57) |
| 231–396 | almost all zero bits | constant |
| 397 | flag | 1 in S5–S12, 0 in S2 and S24–S55 (per template / session); untested |
| 398–412 | (includes the start station BD at −22 from the end) | constant at start station 0 |
| +44 bits | an extra field after the varying one (S4, S8, S11, S12, S48, S51) | **optional** — absent in S45/S50 (accepted) |

The 2-bit flag Civil 3D changes on re-save (open question 9) lies in this
section in records that carry the extra field.

### 1.2 Post-cache block

| content | status |
|---|---|
| design-speed table: BL 0, BL 1, BL n, n × {BL 1, BD station, BD speed, GUID} (one entry, 60 at 0, is the template default) | decoded (O2, S74) |
| 369 bits, including a BD 60 and other fixed fields | constant (9 records) — the default design-speed entry |
| after GUID copy 2: BD 100, BL 0, BL n, n BDs, **BL alignment type** (1 Centerline, 4 Miscellaneous), BL 2, 2, 2, 0, 1\|3 | decoded (O1, S58/S59) |
| GUID copy 2 at ~bit 100 of the block | decoded (same value as copy 1) |
| 1 bit (block bit 340) | 0 in all samples (in S59 the block differs only in the type BL) |
| +208 bits with +8 empty strings (S12, S48) | **optional** — absent in S45/S50 |
| element-ID list (BL n + BDs 1…n before the type, S12, S48, S51) | **optional** — Civil 3D adds it on save |
| station-equation data (S9: +196 bits in section 2, more after the cache) | **optional** — only with station equations |

## 2. Other records created with an alignment

| record | content | status | needed |
|---|---|---|---|
| companion `AECC_FEATURE_STYLE` | own handle; key (`0x8000_0000 \| n`); points of the alignment; 32-hex ID string; refs: own−off → registry, soft pointer → alignment | key, ID, refs decoded; points regenerated by Civil 3D (S35) | **yes** — without it AUDIT reports 2 errors (S30); after AUDIT's fix null references are accepted (S32) |
| registry entry | key, soft-owner ref, ID string; sorted by ID | decoded (`aecc_registry`, also the empty registry of a drawing without alignments, S65); keys re-assigned by Civil 3D on save (S54/S55) | yes, with the companion |
| `AECC_ALIGNMENT_STAEQU_LABEL_GROUP` | refs: layer, alignment, label style | refs decoded; body copied | **no** (S27 → S29) |
| design-speed label group | — | — | no (never created for new alignments) |
| name-template counter, command settings | — | — | no (bookkeeping; untouched in S26–S55) |

## 3. What a new alignment needs, as of S55

From our own computations: own handle, graphics (or none — X2), element
count, element records, end-of-element-0 station, element-0 cache, GUID,
string stream (name, description, 8 + 3 × arcs empty strings), handle
stream (layer, companion, style), companion key / ID / refs, registry
entry.

From documented constants (observed identically in every sample): the
section-2 bits, the 16- and 17-bit fields around GUID copy 1, the
post-cache block, the object-type and EED bits — about 800 bits, plus the
companion body.

Varying but probably irrelevant: the 66-bit section-2 field (X1), the
section-2 flag bit.

Not needed: label group, all optional blocks above.

So the donor drawing reduces to a source of constants. Once X1–X5 are
confirmed, `create_alignment` can write records from constants and
computed values only (and the docs keep the constants with the samples
they came from).

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

## 5. Hypothesis: the constant blocks are default options

The "constant" regions are probably not opaque data but alignment settings
and element constraints at their default values, identical only because
every sample alignment was created the same way (from a polyline, as a
centerline, no design criteria). Evidence so far:

- **Element constraints vary already**: free arcs (fillets) store
  parameters −1, −1, 1, −1, 29, three-point arcs 60, …, 2 (S10, D-012);
  the element header `0x04020008` and the BL 0 after it may encode the
  constraint type (fixed / floating / free) and how the element was made.
- **Creation method changes the layout**: S1 (imported from LandXML) has
  a 22-bit instead of 161-bit gap after its element records.
- **Station equations add data** in section 2 and after the cache (S9).
- **Optional blocks appear with edits** (S12's radius edit, S48 fresh,
  S51 re-save), so some regions are option-dependent.

### Sample plan (each S24 + one alignment, one option changed)

| sample | option | expected region |
|---|---|---|
| O1 | alignment type *Offset* or *Miscellaneous* instead of Centerline | section 2 constants |
| O2 | design speed set (e.g. 30 mph at station 0) | optional block / design-speed group |
| O3 | design criteria on (criteria-based design) | section 2 / post-cache |
| O4 | reference point / start station changed (e.g. 1+00) | start station BD, section 2 |
| O5 | alignment created with layout tools: fixed tangent + fixed curve | element headers / parameters |
| O6 | floating curve (attached to one tangent) | arc parameters |
| O7 | alignment in a site instead of siteless | handle stream, companion |
| O8 | superelevation calculated | optional blocks |

Each sample is compared with the plain S24-style alignment of the same
geometry; the bits that change name the option.

### Results

| option | result |
|---|---|
| O7 site | **partly decoded** (S101–S103): a site is an `AECC_PARCEL` face owning a container; an alignment in a site points both companion references at that container, which takes the companion's body; the old companion is emptied. One container per site holds all its alignments as a topology graph (S104–S106, not decoded); moving out creates a new companion. Read-only in civil3drust (`aecc_site`); new alignments are siteless (D-038). |
| O5 layout tools | **partly decoded** (S89): floating tangent = kind 4 with end point only; the BL after the element header is the element kind. Speed intervals on curves: layout seen, third distance not decoded — civil3drust writes no interval for curve speeds; accepted by Civil 3D, including a table with no intervals at all (S90 → S91). |
| O4 start station | **decoded** (S82, S9): BD ending 20 bits before the section-2 end. With a nonzero start the alignments had no design speeds: interval count 0, empty speed table, 3 strings fewer — so "GUID copy 1/2" are the first speed interval's GUIDs, not an alignment identity. Interval fields: GUID, 13 bits, BD station, BD distance, BD length to element end. civil3drust shifts speeds with the start (D-031, S85). Verified: shifted speeds (S86) and speeds added to speedless alignments (S87 → S88) accepted. |
| O3 design criteria | **decoded** (S79, S81): three flags at the end of section 2, five strings (criteria file, superelevation table, lanes, attainment method), check-set reference (set on save regardless). Verified: flags + strings + check set reproduce criteria-on (S80 C3 → S81). Bits (from the section-2 end): 19 criteria-based design, 18 criteria file, 17 check set, 14 tangency check (default on); confirmed by C1's dialog. |
| O2 design speed | **decoded** (S74): the table at the start of the post-cache block; the "BD 60" constant was the default entry. Two speeds also split the interval cache after the elements (not decoded). Verified: generated one-, two- and three-speed tables (S77) and an edited Civil 3D table (S78) work in Civil 3D; each entry needs a comment string (S76). |
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
