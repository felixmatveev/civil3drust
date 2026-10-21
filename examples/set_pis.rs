//! Move every PI of a Civil 3D alignment (tangents and free curves) and
//! save the drawing. Used to produce a new alignment from a template record
//! of the same shape.
//!
//! Usage:
//!   cargo run --release --example set_pis -- in.dwg out.dwg <alignment> "x,y x,y x,y ..."
//!
//! The number of PIs must match the template. Free curves keep their radius
//! and follow their tangents. The output is read back and decoded.

use civil3drust::aecc_alignment_edit::{pi_alignment, set_pis};
use civil3drust::aecc_elements::{decode_elements, DecodedElements, Segment};
use civil3drust::alignment::Point2;
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

fn parse_pis(s: &str) -> Vec<Point2> {
    s.split_whitespace()
        .map(|p| {
            let (x, y) = p
                .split_once(',')
                .unwrap_or_else(|| fail(format!("bad PI '{p}', expected x,y")));
            let n = |v: &str| {
                v.trim()
                    .parse::<f64>()
                    .unwrap_or_else(|_| fail(format!("bad number '{v}'")))
            };
            Point2::new(n(x), n(y))
        })
        .collect()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        eprintln!("usage: set_pis <in.dwg> <out.dwg> <alignment> \"x,y x,y ...\"");
        std::process::exit(2);
    }
    let pis = parse_pis(&a[4]);
    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        eprintln!(
            "WARNING: acadrust read this file in failsafe mode; records it could not parse\n\
             will be MISSING from the output. ({r})"
        );
    }
    let before = find(&opened.doc, &a[3]);
    let template = pi_alignment(&before).unwrap_or_else(|e| fail(e));
    println!(
        "template PIs: {:?}",
        template
            .pis()
            .iter()
            .map(|p| (p.x, p.y))
            .collect::<Vec<_>>()
    );
    let data = raw::record_data_mut(&mut opened.doc, before.handle)
        .unwrap_or_else(|| fail("record bytes missing"));
    let rep = set_pis(data, &before, &pis).unwrap_or_else(|e| fail(e));
    println!(
        "patched: {} tangent record(s), {} cache record(s){}; {} curve(s) restationed ({} cached station(s))",
        rep.patches.tangent_records,
        rep.patches.cache_records,
        if rep.patches.placeholder_box { ", placeholder box" } else { "" },
        rep.arcs_restationed,
        rep.station_copies
    );
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    let reread =
        raw::open(&a[2]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    let after = find(&reread.doc, &format!("{:X}", before.handle));
    println!("verify {}: length {:.4}", a[2], after.length());
    let mut sta = after.stored_start_station.unwrap_or(0.0);
    for e in &after.elements {
        let (s, t, len) = (e.segment.start(), e.segment.end(), e.segment.length());
        let what = match e.segment {
            Segment::Line { .. } => "tangent".to_string(),
            Segment::Arc { geometry: g, .. } => format!(
                "arc R={:.4} {} delta={:.4}° center ({:.4}, {:.4})",
                g.radius,
                if g.counter_clockwise { "left" } else { "right" },
                g.sweep.to_degrees(),
                g.center.x,
                g.center.y
            ),
        };
        println!(
            "  {what}  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}  sta {:.2}-{:.2}",
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
