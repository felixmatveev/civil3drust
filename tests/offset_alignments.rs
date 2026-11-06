//! Offset alignments (S255, owner): 10' to the left and to the right of a
//! tangent – outward spiral – tangent alignment.

use civil3drust::aecc_alignment_refs::{alignment_refs, offset_parent};
use civil3drust::aecc_elements::{decode_elements, Element, Segment, SPIRAL_OFFSET};
use civil3drust::aecc_offset::{offset_info, Side};
use civil3drust::proxy_graphics as pg;
use civil3drust::raw;

/// Graphics polyline vertices farther than 0.01 from the decoded
/// alignment (sampled every 0.01 along the curves).
fn stray_vertices(r: &raw::RawRecord, segs: &[Segment]) -> usize {
    let g = pg::find(&r.data, 4096).unwrap();
    let mut samples = Vec::new();
    for s in segs {
        let n = (s.length() / 0.01) as usize;
        for i in 0..=n {
            let t = s.length() * i as f64 / n as f64;
            samples.push(match s {
                Segment::Spiral { geometry, .. } => geometry.point_at(t).0,
                Segment::Line { start, end } => {
                    let f = t / s.length();
                    civil3drust::Point2::new(
                        start.x + f * (end.x - start.x),
                        start.y + f * (end.y - start.y),
                    )
                }
                Segment::Arc { .. } => s.start(),
            });
        }
    }
    g.chunks
        .iter()
        .filter(|c| c.opcode == pg::OP_POLYLINE)
        .flat_map(|c| pg::poly_vertices(&r.data, c).into_iter().map(|v| v.1))
        .map(|p| {
            samples
                .iter()
                .map(|q| (p[0] - q.x).hypot(p[1] - q.y))
                .fold(f64::MAX, f64::min)
        })
        .filter(|d| *d > 0.01)
        .count()
}

#[test]
fn s255_offsets_decode() {
    let o = raw::open("samples/S255_offset_alignments_left_right.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    let parent = recs.iter().find(|r| r.handle == 0xFD7).unwrap();
    assert_eq!(offset_info(parent).unwrap(), None);
    assert_eq!(alignment_refs(parent).unwrap().trailing, 0);
    assert_eq!(offset_parent(&o.doc, parent), None);
    let p = decode_elements(parent).unwrap();
    let psegs: Vec<Segment> = p.elements.iter().map(|e| e.segment).collect();
    // One vertex of the parent's graphics, mid-spiral, is 0.08 off the
    // curve (not an alignment point; the offsets have one too).
    assert_eq!(stray_vertices(parent, &psegs), 1);
    for (h, side, left, length) in [
        (0x115F, Side::Left, 10.0, 200.0 + 10.0 / 3.0),
        (0x1170, Side::Right, -10.0, 200.0 - 10.0 / 3.0),
    ] {
        let r = recs.iter().find(|r| r.handle == h).unwrap();
        let info = offset_info(r).unwrap().unwrap();
        assert_eq!(info.side, side);
        assert_eq!(info.left_offset(), left);
        assert_eq!((info.parent_start, info.parent_end), (5.0, 405.0));
        assert_eq!((info.first_element, info.last_element), (7, 9));
        assert_eq!(alignment_refs(r).unwrap().trailing, 1);
        assert_eq!(offset_parent(&o.doc, r), Some(0xFD7));
        let d = decode_elements(r).unwrap();
        assert_eq!(d.elements.len(), 3);
        let Element::Spiral(sp) = &d.elements[1].record else {
            panic!("element 8 is not a spiral");
        };
        assert_eq!(sp.kind, SPIRAL_OFFSET);
        let Segment::Spiral { geometry: g, .. } = d.elements[1].segment else {
            panic!();
        };
        assert_eq!(g.offset, left);
        // The offset spiral turns like the parent's, over a length changed
        // by the offset times the turn (1/3 rad); the end station agrees.
        assert!((d.elements[1].segment.length() - length).abs() < 1e-9);
        assert!((d.length() + 5.0 - info.end).abs() < 1e-9);
        // Every element is the parent's moved sideways by the offset.
        for (e, pe) in d.elements.iter().zip(&p.elements) {
            let n = 20;
            for i in 0..=n {
                let s = pe.segment.length() * i as f64 / n as f64;
                let (pp, dir) = match pe.segment {
                    Segment::Spiral { geometry, .. } => geometry.point_at(s),
                    Segment::Line { start, end } => {
                        let f = s / pe.segment.length();
                        let (dx, dy) = (end.x - start.x, end.y - start.y);
                        let l = dx.hypot(dy);
                        (
                            civil3drust::Point2::new(start.x + f * dx, start.y + f * dy),
                            (dx / l, dy / l),
                        )
                    }
                    Segment::Arc { .. } => unreachable!(),
                };
                let want = (pp.x - left * dir.1, pp.y + left * dir.0);
                let near = (0..=2000)
                    .map(|j| {
                        let t = e.segment.length() * j as f64 / 2000.0;
                        let q = match e.segment {
                            Segment::Spiral { geometry, .. } => geometry.point_at(t).0,
                            Segment::Line { start, end } => {
                                let f = t / e.segment.length();
                                civil3drust::Point2::new(
                                    start.x + f * (end.x - start.x),
                                    start.y + f * (end.y - start.y),
                                )
                            }
                            Segment::Arc { .. } => unreachable!(),
                        };
                        (q.x - want.0).hypot(q.y - want.1)
                    })
                    .fold(f64::MAX, f64::min);
                assert!(near < 0.2, "{h:X}: {near}");
            }
        }
        let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
        assert_eq!(stray_vertices(r, &segs), 1, "{h:X}: graphics off the curve");
    }
}

/// S256 (owner): offsets over part of an alignment with an arc and an
/// inward spiral, offsets whose parent was deleted, an offset of an offset
/// with a widening.
#[test]
fn s256_partial_offsets_and_orphans() {
    let o = raw::open("samples/S256_offsets_partial_widening.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    let rec = |h: u64| recs.iter().find(|r| r.handle == h).unwrap();
    let s255 = raw::open("samples/S255_offset_alignments_left_right.dwg").unwrap();
    let old = raw::records(&s255.doc, "AECC_ALIGNMENT");

    // Parent deleted: still offset alignments, head 2, 2, 2, the parent
    // slot null, the geometry unchanged. The elements were renumbered 1–3;
    // the block keeps the old first and last IDs.
    for h in [0x115F, 0x1170] {
        let r = rec(h);
        let info = offset_info(r).unwrap().unwrap();
        assert_eq!(info.head, [2, 2, 2]);
        assert_eq!((info.first_element, info.last_element), (7, 9));
        assert_eq!(alignment_refs(r).unwrap().trailing, 1);
        assert_eq!(offset_parent(&o.doc, r), None);
        let before = decode_elements(old.iter().find(|x| x.handle == h).unwrap()).unwrap();
        let after = decode_elements(r).unwrap();
        for (a, b) in after.elements.iter().zip(&before.elements) {
            assert!((a.segment.end().x - b.segment.end().x).abs() < 1e-9);
            assert!((a.segment.end().y - b.segment.end().y).abs() < 1e-9);
        }
    }

    // Offsets over parent stations 100–900 of TEST, whose spiral is inward:
    // the stored point is the parent's start (its straight end) and the
    // tail holds the part followed (0 to 0.8416 = station 900).
    for (h, left, end) in [
        (0x11B1, 25.0, 913.8601525487878),
        (0x11C2, -10.0, 894.4559389804848),
    ] {
        let r = rec(h);
        let info = offset_info(r).unwrap().unwrap();
        assert_eq!(info.head, [1, 2, 1]);
        assert_eq!(info.left_offset(), left);
        assert_eq!((info.parent_start, info.parent_end), (100.0, 900.0));
        assert_eq!(offset_parent(&o.doc, r), Some(0x119F));
        let d = decode_elements(r).unwrap();
        assert_eq!(d.stored_start_station, Some(100.0));
        // The stored end station agrees with the decoded length.
        assert!((100.0 + d.length() - end).abs() < 1e-9, "{h:X}");
        assert!((info.end - end).abs() < 1e-9);
        let Element::Spiral(sp) = &d.elements[4].record else {
            panic!("element 15 is not a spiral");
        };
        assert_eq!(sp.kind, SPIRAL_OFFSET);
        assert_eq!((sp.tail[0], (sp.tail[1] * 1e4).round()), (0.0, 8416.0));
        for w in d.elements.windows(2) {
            let (a, b) = (w[0].segment.end(), w[1].segment.start());
            assert!((a.x - b.x).hypot(a.y - b.y) < 1e-9);
        }
    }

    // The offset of an offset with a widening: two records of a new kind
    // (prefix 22) are not decoded yet.
    assert_eq!(offset_parent(&o.doc, rec(0x11D9)), Some(0x11B1));
    assert!(decode_elements(rec(0x11D9)).is_err());
}
