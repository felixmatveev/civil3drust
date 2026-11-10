//! Connected (curb return, type 3) alignments (S282 → S283, owner).

use civil3drust::aecc_alignment_refs::{alignment_refs, connected_inputs, offset_parent};
use civil3drust::aecc_build::alignment_type;
use civil3drust::aecc_elements::decode_elements;
use civil3drust::raw;

#[test]
fn s283_connected_alignment() {
    let o = raw::open("samples/S283_S282_connected_alignment.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    let r = recs.iter().find(|r| r.handle == 0x8E38).unwrap();
    assert_eq!(alignment_type(r).unwrap(), 3);
    // Ordinary element records: tangent, spiral, arc, spiral, tangent.
    let d = decode_elements(r).unwrap();
    assert_eq!(d.elements.len(), 5);
    assert!((d.length() - 485.6194).abs() < 1e-4);
    // It starts 20 − 10 = 10 off WEST-EAST and ends 10 off NORTH-SOUTH's
    // right offset (x = 463.38 + 10).
    let s = d.elements[0].segment.start();
    let e = d.elements[4].segment.end();
    assert!((s.y - 10.0).abs() < 1e-9 && (e.x - 473.3777491542559).abs() < 1e-9);
    // The inputs: soft pointers after the check set (A, A, B, B, A, B).
    assert_eq!(alignment_refs(r).unwrap().trailing, 6);
    assert_eq!(connected_inputs(r), Some(vec![0x8E1A, 0x8E2E]));
    assert_eq!(offset_parent(&o.doc, r), None);
    // Offsets still have their parent; centerlines have no inputs.
    let off = recs.iter().find(|r| r.handle == 0x8E1A).unwrap();
    assert_eq!(offset_parent(&o.doc, off), Some(0x8C4A));
    assert_eq!(connected_inputs(off), None);
}

/// S284: the curve radius changed; the elements change, the inputs stay.
#[test]
fn s284_radius_changed() {
    let o = raw::open("samples/S284_S283_connected_radius.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x8E38)
        .unwrap();
    let d = decode_elements(&r).unwrap();
    let civil3drust::aecc_elements::Segment::Arc { geometry: g, .. } = d.elements[2].segment else {
        panic!()
    };
    assert!((g.radius - 186.8048).abs() < 1e-4);
    assert!((d.length() - 543.4323).abs() < 1e-4);
    assert_eq!(connected_inputs(&r), Some(vec![0x8E1A, 0x8E2E]));
    // The radius is not stored outside the elements.
    for v in [150.0, g.radius] {
        let hits = raw::find_f64s(&r.data, &[v], 0, raw::bit_len(&r.data));
        assert!(hits.iter().all(|&h| h < 6000), "{v} at {hits:?}");
    }
}

/// S285: Circular Fillet between the centerlines; the tangents are stored
/// already trimmed to the arc.
#[test]
fn s285_fillet_between_centerlines() {
    let o = raw::open("samples/S285_connected_fillet_centerlines.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x8E82)
        .unwrap();
    assert_eq!(alignment_type(&r).unwrap(), 3);
    assert_eq!(connected_inputs(&r), Some(vec![0x8C4A, 0x8E11]));
    let d = decode_elements(&r).unwrap();
    assert_eq!(d.elements.len(), 3);
    let civil3drust::aecc_elements::Segment::Arc {
        start,
        end,
        geometry: g,
    } = d.elements[1].segment
    else {
        panic!()
    };
    assert_eq!(g.radius, 150.0);
    assert!((start.x - 308.37774915425604).abs() < 1e-9 && (start.y - 25.0).abs() < 1e-9);
    assert!((end.x - 458.3777491542559).abs() < 1e-9 && (end.y - 175.0).abs() < 1e-9);
    assert!((d.length() - (50.0 + 150.0 * std::f64::consts::FRAC_PI_2)).abs() < 1e-9);
}
