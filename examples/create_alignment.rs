//! Create a new alignment in a Civil 3D drawing (D-021).
//!
//! Usage:
//!   cargo run --release --example create_alignment -- donor.dwg target.dwg out.dwg <name> "x,y x,y"
//!       [--style <name|hex>] [--template <hex>] [--type <centerline|misc|n>]
//!       [--no-label-group] [--no-parcel-entry] [--no-graphics]
//!
//! `donor.dwg` is any drawing with an alignment made by Civil 3D (the
//! template; default: its alignment with the highest handle, or
//! `--template`). Its records are copied into the target with new handles,
//! a new registry entry, the given name and PIs (any number). A PI written
//! `x,y,r` gets a free curve (fillet) of radius `r`. `--style` picks an
//! alignment style of the target (default: the template's, which must then
//! exist in the target). `--type` sets the alignment type (1 = Centerline,
//! 4 = Miscellaneous; S58–S62). Other numbers are written as given for
//! experiments: 2, 3 and 5 on a plain alignment are rejected (S61/S63).

use civil3drust::aecc_create::{create_alignment, CreateOptions};
use civil3drust::aecc_elements::decode_elements;
use civil3drust::alignment::Point2;
use civil3drust::raw;

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

/// PIs as "x,y" or "x,y,radius"; returns the PIs and per-interior-PI radii.
fn parse_pis(s: &str) -> (Vec<Point2>, Vec<Option<f64>>) {
    let mut pis = Vec::new();
    let mut radii = Vec::new();
    let items: Vec<&str> = s.split_whitespace().collect();
    for (i, p) in items.iter().enumerate() {
        let f: Vec<f64> = p
            .split(',')
            .map(|v| {
                v.parse()
                    .unwrap_or_else(|_| fail(format!("bad number in '{p}'")))
            })
            .collect();
        if f.len() < 2 || f.len() > 3 {
            fail(format!("point '{p}' must be x,y or x,y,r"));
        }
        pis.push(Point2::new(f[0], f[1]));
        let interior = i > 0 && i + 1 < items.len();
        if f.len() == 3 && !interior {
            fail(format!("'{p}': only interior PIs can have a curve"));
        }
        if interior {
            radii.push(f.get(2).copied());
        }
    }
    (pis, radii)
}

fn main() {
    let mut a: Vec<String> = std::env::args().collect();
    let mut take = |flag: &str| -> Option<String> {
        let i = a.iter().position(|x| x == flag)?;
        if i + 1 >= a.len() {
            fail(format!("{flag} needs a value"));
        }
        let v = a[i + 1].clone();
        a.drain(i..=i + 1);
        Some(v)
    };
    let style_arg = take("--style");
    let template = take("--template").map(|h| {
        u64::from_str_radix(h.trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| fail("--template takes a hex handle"))
    });
    let alignment_type = take("--type").map(|t| match t.to_ascii_lowercase().as_str() {
        "centerline" => civil3drust::aecc_build::TYPE_CENTERLINE,
        "misc" | "miscellaneous" => civil3drust::aecc_build::TYPE_MISCELLANEOUS,
        n => n
            .parse()
            .unwrap_or_else(|_| fail("--type takes centerline, misc or a number")),
    });
    let no_label = a.iter().any(|x| x == "--no-label-group");
    let no_parcel = a.iter().any(|x| x == "--no-parcel-entry");
    let no_graphics = a.iter().any(|x| x == "--no-graphics");
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
    let (pis, radii) = parse_pis(&a[5]);
    let style = style_arg.map(|s| {
        civil3drust::aecc_alignment_refs::alignment_styles(&target.doc)
            .into_iter()
            .find(|(h, n)| {
                n == &s || u64::from_str_radix(s.trim_start_matches("0x"), 16).ok() == Some(*h)
            })
            .map(|x| x.0)
            .unwrap_or_else(|| fail(format!("target has no alignment style '{s}'")))
    });
    let c = create_alignment(
        &mut target.doc,
        &donor.doc,
        &a[4],
        &pis,
        &radii,
        seed,
        CreateOptions {
            label_group: !no_label,
            parcel_entry: !no_parcel,
            style,
            template,
            graphics: !no_graphics,
            alignment_type,
        },
    )
    .unwrap_or_else(|e| fail(e));
    println!(
        "created '{}': alignment {:X}, style {:X}, companion {:X}, label group {:X} (0 = not written){}",
        c.name,
        c.alignment,
        c.style,
        c.companion,
        c.label_group,
        if c.companion != 0 {
            format!(
                ", registry {:X} key {:X}, ID {}",
                c.registry, c.registry_key, c.id_string
            )
        } else {
            String::new()
        }
    );
    if c.companion != 0 {
        println!(
            "companion graph {}",
            if c.companion_rebuilt {
                "rebuilt"
            } else {
                "kept from the template (unknown form)"
            }
        );
    }
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
                let t = civil3drust::aecc_build::alignment_type(&rec)
                    .map(|t| t.to_string())
                    .unwrap_or_else(|_| "?".into());
                println!(
                    "  [{:X}] {} (type {t}): length {:.4}, starts {}",
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
