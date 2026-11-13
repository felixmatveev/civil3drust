//! Crossing experiment (S362, S363; docs/site-object.md, "Crossing and
//! touching members"): move a siteless straight alignment into a site
//! where it crosses straight members, as S354 shows Civil 3D stores it —
//! one shared node per crossing point, inserted as a vertex into each
//! edge (no split), one loop for the connected part.
//!
//! Usage: exp_site_cross in.dwg out.dwg <alignment hex> <site hex> plain|civil
//!   plain: inserted vertices get the flags of a plain middle vertex (50)
//!   civil: they get Civil 3D's crossing flags of S354 (0x430 = 1072)
//!   none:  no shared node: the edges keep their own nodes and loops,
//!          as if they did not cross (S366: does Civil 3D add the node?)

use civil3drust::aecc_site_edit::{self as edit, Topology, Vertex};
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or_else(|_| fail(format!("not a handle: {s}")))
}

/// Interior intersection of two segments: (point, t along a, t along b).
fn cross(a: [(f64, f64); 2], b: [(f64, f64); 2]) -> Option<((f64, f64), f64, f64)> {
    let (r, s) = (
        (a[1].0 - a[0].0, a[1].1 - a[0].1),
        (b[1].0 - b[0].0, b[1].1 - b[0].1),
    );
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-12 {
        return None;
    }
    let q = (b[0].0 - a[0].0, b[0].1 - a[0].1);
    let t = (q.0 * s.1 - q.1 * s.0) / den;
    let u = (q.0 * r.1 - q.1 * r.0) / den;
    let e = 1e-9;
    (t > e && t < 1.0 - e && u > e && u < 1.0 - e)
        .then(|| ((a[0].0 + t * r.0, a[0].1 + t * r.1), t, u))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 6 {
        fail("usage: exp_site_cross in.dwg out.dwg <alignment hex> <site hex> plain|civil|none");
    }
    let flags = match a[5].as_str() {
        "plain" => 50,
        "civil" => 0x430,
        "none" => 0,
        m => fail(format!("unknown mode {m}")),
    };
    let (al, site) = (hex(&a[3]), hex(&a[4]));
    let mut o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let m = edit::move_into_site_unchecked(&mut o.doc, al, site).unwrap_or_else(|e| fail(e));
    if flags == 0 {
        println!("{al:X} → site {site:X}: no shared node written");
        acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
        return println!("wrote {}", a[2]);
    }
    let cont = raw::records(&o.doc, "AECC_FEATURE_STYLE")
        .into_iter()
        .find(|r| r.handle == m.container)
        .unwrap();
    let mut t = Topology::parse(&cont).unwrap_or_else(|e| fail(e));
    let new = t.edges.iter().rposition(|e| e.is_some()).unwrap();
    let at = |t: &Topology, id: i32| {
        let n = t.nodes.iter().find(|n| n.0 == id).unwrap();
        (n.1, n.2)
    };
    let mut crossings = 0;
    for other in 0..t.edges.len() {
        if other == new || t.edges[other].is_none() {
            continue;
        }
        // Each crossing of a chord of `new` with a chord of `other`.
        'again: loop {
            let (en, eo) = (
                t.edges[new].clone().unwrap(),
                t.edges[other].clone().unwrap(),
            );
            if en
                .vertices
                .iter()
                .chain(&eo.vertices)
                .any(|v| v.bulge.is_some())
            {
                fail("arcs are not handled by this experiment");
            }
            for i in 0..en.vertices.len() - 1 {
                for j in 0..eo.vertices.len() - 1 {
                    let sa = [at(&t, en.vertices[i].node), at(&t, en.vertices[i + 1].node)];
                    let sb = [at(&t, eo.vertices[j].node), at(&t, eo.vertices[j + 1].node)];
                    let Some((p, _, _)) = cross(sa, sb) else {
                        continue;
                    };
                    let id = t.node_next;
                    t.node_next += 1;
                    t.nodes.push((id, p.0, p.1, 0.0));
                    let v = Vertex {
                        kind: 0,
                        flags,
                        node: id,
                        a: 1,
                        b: 1,
                        bulge: None,
                    };
                    t.edges[new]
                        .as_mut()
                        .unwrap()
                        .vertices
                        .insert(i + 1, v.clone());
                    t.edges[other].as_mut().unwrap().vertices.insert(j + 1, v);
                    crossings += 1;
                    continue 'again;
                }
            }
            break;
        }
    }
    if crossings == 0 {
        fail("no crossing: use exp_site move-in");
    }
    // The new edge now joins the part of the edges it crosses: its loop
    // goes (one loop per connected part; S354 has one).
    let first = t.edges[new].as_ref().unwrap().vertices[0].node;
    if let Some(f) = t.faces.first_mut() {
        if f.loops.len() > 1 {
            f.loops.retain(|l| l.0 != first);
        }
    }
    let (d, hb) = t.write(&cont).unwrap_or_else(|e| fail(e));
    let (d, hb) = civil3drust::handle_refs::trim_padding(&d, hb, m.container);
    if !raw::set_record(&mut o.doc, m.container, d, hb) {
        fail("could not write the container");
    }
    println!("{al:X} → site {site:X}: {crossings} crossing(s), vertex flags {flags}");
    acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
