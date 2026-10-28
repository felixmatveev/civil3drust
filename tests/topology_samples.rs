//! Topology node lists in companions and site containers (O7 follow-up).

use civil3drust::aecc_elements::decode_elements;
use civil3drust::aecc_site::{membership, Membership};
use civil3drust::aecc_topology::node_list;
use civil3drust::alignment::Point2;
use civil3drust::raw;

/// Element end points of an alignment, without repeats.
fn end_points(rec: &raw::RawRecord) -> Vec<Point2> {
    let d = decode_elements(rec).unwrap();
    let mut pts: Vec<Point2> = Vec::new();
    for e in &d.elements {
        for p in [e.segment.start(), e.segment.end()] {
            if !pts.iter().any(|q| q.distance_to(p) < 1e-6) {
                pts.push(p);
            }
        }
    }
    pts
}

fn same_set(a: &[Point2], b: &[Point2]) -> bool {
    a.len() == b.len() && a.iter().all(|p| b.iter().any(|q| q.distance_to(*p) < 1e-6))
}

/// Every alignment in these Civil 3D-saved drawings: its companion (or its
/// site's container, together with the other alignments of the site) holds
/// one node per element end point.
#[test]
fn nodes_are_the_element_end_points() {
    for p in [
        "samples/S25_ZERO_CIVIL_second_alignment.dwg",
        "samples/S44_S25_plus_two_tangent_alignment.dwg",
        "samples/S49_S48_C_with_curves.dwg",
        "samples/S102_alignment_in_site.dwg",
        "samples/S104_second_alignment_in_site.dwg",
        "samples/S106_test_back_to_siteless.dwg",
    ] {
        let o = raw::open(p).unwrap();
        let aligns = raw::records(&o.doc, "AECC_ALIGNMENT");
        let holders = raw::records(&o.doc, "AECC_FEATURE_STYLE");
        let mut by_holder: Vec<(u64, Vec<Point2>)> = Vec::new();
        for a in &aligns {
            let h = match membership(&o.doc, a) {
                Membership::Siteless { companion } => companion,
                Membership::InSite { container, .. } => container,
                Membership::Unknown => continue,
            };
            let pts = end_points(a);
            match by_holder.iter_mut().find(|x| x.0 == h) {
                Some(x) => x.1.extend(pts),
                None => by_holder.push((h, pts)),
            }
        }
        for (h, pts) in by_holder {
            let rec = holders.iter().find(|r| r.handle == h).unwrap();
            let l = node_list(&rec.data).unwrap_or_else(|| panic!("{p} {h:X}: no node list"));
            let got: Vec<Point2> = l.nodes.iter().map(|n| n.point()).collect();
            assert!(same_set(&got, &pts), "{p} {h:X}: {got:?} vs {pts:?}");
            assert!(l.nodes.iter().all(|n| n.z == 0.0));
        }
    }
}
