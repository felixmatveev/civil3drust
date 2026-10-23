//! Set the design speeds of a Civil 3D alignment and save the drawing (O2).
//!
//! Usage:
//!   cargo run --release --example set_speeds -- in.dwg out.dwg <alignment> "station:speed ..."
//!
//! `<alignment>` is the name or hex handle. Speeds are in the drawing's
//! speed unit (mph in S74). Example: "0:60 200:30". Without a speed list the
//! current design speeds are printed. docs/alignment-object.md, "Design
//! speeds".

use acadrust::entities::EntityType;
use civil3drust::aecc_design_speed::{design_speeds, set_design_speeds};
use civil3drust::{handle_refs, raw};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() == 2 {
        let o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
        for r in raw::records(&o.doc, "AECC_ALIGNMENT") {
            let name = raw::object_name(&r.data).unwrap_or_default();
            match design_speeds(&r) {
                Ok(v) => {
                    let s: Vec<String> = v
                        .iter()
                        .map(|e| format!("{}:{}", e.station, e.speed))
                        .collect();
                    println!("[{:X}] {name}: {}", r.handle, s.join(" "));
                }
                Err(e) => println!("[{:X}] {name}: {e}", r.handle),
            }
        }
        return;
    }
    if a.len() != 5 {
        eprintln!("usage: set_speeds <in.dwg> [out.dwg <alignment> \"station:speed ...\"]");
        std::process::exit(2);
    }
    let speeds: Vec<(f64, f64)> = a[4]
        .split_whitespace()
        .map(|p| {
            let (st, sp) = p
                .split_once(':')
                .unwrap_or_else(|| fail(format!("'{p}' must be station:speed")));
            (
                st.parse()
                    .unwrap_or_else(|_| fail(format!("bad station '{st}'"))),
                sp.parse()
                    .unwrap_or_else(|_| fail(format!("bad speed '{sp}'"))),
            )
        })
        .collect();
    let mut o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let rec = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| {
            raw::object_name(&r.data).as_deref() == Some(a[3].as_str())
                || u64::from_str_radix(a[3].trim_start_matches("0x"), 16).ok() == Some(r.handle)
        })
        .unwrap_or_else(|| fail(format!("no alignment '{}'", a[3])));
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    let (d, hb) = set_design_speeds(&rec, &speeds, seed).unwrap_or_else(|e| fail(e));
    let (d, hb) = handle_refs::trim_padding(&d, hb, rec.handle);
    for e in o.doc.entities_mut() {
        if let EntityType::Unknown(u) = e {
            if u.common.handle.value() == rec.handle {
                u.raw_dwg_data = Some(d.clone());
                u.dwg_handle_bits = hb as i64;
            }
        }
    }
    acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
    let re = raw::open(&a[2]).unwrap_or_else(|e| fail(format!("does not read back: {e}")));
    let r = raw::records(&re.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == rec.handle)
        .unwrap_or_else(|| fail("alignment missing after write"));
    let v = design_speeds(&r).unwrap_or_else(|e| fail(e));
    println!(
        "[{:X}] design speeds: {}",
        r.handle,
        v.iter()
            .map(|e| format!("{}:{}", e.station, e.speed))
            .collect::<Vec<_>>()
            .join(" ")
    );
}
