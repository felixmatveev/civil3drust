//! Fingerprint the raw records of a DWG, optionally after a save round trip.
//!
//! Usage: cargo run --release --example fingerprint -- file.dwg [roundtrip.dwg]
//!
//! Prints one line per record acadrust keeps as raw bytes (all AECC records
//! and other unknown classes): handle, class, handle-stream bits, size and a
//! hash of the bytes, plus entity/object counts. With a second path the
//! drawing is saved there, read back, and fingerprinted again. Diffing the
//! output of two builds (e.g. two acadrust versions) shows whether records
//! survive unchanged.

use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn hash(d: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    d.hash(&mut h);
    h.finish()
}

fn print(label: &str, doc: &acadrust::document::CadDocument, failsafe: &Option<String>) {
    let ents = doc.entities().count();
    let solids = doc
        .entities()
        .filter(|e| matches!(e, EntityType::Solid3D(_)))
        .count();
    println!(
        "## {label}: {ents} entities ({solids} 3DSOLID), {} objects, failsafe: {}",
        doc.objects.len(),
        failsafe.as_deref().unwrap_or("no")
    );
    let mut rows = Vec::new();
    for e in doc.entities() {
        if let EntityType::Unknown(u) = e {
            if let Some(d) = &u.raw_dwg_data {
                rows.push((
                    u.common.handle.value(),
                    u.dxf_name.clone(),
                    u.dwg_handle_bits,
                    d.len(),
                    hash(d),
                ));
            }
        }
    }
    for (h, o) in &doc.objects {
        if let ObjectType::Unknown {
            type_name,
            raw_dwg_data: Some(d),
            raw_dwg_handle_bits,
            ..
        } = o
        {
            rows.push((
                h.value(),
                type_name.clone(),
                *raw_dwg_handle_bits,
                d.len(),
                hash(d),
            ));
        }
    }
    rows.sort();
    println!("raw records: {}", rows.len());
    for (h, c, hb, n, x) in rows {
        println!("{h:>8X} {c:<48} hb={hb:<6} {n:>7} {x:016X}");
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 2 {
        eprintln!("usage: fingerprint <file.dwg> [roundtrip.dwg]");
        std::process::exit(2);
    }
    let o = civil3drust::raw::open(&a[1]).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", a[1]);
        std::process::exit(1);
    });
    print("read", &o.doc, &o.failsafe_reason);
    if let Some(out) = a.get(2) {
        acadrust::DwgWriter::write_to_file(out, &o.doc).unwrap_or_else(|e| {
            eprintln!("failed to write {out}: {e}");
            std::process::exit(1);
        });
        let r = civil3drust::raw::open(out).unwrap_or_else(|e| {
            eprintln!("failed to read back {out}: {e}");
            std::process::exit(1);
        });
        print("saved and read back", &r.doc, &r.failsafe_reason);
    }
}
