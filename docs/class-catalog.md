# Class catalog: public API framework + what we decoded

This catalog organises everything decoded so far around Civil 3D's
**public .NET API** (Autodesk's online reference; names and meanings only,
per the cleanroom rules). The API gives the class hierarchy and the names
of the public properties; our samples give where and how each one is
stored. Fields that have no public counterpart are classified as caches or
serialization plumbing (D-048).

DXF and C++ class names come from the class tables in our own sample
drawings. API pages used are listed at the end.

## Field categories

Which fields Civil 3D checks and which it rebuilds is collected in
`civil3d-rejects.md` and `civil3d-rebuilds.md`.

| category | meaning | how civil3drust treats it |
|---|---|---|
| **model** | the value behind a public API property | decoded, edited, written |
| **derived** | computed by Civil 3D from model data | recomputed by civil3drust (D-026) |
| **cache** | private bookkeeping Civil 3D rebuilds on save | zeroed or removed, never copied (D-046, D-047) |
| **plumbing** | tags and markers that frame sub-objects | written exactly; Civil 3D checks them |
| **open** | not explained yet | copied from the template; tracked as a MIST |

## Working model: a record follows the C++ class hierarchy

Each AECC record looks like the output of a chain of C++ `dwgOutFields`
calls, base class first. Within the record, nested sub-objects start with
a **type tag**, a full-form BL that tells the reader which class to build:

- Changing a tag's type bytes is rejected as "eWrongObjectType", and
  changing its trailing word as "eOutOfRange" (MIST7, S126–S138; MIST2,
  S151), as a class factory would fail.
- The same tag appears in unrelated classes when they share a sub-object
  class (`0x04020008` in alignment elements, profile styles, turnout
  styles …; `0x1600101F` in 82 classes of S131, S144 and S117 alone, see
  below).

This is a working model, inferred from the samples, not confirmed.

## Classes involved in an alignment

| API class | DXF class | C++ class (class table) | role | civil3drust |
|---|---|---|---|---|
| `Alignment` (← `Feature` ← Civil `Entity`) | `AECC_ALIGNMENT` | `AeccDbAlignment` | the alignment entity | read, edit, create (`aecc_alignment`, `aecc_elements`, `aecc_build`, `aecc_create`, …) |
| `AlignmentEntity` and subclasses (`AlignmentLine`, `AlignmentArc`, `AlignmentSpiral`, `AlignmentSCS` …) | — (sub-objects inside the record) | — | elements | lines, arcs, line-spiral groups read; lines and free arcs written |
| `DesignSpeed` (`CivilWrapper<AeccDbAlignment>`) | — (sub-objects inside the record) | — | design speeds | read, written (`aecc_design_speed`) |
| `StationEquation` (via `Alignment.StationEquations`) | — | — | station equations | not decoded (S9) |
| `AlignmentStyle` | `AECC_ALIGNMENT_STYLE` | `AeccDbAlignmentStyle` | display style | component visibility (`aecc_style`) |
| `AlignmentLabelSetStyle` | `AECC_ALIGNMENT_LABEL_SET` | `AeccDbAlignStnLabelsDlgTemplate` | label set | referenced only |
| `AlignmentDesignCheckSet` | `AECC_ALIGNMENT_DESIGN_CHECK_SET` | `AeccDbAlignmentDesignCheckSet` | design check set | referenced (`aecc_criteria`) |
| `AlignmentDesignSpeedLabelGroup` | `AECC_ALIGNMENT_DESIGNSPEED_LABEL_GROUP` | `AeccDbAlignmentDesignSpeedLabeling` | label group | optional; copied or left out |
| `AlignmentStationLabelGroup`, `AlignmentMinorStationLabelGroup`, `AlignmentGeometryPointLabelGroup`, `AlignmentStationEquationLabelGroup` | `AECC_ALIGNMENT_STATION_LABEL_GROUP` etc. | `AeccDbAlignmentStationLabeling` etc. | label groups | not created for new alignments |
| `Site` | `AECC_PARCEL` | `AeccDbFace` | site (a topology face) | read-only (`aecc_site`, D-038) |
| — (no public class) | `AECC_FEATURE_STYLE` | `AeccDbFeatureStyle` | the alignment's companion: its entry in the parcel topology | rebuilt (`aecc_topology`, `aecc_registry`) |
| — (no public class) | `AECC_ALIGNMENT_PARCEL_NODE` | `AeccDbAlignmentParcelNode` | owner of the companion registry | referenced |
| — (no public class) | `AECC_DISP_REP_ALIGNMENT…` | `AeccDbViewAlignment…` | display representations | template content, never shipped (D-015) |

## `AECC_ALIGNMENT`, layer by layer

Bit positions refer to `alignment-map.md`. "Layer" is the class in the
hierarchy the field most likely belongs to (inferred from its position).

### AutoCAD `Entity` (public ODA DWG specification)

| field | API | category | notes |
|---|---|---|---|
| object type, own handle, EED size | — | plumbing | object type form depends on the class number (D-044) |
| proxy graphics | — | cache | what AutoCAD shows without Civil 3D; placeholder when PROXYGRAPHICS is 0 (S141) |
| common entity data: colour, linetype, lineweight, … | `Entity.Color`, `Linetype`, `LineWeight`, … | model | `aecc_entity` |
| layer (handle ref 0) | `Entity.Layer` | model | `aecc_alignment_refs` |

### Civil 3D `Entity` and `Feature` (section 2)

| field | API | category | notes |
|---|---|---|---|
| `BL 80, 0, 80, 80, 0` | — | plumbing | class header, same in every record |
| format marker at bit 115: `0x1148` / `0x1188` (plain form), `0011000001` / `0100000001` (Properties form) | — | plumbing | says whether the MIST2 block is present (MIST5; S154, S158) |
| 28 bits at bit 119 (Properties form only) | — | open | MIST4: added by an Alignment Properties edit |
| 64 bits after the 14-bit anchor (MIST1) | `Entity.FingerPrint` (UInt64), probably | cache | Civil 3D recomputes it from shape and placement; zeroed (D-046) |
| `BL 0x1600101F` + FILETIME (Properties form only) | — (a "date modified" stamp; no public property found) | cache | MIST3; the same block is in 82 classes (styles, label sets, check sets …), e.g. S24's styles are stamped when the drawing was made (2026-10-22 19:56:44 UTC); plan: drop it |
| `BL 0x0D00D773, BL n` (MIST2) | — | cache | added on re-save; removed (D-047); n = 2 centerline, 1 utility |
| design criteria flags (bits 19, 18, 17, 14 before the section-2 end) | `UseDesignSpeed` (probable), `UseDesignCriteriaFile`, `UseDesignCheckSet`; tangency check (no public property found) | model | `aecc_criteria` |
| flag 16 before the section-2 end | — | open | MIST15 |
| start station (a BD ending 20 bits before the section-2 end) | `StartingStation` / `ReferencePointStation` | model | `aecc_station` |
| first-element ID (twice) | — | derived | `aecc_station::set_first_element` |
| name, description (string stream) | `Name`, `Description` | model | `string_stream` |
| style (handle ref 5) | `StyleId` | model | `set_style` |

### `Alignment`

| field | API | category | notes |
|---|---|---|---|
| element count + element sub-objects | `Entities` (`AlignmentEntityCollection`) | model | see below |
| station at the end of element 0, speed intervals (element start/end, point) | — | derived | rebuilt from geometry and speeds |
| design-speed table: `BL 0, BL 1, BL n`, n × {`BL 1`, station, speed, GUID} | `DesignSpeeds`: `Station`, `Value`; `SpeedNumber` = position; `Comment` = a string in the string stream | model | GUIDs: random per speed with an interval; zero, without an interval, for layout-tool alignments (S161 8C56, S123) — D-045 |
| `BD 100` after the table | — | open | constant in every sample |
| element-ID list (`BL n`, n BDs) | — | cache | added after some saves; cleared (`clear_element_id_list`) |
| alignment type BL | `AlignmentType`: Centerline 1, Offset 2, CurbReturn 3, Utility 4, Rail 5 | model | 1 and 4 seen (S58/S59; the UI calls 4 "Miscellaneous") |
| `BL 2, 2, 2, 0` | — | plumbing/open | constant |
| implied PI display BL | — (no public property found) | model | MIST6: 1 by direction, 2 by curves, 3 none |
| design criteria file strings | `CriteriaFileName` | model | `aecc_criteria` |
| check set (handle ref 6) | `DesignCheckSetId` | model | nulled when the target lacks it |
| companion (handle refs 2, 3) | `SiteId` / `IsSiteless` (via the companion or site container) | model | `aecc_site` |
| station equations | `StationEquations` | open | gap (S9) |
| — | `CreationMode` (`RuleBasedCreation` 1, `ManuallyCreation` 2) | open | no field in the alignment record (S161: polyline- and layout-created alignments differ only in geometry, name, MIST1 and the speed interval); the companion's edge flag `0x20` marks geometry taken from an object and is cleared by layout edits (MIST12) — a candidate, not confirmed |
| — | `Length`, `EndingStation`, `ReferencePoint` | derived | not stored (checked against S24's listing) |

### `AlignmentEntity` sub-objects (element records)

| field | API | category | notes |
|---|---|---|---|
| 11-bit kind prefix: tangent `…010`, arc `…100`, group `…1010` | `EntityType` (`AlignmentEntityType`: Line, Arc, … LineSpiral …) | plumbing | our prefixes are not the API's enum values |
| `BL 0x04020008` | — | plumbing | sub-object tag (MIST7); top byte 4 and bit 0x02 of the third byte checked, low word must be 8 |
| id, before, after | `EntityId`, `EntityBefore`, `EntityAfter` | model | |
| line kind 0 / 4 / 5 / 9 | `Constraint1` Fixed/Float/Free + `Constraint2` (`AlignmentLineConstraintType`): TwoPoints / ThroughPoint / Length / NoConstraint | model | element-methods.md |
| line start/end points, floating length | `StartPoint`, `EndPoint`, `PassThroughPoint1/2`, `Length` | model | |
| arc parameter block `B flag, BD p1, BD p2, BL n1, n1 BD, BL n2, B closed` | `Constraint2` (`AlignmentArcConstraintType`) by `n2`, see below | model | p1 = design speed at the curve (−1 until solved) |
| curve flag bit | — | open | MIST8 |
| group code 162 (line + spiral) | `AlignmentEntityType.LineSpiral` | model (read) | D-042 |
| curve group index | `CurveGroupIndex`, `CurveGroupSubEntityIndex` | open | not located |

Arc `n2` codes and the closest `AlignmentArcConstraintType` names (by
meaning; the API's numeric values are different):

| n2 | stored as | API name (closest) |
|---|---|---|
| 2 | three points | ThreePoints |
| 0 | centre, radius | CenterRadius |
| 1 | centre, point | CenterPassThroughPoint |
| 13 | radius, end point | PassThroughRadius (probable) |
| 8 | end point, from entity end | PassThroughHoldEnd (probable) |
| 11 | end point, direction | PassThroughDirection |
| 29 | radius (free fillet) | Radius |
| 28 | point on the fillet | PassThrough |
| 21 | radius, length | RadiusAndLength |
| 15 | radius, start point (made by Civil 3D) | — |

## Serialization plumbing found so far

| value | where | evidence |
|---|---|---|
| `0x04020008` | first BL of every alignment element; also in profile, profile-view band, turnout, survey-figure and grading-criteria styles, tree and label nodes (11 classes) | S126–S139 (checked); `tags` on S131, S144, S117 |
| `0xTT0M0008` family: `0x0C030008`, `0x05020008`, `0x06020008`, `0x0A020008` … | style and band classes | MIST7 scan of S119 and an older drawing |
| `0x1600101F` + 64-bit FILETIME | 82 classes: styles, label sets, check sets …; alignments in the Properties form | `tags` on S131, S144, S117; timestamps read in S24's styles and in 154 alignment records |
| `0x0D00D773` | MIST2 block; alignments only (section-2 bit 231 or 363) | S151 (checked); `tags` on S131, S144, S117 |
| `0x1148` / `0x1188` | section-2 bit 115 of alignments | S154, S155 (checked) |
| `BL 80, 0, 80, 80, 0` | start of the class data, every alignment | all samples |

`examples/tags.rs` lists candidate tags: full-form BLs reused across
classes. Its output is noisy — UTF-16 text, repeated style data and
one-bit-shifted copies of real tags also match — so a value is only
entered here once a test or a decoded block confirms it.

## Open items, in API terms

- **FingerPrint (MIST1):** the API reads and writes it; zero is accepted.
  If data shortcuts or Civil Worksharing compare fingerprints, a zeroed
  value may matter there (untested).
- **Date-modified block (MIST3):** to be dropped; check whether its absence
  needs a marker change (compare MIST4 and S158).
- **CreationMode, CurveGroupIndex, StationEquations, superelevation and
  cant collections:** public properties not yet located in the record.
- **No public property** for the implied PI display or the tangency
  check, although both are user settings.

## Sources (Autodesk Civil 3D .NET API reference, names and meanings only)

- [Alignment class](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/01132a5f-8329-50b8-8959-94bb5f1207bd.htm)
- [AlignmentType enumeration](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/a11fcbba-1e90-ad37-d265-2243da4da299.htm)
- [Civil Entity class](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/25c59e35-81e6-98af-f089-9e8fe125bca8.htm), [Entity.FingerPrint](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/ae0e32a3-5a2e-2e79-5ea4-0fb99e170476.htm)
- [Feature class](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/1674ab56-8191-840c-ca22-58c58a045210.htm)
- [Alignment.CreationMode](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/6f006362-122f-3170-7c0d-14911785a0dd.htm), [AlignmentCreationType](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/7a9667b7-6977-8b17-eda4-d5962e230857.htm)
- [AlignmentLine class](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/a5e3994b-9649-b265-d0ce-1f3fd67cc64a.htm), [AlignmentEntityCollection](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/58c5b585-6744-f5e2-1fb1-274ec5d57cd2.htm)
- [AlignmentEntityType](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/03162d03-2c0c-3e90-54a8-8706424f3bf9.htm), [AlignmentEntityConstraintType](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/af4aa921-286d-17f5-49e0-9fd8582a0590.htm), [AlignmentLineConstraintType](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/267f0081-f1ba-d2df-0c03-ad7c8077357d.htm), [AlignmentArcConstraintType](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/44c78b46-a3d2-fe35-9537-9ef17f299480.htm)
- [DesignSpeed class](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/e0ba3811-0f31-3304-505e-317a3f153334.htm), [Specifying Design Speeds](https://help.autodesk.com/cloudhelp/2017/ENU/Civil3D-DevGuide/files/GUID-139B5F29-C442-465D-BC4E-C31A58086939.htm)
- [Autodesk.Civil.DatabaseServices namespace](https://help.autodesk.com/cloudhelp/2020/ENU/Civil3D-API/files/html/73fd1950-ee31-00b8-4872-c3f328ea1331.htm), [Styles namespace](https://help.autodesk.com/cloudhelp/2022/ENU/Civil3D-API/files/html/b9f8f1ba-4f26-79a8-6354-46973d084161.htm), [AlignmentDesignCheckSet](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/af544d01-9f0c-edd9-a0bb-4da02ee0032b.htm), [Site.GetAlignmentIds](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/afd23041-5b03-f862-6cef-1b1066c782a4.htm)
