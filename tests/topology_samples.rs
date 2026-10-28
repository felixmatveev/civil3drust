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

use civil3drust::aecc_topology::{companion_graph, edge_points, rebuild_companion};

fn companion_and_alignment(p: &str, comp: u64, align: u64) -> (raw::RawRecord, raw::RawRecord) {
    let o = raw::open(p).unwrap();
    let c = raw::records(&o.doc, "AECC_FEATURE_STYLE")
        .into_iter()
        .find(|r| r.handle == comp)
        .unwrap();
    let a = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == align)
        .unwrap();
    (c, a)
}

/// Every Civil 3D companion's single edge runs through the alignment's
/// element end points in path order, with one arc vertex per curve.
#[test]
fn companion_edges_follow_the_path() {
    for (p, c, a) in [
        ("samples/S25_ZERO_CIVIL_second_alignment.dwg", 0xFB7, 0xFB8),
        (
            "samples/S44_S25_plus_two_tangent_alignment.dwg",
            0xFF2,
            0xFF3,
        ),
        ("samples/S48_S25_plus_C_no_curves.dwg", 0x102A, 0x102B),
        ("samples/S49_S48_C_with_curves.dwg", 0x102A, 0x102B),
        ("samples/S106_test_back_to_siteless.dwg", 0x108E, 0xF9E),
    ] {
        let (c, a) = companion_and_alignment(p, c, a);
        let g = companion_graph(&c.data).unwrap_or_else(|| panic!("{p}: no graph"));
        let d = decode_elements(&a).unwrap();
        let mut want: Vec<Point2> = d.elements.iter().map(|e| e.segment.start()).collect();
        want.push(d.elements.last().unwrap().segment.end());
        let got = edge_points(&g, &g.edges[0]);
        assert_eq!(got.len(), want.len(), "{p}");
        for (x, y) in got.iter().zip(&want) {
            assert!(x.distance_to(*y) < 1e-6, "{p}: {x:?} vs {y:?}");
        }
        let arcs = d
            .elements
            .iter()
            .filter(|e| matches!(e.record, civil3drust::aecc_elements::Element::Arc(_)))
            .count();
        let bulges = g.edges[0]
            .vertices
            .iter()
            .filter(|v| v.bulge.is_some())
            .count();
        assert_eq!(arcs, bulges, "{p}");
        assert_eq!(g.edges[0].flags & 0x800 != 0, arcs > 0, "{p}");
    }
}

/// Rebuilding S49's companion from C's geometry gives Civil 3D's graph
/// (other node IDs) and the same record size and tail.
#[test]
fn rebuilt_companion_matches_civil3d_s49() {
    let (c, a) = companion_and_alignment("samples/S49_S48_C_with_curves.dwg", 0x102A, 0x102B);
    let d = decode_elements(&a).unwrap();
    let (bytes, hb) = rebuild_companion(&c.data, c.handle_bits, &d.elements).unwrap();
    assert_eq!(bytes.len(), c.data.len());
    assert_eq!(hb, c.handle_bits);
    let ours = companion_graph(&bytes).unwrap();
    let theirs = companion_graph(&c.data).unwrap();
    assert_eq!(ours.edges[0].flags, theirs.edges[0].flags);
    let (po, pt) = (
        edge_points(&ours, &ours.edges[0]),
        edge_points(&theirs, &theirs.edges[0]),
    );
    assert!(po.iter().zip(&pt).all(|(x, y)| x.distance_to(*y) < 1e-6));
    for (vo, vt) in ours.edges[0].vertices.iter().zip(&theirs.edges[0].vertices) {
        assert_eq!(
            (vo.kind, vo.flags, vo.a, vo.b),
            (vt.kind, vt.flags, vt.a, vt.b)
        );
        match (vo.bulge, vt.bulge) {
            (Some(x), Some(y)) => assert!((x - y).abs() < 1e-9, "bulge {x} vs {y}"),
            (None, None) => {}
            other => panic!("{other:?}"),
        }
    }
    // Everything after the edge (tail, strings, handles) is identical.
    let tail = |d: &[u8], from: usize| -> Vec<u8> {
        (from..raw::bit_len(d))
            .map(|b| raw::get_bit(d, b))
            .collect()
    };
    assert_eq!(
        tail(&bytes, ours.edges_end),
        tail(&c.data, theirs.edges_end)
    );
}

/// The face-like entry count after the edge goes with the handle stream:
/// 3 references without it, 5 with one (S48 vs S49, S98's F9D). Writing an
/// entry into a three-reference companion made Civil 3D discard it (S109).
#[test]
fn tail_entry_matches_reference_count() {
    for (p, c) in [
        ("samples/S25_ZERO_CIVIL_second_alignment.dwg", 0xFB7),
        ("samples/S48_S25_plus_C_no_curves.dwg", 0x102A),
        ("samples/S49_S48_C_with_curves.dwg", 0x102A),
        ("samples/S98_S97_civ3d_audit_resaved.dwg", 0xF9D),
        ("samples/S106_test_back_to_siteless.dwg", 0x108E),
    ] {
        let o = raw::open(p).unwrap();
        let rec = raw::records(&o.doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == c)
            .unwrap();
        let g = companion_graph(&rec.data).unwrap();
        // Tail: BL 0 ×4, BL 2, then the entry count.
        let mut q = g.edges_end;
        for _ in 0..5 {
            q = raw::read_bl(&rec.data, q).unwrap().1;
        }
        let entries = raw::read_bl(&rec.data, q).unwrap().0 as usize;
        let refs = civil3drust::handle_refs::of_record(&rec).len();
        assert_eq!(refs, 3 + 2 * entries, "{p} {c:X}");
    }
}

/// A companion rebuilt from a template keeps the template's tail (S108's
/// failure: S49's tail written into S98's three-reference companion).
#[test]
fn rebuilt_companion_keeps_the_template_tail() {
    let (c, _) = companion_and_alignment("samples/S98_S97_civ3d_audit_resaved.dwg", 0xF9D, 0xF9E);
    let (_, a) = companion_and_alignment("samples/S49_S48_C_with_curves.dwg", 0x102A, 0x102B);
    let d = decode_elements(&a).unwrap();
    let (bytes, hb) = rebuild_companion(&c.data, c.handle_bits, &d.elements).unwrap();
    let (old, new) = (
        companion_graph(&c.data).unwrap(),
        companion_graph(&bytes).unwrap(),
    );
    let bits =
        |d: &[u8], a: usize, b: usize| -> Vec<u8> { (a..b).map(|i| raw::get_bit(d, i)).collect() };
    let old_ss = civil3drust::string_stream::parse(&c.data, c.handle_bits)
        .unwrap()
        .start;
    let new_ss = civil3drust::string_stream::parse(&bytes, hb).unwrap().start;
    assert_eq!(
        bits(&bytes, new.edges_end, new_ss),
        bits(&c.data, old.edges_end, old_ss)
    );
    assert_eq!(new.edges[0].flags, old.edges[0].flags);
    let refs = |d: &[u8], hb: usize| {
        civil3drust::handle_refs::of_record(&raw::RawRecord {
            data: d.to_vec(),
            handle_bits: hb,
            ..c.clone()
        })
        .iter()
        .map(|r| (r.code, r.handle))
        .collect::<Vec<_>>()
    };
    assert_eq!(refs(&bytes, hb), refs(&c.data, c.handle_bits));
}
