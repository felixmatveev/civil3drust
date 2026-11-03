//! Experiment (S202): which step of generation Civil 3D rejects in a
//! JSON-layout alignment, applied in place to Civil 3D's own alignments.
//!
//! - `json:H` — replace the JSON string with the one `aecc_layout::compose`
//!   builds from the alignment's own element records (nothing else);
//! - `reencode:H` — `replace_elements` with the alignment's own element
//!   records (re-encoded, derived parts rebuilt), JSON kept;
//! - `reencode-mist1:H` — as `reencode`, then MIST1 set back to the
//!   original value;
//! - `zero-mist1:H` — only MIST1 zeroed;
//! - `json-radius:H` — only the JSON's `ArcRadius` replaced by the radius
//!   measured from the curve piece (a last-digits change).
//!
//! Usage: exp_json_bisect in.dwg out.dwg json:H reencode:H ...

use civil3drust::aecc_build::{geometry_key_bit, GEOMETRY_KEY_BITS};
use civil3drust::aecc_element_edit::replace_elements;
use civil3drust::aecc_elements::element_records;
use civil3drust::aecc_layout::{compose, set_layout, Json};
use civil3drust::raw;
use civil3drust::string_stream;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn set_bit(b: &mut [u8], i: usize, v: u8) {
    let m = 1u8 << (7 - i % 8);
    if v == 1 {
        b[i / 8] |= m;
    } else {
        b[i / 8] &= !m;
    }
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
            "reencode" | "reencode-mist1" => {
                let (mut b, hb, _) =
                    replace_elements(&rec, &els, 0x5EED).unwrap_or_else(|e| fail(e));
                if what == "reencode-mist1" {
                    let from = geometry_key_bit(&rec.data).unwrap();
                    let to = geometry_key_bit(&b).unwrap();
                    for i in 0..GEOMETRY_KEY_BITS {
                        set_bit(&mut b, to + i, raw::get_bit(&rec.data, from + i));
                    }
                }
                (b, hb)
            }
            "zero-mist1" => {
                let mut b = rec.data.clone();
                let k = geometry_key_bit(&b).unwrap();
                for i in 0..GEOMETRY_KEY_BITS {
                    set_bit(&mut b, k + i, 0);
                }
                (b, rec.handle_bits)
            }
            "json-radius" => {
                let text = string_stream::layout_json(&rec.data, rec.handle_bits)
                    .unwrap_or_else(|| fail("no JSON"));
                let mut j = Json::parse(&text).unwrap_or_else(|e| fail(e));
                let ours = compose(&els, true).unwrap_or_else(|e| fail(e));
                let radius = |j: &Json| -> Option<f64> {
                    let Some(Json::Arr(o)) = j.get("RGObjects") else {
                        return None;
                    };
                    o.iter().find_map(|x| {
                        match x.get("ConstraintParameters")?.get("ArcRadius")? {
                            Json::Num(v) => Some(*v),
                            _ => None,
                        }
                    })
                };
                let r = radius(&ours).unwrap_or_else(|| fail("no ArcRadius"));
                if let Some(Json::Arr(o)) = j.get_mut("RGObjects") {
                    for x in o {
                        if let Some(Json::Num(v)) = x
                            .get_mut("ConstraintParameters")
                            .and_then(|p| p.get_mut("ArcRadius"))
                        {
                            println!("{h:X}: ArcRadius {v} -> {r}");
                            *v = r;
                        }
                    }
                }
                set_layout(&rec, &j).unwrap_or_else(|e| fail(e))
            }
            _ => fail("unknown op"),
        };
        if !raw::set_record(&mut d.doc, h, bytes, hb) {
            fail(format!("{h:X}: could not write"));
        }
        println!("{h:X}: {what}");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &d.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
