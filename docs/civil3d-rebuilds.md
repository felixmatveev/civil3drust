# What Civil 3D accepts missing and rebuilds

Parts of an alignment and its drawing that Civil 3D accepts absent, zeroed
or stale, with what it does about them when it saves. Under D-049
civil3drust leaves these to Civil 3D: it does not copy them from a
template and does not add them. The opposite list — what Civil 3D refuses
— is `civil3d-rejects.md`.

| part | accepted when | what Civil 3D does on its save | samples | civil3drust |
|---|---|---|---|---|
| Proxy graphics | absent (has-graphics bit 0) | regenerates them from the style and geometry; with PROXYGRAPHICS 0 it writes a placeholder (text + extents) instead | S56 X2 → S57; S140/S141 | written by default, optional (D-027; open under D-049) |
| MIST1, the 66-bit geometry fingerprint (probably `Entity.FingerPrint`) | all zero | recomputes it from shape and placement; on an alignment with a JSON layout it first logs "modified unexpectedly in old version" and re-solves the group's pieces from the JSON (new piece IDs, same geometry) | S56 X1 → S57; S142–S150; S214 → S215 | zeroed (D-046) |
| MIST2 (`BL 0x0D00D773, BL n`) with its format marker | both absent (marker "absent") | adds both on the first re-save | S155 → S157 (plain), S159 → S160 (Properties form) | removed (D-047) |
| Date-modified block (`BL 0x1600101F` + FILETIME, MIST3) with the Alignment Properties header (MIST4) | both absent: the plain form | not added back on save; the record stays in the plain form (S193 → S194) | S144, S122; S193, S194 | dropped: `to_plain_form` (D-057) |
| Speed-interval GUID (first interval) | zero | assigns a new random GUID (but see the X5 row in `civil3d-rejects.md` §4) | S56 X3 → S57 | random GUID written |
| Speed-interval GUID of generated alignments | any value | sometimes replaced by a new GUID on save (S47, S57 X0/X1/X4), sometimes kept (S50 → S51); the rule is not known | S47, S51, S57 | random GUID written |
| Element-ID list after the speed table | absent (`BL 0`) | adds it after some edits | S50 (accepted); S12, S48, S51 (added) | cleared (`clear_element_id_list`) |
| Station-equation label group | absent | accepted; not added back | S27 → S29 | optional (`--no-label-group`) |
| Companion points | stale (another alignment's) | AUDIT clean; regenerated from the alignment | S34 → S35 | rebuilt from the geometry (D-040) |
| First interval's length to the element end (the station at the end of element 0) | stale (donor's value) | AUDIT clean | S26 | written from the geometry since S33 |
| Design check set reference | null with check set off | sets a reference on save | S80 → S81 | copied when the target has it, else null (D-039) |
| Speed interval of a speed on the first element | absent, with a zero GUID in the table | accepted; this is how Civil 3D itself stores a layout-tool alignment's default speed | S161 8C56, S123/S124 (2027) | intervals written; omitting them is a candidate |
| Civil 3D drawing infrastructure (settings, style collections, registry) | a plain AutoCAD drawing | builds it when it opens or saves the drawing | S64 → S65; S67 → S68/S69 | not written; drawings must be Civil 3D-saved (D-030) |
| Names | any length (1 to 65 characters tried) | kept | S36 → S39, S37 → S40 | written as given |
| Curve flag (MIST8, the bit after a curve's header) | 0 | set to 1 for every curve read from the file when the drawing is opened; new curves keep 0 until the next open | S207–S210 | 0 for new curves, kept on rewritten ones |
| Parameter flag of curves, spirals and groups (the first bit of the record) | 0 (unsolved, as civil3drust writes new and re-encoded elements) | sets it to 1 on save; geometry unchanged | S188 → S195, S190 → S196 | written as 0 for new elements |
| Pieces of a spiral group with a JSON layout (Reverse SSCSS/SSCS/SCSS) | absent (only the two fixed elements and the JSON group object), linked to each other or to the group ID | solves the group from the JSON's typed parameters, trims the fixed elements, writes the pieces; the JSON's values win over pieces that were written | S216 → S217 | written as JSON only (D-059) |

## Not tolerated

Not everything Civil 3D could rebuild may be left out. These look like
caches but are checked; see `civil3d-rejects.md`:

- the element header (`0x0008`, lock flag, `0x04`) and the MIST2 tag
  `0x0D00D773`;
- the format marker, which must match whether MIST2 is present;
- the string count (D-056: three strings per curve, spiral or group record, and per speed with an interval);
- the alignment style reference;
- the companion tail, which must match its handle references.
