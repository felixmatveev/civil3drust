# Unread blocks of the control objects (alignments, profiles, feature lines)

One list of what is not decoded yet in the records around alignments,
profiles and feature lines, as of S320. It points to the detailed
entries (MIST numbers, the object docs). Its purpose: since Civil 3D
cross-references these objects heavily, decode each related object
far enough to give it a typed skeleton (known fields + named raw blocks),
then use the links between them to settle the blocks below.

Legend: **opaque** — never looked at; **seen** — values seen, meaning
unknown; **partial** — some fields read.

## Alignment (`AECC_ALIGNMENT`)

| block | status | where it is described |
|---|---|---|
| Section 2, 231–396: ~165 bits, almost all zero | opaque | alignment-map.md §1.1 |
| MIST1 geometry fingerprint (66 bits) | role known, algorithm not | MIST1 |
| MIST2 `0x0D00D773, n`; MIST3 date stamp; MIST4 Properties header bits | handled, contents not decoded | MIST2–MIST4 |
| Section 2, 13 unknown bits before the criteria flags | opaque | alignment-map.md §1.1 |
| Station-equation section-2 bits (S9: +196 bits) | seen | alignment-map.md §1.2 |
| Element header flag bits (0x01, 0x04) | partial | MIST7 |
| Spiral tail `BD 0, BD 0, BD 1` | seen, constant | MIST16 |
| Post-cache 1 bit (block bit 340) | always 0 | alignment-map.md §1.2 |
| Offset block (type 2): head's middle value, region tail, transition values, element map's `−1`, anchors' kinds | partial | MIST19 |
| Connected block (type 3): head values, offset sign, `B 0, BD 0, BD 0, B 0`, empty regions, trailing pick points | partial | MIST20 |
| Superelevation / cant data | opaque (S271 shows section-2 changes) | MIST19 |

## Profile (`AECC_PROFILE`)

| block | status | where |
|---|---|---|
| Between the graphics and the first element (incl. bit 21, the prefix-2 record with ID 0 in newer copies) | opaque | profile-object.md "Open" |
| Element header: 5 bits after `00 0x08 0x00` | always 0 | profile-object.md |
| Parabola tail: `BL 0`, 3 bits, third criteria BD (HSD?) | partial | profile-object.md |
| Circular and asymmetric records after their parameters | opaque | profile-object.md |
| After the element records (PVI anchors to alignment geometry?) | opaque | profile-object.md |
| Parameter code 0x2001 | not seen | profile-object.md |
| Label point name strings (`A`, `HP`, `BVC` …) | seen | profile-object.md |
| Other profile kinds: surface (EG), offset, superimposed, from a feature line or corridor | not sampled | profile-object.md |

## Profile view (`AECC_PROFILE_VIEW`)

Only the links and a few values are read: station and elevation ranges
(seen as BDs), insertion point, Automatic / user modes, bands, split
views — **opaque as a record**.

## Feature line (`AECC_FEATURE_LINE`)

| block | status | where |
|---|---|---|
| Data between the graphics and the strings (~4600 bits, two repeated blocks) | opaque | feature-line-object.md |
| The two XRECORDs created with it | opaque (they are replaced on every save) | feature-line-object.md |
| Vertex flags (50, 2, 10, 178, 0x10F0032, 0x400400) and edge flags 0x10830 | seen | feature-line-object.md, MIST12 |
| Edge's `after` list (`[1]` with an elevation point) | seen | feature-line-object.md |

## Shared objects

| object | status | where |
|---|---|---|
| Site (`AECC_PARCEL`, `AeccImpFace`): the 760-bit block apart from key, date stamp and two defaults; the parcel counters | partial (S321–S331) | site-object.md, MIST23 |
| Site container (`AECC_FEATURE_STYLE`): read to the last bit (nodes, edge slots, faces, tail); open: the loop selector, flag bits, the counter pairs | mapped (S321–S331; readers not updated yet) | site-object.md |
| Alignment companion: tail after the edge | seen, constant | aecc_topology |
| Site list (`AECC_PARCEL_NODE`): keys, names, order, last-created pointer | mapped; unnamed sites open | site-object.md, MIST22 |
| Alignment list (`AECC_ALIGNMENT_PARCEL_NODE`) | references read | aecc_site |
| Parcels (`AECC_PARCEL` per closed region): tail and references read, the rest opaque | partial | site-object.md |
| Profile helpers (`AECC_DB_FLOAT_TANGENT_GRADELEN`, `…FIXED_PARABOLA_HLPT`, `…HLPTPATSPEAK`, `AECC_ENTITYCIRCLEINFOS`, `AECC_ENTITYEXTENDPARAM`, `…FIXED_ARC_HLPT`): 11 bytes, empty so far | seen | profile-object.md |
| `AECC_NOTIFICATIONINFOS`: 18 constant bits | partial | profile-object.md |
| Label groups (alignment station/geometry/design speed/station equation/…; profile line/PVI/sag/crest; view data band) | references read; bodies copied | alignment-map.md §2, profile-object.md |
| Design check sets, criteria files, label sets | opaque | aecc_criteria (flags only) |
| Styles (alignment, profile, view, feature line, site) | passthrough; components read for alignment styles | aecc_style |

## Order proposed for the type skeleton

1. **Typed skeleton per record**: every class above gets a struct with
   its read fields and its unread parts as named raw spans, in one reader
   (the dump and the workbench show the spans). The skeleton is a reading
   aid; writers keep raw records (D-049).
2. **Links first**: the cross references (site ↔ container ↔ members,
   alignment ↔ profiles ↔ views ↔ label groups, profile → helpers) are
   mostly read; a graph of them per drawing locates which unread block
   holds an ID, a handle or a station of another object.
3. **Then the objects whose unread data is most likely to point back**:
   the profile view record (ranges, bands), the profile's post-element
   block (anchors to alignment geometry), the site record and the
   container's face list, the feature line's data block.
