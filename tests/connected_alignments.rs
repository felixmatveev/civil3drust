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
