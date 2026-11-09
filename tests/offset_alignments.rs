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
            // Two-vertex polylines are tick marks; a closed five-vertex one
            // is a frame (S276 draws the extents).
            let n = v.len();
            let closed = n == 5 && v[0].1 == v[4].1;
            let keep = n > 2 && !closed;
            v.into_iter().filter(move |_| keep).map(|v| v.1)
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

/// All handles and class names of a drawing.
fn inventory(path: &str) -> std::collections::BTreeMap<u64, String> {
    use acadrust::entities::EntityType;
    use acadrust::objects::ObjectType;
    let o = raw::open(path).unwrap();
    let name = |s: String| s.split(['(', ' ', '{']).next().unwrap_or("?").to_string();
    let mut out = std::collections::BTreeMap::new();
    for e in o.doc.entities() {
        let c = match e {
            EntityType::Unknown(u) => u.dxf_name.clone(),
            other => name(format!("{other:?}")),
        };
        out.insert(e.common().handle.value(), c);
    }
    for (h, ob) in o.doc.objects.iter() {
        let c = match ob {
            ObjectType::Unknown { type_name, .. } => type_name.clone(),
            other => name(format!("{other:?}")),
        };
        out.insert(h.value(), c);
    }
    out
}

/// S259 → S260 → S261 (owner): an offset alignment added to a drawing
/// without associations, then deleted.
#[test]
fn s259_s261_offset_objects() {
    let a = inventory("samples/S259_offset_base.dwg");
    let b = inventory("samples/S260_S259_offset_left_10.dwg");
    let c = inventory("samples/S261_S260_offset_deleted.dwg");
    let added: Vec<(u64, &str)> = b
        .iter()
        .filter(|(h, _)| !a.contains_key(h))
        .map(|(h, n)| (*h, n.as_str()))
        .collect();
    assert_eq!(
        added,
        [
            (0x8D70, "AECC_FEATURE_STYLE"),
            (0x8D71, "AECC_ALIGNMENT"),
            (0x8D72, "Dictionary"),
            (0x8D73, "Associative"),
            (0x8D74, "Dictionary"),
            (0x8D75, "Associative"),
            (0x8D76, "AECC_ASSOC_ACTION"),
            (0x8D77, "AECC_ASSOC_OFFSET_ALIGNMENT_ACTION_BODY"),
            (0x8D78, "AECC_ASSOC_DEPENDENCY"),
            (0x8D79, "AECC_ASSOC_DEPENDENCY_BODY"),
            (0x8D7A, "AECC_ASSOC_DEPENDENCY"),
            (0x8D7B, "AECC_ASSOC_DEPENDENCY_BODY"),
            // Save-time xrecords (rewritten on every save).
            (0x8D7C, "XRecord"),
            (0x8D7D, "XRecord"),
        ]
    );
    // After the delete only the companion is left of them.
    let kept: Vec<u64> = c.keys().filter(|h| !a.contains_key(h)).copied().collect();
    assert_eq!(kept, [0x8D70, 0x8D87, 0x8D88]);
    assert_eq!(c[&0x8D87], "XRecord");
    // The parent's reactor: the dependency on it, gone after the delete.
    let o = raw::open("samples/S260_S259_offset_left_10.dwg").unwrap();
    let parent = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x8C4A)
        .unwrap();
    let refs = civil3drust::handle_refs::of_record(&parent);
    assert_eq!((refs[0].code, refs[0].handle), (4, 0x8D78));
    let o = raw::open("samples/S261_S260_offset_deleted.dwg").unwrap();
    let parent = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x8C4A)
        .unwrap();
    let refs = civil3drust::handle_refs::of_record(&parent);
    assert_ne!(refs[0].handle, 0x8D78);
}

/// S262 (owner): S260 with the parent deleted; the offset stays.
#[test]
fn s262_parent_deleted() {
    let b = inventory("samples/S260_S259_offset_left_10.dwg");
    let d = inventory("samples/S262_S260_parent_deleted.dwg");
    for h in 0x8D72..=0x8D7B {
        assert!(!d.contains_key(&h), "{h:X} kept");
    }
    assert!(!d.contains_key(&0x8C4A));
    assert_eq!(d[&0x8C49], "AECC_FEATURE_STYLE"); // the parent's companion
    assert_eq!(b[&0x8D71], d[&0x8D71]);
    let o = raw::open("samples/S262_S260_parent_deleted.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x8D71)
        .unwrap();
    let info = offset_info(&r).unwrap().unwrap();
    assert_eq!(info.head, [2, 2, 2]);
    assert_eq!((info.first_element, info.last_element), (3, 3));
    assert_eq!(info.left_offset(), 10.0);
    let refs = alignment_refs(&r).unwrap();
    assert_eq!(refs.trailing, 1);
    assert_eq!(offset_parent(&o.doc, &r), None);
    let all = civil3drust::handle_refs::of_record(&r);
    assert_ne!(all[0].code, 4, "reactor kept");
    let d = decode_elements(&r).unwrap();
    assert_eq!(d.elements.len(), 1);
}

/// Interval (kind, station, GUID) list of an alignment record.
fn intervals_of(r: &raw::RawRecord) -> Vec<(i32, f64, [u8; 16])> {
    let d = decode_elements(r).unwrap();
    let e = d.records_end_bit().unwrap();
    civil3drust::aecc_station_refs::intervals_at(&r.data, e)
        .unwrap()
        .0
        .iter()
        .map(|i| (i.kind, i.station, i.guid))
        .collect()
}

/// Whether the 16 bytes of `g` occur, at any bit, in `r`.
fn holds_guid(r: &raw::RawRecord, g: &[u8; 16]) -> bool {
    let n = raw::bit_len(&r.data);
    (0..n.saturating_sub(128)).any(|b| (0..16).all(|i| raw::get_byte(&r.data, b + 8 * i) == g[i]))
}

/// S260 → S263 → S264 → S265 (owner): the offset changed to 20' left, its
/// stations set to 1+00–9+00 (locks cleared), then made static.
#[test]
fn s263_s265_offset_parameters() {
    let get = |path: &str, h: u64| {
        let o = raw::open(path).unwrap();
        raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap()
    };
    let steps = [
        (
            "samples/S260_S259_offset_left_10.dwg",
            [1, 2, 1],
            true,
            10.0,
            (0.0, 1000.0),
            1000.0,
        ),
        (
            "samples/S263_S260_offset_20.dwg",
            [1, 2, 1],
            true,
            20.0,
            (0.0, 1000.0),
            1000.0,
        ),
        (
            "samples/S264_S263_offset_stations.dwg",
            [1, 2, 1],
            false,
            20.0,
            (100.0, 900.0),
            800.0,
        ),
        (
            "samples/S265_S264_offset_static.dwg",
            [2, 2, 1],
            false,
            20.0,
            (100.0, 900.0),
            800.0,
        ),
    ];
    for (path, head, locks, left, (ps, pe), end) in steps {
        let r = get(path, 0x8D71);
        let i = offset_info(&r).unwrap().unwrap();
        assert_eq!(i.head, head, "{path}");
        assert_eq!((i.lock_start, i.lock_end), (locks, locks), "{path}");
        assert_eq!(i.left_offset(), left, "{path}");
        assert_eq!((i.parent_start, i.parent_end), (ps, pe), "{path}");
        // The offset keeps its own stationing from 0.
        assert_eq!((i.start, i.end), (0.0, end), "{path}");
        let d = decode_elements(&r).unwrap();
        let Segment::Line { start, end: e } = d.elements[0].segment else {
            panic!()
        };
        assert_eq!((start.x, start.y, e.x, e.y), (ps, left, pe, left), "{path}");
        // The parent anchors the range: two intervals at the parent
        // stations whose GUIDs the offset record holds.
        let parent = get(path, 0x8C4A);
        let anchors: Vec<(i32, f64)> = intervals_of(&parent)
            .into_iter()
            .filter(|(_, _, g)| holds_guid(&r, g))
            .map(|(k, s, _)| (k, s))
            .collect();
        let want: Vec<(i32, f64)> = if locks {
            vec![(90, pe), (0, ps)]
        } else {
            vec![(6, pe), (6, ps)]
        };
        let mut a = anchors.clone();
        a.sort_by(|x, y| x.1.total_cmp(&y.1));
        let mut w = want.clone();
        w.sort_by(|x, y| x.1.total_cmp(&y.1));
        assert_eq!(a, w, "{path}");
    }
    // Static: the element is locked (header flag 0x02), nothing else in
    // the record changes; the association objects stay.
    let r = get("samples/S265_S264_offset_static.dwg", 0x8D71);
    let d = decode_elements(&r).unwrap();
    let flags =
        civil3drust::aecc_alignment::read_header(&r.data, d.elements[0].record.header_bit())
            .unwrap()
            .flags;
    assert_eq!(flags, Some(0x02));
    let r = get("samples/S264_S263_offset_stations.dwg", 0x8D71);
    let d = decode_elements(&r).unwrap();
    let flags =
        civil3drust::aecc_alignment::read_header(&r.data, d.elements[0].record.header_bit())
            .unwrap()
            .flags;
    assert_eq!(flags, None);
    let inv = inventory("samples/S265_S264_offset_static.dwg");
    for h in 0x8D72..=0x8D7B {
        assert!(inv.contains_key(&h), "{h:X} removed");
    }
}

/// S266 (owner): the parent shortened to 500 with a dynamic offset;
/// S267: then only the end station's lock cleared.
#[test]
fn s266_s267_parent_edit_and_locks() {
    let get = |path: &str, h: u64| {
        let o = raw::open(path).unwrap();
        raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap()
    };
    for (path, locks, end_kind) in [
        ("samples/S266_S260_parent_shortened.dwg", (true, true), 90),
        ("samples/S267_S266_end_unlocked.dwg", (true, false), 6),
    ] {
        let r = get(path, 0x8D71);
        let i = offset_info(&r).unwrap().unwrap();
        assert_eq!(i.head, [1, 2, 1]);
        assert_eq!((i.lock_start, i.lock_end), locks, "{path}");
        // The offset was rebuilt over the shortened parent and saved so.
        assert_eq!(
            (i.parent_start, i.parent_end, i.start, i.end),
            (0.0, 500.0, 0.0, 500.0)
        );
        let d = decode_elements(&r).unwrap();
        let Segment::Line { start, end } = d.elements[0].segment else {
            panic!()
        };
        assert_eq!((start.x, start.y, end.x, end.y), (0.0, 10.0, 500.0, 10.0));
        let parent = get(path, 0x8C4A);
        let mut anchors: Vec<(i32, f64)> = intervals_of(&parent)
            .into_iter()
            .filter(|(_, _, g)| holds_guid(&r, g))
            .map(|(k, s, _)| (k, s))
            .collect();
        anchors.sort_by(|x, y| x.1.total_cmp(&y.1));
        // Locked start: kind 0; locked end: 90; a set station: 6.
        assert_eq!(anchors, [(0, 0.0), (end_kind, 500.0)], "{path}");
        // The parent's other start/end pair is not moved with the parent:
        // its end stays at 1000, past the parent's new end.
        let other: Vec<(i32, f64)> = intervals_of(&parent)
            .into_iter()
            .filter(|(_, _, g)| !holds_guid(&r, g))
            .map(|(k, s, _)| (k, s))
            .collect();
        assert_eq!(other, [(0, 0.0), (90, 1000.0)], "{path}");
    }
}

/// S269–S271 (owner): static, Station Locking, Cant.
#[test]
fn s269_s271_update_and_lock_modes() {
    let get = |path: &str, h: u64| {
        let o = raw::open(path).unwrap();
        raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap()
    };
    for (path, head, lock_mode, start_kind, locked) in [
        ("samples/S267_S266_end_unlocked.dwg", [1, 2, 1], 1, 0, None),
        (
            "samples/S269_S267_static_end_400.dwg",
            [2, 2, 1],
            1,
            0,
            Some(0x02),
        ),
        (
            "samples/S270_S269_station_locking.dwg",
            [1, 2, 1],
            2,
            6,
            None,
        ),
        ("samples/S271_S270_cant.dwg", [1, 2, 1], 1, 0, None),
    ] {
        let r = get(path, 0x8D71);
        let i = offset_info(&r).unwrap().unwrap();
        assert_eq!((i.head, i.lock_mode), (head, lock_mode), "{path}");
        assert_eq!((i.lock_start, i.lock_end), (true, false), "{path}");
        let d = decode_elements(&r).unwrap();
        let flags =
            civil3drust::aecc_alignment::read_header(&r.data, d.elements[0].record.header_bit())
                .unwrap()
                .flags;
        assert_eq!(flags, locked, "{path}");
        let parent = get(path, 0x8C4A);
        let mut anchors: Vec<(i32, f64)> = intervals_of(&parent)
            .into_iter()
            .filter(|(_, _, g)| holds_guid(&r, g))
            .map(|(k, s, _)| (k, s))
            .collect();
        anchors.sort_by(|x, y| x.1.total_cmp(&y.1));
        assert_eq!(anchors[0], (start_kind, 0.0), "{path}");
        assert_eq!(anchors[1].0, 6, "{path}");
    }
    // Cant: a section-2 BD becomes 4.92125 (1.5 m).
    let r = get("samples/S271_S270_cant.dwg", 0x8D71);
    assert!(!raw::find_f64s(&r.data, &[4.92125], 0, raw::bit_len(&r.data)).is_empty());
    let r = get("samples/S270_S269_station_locking.dwg", 0x8D71);
    assert!(raw::find_f64s(&r.data, &[4.92125], 0, raw::bit_len(&r.data)).is_empty());
}

/// S268 (experiment: a stale offset) → S272 (re-saved) / S273 (AUDIT,
/// re-saved): Civil 3D keeps stale offset geometry until AUDIT.
#[test]
fn s268_stale_offset_kept_until_audit() {
    for (path, length) in [
        ("samples/S268_S266_offset_stale.dwg", 1000.0),
        ("samples/S272_S268_civ3d_resaved.dwg", 1000.0),
        ("samples/S273_S268_audit_resaved.dwg", 500.0),
    ] {
        let o = raw::open(path).unwrap();
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == 0x8D71)
            .unwrap();
        let d = decode_elements(&r).unwrap();
        assert_eq!(d.length(), length, "{path}");
        let i = offset_info(&r).unwrap().unwrap();
        assert_eq!((i.parent_end, i.end), (500.0, 500.0), "{path}");
    }
}

/// The block after the transitions, in every offset sample: the element
/// map covers each non-transition element once; the anchors name the
/// parent's intervals at their stations and span the parent range.
#[test]
fn offset_element_map_and_anchors() {
    use civil3drust::aecc_elements::Element;
    let mut checked = 0;
    for path in [
        "samples/S255_offset_alignments_left_right.dwg",
        "samples/S256_offsets_partial_widening.dwg",
        "samples/S257_widening_transitions.dwg",
        "samples/S258_widening_curves.dwg",
        "samples/S260_S259_offset_left_10.dwg",
        "samples/S262_S260_parent_deleted.dwg",
        "samples/S264_S263_offset_stations.dwg",
        "samples/S266_S260_parent_shortened.dwg",
        "samples/S270_S269_station_locking.dwg",
        "samples/S273_S268_audit_resaved.dwg",
        "samples/S276_offset_arc_base.dwg",
        "samples/S277_S276_widening_added.dwg",
        "samples/S278_S277_widening_30.dwg",
        "samples/S279_S278_curve_reverse_in.dwg",
        "samples/S280_S279_widening_deleted.dwg",
        "samples/S281_S280_two_widenings.dwg",
    ] {
        let o = raw::open(path).unwrap();
        let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
        for r in &recs {
            let Some(info) = offset_info(r).unwrap() else {
                continue;
            };
            let d = decode_elements(r).unwrap();
            let mut ids: Vec<i32> = d
                .elements
                .iter()
                .filter(|e| !matches!(e.record, Element::Widening(_)))
                .map(|e| e.record.id())
                .collect();
            ids.dedup();
            ids.sort();
            let mut mapped: Vec<i32> = info
                .element_map
                .iter()
                .flat_map(|(_, _, v)| v.clone())
                .collect();
            mapped.sort();
            // Orphans keep the map as made (S256 renumbered its elements).
            if info.head[2] == 1 {
                assert_eq!(mapped, ids, "{path} {:X}", r.handle);
            }
            // One anchor per region start and end, shared where regions
            // meet without a transition: start, end and both ends of each
            // transition (2 + 2 per transition).
            let n_tr = info.transitions.as_ref().map_or(0, Vec::len);
            assert_eq!(info.anchors.len(), 2 + 2 * n_tr, "{path} {:X}", r.handle);
            let first = info.anchors.first().unwrap().0;
            let last = info.anchors.last().unwrap().0;
            assert!(
                (first - info.parent_start).abs() < 1e-9,
                "{path} {:X}",
                r.handle
            );
            assert!(
                (last - info.parent_end).abs() < 1e-9,
                "{path} {:X}",
                r.handle
            );
            // 0, 0 — or 5, 5 once Add Automatic Widening was tried (S278).
            assert_eq!(info.tail[0], info.tail[1]);
            if let Some(p) = offset_parent(&o.doc, r) {
                let parent = recs.iter().find(|x| x.handle == p).unwrap();
                let ivs = intervals_of(parent);
                for (st, g) in &info.anchors {
                    let hit = ivs.iter().find(|(_, _, ig)| ig == g);
                    // S258's 127B, a copy of 1234 with its parent copied:
                    // the parent copy has the inner anchors (same GUIDs as
                    // the original's) but not the start and end ones.
                    if hit.is_none()
                        && (path, r.handle) == ("samples/S258_widening_curves.dwg", 0x127B)
                    {
                        assert!(*st == 0.0 || (st - info.parent_end).abs() < 1e-9);
                        continue;
                    }
                    let (_, s, _) = hit.unwrap_or_else(|| {
                        panic!("{path} {:X}: anchor {st} not on the parent", r.handle)
                    });
                    assert!((s - st).abs() < 1e-6, "{path} {:X}", r.handle);
                }
            }
            checked += 1;
        }
    }
    assert!(checked >= 20, "{checked}");
}

/// S274 (S260 saved by civil3drust) and its Civil 3D re-save S275 read
/// back as S260; the networks are still there.
#[test]
fn s274_roundtrip_keeps_offset() {
    for path in [
        "samples/S274_S260_roundtrip.dwg",
        "samples/S275_S274_civ3d_resaved.dwg",
    ] {
        let inv = inventory(path);
        for (h, c) in [
            (0x8D73, "Associative"),
            (0x8D75, "Associative"),
            (0x8D76, "AECC_ASSOC_ACTION"),
        ] {
            assert_eq!(inv.get(&h).map(String::as_str), Some(c), "{path} {h:X}");
        }
        let o = raw::open(path).unwrap();
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == 0x8D71)
            .unwrap();
        let i = offset_info(&r).unwrap().unwrap();
        assert_eq!(
            (i.head, i.left_offset(), i.parent_end),
            ([1, 2, 1], 10.0, 1000.0)
        );
        assert_eq!(offset_parent(&o.doc, &r), Some(0x8C4A));
    }
}

/// S276 → S281 (owner): a widening added to an offset of a tangent – arc –
/// tangent parent, its offset changed to 30, its transition in changed to
/// curve – reverse curve, deleted; then two widenings.
#[test]
fn s276_s281_widening_steps() {
    use civil3drust::aecc_elements::Element;
    let steps: [(&str, usize, Vec<i32>, Vec<i32>); 6] = [
        ("samples/S276_offset_arc_base.dwg", 1, vec![], vec![7, 8, 9]),
        (
            "samples/S277_S276_widening_added.dwg",
            3,
            vec![2, 2],
            vec![7, 12, 10, 13, 11, 8, 9],
        ),
        (
            "samples/S278_S277_widening_30.dwg",
            3,
            vec![2, 2],
            vec![7, 12, 10, 13, 11, 8, 9],
        ),
        (
            "samples/S279_S278_curve_reverse_in.dwg",
            3,
            vec![5, 2],
            vec![7, 14, 10, 13, 11, 8, 9],
        ),
        // Deleting the widening brings back the elements as before (7, 8, 9).
        (
            "samples/S280_S279_widening_deleted.dwg",
            1,
            vec![],
            vec![7, 8, 9],
        ),
        (
            "samples/S281_S280_two_widenings.dwg",
            5,
            vec![2, 2, 2, 2],
            vec![7, 17, 15, 18, 16, 8, 21, 19, 9, 22, 20],
        ),
    ];
    for (path, n_regions, kinds, chain) in steps {
        let o = raw::open(path).unwrap();
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == 0x8DB6)
            .unwrap();
        let info = offset_info(&r).unwrap().unwrap();
        assert_eq!(info.regions.len(), n_regions, "{path}");
        let t: Vec<i32> = info
            .transitions
            .as_ref()
            .unwrap()
            .iter()
            .map(|t| t.kind)
            .collect();
        assert_eq!(t, kinds, "{path}");
        let d = decode_elements(&r).unwrap();
        let mut ids: Vec<i32> = d.elements.iter().map(|e| e.record.id()).collect();
        ids.dedup();
        assert_eq!(ids, chain, "{path}");
        let st = element_stations(&d.elements, 0.0);
        for g in &info.regions {
            let (a, b) = (st[&g.first_element].0, st[&g.last_element].1);
            assert!((a - g.start).abs() < 1e-6, "{path}: {a} vs {}", g.start);
            assert!((b - g.end).abs() < 1e-6, "{path}: {b} vs {}", g.end);
        }
        let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
        assert_eq!(stray_vertices(&r, &segs), 0, "{path}");
        // The curve – reverse curve transition's derived reverse radius:
        // 235 as Civil 3D shows.
        if path.starts_with("samples/S279") {
            let rev = d
                .elements
                .iter()
                .filter(|e| matches!(e.record, Element::Widening(_)) && e.record.id() == 14)
                .nth(1)
                .unwrap();
            let Segment::Arc { geometry: g, .. } = rev.segment else {
                panic!()
            };
            assert!((g.radius - 235.0).abs() < 1e-6, "{}", g.radius);
        }
    }
}
