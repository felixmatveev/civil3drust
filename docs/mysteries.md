# Mysteries (MIST1 … MISTn)

Fields and behaviours of Civil 3D alignment records that are, or were,
not explained. None of the open ones stops Civil 3D from opening
civil3drust's output: they are left out, written as zero, kept as read,
or recomputed by Civil 3D on save. Each entry is self-contained so it can
be worked on separately. Numbers are never reused.

Status words: **SOLVED** — meaning known (the sample that settled it is
named; the finding is in `alignment-object.md`); **PARTIALLY SOLVED** or
**handled** — role and handling settled, content not decoded; **open** —
neither. Hypotheses and plans that later samples settled have been
removed; what remains is the evidence and the current reading.

Bit positions are given for S24-based records unless stated. Sections:
1 = object header + proxy graphics, 2 = from the end of the graphics to
the element count (`docs/alignment-map.md` §1.1), then the element
records, the speed-interval block and the post-cache block (§1.2).
Tools: `hexview --diff a.dwg b.dwg --filter <handle>`, `dumprec`,
`scan_elements`.

| ID | subject | status |
|---|---|---|
| MIST1 | 66-bit section-2 field: geometry fingerprint | partially solved |
| MIST2 | optional field after MIST1 (`BL 0x0D00D773, BL n`) | partially solved |
| MIST3 | date-modified stamp of the Properties form | partially solved |
| MIST4 | Alignment Properties header | handled |
| MIST5 | 2-bit flag changed on re-save | solved: MIST2 marker |
| MIST6 | `BL 2, 2, 2, 0, x` after the type | solved: implied PI display |
| MIST7 | element header `08 00 [02] 04` | structure mapped; bits open |
| MIST8 | curve flag bit | solved: set on load |
| MIST9 | `−1` parameters of curves and groups | mostly solved: criteria values |
| MIST10 | the `256` after a spiral's IDs | solved: spiral type |
| MIST11 | S89's curve speed-interval distance | open |
| MIST12 | companion topology flags | open (uniform form accepted) |
| MIST13 | extra block for a zero speed GUID | open (lead) |
| MIST14 | half-circle rule | one sample |
| MIST15 | flag 16 bits before the section-2 end | solved: tangency swapping |
| MIST16 | three BDs after a spiral's geometry | open |
| MIST17 | 128 bits after a `0x0D` element header | solved: group start point |

## MIST1 — the 66-bit section-2 field — **PARTIALLY SOLVED (S142–S150, S214)**

- **Status:** a geometry fingerprint. Civil 3D computes it from the
  alignment's shape and placement (and, for alignments with a JSON
  layout, the JSON's geometry) and keeps it while the geometry is
  unchanged. civil3drust cannot compute it and writes zeros (D-046).
  Zeros are accepted silently on alignments without a JSON layout; on
  an alignment with one, a mismatch makes Civil 3D log "contains new
  constraint curve group modified unexpectedly in old version" and
  re-solve the group's pieces from the JSON (S214 → S215), so every
  generated or edited JSON-group alignment warns once. The algorithm is
  not known.
- **Where:** section 2, bits 165–230 of the plain 413-bit form (after the
  `BL 80, 0, 80, 80, 0` class header and the 37-bit common entity data);
  199 in the Alignment Properties form, 241 in S59.
  `aecc_build::geometry_key_bit` finds it by the 14 bits that always
  precede it.
- **Deterministic:** the same geometry gives the same value across
  drawings and releases: S161's `from polyline` ((0,0) → (100,0),
  Civil 3D 2024) and S123's tangent (Civil 3D 2027) both store
  `35 5C BE 8C E9 03 C0 A6`. Unchanged by re-saves (S142, S143 = S24),
  renaming (S144) and the proxy-graphics setting (S140 = S141); S57's
  "new value on every save" was Civil 3D recomputing records
  civil3drust had generated.
- **Public name (probable):** `Entity.FingerPrint`, a read/write UInt64
  on Civil 3D's base entity class (API reference,
  `docs/class-catalog.md`).
- **Not a reported quantity:** none of the values in Civil 3D's Entities
  and PIs listing of S24 F9E occurs in section 2 as a 64- or 32-bit
  float.
- **Shape and position** (bytes of the 64-bit part, in file order):

  | sample | change | MIST1 |
  |---|---|---|
  | S24, S143, S144 | — (S144 renamed) | `EE 1E 81 38 E6 21 CE 8B` |
  | S145 | last tangent 200 → 284.6951 | `A4 DD 51 64 A5 60 D7 B1` |
  | S146 | moved 100 north | `0A 1E 81 38 E6 A1 1E 06` |
  | S147 | S146 moved 100 west | `32 1E 81 38 E6 61 6C 2E` |
  | S148 | S146 rotated 90° about its start | `52 1E 81 38 E6 21 1C 6B` |
  | S149 | S148 with radius 50 → 218.0166 | `6B 9E A5 24 C1 6D 78 5C` |
  | S150 | S144 rotated 90° about (0, 0) | `E2 1E 81 38 E6 21 2E 83` |

  38 bits (bytes 1–4 and the low 6 bits of byte 5) never change under
  moves or rotation; the other 28 do. Lengths and radii change all of
  them. So it looks like a 38-bit shape value combined with a 28-bit
  placement value. Two 100-ft single tangents in different drawings and
  directions (S44 FB8, S123 8F01) share the shape bits.
- **JSON layouts (S204, S214 → S215):** zeroing MIST1 alone triggers the
  warning and a rebuild (piece IDs renumbered, the same MIST1 written
  back); a change in the last digits of the JSON's `ArcRadius` does not;
  S204's composed JSON (other pass-through points) did. So it covers
  the JSON's geometry, probably rounded.

## MIST2 — the optional +44-bit field after MIST1 — **PARTIALLY SOLVED (S151–S160)**

- **Status:** a field Civil 3D adds when it re-saves an alignment and
  restores on save when missing. civil3drust never copies or adds it:
  it removes it together with its format marker when creating an
  alignment or writing its geometry (D-047). The meaning of
  `0x0D00D773` and of the second value is not decoded.
- **Content:** `BL 0x0D00D773` (same in all 251 records that have it) and
  `BL n`: 2 for every centerline, 1 for the two Miscellaneous alignments
  (S59 1089, S62 FB6). Civil 3D keeps a stored n (S152 → S156 kept 0).
- **Where:** directly after MIST1 in the plain forms; 98 bits after
  MIST1 in the Alignment Properties form, behind the date-modified block
  (MIST3). Absent in a 2020 drawing (`AeccLand110` classes, checked
  locally).
- **Format marker:** the full BL at section-2 bit 115 is `0x1148` without
  MIST2 and `0x1188` with it in the plain form (MIST5); in the Properties
  form bits 115–124 are `0011000001` without and `0100000001` with it.
  The field must match the marker.
- **Experiments** (S151–S160, from S131 with `exp_mist2`):

  | sample | change | result |
  |---|---|---|
  | S151 | first value → 0 | RECOVER: eWrongObjectType, discarded |
  | S152 | second value 2 → 0 | opens; re-save S156 keeps 0 |
  | S153 | both 0 | RECOVER: eWrongObjectType |
  | S154 | field removed, marker kept | RECOVER: eWrongObjectType |
  | S155 | field removed, marker `0x1148` | opens, AUDIT clean; re-save S157 restores both (= S131) |
  | S158 | Properties form, field removed, marker kept | RECOVER: eWrongObjectType |
  | S159 | Properties form, field removed, marker "absent" | accepted; re-save S160 = S144 |

  The first value is a checked tag (compare MIST7); the second is stored
  data.

## MIST3 — the date-modified stamp — **PARTIALLY SOLVED (S191–S194)**

- **Status:** `BL 0x1600101F` + a 64-bit Windows FILETIME, the time of
  the last Alignment Properties edit, carried only by the Alignment
  Properties form (after MIST1, before MIST2). civil3drust drops it by
  writing the plain form (D-057); Civil 3D accepts that and does not add
  it back (S193 → S194). Removing the block while keeping the Properties
  header is rejected (S191, S192: very slow open, RECOVER,
  eWrongObjectType). How the header announces the block is not decoded.
- **Evidence:** all 36 distinct values in the 154 records that have it
  fall on the days the samples were made, 1 ms resolution (S144's rename
  stamped 22:03:55 UTC, the minute the owner sent it). Grip edits keep
  the stamp (S145–S150).
- **Shared block:** the same `BL 0x1600101F` + FILETIME occurs in 82
  classes (styles, label sets, check sets …): a base-class "date
  modified" block (`docs/class-catalog.md`).
- **Section-2 bits 115 to the MIST1 anchor:** plain form `BL 0x1148`
  (`0x1188` with MIST2), `BL 0`; Properties form `BL 1` (or the flag
  BL, below), `BL 0x08010004`, then 26 bits. The Properties form is 176
  bits longer than the plain one: 34 here, 98 for this block, 44 for
  MIST2. Its first BL holds flags: 0x01 Properties header, 0x02 JSON
  layout (`BL 0x01010034` follows), 0x40 MIST2 (`docs/alignment-object.md`,
  "Generation test").

## MIST4 — the Alignment Properties header — **handled (D-057)**

- **Status:** section-2 bits 115 up to the MIST1 anchor in the Properties
  form (`BL 1`, `BL 0x08010004`, 26 bits; longer forms in S59 and
  S177–S183), which announces the date-modified block (MIST3).
  civil3drust replaces it with the plain form's `BL 0x1148, BL 0` when it
  creates an alignment (S193 accepted); the longer forms (S59, and the
  JSON-layout forms of S177–S183, which need it, S206) are left as they
  are. Its fields are not decoded.
- **Seen:** exactly in alignments edited in the Alignment Properties
  dialog or with the layout tools (S9, S22, S59, S74/S78, S79–S89,
  S98–S100, S144); not set by the creation method (S161: polyline and
  layout-tool alignments both plain).

## MIST5 — the 2-bit flag at S7 bit 2677 — **SOLVED (S154, S155): MIST2 marker**

These bits belong to the full BL at section-2 bit 115, `0x1148` without
MIST2 and `0x1188` with it (MIST2). That is why the first Civil 3D
re-save (S7 → S8, S24 → S131) changed them together with adding MIST2.

## MIST6 — the post-type value `BL 2, 2, 2, 0, x` — **SOLVED (S122)**

x is the implied point-of-intersection display (Alignment Properties →
Point of Intersection): 1 by change in alignment direction, 2 by
individual curves and curve group, 3 do not display. Stored per
alignment; a new alignment takes the drawing's default (S5–S23 default
1, S24 default 3). `aecc_build::pi_display` / `set_pi_display`.

## MIST7 — the element header — **structure mapped; bits open**

```text
00 + 0x0008   must be 8 (0, 7, 9, 0x0108: eOutOfRange)
[flag byte]   present exactly when its bit 0x02 is set:
              0x02 this element's parameters locked (S164, S172)
              0x01 kept by Civil 3D, no visible effect (S127 → S132)
              0x04 accepted, dropped on save (S137 → S139)
type byte     0x04 element; 0x0D first spiral of a group stored as
              separate elements, locked (S177; 128 bits follow, MIST17);
              0x09 the same, unlocked (S202); 0x03, 0x05: eWrongObjectType
```

- **Read as one BL** in the locked form this is `0x04020008`, the value
  first seen in every element record. Civil 3D tells the flag byte from
  the type byte by bit 0x02; a "flag byte" without it (0x00, 0x01) is
  read as a wrong type (S126, S136).
- **Survey** (about 1,200 element headers in `samples/`): `08 00 02 04`
  almost everywhere (all element kinds, all releases, a 2020 production
  drawing checked locally); `08 00 04` in unlocked elements (S164,
  S166–S172); `08 00 02 0D` in Civil 3D's locked group members (S177,
  S178), `08 00 02 09` unlocked (S202). `08 00 0D` (no `02` byte) is
  never written by Civil 3D and is rejected (S185, S189, S197–S200).
- **Experiments** (owner, S125–S138 with `exp_header`, all four elements
  of S24 F9E):

  | header (as BL) | result |
  |---|---|
  | `0x04020008` (S125) | control; re-save S131 |
  | `0x04030008` (S127) | opens; kept on save (S132) |
  | `0x04060008` (S137) | opens; saved as `0x04020008` (S139) |
  | `0x04010008`, `0x04000008`, `0x05020008`, `0x03020008` | RECOVER: eWrongObjectType |
  | `0x04020009`, `…0007`, `…0000`, `…0108`, `0x08000204` | RECOVER: eOutOfRange |

- **Locks:** "Lock all parameter constraints" (Alignment Properties →
  Constraint Editing) is section-2 bit 15 before the end (MIST15) and
  the lock given to new elements; each element's own lock is its `02`
  byte (S172: one element locked in Civil 3D, only its header changed).
  For group members the lock is bit 0x04 of the type byte instead
  (`0D` locked, `09` unlocked, S202). civil3drust: D-051,
  `aecc_alignment::header_bytes`, `read_header`.
- **Not the tangency constraint:** "Constrained by Next (Floating)" on a
  line changes its kind BL (0 → 3), not the header (S201).
- **Ruled out:** drawing units and release (S123 feet, S124 meters,
  Civil 3D 2027: same header); text (it is a BL in the data stream).
- **A family of such values:** `0xTT0M0008` occurs as a full BL in many
  Civil 3D classes (S119 and an older drawing checked locally):
  `0x0C030008` label, band and view styles; `0x04020008` alignment
  elements, profile and survey-figure styles; `0x05020008` surface and
  table styles; `0x06020008` label-collector styles; `0x0A020008`
  section-view bands and profile/section styles. The MIST2 and MIST3
  tags have the same shape. Possibly one sub-object header scheme; not
  relied on.
- **Still open:** flag bit 0x01; what `0x0008` stands for; a bit-level
  reading that covers both the `02` byte and the group-member type bits.

## MIST8 — the curve flag bit — **SOLVED (S207–S210)**

Civil 3D sets the bit right after a curve's or group's header on every
curve it read from the file when it opens the drawing; a curve created
in the session that saves it keeps 0 until the drawing is opened again.
Edits don't change it (S207: S24's 0 → 1 with a radius edit; S208: S131's
1 stays 1; S209: a new curve 0 beside the old 1; S210: S209 reopened and
saved, the new curve 1). Load-time state, not model data: civil3drust
writes 0 for new curves and keeps the stored value on rewritten ones
(test `curve_flag_set_on_load_s207_s210`).

## MIST9 — the `−1` parameters of curve and group records — **mostly solved (S219–S222)**

- **Where:** the parameter block `B flag, BD p1, BD p2, BL n1, n1 × BD`
  after the header of curves, spirals and groups.
- **Found** (owner's S219–S222, criteria-based design on, design speed
  40 mi/h): every curve stores `p1 = 40` (design speed), `p2 = 533` (the
  criteria's minimum radius) and `p3 = 83` (a third criteria value). The
  values follow the criteria, not the curve (an R 50, an R 533 and an
  R 179 curve all store 40, 533, 83). Without criteria all three are −1,
  which Civil 3D accepts, so civil3drust writes −1 (test
  `curve_criteria_values_s219_s222`). `n1` is 1, except 2 on S117's
  centre-and-radius circle (FC2, p1 0.0).
- **Parked (owner):** design criteria are not a priority. Still open:
  what 83 is (minimum transition length or superelevation runoff at
  40 mi/h are candidates) and how the values follow a speed change (S220
  meant to change the speed to 30 but still stores 40).

## MIST10 — the `256` after a spiral's IDs — **SOLVED (S232, S234): spiral type**

- **Where:** the BL after the IDs in every spiral record (fixed spirals,
  free spirals, line-spiral groups, spiral groups) and `"SpiralType"` in
  the JSON layout (S177, S178).
- **Codes** (S232: one outward spiral R 200 → ∞, L 200 per type, laid
  out by the owner): 256 Clothoid, 257 Sine Half-Wave Diminishing
  Tangent, 258 Cubic (JP), 259 Bloss, 260 Cubic Parabola, 261 Sinusoidal
  Curves, 262 Bi-Quadratic; 264 NSW Cubic Parabola (S234: S232 with
  three spirals set to 264 opens clean and lists the type, but Civil 3D
  2024 draws no curve). 263 crashes Civil 3D on open (S233). Nothing else
  in the record changes with the type. Shapes:
  `docs/alignment-object.md`, "Spiral types (S232)"; NSW: `FUTURE.md`,
  FRT-1.
- **Settings vs existing spirals:** S232's "NSW Cubic Parabola"
  alignment, copied from the Bi-Quadratic one with the type changed in
  the settings afterwards, still stores 262 and draws the Bi-Quadratic
  shape.
- **Only clothoids sampled** in groups, free spirals and JSON groups.

## MIST11 — S89's curve speed-interval distance — **SOLVED (S235–S237): a stale interval**

- **Answer (2026-11-04):** the rule holds everywhere: the third distance
  is the element length minus the distance from its start, the length
  being measured to the end point stored in the interval. S89's stored
  end point is 169.264 along the arc (clockwise from the start; the
  earlier "84.628 further on" was measured the wrong way round), and
  84.6364 + 84.628 = 169.264: the curve was 169.264 long when the speed
  was set and is 120.355 now, so the interval kept the old curve. The
  owner's samples follow the rule: S235 (speed mid-curve), S236 (speed
  at the alignment's end: 0.0007 left, no clamping) and S237 (curve
  lengthened by its grip: Civil 3D rewrote both intervals, and moved
  the curve speed's station 113 → 117.143). So Civil 3D refreshes
  intervals on a grip edit, but some earlier edit in S89 left its
  interval stale. civil3drust computes intervals from the current
  geometry (test `speed_interval_third_distance_s89_s235_s237`).


- **Where:** S89's design-speed interval on a curve, third distance.
- **Seen:** 84.63 stored where "length to the element's end" (35.72) is
  expected; every other interval, on lines and curves (S113–S115), fits
  the rule.
- **Closer look (2026-11-04):** S89's interval stores station 285.63,
  distance from the curve start 84.6364 and 84.628, and its "element
  end" point lies 84.628 along the arc, not at the curve's real end
  (120.355 along). So when the interval was written the curve probably
  ended at about 2+85.62, the speed sat at (or just past) its end, and
  the curve was lengthened afterwards without the interval being
  rewritten. Two readings to separate: the value is the element length
  when the speed lies at or past the element end; and intervals are not
  rewritten when the element changes later.
- **Plan (owner, from S24):** A1 a layout-tool alignment: tangent
  (0, 0) → (100, 0), fixed curve by three points, tangent; speeds 60 at
  0+00 and 30 mid-curve; save. A2 A1 plus a speed 40 exactly at the
  curve's end station (PT from the grid); save. A3 A2 with the curve
  lengthened by its pass-through grip; save.

## MIST12 — companion topology flags — **handled (D-064, S238–S241)**

- **Status:** the flags record the session's history (built in the
  saving session, loaded from the file, made from an object), not model
  data. Companions written in one form, edge `0x10` and vertices
  (10, 0, 0), whatever they were before, open cleanly with a clean AUDIT
  (S238, S239, owner) and Civil 3D's re-saves keep that form (S240,
  S241). civil3drust writes only that form (D-064). The individual bits
  (`0x20`, `0x800`, vertex 2/50/178, a/b = 1 for siteless alignments)
  are not decoded further.


- **Where:** companion `AECC_FEATURE_STYLE` edges: vertex flags
  (2, 10, 50, 178 seen) and the a/b face IDs; the face-like tail entry.
- **civil3drust** writes the uniform form (flags 10, a = b = 0, D-040)
  and Civil 3D accepts it (S111). Site containers (S104–S112) are not
  written.
- **Edge flags and creation (S161):** an alignment made from a polyline
  (8C49) has edge flags `0x030` and first vertex (flags, a, b) = (2, 1,
  1); one made with the layout tools (8C55) has `0x810` and (10, 0, 0),
  and a companion 76 data bits shorter with 2 handle references fewer.
  Values seen in 300 decoded companions: `0x010`, `0x030`, `0x810`,
  `0x830`. The `0x20` bit with (2 or 50, 1, 1) follows "geometry taken
  from an object" (S25 FB8, S161 8C49; kept by a plain re-save, S26 →
  S28; cleared by a layout-tool edit, S5 → S11). `0x800` has no simple
  rule yet (S11 `0x810` → S12 `0x010`).
- **Survey (2026-11-04, 805 companions in `samples/`):** `0x010` 215,
  `0x030` 60, `0x810` 511, `0x830` 19. Along the save chains the flags
  follow what happened in the session, not the geometry:
  - **built or rebuilt in the session that saves it:** vertex flags 10.
    After a parameter edit or a full rebuild (S6, S8, S12, S14, S57, S93,
    S131, S207, S208) the edge is `0x010`; after creation or adding
    elements with the layout tools (S11, S24, S161 8C56, S209, S96 FB3)
    it is `0x810`; from an object (polyline, S2, S5, S25 FB8, S161 8C49)
    `0x030` with the first vertex (2 or 50, 1, 1).
  - **loaded and saved without a rebuild:** vertex flags 10 → 2 and
    `0x800` added (S209 → S210, S26 → S28, S2 → S4, S137 → S139). Like
    the curve flag (MIST8), 2 means "read from the file".
  - The same input can come out either way (S137 → S139 `0x810`/2,
    S137 → S142 `0x010`/10), so the rebuild depends on what was done in
    the session, not only on the file.
  Reading: all of these bits are session history, not model data.
- **Test (S238, S239, `exp_companion`):** companions written in the
  rebuilt form whatever they were (edge `0x010`, vertices (10, 0, 0)):
  S238 = S28 with F9E (`0x810`, loaded) and FB3 (`0x830`, from an
  object); S239 = S161 with 8C49 (`0x030`, from a polyline, a companion
  with 2 more handle references) and 8C56 (`0x810`, layout tools). If
  Civil 3D opens both without RECOVER and AUDIT is clean, civil3drust
  can write that one form for every companion instead of copying the
  template's flags (D-040).
- **Result:** both clean (owner); re-saves S240, S241 keep the form.

## MIST13 — Civil 3D's extra block for a zero GUID — **SOLVED (S56/S57): a duplicate speed interval**

- **Answer (2026-11-04, from existing samples):** the 617-bit block is a
  second design-speed interval, and the 2 strings are its two strings.
  In S56, X3 and X5 each had one interval and one table entry, both with
  a zero GUID. On re-save (S57):
  - X3 (graphics, MIST1 and label group present): the interval and the
    table entry got the same new GUID; still one interval.
  - X5 (none of those): the table entry got a new GUID and a new
    interval with that GUID was written after the old one, which kept
    its zero GUID. The two intervals are identical apart from the GUID
    (station 0, element 1, same points). So the zero-GUID interval is
    left as an orphan.
  Which of X5's three differences sends Civil 3D down that path is not
  known (X2, without graphics only, had a random GUID). The "exactly 617
  bits" match with S161 was partly chance: an interval's size depends
  on its BD values (8C49's is 609 bits). civil3drust never writes a
  zero-GUID interval (D-045), so it cannot produce the orphan. Its
  reader finds the speed table through the first interval's GUID and so
  cannot read X5's speeds in S57 (test
  `zero_guid_interval_duplicated_s57_x5`).


- **Where:** after the cache in S57's X5 alignment.
- **Seen:** a record with a zero speed-interval GUID made Civil 3D add a
  617-bit block and 2 strings that no other record has. Civil 3D 2027
  itself writes zero GUIDs (S123, S124: one speed, no interval block);
  civil3drust gives such entries a fresh GUID when it rewrites speeds
  (D-045).
- **Lead (S161):** Civil 3D 2024 also writes no interval and a zero GUID
  for an alignment made with the layout tools (8C56), while one made from
  a polyline (8C49) has an interval and a random GUID; the difference is
  exactly 617 bits and 2 strings. So the block is probably the speed
  interval Civil 3D adds for the zero-GUID entry. To confirm by comparing
  S57 X5's block with S161 8C49's interval.

## MIST14 — the half-circle rule — **one sample**

- **Seen:** an open curve defined by its centre, with nothing before it,
  is drawn as exactly 180° ending where the next line leaves it (S118);
  its start is not stored.
- **Test:** a second such curve with a different sweep.
- **Question (2026-11-04):** is the 180° a rule, or did S118's start
  happen to lie 180° away (e.g. the radius pick point, which is not
  stored)? **Plan (owner, from S24):** two alignments, each starting with
  Fixed Curve (center and radius), center (1000, 0), radius 100,
  clockwise, followed by Floating Line (from curve, through point) to
  (1000, 300): `C east` with the radius picked at (1100, 0), `C north`
  with it picked at (1000, 100). If both are drawn as half circles the
  rule holds; if their starts follow the pick point, the start comes
  from somewhere we have not read.

## MIST15 — the flag 16 bits before the section-2 end — **SOLVED (S163)**

Alignment Properties → Constraint Editing → "Always perform implied
tangency constraint swapping" (1 = on; S163 turned it on, nothing else
changed). The bit before it (15 before the end) is "Lock all parameter
constraints" (S164). Both are read by `aecc_criteria::constraint_editing`
(`alignment-object.md`, "Design criteria"). The neighbouring design
criteria flags are 19 criteria-based design, 18 criteria file, 17 check
set, 14 check for tangency. Bit 13 before the end is still unexplained.

## MIST16 — the three BDs after a spiral's geometry — **open**

- **Where:** the end of every fixed spiral record, after the lengths and
  radii: `BD 0, BD 0, BD 1`. Spirals Civil 3D places from the element
  before (codes 6 and 8, S225) have two: `BD 0, BD 1`.
- **Seen:** identical in every fixed spiral sampled (inward, outward,
  compound, through-point, S162) and for every spiral type (S232).

## MIST17 — the 128 bits after a `0x0D` element header — **SOLVED (S177–S182, S206)**

- **Answer:** two RDs, the group's start point, stored before the
  parameter flag of the first spiral of a group Civil 3D keeps as
  separate elements with a JSON layout (header type byte `0D` locked,
  `09` unlocked, MIST7). Equal to the member spiral's start in all 15
  cases (S177, S178, S180–S182). `SpiralRecord::member`.
- **Member vs JSON:** a member spiral without the JSON flag is accepted
  (S206); the earlier rejections (S185, S189, S197–S200) came from the
  bad `08 00 0D` header, not from a missing JSON. A JSON group without
  any pieces is accepted too: Civil 3D solves the pieces from it (S216 →
  S217).

## Not sampled yet (not mysteries, just gaps)

- Real-world alignments (a 2020 production drawing, checked locally): 2
  of 10 alignments do not decode (one "tangents around free arc don't
  meet at a PI"), and the alignment type / implied PI display are found
  in only 1 of 10 — the block after the design-speed table has another
  layout there. Needs purpose-made samples to reproduce.
- Offset and other alignment types (S1-style imports have no element
  records and a 22-bit gap after the elements instead of 161 bits).
- Widening: offset alignments with widening regions (start and end
  stations, offset, transitions in and out — linear, curved, or by
  criteria) and the parent alignment's link to them. Sample plan: one
  alignment, an offset alignment, then one widening added, edited and
  removed, each step saved separately.
- Station equations (data seen in S9, not decoded).
- Alignment groups (owner): Civil 3D's own grouping of alignments in the
  Toolspace (Prospector), for siteless alignments and inside each site.
  Sample plan, each step saved separately from S24: 1. two siteless
  alignments; 2. a group created with both; 3. a second group, one
  alignment moved to it; 4. one alignment taken out of all groups; 5. a
  site with two alignments and its own group.
- Site members: deleting them and writing sites (postponed to the site
  work).
- Non-clothoid spirals in groups, free spirals and JSON groups; a second
  length/radius pair and inward spirals for the spiral types
  (`alignment-object.md`, "Spiral types").
