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
            _ => unreachable!(),
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
            _ => unreachable!(),
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
            _ => unreachable!(),
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
            _ => unreachable!(),
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
            _ => unreachable!(),
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
    assert!(d.detached.is_empty());
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
fn s100_interior_tangent_deleted_leaves_a_detached_piece() {
    let r = record("samples/S100_S98_interior_tangent_deleted.dwg", 0xFB9);
    let d = decode_elements(&r).unwrap();
    // Only the chain from the first element is the alignment (stations
    // 0+00 to 3+45.66, owner's screenshot); A5 T3 is a detached piece.
    assert_eq!(chain_of(&r), [(1, 0, 4), (4, 1, 0)]);
    assert_eq!(d.detached.len(), 1);
    let ids: Vec<i32> = d.detached[0].iter().map(|e| e.record.id()).collect();
    assert_eq!(ids, [5, 3]);
    let a4_end = d.elements[1].segment.end();
    assert!(a4_end.distance_to(Point2::new(300.0, -880.0)) < 1e-9);
    let a5_start = d.detached[0][0].segment.start();
    assert!(a5_start.distance_to(Point2::new(300.0, -1020.0)) < 1e-9);
    assert!((d.length() - 345.6637).abs() < 1e-3);

    let s98 = record("samples/S98_S97_civ3d_audit_resaved.dwg", 0xFB9);
    let (b, hb, rep) = remove_element(&s98, 2, 7).unwrap();
    assert_eq!(rep.floating, [4, 5]);
    let ours = edited(&s98, (b, hb));
    assert_eq!(element_list_bits(&ours), element_list_bits(&r));
    // Elements of the detached piece can't be edited; the alignment can,
    // and the detached piece is kept as it is.
    assert!(remove_element(&ours, 3, 7).is_err());
    // T1 can't go: A4 (13) would lose the tangent it starts from.
    assert!(remove_element(&ours, 1, 7).is_err());
    let (b2, hb2, rep2) = remove_element(&ours, 4, 7).unwrap();
    assert_eq!(rep2.elements, 1);
    assert!((rep2.length - 220.0).abs() < 1e-9);
    let after = decode_elements(&edited(&ours, (b2, hb2))).unwrap();
    assert_eq!(after.detached.len(), 1);
    let ids: Vec<i32> = after.detached[0].iter().map(|e| e.record.id()).collect();
    assert_eq!(ids, [5, 3]);
}

/// The interval blocks after the element records: (station, distance from
/// element start, length to element end, kind, sub, next ID, 6 points) per
/// interval, without the GUIDs.
fn intervals(r: &raw::RawRecord) -> Vec<(Vec<f64>, [i32; 3], Vec<u8>)> {
    let d = decode_elements(r).unwrap();
    let mut p = d.records_end_bit().unwrap();
    let mut out = Vec::new();
    for _ in 0..3 {
        p = raw::read_bl(&r.data, p).unwrap().1;
    }
    let (n, mut p2) = raw::read_bl(&r.data, p).unwrap();
    for _ in 0..n {
        let body_start = p2 + 128;
        let mut q = body_start + 13;
        let mut v = Vec::new();
        for _ in 0..3 {
            let (x, r2) = raw::read_bd(&r.data, q).unwrap();
            v.push(x);
            q = r2;
        }
        let mut k = [0; 3];
        for x in &mut k {
            let (y, r2) = raw::read_bl(&r.data, q).unwrap();
            *x = y;
            q = r2;
        }
        for i in 0..6 {
            v.push(raw::get_f64(&r.data, q + 64 * i));
        }
        q += 384;
        let bits = (body_start..q).map(|b| raw::get_bit(&r.data, b)).collect();
        out.push((v, k, bits));
        p2 = q;
    }
    out
}

/// S113: a design speed starting on a curve (1+18 on the curve 1+03.90 to
/// 1+33.55). The curve interval is `BL 2, BL 1` with the tangent's distance
/// rule; civil3drust writes the same interval bit for bit.
#[test]
fn speed_interval_on_a_curve_s113() {
    use civil3drust::aecc_design_speed::set_design_speeds;
    let r = record("samples/S113_line_methods.dwg", 0x10E4);
    let theirs = intervals(&r);
    // The three BLs are the element's ID and links: curve 2 (before 1,
    // after 3).
    let curve = theirs.iter().find(|i| i.1[0] == 2).unwrap();
    assert_eq!(curve.1, [2, 1, 3]);
    let v = &curve.0;
    assert_eq!(v[0], 118.0);
    assert!((v[1] - 14.102228).abs() < 1e-6);
    assert!(
        (v[1] + v[2] - 29.650071).abs() < 1e-6,
        "third = length to the end"
    );
    let (b, hb) = set_design_speeds(&r, &[(0.0, 60.0), (118.0, 30.0)], 5).unwrap();
    let ours = intervals(&edited(&r, (b, hb)));
    // Same fields; values equal up to Civil 3D's last-digit rounding.
    for kind in [1, 2] {
        let o = ours.iter().find(|i| i.1[0] == kind).unwrap();
        let t = theirs.iter().find(|i| i.1[0] == kind).unwrap();
        assert_eq!(o.1, t.1);
        for (a, b) in o.0.iter().zip(&t.0) {
            assert!(
                (a - b).abs() < 1e-9 * (1.0 + b.abs()),
                "kind {kind}: {a} vs {b}"
            );
        }
    }
}

/// S113: Civil 3D's line methods. `fixed line by length` is stored as a
/// plain two-point line; `floating line by length` is kind 5 (length);
/// `free line` is kind 9, tangent to the three-point curves on both sides,
/// which it trims (labels: PT 0+22.18, PC 3+10.06, EP 3+89.12).
#[test]
fn line_methods_s113() {
    use civil3drust::aecc_alignment::{KIND_FIXED, KIND_FLOATING_LENGTH, KIND_FREE};
    let kind_of = |h: u64, id: i32| {
        let d = decode_elements(&record("samples/S113_line_methods.dwg", h)).unwrap();
        let t = d
            .elements
            .iter()
            .find_map(|e| match &e.record {
                Element::Tangent(t) if t.id == id => Some(t.clone()),
                _ => None,
            })
            .unwrap();
        (t.kind, t.length, d)
    };
    let (k, _, d) = kind_of(0x10E4, 3);
    assert_eq!(k, KIND_FIXED);
    assert!((d.length() - 233.5478).abs() < 1e-4);
    let (k, l, d) = kind_of(0x10EA, 4);
    assert_eq!((k, l), (KIND_FLOATING_LENGTH, Some(100.0)));
    assert!((d.length() - 233.5478).abs() < 1e-4);
    let (k, _, d) = kind_of(0x10F4, 4);
    assert_eq!(k, KIND_FREE);
    let mut sta = 0.0;
    let stations: Vec<f64> = d
        .elements
        .iter()
        .map(|e| {
            sta += e.segment.length();
            sta
        })
        .collect();
    for (got, want) in stations.iter().zip([22.18, 310.06, 389.12]) {
        assert!((got - want).abs() < 0.005, "{got} vs {want}");
    }
}

/// S114 → S115: Civil 3D kept civil3drust's curve intervals but rewrote
/// the three link BLs to the containing element's ID, before and after.
/// civil3drust now writes them that way: same IDs and values as Civil 3D.
#[test]
fn interval_links_match_civil3d_s115() {
    use civil3drust::aecc_design_speed::set_design_speeds;
    for (h, speeds) in [
        (
            0x10F4,
            vec![(0.0, 60.0), (10.0, 50.0), (200.0, 40.0), (350.0, 30.0)],
        ),
        (0x10EA, vec![(0.0, 60.0), (120.0, 35.0)]),
    ] {
        let r = record("samples/S113_line_methods.dwg", h);
        let (b, hb) = set_design_speeds(&r, &speeds, 5).unwrap();
        let mut ours = intervals(&edited(&r, (b, hb)));
        let mut theirs = intervals(&record("samples/S115_S114_civ3d_resaved.dwg", h));
        let key = |x: &(Vec<f64>, [i32; 3], Vec<u8>)| (x.0[0] * 1000.0).round() as i64;
        ours.sort_by_key(key);
        theirs.sort_by_key(key);
        assert_eq!(ours.len(), theirs.len());
        for (o, t) in ours.iter().zip(&theirs) {
            assert_eq!(o.1, t.1, "{h:X} station {}", t.0[0]);
            for (a, b) in o.0.iter().zip(&t.0) {
                assert!((a - b).abs() < 1e-9 * (1.0 + b.abs()), "{h:X}: {a} vs {b}");
            }
        }
    }
}

// ---- "Lock all parameter constraints" (MIST7, S163 on / S164 off) ----

use civil3drust::aecc_alignment::read_element_header;
use civil3drust::aecc_criteria::constraint_editing;

fn lock_sample(path: &str) -> raw::RawRecord {
    let o = raw::open(path).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xF9E)
        .unwrap()
}

/// The header flag byte of every element (`None`: no byte).
fn header_flags(r: &raw::RawRecord) -> Vec<Option<u8>> {
    decode_elements(r)
        .unwrap()
        .all()
        .map(|e| {
            read_element_header(&r.data, e.record.header_bit())
                .unwrap()
                .0
        })
        .collect()
}

const S163: &str = "samples/S163_S24_tangency_swapping_on.dwg";
const S164: &str = "samples/S164_S24_lock_parameters_off.dwg";

#[test]
fn unlocked_element_headers_read_s164() {
    let (on, off) = (lock_sample(S163), lock_sample(S164));
    assert_eq!(header_flags(&on), [Some(2); 4]);
    assert_eq!(header_flags(&off), [None; 4]);
    let (a, b) = (
        decode_elements(&on).unwrap(),
        decode_elements(&off).unwrap(),
    );
    assert_eq!(chain_of(&on), chain_of(&off));
    assert_eq!(a.stored_start_station, Some(0.0));
    assert_eq!(b.stored_start_station, Some(0.0));
    assert!((a.length() - b.length()).abs() < 1e-12);
    // The alignment-wide option sits next to MIST15.
    let (c, d) = (
        constraint_editing(&on).unwrap(),
        constraint_editing(&off).unwrap(),
    );
    assert!(c.lock_parameters && c.swap_tangency);
    assert!(!d.lock_parameters && !d.swap_tangency);
}

#[test]
fn edits_keep_the_unlocked_form_s164() {
    for (path, want) in [(S163, Some(2)), (S164, None)] {
        let r = lock_sample(path);
        // Remove the arc (ID 4), then add a curve back on tangent 1.
        let (d, hb, _) = remove_element(&r, 4, 7).unwrap();
        let r1 = edited(&r, (d, hb));
        assert_eq!(header_flags(&r1), [want; 3], "{path} remove");
        let (d, hb, _) = add_free_curve(&r1, 1, 50.0, 7).unwrap();
        let r2 = edited(&r1, (d, hb));
        assert_eq!(header_flags(&r2), [want; 4], "{path} add");
        // Whole-geometry rewrite.
        let pis = [
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(100.0, 100.0),
        ];
        let (d, hb, _) = civil3drust::aecc_build::set_geometry(&r, &pis, &[Some(30.0)]).unwrap();
        let r3 = edited(&r, (d, hb));
        assert_eq!(header_flags(&r3), [want; 3], "{path} set_geometry");
        assert_eq!(
            constraint_editing(&r3).unwrap().lock_parameters,
            want.is_some()
        );
    }
    // The unlocked form is the locked one without the flag byte: the
    // same edit on S163 and S164 gives records 8 bits per element apart.
    let on = lock_sample(S163);
    let off = lock_sample(S164);
    let (d1, _, _) = remove_element(&on, 4, 7).unwrap();
    let (d2, _, _) = remove_element(&off, 4, 7).unwrap();
    let n = |d: &Vec<u8>| raw::bit_len(d);
    assert!(n(&d1) >= n(&d2) + 3 * 8);
}

#[test]
fn unlocked_edits_survive_civil3d_resave_s166_s169() {
    for (ours, resaved, n) in [
        (
            "samples/S166_S164_arc_removed_unlocked.dwg",
            "samples/S168_S166_civ3d_resaved.dwg",
            3,
        ),
        (
            "samples/S167_S166_curve_added_unlocked.dwg",
            "samples/S169_S167_civ3d_resaved.dwg",
            4,
        ),
    ] {
        let (a, b) = (lock_sample(ours), lock_sample(resaved));
        assert_eq!(header_flags(&a), vec![None; n]);
        assert_eq!(header_flags(&b), vec![None; n]);
        assert_eq!(chain_of(&a), chain_of(&b));
        let (da, db) = (decode_elements(&a).unwrap(), decode_elements(&b).unwrap());
        assert!((da.length() - db.length()).abs() < 1e-9);
        assert!(!constraint_editing(&b).unwrap().lock_parameters);
    }
}

/// Switching the options rewrites bits 16/15 and every header: S163 with
/// both off equals S164 from the criteria flags to the end of the element
/// list, and S164 with both on equals S163.
#[test]
fn set_constraint_editing_matches_civil3d_s163_s164() {
    use civil3drust::aecc_criteria::{set_constraint_editing, ConstraintEditing};
    let span = |r: &raw::RawRecord| -> Vec<u8> {
        let d = decode_elements(r).unwrap();
        let a = d.first_record_bit().unwrap() - 21 - 20;
        let z = d.records_end_bit().unwrap();
        (a..z).map(|i| raw::get_bit(&r.data, i)).collect()
    };
    let set = |r: &raw::RawRecord, on: bool| {
        let c = ConstraintEditing {
            swap_tangency: on,
            lock_parameters: on,
        };
        let (data, handle_bits) = set_constraint_editing(r, c).unwrap();
        let out = raw::RawRecord {
            data,
            handle_bits,
            ..r.clone()
        };
        assert_eq!(constraint_editing(&out).unwrap(), c);
        out
    };
    let (on, off) = (lock_sample(S163), lock_sample(S164));
    assert_eq!(span(&set(&on, false)), span(&off));
    assert_eq!(span(&set(&off, true)), span(&on));
}

// ---- Created unlocked (S170, re-saved S171) and one element locked (S172) ----

fn s17x(path: &str) -> raw::RawRecord {
    let o = raw::open(path).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xFB3)
        .unwrap()
}

#[test]
fn lock_is_per_element_s170_s172() {
    let ours = s17x("samples/S170_S24_created_unlocked.dwg");
    let resaved = s17x("samples/S171_S170_civ3d_resaved.dwg");
    let partial = s17x("samples/S172_S171_one_element_locked.dwg");
    assert_eq!(header_flags(&ours), [None; 4]);
    assert_eq!(header_flags(&resaved), [None; 4]);
    assert_eq!(chain_of(&ours), chain_of(&resaved));
    // Storage order 1, 4, 2, 3: only tangent 2 locked; "Lock all" stays off.
    assert_eq!(header_flags(&partial), [None, None, Some(2), None]);
    assert!(!constraint_editing(&partial).unwrap().lock_parameters);
    assert_eq!(chain_of(&ours), chain_of(&partial));
    // A new element follows "Lock all" (off); tangent 2 keeps its lock.
    let (d, hb, rep) = add_free_curve(&partial, 2, 30.0, 7).unwrap();
    let r = edited(&partial, (d, hb));
    let ids: Vec<i32> = decode_elements(&r)
        .unwrap()
        .all()
        .map(|e| e.record.id())
        .collect();
    let flags: Vec<(i32, Option<u8>)> = ids.into_iter().zip(header_flags(&r)).collect();
    let new = rep.added_id.unwrap();
    for (id, f) in flags {
        assert_eq!(f, (id == 2).then_some(2), "element {id} (new {new})");
    }
}

/// The group-member header (MIST7): S177's first-of-group spirals are
/// `02 0D`; with "Lock all parameter constraints" off Civil 3D writes
/// `02 09` (S202) and drops the `02` byte of the other elements.
/// `set_constraint_editing` on S177 gives S202's element records.
#[test]
fn group_member_header_unlocked_s177_s202() {
    use civil3drust::aecc_alignment::{read_header, TYPE_GROUP_MEMBER_UNLOCKED};
    use civil3drust::aecc_criteria::{set_constraint_editing, ConstraintEditing};
    let rec = |p: &str| {
        let o = raw::open(p).unwrap();
        raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == 0xFCE)
            .unwrap()
    };
    let span = |r: &raw::RawRecord| -> Vec<u8> {
        let d = decode_elements(r).unwrap();
        let (a, z) = (d.first_record_bit().unwrap(), d.records_end_bit().unwrap());
        (a..z).map(|i| raw::get_bit(&r.data, i)).collect()
    };
    let (s177, s202) = (
        rec("samples/S177_long_spiral_method_alignments.dwg"),
        rec("samples/S202_S177_lock_all_off.dwg"),
    );
    let hs: Vec<_> = {
        let d = decode_elements(&s202).unwrap();
        d.stored_records()
            .iter()
            .map(|r| read_header(&s202.data, r.header_bit()).unwrap())
            .collect()
    };
    let members: Vec<_> = hs.iter().filter(|h| h.is_group_member()).collect();
    assert_eq!(members.len(), 1);
    assert_eq!(
        (members[0].flags, members[0].type_byte),
        (Some(2), TYPE_GROUP_MEMBER_UNLOCKED)
    );
    assert!(hs.iter().all(|h| !h.locked()));
    assert!(!constraint_editing(&s202).unwrap().lock_parameters);
    let (a, b) = (
        decode_elements(&s177).unwrap(),
        decode_elements(&s202).unwrap(),
    );
    assert!((a.length() - b.length()).abs() < 1e-9);
    let off = ConstraintEditing {
        swap_tangency: constraint_editing(&s177).unwrap().swap_tangency,
        lock_parameters: false,
    };
    let (data, handle_bits) = set_constraint_editing(&s177, off).unwrap();
    let ours = raw::RawRecord {
        data,
        handle_bits,
        ..s177.clone()
    };
    assert_eq!(span(&ours), span(&s202));
    // The JSON layout follows: `ConstraintLocked` 0 everywhere, the
    // string identical to Civil 3D's.
    use civil3drust::string_stream::layout_json;
    let json = |r: &raw::RawRecord| layout_json(&r.data, r.handle_bits).unwrap();
    assert_eq!(json(&ours), json(&s202));
    // And back: locking again gives S177's string.
    let on = ConstraintEditing {
        lock_parameters: true,
        ..off
    };
    let (data, handle_bits) = set_constraint_editing(&ours, on).unwrap();
    let back = raw::RawRecord {
        data,
        handle_bits,
        ..s177.clone()
    };
    assert_eq!(json(&back), json(&s177));
    assert_eq!(span(&back), span(&s177));
}

/// Tangency constraint is not in the header (MIST7): S201 is S172 with
/// tangent 1 set to "Constrained by Next (Floating)". Its kind becomes 3
/// (start point only), the following free curve becomes "floating, start
/// point and radius", the headers stay as they were, the geometry too.
#[test]
fn constrained_by_next_is_the_kind_field_s201() {
    use civil3drust::aecc_alignment::KIND_FLOATING_NEXT;
    use civil3drust::aecc_elements::ARC_FLOATING_START;
    let (before, after) = (
        s17x("samples/S172_S171_one_element_locked.dwg"),
        s17x("samples/S201_S172_line1_constrained_by_next.dwg"),
    );
    assert_eq!(header_flags(&before), header_flags(&after));
    let d = decode_elements(&after).unwrap();
    let kinds: Vec<String> = d
        .elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => format!("L{}", t.kind),
            Element::Arc(a) => format!("A{}", a.params.n2),
            _ => "?".into(),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            format!("L{KIND_FLOATING_NEXT}"),
            format!("A{ARC_FLOATING_START}"),
            "L0".into(),
            "L0".into()
        ]
    );
    let d0 = decode_elements(&before).unwrap();
    assert!((d.length() - d0.length()).abs() < 1e-9);
    assert_eq!(chain_of(&before), chain_of(&after));
}

/// MIST8: the curve flag is set for curves Civil 3D read from the file;
/// curves made in the session that saves them keep 0 until the drawing is
/// opened again (S207–S210).
#[test]
fn curve_flag_set_on_load_s207_s210() {
    let flags = |p: &str| -> Vec<(i32, bool)> {
        let r = edit_base_f9e(p);
        decode_elements(&r)
            .unwrap()
            .elements
            .iter()
            .filter_map(|e| match &e.record {
                Element::Arc(a) => Some((a.id, a.flag)),
                _ => None,
            })
            .collect()
    };
    // S24 (0) opened, radius changed, saved: 1.
    assert_eq!(flags("samples/S24_ZERO_CIVIL.dwg"), [(4, false)]);
    assert_eq!(flags("samples/S207_S24_radius_75.dwg"), [(4, true)]);
    // S131 (1), radius changed: still 1.
    assert_eq!(flags("samples/S208_S131_radius_75.dwg"), [(4, true)]);
    // A curve added in the session: 0; after reopening and saving: 1.
    assert_eq!(
        flags("samples/S209_S131_curve_added.dwg"),
        [(4, true), (5, false)]
    );
    assert_eq!(
        flags("samples/S210_S209_reopened_resaved.dwg"),
        [(4, true), (5, true)]
    );
}

fn edit_base_f9e(path: &str) -> raw::RawRecord {
    let o = raw::open(path).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xF9E)
        .unwrap()
}

/// MIST9: with criteria-based design on, every curve stores the design
/// speed (p1 = 40), the criteria's minimum radius (p2 = 533, as the grid
/// shows) and a third criteria value (83, not identified yet); without
/// criteria all three are −1 (S219–S222).
#[test]
fn curve_criteria_values_s219_s222() {
    let params = |p: &str| -> Vec<(f64, f64, f64, f64)> {
        let r = edit_base_f9e(p);
        decode_elements(&r)
            .unwrap()
            .elements
            .iter()
            .filter_map(|e| match &e.record {
                Element::Arc(a) => Some((a.params.p1, a.params.p2, a.params.p3, {
                    let ArcDef::Free { radius, .. } = a.def else {
                        unreachable!()
                    };
                    radius
                })),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        params("samples/S24_ZERO_CIVIL.dwg"),
        [(-1.0, -1.0, -1.0, 50.0)]
    );
    for p in [
        "samples/S219_S24_criteria_on.dwg",
        "samples/S220_S219_speed_change_attempt.dwg",
    ] {
        assert_eq!(params(p), [(40.0, 533.0, 83.0, 50.0)], "{p}");
    }
    assert_eq!(
        params("samples/S221_S219_criteria_curve_added.dwg"),
        [(40.0, 533.0, 83.0, 50.0), (40.0, 533.0, 83.0, 533.0)]
    );
    let c4 = params("samples/S222_S221_radius_179.dwg");
    assert_eq!((c4[1].0, c4[1].1, c4[1].2), (40.0, 533.0, 83.0));
    assert!((c4[1].3 - 179.0367).abs() < 1e-3);
}

/// `set_length` / `set_pis` move the tangents and let free arcs follow.
/// Any other element keeps its stored geometry, so the edit is refused
/// instead of writing a chain with a gap: a fixed spiral after the tangent
/// (S162 FD7), a line constrained by the next element before an open curve
/// (S242 FEA), a floating curve (S117 FCB), a spiral group held by the
/// JSON layout (S223), offset alignments (S260 dynamic, S265 static).
#[test]
fn pi_edits_refuse_alignments_with_other_elements() {
    use civil3drust::aecc_alignment_edit::{set_pis, set_tangent_length, EditError};
    for (path, handle) in [
        ("samples/S162_S120_free_spirals.dwg", 0xFD7),
        ("samples/S242_S24_open_centre_curves.dwg", 0xFEA),
        ("samples/S117_S116_curve_methods.dwg", 0xFCB),
        // A group at the end held only by the JSON layout (unsolved).
        ("samples/S223_S178_group_at_alignment_end.dwg", 0x1208),
        // Offsets of a tangent: dynamic and static.
        ("samples/S260_S259_offset_left_10.dwg", 0x8D71),
        ("samples/S265_S264_offset_static.dwg", 0x8D71),
    ] {
        let o = raw::open(path).unwrap();
        let rec = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == handle)
            .unwrap();
        let d = decode_elements(&rec).unwrap();
        let mut data = rec.data.clone();
        let r = set_tangent_length(&mut data, &d, 0, 137.25);
        assert!(
            matches!(r, Err(EditError::NotPiLayout(_))),
            "{path} {handle:X}: {r:?}"
        );
        let pis = [Point2::new(0.0, 0.0), Point2::new(50.0, 50.0)];
        let r = set_pis(&mut data, &d, &pis);
        assert!(
            matches!(r, Err(EditError::NotPiLayout(_))),
            "{path} {handle:X}: {r:?}"
        );
        // Nothing was written.
        assert_eq!(data, rec.data);
    }
}
