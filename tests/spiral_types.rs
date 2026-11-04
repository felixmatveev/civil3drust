//! Spiral types (S232, owner): one alignment per type, a 100' tangent and
//! an outward spiral (R 200 → ∞, L 200). The MIST10 code names the type
//! and each type's shape matches Civil 3D's proxy graphics.

use civil3drust::aecc_elements::{decode_elements, Element, Segment};
use civil3drust::proxy_graphics as pg;
use civil3drust::raw;
use civil3drust::spiral_type::SpiralType;

/// Largest distance from a graphics vertex to the decoded geometry
/// (sampled every 0.01).
fn worst_vertex(r: &raw::RawRecord, segs: &[Segment]) -> f64 {
    let g = pg::find(&r.data, 4096).unwrap();
    let mut samples = Vec::new();
    for s in segs {
        match s {
            Segment::Spiral { geometry, .. } => {
                let n = (geometry.length / 0.01) as usize;
                for i in 0..=n {
                    let (p, _) = geometry.point_at(geometry.length * i as f64 / n as f64);
                    samples.push(p);
                }
            }
            other => {
                samples.push(other.start());
                samples.push(other.end());
            }
        }
    }
    let line_dist = |p: [f64; 3]| {
        segs.iter()
            .filter_map(|s| match s {
                Segment::Line { start, end } => {
                    let (dx, dy) = (end.x - start.x, end.y - start.y);
                    let t = (((p[0] - start.x) * dx + (p[1] - start.y) * dy) / (dx * dx + dy * dy))
                        .clamp(0.0, 1.0);
                    Some((p[0] - start.x - t * dx).hypot(p[1] - start.y - t * dy))
                }
                _ => None,
            })
            .fold(f64::MAX, f64::min)
    };
    g.chunks
        .iter()
        .filter(|c| c.opcode == pg::OP_POLYLINE)
        .flat_map(|c| pg::poly_vertices(&r.data, c).into_iter().map(|v| v.1))
        .map(|p| {
            let near = samples
                .iter()
                .map(|q| (p[0] - q.x).hypot(p[1] - q.y))
                .fold(f64::MAX, f64::min);
            near.min(line_dist(p))
        })
        .fold(0.0, f64::max)
}

#[test]
fn s232_codes_and_shapes() {
    let o = raw::open("samples/S232_spiral_types.dwg").unwrap();
    let mut seen = Vec::new();
    for r in raw::records(&o.doc, "AECC_ALIGNMENT") {
        let name = raw::object_name(&r.data).unwrap_or_default();
        let d = decode_elements(&r).unwrap();
        let spiral = d
            .elements
            .iter()
            .find_map(|e| match &e.record {
                Element::Spiral(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap();
        let ty = SpiralType::from_code(spiral.mode).unwrap();
        // The copied alignment whose type was changed in the settings
        // keeps the Bi-Quadratic code and shape.
        if name == "NSW Cubic Parabola" {
            assert_eq!(ty, SpiralType::Biquadratic);
        } else {
            let norm = |s: &str| s.replace(['-', ' '], "").to_lowercase();
            let want = norm(&name.replace("Japanese Cubic", "Cubic (JP)"));
            let have = norm(ty.name());
            assert!(
                have.starts_with(&want) || want.starts_with(&have),
                "{name}: {ty:?}"
            );
            seen.push(ty);
        }
        let segs: Vec<Segment> = d.elements.iter().map(|e| e.segment).collect();
        let w = worst_vertex(&r, &segs);
        println!("{name}: {ty:?} worst {w:.5}");
        assert!(w < 0.01, "{name}: graphics off by {w}");
        // The spiral ends where Civil 3D draws it.
        let end = segs.last().unwrap().end();
        let g = pg::find(&r.data, 4096).unwrap();
        let last = g
            .chunks
            .iter()
            .filter(|c| c.opcode == pg::OP_POLYLINE)
            .flat_map(|c| pg::poly_vertices(&r.data, c).into_iter().map(|v| v.1))
            .map(|p| (p[0] - end.x).hypot(p[1] - end.y))
            .fold(f64::MAX, f64::min);
        assert!(last < 1e-3, "{name}: end off by {last}");
    }
    seen.sort_by_key(|t| t.code());
    let sampled: Vec<SpiralType> = SpiralType::ALL
        .into_iter()
        .filter(|t| *t != SpiralType::NswCubicParabola)
        .collect();
    assert_eq!(seen, sampled);
}
