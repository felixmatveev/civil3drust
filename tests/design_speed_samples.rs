//! Design speeds (O2, S74).

use civil3drust::aecc_design_speed::{design_speeds, set_design_speeds};
use civil3drust::raw;

fn rec(f: &str, h: u64) -> raw::RawRecord {
    let o = raw::open(f).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap()
}

fn pairs(r: &raw::RawRecord) -> Vec<(f64, f64)> {
    design_speeds(r)
        .unwrap()
        .iter()
        .map(|e| (e.station, e.speed))
        .collect()
}

#[test]
fn s74_speeds() {
    let f = "samples/S74_O2_design_speeds.dwg";
    assert_eq!(pairs(&rec(f, 0x1063)), [(0.0, 60.0)]);
    assert_eq!(pairs(&rec(f, 0x106A)), [(0.0, 30.0)]);
    assert_eq!(pairs(&rec(f, 0x1074)), [(0.0, 60.0), (200.0, 30.0)]);
    // Civil 3D's default in the S24 template: 60 at station 0.
    assert_eq!(
        pairs(&rec("samples/S24_ZERO_CIVIL.dwg", 0xF9E)),
        [(0.0, 60.0)]
    );
}

#[test]
fn rewrite_same_speeds_is_identity() {
    let r = rec("samples/S74_O2_design_speeds.dwg", 0x1063);
    let (d, hb) = set_design_speeds(&r, &[(0.0, 60.0)], 1).unwrap();
    let data_end = r.data.len() * 8 - r.handle_bits;
    assert_eq!(d.len() * 8 - hb, data_end);
    for b in 0..data_end {
        assert_eq!(raw::get_bit(&d, b), raw::get_bit(&r.data, b), "bit {b}");
    }
}

#[test]
fn rewrite_to_two_speeds() {
    let r = rec("samples/S74_O2_design_speeds.dwg", 0x106A);
    let (d, hb) = set_design_speeds(&r, &[(0.0, 60.0), (200.0, 30.0)], 9).unwrap();
    let r2 = raw::RawRecord {
        data: d,
        handle_bits: hb,
        ..r.clone()
    };
    assert_eq!(pairs(&r2), [(0.0, 60.0), (200.0, 30.0)]);
    assert_eq!(raw::object_name(&r2.data), raw::object_name(&r.data));
    // Three strings per speed: its comment and its interval's two.
    let n = |r: &raw::RawRecord| {
        civil3drust::string_stream::parse(&r.data, r.handle_bits)
            .unwrap()
            .strings
            .len()
    };
    assert_eq!(n(&r2), n(&r) + 3);
}

/// Editing Civil 3D's own two-entry table (S74 `mixed speed`), whose entry
/// GUIDs also occur outside the table (S78).
#[test]
fn rewrite_civil3d_two_speed_table() {
    let r = rec("samples/S74_O2_design_speeds.dwg", 0x1074);
    let old = design_speeds(&r).unwrap();
    let speeds = [(0.0, 50.0), (150.0, 40.0), (300.0, 30.0)];
    let (d, hb) = set_design_speeds(&r, &speeds, 5).unwrap();
    let r2 = raw::RawRecord {
        data: d,
        handle_bits: hb,
        ..r.clone()
    };
    let new = design_speeds(&r2).unwrap();
    assert_eq!(pairs(&r2), speeds);
    // Old GUIDs kept: the identity first, then the other old entry's.
    assert_eq!(new[0].id, old[1].id);
    assert_eq!(new[1].id, old[0].id);
}

/// No design speeds after a start-station change (S82, S9, S1).
#[test]
fn empty_speed_tables() {
    assert!(
        design_speeds(&rec("samples/S82_O4_start_station.dwg", 0xFB6))
            .unwrap()
            .is_empty()
    );
    assert!(
        design_speeds(&rec("samples/S82_O4_start_station.dwg", 0xFB9))
            .unwrap()
            .is_empty()
    );
    assert!(design_speeds(&rec(
        "samples/S9_A_seg2plus10_civ3d_resaved_sta100.dwg",
        0x8C5C
    ))
    .unwrap()
    .is_empty());
}

#[test]
fn clear_then_set_start_matches_civil3d_form() {
    use civil3drust::aecc_design_speed::clear_design_speeds;
    use civil3drust::aecc_station::{set_start_station, start_station};
    let r = rec("samples/S77_S24_created_design_speeds_v2.dwg", 0xFB3);
    let (d, hb) = clear_design_speeds(&r).unwrap();
    let r2 = raw::RawRecord {
        data: d,
        handle_bits: hb,
        ..r.clone()
    };
    assert!(design_speeds(&r2).unwrap().is_empty());
    let (d, hb) = set_start_station(&r2, 100.0).unwrap();
    let r3 = raw::RawRecord {
        data: d,
        handle_bits: hb,
        ..r.clone()
    };
    assert_eq!(start_station(&r3), Some(100.0));
    assert_eq!(raw::object_name(&r3.data).as_deref(), Some("Speed 30"));
    let n = |r: &raw::RawRecord| {
        civil3drust::string_stream::parse(&r.data, r.handle_bits)
            .unwrap()
            .strings
            .len()
    };
    // S9/S82: 5 empty strings after the name instead of 8.
    assert_eq!(n(&r3), n(&r) - 3);
    // Same record size as Civil 3D's S82 FB9 minus its user-name string
    // is not comparable (different section-2 form); check the station of
    // Civil 3D's own records instead.
    assert_eq!(
        start_station(&rec("samples/S82_O4_start_station.dwg", 0xFB6)),
        Some(-100.0)
    );
}

/// D-031: moving the start station shifts the design speeds with it.
#[test]
fn move_start_shifts_speeds() {
    use civil3drust::aecc_design_speed::first_interval_station;
    use civil3drust::aecc_station::{move_start_station, start_station};
    let r = rec("samples/S77_S24_created_design_speeds_v2.dwg", 0xFB9);
    let (d, hb) = move_start_station(&r, 250.0).unwrap();
    let r2 = raw::RawRecord {
        data: d,
        handle_bits: hb,
        ..r.clone()
    };
    assert_eq!(start_station(&r2), Some(250.0));
    assert_eq!(pairs(&r2), [(250.0, 50.0), (400.0, 40.0), (550.0, 30.0)]);
    assert_eq!(first_interval_station(&r2), Some(250.0));
    let ids = |r: &raw::RawRecord| {
        design_speeds(r)
            .unwrap()
            .iter()
            .map(|e| e.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&r2), ids(&r));
    // Civil 3D's own interval fields: station, distance, length (S84, S74).
    let s84 = rec("samples/S84_S83_civ3d_resaved.dwg", 0xFB3);
    assert_eq!(first_interval_station(&s84), Some(100.0));
    assert_eq!(pairs(&s84), [(100.0, 60.0)]);
}

/// The interval block computed from the geometry reproduces Civil 3D's
/// records bit for bit (S79 `DESING OFF` two intervals, S84 one). (S24's
/// first element ends at a fillet; Civil 3D's interval holds its length
/// and end point with last-digit rounding differences, 88.19660112501052
/// vs …053 and y −2.1e−14 vs 0, so it is not compared bit for bit.)
#[test]
fn intervals_match_civil3d() {
    for (f, h, speeds) in [
        (
            "samples/S79_O3_design_criteria.dwg",
            0xFB6,
            vec![(0.0, 60.0), (200.0, 30.0)],
        ),
        (
            "samples/S84_S83_civ3d_resaved.dwg",
            0xFB3,
            vec![(100.0, 60.0)],
        ),
    ] {
        let r = rec(f, h);
        let (d, hb) = set_design_speeds(&r, &speeds, 1).unwrap();
        let data_end = r.data.len() * 8 - r.handle_bits;
        assert_eq!(d.len() * 8 - hb, data_end, "{f}");
        for b in 0..data_end {
            assert_eq!(raw::get_bit(&d, b), raw::get_bit(&r.data, b), "{f} bit {b}");
        }
    }
}

/// Adding speeds to an alignment without any (S82 `100 PLUS`, start −1+00).
#[test]
fn add_speeds_to_empty() {
    let r = rec("samples/S82_O4_start_station.dwg", 0xFB6);
    assert!(design_speeds(&r).unwrap().is_empty());
    let (d, hb) = set_design_speeds(&r, &[(-100.0, 40.0), (150.0, 25.0)], 3).unwrap();
    let r2 = raw::RawRecord {
        data: d,
        handle_bits: hb,
        ..r.clone()
    };
    assert_eq!(pairs(&r2), [(-100.0, 40.0), (150.0, 25.0)]);
    let n = |r: &raw::RawRecord| {
        civil3drust::string_stream::parse(&r.data, r.handle_bits)
            .unwrap()
            .strings
            .len()
    };
    assert_eq!(n(&r2), n(&r) + 6);
    assert_eq!(raw::object_name(&r2.data).as_deref(), Some("100 PLUS"));
    // Outside the alignment: refused.
    assert!(set_design_speeds(&r, &[(-200.0, 40.0)], 3).is_err());
}

/// MIST13: the 617-bit block Civil 3D added to S56's X5 is a second speed
/// interval. X3 and X5 both had a zero GUID in their one interval and
/// table entry. In X3 Civil 3D gave both a new GUID; in X5 (also without
/// graphics, MIST1 or label group) it gave the table entry a new GUID,
/// wrote a new interval for it, and left the zero-GUID interval in place.
#[test]
fn zero_guid_interval_duplicated_s57_x5() {
    use civil3drust::aecc_design_speed::{interval_count, ZERO_GUID};
    use civil3drust::aecc_elements::decode_elements;
    let guid = |r: &raw::RawRecord, i: usize| -> [u8; 16] {
        // Interval i: after the elements, BL 0 ×3 and BL n (2 + 2 + 2 + 10
        // bits), then 617 bits per interval for these alignments.
        let end = decode_elements(r).unwrap().records_end_bit().unwrap();
        let at = end + 16 + 617 * i;
        std::array::from_fn(|k| raw::get_byte(&r.data, at + 8 * k))
    };
    let s56 = "samples/S56_S24_minimal_experiments.dwg";
    let s57 = "samples/S57_S56_civ3d_resaved.dwg";
    for h in [0xFBC, 0xFC2] {
        let r = rec(s56, h);
        assert_eq!(interval_count(&r).unwrap(), 1);
        assert_eq!(guid(&r, 0), ZERO_GUID);
    }
    let x3 = rec(s57, 0xFBC);
    assert_eq!(interval_count(&x3).unwrap(), 1);
    assert_ne!(guid(&x3, 0), ZERO_GUID);
    assert_eq!(design_speeds(&x3).unwrap()[0].id, guid(&x3, 0));
    let x5 = rec(s57, 0xFC2);
    assert_eq!(interval_count(&x5).unwrap(), 2);
    assert_eq!(guid(&x5, 0), ZERO_GUID);
    assert_ne!(guid(&x5, 1), ZERO_GUID);
    // The two intervals are the same apart from the GUID.
    let d = &x5.data;
    let end = decode_elements(&x5).unwrap().records_end_bit().unwrap();
    let body = |i: usize| -> Vec<u8> {
        let at = end + 16 + 617 * i + 128;
        (at..at + 617 - 128).map(|b| raw::get_bit(d, b)).collect()
    };
    assert_eq!(body(0), body(1));
}

/// One design-speed interval as stored: station, distance from the element
/// start, length to the element end, element IDs and the six RDs.
fn intervals(r: &raw::RawRecord) -> Vec<(f64, f64, f64, i32, [f64; 6])> {
    use civil3drust::aecc_elements::decode_elements;
    let d = &r.data;
    let mut q = decode_elements(r).unwrap().records_end_bit().unwrap();
    for _ in 0..3 {
        q = raw::read_bl(d, q).unwrap().1;
    }
    let (n, mut q) = raw::read_bl(d, q).unwrap();
    (0..n)
        .map(|_| {
            q += 128 + 13;
            let mut bd = [0.0; 3];
            for v in &mut bd {
                let (x, m) = raw::read_bd(d, q).unwrap();
                *v = x;
                q = m;
            }
            let (id, m) = raw::read_bl(d, q).unwrap();
            q = raw::read_bl(d, raw::read_bl(d, m).unwrap().1).unwrap().1;
            let rd = std::array::from_fn(|k| raw::get_f64(d, q + 64 * k));
            q += 384;
            (bd[0], bd[1], bd[2], id, rd)
        })
        .collect()
}

/// MIST11: the third distance is always the element length (start to the
/// stored end point) minus the distance from the start. S89's curve
/// interval stores an end point 169.264 along the arc, the curve's
/// length when the speed was set; the curve is 120.355 long now, so that
/// interval is stale. Civil 3D rewrites intervals when it edits a curve by
/// its grip (S237).
#[test]
fn speed_interval_third_distance_s89_s235_s237() {
    let r = |p: &str, h| rec(p, h);
    // S89: end point 169.264 along the arc (clockwise from the start).
    let s89 = r("samples/S89_O5_layout_tangents_curves.dwg", 0x8C43);
    let (sta, dist, third, id, rd) = intervals(&s89)[0];
    assert_eq!((sta, id), (285.63, 2));
    let c = (2433.8791, 7416.7853);
    let ang = |x: f64, y: f64| (y - c.1).atan2(x - c.0);
    let along = (ang(rd[0], rd[1]) - ang(rd[2], rd[3])).rem_euclid(std::f64::consts::TAU) * 67.175;
    assert!((along - 169.264).abs() < 1e-2, "{along}");
    assert!((dist + third - along).abs() < 1e-2);
    // S235–S237: the rule holds on curves and at an element's end.
    for (p, want) in [
        (
            "samples/S235_S24_layout_speeds.dwg",
            vec![(113.0, 13.0, 12.3436)],
        ),
        (
            "samples/S236_S235_speed_at_end.dwg",
            vec![(154.15, 28.8064, 0.0007), (113.0, 13.0, 12.3436)],
        ),
        (
            "samples/S237_S236_curve_lengthened.dwg",
            vec![(161.9675, 28.8064, 0.0007), (117.143, 17.143, 16.0181)],
        ),
    ] {
        let got: Vec<(f64, f64, f64)> = intervals(&r(p, 0xFD7))
            .into_iter()
            .filter(|i| i.0 > 0.0)
            .map(|i| (i.0, i.1, i.2))
            .collect();
        assert_eq!(got.len(), want.len(), "{p}");
        for (g, w) in got.iter().zip(&want) {
            assert!(
                (g.0 - w.0).abs() < 1e-3 && (g.1 - w.1).abs() < 1e-3 && (g.2 - w.2).abs() < 1e-3,
                "{p}: {g:?} vs {w:?}"
            );
        }
    }
}
