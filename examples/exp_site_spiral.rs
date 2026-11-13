//! Spiral experiment (S370, S371; docs/site-object.md, "Spirals in a
//! site"): move a siteless alignment with spirals into a site, its edge
//! taken from its own companion.
//!
//! Usage: exp_site_spiral in.dwg out.dwg <alignment hex> <site hex> companion | chords i,j,...
//!   companion: the companion's vertices as they are (Civil 3D's arc
//!              pieces for each spiral, as in S368)
//!   chords:    only the companion vertices at the given indices; a kept
//!              vertex keeps its bulge only if the next kept vertex is
//!              the next one (spirals become straight chords)

use civil3drust::aecc_site_edit::{self as edit, PathFrom, Topology};
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or_else(|_| fail(format!("not a handle: {s}")))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 6 {
        fail("usage: exp_site_spiral in.dwg out.dwg <alignment hex> <site hex> companion | chords i,j,...");
    }
    let (al, site) = (hex(&a[3]), hex(&a[4]));
    let mut o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let path = match a[5].as_str() {
        "companion" => PathFrom::Companion,
        "chords" => {
            let keep: Vec<usize> = a
                .get(6)
                .unwrap_or_else(|| fail("chords needs indices"))
                .split(',')
                .map(|x| x.parse().unwrap_or_else(|_| fail(format!("index {x}"))))
                .collect();
            let r = raw::records(&o.doc, "AECC_ALIGNMENT")
                .into_iter()
                .find(|r| r.handle == al)
                .unwrap_or_else(|| fail("no such alignment"));
            let comp_h = civil3drust::handle_refs::of_record(&r)[2].handle;
            let c = raw::records(&o.doc, "AECC_FEATURE_STYLE")
                .into_iter()
                .find(|c| c.handle == comp_h)
                .unwrap();
            let t = Topology::parse(&c).unwrap_or_else(|e| fail(e));
            let e = t.edges.iter().flatten().next().unwrap();
            let pt = |i: usize| {
                let n = t.nodes.iter().find(|n| n.0 == e.vertices[i].node).unwrap();
                (n.1, n.2)
            };
            let pts: Vec<(f64, f64)> = keep.iter().map(|&i| pt(i)).collect();
            let bulges = keep
                .windows(2)
                .map(|w| {
                    if w[1] == w[0] + 1 {
                        e.vertices[w[0]].bulge
                    } else {
                        None
                    }
                })
                .collect();
            PathFrom::Points(pts, bulges)
        }
        m => fail(format!("unknown mode {m}")),
    };
    let m = edit::move_into_site_with(&mut o.doc, al, site, true, path).unwrap_or_else(|e| fail(e));
    println!(
        "{al:X} → site {:X} (container {:X}); companion {:X} emptied",
        m.site, m.container, m.companion
    );
    acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
