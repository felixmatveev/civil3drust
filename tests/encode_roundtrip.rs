//! Every element record of a broad set of samples, re-encoded from its
//! decoded fields with `aecc_encode::encode_element`, must equal Civil 3D's
//! bits exactly (kind prefix, header, record).

use civil3drust::aecc_alignment::{read_header, Linked};
use civil3drust::aecc_build::Bits;
use civil3drust::aecc_elements::element_records;
use civil3drust::aecc_encode::encode_element;
use civil3drust::raw;

const SAMPLES: &[&str] = &[
    "S5_100-200-300.dwg",
    "S10_arc_civ3d.dwg",
    "S11_100-200-300_r50.dwg",
    "S24_ZERO_CIVIL.dwg",
    "S89_O5_layout_tangents_curves.dwg",
    "S94_S93_first_tangent_deleted.dwg",
    "S99_S98_last_tangent_deleted.dwg",
    "S100_S98_interior_tangent_deleted.dwg",
    "S113_line_methods.dwg",
    "S117_S116_curve_methods.dwg",
    "S118_S116_ccw_curves.dwg",
    "S119_S118_first_circle_edited.dwg",
    "S162_S120_free_spirals.dwg",
    "S164_S24_lock_parameters_off.dwg",
    "S165_S120_free_spiral.dwg",
    "S172_S171_one_element_locked.dwg",
    "S173_S120_scs_floating_spirals.dwg",
    "S174_S120_reverse_curves_spirals.dwg",
    "S175_S120_spiral_spiral.dwg",
    "S176_S120_spiral_line_spiral.dwg",
    "S177_long_spiral_method_alignments.dwg",
    "S178_S177_long_reverse_spirals.dwg",
    "S225_S177_end_entities_deleted.dwg",
    "S255_offset_alignments_left_right.dwg",
    "S256_offsets_partial_widening.dwg",
    "S257_widening_transitions.dwg",
    "S258_widening_curves.dwg",
    "S283_S282_connected_alignment.dwg",
    "S284_S283_connected_radius.dwg",
    "S285_connected_fillet_centerlines.dwg",
    "S286_connected_curve_groups.dwg",
    "S288_connected_reverse_spirals.dwg",
];

fn bits(data: &[u8], from: usize, to: usize) -> String {
    (from..to)
        .map(|b| char::from(b'0' + raw::get_bit(data, b)))
        .collect()
}

#[test]
fn every_element_record_reencodes_bit_for_bit() {
    let mut checked = 0;
    let mut kinds = std::collections::BTreeMap::new();
    let mut failures = Vec::new();
    for name in SAMPLES {
        let o = raw::open(format!("samples/{name}")).unwrap();
        for r in raw::records(&o.doc, "AECC_ALIGNMENT") {
            for el in element_records(&r.data) {
                let h = read_header(&r.data, el.header_bit()).expect("header");
                let mut w = Bits::default();
                encode_element(&mut w, &h, &el);
                let ours: String = w.0.iter().map(|b| char::from(b'0' + b)).collect();
                let theirs = bits(&r.data, el.header_bit() - 11, el.end_bit());
                checked += 1;
                let k = format!(
                    "{}/{}",
                    civil3drust::aecc_encode::kind_prefix(&el),
                    h.type_byte
                );
                *kinds
                    .entry(match &el {
                        civil3drust::aecc_elements::Element::Tangent(t) => {
                            format!("tangent kind {}", t.kind)
                        }
                        civil3drust::aecc_elements::Element::Arc(a) => {
                            format!("arc n2 {}", a.params.n2)
                        }
                        civil3drust::aecc_elements::Element::Spiral(s) => {
                            format!("spiral type {} hdr {k}", s.kind)
                        }
                        civil3drust::aecc_elements::Element::Group(g) => {
                            format!("group {k} code {}", g.code)
                        }
                        civil3drust::aecc_elements::Element::FreeSpiral(_) => "free spiral".into(),
                        civil3drust::aecc_elements::Element::Widening(w) => {
                            format!(
                                "widening prefix {}",
                                civil3drust::aecc_widening::kind_prefix(&w.def)
                            )
                        }
                        civil3drust::aecc_elements::Element::LineSpiral(_) => "line spiral".into(),
                    })
                    .or_insert(0) += 1;
                if h.flags.is_none() {
                    *kinds.entry("unlocked header".to_string()).or_insert(0) += 1;
                }
                if ours != theirs {
                    let at = ours.chars().zip(theirs.chars()).position(|(a, b)| a != b);
                    failures.push(format!(
                        "{name} {:X} element {}: {} vs {} bits, first difference at {:?}",
                        r.handle,
                        el.id(),
                        ours.len(),
                        theirs.len(),
                        at
                    ));
                }
            }
        }
    }
    eprintln!("{checked} records: {kinds:#?}");
    // Every kind of record is covered (S5 … S178): 4 line kinds, 10 curve
    // codes, 3 spiral types × 2 header types, free spirals, 12 group codes.
    assert!(kinds.len() >= 33, "coverage shrank: {kinds:#?}");
    assert!(checked > 100, "only {checked} records");
    assert!(
        failures.is_empty(),
        "{} of {checked} differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
