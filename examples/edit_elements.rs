//! Remove an element from, or add a free curve to, a Civil 3D alignment
//! (O6, D-036).
//!
//! Usage:
//!   cargo run --release --example edit_elements -- in.dwg out.dwg <alignment> remove <id>
//!   cargo run --release --example edit_elements -- in.dwg out.dwg <alignment> curve <tangent id> <radius>
//!
//! `<alignment>` is the name or hex handle; element IDs are the ones shown
//! by `examples/alignments`. `curve` adds a free curve at the PI after the
//! given tangent. The output is read back and decoded.

use civil3drust::aecc_element_edit::{add_free_curve, remove_element};
use civil3drust::aecc_elements::{decode_elements, DecodedElements, Element, Segment};
use civil3drust::raw;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn find(doc: &acadrust::document::CadDocument, key: &str) -> (raw::RawRecord, DecodedElements) {
    let want = u64::from_str_radix(key.trim_start_matches("0x"), 16).ok();
    for rec in raw::records(doc, "AECC_ALIGNMENT") {
        match decode_elements(&rec) {
            Ok(d) if d.name.as_deref() == Some(key) || Some(d.handle) == want => return (rec, d),
            Err(e) if Some(rec.handle) == want => fail(format!("alignment {key}: {e}")),
            _ => {}
        }
    }
    fail(format!(
        "no decodable alignment named or with handle '{key}'"
    ))
}

fn describe(d: &DecodedElements) {
    for e in &d.elements {
        let what = match &e.record {
            Element::Tangent(t) => format!("tangent {}", t.id),
            Element::Arc(a) => format!(
                "{} curve {} R={}",
                a.def.describe(),
                a.id,
                match e.segment {
                    Segment::Arc { geometry, .. } => geometry.radius,
                    _ => f64::NAN,
                }
            ),
            _ => unreachable!(),
        };
        let (s, t) = (e.segment.start(), e.segment.end());
        println!(
            "  {what:<24} ({:.3}, {:.3}) -> ({:.3}, {:.3})  L={:.4}",
            s.x,
            s.y,
            t.x,
            t.y,
            e.segment.length()
        );
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let usage = "usage: edit_elements <in.dwg> <out.dwg> <alignment> remove <id>\n       \
                 edit_elements <in.dwg> <out.dwg> <alignment> curve <tangent id> <radius>";
    if a.len() < 6 {
        fail(usage);
    }
    let num = |s: &str| -> i32 { s.parse().unwrap_or_else(|_| fail("IDs are whole numbers")) };
    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        eprintln!(
            "WARNING: failsafe read; unparsed records will be missing from the output. ({r})"
        );
    }
    let (rec, before) = find(&opened.doc, &a[3]);
    let seed = rec.handle ^ (rec.data.len() as u64) << 20;
    let (bytes, hb, rep) = match (a[4].as_str(), a.len()) {
        ("remove", 6) => remove_element(&rec, num(&a[5]), seed),
        ("curve", 7) => {
            let r: f64 = a[6]
                .parse()
                .unwrap_or_else(|_| fail("radius must be a number"));
            add_free_curve(&rec, num(&a[5]), r, seed)
        }
        _ => fail(usage),
    }
    .unwrap_or_else(|e| fail(e));
    if !raw::set_record(&mut opened.doc, rec.handle, bytes, hb) {
        fail("could not replace the record");
    }
    println!(
        "[{:X}] {} elements, length {:.4} -> {:.4}, start station {:.4}{}{}",
        rec.handle,
        rep.elements,
        before.length(),
        rep.length,
        rep.start_station,
        rep.added_id
            .map_or(String::new(), |i| format!(", new curve ID {i}")),
        if rep.floating.is_empty() {
            String::new()
        } else {
            format!(", now floating: {:?}", rep.floating)
        }
    );
    if !rep.speeds.is_empty() {
        println!("design speeds: {:?}", rep.speeds);
    }
    match civil3drust::aecc_topology::refresh_companion(&mut opened.doc, rec.handle) {
        Ok(true) => println!("companion graph rebuilt"),
        Ok(false) => println!("companion graph left as it was (site member or unknown form)"),
        Err(e) => fail(e),
    }
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    let reread =
        raw::open(&a[2]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    let (_, after) = find(&reread.doc, &format!("{:X}", rec.handle));
    println!("verify {}:", a[2]);
    describe(&after);
}
