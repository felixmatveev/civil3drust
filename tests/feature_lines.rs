//! Feature lines (S313–S318; docs/feature-line-object.md).

use civil3drust::aecc_feature_line::{feature_lines, vertices, FeatureVertex};
use civil3drust::aecc_site::sites;
use civil3drust::raw;

fn open(p: &str) -> acadrust::document::CadDocument {
    raw::open(p).unwrap().doc
}

fn xyz(v: &[FeatureVertex]) -> Vec<(i32, f64, f64, f64)> {
    v.iter().map(|v| (v.node, v.x, v.y, v.z)).collect()
}

fn close(a: &[(i32, f64, f64, f64)], b: &[(i32, f64, f64, f64)]) {
    assert_eq!(a.len(), b.len(), "{a:?} vs {b:?}");
    for (p, q) in a.iter().zip(b) {
        assert_eq!(p.0, q.0, "{a:?} vs {b:?}");
        for (s, t) in [(p.1, q.1), (p.2, q.2), (p.3, q.3)] {
            assert!((s - t).abs() < 1e-4, "{a:?} vs {b:?}");
        }
    }
}

/// The empty site has no feature line; S314 adds one with three points
/// whose elevations are the bounding box's 100–105.
#[test]
fn three_points_s313_s314() {
    assert!(feature_lines(&open("samples/S313_fl_empty.dwg")).is_empty());
    let d = open("samples/S314_fl_3pts.dwg");
    let fl = feature_lines(&d);
    assert_eq!(fl.len(), 1);
    assert_eq!(fl[0].name, "TEST FEATURE LINE");
    assert_eq!(fl[0].description, "3 points");
    assert_eq!(fl[0].container, Some(0x8C43));
    let v = vertices(&d, &fl[0]).unwrap();
    close(
        &xyz(&v),
        &[
            (1, 0.0, 0.0, 100.0),
            (2, 100.0, 0.0, 105.0),
            (3, 177.8307, 49.8409, 103.0),
        ],
    );
    assert!(v.iter().all(|v| v.bulge.is_none()));
}

/// The fillet replaces the middle PI (node 2) by its tangent points 4 and
/// 5; the arc is a bulge on node 4 whose turn equals the deflection, and
/// its radius is 10 (from the bulge and the chord).
#[test]
fn fillet_s315() {
    let d = open("samples/S315_fl_curve.dwg");
    let fl = &feature_lines(&d)[0];
    let v = vertices(&d, fl).unwrap();
    assert_eq!(v.iter().map(|v| v.node).collect::<Vec<_>>(), [1, 4, 5, 3]);
    let b = v[1].bulge.unwrap();
    let delta = 4.0 * b.atan();
    let deflection = (49.84087518742308f64).atan2(177.83072233200073 - 100.0);
    assert!((delta - deflection).abs() < 1e-9);
    let chord = ((v[2].x - v[1].x).powi(2) + (v[2].y - v[1].y).powi(2)).sqrt();
    let radius = chord / (2.0 * (delta / 2.0).sin());
    assert!((radius - 10.0).abs() < 1e-3, "{radius}");
}

/// S316: the fillet's end at elevation 200 and an elevation point (node
/// 6) on the last tangent; the edge then lists one entry after its
/// vertices.
#[test]
fn elevation_point_s316() {
    let d = open("samples/S316_fl_edit.dwg");
    let fl = &feature_lines(&d)[0];
    let v = vertices(&d, fl).unwrap();
    assert_eq!(
        v.iter().map(|v| v.node).collect::<Vec<_>>(),
        [1, 4, 5, 6, 3]
    );
    assert_eq!(v[2].z, 200.0);
    assert!(v[3].is_elevation_point());
    assert!(!v.iter().any(|v| v.node != 6 && v.is_elevation_point()));
    // On the line from the fillet's end to the last point.
    let (a, p, c) = (v[2], v[3], v[4]);
    let cross = (c.x - a.x) * (p.y - a.y) - (c.y - a.y) * (p.x - a.x);
    assert!(cross.abs() < 1e-6);
}

/// The paste (S317) put the copy in a new site `TEST|1` with its own
/// container; the copy's nodes are the original's moved by one vector, and
/// the elevation point's node is renumbered 6 → 8. S318 renames the site
/// to `TEST1` and changes nothing else.
#[test]
fn pasted_copy_s317_s318() {
    for p in [
        "samples/S317_fl_pasted.dwg",
        "samples/S318_fl_pasted_site_renamed.dwg",
    ] {
        let d = open(p);
        let s = sites(&d);
        let names: Vec<&str> = s.iter().map(|s| s.name.as_str()).collect();
        let renamed = p.contains("S318");
        assert_eq!(
            names,
            ["TEST", if renamed { "TEST1" } else { "TEST|1" }],
            "{p}"
        );
        let fl = feature_lines(&d);
        assert_eq!(fl.len(), 2);
        let (a, b) = (&fl[0], &fl[1]);
        assert_eq!(a.container, Some(0x8C43));
        assert_eq!(b.container, Some(0x8DA9));
        assert_eq!(b.name, "TEST FEATURE LINE (1) (1)");
        assert_eq!(s[1].container, Some(0x8DA9));
        let (va, vb) = (vertices(&d, a).unwrap(), vertices(&d, b).unwrap());
        assert_eq!(
            vb.iter().map(|v| v.node).collect::<Vec<_>>(),
            [1, 4, 5, 8, 3]
        );
        let (dx, dy) = (vb[0].x - va[0].x, vb[0].y - va[0].y);
        assert!((dx + 0.2849).abs() < 1e-4 && (dy - 57.0468).abs() < 1e-4);
        for (p, q) in va.iter().zip(&vb) {
            assert!((q.x - p.x - dx).abs() < 1e-9 && (q.y - p.y - dy).abs() < 1e-9);
            assert!((q.z - p.z).abs() < 1e-9);
            assert_eq!(p.bulge, q.bulge);
        }
    }
}
