# What Civil 3D rejects, crashes on or repairs

Every condition we have seen Civil 3D refuse, crash on, flag in AUDIT or
quietly change, from our own samples (Civil 3D 2024 unless noted). This is
the list behind civil3drust's pre-write checks: a file civil3drust writes
must not contain any of these (D-049). The opposite list — what Civil 3D
accepts missing and rebuilds — is `civil3d-rebuilds.md`.

How to read a row: **condition** in the file → **Civil 3D's response** →
**samples** that show it → **civil3drust**: how the writer avoids it today
(*guarded*) or what is still to do (*to check*).

## 1. Rejected on open: RECOVER discards the object

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
| String stream too short. Civil 3D expects 9 strings, +3 per design speed with an interval (+1 without), +3 per element record that is not a line (curve, spiral, spiral group), +1 for a JSON layout string (D-056); more are accepted | RECOVER deletes the alignment, "Error 151 (eOutOfRange)" (it reads past the end of the strings). Depends on what follows the stream: in S184 13 of 41 short alignments were discarded, the others opened, some keeping garbage strings in the re-save | S46 (arcs); S184 → S186 (curves, spirals, groups; all 30 remaining alignments re-saved with exactly this count) | guarded: `write_elements` adds 3 per non-line record; `aecc_element_edit::required_string_count`; test `string_count` |
| Alignment type 2, 3 or 5 on a plain alignment (Offset, CurbReturn, Rail need data a plain alignment lacks) | errors; RECOVER removes the alignments | S61 → S63 | guarded: `create_alignment --type` names only 1 and 4; other numbers are an experiment option |
| Element sub-object tag with a wrong type part: top byte ≠ 4, or third byte without bit 0x02 | "Error 34 (eWrongObjectType)", alignment discarded | S126, S128, S136, S138 | guarded: elements are written as `0x08 0x00 [0x02] 0x04`, the flag byte present exactly when that element is locked (S164, S172, D-051) |
| Element sub-object tag with a wrong low word (anything but `0x0008`: 0, 7, 9, `0x0108`) | "Error 151 (eOutOfRange)", alignment discarded | S129, S133, S134, S135 | guarded: as above |
| Element tag bytes reversed (`0x08000204`) | eOutOfRange, discarded; the drawing opens very slowly and draws with glitches | S130 | guarded: as above |
| MIST2 tag ≠ `0x0D00D773` | eWrongObjectType, alignment discarded | S151, S153 | guarded: civil3drust never writes MIST2 (D-047) |
| MIST2 removed while the format marker (section-2 bits 115–124) still says "present" — plain form `0x1188`, Properties form `0100000001` | eWrongObjectType, alignment discarded | S154 (plain), S158 (Properties) | guarded: `remove_resave_field` switches the marker with the field |
| Date-modified block (MIST3) removed while the Alignment Properties header (section-2 bits 115…MIST1 anchor) is kept, with or without MIST2 | very slow open; RECOVER: eWrongObjectType, alignment discarded | S191, S192 | guarded: the block is only dropped together with the header, by writing the plain form (D-057) |
| JSON layout string removed from an alignment that has one (the strings after it move up one slot) | RECOVER; the alignment comes back empty, without a name | S183 (owner) | guarded: civil3drust never removes it; `exp_json remove` is an experiment |
| Group-member spiral (header `0x0D020008` with its start point) in an alignment without the JSON flag (unconfirmed: the samples also had the bad header below) (section-2 Properties-form flags `0x02` + `BL 0x01010034`), with or without a JSON string | RECOVER, "Error 34 (eWrongObjectType)", alignment discarded | S185 → S187; S189 (string count correct); S197, S198 (composed JSON string present, plain form) | guarded: `replace_elements` refuses member spirals when the alignment has no JSON string; write them as ordinary spirals (`0x04`, S190) |
| Group-member header `08 00 0D` (locked type byte without the `02` byte; Civil 3D writes `08 00 02 0D` locked, `08 00 02 09` unlocked) | likely eWrongObjectType (written in S185, S189, S197–S200, all rejected; not tested alone) | S202 (Civil 3D's unlocked form) | guarded: `aecc_alignment::header_bytes` |
| Companion tail entry count does not match its handle references (no face-like entry ↔ 3 references; one entry ↔ 5) | companion discarded, eWrongObjectType ("6 errors fixed, 3 objects erased"); the alignments keep dangling companion references | S108 → S109 | guarded: the rebuild keeps the template's edge flags and tail (D-040, test `tail_entry_matches_reference_count`) |

## 2. Opens, then misbehaves

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
| JSON layout string emptied (string kept, no text) | no RECOVER, AUDIT clean, but the stored-apart spiral groups no longer re-solve; on the next edit they are cut from the following element, which becomes a detached piece; persists after save and reopen | S180 → S182 | guarded: civil3drust never empties it |
| Null alignment style reference | no RECOVER prompt; alignment listed in Prospector but not drawn; **selecting it crashes Civil 3D** (unhandled access violation reading 0x0098). AUDIT: 3 errors, names "Alignment Style eNullObjectId → Standard"; afterwards it draws as a dot until moved | S67 → S68, S69 | guarded: `create_alignment` refuses a style the target lacks (D-025) |
| Alignment style with no owner | style dropped on save; alignment shown with the default style; without AUDIT the reference is left broken (points at the dropped style) | S70 → S71 (AUDIT), S72 (no AUDIT) | to check: styles are only referenced from the target today |
| Alignment style owned outside Civil 3D's style collection (own dictionary) | style ignored; alignment shown with the default style | S73 | to check: as above; own styles need Civil 3D's collection chain (D-030) |

## 3. Opens; AUDIT reports errors

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
| No companion and no registry entry; the alignment's two companion references null | AUDIT: 2 errors (no objects named). With fixing, the references stay null and no companion is created; without fixing, the file is kept as is | S30 → S31, S32; probably also the 2 errors in S67, S70, S73 | guarded: `create_alignment` writes a companion and registry entry unless `--no-parcel-entry` |

## 4. Accepted, but Civil 3D changes it

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
| Design-criteria flags set without the criteria file strings or a check set | flags cleared when the alignment is viewed (bits `111 0111` → `100 0101`, tangency check cleared too) | S80 `C1 flags` → S81 | to check: write the flags only with their strings and check set (`aecc_criteria`) |
| Element tag third byte with bit 0x04 (`0x04060008`) | accepted, AUDIT clean; written back as `0x04020008` | S137 → S139 | — (never written) |
| Element tag third byte with bit 0x01 (`0x04030008`) | accepted and kept on save | S127 → S132 | — (never written) |
| Zero identity GUID together with no graphics, zero MIST1 and no label group (X5) | accepted; the GUID stays zero and Civil 3D adds a 617-bit block and 2 strings (MIST13; probably a speed interval) | S56 X5 → S57 | guarded: `create_alignment` writes a random GUID; zero speed GUIDs are replaced (D-045) |

## 5. Outside Civil 3D

| condition | response | samples | civil3drust |
|---|---|---|---|
| A drawing written by civil3drust | Civil 3D shows its "Non Autodesk DWG" notice on open; no other effect | S26 and later | intended (D-022) |
| S56 (six generated alignments, some without graphics) | BricsCAD Lite did not open it; reason unknown | S56 | to check |

## Pre-write checks (summary)

What a file civil3drust writes must satisfy, from the rows above:

1. The string count is at least `required_string_count` (3 per curve,
   spiral or group record, D-056).
2. Alignment type is 1 or 4 unless the data for other types is written.
3. Every element starts with `0x0008`, then `0x02` if that element's
   parameters are locked (none if not), then `0x04`.
4. MIST2 is absent and the format marker says "absent" (or both present,
   untouched, as Civil 3D wrote them).
5. Each companion's tail matches its handle references.
6. The style reference is non-null and points at an alignment style the
   target already has (one in Civil 3D's own style collection).
7. Companion references are non-null (or the AUDIT warnings are accepted).
8. Design-criteria flags are set only with their strings and check set.

Checks 1–6 are enforced by the writer's construction today; 7 is the
default; 8 is not enforced yet. A single validation pass that runs before
every write and also checks drawings read from elsewhere is the next step.
