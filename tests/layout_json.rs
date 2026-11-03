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
fn json_group_reads_in_place_of_the_gap() {
    // S216: four alignments hold only the fixed elements and the JSON
    // group (D-059); the reader keeps the group as unsolved.
    use civil3drust::aecc_elements::decode_elements;
    let d = civil3drust::raw::open("samples/S216_S178_group_from_parameters.dwg").unwrap();
    let mut n = 0;
    for r in civil3drust::raw::records(&d.doc, "AECC_ALIGNMENT") {
        let de = decode_elements(&r).unwrap();
        if [0x1208, 0x120B, 0x120E, 0x1211].contains(&r.handle) {
            assert_eq!(de.elements.len(), 1, "{:X}", r.handle);
            assert_eq!(de.unsolved.len(), 1, "{:X}", r.handle);
            let g = &de.unsolved[0];
            assert_eq!((g.id, g.previous, g.next), (3, 1, 2), "{:X}", r.handle);
            assert_eq!(g.following.len(), 1, "{:X}", r.handle);
            assert!(!de.is_complete());
            n += 1;
        } else {
            assert!(de.is_complete(), "{:X}", r.handle);
        }
        if r.handle == 0x120E {
            let p: Vec<f64> = de.unsolved[0].params.iter().map(|x| x.1).collect();
            assert_eq!(p, [150.0, 170.0, 500.0, 190.0]);
        }
    }
    assert_eq!(n, 4);
    // Civil 3D's re-save S217 has every group solved.
    let d = civil3drust::raw::open("samples/S217_S216_civ3d_resaved.dwg").unwrap();
    for r in civil3drust::raw::records(&d.doc, "AECC_ALIGNMENT") {
        assert!(decode_elements(&r).unwrap().is_complete(), "{:X}", r.handle);
    }
}
