//! The JSON layout string (`aecc_layout`): Civil 3D's strings re-serialize
//! byte for byte, the group start point (MIST17) is the first spiral's
//! start, and the generated S197/S198 hold what `compose` builds from
//! their elements.

use civil3drust::aecc_elements::{element_records, Element};
use civil3drust::aecc_layout::{compose, Json};
use civil3drust::{raw, string_stream};

fn layouts(file: &str) -> Vec<(u64, String, Vec<Element>)> {
    let d = raw::open(format!("samples/{file}")).unwrap();
    raw::records(&d.doc, "AECC_ALIGNMENT")
        .into_iter()
        .filter_map(|r| {
            let j = string_stream::layout_json(&r.data, r.handle_bits)?;
            Some((r.handle, j, element_records(&r.data)))
        })
        .collect()
}

#[test]
fn civil3d_json_reserializes_exactly() {
    let mut n = 0;
    for f in [
        "S177_long_spiral_method_alignments.dwg",
        "S178_S177_long_reverse_spirals.dwg",
        "S181_S179_civ3d_resaved.dwg",
    ] {
        for (h, j, els) in layouts(f) {
            assert_eq!(Json::parse(&j).unwrap().to_civil3d(), j, "{f} {h:X}");
            // MIST17: the member spiral stores the group's start point.
            let m = els
                .iter()
                .find_map(|e| match e {
                    Element::Spiral(s) => s.member.map(|p| (p, s.start)),
                    _ => None,
                })
                .unwrap();
            assert_eq!(m.0, m.1, "{f} {h:X}");
            n += 1;
        }
    }
    assert_eq!(n, 9);
}

#[test]
fn composed_json_matches_civil3d_structure() {
    // Same objects, IDs, links, types and parameter names as Civil 3D's;
    // values differ only where Civil 3D keeps the typed points.
    for (h, j, els) in layouts("S178_S177_long_reverse_spirals.dwg") {
        let ours = compose(&els, true).unwrap().to_civil3d();
        let strip = |s: &str| -> Vec<String> {
            s.lines()
                .map(|l| l.split(':').next().unwrap().to_string())
                .collect()
        };
        assert_eq!(strip(&ours), strip(&j), "{h:X}");
        for key in ["\"ConstraintType\"", "\"PreviousId\"", "\"NextId\""] {
            let pick = |s: &str| -> Vec<String> {
                s.lines()
                    .filter(|l| l.contains(key))
                    .map(str::to_string)
                    .collect()
            };
            assert_eq!(pick(&ours), pick(&j), "{h:X} {key}");
        }
    }
}

#[test]
fn generated_json_is_composed_from_the_elements() {
    let s197 = layouts("S197_S24_generated_layout_json.dwg");
    assert_eq!(s197.len(), 6);
    for (h, j, els) in s197 {
        assert_eq!(compose(&els, true).unwrap().to_civil3d(), j, "{h:X}");
    }
    // S198: the same with Spiral1Length + 20 in the JSON only.
    for (h, j, els) in layouts("S198_S24_generated_layout_json_tweaked.dwg") {
        let ours = compose(&els, true).unwrap();
        let theirs = Json::parse(&j).unwrap();
        let s1 = |v: &Json| -> f64 {
            let Some(Json::Arr(o)) = v.get("RGObjects") else {
                panic!()
            };
            o.iter()
                .find_map(
                    |x| match x.get("ConstraintParameters")?.get("Spiral1Length")? {
                        Json::Num(v) => Some(*v),
                        _ => None,
                    },
                )
                .unwrap()
        };
        assert!((s1(&theirs) - s1(&ours) - 20.0).abs() < 1e-9, "{h:X}");
    }
}

#[test]
fn layout_from_typed_parameters_matches_civil3d_group() {
    // S178 11F8: line 1, curve 2, Reverse SCSS group typed as 130, 550,
    // 160, 210. The group object written from those parameters is the
    // one Civil 3D wrote, line for line.
    use civil3drust::aecc_alignment::Linked;
    use civil3drust::aecc_layout::{layout, Group};
    let (_, j, els) = layouts("S178_S177_long_reverse_spirals.dwg")
        .into_iter()
        .find(|(h, _, _)| *h == 0x11F8)
        .unwrap();
    let fixed: Vec<Element> = els.into_iter().filter(|e| e.id() <= 2).collect();
    let ours = layout(
        &fixed,
        &[Group {
            method: "SCSS",
            previous: 1,
            next: 2,
            params: vec![130.0, 550.0, 160.0, 210.0],
        }],
        true,
    )
    .unwrap()
    .to_civil3d();
    let group = |s: &str| -> String {
        let i = s.find("\"RGObjectType\": 7").unwrap();
        s[i..].to_string()
    };
    assert_eq!(group(&ours), group(&j));
}

#[test]
fn json_group_is_solved_like_civil3d() {
    // S218: four alignments hold only the fixed elements and the JSON
    // group (D-059). civil3drust solves the groups (D-061); Civil 3D's own
    // solution of the same file (S217, from S216) agrees within 1e-6.
    use civil3drust::aecc_elements::decode_elements;
    let ours = civil3drust::raw::open("samples/S218_S178_group_parameters_written.dwg").unwrap();
    let c3d = civil3drust::raw::open("samples/S217_S216_civ3d_resaved.dwg").unwrap();
    let theirs = civil3drust::raw::records(&c3d.doc, "AECC_ALIGNMENT");
    let mut n = 0;
    for r in civil3drust::raw::records(&ours.doc, "AECC_ALIGNMENT") {
        let a = decode_elements(&r).unwrap();
        assert!(a.is_complete(), "{:X}", r.handle);
        let q = theirs.iter().find(|q| q.handle == r.handle).unwrap();
        let b = decode_elements(q).unwrap();
        assert_eq!(a.elements.len(), b.elements.len(), "{:X}", r.handle);
        assert!((a.length() - b.length()).abs() < 1e-6, "{:X}", r.handle);
        for (x, y) in a.elements.iter().zip(&b.elements) {
            assert!(x.segment.start().distance_to(y.segment.start()) < 1e-6);
            assert!(x.segment.end().distance_to(y.segment.end()) < 1e-6);
        }
        // All seven alignments have a JSON group; S178's own three store
        // pieces (set aside for the JSON solution), the four new ones not.
        assert_eq!(a.layout_groups.len(), 1);
        assert!(a.layout_groups[0].solved);
        if a.layout_groups[0].stored_pieces.is_empty() {
            n += 1;
        }
    }
    assert_eq!(n, 4);
}

#[test]
fn solved_json_group_is_written_back_as_json() {
    // Decode → write the records back: the solved pieces are dropped, the
    // record keeps the two fixed elements and the same JSON (D-061).
    use civil3drust::aecc_element_edit::replace_elements;
    use civil3drust::aecc_elements::decode_elements;
    let d = civil3drust::raw::open("samples/S218_S178_group_parameters_written.dwg").unwrap();
    let r = civil3drust::raw::records(&d.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x120E)
        .unwrap();
    let before = decode_elements(&r).unwrap();
    let els: Vec<Element> = before
        .all()
        .filter(|e| e.part <= 1)
        .map(|e| e.record.clone())
        .collect();
    let (b, hb, _) = replace_elements(&r, &els, 1).unwrap();
    let w = civil3drust::raw::RawRecord {
        data: b,
        handle_bits: hb,
        ..r.clone()
    };
    assert_eq!(element_records(&w.data).len(), 2);
    assert_eq!(
        string_stream::layout_json(&w.data, w.handle_bits),
        string_stream::layout_json(&r.data, r.handle_bits)
    );
    let after = decode_elements(&w).unwrap();
    assert!(after.is_complete());
    assert!((after.length() - before.length()).abs() < 1e-9);
}

#[test]
fn json_wins_over_stored_pieces_like_civil3d() {
    // S216 1214/1217 store pieces with Spiral1 as typed but the JSON says
    // + 20; Civil 3D re-solved from the JSON (S217). So does the reader by
    // default; the option reads the stored pieces.
    use civil3drust::aecc_elements::{decode_elements, decode_elements_with, DecodeOptions};
    let ours = civil3drust::raw::open("samples/S216_S178_group_from_parameters.dwg").unwrap();
    let c3d = civil3drust::raw::open("samples/S217_S216_civ3d_resaved.dwg").unwrap();
    let theirs = civil3drust::raw::records(&c3d.doc, "AECC_ALIGNMENT");
    for h in [0x1214u64, 0x1217] {
        let r = civil3drust::raw::records(&ours.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let a = decode_elements(&r).unwrap();
        let b = decode_elements(theirs.iter().find(|q| q.handle == h).unwrap()).unwrap();
        assert!((a.length() - b.length()).abs() < 1e-6, "{h:X}");
        assert!(!a.layout_groups[0].stored_pieces.is_empty());
        let stored = decode_elements_with(
            &r,
            DecodeOptions {
                stored_group_pieces: true,
            },
        )
        .unwrap();
        assert!((stored.length() - a.length()).abs() > 1.0, "{h:X}");
    }
}
