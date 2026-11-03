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

/// S162: fixed spirals of every type after a line from (x0, 0) to
/// (x0 + 100, 0); expected ends are the last points of the spiral Civil
/// 3D drew in each record's proxy graphics.
#[test]
fn s162_spirals_match_civil3d_drawing() {
    use civil3drust::aecc_elements::{SPIRAL_COMPOUND, SPIRAL_INWARD, SPIRAL_OUTWARD};
    let o = raw::open("samples/S162_S120_free_spirals.dwg").unwrap();
    let inf = f64::INFINITY;
    // handle, type, clockwise, length, start R, end R, drawn end
    let cases = [
        (
            0xFD7,
            SPIRAL_INWARD,
            true,
            200.0,
            inf,
            200.0,
            (295.0575, -32.7428),
        ),
        (
            0x1071,
            SPIRAL_INWARD,
            false,
            120.0,
            inf,
            300.0,
            (619.5209, 7.9772),
        ),
        (
            0x109F,
            SPIRAL_OUTWARD,
            true,
            120.0,
            300.0,
            inf,
            (918.7232, -15.927),
        ),
        (
            0x10A1,
            SPIRAL_COMPOUND,
            true,
            200.0,
            200.0,
            300.0,
            (1275.3496, -83.5756),
        ),
        (
            0x10A3,
            SPIRAL_INWARD,
            false,
            118.1325,
            inf,
            45.8811,
            (1400.0, 45.0),
        ),
        (
            0x10AE,
            SPIRAL_OUTWARD,
            false,
            111.6352,
            88.1522,
            inf,
            (1400.0, 45.0),
        ),
    ];
    for (h, kind, cw, l, r0, r1, end) in cases {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let d = decode_elements(&r).unwrap();
        let Element::Spiral(sp) = &d.elements[1].record else {
            panic!("{h:X}: no spiral record")
        };
        assert_eq!((sp.kind, sp.clockwise, sp.mode), (kind, cw, 256), "{h:X}");
        let close = |a: f64, b: f64| (a == b) || (a - b).abs() < 1e-4;
        assert!(
            close(sp.length, l) && close(sp.start_radius, r0) && close(sp.end_radius, r1),
            "{h:X}"
        );
        assert_eq!(sp.tail, [0.0, 0.0, 1.0]);
        let e = d.elements[1].segment.end();
        assert!(near(e, end.0, end.1), "{h:X}: {e:?}");
        assert!(near(d.elements[0].segment.end(), sp.start.x, sp.start.y));
    }
}

/// S165: free spirals between two lines (incurve and outcurve, L = 800)
/// and SPIRAL C edited to settle the compound spiral's field order; the
/// expected points are where Civil 3D's drawn spiral starts and ends.
#[test]
fn s165_free_spirals_and_compound_order() {
    let o = raw::open("samples/S165_S120_free_spiral.dwg").unwrap();
    let get = |h: u64| {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        decode_elements(&r).unwrap()
    };
    for (h, start, end, inward) in [
        (0xFDA, (3047.9642, -422.1033), (3757.5827, -352.068), true),
        (
            0x10C5,
            (5857.3832, -263.5831),
            (6496.9412, -578.9068),
            false,
        ),
    ] {
        let d = get(h);
        assert_eq!(d.record_count(), 3);
        let Element::FreeSpiral(sp) = &d.elements[1].record else {
            panic!("{h:X}: no free spiral")
        };
        assert_eq!(sp.length, 800.0);
        let Segment::Spiral {
            start: s,
            end: e,
            geometry: g,
        } = d.elements[1].segment
        else {
            panic!("{h:X}: not a spiral segment")
        };
        assert!(near(s, start.0, start.1) && near(e, end.0, end.1), "{h:X}");
        let r = if inward { g.radius } else { g.start_radius };
        assert!((r - 250.631).abs() < 1e-3 && !g.counter_clockwise);
        // Both lines meet the spiral.
        assert!(near(d.elements[0].segment.end(), start.0, start.1));
        assert!(near(d.elements[2].segment.start(), end.0, end.1));
    }
    let d = get(0x10A1);
    let Element::Spiral(sp) = &d.elements[1].record else {
        panic!("no spiral")
    };
    assert_eq!(
        (sp.start_radius, sp.length, sp.end_radius),
        (200.0, 800.0, 250.0)
    );
    assert!(near(d.elements[1].segment.end(), 965.1484, -411.2572));
}

/// S173: spiral groups (one record, several parts). Expected points are
/// the joints of the path Civil 3D drew in each record's proxy graphics.
#[test]
fn s173_spiral_groups_match_civil3d_drawing() {
    let o = raw::open("samples/S173_S120_scs_floating_spirals.dwg").unwrap();
    let joints = |h: u64| -> Vec<(f64, f64)> {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let d = decode_elements(&r).unwrap();
        let mut v = vec![d.elements[0].segment.start()];
        v.extend(d.elements.iter().map(|e| e.segment.end()));
        v.into_iter().map(|p| (p.x, p.y)).collect()
    };
    let cases: [(u64, &[(f64, f64)]); 4] = [
        // Free Spiral-Curve-Spiral: line, TS, SC, CS, ST, line end.
        (
            0xFC5,
            &[
                (511.0, 803.0),
                (1029.5984, 1291.2202),
                (1144.8417, 1386.8694),
                (1535.443, 1409.0588),
                (1697.6567, 1293.0199),
                (2205.0, 862.0),
            ],
        ),
        // Floating Curve with Spiral (radius, length).
        (
            0xFB9,
            &[
                (428.0, 2008.0),
                (1113.5, 2176.5),
                (1494.4754, 2142.3444),
                (1241.06, 1834.1561),
            ],
        ),
        // Floating Curve with Spiral (radius, through point): line trimmed.
        (
            0xFBC,
            &[
                (1809.5, 1752.5),
                (1971.3346, 1820.2081),
                (2353.0101, 1845.3614),
                (2400.0, 1800.0),
            ],
        ),
        // Floating Line with Spiral (from curve end, length).
        (
            0xFB6,
            &[
                (2377.5, 1495.0),
                (3051.5, 1554.0),
                (3122.3887, 1740.4594),
                (3233.9661, 2124.5824),
            ],
        ),
    ];
    for (h, want) in cases {
        let got = joints(h);
        assert_eq!(got.len(), want.len(), "{h:X}: {got:?}");
        for (g, w) in got.iter().zip(want) {
            assert!(
                (g.0 - w.0).abs() < 1e-3 && (g.1 - w.1).abs() < 1e-3,
                "{h:X}: {got:?}"
            );
        }
    }
}

/// S174: floating reverse curves with spirals. The group starts on the
/// previous (three-point) curve, which it trims; joints as Civil 3D drew.
#[test]
fn s174_reverse_curves_with_spirals() {
    let o = raw::open("samples/S174_S120_reverse_curves_spirals.dwg").unwrap();
    type Case<'a> = (u64, &'a [(f64, f64)], f64);
    let cases: [Case; 2] = [
        (
            0xFBF,
            &[
                (1985.0, 2454.5),
                (2234.7204, 2224.9383),
                (2823.7016, 2901.8291),
                (3095.108, 3280.8012),
                (3200.0, 2900.0),
            ],
            200.0,
        ),
        (
            0xFC2,
            &[
                (3629.0, 2350.5),
                (3817.033, 2103.6556),
                (4442.6995, 2415.7895),
                (4898.6665, 2876.5096),
                (5900.0, 3100.0),
            ],
            1302.9691,
        ),
    ];
    for (h, want, r2) in cases {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let d = decode_elements(&r).unwrap();
        let mut got = vec![d.elements[0].segment.start()];
        got.extend(d.elements.iter().map(|e| e.segment.end()));
        assert_eq!(got.len(), want.len(), "{h:X}");
        for (g, w) in got.iter().zip(want) {
            assert!(near(*g, w.0, w.1), "{h:X}: {got:?}");
        }
        let Segment::Arc { geometry, .. } = d.elements[3].segment else {
            panic!("{h:X}: no second curve")
        };
        assert!((geometry.radius - r2).abs() < 1e-3 && !geometry.counter_clockwise);
    }
}

/// S175: spiral-spiral groups. Between two lines (two spirals, no curve)
/// and between two curves (compound and reverse; A1 / A2 = 1 stored, the
/// rest solved). Points as Civil 3D drew them; FE0's spirals are 8300 and
/// 3900 long, where Civil 3D's drawing differs from the exact clothoid
/// by about 0.005.
#[test]
fn s175_spiral_spiral_groups() {
    let o = raw::open("samples/S175_S120_spiral_spiral.dwg").unwrap();
    type Case<'a> = (u64, &'a [(f64, f64)], f64);
    let cases: [Case; 3] = [
        (
            0xFE3,
            &[
                (6305.5, 98.5),
                (7355.6223, 1256.1543),
                (7760.0423, 1530.425),
                (8280.3228, 1273.0499),
                (9416.0, 226.5),
            ],
            1e-3,
        ),
        (
            0xFDD,
            &[
                (4338.0, 654.0),
                (4505.7253, 128.5308),
                (4914.5125, 1230.8547),
                (5954.3616, 1239.3772),
                (5724.0, 512.5),
            ],
            1e-3,
        ),
        (
            0xFE0,
            &[
                (12300.0, 803.0),
                (13032.9913, 2342.632),
                (17598.41, -441.9337),
                (21418.0683, 99.8103),
                (19152.5, -104.0),
            ],
            1e-2,
        ),
    ];
    for (h, want, tol) in cases {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let d = decode_elements(&r).unwrap();
        let mut got = vec![d.elements[0].segment.start()];
        got.extend(d.elements.iter().map(|e| e.segment.end()));
        assert_eq!(got.len(), want.len(), "{h:X}");
        for (g, w) in got.iter().zip(want) {
            assert!(
                (g.x - w.0).abs() < tol && (g.y - w.1).abs() < tol,
                "{h:X}: {got:?}"
            );
        }
    }
}

/// S176: spiral-line-spiral between two curves, by spiral lengths (line
/// solved) and by line length (spirals with equal A solved), compound and
/// reverse. Points as drawn; FEC's 12 000-long spiral is drawn ~0.02 off
/// the exact clothoid.
#[test]
fn s176_spiral_line_spiral() {
    let o = raw::open("samples/S176_S120_spiral_line_spiral.dwg").unwrap();
    type Case<'a> = (u64, &'a [(f64, f64)], f64, f64);
    let cases: [Case; 4] = [
        (
            0xFE6,
            &[
                (13577.164, 10119.7097),
                (13971.4112, 10186.5308),
                (21194.3437, 11623.7236),
                (21389.2657, 11668.2235),
            ],
            7364.5283,
            1e-3,
        ),
        (
            0xFE9,
            &[
                (13280.7034, 30726.3535),
                (13450.3818, 30832.2115),
                (18356.1288, 33962.3894),
                (18699.3002, 34167.6363),
            ],
            5819.3099,
            1e-3,
        ),
        (
            0xFEC,
            &[
                (10079.6724, 7037.6971),
                (15515.8504, 3908.092),
                (16102.7634, 3783.4599),
                (21539.4255, 8760.212),
            ],
            600.0,
            3e-2,
        ),
        (
            0xFEF,
            &[
                (13923.2411, 19350.8108),
                (16921.1568, 22470.8383),
                (17120.7114, 22929.29),
                (19468.3668, 25893.388),
            ],
            500.0,
            1e-3,
        ),
    ];
    for (h, want, line, tol) in cases {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let d = decode_elements(&r).unwrap();
        assert_eq!(d.record_count(), 3, "{h:X}");
        // Group parts: spiral, line, spiral (elements 1..=3).
        let got: Vec<Point2> = std::iter::once(d.elements[1].segment.start())
            .chain(d.elements[1..4].iter().map(|e| e.segment.end()))
            .collect();
        for (g, w) in got.iter().zip(want) {
            assert!(
                (g.x - w.0).abs() < tol && (g.y - w.1).abs() < tol,
                "{h:X}: {got:?}"
            );
        }
        assert!(
            (d.elements[2].segment.length() - line).abs() < 1e-3,
            "{h:X}"
        );
        // Both curves are trimmed to the group.
        assert_eq!(d.elements[0].segment.end(), got[0]);
        assert_eq!(d.elements[4].segment.start(), got[3]);
    }
}

/// S177: the five long spiral groups. SCSCS and SCSSCS between tangents are
/// group records (kinds 24, 26) placed from a stored TS; the three others
/// are stored by Civil 3D as separate elements, the first spiral with
/// header 0x0D020008, plus a JSON layout string. Joints as drawn.
#[test]
fn s177_long_spiral_groups() {
    use civil3drust::string_stream;
    let o = raw::open("samples/S177_long_spiral_method_alignments.dwg").unwrap();
    type Case<'a> = (u64, &'a str, usize, &'a [(f64, f64)]);
    let cases: [Case; 5] = [
        (
            0xFC8,
            "Free Compound Spiral-Curve-Spiral-Curve-Spiral (Between two tangents)",
            3,
            &[
                (868.5, 0.0),
                (1018.3682, 4.6846),
                (1089.2732, 14.5688),
                (1185.5983, 40.8748),
                (1266.2954, 83.405),
                (1378.4615, 182.6449),
            ],
        ),
        (
            0xFCB,
            "Free Reverse Spiral-Curve-Spiral-Spiral-Curve-Spiral (Between two tangents)",
            3,
            &[
                (1000.0, 0.0),
                (1149.7658, 6.243),
                (1340.4334, 63.4869),
                (1596.8707, 218.8276),
                (1842.8918, 292.6902),
                (1992.7197, 298.043),
            ],
        ),
        (
            0xFCE,
            "Reverse S-S-C-S-S (between two curves)",
            7,
            &[
                (293.0136, -55.5923),
                (428.8323, -119.1195),
                (565.2636, -181.2103),
            ],
        ),
        (
            0xFD1,
            "Free Reverse Spiral-Curve-Spiral-Curve (From curve, end entity)",
            6,
            &[(293.0139, -55.5924), (910.7258, -205.3593)],
        ),
        (
            0xFD4,
            "Free Reverse Spiral-Curve-Spiral-Spiral (From entity_ end curve)",
            6,
            &[
                (999.9963, 0.0),
                (1468.8654, 140.7668),
                (1596.5307, 219.4052),
            ],
        ),
    ];
    for (h, name, records, want) in cases {
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let d = decode_elements(&r).unwrap();
        assert_eq!(d.name.as_deref(), Some(name), "{h:X}");
        assert_eq!(d.record_count(), records, "{h:X}");
        let joints: Vec<Point2> = d.elements.iter().map(|e| e.segment.end()).collect();
        for w in want {
            assert!(
                joints.iter().any(|j| near(*j, w.0, w.1)),
                "{h:X}: {w:?} not in {joints:?}"
            );
        }
        let json = string_stream::layout_json(&r.data, r.handle_bits);
        assert_eq!(json.is_some(), records > 3, "{h:X}");
    }
}
