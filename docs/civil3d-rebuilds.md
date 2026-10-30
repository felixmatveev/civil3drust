# What Civil 3D accepts missing and rebuilds

Parts of an alignment and its drawing that Civil 3D accepts absent, zeroed
or stale, with what it does about them when it saves. Under D-049
civil3drust leaves these to Civil 3D: it does not copy them from a
template and does not add them. The opposite list — what Civil 3D refuses
— is `civil3d-rejects.md`.

| part | accepted when | what Civil 3D does on its save | samples | civil3drust |
|---|---|---|---|---|
| Proxy graphics | absent (has-graphics bit 0) | regenerates them from the style and geometry; with PROXYGRAPHICS 0 it writes a placeholder (text + extents) instead | S56 X2 → S57; S140/S141 | written by default, optional (D-027; open under D-049) |
| MIST1, the 64-bit geometry fingerprint (probably `Entity.FingerPrint`) | all zero | recomputes it from shape and placement | S56 X1 → S57; S142–S150 | zeroed (D-046) |
| MIST2 (`BL 0x0D00D773, BL n`) with its format marker | both absent (marker "absent") | adds both on the first re-save | S155 → S157 (plain), S159 → S160 (Properties form) | removed (D-047) |
| Date-modified block (`BL 0x1600101F` + FILETIME, MIST3) | — | written with the time of an Alignment Properties edit | S144, S122 | plan: drop it (test pending) |
| Identity GUID (first speed interval) | zero | assigns a new random GUID (but see the X5 row in `civil3d-rejects.md` §4) | S56 X3 → S57 | random GUID written |
| Identity GUID of generated alignments | any value | sometimes replaced by a new GUID on save (S47, S57 X0/X1/X4), sometimes kept (S50 → S51); the rule is not known | S47, S51, S57 | random GUID written |
| Element-ID list after the speed table | absent (`BL 0`) | adds it after some edits | S50 (accepted); S12, S48, S51 (added) | cleared (`clear_element_id_list`) |
| Station-equation label group | absent | accepted; not added back | S27 → S29 | optional (`--no-label-group`) |
| Companion points | stale (another alignment's) | AUDIT clean; regenerated from the alignment | S34 → S35 | rebuilt from the geometry (D-040) |
| Element-0 station cache | stale (donor's value) | AUDIT clean | S26 | written from the geometry since S33 |
| Design check set reference | null with check set off | sets a reference on save | S80 → S81 | copied when the target has it, else null (D-039) |
| Speed interval of a speed on the first element | absent, with a zero GUID in the table | accepted; this is how Civil 3D itself stores a layout-tool alignment's default speed | S161 8C56, S123/S124 (2027) | intervals written; omitting them is a candidate |
| Civil 3D drawing infrastructure (settings, style collections, registry) | a plain AutoCAD drawing | builds it when it opens or saves the drawing | S64 → S65; S67 → S68/S69 | not written; drawings must be Civil 3D-saved (D-030) |
| Names | any length (1 to 65 characters tried) | kept | S36 → S39, S37 → S40 | written as given |

## Not tolerated

Not everything Civil 3D could rebuild may be left out. These look like
caches but are checked; see `civil3d-rejects.md`:

- the element tag `0x04020008` and the MIST2 tag `0x0D00D773`;
- the format marker, which must match whether MIST2 is present;
- the string count (three strings per free arc);
- the alignment style reference;
- the companion tail, which must match its handle references.
