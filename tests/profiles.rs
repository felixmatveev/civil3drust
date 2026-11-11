//! Profiles and profile views (S298–S305): element records and links.

use civil3drust::aecc_profile::{parent_alignment, pointers_to, profile_elements, ProfileElement};
use civil3drust::{raw, Point2};

fn open(id: &str) -> raw::Opened {
    let path = std::fs::read_dir("samples")
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(&format!("{id}_"))
        })
        .unwrap();
    raw::open(path.to_str().unwrap()).unwrap()
}

fn profile(o: &raw::Opened, h: u64) -> raw::RawRecord {
    raw::records(&o.doc, "AECC_PROFILE")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap()
}

/// (id, before, after, start, end) of each tangent; ids of curves.
fn tangents(els: &[ProfileElement]) -> Vec<(i32, i32, i32, Point2, Point2)> {
    els.iter()
        .filter_map(|e| match e {
            ProfileElement::Tangent(t) => Some((t.id, t.before, t.after, t.start, t.end)),
            _ => None,
        })
        .collect()
}

fn p(x: f64, y: f64) -> Point2 {
    Point2::new(x, y)
}

fn near(a: Point2, b: Point2) -> bool {
    (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4
}

#[test]
fn layout_profile_tangents_and_curve() {
    // S299: two tangents through (−100, −20), (871.2385, 10), (1900, −10).
    let o = open("S299");
    let els = profile_elements(&profile(&o, 0x8D72).data);
    let t = tangents(&els);
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].0, t[0].1, t[0].2), (1, 0, 2));
    assert!(near(t[0].3, p(-100.0, -20.0)) && near(t[0].4, p(871.2385, 10.0)));
    assert_eq!((t[1].0, t[1].1, t[1].2), (2, 1, 0));
    assert!(near(t[1].3, p(871.2385, 10.0)) && near(t[1].4, p(1900.0, -10.0)));

    // S300: a vertical curve 3 between them.
    let o = open("S300");
    let els = profile_elements(&profile(&o, 0x8D72).data);
    let t = tangents(&els);
    assert_eq!((t[0].2, t[1].1), (3, 3));
    let c = els
        .iter()
        .find_map(|e| match e {
            ProfileElement::Curve(c) => Some(c),
            _ => None,
        })
        .unwrap();
    assert_eq!((c.id, c.before, c.after), (3, 1, 2));
    assert_eq!(c.lead, 0.0);
    assert!((c.length - 488.7677).abs() < 1e-4);
    assert_eq!(c.tail, [-1.0; 3]);
}

#[test]
fn pvi_edit_and_alignment_edit() {
    // S301: the middle PVI's elevation 10 → 0 (both tangents).
    let o = open("S301");
    let t = tangents(&profile_elements(&profile(&o, 0x8D72).data));
    assert!(near(t[0].4, p(871.2385, 0.0)) && near(t[1].3, p(871.2385, 0.0)));
    // S302: the alignment's first tangent 49.2276 shorter: the PVI moves
    // back by as much; the outer points stay.
    let o = open("S302");
    let t = tangents(&profile_elements(&profile(&o, 0x8D72).data));
    assert!(near(t[0].3, p(-100.0, -20.0)) && near(t[1].4, p(1900.0, -10.0)));
    assert!(near(t[0].4, p(871.2385 - (688.2720 - 639.0444), 0.0)));
}

#[test]
fn links_between_alignment_profiles_and_views() {
    let o = open("S304");
    let al = raw::records(&o.doc, "AECC_ALIGNMENT").remove(0);
    assert_eq!(al.handle, 0x8C4A);
    // The alignment: soft pointers to its profiles, then its views.
    assert_eq!(
        pointers_to(&o.doc, &al, "AECC_PROFILE", Some(4)),
        [0x8D72, 0x8D91]
    );
    assert_eq!(
        pointers_to(&o.doc, &al, "AECC_PROFILE_VIEW", Some(4)),
        [0x8D70, 0x8DA9]
    );
    for h in [0x8D72, 0x8D91] {
        assert_eq!(parent_alignment(&o.doc, &profile(&o, h)), Some(0x8C4A));
    }
    for v in raw::records(&o.doc, "AECC_PROFILE_VIEW") {
        assert_eq!(parent_alignment(&o.doc, &v), Some(0x8C4A));
        assert_eq!(
            pointers_to(&o.doc, &v, "AECC_PROFILE", Some(4)),
            [0x8D72, 0x8D91]
        );
    }
    // S305: view 8D70 deleted; the profiles stay, the alignment keeps
    // only the remaining view.
    let o = open("S305");
    let al = raw::records(&o.doc, "AECC_ALIGNMENT").remove(0);
    assert_eq!(
        pointers_to(&o.doc, &al, "AECC_PROFILE", Some(4)),
        [0x8D72, 0x8D91]
    );
    assert_eq!(
        pointers_to(&o.doc, &al, "AECC_PROFILE_VIEW", Some(4)),
        [0x8DA9]
    );
}

#[test]
fn curve_length_typed() {
    // S306: S305 with the vertical curve's length set to 150 (owner).
    let o = open("S306");
    let els = profile_elements(&profile(&o, 0x8D72).data);
    let c = els
        .iter()
        .find_map(|e| match e {
            ProfileElement::Curve(c) => Some(c),
            _ => None,
        })
        .unwrap();
    assert_eq!(c.length, 150.0);
    assert_eq!(tangents(&els).len(), 2);
}

#[test]
fn layout_matches_civil3d_api() {
    // C3DDUMP on S306 (owner; samples/S306_profile_curve_length_150.api.txt).
    use civil3drust::aecc_profile::layout;
    let o = open("S306");
    let l = layout(&profile_elements(&profile(&o, 0x8D72).data)).unwrap();
    let want = [(-100.0, -20.0), (822.01097503767369, 0.0), (1900.0, -10.0)];
    assert_eq!(l.pvis.len(), 3);
    for (p, (x, y)) in l.pvis.iter().zip(want) {
        assert!((p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9);
    }
    assert_eq!(l.station_range(), (-100.0, 1900.0));
    let c = l.curves[0];
    assert_eq!(c.id, 3);
    assert!((c.grade_in - 0.021691715762041527).abs() < 1e-12);
    assert!((c.grade_out + 0.0092765322915504579).abs() < 1e-12);
    assert!((c.grade_change() + 0.030968248053591985).abs() < 1e-12);
    assert!((c.k() - 48.436708379633892).abs() < 1e-9);
    let (s, e) = (c.start(), c.end());
    assert!((s.x - 747.01097503767369).abs() < 1e-9 && (s.y + 1.6268786821531145).abs() < 1e-9);
    assert!((e.x - 897.01097503767369).abs() < 1e-9 && (e.y + 0.69573992186628431).abs() < 1e-9);
    // The high point.
    let hp = c.start().x - c.grade_in * c.length / c.grade_change();
    assert!((hp - 852.078506099665).abs() < 1e-9);
    assert!((c.elevation_at(hp) + 0.48733117234502177).abs() < 1e-9);

    let l = layout(&profile_elements(&profile(&o, 0x8D91).data)).unwrap();
    let want = [
        (-100.0, -32.5),
        (885.34017744899984, -10.0),
        (1900.0, -30.0),
    ];
    for (p, (x, y)) in l.pvis.iter().zip(want) {
        assert!((p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9);
    }
    assert!(l.curves.is_empty());
}
