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
}
