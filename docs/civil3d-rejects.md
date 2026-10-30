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
| String stream too short: each free arc needs 3 empty strings (tangent-only 12 strings, one arc 15, two arcs 18) | does not open; RECOVER deletes the alignment (it reads past the end of the strings) | S46; fixed in S50 (S49 compared) | guarded: `set_geometry` sets the string count from the arcs |
| Alignment type 2, 3 or 5 on a plain alignment (Offset, CurbReturn, Rail need data a plain alignment lacks) | errors; RECOVER removes the alignments | S61 → S63 | guarded: `create_alignment --type` names only 1 and 4; other numbers are an experiment option |
| Element sub-object tag with a wrong type part: top byte ≠ 4, or third byte without bit 0x02 | "Error 34 (eWrongObjectType)", alignment discarded | S126, S128, S136, S138 | guarded: elements are always written with `0x04020008` |
| Element sub-object tag with a wrong low word (anything but `0x0008`: 0, 7, 9, `0x0108`) | "Error 151 (eOutOfRange)", alignment discarded | S129, S133, S134, S135 | guarded: as above |
| Element tag bytes reversed (`0x08000204`) | eOutOfRange, discarded; the drawing opens very slowly and draws with glitches | S130 | guarded: as above |
| MIST2 tag ≠ `0x0D00D773` | eWrongObjectType, alignment discarded | S151, S153 | guarded: civil3drust never writes MIST2 (D-047) |
| MIST2 removed while the format marker (section-2 bits 115–124) still says "present" — plain form `0x1188`, Properties form `0100000001` | eWrongObjectType, alignment discarded | S154 (plain), S158 (Properties) | guarded: `remove_resave_field` switches the marker with the field |
| Companion tail entry count does not match its handle references (no face-like entry ↔ 3 references; one entry ↔ 5) | companion discarded, eWrongObjectType ("6 errors fixed, 3 objects erased"); the alignments keep dangling companion references | S108 → S109 | guarded: the rebuild keeps the template's edge flags and tail (D-040, test `tail_entry_matches_reference_count`) |

## 2. Opens, then misbehaves

| condition | Civil 3D's response | samples | civil3drust |
|---|---|---|---|
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

1. Every free arc has its 3 strings; the string count matches the elements.
2. Alignment type is 1 or 4 unless the data for other types is written.
3. Every element starts with `0x04020008`.
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
