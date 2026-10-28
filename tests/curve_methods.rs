//! S117: one curve per Civil 3D curve layout method (owner, from S116).
//! Expected geometry is what Civil 3D drew (the record's proxy graphics)
//! and what the owner typed (alignment descriptions).

use civil3drust::aecc_elements::{decode_elements, ArcDef, Element, Segment};
use civil3drust::alignment::Point2;
use civil3drust::raw;

fn arc(handle: u64) -> (ArcDef, Segment, Vec<Segment>) {
    let o = raw::open("samples/S117_S116_curve_methods.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == handle)
        .unwrap();
    let d = decode_elements(&r).unwrap();
    let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
    let (def, seg) = d
        .elements
        .iter()
        .find_map(|e| match &e.record {
            Element::Arc(a) => Some((a.def, e.segment)),
            _ => None,
        })
        .unwrap();
    (def, seg, segs)
}

fn near(a: Point2, x: f64, y: f64) -> bool {
    (a.x - x).abs() < 1e-3 && (a.y - y).abs() < 1e-3
}

fn geom(s: Segment) -> (Point2, Point2, f64, bool, f64) {
    let Segment::Arc {
        start,
        end,
        geometry: g,
    } = s
    else {
        panic!("not an arc")
    };
    (start, end, g.radius, g.counter_clockwise, g.length)
}

#[test]
fn fixed_curves_are_stored_by_three_points() {
    for h in [0xFB3, 0xFB6, 0xFB9, 0xFBC, 0xFBF] {
        assert!(matches!(arc(h).0, ArcDef::ThreePoint { .. }), "{h:X}");
    }
}

#[test]
fn centre_curves_are_whole_circles() {
    // FC2 centre (0,0) R 100; FC8 through (0,300) heading +x clockwise
    // R 100 → centre (0,200); FC5 centre (300,0) through (200,0).
    for (h, cx, cy) in [(0xFC2, 0.0, 0.0), (0xFC8, 0.0, 200.0), (0xFC5, 300.0, 0.0)] {
        let (def, seg, segs) = arc(h);
        assert_eq!(segs.len(), 1);
        let (s, e, r, ccw, l) = geom(seg);
        assert!(near(s, cx + 100.0, cy) && near(e, cx + 100.0, cy), "{h:X}");
        assert!((r - 100.0).abs() < 1e-9 && !ccw);
        assert!((l - 200.0 * std::f64::consts::PI).abs() < 1e-6);
        match def {
            ArcDef::CenterRadius { clockwise, .. } => assert!(clockwise && h != 0xFC5),
            ArcDef::CenterThrough {
                through, clockwise, ..
            } => assert!(clockwise && h == 0xFC5 && near(through, 200.0, 0.0)),
            _ => panic!("{h:X}: {def:?}"),
        }
    }
}

#[test]
fn floating_curves() {
    // From entity, radius, through point (code 13): line trimmed.
    let (_, seg, segs) = arc(0xFCB);
    assert!(near(segs[0].end(), 98.3485, 1000.0));
    assert!(near(geom(seg).1, 190.0, 1060.0));
    // From entity end, through point (code 8).
    let (def, seg, _) = arc(0xFCE);
    assert!(matches!(def, ArcDef::FloatingThrough { .. }));
    let (s, e, r, ccw, _) = geom(seg);
    assert!(near(s, 100.0, 800.0) && near(e, 190.0, 900.0) && (r - 90.5).abs() < 1e-9 && ccw);
    // From entity end, radius, length (code 21), clockwise.
    let (def, seg, _) = arc(0xFD1);
    assert!(matches!(
        def,
        ArcDef::FloatingLength {
            clockwise: true,
            ..
        }
    ));
    let (s, e, r, ccw, l) = geom(seg);
    assert!(near(s, 100.0, 700.0) && near(e, 199.7495, 607.0737));
    assert!((r - 100.0).abs() < 1e-9 && !ccw && (l - 150.0).abs() < 1e-9);
    // From entity, through point, direction (code 11): line extended.
    let (def, seg, segs) = arc(0xFD4);
    assert!(matches!(def, ArcDef::FloatingDirection { .. }));
    assert!(near(segs[0].end(), 139.5012, 1100.0));
    let (s, e, r, ccw, _) = geom(seg);
    assert!(near(s, 139.5012, 1100.0) && near(e, 250.0, 1200.0));
    assert!((r - 111.0499).abs() < 1e-3 && ccw);
}

#[test]
fn free_fillet_through_point() {
    let (def, seg, segs) = arc(0xFDA);
    assert!(matches!(def, ArcDef::FreeThrough { through, .. } if near(through, 88.0, 1558.0)));
    let (s, e, r, ccw, _) = geom(seg);
    assert!(near(s, 21.4274, 1492.3328) && near(e, 174.6492, 1587.4208));
    assert!((r - 206.7669).abs() < 1e-3 && !ccw);
    assert!(near(segs[0].end(), 21.4274, 1492.3328) && near(segs[2].start(), 174.6492, 1587.4208));
}

fn decoded_s118(handle: u64) -> Vec<Segment> {
    let o = raw::open("samples/S118_S116_ccw_curves.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == handle)
        .unwrap();
    let d = decode_elements(&r).unwrap();
    d.elements.iter().map(|e| e.segment).collect()
}

#[test]
fn s118_direction_bit_zero_is_counter_clockwise() {
    let segs = decoded_s118(0x1057);
    let (s, e, r, ccw, l) = geom(segs[1]);
    assert!(ccw && (r - 100.0).abs() < 1e-9 && (l - 100.0).abs() < 1e-9);
    assert!(near(s, 227.5, 1890.0) && near(e, 311.6471, 1935.9698));
}

#[test]
fn s118_open_centre_curves_between_lines() {
    // Centre curve, free line, centre curve, floating line to (549.5, 2061),
    // as drawn by Civil 3D.
    let segs = decoded_s118(0x1061);
    assert_eq!(segs.len(), 4);
    let (s, e, r, ccw, _) = geom(segs[0]);
    // Nothing before it: a half circle ending where the free line leaves.
    assert!(near(s, 271.8855, 2114.5102) && near(e, 204.1145, 2161.4898));
    assert!((r - 41.2311).abs() < 1e-3 && !ccw);
    let (s, e, r, ccw, _) = geom(segs[2]);
    assert!(near(s, 330.809, 2344.2544) && near(e, 536.0854, 2286.6918));
    assert!((r - 112.7841).abs() < 1e-3 && !ccw);
    assert!(near(segs[3].end(), 549.5, 2061.0));
}

#[test]
fn s119_line_spiral_group_before_centre_curve() {
    let o = raw::open("samples/S119_S118_first_circle_edited.dwg").unwrap();
    let r = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x1061)
        .unwrap();
    let d = decode_elements(&r).unwrap();
    assert_eq!(d.record_count(), 5);
    assert!(d.has_groups());
    let Element::LineSpiral(g) = &d.elements[0].record else {
        panic!("element 7 is not a line-spiral group")
    };
    assert_eq!((g.id, g.before, g.after, g.mode), (7, 0, 2, 256));
    assert!(near(g.start, 438.0, 2124.0) && (g.spiral_length - 21.5058).abs() < 1e-4);
    // As drawn: line to TS, clothoid to SC on the first circle.
    let line = d.elements[0].segment;
    assert!(matches!(line, Segment::Line { .. }) && near(line.end(), 254.4271, 2098.2015));
    let Segment::Spiral { end, geometry, .. } = d.elements[1].segment else {
        panic!("no spiral part")
    };
    assert!(near(end, 233.016, 2097.0713) && !geometry.counter_clockwise);
    let (s, e, ..) = geom(d.elements[2].segment);
    assert!(near(s, 233.016, 2097.0713) && near(e, 204.1145, 2161.4898));
    assert!((d.length() - 1019.3793).abs() < 1e-3);
}
