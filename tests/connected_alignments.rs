//! Connected (curb return, type 3) alignments (S282 → S283, owner).

use civil3drust::aecc_alignment_refs::{alignment_refs, connected_inputs, offset_parent};
use civil3drust::aecc_build::alignment_type;
use civil3drust::aecc_elements::decode_elements;
use civil3drust::aecc_elements::Segment;
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

/// S286: Spiral-Curve-Spiral below and above 180°; the Three Centered Arc
/// does not decode yet.
#[test]
fn s286_curve_groups() {
    let o = raw::open("samples/S286_connected_curve_groups.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    let get = |h: u64| recs.iter().find(|r| r.handle == h).unwrap();
    for (h, len, sweep) in [(0x8E98, 485.6194, 13.6056), (0x8EC8, 881.8583, 222.2535)] {
        let d = decode_elements(get(h)).unwrap();
        assert!((d.length() - len).abs() < 1e-4, "{h:X}");
        let civil3drust::aecc_elements::Segment::Arc { geometry: g, .. } = d.elements[2].segment
        else {
            panic!()
        };
        assert!((g.radius - 150.0).abs() < 1e-9);
        assert!((g.sweep.to_degrees() - sweep).abs() < 1e-4, "{h:X}");
        assert_eq!(connected_inputs(get(h)), Some(vec![0x8C4A, 0x8E11]));
    }
    // Three Centered Arc: one prefix-22 record (kind 1) with the dialog's
    // R1 75, L1 10, R2 40, R3 50, L3 10; arc 2 takes the rest of the turn
    // and the arcs meet tangent 3 where Civil 3D put it.
    let d = decode_elements(get(0x8EA5)).unwrap();
    assert!((d.length() - 119.4985).abs() < 1e-4);
    let arcs: Vec<_> = d
        .elements
        .iter()
        .filter_map(|e| match e.segment {
            civil3drust::aecc_elements::Segment::Arc { geometry: g, .. } => Some(g),
            _ => None,
        })
        .collect();
    assert_eq!(arcs.len(), 3);
    for (g, (r, deg)) in arcs
        .iter()
        .zip([(75.0, 7.6394), (40.0, 70.9014), (50.0, 11.4592)])
    {
        assert!((g.radius - r).abs() < 1e-9);
        assert!(g.counter_clockwise);
        assert!((g.sweep.to_degrees() - deg).abs() < 1e-4);
    }
    assert_eq!(connected_inputs(get(0x8EA5)), Some(vec![0x8C4A, 0x8E11]));
}

#[test]
fn s287_converted_to_static() {
    use civil3drust::aecc_alignment::Linked;
    use civil3drust::aecc_offset::connected_head;
    let a = raw::open("samples/S286_connected_curve_groups.dwg").unwrap();
    let b = raw::open("samples/S287_S286_connected_static.dwg").unwrap();
    let get = |o: &raw::Opened, h: u64| {
        raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap()
    };
    let (ra, rb) = (get(&a, 0x8EC8), get(&b, 0x8EC8));
    // Still type 3 with the same inputs; the head goes 1, 1 → 2, 2.
    assert_eq!(connected_head(&ra).unwrap(), Some([1, 1]));
    assert_eq!(connected_head(&rb).unwrap(), Some([2, 2]));
    assert_eq!(connected_inputs(&rb), Some(vec![0x8C4A, 0x8E11]));
    assert_eq!(connected_head(&get(&b, 0x8E98)).unwrap(), Some([1, 1]));
    // Same geometry; the last tangent is renumbered 3 → 2.
    let (da, db) = (decode_elements(&ra).unwrap(), decode_elements(&rb).unwrap());
    assert_eq!(da.elements.len(), db.elements.len());
    for (x, y) in da.elements.iter().zip(&db.elements) {
        assert_eq!(x.segment, y.segment);
    }
    let ids = |d: &civil3drust::aecc_elements::DecodedElements| {
        d.elements.iter().map(|e| e.id()).collect::<Vec<_>>()
    };
    assert_eq!(ids(&da), [1, 4, 5, 6, 3]);
    assert_eq!(ids(&db), [1, 4, 5, 6, 2]);
    // Converted through the Alignment Properties form (user string); the
    // curve-group JSON in the JSON slot is not the name.
    assert_eq!(
        raw::section2_form(&rb.data),
        Some(raw::Section2Form::Properties)
    );
    assert_eq!(
        raw::object_name(&rb.data).as_deref(),
        Some("spiral - curve - spiral more than 180deg")
    );
}

#[test]
fn s288_reverse_spiral_groups() {
    // Connected alignments from WEST-EAST's 25' offset curve to
    // NORTH-SOUTH's tangent: their middle is a JSON layout group (2562
    // reverse spiral – spiral – curve – spiral, 2566 spiral – curve –
    // spiral – reverse spiral), solved like 2561 and 2565.
    let o = raw::open("samples/S288_connected_reverse_spirals.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    let get = |h: u64| recs.iter().find(|r| r.handle == h).unwrap();
    for (h, ty, len, sweep, name) in [
        (
            0x8F18,
            2562,
            1058.9055,
            213.0478,
            "REVERSE SPIRAL SPIRAL CURVE SPIRAL",
        ),
        (
            0x8F32,
            2566,
            310.8924,
            228.4803,
            "REVERSE SPIRAL - CURVE - SPIRAL SPIRAL",
        ),
    ] {
        let r = get(h);
        let d = decode_elements(r).unwrap();
        assert!(d.is_complete(), "{h:X}");
        assert_eq!(d.layout_groups[0].constraint_type, ty);
        assert!((d.length() - len).abs() < 1e-4, "{h:X} {}", d.length());
        let big = d
            .elements
            .iter()
            .filter_map(|e| match e.segment {
                Segment::Arc { geometry: g, .. } => Some(g.sweep.to_degrees()),
                _ => None,
            })
            .fold(0.0, f64::max);
        assert!((big - sweep).abs() < 1e-4, "{h:X} {big}");
        let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
        assert_eq!(stray_vertices(r, &segs), 0, "{h:X}");
        assert_eq!(raw::object_name(&r.data).as_deref(), Some(name));
        assert_eq!(connected_inputs(r).map(|v| v.len()), Some(2));
    }
}
