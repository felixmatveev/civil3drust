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
- **Hypothesis (owner):** derived from the proxy graphics rather than the
  element geometry. Test: the same drawing saved with proxy graphics on
  and off (PROXYGRAPHICS 1/0) — does the field change or disappear?
- **civil3drust today:** copies the template's value.

## MIST2 — the optional +44-bit field after MIST1

- **Where:** right after MIST1 when present (section 2 grows 413 → 457).
- **Seen:** added by Civil 3D re-saves (S4, S8, S11, S12, S48, S51) and to
  every alignment of S57; absent in generated records (S45, S50 — accepted).
- **Hypothesis:** a cache tied to MIST1 (same proxy-graphics test).

## MIST3 — the 31-bit section-2 field

- **Where:** bits 300–330 of the 589-bit section-2 form (criteria
  samples, S79–S81).
- **Seen:** varies between records with otherwise identical section 2;
  probably a cache.

## MIST4 — the +28-bit field after the class header

- **Where:** section-2 bit 119, after `BL 1` instead of the plain form's
  32-bit value. Content `0000000100000000000000000100` in every case.
- **Seen:** exactly in alignments edited in the Alignment Properties
  dialog or made with the layout tools (S9, S22, S59, S74/S78, S79–S88,
  S89, S98–S100). Records without it are accepted.
- **Question:** what the two set bits mean (a "modified by UI" marker, a
  version of a sub-object?).

## MIST5 — the 2-bit flag at S7 bit 2677

- **Where:** section 2 (S5–S8 numbering: bit 2677).
- **Seen:** Civil 3D changed it when re-saving the generated S7 as S8.
- **Hypothesis:** part of MIST2's appearance (re-save marker).

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
  point).
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

## MIST14 — the half-circle rule

- **Seen:** an open curve defined by its centre, with nothing before it,
  is drawn as exactly 180° ending where the next line leaves it (S118);
  its start is not stored. One sample only.
- **Test:** a second such curve drawn with a different sweep.

## Not sampled yet (not mysteries, just gaps)

- Offset and other alignment types (S1-style imports have no element
  records and a 22-bit gap after the elements instead of 161 bits).
- Station equations (data seen in S9, not decoded).
- Spiral groups other than line-spiral (S120 → S121).
