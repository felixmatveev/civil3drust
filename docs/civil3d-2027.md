# Civil 3D 2027 drawings

Samples: S123 (feet) and S124 (meters), new drawings from Civil 3D 2027
(owner), each with one alignment of one tangent (0,0) → (100,0).
Compared with the Civil 3D 2024 samples (S5–S122).

## Unchanged

- File format AC1032 (same as all earlier samples).
- Class entries for the alignment classes: same names, application name
  (`AeccLand130`) and versions. The class list differs (642/643 classes
  against about 600), as it does between any two templates.
- Alignment record: plain 413-bit section 2 (the MIST1 cache bits differ,
  as between any two alignments), the 306-bit tangent record, element
  header `0x04020008` (MIST7), handle stream of seven references,
  alignment type, design criteria flags, implied PI display (1).
- Edits (`set_length`, `set_speeds`) and `create_alignment` with a 2024
  donor work unchanged.

## Differences that mattered

1. **One-byte object type.** In both drawings the companion class
   `AECC_FEATURE_STYLE` is number 736/738, which DWG stores as `01` +
   one byte (10 bits) instead of `10` + RS (18 bits). civil3drust
   assumed 18 bits for the own handle and for writing class numbers, so
   using a 2027 drawing as the donor failed ("record …: own handle").
   This depends on the class order of the template, not on the release.
   Fixed: `handle_refs::object_type_bits`, `set_object_type` writes the
   form AutoCAD uses for the number (D-044).
2. **Speed with a zero GUID and no interval.** The alignments carry one
   design speed (0:60 in S123, 0:100 in S124) with no interval block and
   an all-zero GUID in the table entry. 2024 drawings always had an
   interval with a random GUID for a speed at the start. Rewriting the
   speeds copied the zero GUID into the new interval, and the identity
   update then searched for the second copy by value and overwrote an
   unrelated run of zero bits. Fixed: the second copy is found through
   the table (`aecc_design_speed::entry_guid_bits`), and zero GUIDs are
   replaced by fresh ones when speeds are rewritten (D-045). See MIST13.

## Other differences (no action)

- The alignments sit on a named layer, not layer 0 (template).
- S123's proxy graphics are Civil 3D's placeholder (text + extents)
  because the drawing has PROXYGRAPHICS 0 (S124 too); see S141.
- Section-2 flag MIST15 is 1, as in the S5–S23 drawings.

Tests: `tests/civil3d_2027.rs`.
