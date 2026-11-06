//! Offset alignments (S255, owner): 10' to the left and to the right of a
//! tangent – outward spiral – tangent alignment.

use civil3drust::aecc_alignment::Linked;
use civil3drust::aecc_alignment_refs::{alignment_refs, offset_parent};
use civil3drust::aecc_elements::{decode_elements, Element, Segment, SPIRAL_OFFSET};
use civil3drust::aecc_offset::{offset_info, Side};
use civil3drust::proxy_graphics as pg;
use civil3drust::raw;

/// Point at distance `t` along `seg`.
fn point_on(seg: &Segment, t: f64) -> civil3drust::Point2 {
    match *seg {
        Segment::Spiral { geometry, .. } => geometry.point_at(t).0,
        Segment::Line { start, end } => {
            let f = t / seg.length();
            civil3drust::Point2::new(
                start.x + f * (end.x - start.x),
                start.y + f * (end.y - start.y),
            )
        }
        Segment::Arc {
            start, geometry: g, ..
        } => {
            let a0 = (start.y - g.center.y).atan2(start.x - g.center.x);
            let s = if g.counter_clockwise { 1.0 } else { -1.0 };
            let a = a0 + s * t / g.radius;
            civil3drust::Point2::new(
                g.center.x + g.radius * a.cos(),
                g.center.y + g.radius * a.sin(),
            )
        }
    }
}

/// Vertices of the graphics' curve polylines (more than two vertices; the
/// two-vertex ones are tick marks) farther than 0.01 from the decoded
/// alignment (sampled every 0.01 along the curves).
fn stray_vertices(r: &raw::RawRecord, segs: &[Segment]) -> usize {
    let g = pg::find(&r.data, 4096).unwrap();
    let mut samples = Vec::new();
    for s in segs {
        let n = (s.length() / 0.01).ceil() as usize;
        for i in 0..=n {
            samples.push(point_on(s, s.length() * i as f64 / n as f64));
        }
    }
    g.chunks
        .iter()
        .filter(|c| c.opcode == pg::OP_POLYLINE)
        .flat_map(|c| {
            let v = pg::poly_vertices(&r.data, c);
            let n = v.len();
            v.into_iter().filter(move |_| n > 2).map(|v| v.1)
        })
        .map(|p| {
            let d = samples
                .iter()
                .map(|q| (p[0] - q.x).hypot(p[1] - q.y))
                .fold(f64::MAX, f64::min);
            d
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
    assert_eq!(stray_vertices(parent, &psegs), 0);
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
        assert_eq!(stray_vertices(r, &segs), 0, "{h:X}: graphics off the curve");
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

    // The offset of an offset with a widening: linear transitions (prefix
    // 22); the second crosses onto the spiral with three points, the last
    // where the trimmed offset spiral starts.
    assert_eq!(offset_parent(&o.doc, rec(0x11D9)), Some(0x11B1));
    let d = decode_elements(rec(0x11D9)).unwrap();
    for w in d.elements.windows(2) {
        let (a, b) = (w[0].segment.end(), w[1].segment.start());
        assert!((a.x - b.x).hypot(a.y - b.y) < 1e-9);
    }
    let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
    // Element ends against the stations Civil 3D stores for the regions.
    let info = offset_info(rec(0x11D9)).unwrap().unwrap();
    assert_eq!(info.regions.len(), 3);
    assert_eq!(info.regions[1].offset, -30.0);
    let station_after = |id: i32| {
        let mut s = 100.0;
        for e in &d.elements {
            s += e.segment.length();
            if e.record.id() == id
                && d.elements
                    .iter()
                    .rposition(|x| x.record.id() == id)
                    .is_some_and(|j| std::ptr::eq(&d.elements[j], e))
            {
                return s;
            }
        }
        unreachable!()
    };
    let r = &info.regions;
    for (id, want, tol) in [
        (14, r[0].end, 1e-9),
        (17, r[1].start, 1e-9),
        (16, r[1].end, 1e-9),
        // Over the spiral the transition is three points: Civil 3D's
        // stations agree with lines joining them.
        (18, r[2].start, 1e-9),
        (15, r[2].end, 1e-9),
    ] {
        let got = station_after(id);
        assert!((got - want).abs() < tol, "{id}: {got} vs {want}");
    }
    assert_eq!(stray_vertices(rec(0x11D9), &segs), 0);
}

/// S257 (owner): offsets at 0 with one widening (26 left, stations
/// 112.17–260.54), transitions of 75 in and out — linear (1218) and
/// curve-line-curve with radii 45 and 25 (1234).
#[test]
fn s257_widenings() {
    let o = raw::open("samples/S257_widening_transitions.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    for (h, stations) in [
        (
            0x1218,
            [
                37.17428376534788,
                116.5531222552124,
                264.9231247834791,
                344.30196327334363,
                424.154815000994,
            ],
        ),
        (
            0x1234,
            [
                37.17428376534788,
                117.120138730357,
                265.49014125862374,
                345.43599622363286,
                425.28884795128323,
            ],
        ),
    ] {
        let r = recs.iter().find(|r| r.handle == h).unwrap();
        let d = decode_elements(r).unwrap();
        // Element ends match the region stations Civil 3D stores.
        let mut ends = Vec::new();
        let mut s = 0.0;
        for (i, e) in d.elements.iter().enumerate() {
            s += e.segment.length();
            let last = d
                .elements
                .get(i + 1)
                .is_none_or(|n| n.record.id() != e.record.id());
            if last {
                ends.push(s);
            }
        }
        assert_eq!(ends.len(), 5, "{h:X}");
        for (a, b) in ends.iter().zip(stations) {
            assert!((a - b).abs() < 1e-6, "{h:X}: {a} vs {b}");
        }
        let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
        assert_eq!(stray_vertices(r, &segs), 0, "{h:X}: graphics off the curve");
    }
}

#[test]
fn s257_offset_regions_and_transitions() {
    use civil3drust::aecc_offset::OffsetTransition;
    let o = raw::open("samples/S257_widening_transitions.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    for (h, kind, ids, own, values) in [
        (
            0x1218,
            1,
            [6, 7],
            [116.5531222552124, 264.9231247834791],
            vec![0.0, 0.0, 2.0, 0.0],
        ),
        (
            0x1234,
            2,
            [8, 9],
            [117.120138730357, 265.49014125862374],
            vec![0.0, 45.0, 25.0],
        ),
    ] {
        let r = recs.iter().find(|r| r.handle == h).unwrap();
        let info = offset_info(r).unwrap().unwrap();
        assert_eq!(info.regions.len(), 3);
        let w = &info.regions[1];
        assert_eq!((w.offset, w.kind), (-26.0, 1));
        assert!((w.parent_start - 112.17428376534788).abs() < 1e-9);
        assert!((w.parent_end - 260.5442862936146).abs() < 1e-9);
        assert_eq!([w.start, w.end], own);
        assert_eq!((w.first_element, w.last_element), (4, 4));
        let ends: Vec<i32> = info.regions.iter().map(|r| r.kind).collect();
        assert_eq!(ends, [2, 1, 2]);
        assert_eq!(
            info.transitions.unwrap(),
            ids.iter()
                .map(|&element| OffsetTransition {
                    kind,
                    length: 75.0,
                    element,
                    values: values.clone(),
                })
                .collect::<Vec<_>>()
        );
    }
}

/// Element start/end stations by ID (first part's start, last part's end).
fn element_stations(
    elements: &[civil3drust::aecc_elements::ResolvedElement],
    start: f64,
) -> std::collections::HashMap<i32, (f64, f64)> {
    let mut out = std::collections::HashMap::new();
    let mut s = start;
    for e in elements {
        let a = s;
        s += e.segment.length();
        out.entry(e.record.id()).or_insert((a, s)).1 = s;
    }
    out
}

/// S258 (owner): widenings (57.055 left, nominal 10.28 left) on a parent
/// with an arc and a spiral: transitions ending on the arc, a widening
/// over the arc, one ending on the spiral (linear, point lists), and
/// curve – reverse curve and curve – curve – reverse curve transitions.
#[test]
fn s258_widenings_on_curves() {
    use civil3drust::aecc_widening::WideningDef;
    let o = raw::open("samples/S258_widening_curves.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    for h in [0x1297, 0x12A6, 0x12B4, 0x12C2, 0x12D0] {
        let r = recs.iter().find(|r| r.handle == h).unwrap();
        let info = offset_info(r).unwrap().unwrap();
        assert_eq!(info.regions.len(), 3);
        let d = decode_elements(r).unwrap();
        for w in d.elements.windows(2) {
            let (a, b) = (w[0].segment.end(), w[1].segment.start());
            // The offset spiral (parent R 14.94, turning 2.1 rad) starts
            // 8e-6 from the end of the offset tangent Civil 3D stores.
            let gap = (a.x - b.x).hypot(a.y - b.y);
            assert!(gap < 1e-5, "{h:X}: {gap}");
        }
        // Every region starts and ends where Civil 3D says.
        let st = element_stations(&d.elements, 0.0);
        for g in &info.regions {
            let (a, b) = (st[&g.first_element].0, st[&g.last_element].1);
            assert!((a - g.start).abs() < 1e-6, "{h:X}: {a} vs {}", g.start);
            assert!((b - g.end).abs() < 1e-6, "{h:X}: {b} vs {}", g.end);
        }
        let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
        assert_eq!(stray_vertices(r, &segs), 0, "{h:X}: graphics off the curve");
    }
    // Transition settings: type, length, element, radii as Civil 3D shows.
    for (h, want) in [
        (
            0x1297,
            vec![(2, 8, vec![0.0, 45.0, 25.0]), (2, 9, vec![0.0, 45.0, 25.0])],
        ),
        (
            0x12B4,
            vec![
                (2, 8, vec![0.0, 45.0, 25.0]),
                (1, 14, vec![0.0, 0.0, 2.0, 0.0]),
            ],
        ),
        (
            0x12C2,
            vec![
                (4, 14, vec![0.0, 45.0, 60.032898432624165, 25.0]),
                (4, 17, vec![0.0, 45.0, 71.89078541519343, 25.0]),
            ],
        ),
        (
            0x12D0,
            vec![(5, 19, vec![0.0, 0.0, 45.0]), (5, 22, vec![0.0, 0.0, 45.0])],
        ),
    ] {
        let r = recs.iter().find(|r| r.handle == h).unwrap();
        let t = offset_info(r).unwrap().unwrap().transitions.unwrap();
        let got: Vec<_> = t
            .iter()
            .map(|t| (t.kind, t.element, t.values.clone()))
            .collect();
        assert_eq!(got, want, "{h:X}");
        assert_eq!(t[0].length, 75.0);
        assert!((t[1].length - 74.98759200070248).abs() < 1e-12);
    }
    // The reverse radii Civil 3D shows for 12D0 (curve – reverse curve):
    // 38.516 in, 126.914 out.
    let r = recs.iter().find(|r| r.handle == 0x12D0).unwrap();
    let d = decode_elements(r).unwrap();
    let mut radii = Vec::new();
    for (i, e) in d.elements.iter().enumerate() {
        let civil3drust::aecc_elements::Element::Widening(w) = &e.record else {
            continue;
        };
        assert!(matches!(w.def, WideningDef::CurveReverseCurve { .. }));
        if e.part == 0 {
            let Segment::Arc { geometry: g, .. } = d.elements[i + 1].segment else {
                panic!()
            };
            radii.push((g.radius * 1000.0).round() / 1000.0);
        }
    }
    assert_eq!(radii, [38.516, 126.914]);
}
