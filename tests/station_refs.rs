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
