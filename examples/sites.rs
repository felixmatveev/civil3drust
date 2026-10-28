//! List the sites of a drawing and the site of each alignment (O7).
//!
//! Usage: cargo run --release --example sites -- file.dwg

use civil3drust::aecc_elements::decode_elements;
use civil3drust::aecc_site::{membership, sites, Membership};
use civil3drust::raw;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 2 {
        eprintln!("usage: sites <file.dwg>");
        std::process::exit(2);
    }
    let o = raw::open(&a[1]).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1)
    });
    let all = sites(&o.doc);
    println!("{} site(s)", all.len());
    for s in &all {
        println!(
            "  [{:X}] {:?}  container {}  description {:?}",
            s.handle,
            s.name,
            s.container.map_or("-".into(), |c| format!("{c:X}")),
            s.description
        );
    }
    for rec in raw::records(&o.doc, "AECC_ALIGNMENT") {
        let name = decode_elements(&rec)
            .ok()
            .and_then(|d| d.name)
            .unwrap_or_default();
        let m = match membership(&o.doc, &rec) {
            Membership::Siteless { companion } => format!("siteless (companion {companion:X})"),
            Membership::InSite { site, container } => {
                let n = all
                    .iter()
                    .find(|s| s.handle == site)
                    .map_or("", |s| &s.name);
                format!("site {n:?} [{site:X}] (container {container:X})")
            }
            Membership::Unknown => "unknown".into(),
        };
        println!("alignment [{:X}] {name}: {m}", rec.handle);
    }
}
