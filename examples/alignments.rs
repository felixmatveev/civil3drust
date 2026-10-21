//! List the tangent-only alignments in a DWG, decoded from their tangent
//! element records (see docs/alignment-object.md).
//!
//! Usage: cargo run --release --example alignments -- file.dwg

use civil3drust::aecc_alignment::{decode, record_name};
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
        match decode(rec) {
            Ok(d) => {
                let a = &d.alignment;
                let sta = match d.stored_start_station {
                    Some(s) => format!("stations {:.2}-{:.2}", s, a.end_station()),
                    None => format!(
                        "start station not found; shown from 0 to {:.2}",
                        a.end_station()
                    ),
                };
                println!(
                    "\n[{:X}] {}  {} tangent(s), length {:.4}, {}",
                    d.handle,
                    a.name,
                    a.segment_count(),
                    a.length(),
                    sta
                );
                for (t, r) in a.tangents().iter().zip(&d.tangents) {
                    println!(
                        "  id {:>2} (before {}, after {})  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}  sta {:.2}-{:.2}  @bit {}",
                        r.id,
                        r.before,
                        r.after,
                        t.start.x,
                        t.start.y,
                        t.end.x,
                        t.end.y,
                        t.length(),
                        t.start_station,
                        t.end_station,
                        r.coords_bit
                    );
                }
            }
            Err(e) => println!(
                "\n[{:X}] {}  not decoded: {e}",
                rec.handle,
                record_name(&rec.data).unwrap_or_else(|| "?".into())
            ),
        }
    }
}
