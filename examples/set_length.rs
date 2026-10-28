//! Change one tangent's length in a Civil 3D alignment and save the drawing.
//!
//! Usage:
//!   cargo run --release --example set_length -- in.dwg out.dwg <alignment> <segment> <length>
//!
//! `<alignment>` is the alignment name or its hex handle; `<segment>` is the
//! 1-based tangent number in stationing order (curves are not counted), and
//! `<length>` its PI-to-PI length. The tangent keeps its start PI and
//! direction; every later PI moves with it (D-003), and free curves
//! (fillets) follow. Alignments with arcs defined by three points are
//! refused. The output is read back and decoded to verify.

use civil3drust::aecc_alignment_edit::set_tangent_length;
use civil3drust::aecc_elements::{decode_elements, DecodedElements, Segment};
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

fn print(d: &DecodedElements) {
    let mut sta = d.stored_start_station.unwrap_or(0.0);
    for e in &d.elements {
        let (s, t) = (e.segment.start(), e.segment.end());
        let kind = match e.segment {
            Segment::Line { .. } => "tangent",
            Segment::Arc { .. } => "arc    ",
            Segment::Spiral { .. } => "spiral ",
        };
        let len = e.segment.length();
        println!(
            "  {kind} ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}  sta {:.2}-{:.2}",
            s.x,
            s.y,
            t.x,
            t.y,
            len,
            sta,
            sta + len
        );
        sta += len;
    }
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
    let data = raw::record_data_mut(&mut opened.doc, before.handle)
        .unwrap_or_else(|| fail("record bytes missing"));
    let rep = set_tangent_length(data, &before, seg - 1, len).unwrap_or_else(|e| fail(e));
    let p = &rep.patches;
    println!(
        "patched [{:X}] {}: {} tangent record(s), {} cache record(s), {} graphics vertex(es), {} marker(s){}; {} curve(s) restationed ({} cached station(s))",
        before.handle,
        before.name.as_deref().unwrap_or("?"),
        p.tangent_records,
        p.cache_records,
        p.graphics_vertices,
        p.graphics_markers,
        if p.placeholder_box { ", placeholder box" } else { "" },
        rep.arcs_restationed,
        rep.station_copies
    );

    match civil3drust::aecc_topology::refresh_companion(&mut opened.doc, before.handle) {
        Ok(true) => println!("companion graph rebuilt"),
        Ok(false) => println!("companion graph left as it was (site member or unknown form)"),
        Err(e) => fail(e),
    }
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
    print(&after);
}
