//! Experiment S318: rename a site (the `AECC_PARCEL` with class
//! `AeccImpFace`) in its own record and in the site list
//! (`AECC_PARCEL_NODE`), nothing else. Made to test whether the `|` in the
//! name Civil 3D gave a pasted feature line's new site (`TEST|1`, S317) is
//! why the pasted feature line cannot be edited.
//!
//! Usage: cargo run --release --example exp_site_name -- in.dwg out.dwg <old> <new>

use civil3drust::{handle_refs, raw, string_stream};

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        fail("usage: exp_site_name in.dwg out.dwg <old> <new>");
    }
    let (old, new) = (a[3].as_str(), a[4].as_str());
    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        fail(format!("read in failsafe mode ({r})"));
    }
    let mut n = 0;
    for class in ["AECC_PARCEL", "AECC_PARCEL_NODE"] {
        for rec in raw::records(&opened.doc, class) {
            let Ok(ss) = string_stream::parse(&rec.data, rec.handle_bits) else {
                continue;
            };
            let Some(i) = ss.strings.iter().position(|s| s.text == old) else {
                continue;
            };
            let (d, hb) = string_stream::replace(&rec.data, rec.handle_bits, i, new)
                .unwrap_or_else(|e| fail(format!("{:X}: {e:?}", rec.handle)));
            let (d, hb) = handle_refs::trim_padding(&d, hb, rec.handle);
            if !raw::set_record(&mut opened.doc, rec.handle, d, hb) {
                fail(format!("{:X}: could not replace the record", rec.handle));
            }
            println!("{:X} {class}: string #{i} {old:?} -> {new:?}", rec.handle);
            n += 1;
        }
    }
    if n == 0 {
        fail(format!("no site or site list holds {old:?}"));
    }
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
