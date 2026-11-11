//! Profiles and profile views (S298–S305): element records and links.
// Values copied verbatim from the C3DDUMP output.
#![allow(clippy::excessive_precision)]

use civil3drust::aecc_profile::{
    layout, parent_alignment, pointers_to, profile_elements, CurveDef, ProfileElement,
};
use civil3drust::{raw, Point2};

fn open(id: &str) -> raw::Opened {
    let path = std::fs::read_dir("samples")
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_str().unwrap();
            n.starts_with(&format!("{id}_")) && n.ends_with(".dwg")
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
    let CurveDef::ParabolaLength(l) = c.def else {
        panic!("{:?}", c.def)
    };
    assert!((l - 488.7677).abs() < 1e-4);
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
    assert_eq!(c.def, CurveDef::ParabolaLength(150.0));
    assert_eq!(tangents(&els).len(), 2);
}

#[test]
fn layout_matches_civil3d_api() {
    // C3DDUMP on S306 (owner; samples/S306_profile_curve_length_150.api.txt).
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
    let hp = c.high_low_point();
    assert!((hp.x - 852.078506099665).abs() < 1e-9);
    assert!((hp.y + 0.48733117234502177).abs() < 1e-9);

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

/// Civil 3D's values for one curve (C3DDUMP, S307).
struct Api {
    handle: u64,
    def: CurveDef,
    pvi: (f64, f64),
    start: (f64, f64),
    end: (f64, f64),
    length: f64,
    k: f64,
    high_low: (f64, f64),
    radius: Option<f64>,
}

#[test]
fn curve_types_match_civil3d_api() {
    // S307 (owner; samples/S307_profile_curve_types.api.txt): one profile per
    // curve type and parameter, each tangent – curve – tangent.
    let cases = [
        Api {
            handle: 0x8D91, // parabolic-K, K = 50
            def: CurveDef::ParabolaK(50.0),
            pvi: (885.34017744899984, -10.0),
            start: (778.975694724823, -12.428806737070099),
            end: (991.70466017317665, -12.096554537002586),
            length: 212.72896544835362,
            k: 49.999999999999979,
            high_low: (893.14946189146508, -11.125241826167837),
            radius: None,
        },
        Api {
            handle: 0x8E1B, // circular-length, L = 200
            def: CurveDef::CircularLength(200.0),
            pvi: (576.79917525866222, -25.207147029689715),
            start: (476.787864089877, -27.598719305224677),
            end: (676.78786408987628, -28.407785319687264),
            length: 199.99999999999926,
            k: 35.763446710745136,
            high_low: (562.32013250177727, -26.57619812866551),
            radius: Some(3577.8302968068811),
        },
        Api {
            handle: 0x8E20, // circular-radius, R = 500
            def: CurveDef::CircularRadius(500.0),
            pvi: (446.13909273603713, -33.793881967703513),
            start: (425.23281973748431, -34.772706766578096),
            end: (467.0540338709568, -34.565624152476516),
            length: 41.821214133472495,
            k: 4.9954415073601268,
            high_low: (448.61703861929527, -34.22558573243964),
            radius: Some(500.0),
        },
        Api {
            handle: 0x8E2A, // circular-pass-thru
            def: CurveDef::CircularThrough(Point2::new(437.9567524160084, -44.94837421430729)),
            pvi: (439.30625092072842, -43.2167551340659),
            start: (359.1574018969136, -47.181190670514752),
            end: (519.499889567922, -46.138239258380281),
            length: 160.34248767100837,
            k: 18.667530926451128,
            high_low: (451.47149898806435, -44.899500323904476),
            radius: Some(1868.5924330747932),
        },
        Api {
            handle: 0x8E4D, // asymmetric, L1 = 100, L2 = 150
            def: CurveDef::Asymmetric {
                length1: 100.0,
                length2: 150.0,
            },
            pvi: (1368.5474882013423, -26.842869344524935),
            start: (1268.5474882013423, -33.836947014937195),
            end: (1518.5474882013423, -34.676315480019191),
            length: 250.0,
            k: 20.464335621230973,
            high_low: (1363.9669234068581, -30.500092309465956),
            radius: None,
        },
    ];
    let o = open("S307");
    let near = |a: f64, b: f64, tol: f64| (a - b).abs() < tol;
    for c in cases {
        let h = c.handle;
        let els = profile_elements(&profile(&o, h).data);
        let curve = els
            .iter()
            .find_map(|e| match e {
                ProfileElement::Curve(c) => Some(c),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{h:X}: no curve"));
        let close_def = |a: CurveDef, b: CurveDef| {
            use CurveDef::*;
            let n = |x: f64, y: f64| (x - y).abs() < 1e-9;
            match (a, b) {
                (ParabolaLength(x), ParabolaLength(y))
                | (ParabolaK(x), ParabolaK(y))
                | (CircularLength(x), CircularLength(y))
                | (CircularRadius(x), CircularRadius(y)) => n(x, y),
                (CircularThrough(p), CircularThrough(q)) => n(p.x, q.x) && n(p.y, q.y),
                (
                    Asymmetric { length1, length2 },
                    Asymmetric {
                        length1: m1,
                        length2: m2,
                    },
                ) => n(length1, m1) && n(length2, m2),
                _ => false,
            }
        };
        assert!(close_def(curve.def, c.def), "{h:X} {:?}", curve.def);
        let l = layout(&els).unwrap_or_else(|| panic!("{h:X}: no layout"));
        let p = l.curves[0];
        assert!(
            near(p.pvi.x, c.pvi.0, 1e-9) && near(p.pvi.y, c.pvi.1, 1e-9),
            "{h:X}"
        );
        let (s, e) = (p.start(), p.end());
        assert!(
            near(s.x, c.start.0, 1e-7) && near(s.y, c.start.1, 1e-7),
            "{h:X} start {s:?}"
        );
        assert!(
            near(e.x, c.end.0, 1e-7) && near(e.y, c.end.1, 1e-7),
            "{h:X} end {e:?}"
        );
        assert!(
            near(p.length(), c.length, 1e-7),
            "{h:X} length {}",
            p.length()
        );
        assert!(near(p.k(), c.k, 1e-7), "{h:X} K {}", p.k());
        let hl = p.high_low_point();
        assert!(
            near(hl.x, c.high_low.0, 1e-7) && near(hl.y, c.high_low.1, 1e-7),
            "{h:X} high/low {hl:?}"
        );
        if let (Some(r), civil3drust::aecc_profile::CurveShape::Circular { radius, .. }) =
            (c.radius, p.shape)
        {
            assert!(near(radius, r, 1e-6), "{h:X} radius {radius}");
        }
    }
}

/// An entity in a `C3DDUMP` file: id, [start station, start elevation, end
/// station, end elevation].
type ApiEntity = (i32, [f64; 4]);
/// A profile in a `C3DDUMP` file: handle and its entities.
type ApiProfile = (u64, Vec<ApiEntity>);

/// Each profile's entities in a `C3DDUMP` file.
fn api_entities(path: &str) -> Vec<ApiProfile> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut out: Vec<ApiProfile> = Vec::new();
    let mut cur: Option<ApiEntity> = None;
    let flush = |out: &mut Vec<ApiProfile>, cur: &mut Option<ApiEntity>| {
        if let (Some(e), Some(last)) = (cur.take(), out.last_mut()) {
            last.1.push(e);
        }
    };
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = line.strip_prefix("  Profile [") {
            flush(&mut out, &mut cur);
            let h = u64::from_str_radix(&rest[..rest.find(']').unwrap()], 16).unwrap();
            out.push((h, Vec::new()));
        } else if line.starts_with("  ProfileView [") {
            flush(&mut out, &mut cur);
        } else if t.starts_with("Entities[") {
            flush(&mut out, &mut cur);
            cur = Some((0, [f64::NAN; 4]));
        } else if let Some(e) = cur.as_mut() {
            let num = |p: &str| t.strip_prefix(p).and_then(|v| v.parse::<f64>().ok());
            if let Some(v) = num("EntityId: ") {
                e.0 = v as i32;
            }
            for (i, key) in [
                "StartStation: ",
                "StartElevation: ",
                "EndStation: ",
                "EndElevation: ",
            ]
            .iter()
            .enumerate()
            {
                if let Some(v) = num(key) {
                    e.1[i] = v;
                }
            }
        }
    }
    flush(&mut out, &mut cur);
    out
}

#[test]
fn every_entity_matches_civil3d_api() {
    // Every entity of every profile in the samples with a C3DDUMP: start
    // and end as Civil 3D reports them (S306, S307, S308).
    use civil3drust::aecc_profile::segments;
    for id in ["S306", "S307", "S308", "S309"] {
        let api_path = std::fs::read_dir("samples")
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                let n = p.file_name().unwrap().to_str().unwrap();
                n.starts_with(&format!("{id}_")) && n.ends_with(".api.txt")
            })
            .unwrap();
        let o = open(id);
        for (h, ents) in api_entities(api_path.to_str().unwrap()) {
            let els = profile_elements(&profile(&o, h).data);
            if (id, h) == ("S309", 0x8F84) {
                // Circular, high/low point: Civil 3D kept no element record
                // (its API reports a half circle with invalid grades).
                assert!(els.is_empty());
                continue;
            }
            let segs = segments(&els).unwrap_or_else(|| panic!("{id} {h:X}: not resolved"));
            assert_eq!(segs.len(), ents.len(), "{id} {h:X}");
            for (eid, [ss, se, es, ee]) in ents {
                let s = segs.iter().find(|s| s.id == eid).unwrap();
                let ok = |a: f64, b: f64| (a - b).abs() < 1e-6;
                assert!(
                    ok(s.start.x, ss) && ok(s.start.y, se) && ok(s.end.x, es) && ok(s.end.y, ee),
                    "{id} {h:X} entity {eid}: {:?} → {:?}, Civil 3D ({ss}, {se}) → ({es}, {ee})",
                    s.start,
                    s.end
                );
                // The shape passes through both ends.
                assert!(
                    ok(s.shape.elevation_at(s.start.x), s.start.y),
                    "{id} {h:X} {eid}"
                );
                assert!(
                    ok(s.shape.elevation_at(s.end.x), s.end.y),
                    "{id} {h:X} {eid}"
                );
            }
        }
    }
}
