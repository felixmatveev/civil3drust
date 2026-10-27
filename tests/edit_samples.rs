//! Element edits made in Civil 3D (O6: S92 → S93 → S94 …).

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
