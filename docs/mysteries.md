# Mysteries (MIST1 … MISTn)

Fields and behaviours of Civil 3D alignment records that are not yet
explained. None of them stops Civil 3D from opening civil3drust's output:
today they are copied from a template record, left out, or recomputed by
Civil 3D on save. Each entry is self-contained so it can be worked on
separately. When one is solved, mark it **SOLVED** with the sample that
settled it, move the finding into `alignment-object.md`, and keep the
number (numbers are never reused).

Bit positions are given for S24-based records unless stated. Sections:
1 = object header + proxy graphics, 2 = from the end of the graphics to
the element count (`docs/alignment-map.md` §1.1), then the element
records, the speed-interval block and the post-cache block (§1.2).
Tools: `hexview --diff a.dwg b.dwg --filter <handle>`, `dumprec`,
`scan_elements`.

## MIST1 — the 66-bit section-2 field

- **Where:** section 2, bits 165–230 of the plain 413-bit form (after the
  `BL 80, 0, 80, 80, 0` class header and the 37-bit common entity data).
- **Seen:** differs per alignment; Civil 3D writes a new value on every
  save (S57), the same value for the same geometry (S94 = S96 after an
  element was deleted and re-added). All zeros is accepted (S56 X1).
- **Ruled out: proxy graphics.** S140 (PROXYGRAPHICS 1) and S141
  (PROXYGRAPHICS 0) are the same drawing saved both ways: in all 21
  alignments everything after the graphics, MIST1 included, is
  bit-identical, and so are the handle streams. The field is not derived
  from, or stored because of, the proxy graphics.
- **Not a reported quantity:** none of the values in Civil 3D's Entities
  and PIs listing of S24 F9E (lengths, stations, coordinates, radius,
  directions and delta in degrees or radians, chord, tangent, external,
  mid-ordinate, mid/centre points, degree of curvature) occurs in
  section 2 as a 64-bit or 32-bit float at any bit offset. They occur
  only later (element records, station cache, speed table). Two 100-ft
  single tangents in different drawings (S44 FB8, S123 8F01) share most
  of the field's bytes, so a hash or checksum of the geometry is likely.
- **Content-derived, not time:** S142 and S143, the same drawing saved
  minutes apart, have byte-identical F9E records, and MIST1 is the same
  as in S24, S131 and S139: Civil 3D keeps it across re-saves while the
  geometry is unchanged. S57's "new value on every save" was Civil 3D
  recomputing records civil3drust had generated. So it is a deterministic
  value computed from the alignment — most likely a hash or checksum —
  and not a timestamp or random ID.
- **Not the name:** S144 (S143 renamed) keeps the same value, moved 34
  bits later by the fields the rename added in front of it.
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

  A length change rewrites every byte; a pure move keeps bytes 1–4 and
  changes 0 and 5–7. The two 100-ft single tangents in different
  drawings and directions (S44 FB8, S123 8F01) also share exactly bytes
  1–4 (`5C BE 8C E9`).
- **Split:** across S144–S148, 38 bits never change under moves or
  rotation: bytes 1–4 and the low 6 bits of byte 5 (the 100-ft tangents
  agree on these too, `0x43`/`0x03`). The other 28 bits — the 2 leading
  bits, byte 0, the top 2 bits of byte 5 and bytes 6–7 — change with any
  move or rotation. So MIST1 looks like a 38-bit shape value (invariant
  under rigid motion) combined with a 28-bit placement value, not one
  hash. The shape part is not the total length as a float.
- **Inputs:** the radius feeds the shape bits (S149), like the lengths
  (S145). Rotating in place (S150) keeps the 2 leading bits and byte 5
  and changes the low half of byte 0 and bytes 6–7; moving north changes
  byte 5's top bits and rotating there (S148) changes them back, so
  position and direction are mixed in the placement bits (perhaps a
  value from the extents or the end points). The formula is still
  unknown.
- **civil3drust today:** copies the template's value.

## MIST2 — the optional +44-bit field after MIST1

- **Where:** right after MIST1 when present (section 2 grows 413 → 457).
- **Seen:** added by Civil 3D re-saves (S4, S8, S11, S12, S48, S51) and to
  every alignment of S57; absent in generated records (S45, S50 — accepted).
- **Hypothesis:** a cache tied to MIST1. Not tied to proxy graphics
  (S140 = S141 after the graphics).

## MIST3 — the 31-bit section-2 field

- **Where:** bits 300–330 of the 589-bit section-2 form (criteria
  samples, S79–S81).
- **Seen:** varies between records with otherwise identical section 2;
  probably a cache. The 589-bit form is not specific to design criteria:
  renaming an alignment (S144) also produced it.

## MIST4 — the +28-bit field after the class header

- **Where:** section-2 bit 119, after `BL 1` instead of the plain form's
  32-bit value. Content `0000000100000000000000000100` in every case.
- **Seen:** exactly in alignments edited in the Alignment Properties
  dialog or made with the layout tools (S9, S22, S59, S74/S78, S79–S88,
  S89, S98–S100, and a rename in S144). Records without it are accepted.
- **Question:** what the two set bits mean (a "modified by UI" marker, a
  version of a sub-object?).

## MIST5 — the 2-bit flag at S7 bit 2677

- **Where:** section 2 (S5–S8 numbering: bit 2677).
- **Seen:** Civil 3D changed it when re-saving the generated S7 as S8.
- **Hypothesis:** part of MIST2's appearance (re-save marker). Seen again
  in S24 → S131: the first Civil 3D re-save changes this flag and adds
  MIST2 in the same save; later re-saves keep both (S139, S142, S143).

## MIST6 — the post-type value `BL 2, 2, 2, 0, x` — **SOLVED (S122)**

- **Where:** post-cache block, right after the alignment type BL.
- **Answer:** x is the implied point-of-intersection display option on
  the Alignment Properties "Point of Intersection" tab: 1 = by change in
  alignment direction, 2 = by individual curves and curve group, 3 = do
  not display. It is stored per alignment; a new alignment takes the
  drawing's default, which is why it looked per-drawing (S5–S23 default
  1, S24 default 3).
- **Settled by:** S122, three identical alignments in S24 set to 1, 2
  and 3 by the owner. The drawing scale was ruled out first (S24 with
  1" = 40' still has 3).
- **civil3drust:** `aecc_build::pi_display` / `set_pi_display`; finding
  moved to `alignment-object.md`.

## MIST7 — the element header `0x04020008`

- **Where:** first BL of every element record (lines, curves, groups).
- **Seen:** identical in every record of every sample. The BL after it is
  the line kind (0, 4, 5, 9) in line records.
- **Question:** version / class tag of the element sub-object?
- **Ruled out:** drawing units and Civil 3D release. S123 (feet) and
  S124 (meters), both new drawings from Civil 3D 2027, have the same
  header as every Civil 3D 2024 sample. The owner's imperial-vs-metric
  hypothesis (with `0x02` as the feet unit code) does not hold.
- **Not text:** the owner read the bytes as control characters (EOT STX
  NUL BS). The value is a BL in the data stream, stored low byte first
  (`08 00 02 04`); strings live in the separate string stream.
- **Not alignment-specific:** in S119 the same 32-bit value (as a full BL)
  also occurs in `AECC_PROFILE_STYLE`, the profile-view band styles,
  `AECC_SVFIGURE_STYLE`, `AECC_TURNOUT_STYLE`, `AECC_TREE_NODE`,
  `AECC_LABEL_NODE` and `AECC_LABEL_COLLECTOR_STYLE`; neighbours
  `0x04010008` (label styles) and `0x05020008` (surface and table styles)
  occur too. A chance match of 34 bits is very unlikely at these counts,
  so it looks like a generic Civil 3D sub-object tag, possibly a version
  pair (4.2, 4.1, 5.2) with a type code 8.
- **Experiment, round 1** (owner, Civil 3D 2024): S125–S130 made with
  `exp_header` from S24 (F9E: three tangents and a free curve), the
  header of all four elements changed:

  | sample | header | result |
  |---|---|---|
  | S125 | `0x04020008` (control) | opens; re-saved as S131 |
  | S126 | `0x04010008` | RECOVER: F9E "Error 34 (eWrongObjectType)", discarded |
  | S127 | `0x04030008` | opens; re-saved as S132 with `0x04030008` kept |
  | S128 | `0x05020008` | RECOVER: eWrongObjectType, discarded |
  | S129 | `0x04020009` | RECOVER: "Error 151 (eOutOfRange)", discarded |
  | S130 | `0x08000204` | RECOVER: eOutOfRange, discarded; very slow, display glitches |

  So the value is read and checked, not ignored: the top byte must be 4,
  the third byte 2 or 3 (1 is a wrong type; perhaps bit 0x02 must be set
  and bit 0x01 is free), and the low word 8 is accepted while 9 is out of
  range. Civil 3D writes back what it read (S132 = S131 apart from the
  four header bits), so this is stored data, not a version it stamps.
  A likely reading: the high word identifies the sub-object type and the
  low word is its schema version, 8 being the newest Civil 3D knows.
- **Experiment, round 2** (owner, Civil 3D 2024):

  | sample | header | result |
  |---|---|---|
  | S133 | `0x04020007` | RECOVER: eOutOfRange, discarded |
  | S134 | `0x04020000` | RECOVER: eOutOfRange, discarded |
  | S135 | `0x04020108` | RECOVER: eOutOfRange, discarded |
  | S136 | `0x04000008` | RECOVER: eWrongObjectType, discarded |
  | S137 | `0x04060008` | opens, AUDIT clean; re-saved as S139 with `0x04020008` |
  | S138 | `0x03020008` | RECOVER: eWrongObjectType, discarded |

- **What Civil 3D 2024 accepts** (rounds 1 and 2):
  - **Low word** (bytes 0–1): exactly 8. 0, 7, 9 and `0x0108` are
    "out of range", so it is not a maximum version but a fixed value of
    the whole 16-bit field.
  - **Top byte:** exactly 4; 3 and 5 are "wrong object type".
  - **Third byte:** bit 0x02 must be set (0 and 1 are "wrong object
    type"; 2, 3 and 6 are accepted). Bit 0x01 is stored and written back
    (S127 → S132); bit 0x04 is accepted but dropped on save (S137 →
    S139).
  - So the high word works as a type tag with flag bits, and the low word
    as a fixed size or version that must match. The meaning of the flag
    bits is unknown.
- **A family of such headers** (S119 against an older Civil 3D drawing,
  AC1027 with `AeccLand110` classes, checked locally and not committed):
  values `0xTT0M0008` occur as full BLs in many classes in both files.
  The low word is always `0x0008`, the third byte mostly 2 or 3, the top
  byte depends on the class: `0x0C030008` in about 25 style classes
  (label, band and view styles), `0x04020008` in alignment elements,
  profile and survey-figure styles, `0x05020008` in surface and table
  styles, `0x06020008` in label-collector styles, `0x0A020008` in
  section-view band and profile/section entity styles. Classes present in
  both files carry the same value in both, so the values did not change
  between the 11.0 and 13.0 class schemas. Reading: the top byte names
  the kind of sub-object (Civil 3D answers "wrong object type" when it
  changes) and `0x0008` is a fixed field that must match; neither is a
  release version. (Scanning bit by bit also finds the same bytes one bit
  off, e.g. `0x02010004` and `0x08040010` around `0x04020008`; those are
  artifacts and were removed. Values seen in a single class, mostly in
  surface records full of binary data, may be chance matches.)
- **Unchanged in a 2020 drawing** (AC1027, `AeccLand110` classes; a
  production drawing checked locally, not committed): every element
  record of its 10 alignments starts with `0x04020008`, one per element.
- **Status:** behaviour mapped (S125–S139); no setting found that
  changes it. civil3drust writes `0x04020008`, the only value seen in
  Civil 3D's own drawings. Still open in case another element type or DWG format
  version shows a different value.

## MIST8 — the curve flag bit

- **Where:** the bit right after the header in curve and group records.
- **Seen:** 1 in S10, S14, S15; 0 in S11–S13. Civil 3D sets it after
  geometry edits it saved (S14, S15) and when re-saving our generated free
  curves unchanged (S92 → S93), but not after its own radius edit (S12).

## MIST9 — the `−1` parameters of curve and group records

- **Where:** `BD p2` and the `n1` BDs of the parameter block
  (`B flag, BD p1, BD p2, BL n1, n1 × BD, BL code`). p1 is the design
  speed at the curve (−1 until solved).
- **Seen:** always −1; `n1` is 1, except 2 for S117's centre-and-radius
  circle (FC2), whose p1 is also 0.0 instead of −1.
- **Hypothesis:** criteria values (minimum radius, superelevation …) left
  unset; a criteria-based alignment with curves would show them.

## MIST10 — the `256` in the line-spiral group

- **Where:** S119 element 7, the BL after the IDs (before the start
  point); also in S121's fixed spiral at the same place.
- **Hypothesis:** spiral definition or type flags (clothoid, by length vs
  A). S121 (spiral methods) should vary it.

## MIST11 — S89's curve speed-interval distance

- **Where:** S89's interval on a curve, third distance.
- **Seen:** 84.63 stored where "length to the element's end" (35.72) is
  expected; every other interval, on lines and curves (S113–S115), fits
  the rule.

## MIST12 — companion topology flags

- **Where:** companion `AECC_FEATURE_STYLE` edges: vertex flags
  (2, 10, 50, 178 seen) and the a/b face IDs; the face-like tail entry.
- **Seen:** civil3drust writes the uniform form (flags 10, a = b = 0,
  D-040) and Civil 3D accepts it (S111). Site containers (S104–S112) are
  not written.

## MIST13 — Civil 3D's extra block for a zero GUID

- **Where:** after the cache in S57's X5 alignment.
- **Seen:** a record with a zero speed-interval GUID made Civil 3D add a
  617-bit block and 2 strings that no other record has.
- **Also seen:** Civil 3D 2027 itself writes a zero GUID: S123 and S124
  each have one speed (0:60, 0:100) with no interval block and an all-zero
  GUID in the table entry. civil3drust gives such entries a fresh GUID
  when it rewrites the speeds (D-045).

## MIST14 — the half-circle rule

- **Seen:** an open curve defined by its centre, with nothing before it,
  is drawn as exactly 180° ending where the next line leaves it (S118);
  its start is not stored. One sample only.
- **Test:** a second such curve drawn with a different sweep.

## MIST15 — the flag 16 bits before the section-2 end

- **Where:** section 2, the bit 16 before its end, between the
  design-criteria flags (19 criteria-based design, 18 criteria file,
  17 check set, 14 check for tangency; `aecc_criteria`).
- **Seen:** 1 in the S5–S23 drawings, S10, S89 and the Civil 3D 2027
  drawings S123/S124; 0 in every S24-based drawing (247 records). Exactly
  the drawings whose implied PI display (MIST6) is 1, so it is probably
  another per-alignment option taken from the drawing's defaults.
- **Test:** compare the Alignment Properties tabs (Design Criteria, Point
  of Intersection, Constraint Editing) of S11's and S24's alignments for a
  setting that differs; then change it on one alignment and save.
- **civil3drust today:** copies the template's value.

## Not sampled yet (not mysteries, just gaps)

- Real-world alignments (a 2020 production drawing, checked locally): 2
  of 10 alignments do not decode (one "tangents around free arc don't
  meet at a PI"), and the alignment type / implied PI display are found
  in only 1 of 10 — the block after the design-speed table has another
  layout there. Needs purpose-made samples to reproduce.

- Offset and other alignment types (S1-style imports have no element
  records and a 22-bit gap after the elements instead of 161 bits).
- Station equations (data seen in S9, not decoded).
- Spiral groups other than line-spiral (S120 → S121).
