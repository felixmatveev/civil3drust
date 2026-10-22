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
| S10 | `S10_arc_civ3d.dwg` | Civil 3D 2024 | Alignment `Alignment - (3)`: a single arc, R = 100 ft, from (0,0) to (100,100) (centre (0,100), turning left); saved before being drawn (placeholder). | 972957 | `2ba4e7ba2c87348c` |
| S11 | `S11_100-200-300_r50.dwg` | Civil 3D 2024 | S5 with a 50 ft free curve (fillet) added at the first PI (100,0) in Civil 3D; saved before being drawn (placeholder). | 985259 | `fa71ac1b7eac5296` |
| S12 | `S12_100-200-300_r75.dwg` | Civil 3D 2024 | S11 with the free curve radius changed from 50 to 75 ft in Civil 3D; saved. | 981895 | `014ba663499648c3` |
| S13 | `S13_S11_r75_generated.dwg` | generated | S11 with the free curve radius set to 75 by `examples/set_radius`; acadrust dropped the `3DSOLID` in `PRESSUREPARTERRORBLOCK`. | 946198 | `752fe2a788491712` |
| S14 | `S14_S13_civ3d_resaved.dwg` | Civil 3D 2024 | S13 opened in Civil 3D (AUDIT 0 errors, geometry correct), saved. | 971048 | `31c382f345c851fb` |
| S15 | `S15_100-200-300_r50_seg3plus100.dwg` | Civil 3D 2024 | S11 with tangent 3 lengthened by 100 ft (PI-to-PI 100 → 200) in Civil 3D; saved. | 980908 | `2ca95b85944e46f0` |
| S15 | `S15_100-200-300_r50_seg3plus100.xml` | Civil 3D 2024 | LandXML 1.2 export of S15 by Civil 3D (reference geometry). | 2038 | `485fd5e19f8bdaab` |
| S16 | `S16_S11_seg3plus100_generated.dwg` | generated | S11 with tangent 3 set to 200 by `examples/set_length`; acadrust dropped the `3DSOLID` in `PRESSUREPARTERRORBLOCK`. | 946198 | `e4f0d63dc40fd267` |
| S17 | `S17_generated_L90_r50.dwg` | generated | S11 used as a template: all PIs set to (0,0), (100,0), (100,75), (100,100) by `examples/set_pis`, keeping the R = 50 free curve (now a 90° turn). The PI at (100,75) has zero deflection because the template has four PIs. | 946198 | `975ae1ab1d1a77b5` |
| S17 | `S17_generated_L90_r50.xml` | Civil 3D 2024 | LandXML 1.2 export by Civil 3D after opening S17 (AUDIT 0 errors). | 1882 | `532024616a90db2d` |
| S18 | `S18_r50_NO_STYLE.dwg` | Civil 3D 2024 | The S15 alignment with the owner's own alignment style `NO STYLE` (created with *New*, every component off); purged. | 930646 | `e54b4d4f8127645a` |
| S19 | `S19_r50_TEST_STYLE.dwg` | Civil 3D 2024 | As S18, alignment style `TEST STYLE` (owner's own, *New*, only lines drawn). Both styles exist in both files. | 930874 | `ae054085dc7f117f` |
| S20 | `S20_S18_TEST_STYLE_generated.dwg` | generated | S18 with the alignment style set to `TEST STYLE` by `examples/set_style` (one handle byte). Its alignment record is byte-identical to S19's. Opened in Civil 3D: AUDIT 0 errors. | 914473 | `22a1d39bbe00f5f4` |
| S21 | `S21_TEST_STYLE_line_off.dwg` | Civil 3D 2024 | As S19, `TEST STYLE` with the line component off (draws nothing). | 939379 | `c8c23d4ffa36e46c` |
| S22 | `S22_TEST_STYLE_line_on.dwg` | Civil 3D 2024 | S21 with `TEST STYLE`'s line component turned on (only change). | 938282 | `28334aba5c207f3b` |
| S23 | `S23_S21_line_on_generated.dwg` | generated | S21 with component 0 (line) of `TEST STYLE` turned on by `examples/style_components` (one bit). | 916134 | `e92104ac03feff32` |
| S24 | `S24_ZERO_CIVIL.dwg` | Civil 3D 2024 | Owner's simplified, purged base drawing: one alignment with the S15 geometry and `TEST STYLE`; no 3DSOLID (reads without failsafe). Base for later samples. | 472697 | `902f7ab2a846bc79` |
| S25 | `S25_ZERO_CIVIL_second_alignment.dwg` | Civil 3D 2024 | S24 plus `Alignment - (2)`, created from a polyline (100,100) → (0,100), `TEST STYLE`. Creation sample A. | 473088 | `380569f9d3b3e64e` |
| S26 | `S26_S24_created_alignment.dwg` | generated | S24 plus alignment `civil3drust - 1`, tangent (0,−100) → (200,−100), by `examples/create_alignment` (donor S25). Opened in Civil 3D: AUDIT 0 errors ("Non Autodesk DWG" notice on open). | 449435 | `01cf39c694422bc8` |
| S27 | `S27_S24_created_alignment_no_label_group.dwg` | generated | As S26 without the station-equation label group. Opened in Civil 3D: AUDIT 0 errors (same notice). | 449413 | `f39457789388ea83` |
| S28 | `S28_S26_civ3d_resaved.dwg` | Civil 3D 2024 | S26 opened in Civil 3D (AUDIT 0 errors), saved. | 473638 | `14a855aa9c890a97` |
| S29 | `S29_S27_civ3d_resaved.dwg` | Civil 3D 2024 | S27 opened in Civil 3D (AUDIT 0 errors), saved; no label group was added. | 474462 | `fc003fd073966c86` |
| S30 | `S30_S24_created_alignment_no_parcel_entry.dwg` | generated | As S27 (no label group) and also without the parcel entry: no companion object, registry unchanged from S24, the alignment's two companion references nulled. Opened in Civil 3D: AUDIT **2 errors** (not fixed). | 449218 | `7ea41a31f21227ea` |
| S31 | `S31_S30_civ3d_resaved.dwg` | Civil 3D 2024 | S30 opened (AUDIT: 2 errors found, 0 fixed), saved without fixing. | 472421 | `a071bb0e0c883c09` |
| S32 | `S32_S30_audit_fixed.dwg` | Civil 3D 2024 | S30 opened, AUDIT with fixing (2 errors), saved. | 473092 | `67352f1541bae3e1` |
| S33 | `S33_S24_created_alignment_v2.dwg` | generated | As S26 (parcel entry + label group) after the first-element end-station fix: the cached station at bit 2205 is 200, not the donor's 99.99999999999999. | 449435 | `601f28fffb6e98a9` |
| S34 | `S34_S25_companions_swapped.dwg` | generated | S25 with the parcel companions of the two alignments swapped (alignment refs and companion back-pointers) by `examples/swap_companions`. Experiment. Opened in Civil 3D: AUDIT 0 errors. | 449628 | `1f2c833fd747fcb0` |
| S35 | `S35_S34_civ3d_resaved.dwg` | Civil 3D 2024 | S34 opened (AUDIT 0 errors), saved. | 472416 | `461c5cf809a5f40c` |

Comparison pairs used so far: S2→S4 (Civil 3D re-save of a patched file),
S5→S6 (segment 2 +10 ft), S7 vs S6 (generated vs Civil 3D), S7→S8
(Civil 3D re-save), S8→S9 (start station). S10 is a single-arc reference. S5→S11 (free curve added at PI 1), S11→S12 (radius 50→75), S13 vs S12 (generated vs Civil 3D), S13→S14 (Civil 3D re-save), S14 vs S12, S11→S15 (tangent 3 +100 ft past a fillet), S16 vs S15 (generated vs Civil 3D), S18 vs S19 (alignment style), S20 vs S19 (generated vs Civil 3D), S21 → S22 (style line component on), S23 vs S22 (generated vs Civil 3D), S24 → S25 (alignment created), S26/S27 (alignment generated with/without label group), S26→S28 and S27→S29 (Civil 3D re-saves). S15's LandXML is Civil 3D's own export of the same geometry.

S1, S5–S19 come from a template whose `PRESSUREPARTERRORBLOCK` holds a
`3DSOLID` acadrust 0.6.5 cannot parse (S7–S9, S13, S14, S16 and S17 no longer have it).
