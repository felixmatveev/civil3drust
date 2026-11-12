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
| Group-member spiral in an alignment without the JSON flag (section-2 flags `0x02` + `BL 0x01010034`) | **not rejected** once the header is right (S206: `08 00 02 09` members accepted); S189/S197–S200 were rejected for the bad header below | S206 → S213 | — |
| Group-member header `08 00 0D` (locked type byte without the `02` byte; Civil 3D writes `08 00 02 0D` locked, `08 00 02 09` unlocked) | RECOVER, "Error 34 (eWrongObjectType)", alignment discarded | S185, S189, S197–S200 (all generated groups had it); fixed in S205/S206, which open | guarded: `aecc_alignment::header_bytes` |
| Companion tail entry count does not match its handle references (no face-like entry ↔ 3 references; one entry ↔ 5) | companion discarded, eWrongObjectType ("6 errors fixed, 3 objects erased"); the alignments keep dangling companion references | S108 → S109 | guarded: the rebuild keeps the template's edge flags and tail (D-040, test `tail_entry_matches_reference_count`) |
| Spiral-type code 263 (with 264) in fixed spiral records (S233: two spirals 263, three 264; nothing else changed) | **Civil 3D crashes on open**: "FATAL ERROR: Unhandled Access Violation Reading 0x0008 Exception at BBB04855h" (owner). 263 causes it: S234 (264 only) opens clean, 264 being NSW Cubic Parabola | S233 | guarded: civil3drust writes back only codes it read; `exp_spiral_code` is an experiment |

## 2. Opens, then misbehaves

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
| JSON layout string emptied (string kept, no text) | no RECOVER, AUDIT clean, but the stored-apart spiral groups no longer re-solve; on the next edit they are cut from the following element, which becomes a detached piece; persists after save and reopen | S180 → S182 | guarded: civil3drust never empties it |
| JSON string present but no JSON flag in section 2 (plain form `0x1148`) | opens, AUDIT clean, but the JSON is read as the alignment's name (no JSON slot); selecting the alignment breaks the GUI | S206 → S213 | guarded: JSON only in the Properties form with flag `0x02` (`exp_chains --json` with a JSON-form template) |
| JSON group of the sampled methods at an end of the alignment (`PreviousId` or `NextId` 0, no fixed element on that side) | opens clean; the group is dropped silently: not drawn, JSON string emptied on save | S223 → S224; owner: such groups cannot be created, deleting the end entity turns a group into separate pieces (S225) | guarded: `aecc_layout::layout` refuses a group with `PreviousId` or `NextId` 0 |
| Group alignment with a JSON layout whose MIST1 does not match (zeroed, or JSON geometry changed) | opens; Event Viewer warning "contains new constraint curve group modified unexpectedly in old version. The modification won't be kept in new release."; Civil 3D rebuilds the pieces from the JSON (new piece IDs, same geometry) and recomputes MIST1; the group stays a group | S204 → S211, S205 → S212, S214 → S215 | accepted: civil3drust cannot compute MIST1; the warning appears once per written group alignment |
| Null alignment style reference | no RECOVER prompt; alignment listed in Prospector but not drawn; **selecting it crashes Civil 3D** (unhandled access violation reading 0x0098). AUDIT: 3 errors, names "Alignment Style eNullObjectId → Standard"; afterwards it draws as a dot until moved | S67 → S68, S69 | guarded: `create_alignment` refuses a style the target lacks (D-025) |
| Alignment style with no owner | style dropped on save; alignment shown with the default style; without AUDIT the reference is left broken (points at the dropped style) | S70 → S71 (AUDIT), S72 (no AUDIT) | to check: styles are only referenced from the target today |
| Alignment style owned outside Civil 3D's style collection (own dictionary) | style ignored; alignment shown with the default style | S73 | to check: as above; own styles need Civil 3D's collection chain (D-030) |

## 3. Opens; AUDIT reports errors

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
| No companion and no registry entry; the alignment's two companion references null | AUDIT: 2 errors (no objects named). With fixing, the references stay null and no companion is created; without fixing, the file is kept as is | S30 → S31, S32; probably also the 2 errors in S67, S70, S73 | guarded: `create_alignment` writes a companion and registry entry unless `--no-parcel-entry` |
| Alignment's edge ID field (133 bits after MIST1/MIST2) ≠ the ID of its edge in the site container | AUDIT: 1 error, fixed — Civil 3D adds the alignment to the container again under a new edge ID on the same nodes and leaves the old edge behind (members list the alignment twice) | S335 → S344 | to do in the library writer; `exp_site move-in` / `move-out` set the field (S345: no issues) |

## 4. Accepted, but Civil 3D changes it

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
| Design-criteria flags set without the criteria file strings or a check set | flags cleared when the alignment is viewed (bits `111 0111` → `100 0101`, tangency check cleared too) | S80 `C1 flags` → S81 | to check: write the flags only with their strings and check set (`aecc_criteria`) |
| Element tag third byte with bit 0x04 (`0x04060008`) | accepted, AUDIT clean; written back as `0x04020008` | S137 → S139 | — (never written) |
| Element tag third byte with bit 0x01 (`0x04030008`) | accepted and kept on save | S127 → S132 | — (never written) |
| Two station anchors with one GUID (a masking region given the GUIDs of two station equations' intervals) | accepted, AUDIT clean; on save the duplicate intervals are dropped and the region takes the equations' stations | S250 → S251 | guarded: new anchors never reuse a GUID the record has (`fresh_guid`) |
| Zero speed-interval GUID together with no graphics, zero MIST1 and no label group (X5) | accepted; Civil 3D gives the table entry a new GUID and writes a second interval for it, leaving the zero-GUID interval as an orphan (MIST13) | S56 X5 → S57 | guarded: `create_alignment` writes a random GUID; zero speed GUIDs are replaced (D-045) |

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
   parameters are locked (none if not), then `0x04`; the first spiral of
   a group kept apart with a JSON layout is `08 00 02 0D` (locked) or
   `08 00 02 09` (unlocked), never `08 00 0D`.
4. Spiral-type codes are only ones Civil 3D wrote (256–262, 264; 263
   crashes Civil 3D).
5. MIST2 is absent and the format marker says "absent" (or both present,
   untouched, as Civil 3D wrote them).
6. Each companion's tail matches its handle references.
7. The style reference is non-null and points at an alignment style the
   target already has (one in Civil 3D's own style collection).
8. Companion references are non-null (or the AUDIT warnings are accepted).
9. Design-criteria flags are set only with their strings and check set.

Checks 1–7 are enforced by the writer's construction today (4: codes
are kept as read, new spirals are clothoids); 8 is the default; 9 is not
enforced yet. A single validation pass that runs before
every write and also checks drawings read from elsewhere is the next step.
