//! Create a new alignment in a Civil 3D drawing (D-021).
//!
//! Usage:
//!   cargo run --release --example create_alignment -- donor.dwg target.dwg out.dwg <name> "x,y x,y" [--no-label-group] [--no-parcel-entry]
//!
//! `donor.dwg` is `target.dwg` plus one alignment created in Civil 3D (e.g.
//! S25 for S24). Its records are copied into the target with new handles,
//! a new ID string, the given name (same length as the donor alignment's
//! name) and the given PIs (same count as the donor alignment's).

use civil3drust::aecc_create::{create_alignment, CreateOptions};
use civil3drust::aecc_elements::decode_elements;
use civil3drust::alignment::Point2;
use civil3drust::raw;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn parse_pis(s: &str) -> Vec<Point2> {
    s.split_whitespace()
        .map(|p| {
            let (x, y) = p
                .split_once(',')
                .unwrap_or_else(|| fail(format!("bad point '{p}'")));
            Point2::new(
                x.parse()
                    .unwrap_or_else(|_| fail(format!("bad x in '{p}'"))),
                y.parse()
                    .unwrap_or_else(|_| fail(format!("bad y in '{p}'"))),
            )
        })
        .collect()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let no_label = a.iter().any(|x| x == "--no-label-group");
    let no_parcel = a.iter().any(|x| x == "--no-parcel-entry");
    let a: Vec<String> = a.into_iter().filter(|x| !x.starts_with("--no-")).collect();
    if a.len() != 6 {
        eprintln!(
            "usage: create_alignment <donor.dwg> <target.dwg> <out.dwg> <name> \"x,y x,y ...\""
        );
        std::process::exit(2);
    }
    let donor = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let mut target = raw::open(&a[2]).unwrap_or_else(|e| fail(e));
    for (f, o) in [(&a[1], &donor), (&a[2], &target)] {
        if let Some(r) = &o.failsafe_reason {
            eprintln!(
                "WARNING: {f} read in failsafe mode; unparsed records will be missing. ({r})"
            );
        }
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    let c = create_alignment(
        &mut target.doc,
        &donor.doc,
        &a[4],
        &parse_pis(&a[5]),
        seed,
        CreateOptions {
            label_group: !no_label,
            parcel_entry: !no_parcel,
        },
    )
    .unwrap_or_else(|e| fail(e));
    println!(
        "created '{}': alignment {:X}, companion {:X}, label group {:X} (0 = not written){}",
        c.name,
        c.alignment,
        c.companion,
        c.label_group,
        if c.companion != 0 {
            format!(", registry {:X} entry, ID {}", c.registry, c.id_string)
        } else {
            String::new()
        }
    );
    acadrust::DwgWriter::write_to_file(&a[3], &target.doc).unwrap_or_else(|e| fail(e));

    let re =
        raw::open(&a[3]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    println!("verify {}:", a[3]);
    for rec in raw::records(&re.doc, "AECC_ALIGNMENT") {
        match decode_elements(&rec) {
            Ok(d) => {
                let pts: Vec<String> = d
                    .elements
                    .iter()
                    .map(|e| {
                        let s = e.segment.start();
                        format!("({:.3},{:.3})", s.x, s.y)
                    })
                    .collect();
                println!(
                    "  [{:X}] {}: length {:.4}, starts {}",
                    rec.handle,
                    d.name.as_deref().unwrap_or("?"),
                    d.length(),
                    pts.join(" ")
                );
            }
            Err(e) => println!("  [{:X}] decode error: {e}", rec.handle),
        }
    }
}
