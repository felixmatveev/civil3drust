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
