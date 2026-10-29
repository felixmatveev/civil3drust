//! Change the radius of a free arc (fillet) in a Civil 3D alignment.
//!
//! Usage:
//!   cargo run --release --example set_radius -- in.dwg out.dwg <alignment> <arc id> <radius>
//!
//! `<alignment>` is the name or hex handle; `<arc id>` is the element ID
//! shown by `examples/alignments`. The output is read back and decoded.

use civil3drust::aecc_arc_edit::set_free_arc_radius;
use civil3drust::aecc_elements::{decode_elements, DecodedElements};
use civil3drust::raw;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn find(doc: &acadrust::document::CadDocument, key: &str) -> DecodedElements {
    let want = u64::from_str_radix(key.trim_start_matches("0x"), 16).ok();
    for rec in raw::records(doc, "AECC_ALIGNMENT") {
        match decode_elements(&rec) {
            Ok(d) if d.name.as_deref() == Some(key) || Some(d.handle) == want => return d,
            Err(e) if Some(rec.handle) == want => fail(format!("alignment {key}: {e}")),
            _ => {}
        }
    }
    fail(format!(
        "no decodable alignment named or with handle '{key}'"
    ))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 6 {
        eprintln!("usage: set_radius <in.dwg> <out.dwg> <alignment> <arc id> <radius>");
        std::process::exit(2);
    }
    let id: i32 = a[4]
        .parse()
        .unwrap_or_else(|_| fail("arc id must be a whole number"));
    let radius: f64 = a[5]
        .parse()
        .unwrap_or_else(|_| fail("radius must be a number"));

    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        eprintln!(
            "WARNING: acadrust read this file in failsafe mode; records it could not parse\n\
             will be MISSING from the output. ({r})"
        );
    }
    let before = find(&opened.doc, &a[3]);
    let data = raw::record_data_mut(&mut opened.doc, before.handle)
        .unwrap_or_else(|| fail("record bytes missing"));
    let rep = set_free_arc_radius(data, &before, id, radius).unwrap_or_else(|e| fail(e));
    println!(
        "patched [{:X}] arc {id}: R {} -> {}, PC station {:.4} -> {:.4} ({} cached cop{})",
        before.handle,
        rep.old_radius,
        rep.new_radius,
        rep.old_pc_station,
        rep.new_pc_station,
        rep.station_copies,
        if rep.station_copies == 1 { "y" } else { "ies" }
    );
    if rep.graphics_stale {
        println!("note: display graphics are out of date until Civil 3D redraws the alignment");
    }

    match civil3drust::aecc_topology::refresh_companion(&mut opened.doc, before.handle) {
        Ok(true) => println!("companion graph rebuilt"),
        Ok(false) => println!("companion graph left as it was (site member or unknown form)"),
        Err(e) => fail(e),
    }
    // Re-save caches are dropped, never carried over (D-046, D-047).
    civil3drust::aecc_build::drop_record_caches(&mut opened.doc, before.handle);
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    let reread =
        raw::open(&a[2]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    let after = find(&reread.doc, &format!("{:X}", before.handle));
    println!(
        "verify {}: length {:.4} -> {:.4}",
        a[2],
        before.length(),
        after.length()
    );
}
