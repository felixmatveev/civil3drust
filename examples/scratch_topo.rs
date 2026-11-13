use civil3drust::aecc_site_edit::Topology;
use civil3drust::raw;
fn main() {
    for f in std::env::args().skip(1) {
        let o = raw::open(&f).unwrap();
        println!("=== {}", f.rsplit('/').next().unwrap());
        for r in raw::records(&o.doc, "AECC_FEATURE_STYLE") {
            let Ok(t) = Topology::parse(&r) else { continue };
            if t.nodes.is_empty() {
                continue;
            }
            println!(
                "[{:X}] owner {:?} node_next {} edge_next {} members {:X?} faces {:?}",
                r.handle, t.owner, t.node_next, t.edge_next, t.members, t.faces
            );
            for e in t.edges.iter().flatten() {
                println!(
                    "  edge {} flags {:#X} after {:?}, {} vertices",
                    e.id,
                    e.flags,
                    e.after,
                    e.vertices.len()
                );
                for v in &e.vertices {
                    let n = t.nodes.iter().find(|n| n.0 == v.node).unwrap();
                    println!(
                        "     kind {} flags {:#X} node {:>3} ({:.4}, {:.4}) a {} b {} bulge {:?}",
                        v.kind, v.flags, v.node, n.1, n.2, v.a, v.b, v.bulge
                    );
                }
            }
        }
    }
}
