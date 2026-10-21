# Sample drawings

Purpose-made drawings used for the findings in [`../docs/`](../docs/). Each
differs from another by a controlled, documented change. Only files made
for this project belong here; never add client or production drawings.

Civil 3D files were saved by Autodesk Civil 3D 2024 (build U.202, AutoCAD
24.3) in AC1032 (2018) format. Generated files were written by acadrust
0.6.5 through this project's code.

| ID | File | Made by | How | Bytes | SHA-256 (first 16) |
|----|------|---------|-----|------:|--------------------|
| S1 | `S1_TEST-ALIGN.dwg` | Civil 3D 2024 | New drawing; Insert → LandXML of one 100-unit tangent (0,0)→(100,0); saved. Placeholder display cache. | 979356 | `b318e47651a0088f` |
| S2 | `S2_TEST-LINE1.dwg` | Civil 3D 2024 | New drawing (meters); 2-vertex polyline (0,0)–(100,0) from DXF; Create Alignment from Objects (alignment runs (100,0)→(0,0)); saved. | 449024 | `d7cf5802c36b9df6` |
| S3 | `S3_TEST-LINE1_L150.dwg` | generated | S2 with the tangent set to 150 by the pre-repository prototype (same patches as `aecc_alignment_edit`), written by acadrust 0.6.5. | 430012 | `9815f9f9b0b6cbf5` |
| S4 | `S4_TEST-LINE1_L150_civil.dwg` | Civil 3D 2024 | S3 opened in Civil 3D (worked), saved. | 449285 | `1927652aed2777e7` |
| S5 | `S5_100-200-300.dwg` | Civil 3D 2024 | Alignment `Alignment - (2)`: tangents (0,0)–(100,0)–(200,50)–(300,50), stations from 0+00; saved before being drawn (placeholder). Pair sample A. | 978034 | `11c7a1495a9ab34f` |
| S6 | `S6_100-200-30010.dwg` | Civil 3D 2024 | As S5 with segment 2 10 ft longer. Pair sample B. | 981584 | `7cfe392b313ce64d` |
| S7 | `S7_A_seg2plus10.dwg` | generated | S5 with segment 2 set to 121.80339887498948 by `examples/set_length`; acadrust dropped the `3DSOLID` in `PRESSUREPARTERRORBLOCK`. | 944989 | `ca970153c9c11980` |
| S8 | `S8_A_seg2plus10_civ3d_resaved.dwg` | Civil 3D 2024 | S7 opened in Civil 3D (AUDIT 0 errors), saved. | 960944 | `df0b9e24be8db1a0` |
| S9 | `S9_A_seg2plus10_civ3d_resaved_sta100.dwg` | Civil 3D 2024 | S8 with start station changed to 1+00, saved. | 961741 | `e004a7619b0fe76e` |

Comparison pairs used so far: S2→S4 (Civil 3D re-save of a patched file),
S5→S6 (segment 2 +10 ft), S7 vs S6 (generated vs Civil 3D), S7→S8
(Civil 3D re-save), S8→S9 (start station).

S1, S5–S9 come from a template whose `PRESSUREPARTERRORBLOCK` holds a
`3DSOLID` acadrust 0.6.5 cannot parse (S7–S9 no longer have it).
