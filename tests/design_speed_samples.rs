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
    // One comment string per entry (S76).
    let n = |r: &raw::RawRecord| {
        civil3drust::string_stream::parse(&r.data, r.handle_bits)
            .unwrap()
            .strings
            .len()
    };
    assert_eq!(n(&r2), n(&r) + 1);
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
