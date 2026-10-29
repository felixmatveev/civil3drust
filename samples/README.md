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
| S36 | `S36_S24_renamed_A.dwg` | generated | S24 with the alignment renamed `A` by `examples/set_name` (750 → 722-byte record). Opened in Civil 3D: AUDIT 0 errors. | 448679 | `b99b7036c391c700` |
| S37 | `S37_S24_renamed_long.dwg` | generated | S24 with the alignment renamed to a 65-character name (750 → 850 bytes). Opened in Civil 3D: AUDIT 0 errors. | 448775 | `b69c6c6be7264eb4` |
| S38 | `S38_S24_created_B.dwg` | generated | S24 plus alignment `B` (0,−100) → (200,−100) by `examples/create_alignment` (donor S25). Opened in Civil 3D: AUDIT 0 errors. | 449506 | `f31fe0462f0e4e2f` |
| S39 | `S39_S36_civ3d_resaved.dwg` | Civil 3D 2024 | S36 opened in Civil 3D (AUDIT 0 errors), saved; name `A` kept. | 472069 | `9a7e881494a653a3` |
| S40 | `S40_S37_civ3d_resaved.dwg` | Civil 3D 2024 | S37 opened in Civil 3D (AUDIT 0 errors), saved; 65-character name kept. | 472827 | `86376b1554ce3762` |
| S41 | `S41_S38_civ3d_resaved.dwg` | Civil 3D 2024 | S38 opened in Civil 3D (AUDIT 0 errors), saved; `B` kept. | 473472 | `e89f35600b0d7a42` |
| S42 | `S42_S41_descriptions.dwg` | Civil 3D 2024 | S41 with descriptions typed in Civil 3D: `Alignment - (1)` "This is test description", `B` a long text with an empty line and "new paragraph". | 473445 | `f5e4d237df6776e1` |
| S43 | `S43_S41_descriptions_generated.dwg` | generated | S41 with the same two descriptions set by `examples/set_name --description`; both alignment records byte-identical to S42's. | 451556 | `b36b50733a517fc1` |
| S44 | `S44_S25_plus_two_tangent_alignment.dwg` | Civil 3D 2024 | S25 (re-saved) plus `Alignment - (3)` created from a 3-vertex polyline (0,−200) → (100,−200) → (200,−150), no curve. | 475488 | `a4020c488117a446` |
| S45 | `S45_S24_created_two_tangents.dwg` | generated | S24 plus `Two tangents` with the same PIs as S44's alignment, by `examples/create_alignment` (donor S25, one-tangent). Element list, cache and graphics equal Civil 3D's except the identity value. Opened in Civil 3D: AUDIT 0 errors. | 449570 | `902a42e479646fda` |
| S46 | `S46_S24_created_five_pis_two_curves.dwg` | generated | S24 plus `C`: PIs (0,−200) (100,−200) (200,−150) (300,−150) (400,−250), free curves R50 at PI 1 and R30 at PI 3. **Does not open in Civil 3D**; RECOVER removes alignment `C` (fixed in S50). | 449881 | `602062bf256ae6f1` |
| S47 | `S47_S45_civ3d_resaved.dwg` | Civil 3D 2024 | S45 opened (AUDIT 0 errors), saved. Both alignments got new identity GUIDs. | 472867 | `085e9a329e414716` |
| S48 | `S48_S25_plus_C_no_curves.dwg` | Civil 3D 2024 | S25 plus alignment `C` from a polyline (0,−200) (100,−200) (200,−150) (300,−150) (400,−250), no curves; saved. | 472870 | `dfd9ba74970f7fff` |
| S49 | `S49_S48_C_with_curves.dwg` | Civil 3D 2024 | S48 with free curves added to `C`: R50 at PI 1, R30 at PI 3; saved. Reference for S46. | 474891 | `8893150dda09717a` |
| S50 | `S50_S24_created_C_v2.dwg` | generated | S46 regenerated after the fix (3 empty strings per arc, attribute chunks after arcs): equals S49's `C` in elements, strings and graphics apart from the identity, the 62-bit field and PC-station rounding. Opened in Civil 3D: AUDIT 0 errors. | 450048 | `b6aec3a18a620821` |
| S51 | `S51_S50_civ3d_resaved.dwg` | Civil 3D 2024 | S50 opened (AUDIT 0 errors), saved; geometry and identity GUID kept. | 472836 | `55d10d4c2c411195` |
| S52 | `S52_S44_created_D.dwg` | generated | S44 (three alignments, registry with three entries) plus `D` (0,−300) (150,−300) (250,−250, R40) (350,−250), template from S25 — the donor is no longer target + 1; registry entry added by `aecc_registry`. Opened in Civil 3D: AUDIT 0 errors. | 453160 | `657c743c043db7ce` |
| S53 | `S53_S24_created_E1_E2.dwg` | generated | S24 plus two alignments created one after the other: `E1 tangents` (0,−400) (120,−400) (200,−350) and `E2 curves` (0,−500) (100,−500, R40) (200,−450) (300,−500, R25) (400,−500). Opened in Civil 3D: AUDIT 0 errors. | 450987 | `b220929ee8151802` |
| S54 | `S54_S52_civ3d_resaved.dwg` | Civil 3D 2024 | S52 opened (AUDIT 0 errors), saved. | 473836 | `0b638a28742c0d39` |
| S55 | `S55_S53_civ3d_resaved.dwg` | Civil 3D 2024 | S53 opened (AUDIT 0 errors), saved (after S54, same session). | 474145 | `cd25f65e6108315c` |
| S56 | `S56_S24_minimal_experiments.dwg` | generated | S24 plus six alignments X0–X5, each with one part emptied (zeroed section-2 field, no graphics, zeroed GUID, no label group, all) by `examples/exp_minimal`. Opened in Civil 3D: AUDIT 0 errors, all six kept. | 454664 | `cfb42e65d3e13ca3` |
| S57 | `S57_S56_civ3d_resaved.dwg` | Civil 3D 2024 | S56 opened (AUDIT 0 errors), saved. (A DXF saved at the same time has an empty ENTITIES section — Civil 3D leaves its entities out of DXF — so it is not kept.) | 474283 | `fa6281adc71b795c` |
| S58 | `S58_O1_centerline.dwg` | Civil 3D 2024 | O1: alignment "Centerline" (1089) created from a polyline, one tangent (0,0)→(500,0), type Centerline. | 474510 | `a18b423c0ed30fc1` |
| S59 | `S59_O1_miscellaneous.dwg` | Civil 3D 2024 | O1: same alignment with type Miscellaneous. | 476389 | `243dbfb556f935db` |
| S60 | `S60_S24_created_centerline_and_misc.dwg` | civil3drust | S24 + "Generated centerline" (type 1) and "Generated misc" (type 4), template S58 1089: `create_alignment --type centerline` / `--type misc`. | 450375 | `bd74df5a936697cd` |
| S61 | `S61_S24_created_types_2_3_5.dwg` | civil3drust | S24 + "Type 2", "Type 3", "Type 5" (`create_alignment --type 2/3/5`, template S58 1089): which types the other values are. | 451306 | `61cc51e939ea55a4` |
| S62 | `S62_S60_civ3d_resaved.dwg` | Civil 3D 2024 | S60 opened (no issues; types shown as Centerline and Miscellaneous), saved. | 473545 | `66a43185caa07904` |
| S63 | `S63_S61_civ3d_recovered.dwg` | Civil 3D 2024 | S61 opened with errors; RECOVER removed the three generated alignments; saved. | 474433 | `f0d9f996f0113a10` |
| S64 | `S64_ZERO_autocad.dwg` | AutoCAD 2024 | Owner's minimal empty AutoCAD drawing: no AECC classes or objects. | 32180 | `5a2e5e47023daa10` |
| S65 | `S65_S64_civ3d_resaved.dwg` | Civil 3D 2024 | S64 opened in Civil 3D and saved: Civil 3D's default drawing infrastructure, no alignment, empty companion registry. | 469758 | `bb779bb512bd8364` |
| S66 | `S66_S65_created_alignment.dwg` | civil3drust | S65 + alignment "Minimal" (0,0)→(500,0), style 48C, no label group, template S58 1089 (`create_alignment … --style 48C --no-label-group`); adds the `AECC_ALIGNMENT` class entry. | 447732 | `711378e77281ad84` |
| S67 | `S67_S64_bare_alignment.dwg` | civil3drust | S64 + one alignment record "Bare" with null style and companion references, plus its class entry (`examples/exp_bare`, record built from S58 1089 in S24). | 27899 | `34cd9d9ddf1c9363` |
| S68 | `S68_S67_civ3d_audit_resaved.dwg` | Civil 3D 2024 | S67 opened (no RECOVER prompt; selecting the alignment crashed), AUDIT (3 errors fixed: null style → Standard), saved. | 467487 | `e24a5b247bbd3daf` |
| S69 | `S69_S67_civ3d_resaved_no_audit.dwg` | Civil 3D 2024 | S67 opened and saved without AUDIT. | 468994 | `0db530a212f325fc` |
| S70 | `S70_S64_bare_alignment_test_style.dwg` | civil3drust | S64 + `TEST STYLE` copied from S24 (owner and default-style pointer null, DASHED2 → Continuous) + alignment "Bare" using it (`examples/exp_bare … FA9`). | 28507 | `6a8fe5fa20a70403` |
| S71 | `S71_S70_civ3d_audit_resaved.dwg` | Civil 3D 2024 | S70 opened (no RECOVER prompt; alignment shown with the default style), AUDIT (2 errors fixed), saved. | 469854 | `a1a4a1b9b172b017` |
| S72 | `S72_S70_civ3d_resaved_no_audit.dwg` | Civil 3D 2024 | S70 opened and saved without AUDIT. | 469764 | `6007bb53fc21f788` |
| S73 | `S73_S64_bare_alignment_test_style_owned.dwg` | civil3drust | As S70, but the style is owned by a new dictionary `CIVIL3DRUST_STYLES` in the named object dictionary (`examples/exp_bare … FA9`). | 28763 | `a0e00161db900dc6` |
| S74 | `S74_O2_design_speeds.dwg` | Civil 3D 2024 | O2: alignments `60MPH` (60 mph), `30MPH` (30 mph), `mixed speed` (60 mph from 0+00, 30 mph from 2+00). | 477571 | `fc6e6130c6b103f2` |
| S75 | `S75_S24_created_design_speeds.dwg` | civil3drust | S24 + `Speed 30` (0:30) and `Speeds 60-30` (0:60 200:30), (0,−100)/(0,−200) → (500,·), template S58 1089; `create_alignment` then `set_speeds`. | 450506 | `5702d205a84eeae7` |
| S76 | `S76_S75_civ3d_resaved.dwg` | Civil 3D 2024 | S75 opened (no errors; garbage comment in the second speed row of `Speeds 60-30`), saved. | 473889 | `29fbfa3751cf8de8` |
| S77 | `S77_S24_created_design_speeds_v2.dwg` | civil3drust | As S75 with one comment string per speed entry, plus `Speeds 50-40-30` (0:50 150:40 300:30). | 451694 | `29fbde1589833fe6` |
| S78 | `S78_S74_mixed_speed_50_40_30.dwg` | civil3drust | S74 with `mixed speed` set to 0:50 150:40 300:30 (`set_speeds`). | 457960 | `afc4cb454a2a78e9` |
| S79 | `S79_O3_design_criteria.dwg` | Civil 3D 2024 | O3: S77 with `Speed 30` renamed `DESIGN ON` (design criteria on) and `Speeds 60-30` renamed `DESING OFF` (criteria off, comment `COMMENT` in a speed row); saved. | 473575 | `4fd7c407f0aef89f` |
| S80 | `S80_S24_criteria_experiments.dwg` | civil3drust | S24 + `C0 control`, `C1 flags`, `C2 checkset`, `C3 full` (`examples/exp_criteria`, template S58 1089, check set 4D3). | 453237 | `2990b91b1976caad` |
| S81 | `S81_S80_civ3d_resaved.dwg` | Civil 3D 2024 | S80 opened, C0/C1/C3 viewed in Alignment Properties (C3 showed S79's criteria), saved. | 473799 | `c6941bf294ede74d` |
| S82 | `S82_O4_start_station.dwg` | Civil 3D 2024 | O4: S81 with C3 deleted and C0/C1/C2 renamed `ZERO START` (start 0+00), `100 PLUS` (reference station −1+00 at the start point), `100 MINUS` (start +1+00); saved. | 474276 | `2ee4850d12095733` |
| S83 | `S83_S24_created_start_stations.dwg` | civil3drust | S24 + `Start 100 speeds kept`, `Start 100 no speeds`, `Start -100 no speeds` (`create_alignment`, template S58 1089, then `set_start` with/without `--clear-speeds`). | 451216 | `69a7b98a874f85ee` |
| S84 | `S84_S83_civ3d_resaved.dwg` | Civil 3D 2024 | S83 opened (AUDIT no errors), saved: the kept speed moved to 1+00. | 473767 | `56b0de4bc2d59d69` |
| S85 | `S85_S24_created_start_shift_speeds.dwg` | civil3drust | S24 + `Shift +100`, `Shift -100 two speeds`, `Shift +250 three speeds`: speeds set, then `set_start` shifting them (D-031). | 452148 | `f0b71d92e532a645` |
| S86 | `S86_S85_civ3d_resaved.dwg` | Civil 3D 2024 | S85 opened (no AUDIT errors), saved. | 473806 | `6e1eeda89a402ff7` |
| S87 | `S87_S82_speeds_added.dwg` | civil3drust | S82 with speeds written by `set_speeds`: `ZERO START` 0:60 100:50 250:40 400:30, `100 PLUS` −100:40 150:25, `100 MINUS` 100:55 (the last two had none). | 453254 | `cbefe388af2ebd86` |
| S88 | `S88_S87_civ3d_resaved.dwg` | Civil 3D 2024 | S87 opened (no AUDIT errors, speeds as written), saved. | 474753 | `225160d6eee17eb3` |
| S89 | `S89_O5_layout_tangents_curves.dwg` | Civil 3D 2024 | O5: alignment by layout tools in the S1–S19 template (3DSOLID: failsafe read): fixed tangent, fixed curve (3 points), floating tangent, free curve, tangent; speeds 0:60, 285.63:30 (on the curve). | 979503 | `e1945d49168d978b` |
| S90 | `S90_S24_curve_speeds.dwg` | civil3drust | S24 + `Curve speed` (0:60 260:30), `Tangent speeds` (0:60 400:30), `Only curve speed` (260:40), each 0,y → 300,y (R 80) → 300,y−300; `create_alignment` + `set_speeds`. | 452303 | `a74e467d6b320427` |
| S91 | `S91_S90_civ3d_resaved.dwg` | Civil 3D 2024 | S90 opened (no AUDIT errors, all speeds shown), saved. | 474086 | `94120d7106e31488` |
| S92 | `S92_S24_created_two_curves.dwg` | civil3drust | O6 base: S24 + `Edit base` (FB3), 0,0 → 300,0 (R 80) → 300,−300 (R 80) → 600,−300, template S58 1089 (`create_alignment`). Elements T1 A4 T2 A5 T3. Start of the add/remove series (S93–S97). | 449923 | `686149058bfc1781` |
| S93 | `S93_S92_civ3d_resaved.dwg` | Civil 3D 2024 | O6 baseline: S92 opened (AUDIT), saved unchanged. | 473949 | `41b2d0bf88313fd6` |
| S94 | `S94_S93_first_tangent_deleted.dwg` | Civil 3D 2024 | O6: S93 with the first tangent (T1) of `Edit base` deleted, saved. | 475718 | `a5f34d5149e23ac1` |
| S95 | `S95_S94_second_curve_deleted.dwg` | Civil 3D 2024 | O6: S94 with the curve between the vertical and the last tangent (A5) deleted, saved. | 473543 | `169304415ca2e00a` |
| S96 | `S96_S95_curve_added.dwg` | Civil 3D 2024 | O6: S95 with a free curve R 80 added back at PI (300, −300), saved. | 474761 | `a8a68f797b40be66` |
| S97 | `S97_S24_element_edits.dwg` | civil3drust | S24 + four alignments created as S92's shape (template S58 1089), then edited with `examples/edit_elements`: `Remove first` (speeds 0:60 300:40, remove 1), `Remove curve` (remove 5), `Add curve` (one curve, `curve 2 80`), `Chain` (remove 1, remove 5, curve 2 80 — S94–S96 replayed). | 454187 | `246f10384695f9a7` |
| S98 | `S98_S97_civ3d_audit_resaved.dwg` | Civil 3D 2024 | S97 opened, AUDIT, all four edited alignments checked (geometry, start station, design speeds editable); `TEST STYLE` curve component turned on; saved. | 478060 | `2cfa8444aa8b21ee` |
| S99 | `S99_S98_last_tangent_deleted.dwg` | Civil 3D 2024 | O6: S98 with the last tangent (T3) of `Remove first` (FB3) deleted, saved. | 476172 | `976d44d140afcf9c` |
| S100 | `S100_S98_interior_tangent_deleted.dwg` | Civil 3D 2024 | O6: S98 with the tangent between the two curves (T2) of `Add curve` (FB9) deleted, saved. | 478157 | `2daf30bca1b9d0fd` |
| S101 | `S101_empty_site.dwg` | Civil 3D 2024 | O7: from S98 (owner): the four generated alignments deleted, `Alignment - (1)` renamed `TEST`, empty site `TEST SITE` (description on two lines) created; saved. | 474722 | `dd4c528e9193aebc` |
| S102 | `S102_alignment_in_site.dwg` | Civil 3D 2024 | O7: S101 with `TEST` moved into `TEST SITE`; saved. | 475518 | `e00a28cf4c09148b` |
| S103 | `S103_second_site.dwg` | Civil 3D 2024 | O7: S102 plus a second, empty site `TEST2`; saved. | 477606 | `2484cef91befbb46` |
| S104 | `S104_second_alignment_in_site.dwg` | Civil 3D 2024 | O7: `TEST` in `TEST SITE` plus a second alignment `CREATED to TEST SITE` (103A, two tangents) created in that site; a second empty site `TEST 2` (1068). (Not from S103: its `TEST2` is gone.) | 477225 | `21d62455410912f3` |
| S105 | `S105_test_moved_to_test2.dwg` | Civil 3D 2024 | O7: S104 with `TEST` moved to `TEST 2`; saved. | 476841 | `1c7cbf8adf1c026c` |
| S106 | `S106_test_back_to_siteless.dwg` | Civil 3D 2024 | O7: S105 with `TEST` moved back to `<None>` (siteless); saved. | 477354 | `d84d2593dcdbcc41` |
| S107 | `S107_S104_created_siteless.dwg` | civil3drust | S104 + siteless `SITELESS NEW` (107D), 0,−400 → 300,−400 (R 80) → 300,−700 → 600,−700, template S98 F9E (`create_alignment`): a new alignment in a drawing with sites. Owner: AUDIT 0 errors; `S107_S104_created_siteless.xml` is Civil 3D's LandXML export of it. | 460235 | `50296fb51c87a7eb` |
| S108 | `S108_S24_rebuilt_companions.dwg` | civil3drust | S24 + `Graph two curves` (FB3: 0,−400 → 300,−400 R80 → 300,−700 R80 → 600,−700), `Graph tangents` (FB6: 0,−900 → 200,−900 → 300,−1000 → 500,−1000), `Graph edited` (FB9: as FB3 at y −1200, then `edit_elements remove 1`); template S98 F9E; companions' graphs rebuilt from the geometry (`aecc_topology::rebuild_companion`, first version). **Rejected**: Civil 3D discarded the three companions (eWrongObjectType), see S109. | 452872 | `f853f001bcf53655` |
| S109 | `S109_S108_civ3d_recovered.dwg` | Civil 3D 2024 | S108 opened: RECOVER discarded companions FB2, FB5, FB8 ("Error 34 (eWrongObjectType)"), 6 errors fixed; saved. The alignments keep dangling companion references. | 474348 | `e102e17a489b0960` |
| S110 | `S110_S24_rebuilt_companions_v2.dwg` | civil3drust | As S108 with the fixed rebuild: edge flags and tail kept from the template companion (S98 F9D: no face-like entry, three references), handle stream trimmed. | 452875 | `497bc4625ad336f7` |
| S111 | `S111_S110_civ3d_audit_resaved.dwg` | Civil 3D 2024 | S110 opened (no RECOVER prompt), AUDIT 0 errors, saved. | 474663 | `ad5fbc40bc51c95b` |
| S112 | `S112_S104_third_alignment_in_site.dwg` | Civil 3D 2024 | O7: S104 + `test 3` (10A3, U shape (43.2,62.6) → (43.2,−86.5) → (131.1,−86.5) → (131.1,75.5)) created in `TEST SITE`, crossing `TEST` and `CREATED to TEST SITE`; Civil 3D made two parcels (10AF, 10B0). | 480845 | `cd4acf872889eeb9` |
| S113 | `S113_line_methods.dwg` | Civil 3D 2024 | Line methods (owner): `fixed line by length` (10E4: fixed line, curve, Fixed Line from curve end + length 100; speeds 0:60, 1+18:30 on the curve), `floating line by length` (10EA: same, last line Floating Line from curve end + length 100), `free line` (10F4: two fixed curves made as separate pieces, then Free Line between them, which joined them), and an alignment `EMPTY` with no elements (10D0). | 483659 | `c08df3e4efbf07f7` |
| S114 | `S114_S113_curve_speeds.dwg` | civil3drust | S113 + speeds `free line` 0:60 10:50 200:40 350:30 (10 on the first curve, 350 on the last) and `floating line by length` 0:60 120:35 (on the curve), written by `set_speeds` with curve intervals. | 461924 | `f3b3f36e2eafe7cd` |
| S115 | `S115_S114_civ3d_resaved.dwg` | Civil 3D 2024 | S114 opened (no errors; speeds shown and editable, owner), saved. | 484550 | `311468ad8480ab27` |
| S116 | `S116_S24_curve_method_alignments.dwg` | civil3drust | S24 + 14 empty alignments (FB3 … FDA, every third handle), each named after one curve method: Fixed Curve (Three point), the seven More Fixed Curves, Floating Curve (From entity, radius, through point), the three More Floating Curves, Free Curve Fillet (radius) and (through point). Copied from S113's `EMPTY` (`create_alignment … "" --template 10D0`). For the owner to lay out one curve per alignment. | 457673 | `6fcf4b6a7ecd5e55` |
| S117 | `S117_S116_curve_methods.dwg` | Civil 3D 2024 | S116 with one curve laid out per method by the owner (inputs in each alignment's description); saved. FC5's description says through point 0,0 but Civil 3D stored and drew (200, 0). | 481739 | `ca7c82dfb85ce123` |
| S118 | `S118_S116_ccw_curves.dwg` | Civil 3D 2024 | From S116 (owner): `CCW` (1057: fixed line, then Floating Curve from entity end, radius 100, length 100, counter-clockwise) and `Center and Radius` (1061: two Fixed Curves by centre and radius joined by a Free Line, then a Floating Line through a point); the other alignments removed; saved. | 479879 | `6f82511053d98996` |
| S119 | `S119_S118_first_circle_edited.dwg` | Civil 3D 2024 | S118's `Center and Radius` with the first circle edited (owner): a line from (438, 2124) with a spiral into the first curve was added before it (element 7); `CCW` removed; saved. | 481868 | `666de70f086dc4e9` |
| S120 | `S120_S24_spiral_method_alignments.dwg` | civil3drust | S24 + 21 empty alignments (FB3 … FEF, every third handle), each named after one spiral layout method from the owner's menu screenshots: the 2 Floating Line with Spiral, the 10 curve-with-spiral methods and the 9 spiral methods, in menu order. Copied from S113's `EMPTY` (`create_alignment … "" --template 10D0`). For the owner to lay out one element per alignment. | 461383 | `75d8e4f082704fd5` |
| S121 | `S121_S120_fixed_spiral.dwg` | Civil 3D 2024 | S120 with `Fixed Spiral` (FD7) laid out by the owner: line 0,0 → 100,0, then a fixed spiral (description: inward, clockwise, R=100, L=200; Civil 3D drew R = 200, L = 200); the other alignments still empty. | 477639 | `907893c1746bf4a4` |
| S122 | `S122_S24_pi_display_1_2_3.dwg` | Civil 3D 2024 | MIST6 (owner): S24 with three identical alignments, `Alignment - (1)` (F9E), `(2)` (1008), `(3)` (100A), differing only in Alignment Properties → Point of Intersection: by change in alignment direction, by individual curves and curve group, do not display. S121 is reserved for the filled copy of S120. | 473452 | `fae3cf4fcf76e16f` |
| S123 | `S123_c3d2027_feet_tangent.dwg` | Civil 3D 2027 | MIST7 (owner): new drawing, alignment `TEST` (8F01), one tangent (0,0) → (100,0). INSUNITS 2 (feet), MEASUREMENT 0. Sent as the metric file; its header says feet. | 996486 | `00fafe99a7034f42` |
| S124 | `S124_c3d2027_meters_tangent.dwg` | Civil 3D 2027 | MIST7 (owner): new drawing, alignment `test` (71B4), one tangent (0,0) → (100,0). INSUNITS 6 (meters), MEASUREMENT 1. Sent as the imperial file; its header says meters. | 984766 | `2a1df2654f26278a` |
| S125 | `S125_S24_header_control.dwg` | civil3drust | MIST7 control: S24 read and written by civil3drust (acadrust 0.6.7) with no change (`exp_header … keep`). Its AECC records are identical to S24. | 448774 | `ec4cf24112440968` |
| S126 | `S126_S24_header_04010008.dwg` | civil3drust | MIST7: S24 with the header of all four elements of F9E set to `0x04010008` (middle byte 2 → 1; the value seen in label styles). | 448806 | `209370d60783dd07` |
| S127 | `S127_S24_header_04030008.dwg` | civil3drust | MIST7: as S126 with `0x04030008` (middle byte 2 → 3; not seen anywhere). | 448774 | `2507e627f48875b4` |
| S128 | `S128_S24_header_05020008.dwg` | civil3drust | MIST7: as S126 with `0x05020008` (top byte 4 → 5; the value seen in surface and table styles). | 448774 | `aa1234cdd97f7bfa` |
| S129 | `S129_S24_header_04020009.dwg` | civil3drust | MIST7: as S126 with `0x04020009` (low byte 8 → 9). | 448774 | `80bac881a1a75dfd` |
| S130 | `S130_S24_header_08000204.dwg` | civil3drust | MIST7: as S126 with `0x08000204` (bytes reversed: the EOT STX NUL BS reading). | 448774 | `00d54e1ad2442be2` |
| S131 | `S131_S125_civ3d_resaved.dwg` | Civil 3D 2024 | S125 (control) opened without RECOVER and saved (owner). First drawing written with acadrust 0.6.7 to be opened in Civil 3D. Header kept `0x04020008`. | 474663 | `0707d6a9c29e2560` |
| S132 | `S132_S127_civ3d_resaved.dwg` | Civil 3D 2024 | S127 (`0x04030008`) opened without RECOVER and saved (owner). Civil 3D wrote `0x04030008` back; F9E differs from S131 only in those four header bits. | 477063 | `3272d36a844d93de` |
| S133 | `S133_S24_header_04020007.dwg` | civil3drust | MIST7 round 2: S24 with the header of all four elements of F9E set to `0x04020007` (low word 8 → 7). | 448774 | `fd4bbcdcffc6ae1a` |
| S134 | `S134_S24_header_04020000.dwg` | civil3drust | MIST7 round 2: as S133 with `0x04020000` (low word 0). | 448774 | `b6fcf04ea594b858` |
| S135 | `S135_S24_header_04020108.dwg` | civil3drust | MIST7 round 2: as S133 with `0x04020108` (second byte 0 → 1: one 16-bit field or two bytes?). | 448806 | `9eaec589a9750cdb` |
| S136 | `S136_S24_header_04000008.dwg` | civil3drust | MIST7 round 2: as S133 with `0x04000008` (third byte 0: is bit 0x02 required?). | 448774 | `6175bc117d687158` |
| S137 | `S137_S24_header_04060008.dwg` | civil3drust | MIST7 round 2: as S133 with `0x04060008` (third byte 6: 0x02 plus 0x04). | 448774 | `c2b8d90283d841f3` |
| S138 | `S138_S24_header_03020008.dwg` | civil3drust | MIST7 round 2: as S133 with `0x03020008` (top byte 4 → 3). | 448774 | `19d03442bda156f8` |
| S139 | `S139_S137_civ3d_resaved.dwg` | Civil 3D 2024 | S137 (`0x04060008`) opened without errors, AUDIT clean, saved (owner). Civil 3D wrote `0x04020008` back: F9E is identical to S131. | 476514 | `d5220bbdec203f8a` |

Comparison pairs used so far: S2→S4 (Civil 3D re-save of a patched file),
S5→S6 (segment 2 +10 ft), S7 vs S6 (generated vs Civil 3D), S7→S8
(Civil 3D re-save), S8→S9 (start station). S10 is a single-arc reference. S5→S11 (free curve added at PI 1), S11→S12 (radius 50→75), S13 vs S12 (generated vs Civil 3D), S13→S14 (Civil 3D re-save), S14 vs S12, S11→S15 (tangent 3 +100 ft past a fillet), S16 vs S15 (generated vs Civil 3D), S18 vs S19 (alignment style), S20 vs S19 (generated vs Civil 3D), S21 → S22 (style line component on), S23 vs S22 (generated vs Civil 3D), S24 → S25 (alignment created), S36–S38 → S39–S41 (re-saves), S41 → S42 (descriptions) vs S43 (generated), S44 FB8 vs FF3 (one vs two tangents), S45 vs S44 (generated vs Civil 3D), S48 → S49 (curves added), S46/S50 vs S49 (generated vs Civil 3D), S26/S27 (alignment generated with/without label group), S26→S28 and S27→S29 (Civil 3D re-saves), S58 vs S59 (alignment type), S60→S62 and S61→S63 (generated types), S64→S65 (Civil 3D infrastructure added on save), S67→S68/S69 (bare alignment with/without AUDIT). S15's LandXML is Civil 3D's own export of the same geometry.

S1, S5–S19 come from a template whose `PRESSUREPARTERRORBLOCK` holds a
`3DSOLID` acadrust 0.6.5 cannot parse (S7–S9, S13, S14, S16 and S17 no longer have it).
