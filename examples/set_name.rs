//! Rename a Civil 3D alignment (any name length) and save the drawing.
//!
//! Usage:
//!   cargo run --release --example set_name -- in.dwg out.dwg <alignment> <new name> [--description "text"]
//!
//! Use `-` as the new name to keep the current one (e.g. to set only the
//! description). `\n` in the description becomes a line break.
//!
//! `<alignment>` is the current name or hex handle. The name's string in
//! the record's string stream is rewritten and the stream size updated
//! (docs/alignment-object.md, "Names and the string stream").

use acadrust::entities::EntityType;
use civil3drust::{raw, string_stream};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let mut a: Vec<String> = std::env::args().collect();
    let mut description = None;
    if let Some(i) = a.iter().position(|x| x == "--description") {
        if i + 1 >= a.len() {
            fail("--description needs a value");
        }
        description = Some(a[i + 1].replace("\\n", "\n"));
        a.drain(i..=i + 1);
    }
    if a.len() != 5 {
        eprintln!(
            "usage: set_name <in.dwg> <out.dwg> <alignment> <new name|-> [--description \"text\"]"
        );
        std::process::exit(2);
    }
    let mut o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &o.failsafe_reason {
        eprintln!("WARNING: read in failsafe mode; unparsed records will be missing. ({r})");
    }
    let rec = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| {
            raw::object_name(&r.data).as_deref() == Some(a[3].as_str())
                || u64::from_str_radix(&a[3], 16).ok() == Some(r.handle)
        })
        .unwrap_or_else(|| fail(format!("no alignment '{}'", a[3])));
    let old = raw::object_name(&rec.data).unwrap_or_else(|| fail("alignment has no name"));
    let new_name = if a[4] == "-" {
        old.clone()
    } else {
        a[4].clone()
    };
    let (mut bytes, mut hb) = string_stream::set_name(&rec.data, rec.handle_bits, &old, &new_name)
        .unwrap_or_else(|e| fail(e));
    if let Some(d) = &description {
        (bytes, hb) = string_stream::set_description(&bytes, hb, d).unwrap_or_else(|e| fail(e));
    }
    println!(
        "[{:X}] '{}' -> '{}': {} -> {} bytes",
        rec.handle,
        old,
        new_name,
        rec.data.len(),
        bytes.len()
    );
    for e in o.doc.entities_mut() {
        if let EntityType::Unknown(u) = e {
            if u.common.handle.value() == rec.handle {
                u.raw_dwg_data = Some(bytes.clone());
                u.dwg_handle_bits = hb as i64;
            }
        }
    }
    acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
    let re = raw::open(&a[2]).unwrap_or_else(|e| fail(format!("does not read back: {e}")));
    for r in raw::records(&re.doc, "AECC_ALIGNMENT") {
        let ss = string_stream::parse(&r.data, r.handle_bits).unwrap_or_else(|e| fail(e));
        let d = civil3drust::aecc_elements::decode_elements(&r).unwrap_or_else(|e| fail(e));
        println!(
            "  [{:X}] {:?}  length {:.4}  strings {}  description {:?}",
            r.handle,
            d.name.as_deref().unwrap_or("?"),
            d.length(),
            ss.strings.len(),
            ss.strings.get(1).map(|s| s.text.as_str()).unwrap_or("")
        );
    }
}
