//! Element edits made in Civil 3D (O6: S92 → S93 → S94 …).

use civil3drust::aecc_alignment::Linked;
use civil3drust::aecc_design_speed::design_speeds;
use civil3drust::aecc_elements::{decode_elements, ArcDef, Element, ARC_FLOATING_START, ARC_FREE};
use civil3drust::alignment::Point2;
use civil3drust::raw;

fn edit_base(path: &str) -> raw::RawRecord {
    let o = raw::open(path).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xFB3)
        .unwrap()
}

#[test]
fn s94_first_tangent_deleted() {
    let r = edit_base("samples/S94_S93_first_tangent_deleted.dwg");
    let d = decode_elements(&r).unwrap();
    // IDs kept, ID 1 gone; chain A4 T2 A5 T3.
    let ids: Vec<i32> = d
        .elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => t.id,
            Element::Arc(a) => a.id,
        })
        .collect();
    assert_eq!(ids, [4, 2, 5, 3]);
    assert!((d.length() - 611.3274).abs() < 1e-3);
    // Stationing kept: the start becomes the old PC station.
    assert!((d.stored_start_station.unwrap() - 220.0).abs() < 1e-9);

    let Element::Arc(a4) = &d.elements[0].record else {
        panic!("first element is not an arc")
    };
    assert_eq!((a4.before, a4.after), (0, 2));
    assert_eq!(a4.params.n2, ARC_FLOATING_START);
    assert_eq!(a4.params.p1, 60.0); // design speed at the curve
    let ArcDef::Floating { radius, start, .. } = a4.def else {
        panic!("A4 is not floating")
    };
    assert_eq!(radius, 80.0);
    assert!(start.distance_to(Point2::new(220.0, 0.0)) < 1e-9);
    let end = d.elements[0].segment.end();
    assert!(end.distance_to(Point2::new(300.0, -80.0)) < 1e-9);

    let Element::Arc(a5) = &d.elements[2].record else {
        panic!("third element is not an arc")
    };
    assert_eq!(a5.params.n2, ARC_FREE);
    assert_eq!(a5.params.p1, 60.0);
    assert!(matches!(a5.def, ArcDef::Free { .. }));

    // Civil 3D leaves the speed at 0+00, before the new start (2+20).
    let v: Vec<(f64, f64)> = design_speeds(&r)
        .unwrap()
        .iter()
        .map(|s| (s.station, s.speed))
        .collect();
    assert_eq!(v, [(0.0, 60.0)]);
}

#[test]
fn p1_is_the_design_speed_at_the_curve_s89() {
    let o = raw::open("samples/S89_O5_layout_tangents_curves.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT").remove(0);
    let d = decode_elements(&r).unwrap();
    let p1: Vec<(i32, f64)> = d
        .elements
        .iter()
        .filter_map(|e| match &e.record {
            Element::Arc(a) => Some((a.id, a.params.p1)),
            _ => None,
        })
        .collect();
    // 60 mph from 0+00, 30 mph from 2+85.63 (on curve 2, before curve 5).
    assert_eq!(p1, [(2, 60.0), (5, 30.0)]);
}

fn chain(path: &str) -> Vec<(i32, i32, i32)> {
    let d = decode_elements(&edit_base(path)).unwrap();
    d.elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => (t.id, t.before, t.after),
            Element::Arc(a) => (a.id, a.before, a.after),
        })
        .collect()
}

#[test]
fn s95_curve_between_tangents_deleted() {
    assert_eq!(
        chain("samples/S95_S94_second_curve_deleted.dwg"),
        [(4, 0, 2), (2, 4, 3), (3, 2, 0)]
    );
    let d = decode_elements(&edit_base("samples/S95_S94_second_curve_deleted.dwg")).unwrap();
    assert!((d.length() - 645.6637).abs() < 1e-3);
}

#[test]
fn s96_curve_added_back_gets_id_5_and_is_stored_last() {
    let r = edit_base("samples/S96_S95_curve_added.dwg");
    assert_eq!(
        chain("samples/S96_S95_curve_added.dwg"),
        [(4, 0, 2), (2, 4, 5), (5, 2, 3), (3, 5, 0)]
    );
    let stored: Vec<i32> = civil3drust::aecc_elements::element_records(&r.data)
        .iter()
        .map(|e| match e {
            Element::Tangent(t) => t.id,
            Element::Arc(a) => a.id,
        })
        .collect();
    assert_eq!(stored, [2, 3, 4, 5]);
    let d = decode_elements(&r).unwrap();
    let Element::Arc(a5) = &d.elements[2].record else {
        panic!()
    };
    assert!(!a5.flag);
    assert_eq!(a5.params.p1, -1.0);
    assert!((d.length() - 611.3274).abs() < 1e-3);
}

// ---- civil3drust's own element edits, replayed against S94–S96 ----

use civil3drust::aecc_element_edit::{add_free_curve, remove_element};
use civil3drust::aecc_station::first_element_fields;

fn edited(rec: &raw::RawRecord, out: (Vec<u8>, usize)) -> raw::RawRecord {
    raw::RawRecord {
        data: out.0,
        handle_bits: out.1,
        ..rec.clone()
    }
}

/// The element list as stored: count + prefixed records, as bits.
fn element_list_bits(r: &raw::RawRecord) -> Vec<u8> {
    let d = decode_elements(r).unwrap();
    let first = d
        .elements
        .iter()
        .map(|e| e.record.header_bit())
        .min()
        .unwrap();
    let end = d
        .elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => t.end_bit(),
            Element::Arc(a) => a.end_bit,
        })
        .max()
        .unwrap();
    (first - 21..end)
        .map(|b| raw::get_bit(&r.data, b))
        .collect()
}

#[test]
fn remove_first_tangent_like_s94() {
    let s93 = edit_base("samples/S93_S92_civ3d_resaved.dwg");
    let (d, hb, rep) = remove_element(&s93, 1, 7).unwrap();
    let r = edited(&s93, (d, hb));
    assert_eq!(rep.floating, [4]);
    assert_eq!(rep.speeds, [(220.0, 60.0)]); // moved onto the new start
    assert_eq!(
        chain_of(&r),
        chain("samples/S94_S93_first_tangent_deleted.dwg")
    );
    let dec = decode_elements(&r).unwrap();
    assert!((dec.stored_start_station.unwrap() - 220.0).abs() < 1e-9);
    assert!((dec.length() - 611.3274).abs() < 1e-3);
    assert_eq!(first_element_fields(&r).unwrap().2, 4);
    let v: Vec<(f64, f64)> = design_speeds(&r)
        .unwrap()
        .iter()
        .map(|s| (s.station, s.speed))
        .collect();
    assert_eq!(v, [(220.0, 60.0)]);
}

#[test]
fn remove_curve_between_tangents_matches_s95_bit_for_bit() {
    let s94 = edit_base("samples/S94_S93_first_tangent_deleted.dwg");
    let s95 = edit_base("samples/S95_S94_second_curve_deleted.dwg");
    let (d, hb, _) = remove_element(&s94, 5, 7).unwrap();
    let r = edited(&s94, (d, hb));
    assert_eq!(element_list_bits(&r), element_list_bits(&s95));
}

#[test]
fn add_free_curve_matches_s96_bit_for_bit() {
    let s95 = edit_base("samples/S95_S94_second_curve_deleted.dwg");
    let s96 = edit_base("samples/S96_S95_curve_added.dwg");
    let (d, hb, rep) = add_free_curve(&s95, 2, 80.0, 7).unwrap();
    assert_eq!(rep.added_id, Some(5));
    let r = edited(&s95, (d, hb));
    assert_eq!(element_list_bits(&r), element_list_bits(&s96));
    assert!((decode_elements(&r).unwrap().length() - 611.3274).abs() < 1e-3);
}

#[test]
fn refuses_edits_without_a_known_form() {
    let s93 = edit_base("samples/S93_S92_civ3d_resaved.dwg");
    // Interior tangent after a floating curve: no sample of that form.
    let s95 = edit_base("samples/S95_S94_second_curve_deleted.dwg");
    assert!(remove_element(&s95, 2, 7).is_err());
    assert!(remove_element(&s93, 9, 7).is_err()); // no such element
    assert!(add_free_curve(&s93, 4, 50.0, 7).is_err()); // not a tangent
    assert!(add_free_curve(&s93, 1, 500.0, 7).is_err()); // doesn't fit
}

#[test]
fn first_element_fields_name_the_chain_head() {
    for p in [
        "samples/S24_ZERO_CIVIL.dwg",
        "samples/S82_O4_start_station.dwg",
        "samples/S89_O5_layout_tangents_curves.dwg",
        "samples/S93_S92_civ3d_resaved.dwg",
        "samples/S94_S93_first_tangent_deleted.dwg",
        "samples/S96_S95_curve_added.dwg",
    ] {
        let o = raw::open(p).unwrap();
        for r in raw::records(&o.doc, "AECC_ALIGNMENT") {
            let head = decode_elements(&r).unwrap().elements[0].record.id();
            assert_eq!(
                first_element_fields(&r).map(|f| f.2),
                Some(head),
                "{p} {:X}",
                r.handle
            );
        }
    }
}

fn chain_of(r: &raw::RawRecord) -> Vec<(i32, i32, i32)> {
    decode_elements(r)
        .unwrap()
        .elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => (t.id, t.before, t.after),
            Element::Arc(a) => (a.id, a.before, a.after),
        })
        .collect()
}

#[test]
fn element_id_list_is_dropped_s51() {
    use civil3drust::aecc_build::{alignment_type, clear_element_id_list};
    let o = raw::open("samples/S51_S50_civ3d_resaved.dwg").unwrap();
    let mut n = 0;
    for r in raw::records(&o.doc, "AECC_ALIGNMENT") {
        let Some((d, hb)) = clear_element_id_list(&r).unwrap() else {
            continue;
        };
        let c = edited(&r, (d, hb));
        assert_eq!(alignment_type(&c).unwrap(), alignment_type(&r).unwrap());
        assert!(clear_element_id_list(&c).unwrap().is_none());
        assert_eq!(chain_of(&c), chain_of(&r));
        n += 1;
    }
    assert!(n > 0, "S51 has an element-ID list");
}

fn record(path: &str, handle: u64) -> raw::RawRecord {
    let o = raw::open(path).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == handle)
        .unwrap()
}

#[test]
fn s99_last_tangent_after_free_curve_deleted() {
    use civil3drust::aecc_elements::ARC_FLOATING_END;
    let r = record("samples/S99_S98_last_tangent_deleted.dwg", 0xFB3);
    let d = decode_elements(&r).unwrap();
    assert!(d.gaps.is_empty());
    assert_eq!(chain_of(&r), [(4, 0, 2), (2, 4, 5), (5, 2, 0)]);
    let Element::Arc(a5) = &d.elements[2].record else {
        panic!()
    };
    assert_eq!(a5.params.n2, ARC_FLOATING_END);
    let ArcDef::FloatingEnd { end, .. } = a5.def else {
        panic!()
    };
    assert!(end.distance_to(Point2::new(380.0, -300.0)) < 1e-9);
    assert!(
        d.elements[2]
            .segment
            .start()
            .distance_to(Point2::new(300.0, -220.0))
            < 1e-9
    );
    assert!((d.length() - 391.3274).abs() < 1e-3);

    // civil3drust's own edit gives the same element list.
    let s98 = record("samples/S98_S97_civ3d_audit_resaved.dwg", 0xFB3);
    let (b, hb, rep) = remove_element(&s98, 3, 7).unwrap();
    assert_eq!(rep.floating, [5]);
    assert_eq!(
        element_list_bits(&edited(&s98, (b, hb))),
        element_list_bits(&r)
    );
}

#[test]
fn s100_interior_tangent_deleted_leaves_a_gap() {
    let r = record("samples/S100_S98_interior_tangent_deleted.dwg", 0xFB9);
    let d = decode_elements(&r).unwrap();
    assert_eq!(chain_of(&r), [(1, 0, 4), (4, 1, 0), (5, 0, 3), (3, 5, 0)]);
    assert_eq!(d.gaps, [2]);
    assert!(
        d.elements[1]
            .segment
            .end()
            .distance_to(Point2::new(300.0, -880.0))
            < 1e-9
    );
    assert!(
        d.elements[2]
            .segment
            .start()
            .distance_to(Point2::new(300.0, -1020.0))
            < 1e-9
    );
    assert!((d.length() - 691.3274).abs() < 1e-3);

    let s98 = record("samples/S98_S97_civ3d_audit_resaved.dwg", 0xFB9);
    let (b, hb, rep) = remove_element(&s98, 2, 7).unwrap();
    assert_eq!(rep.floating, [4, 5]);
    let ours = edited(&s98, (b, hb));
    assert_eq!(element_list_bits(&ours), element_list_bits(&r));
    // Editing a gapped alignment is refused for now.
    assert!(remove_element(&ours, 1, 7).is_err());
}
