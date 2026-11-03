//! Experiment (S202): which step of generation Civil 3D rejects in a
//! JSON-layout alignment, applied in place to Civil 3D's own alignments.
//!
//! - `json:H` — replace the JSON string with the one `aecc_layout::compose`
//!   builds from the alignment's own element records (nothing else);
//! - `reencode:H` — `replace_elements` with the alignment's own element
//!   records (re-encoded, derived parts rebuilt), JSON kept.
//!
//! Usage: exp_json_bisect in.dwg out.dwg json:H reencode:H ...

use civil3drust::aecc_element_edit::replace_elements;
use civil3drust::aecc_elements::element_records;
use civil3drust::aecc_layout::{compose, set_layout};
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        fail("usage: exp_json_bisect in.dwg out.dwg json:H|reencode:H ...");
    }
    let mut d = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    for op in &a[3..] {
        let (what, h) = op.split_once(':').unwrap_or_else(|| fail("op:handle"));
        let h = u64::from_str_radix(h, 16).unwrap_or_else(|_| fail("handle"));
        let rec = raw::records(&d.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap_or_else(|| fail(format!("{h:X} not found")));
        let els = element_records(&rec.data);
        let (bytes, hb) = match what {
            "json" => {
                let locked = civil3drust::aecc_build::element_header_flags(&rec).is_some();
                let j = compose(&els, locked).unwrap_or_else(|e| fail(e));
                set_layout(&rec, &j).unwrap_or_else(|e| fail(e))
            }
            "reencode" => {
                let (b, hb, _) = replace_elements(&rec, &els, 0x5EED).unwrap_or_else(|e| fail(e));
                (b, hb)
            }
            _ => fail("op must be json or reencode"),
        };
        if !raw::set_record(&mut d.doc, h, bytes, hb) {
            fail(format!("{h:X}: could not write"));
        }
        println!("{h:X}: {what}");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &d.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
