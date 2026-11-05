//! S243 (owner): station-anchored data — a design speed on a spiral, a
//! masking region and a station equation. All three live in the interval
//! block after the elements (kinds 2, 4, 3), with their own lists around
//! the design-speed table.

use civil3drust::aecc_build::{alignment_type, masking_regions, pi_display};
use civil3drust::aecc_design_speed::{design_speeds, set_design_speeds};
use civil3drust::aecc_element_edit::required_string_count;
use civil3drust::aecc_station_refs::{
    intervals, station_equations, KIND_EQUATION, KIND_MASK, KIND_SPEED,
};
use civil3drust::raw::{self, RawRecord};
use civil3drust::string_stream;

fn rec(h: u64) -> RawRecord {
    let o = raw::open("samples/S243_spiral_speed_masking_equation.dwg").unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap()
}

fn kinds(r: &RawRecord) -> Vec<(i32, f64)> {
    intervals(r)
        .unwrap()
        .1
        .iter()
        .map(|i| (i.kind, i.station))
        .collect()
}

#[test]
fn s243_station_refs() {
    let speed = rec(0xFD7);
    let mask = rec(0xFDF);
    let equ = rec(0xFE5);
    for r in [&speed, &mask, &equ] {
        let s: Vec<(f64, f64)> = design_speeds(r)
            .unwrap()
            .iter()
            .map(|e| (e.station, e.speed))
            .collect();
        assert_eq!(s, [(0.0, 60.0), (200.0, 30.0)]);
        assert_eq!(alignment_type(r).unwrap(), 1);
        pi_display(r).unwrap();
        let n = string_stream::parse(&r.data, r.handle_bits)
            .unwrap()
            .strings
            .len();
        // The rule plus the Properties form's user string.
        assert_eq!(required_string_count(r).unwrap() + 1, n, "{:X}", r.handle);
    }
    assert_eq!(kinds(&speed), [(KIND_SPEED, 0.0), (KIND_SPEED, 200.0)]);
    let k = kinds(&mask);
    assert_eq!(k.iter().filter(|x| x.0 == KIND_MASK).count(), 2);
    let m = masking_regions(&mask).unwrap();
    assert_eq!(m.len(), 1);
    assert!((m[0].start - 290.2994).abs() < 1e-4 && (m[0].end - 334.1108).abs() < 1e-4);
    assert!(masking_regions(&speed).unwrap().is_empty());
    let e = station_equations(&equ).unwrap();
    assert_eq!(e.len(), 1);
    assert_eq!(
        (e[0].raw_station, e[0].back, e[0].ahead),
        (190.0, 190.0, 310.0)
    );
    assert!(kinds(&equ).contains(&(KIND_EQUATION, 190.0)));
    assert!(station_equations(&speed).unwrap().is_empty());
}

/// Rewriting the speeds keeps the other intervals, the equation list and
/// the masking list; a speed on a spiral gets its interval with the point
/// on the spiral (Civil 3D's to 1e-9).
#[test]
fn s243_speed_rewrite_keeps_other_refs() {
    for h in [0xFD7, 0xFDF, 0xFE5] {
        let r = rec(h);
        let (b, hb) = set_design_speeds(&r, &[(0.0, 60.0), (200.0, 30.0)], 7).unwrap();
        let n = RawRecord {
            data: b,
            handle_bits: hb,
            ..r.clone()
        };
        let mut old = kinds(&r);
        let mut new = kinds(&n);
        old.sort_by(|a, b| a.partial_cmp(b).unwrap());
        new.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(old, new, "{h:X}");
        assert_eq!(masking_regions(&n).unwrap(), masking_regions(&r).unwrap());
        assert_eq!(station_equations(&n), station_equations(&r));
        assert_eq!(n.data.len(), r.data.len());
        if h == 0xFD7 {
            // Only the last digits of the spiral's end and point differ.
            let diff: Vec<usize> = (0..r.data.len())
                .filter(|&i| r.data[i] != n.data[i])
                .collect();
            assert!(diff
                .iter()
                .all(|&i| (3295..=3322).contains(&i) || i + 1 == r.data.len()));
        }
    }
}

/// S244 (S243's speeds rewritten by civil3drust) re-saved by Civil 3D:
/// the same speeds, masking region and station equation.
#[test]
fn s245_resave_reads_back() {
    let o = raw::open("samples/S245_S244_civ3d_resaved.dwg").unwrap();
    let want: [(u64, &[(f64, f64)]); 3] = [
        (0xFD7, &[(0.0, 60.0), (150.0, 40.0), (250.0, 30.0)]),
        (0xFDF, &[(0.0, 50.0), (320.0, 35.0)]),
        (0xFE5, &[(0.0, 55.0), (250.0, 45.0)]),
    ];
    for (h, speeds) in want {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let got = design_speeds(&r).unwrap();
        assert_eq!(got.len(), speeds.len());
        for (g, (st, sp)) in got.iter().zip(speeds) {
            assert!((g.station - st).abs() < 1e-6 && g.speed == *sp, "{h:X}");
        }
    }
    let r = |h| {
        raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap()
    };
    let m = masking_regions(&r(0xFDF)).unwrap();
    assert_eq!(m, masking_regions(&rec(0xFDF)).unwrap());
    let e = station_equations(&r(0xFE5)).unwrap();
    assert_eq!((e[0].back, e[0].ahead), (190.0, 310.0));
}

/// S246 (owner): four masking regions from the Masking tab. The stored bit
/// is the inverted Mask check box; Lock To Start / End leave no flag (the
/// locked regions just start at 0 and end at the alignment's end); the
/// comment is the region's string.
#[test]
fn s246_masking_regions() {
    let o = raw::open("samples/S246_S245_masking_regions.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xFDF)
        .unwrap();
    let m = masking_regions(&r).unwrap();
    let got: Vec<(f64, f64, bool)> = m
        .iter()
        .map(|x| {
            (
                (x.start * 100.0).round() / 100.0,
                (x.end * 100.0).round() / 100.0,
                x.masked,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (0.0, 69.2, true),
            (139.93, 181.09, false),
            (290.3, 334.11, true),
            (363.4, 430.86, true)
        ]
    );
    // Locks as Civil 3D shows them: first region from the start, last to
    // the end (derived; not stored).
    let locks: Vec<(bool, bool)> = m.iter().map(|x| (x.lock_to_start, x.lock_to_end)).collect();
    assert_eq!(
        locks,
        [(true, false), (false, false), (false, false), (false, true)]
    );
    // A lock wins over the recorded station; an unlocked side keeps it.
    assert_eq!(m[3].range(0.0, 431.0).1, 431.0);
    let mut unlocked = m[0].clone();
    unlocked.lock_to_start = false;
    assert_eq!(unlocked.range(-5.0, 431.0).0, m[0].start);
    let k = kinds(&r);
    assert_eq!(k.iter().filter(|x| x.0 == KIND_MASK).count(), 8);
    let s = string_stream::parse(&r.data, r.handle_bits).unwrap();
    assert!(s.strings.iter().any(|x| x.text == "R1"));
    assert_eq!(required_string_count(&r).unwrap() + 1, s.strings.len());
}

/// S247 (owner): FE5 with two equations (Station Control tab: 1+10 back,
/// 1+09 ahead; 1+90 raw back, 1+89 back, 3+10 ahead; both Increasing).
/// The second's back station is derived from the raw one through the
/// first equation (190 − 110 + 109); the equation numbers are strings.
#[test]
fn s247_two_station_equations() {
    let o = raw::open("samples/S247_S246_two_station_equations.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xFE5)
        .unwrap();
    let e = station_equations(&r).unwrap();
    let got: Vec<(f64, f64, f64, bool)> = e
        .iter()
        .map(|x| {
            let q = |v: f64| (v * 1e6).round() / 1e6;
            (q(x.raw_station), q(x.back), q(x.ahead), x.increasing)
        })
        .collect();
    assert_eq!(
        got,
        [(110.0, 110.0, 109.0, true), (190.0, 189.0, 310.0, true)]
    );
    assert!((e[1].back - (e[1].raw_station - e[0].raw_station + e[0].ahead)).abs() < 1e-9);
    assert!(e.iter().all(|x| x.unknown == 1214));
    let k = kinds(&r);
    assert_eq!(k.iter().filter(|x| x.0 == KIND_EQUATION).count(), 2);
    let s = string_stream::parse(&r.data, r.handle_bits).unwrap();
    let texts: Vec<&str> = s.strings.iter().map(|x| x.text.as_str()).collect();
    assert!(texts.contains(&"1") && texts.contains(&"2"));
    assert_eq!(required_string_count(&r).unwrap() + 1, s.strings.len());
}

/// S248 (owner): S247 with equation 2 set to Decreasing and comments
/// typed for both. The bit after Station Ahead is Increase/Decrease (0 =
/// Decreasing); each equation's two strings are its comment and number.
#[test]
fn s248_decreasing_equation() {
    let o = raw::open("samples/S248_S247_decreasing_equation.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xFE5)
        .unwrap();
    let e = station_equations(&r).unwrap();
    assert_eq!(
        e.iter().map(|x| x.increasing).collect::<Vec<_>>(),
        [true, false]
    );
    assert_eq!((e[1].back.round(), e[1].ahead), (189.0, 310.0));
    let s = string_stream::parse(&r.data, r.handle_bits).unwrap();
    let texts: Vec<&str> = s.strings.iter().map(|x| x.text.as_str()).collect();
    let pos = |t: &str| texts.iter().position(|x| *x == t).unwrap();
    assert_eq!(pos("1"), pos("First record") + 1);
    assert_eq!(pos("2"), pos("Decreasing start") + 1);
    assert_eq!(pos("Decreasing start"), pos("1") + 1);
}

/// Stations with equations applied (S247: Civil 3D's Station Control tab
/// shows End 5+50.86'; S248 with equation 2 Decreasing).
#[test]
fn stations_with_equations() {
    use civil3drust::aecc_elements::decode_elements;
    use civil3drust::aecc_station_refs::{display_station, station_back};
    let end = |file: &str| {
        let o = raw::open(file).unwrap();
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == 0xFE5)
            .unwrap();
        let e = station_equations(&r).unwrap();
        let len = decode_elements(&r).unwrap().length();
        (display_station(len, &e), station_back(1, &e).unwrap())
    };
    let (inc, back) = end("samples/S247_S246_two_station_equations.dwg");
    assert!((inc - 550.859).abs() < 1e-3, "{inc}");
    assert!((back - 189.0).abs() < 1e-6);
    let (dec, _) = end("samples/S248_S247_decreasing_equation.dwg");
    assert!((dec - (310.0 - (430.859 - 190.0))).abs() < 1e-3, "{dec}");
}

/// S249 (owner): S248 with each alignment's start station moved through
/// the reference point. Civil 3D stores only the new start station (no
/// reference point), removes the design speeds, moves masking regions
/// and the equations' Raw Station Back with it, and keeps Station Back
/// and Station Ahead as typed.
#[test]
fn s249_start_station_moved() {
    use civil3drust::aecc_station::start_station;
    let o = raw::open("samples/S249_S248_start_stations_moved.dwg").unwrap();
    let r = |h| {
        raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap()
    };
    let (a, m, e) = (r(0xFD7), r(0xFDF), r(0xFE5));
    assert_eq!(start_station(&a), Some(5.0));
    assert_eq!(start_station(&m), Some(0.5));
    assert_eq!(start_station(&e), Some(5.0));
    for x in [&a, &m, &e] {
        assert!(design_speeds(x).unwrap().is_empty());
        assert!(kinds(x).iter().all(|k| k.0 != KIND_SPEED));
    }
    let old = raw::open("samples/S248_S247_decreasing_equation.dwg").unwrap();
    let o_r = |h| {
        raw::records(&old.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap()
    };
    for (n, p) in masking_regions(&m)
        .unwrap()
        .iter()
        .zip(masking_regions(&o_r(0xFDF)).unwrap())
    {
        assert!((n.start - p.start - 0.5).abs() < 1e-9 && (n.end - p.end - 0.5).abs() < 1e-9);
        assert_eq!(
            (n.lock_to_start, n.lock_to_end),
            (p.lock_to_start, p.lock_to_end)
        );
    }
    for (n, p) in station_equations(&e)
        .unwrap()
        .iter()
        .zip(station_equations(&o_r(0xFE5)).unwrap())
    {
        assert!((n.raw_station - p.raw_station - 5.0).abs() < 1e-6);
        assert_eq!(
            (n.back, n.ahead, n.increasing),
            (p.back, p.ahead, p.increasing)
        );
    }
}

fn rec_in(file: &str, h: u64) -> RawRecord {
    let o = raw::open(file).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap()
}

fn with(r: &RawRecord, (data, handle_bits): (Vec<u8>, usize)) -> RawRecord {
    RawRecord {
        data,
        handle_bits,
        ..r.clone()
    }
}

/// Writing S248's two equations again gives Civil 3D's record back.
#[test]
fn equations_rewritten_like_civil3d_s248() {
    use civil3drust::aecc_station_refs::{set_station_equations, EquationSpec};
    let r = rec_in("samples/S248_S247_decreasing_equation.dwg", 0xFE5);
    let specs: Vec<EquationSpec> = station_equations(&r)
        .unwrap()
        .iter()
        .zip(["First record", "Decreasing start"])
        .map(|(e, c)| EquationSpec {
            raw_station: e.raw_station,
            ahead: e.ahead,
            increasing: e.increasing,
            comment: c.into(),
        })
        .collect();
    let n = with(&r, set_station_equations(&r, &specs, 1).unwrap());
    assert_eq!(station_equations(&n), station_equations(&r));
    let s = |x: &RawRecord| {
        string_stream::parse(&x.data, x.handle_bits)
            .unwrap()
            .strings
            .iter()
            .map(|t| t.text.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(s(&n), s(&r));
    let mut a = kinds(&r);
    let mut b = kinds(&n);
    a.sort_by(|x, y| x.partial_cmp(y).unwrap());
    b.sort_by(|x, y| x.partial_cmp(y).unwrap());
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.0, y.0);
        assert!((x.1 - y.1).abs() < 1e-6);
    }
    assert_eq!(design_speeds(&n).unwrap(), design_speeds(&r).unwrap());
}

/// New equations and regions on an alignment that has none (S243 FD7),
/// then removed again.
#[test]
fn equations_and_regions_added_and_removed() {
    use civil3drust::aecc_station_refs::{
        display_station, set_masking_regions, set_station_equations, EquationSpec, MaskSpec,
    };
    let r = rec(0xFD7);
    let eqs = [
        EquationSpec {
            raw_station: 150.0,
            ahead: 1000.0,
            increasing: true,
            comment: "E1".into(),
        },
        EquationSpec {
            raw_station: 320.0,
            ahead: 2000.0,
            increasing: false,
            comment: String::new(),
        },
    ];
    let a = with(&r, set_station_equations(&r, &eqs, 11).unwrap());
    let e = station_equations(&a).unwrap();
    assert_eq!(e.len(), 2);
    assert_eq!(e[1].back, 1170.0); // 320 − 150 + 1000
    assert_eq!(display_station(400.0, &e), 1920.0);
    let regions = [
        MaskSpec {
            start: 0.0,
            end: 50.0,
            masked: true,
            lock_to_start: true,
            lock_to_end: false,
            comment: "M1".into(),
        },
        MaskSpec {
            start: 210.0,
            end: 999.0,
            masked: false,
            lock_to_start: false,
            lock_to_end: true,
            comment: String::new(),
        },
    ];
    let b = with(&a, set_masking_regions(&a, &regions, 21).unwrap());
    let m = masking_regions(&b).unwrap();
    assert_eq!(m.len(), 2);
    assert_eq!(
        (m[0].start, m[0].lock_to_start, m[0].masked),
        (0.0, true, true)
    );
    assert_eq!(
        (m[1].end, m[1].lock_to_end, m[1].masked),
        (400.0, true, false)
    );
    assert_eq!(station_equations(&b).unwrap(), e);
    assert_eq!(design_speeds(&b).unwrap(), design_speeds(&r).unwrap());
    let texts: Vec<String> = string_stream::parse(&b.data, b.handle_bits)
        .unwrap()
        .strings
        .iter()
        .map(|t| t.text.clone())
        .collect();
    let n = texts.len();
    assert_eq!(texts[n - 2], "M1");
    assert!(texts.contains(&"E1".to_string()) && texts.contains(&"2".to_string()));
    assert_eq!(required_string_count(&b).unwrap() + 1, n);
    // Remove both again: back to the original strings and stations.
    let c = with(&b, set_masking_regions(&b, &[], 0).unwrap());
    let c = with(&c, set_station_equations(&c, &[], 0).unwrap());
    assert!(masking_regions(&c).unwrap().is_empty());
    assert!(station_equations(&c).unwrap().is_empty());
    assert_eq!(c.data.len(), r.data.len());
    assert_eq!(kinds(&c), kinds(&r));
}

/// S246's four regions written again: the same regions, locks and comment.
#[test]
fn regions_rewritten_like_civil3d_s246() {
    use civil3drust::aecc_station_refs::{set_masking_regions, MaskSpec};
    let r = rec_in("samples/S246_S245_masking_regions.dwg", 0xFDF);
    let old = masking_regions(&r).unwrap();
    let specs: Vec<MaskSpec> = old
        .iter()
        .enumerate()
        .map(|(i, m)| MaskSpec {
            start: m.start,
            end: m.end,
            masked: m.masked,
            lock_to_start: m.lock_to_start,
            lock_to_end: m.lock_to_end,
            comment: if i == 0 { "R1".into() } else { String::new() },
        })
        .collect();
    let n = with(&r, set_masking_regions(&r, &specs, 3).unwrap());
    let new = masking_regions(&n).unwrap();
    assert_eq!(new.len(), 4);
    for (a, b) in old.iter().zip(&new) {
        assert_eq!(
            (a.masked, a.lock_to_start, a.lock_to_end),
            (b.masked, b.lock_to_start, b.lock_to_end)
        );
        assert!((a.start - b.start).abs() < 1e-6 && (a.end - b.end).abs() < 1e-6);
        assert_eq!((a.start_guid, a.end_guid), (b.start_guid, b.end_guid));
    }
    let t = |x: &RawRecord| {
        string_stream::parse(&x.data, x.handle_bits)
            .unwrap()
            .strings
            .iter()
            .map(|t| t.text.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(t(&n), t(&r));
    assert_eq!(n.data.len(), r.data.len());
}

/// S250 → S251 (owner: clean open and AUDIT): civil3drust gave FD7's first
/// masking region the same two GUIDs as its station equations (one seed
/// for both edits), and Civil 3D merged them: the region moved onto the
/// equations' stations (1+50–3+20) and the duplicate intervals were
/// dropped. Everything else reads back as written. New anchors now never
/// reuse a GUID the record has.
#[test]
fn s251_resave_and_unique_guids() {
    use civil3drust::aecc_station_refs::{
        intervals, set_masking_regions, set_station_equations, EquationSpec, MaskSpec,
    };
    let s251 = |h| rec_in("samples/S251_S250_civ3d_resaved.dwg", h);
    let s250 = |h| rec_in("samples/S250_S245_equations_and_masking_written.dwg", h);
    for h in [0xFDF, 0xFE5] {
        // Raw stations re-rounded by Civil 3D (119.9999999254942).
        let eq = |x: &RawRecord| {
            station_equations(x)
                .unwrap()
                .iter()
                .map(|e| {
                    (
                        (e.raw_station * 1e6).round() as i64,
                        e.back,
                        e.ahead,
                        e.increasing,
                        e.guid,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(eq(&s251(h)), eq(&s250(h)));
        // Civil 3D gives the regions new GUIDs on save and re-rounds a
        // locked end; the model is the same.
        let key = |x: &RawRecord| {
            masking_regions(x)
                .unwrap()
                .iter()
                .map(|m| {
                    let q = |v: f64| (v * 1e6).round() as i64;
                    (
                        q(m.start),
                        q(m.end),
                        m.masked,
                        m.lock_to_start,
                        m.lock_to_end,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(key(&s251(h)), key(&s250(h)));
    }
    let m = masking_regions(&s251(0xFD7)).unwrap();
    assert!((m[0].start - 150.0).abs() < 1e-6 && (m[0].end - 320.0).abs() < 1e-6);
    // The same two edits with one seed now give distinct GUIDs.
    let r = rec(0xFD7);
    let eq = [EquationSpec {
        raw_station: 150.0,
        ahead: 1000.0,
        increasing: true,
        comment: String::new(),
    }];
    let a = with(&r, set_station_equations(&r, &eq, 5).unwrap());
    let mk = [MaskSpec {
        start: 0.0,
        end: 50.0,
        masked: true,
        lock_to_start: false,
        lock_to_end: false,
        comment: String::new(),
    }];
    let b = with(&a, set_masking_regions(&a, &mk, 5).unwrap());
    let g: Vec<[u8; 16]> = intervals(&b).unwrap().1.iter().map(|i| i.guid).collect();
    let mut u = g.clone();
    u.sort();
    u.dedup();
    assert_eq!(u.len(), g.len());
}
