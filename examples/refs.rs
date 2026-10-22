//! List the handle references of AECC records, with the class of each
//! referenced object.
//!
//! Usage: cargo run --release --example refs -- file.dwg [AECC_CLASS] [handle]
//! (default class AECC_ALIGNMENT).

use civil3drust::{handle_refs, raw};

fn code_name(c: u8) -> &'static str {
    match c {
        2 => "soft owner",
        3 => "hard owner",
        4 => "soft ptr",
        5 => "hard ptr",
        6 => "own+1",
        8 => "own-1",
        0xA => "own+off",
        0xC => "own-off",
        _ => "?",
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 2 {
        eprintln!("usage: refs <file.dwg> [AECC_CLASS] [handle]");
        std::process::exit(2);
    }
    let class = a.get(2).map(String::as_str).unwrap_or("AECC_ALIGNMENT");
    let only = a.get(3).and_then(|h| u64::from_str_radix(h, 16).ok());
    let o = raw::open(&a[1]).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1)
    });
    for rec in raw::records(&o.doc, class) {
        if only.is_some_and(|h| h != rec.handle) {
            continue;
        }
        println!(
            "[{:X}] {} {} bytes, handle stream {} bits from bit {}",
            rec.handle,
            raw::object_name(&rec.data).unwrap_or_default(),
            rec.data.len(),
            rec.handle_bits,
            raw::bit_len(&rec.data) - rec.handle_bits
        );
        for (i, r) in handle_refs::of_record(&rec).iter().enumerate() {
            let target = if r.handle == 0 {
                "(null)".to_string()
            } else {
                let cls = raw::class_of(&o.doc, r.handle).unwrap_or("?".into());
                let name = raw::records(&o.doc, &cls)
                    .into_iter()
                    .find(|x| x.handle == r.handle)
                    .and_then(|x| raw::object_name(&x.data))
                    .unwrap_or_default();
                format!("{cls} {name}")
            };
            println!(
                "  #{i:<2} @{:<5} {:X}.{} {:<10} {:X}  {target}",
                r.bit,
                r.code,
                r.counter,
                code_name(r.code),
                r.handle
            );
        }
    }
}
