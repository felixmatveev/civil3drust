//! Change one tangent's length in a Civil 3D alignment and save the drawing.
//!
//! Usage:
//!   cargo run --release --example set_length -- in.dwg out.dwg <alignment> <segment> <length>
//!
//! `<alignment>` is the alignment name or its hex handle; `<segment>` is
//! 1-based. The segment keeps its start PI and direction; every later PI
//! moves with it (D-003). The output is read back and decoded to verify.

use civil3drust::aecc_alignment::{decode, DecodedAlignment};
use civil3drust::aecc_alignment_edit::apply;
use civil3drust::raw;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn find(doc: &acadrust::document::CadDocument, key: &str) -> DecodedAlignment {
    let want = u64::from_str_radix(key.trim_start_matches("0x"), 16).ok();
    for rec in raw::records(doc, "AECC_ALIGNMENT") {
        match decode(&rec) {
            Ok(d) if d.alignment.name == key || Some(d.handle) == want => return d,
            Err(e) if Some(rec.handle) == want => {
                fail(format!("alignment {key} can't be edited: {e}"))
            }
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
        eprintln!("usage: set_length <in.dwg> <out.dwg> <alignment> <segment> <length>");
        std::process::exit(2);
    }
    let seg: usize = a[4]
        .parse()
        .unwrap_or_else(|_| fail("segment must be a whole number"));
    let len: f64 = a[5]
        .parse()
        .unwrap_or_else(|_| fail("length must be a number"));
    if seg == 0 {
        fail("segments are numbered from 1");
    }

    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        eprintln!(
            "WARNING: acadrust read this file in failsafe mode; records it could not parse\n\
             will be MISSING from the output. ({r})"
        );
    }
    let before = find(&opened.doc, &a[3]);
    let mut target = before.alignment.clone();
    target
        .set_segment_length(seg - 1, len)
        .unwrap_or_else(|e| fail(e));

    let data = raw::record_data_mut(&mut opened.doc, before.handle)
        .unwrap_or_else(|| fail("record bytes missing"));
    let rep = apply(data, &before, &target).unwrap_or_else(|e| fail(e));
    println!(
        "patched [{:X}] {}: {} tangent record(s), {} cache record(s), {} graphics vertex(es), {} marker(s){}",
        before.handle,
        before.alignment.name,
        rep.tangent_records,
        rep.cache_records,
        rep.graphics_vertices,
        rep.graphics_markers,
        if rep.placeholder_box { ", placeholder box" } else { "" }
    );

    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));

    let reread =
        raw::open(&a[2]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    let after = find(&reread.doc, &format!("{:X}", before.handle));
    println!(
        "verify {}: length {:.4} -> {:.4}",
        a[2],
        before.alignment.length(),
        after.alignment.length()
    );
    for t in after.alignment.tangents() {
        println!(
            "  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}",
            t.start.x,
            t.start.y,
            t.end.x,
            t.end.y,
            t.length()
        );
    }
}
