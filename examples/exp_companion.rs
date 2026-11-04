//! MIST12 experiment: rewrite the companions of siteless alignments in the
//! form Civil 3D writes after rebuilding a graph in a session: edge flags
//! `0x10`, vertex flags 10, a = b = 0 (whatever form they had: from an
//! object `0x30`/`0x830` with (2|50, 1, 1), layout `0x810`, loaded (2, 0,
//! 0)). Nodes and vertices come from the alignment's geometry; the tail,
//! strings and handle stream are kept.
//!
//! Usage: cargo run --release --example exp_companion -- in.dwg out.dwg <alignment hex> ...

use civil3drust::aecc_alignment_refs::alignment_refs;
use civil3drust::aecc_elements::decode_elements;
use civil3drust::aecc_topology::{companion_graph, rebuild_companion_with, EDGE_REBUILT};
use civil3drust::{handle_refs, raw};

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        fail("usage: exp_companion in.dwg out.dwg <alignment hex> ...");
    }
    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        fail(format!("read in failsafe mode ({r})"));
    }
    for h in &a[3..] {
        let h = u64::from_str_radix(h, 16).unwrap_or_else(|_| fail("bad handle"));
        let rec = raw::records(&opened.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap_or_else(|| fail(format!("alignment {h:X} not found")));
        let d = decode_elements(&rec).unwrap_or_else(|e| fail(e));
        let c = alignment_refs(&rec)
            .unwrap_or_else(|| fail("references not read"))
            .companion
            .handle;
        let comp = raw::records(&opened.doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == c)
            .unwrap_or_else(|| fail(format!("companion {c:X} not found")));
        let old = companion_graph(&comp.data).unwrap_or_else(|| fail("companion graph not read"));
        let (bytes, hb) = rebuild_companion_with(
            &comp.data,
            comp.handle_bits,
            &d.elements,
            Some(EDGE_REBUILT),
        )
        .unwrap_or_else(|| fail("companion not rebuilt"));
        let (bytes, hb) = handle_refs::trim_padding(&bytes, hb, c);
        if !raw::set_record(&mut opened.doc, c, bytes, hb) {
            fail("could not replace the companion");
        }
        let e = &old.edges[0];
        println!(
            "{h:X}: companion {c:X} edge {:#05x} vertices {:?} -> {:#05x}, (10, 0, 0) x {}",
            e.flags,
            e.vertices
                .iter()
                .map(|v| (v.flags, v.a, v.b))
                .collect::<Vec<_>>(),
            EDGE_REBUILT,
            d.elements.len() + 1
        );
    }
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
