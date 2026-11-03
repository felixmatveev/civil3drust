//! Experiment S179/S180: drop Civil 3D's JSON layout string (S177, S178)
//! from every alignment that has one, for the owner to open in Civil 3D.
//!
//! Modes: `remove` (the string is taken out of the stream; the strings
//! after it move up) or `empty` (the string stays, with no text).
//!
//! Usage: cargo run --release --example exp_json -- in.dwg out.dwg remove|empty

use civil3drust::{handle_refs, raw, string_stream};

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 4 || !["remove", "empty"].contains(&a[3].as_str()) {
        fail("usage: exp_json in.dwg out.dwg remove|empty");
    }
    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        fail(format!("read in failsafe mode ({r})"));
    }
    let mut n = 0;
    for rec in raw::records(&opened.doc, "AECC_ALIGNMENT") {
        let Ok(ss) = string_stream::parse(&rec.data, rec.handle_bits) else {
            continue;
        };
        let Some(i) = ss.strings.iter().position(|s| raw::is_layout_json(&s.text)) else {
            continue;
        };
        let (d, hb) = if a[3] == "remove" {
            string_stream::remove(&rec.data, rec.handle_bits, i)
        } else {
            string_stream::replace(&rec.data, rec.handle_bits, i, "")
        }
        .unwrap_or_else(|e| fail(format!("{:X}: {e:?}", rec.handle)));
        let (d, hb) = handle_refs::trim_padding(&d, hb, rec.handle);
        if !raw::set_record(&mut opened.doc, rec.handle, d, hb) {
            fail(format!("{:X}: could not replace the record", rec.handle));
        }
        println!(
            "{:X} {}: JSON string #{i} ({} chars) {}",
            rec.handle,
            raw::object_name(&rec.data).unwrap_or_default(),
            ss.strings[i].text.len(),
            if a[3] == "remove" {
                "removed"
            } else {
                "emptied"
            }
        );
        n += 1;
    }
    if n == 0 {
        fail("no alignment has a JSON layout string");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
