//! Delete alignments, as Civil 3D does (S228 → S229): the alignment and
//! its label groups are removed, its companion kept and emptied.
//!
//! Usage: cargo run --release --example delete_alignment -- in.dwg out.dwg <name|handle> ...

use civil3drust::aecc_delete::delete_alignment;
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        fail("usage: delete_alignment in.dwg out.dwg <name|handle> ...");
    }
    let mut d = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    for key in &a[3..] {
        let rec = raw::records(&d.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| {
                raw::object_name(&r.data).as_deref() == Some(key.as_str())
                    || u64::from_str_radix(key, 16).ok() == Some(r.handle)
            })
            .unwrap_or_else(|| fail(format!("no alignment {key}")));
        let name = raw::object_name(&rec.data).unwrap_or_default();
        let x = delete_alignment(&mut d.doc, rec.handle).unwrap_or_else(|e| fail(e));
        println!(
            "deleted [{:X}] {name}: label groups {:X?}, companion {:X} emptied",
            x.alignment, x.label_groups, x.companion
        );
    }
    acadrust::DwgWriter::write_to_file(&a[2], &d.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
