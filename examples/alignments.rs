//! List the alignments in a DWG with their tangent and arc elements,
//! decoded from the element records (see docs/alignment-object.md).
//!
//! Usage: cargo run --release --example alignments -- file.dwg

use civil3drust::aecc_alignment::{record_name, Linked};
use civil3drust::aecc_elements::{decode_elements, Element, Segment};
use civil3drust::raw;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: alignments <file.dwg>");
        std::process::exit(2);
    };
    let opened = raw::open(&path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    if let Some(r) = &opened.failsafe_reason {
        eprintln!("note: read in failsafe mode; some records were dropped ({r})");
    }
    let recs = raw::records(&opened.doc, "AECC_ALIGNMENT");
    println!("{path}: {} AECC_ALIGNMENT record(s)", recs.len());
    for rec in &recs {
        let d = match decode_elements(rec) {
            Ok(d) => d,
            Err(e) => {
                println!(
                    "\n[{:X}] {}  not decoded: {e}",
                    rec.handle,
                    record_name(&rec.data).unwrap_or_else(|| "?".into())
                );
                continue;
            }
        };
        let start = d.stored_start_station.unwrap_or(0.0);
        let arcs = d
            .elements
            .iter()
            .filter(|e| matches!(e.segment, Segment::Arc { .. }))
            .count();
        println!(
            "\n[{:X}] {}  {} element(s) ({} tangent, {} arc), length {:.4}, stations {:.2}-{:.2}{}",
            d.handle,
            d.name.as_deref().unwrap_or("?"),
            d.elements.len(),
            d.elements.len() - arcs,
            arcs,
            d.length(),
            start,
            start + d.length(),
            if d.stored_start_station.is_none() {
                " (start station not found; shown from 0)"
            } else {
                ""
            }
        );
        let mut sta = start;
        for e in &d.elements {
            let len = e.segment.length();
            let (s, t) = (e.segment.start(), e.segment.end());
            let what = match (&e.segment, &e.record) {
                (Segment::Line { .. }, _) => "tangent".to_string(),
                (Segment::Arc { geometry: g, .. }, rec) => {
                    let kind = match rec {
                        Element::Arc(a) => a.def.describe(),
                        Element::Tangent(_) => "?",
                    };
                    format!(
                        "arc ({kind}) R={:.4} {} delta={:.4}° center ({:.4}, {:.4})",
                        g.radius,
                        if g.counter_clockwise { "left" } else { "right" },
                        g.sweep.to_degrees(),
                        g.center.x,
                        g.center.y
                    )
                }
            };
            println!(
                "  id {:>2} (before {}, after {})  {}  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}  sta {:.2}-{:.2}",
                e.id(),
                e.before(),
                e.after(),
                what,
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
        for (k, piece) in d.detached.iter().enumerate() {
            println!(
                "  detached piece {} (ignored by Civil 3D, no stations):",
                k + 1
            );
            for e in piece {
                let (s, t) = (e.segment.start(), e.segment.end());
                println!(
                    "    id {:>2} (before {}, after {})  {}  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}",
                    e.id(),
                    e.before(),
                    e.after(),
                    if matches!(e.segment, Segment::Line { .. }) { "tangent" } else { "arc" },
                    s.x,
                    s.y,
                    t.x,
                    t.y,
                    e.segment.length()
                );
            }
        }
    }
}
