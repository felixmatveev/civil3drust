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

#[test]
fn s289_reverse_spiral_spiral_curve_spiral_spiral() {
    // Between two curves (both inputs now have curves): the middle is the
    // centerline group 2569, solved as before; the graphics lie on it.
    let o = raw::open("samples/S289_connected_reverse_sscss.dwg").unwrap();
    let recs = raw::records(&o.doc, "AECC_ALIGNMENT");
    let r = recs.iter().find(|r| r.handle == 0x8F9A).unwrap();
    let d = decode_elements(r).unwrap();
    assert!(d.is_complete());
    assert_eq!(d.alignment_type, Some(3));
    assert_eq!(d.layout_groups[0].constraint_type, 2569);
    assert!((d.length() - 497.5844).abs() < 1e-4, "{}", d.length());
    let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
    assert_eq!(segs.len(), 7);
    let Segment::Arc { geometry: g, .. } = segs[3] else {
        panic!()
    };
    assert!((g.radius - 100.0).abs() < 1e-9);
    assert!((g.sweep.to_degrees() - 107.1006).abs() < 1e-4);
    assert_eq!(stray_vertices(r, &segs), 0);
    assert_eq!(connected_inputs(r), Some(vec![0x8C4A, 0x8E11]));
}

fn decoded(
    path: &str,
) -> Vec<(
    u64,
    Result<civil3drust::aecc_elements::DecodedElements, String>,
)> {
    let o = raw::open(path).unwrap();
    raw::records(&o.doc, "AECC_ALIGNMENT")
        .iter()
        .map(|r| (r.handle, decode_elements(r).map_err(|e| e.to_string())))
        .collect()
}

fn segments_of(
    all: &[(
        u64,
        Result<civil3drust::aecc_elements::DecodedElements, String>,
    )],
    h: u64,
) -> Vec<Segment> {
    let d = all
        .iter()
        .find(|(x, _)| *x == h)
        .unwrap()
        .1
        .as_ref()
        .unwrap();
    d.elements.iter().map(|e| e.segment).collect()
}

fn moved(s: &Segment, dx: f64) -> Segment {
    let m = |p: civil3drust::Point2| civil3drust::Point2::new(p.x + dx, p.y);
    match *s {
        Segment::Line { start, end } => Segment::Line {
            start: m(start),
            end: m(end),
        },
        Segment::Arc {
            start,
            end,
            mut geometry,
        } => {
            geometry.center = m(geometry.center);
            Segment::Arc {
                start: m(start),
                end: m(end),
                geometry,
            }
        }
        Segment::Spiral {
            start,
            end,
            mut geometry,
        } => {
            geometry.start = m(geometry.start);
            Segment::Spiral {
                start: m(start),
                end: m(end),
                geometry,
            }
        }
    }
}

fn close(a: &Segment, b: &Segment) -> bool {
    let d = |p: civil3drust::Point2, q: civil3drust::Point2| p.distance_to(q) < 1e-6;
    d(a.start(), b.start()) && d(a.end(), b.end()) && (a.length() - b.length()).abs() < 1e-6
}

const BASE: &str = "samples/S287_S286_connected_static.dwg";
const DYNAMIC: [u64; 3] = [0x8E82, 0x8E98, 0x8EA5];
const STATIC: u64 = 0x8EC8;

#[test]
fn s290_input_moved_east() {
    // NORTH-SOUTH moved 20' east: the dynamic connected alignments are
    // rebuilt 20' east, the static one stays.
    let (a, b) = (
        decoded(BASE),
        decoded("samples/S290_S287_input_moved_east.dwg"),
    );
    for h in DYNAMIC {
        let (x, y) = (segments_of(&a, h), segments_of(&b, h));
        assert_eq!(x.len(), y.len());
        for (s, t) in x.iter().zip(&y) {
            assert!(close(&moved(s, 20.0), t), "{h:X}");
        }
    }
    assert_eq!(segments_of(&a, STATIC), segments_of(&b, STATIC));
}

#[test]
fn s291_input_start_station() {
    // WEST-EAST's start station 0 → 1000+00: the connected alignments keep
    // their geometry (the block's stations on WEST-EAST follow).
    let (a, b) = (
        decoded(BASE),
        decoded("samples/S291_S287_input_start_station.dwg"),
    );
    for h in DYNAMIC.into_iter().chain([STATIC]) {
        let (x, y) = (segments_of(&a, h), segments_of(&b, h));
        assert!(x.iter().zip(&y).all(|(s, t)| close(s, t)), "{h:X}");
    }
}

#[test]
fn s292_input_radius() {
    // WEST-EAST's curve grows (R 544.62 → 2222.48) and reaches the
    // connections: the fillet and the three centered arc start with a
    // fixed curve 25' off the input's curve instead of a tangent; the
    // spiral – curve – spiral is left without elements; the static one
    // stays.
    let (a, b) = (decoded(BASE), decoded("samples/S292_S287_input_radius.dwg"));
    for (h, len) in [(0x8E82, 292.9941), (0x8EA5, 123.6059)] {
        let d = b.iter().find(|(x, _)| *x == h).unwrap().1.as_ref().unwrap();
        assert!((d.length() - len).abs() < 1e-4, "{h:X} {}", d.length());
        let segs = segments_of(&b, h);
        let Segment::Arc { geometry: g, .. } = segs[0] else {
            panic!("{h:X}")
        };
        assert!((g.radius - 2247.4846).abs() < 1e-4);
        // Each joint is tangent: the direction carries over.
        let h_ = 1e-4;
        for w in segs.windows(2) {
            let (l0, l1) = (w[0].length(), w[1].length());
            let (p0, p1) = (point_on(&w[0], l0 - h_), point_on(&w[0], l0));
            let (q0, q1) = (point_on(&w[1], 0.0), point_on(&w[1], h_.min(l1)));
            let p = ((p1.x - p0.x) / h_, (p1.y - p0.y) / h_);
            let q = ((q1.x - q0.x) / h_, (q1.y - q0.y) / h_);
            assert!(
                (p.0 - q.0).abs() < 1e-3 && (p.1 - q.1).abs() < 1e-3,
                "{h:X} {p:?} {q:?}"
            );
            assert!(p1.distance_to(q0) < 1e-6, "{h:X}");
        }
    }
    let scs = &b.iter().find(|(x, _)| *x == 0x8E98).unwrap().1;
    assert!(scs
        .as_ref()
        .is_err_and(|e| e.contains("no element records")));
    assert_eq!(segments_of(&a, STATIC), segments_of(&b, STATIC));
}

#[test]
fn s293_connection_offset_and_overlap() {
    // Spiral-curve-spiral: offset in 25 → 30, overlap in 25 → 40, overlap
    // out 25 → 50 (owner's screenshot).
    let b = decoded("samples/S293_S287_connection_offset_overlap.dwg");
    let segs = segments_of(&b, 0x8E98);
    assert!((segs[0].length() - 40.0).abs() < 1e-9);
    assert!((segs[0].start().y - 30.0).abs() < 1e-9);
    assert!((segs[4].length() - 50.0).abs() < 1e-9);
    let total: f64 = segs.iter().map(Segment::length).sum();
    assert!((total - 525.6194).abs() < 1e-4);
}

#[test]
fn s294_audit_keeps_the_static_alignment() {
    let (a, b) = (
        decoded(BASE),
        decoded("samples/S294_S287_audit_resaved.dwg"),
    );
    for h in DYNAMIC.into_iter().chain([STATIC]) {
        let (x, y) = (segments_of(&a, h), segments_of(&b, h));
        assert!(x.iter().zip(&y).all(|(s, t)| close(s, t)), "{h:X}");
    }
}
