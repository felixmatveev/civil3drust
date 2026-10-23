//! Layout-tool alignment (O5, S89): floating tangent, speed on a curve.

use civil3drust::aecc_alignment::KIND_FLOATING;
use civil3drust::aecc_design_speed::{design_speeds, set_design_speeds};
use civil3drust::aecc_elements::{decode_elements, Element};
use civil3drust::raw;

fn rec() -> raw::RawRecord {
    let o = raw::open("samples/S89_O5_layout_tangents_curves.dwg").unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT").remove(0)
}

#[test]
fn s89_floating_tangent() {
    let r = rec();
    let d = decode_elements(&r).unwrap();
    assert_eq!(d.elements.len(), 5);
    assert!((d.length() - 587.054).abs() < 1e-3);
    let e3 = d
        .elements
        .iter()
        .find_map(|e| match &e.record {
            Element::Tangent(t) if t.id == 3 => Some(t.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(e3.kind, KIND_FLOATING);
    // Starts where the three-point arc 2 ends.
    assert!((e3.start.x - 2499.9864).abs() < 1e-3 && (e3.start.y - 7404.8560).abs() < 1e-3);
}

#[test]
fn s89_speed_on_curve() {
    let r = rec();
    let v: Vec<(f64, f64)> = design_speeds(&r)
        .unwrap()
        .iter()
        .map(|e| (e.station, e.speed))
        .collect();
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], (0.0, 60.0));
    assert!((v[1].0 - 285.63).abs() < 1e-9 && v[1].1 == 30.0);
    // Rewriting keeps both entries; only the tangent speed gets an interval.
    let (d, hb) = set_design_speeds(&r, &v, 1).unwrap();
    let r2 = raw::RawRecord {
        data: d,
        handle_bits: hb,
        ..r.clone()
    };
    assert_eq!(design_speeds(&r2).unwrap().len(), 2);
}
