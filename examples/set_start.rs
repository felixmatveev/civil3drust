//! Set the start station of a Civil 3D alignment (O4).
//!
//! Usage:
//!   cargo run --release --example set_start -- in.dwg out.dwg <alignment> <station> [--clear-speeds]
//!
//! Design speeds are kept and shifted with the start (D-031); Civil 3D
//! instead removes them (S82), which `--clear-speeds` does. docs/alignment-object.md, "Start station".

use acadrust::entities::EntityType;
use civil3drust::aecc_design_speed::clear_design_speeds;
use civil3drust::aecc_station::{move_start_station, set_start_station, start_station};
use civil3drust::{handle_refs, raw};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let mut a: Vec<String> = std::env::args().collect();
    let clear = a.iter().any(|x| x == "--clear-speeds");
    a.retain(|x| x != "--clear-speeds");
    if a.len() != 5 {
        eprintln!("usage: set_start <in.dwg> <out.dwg> <alignment> <station> [--clear-speeds]");
        std::process::exit(2);
    }
    let station: f64 = a[4].parse().unwrap_or_else(|_| fail("bad station"));
    let mut o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let rec = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| {
            raw::object_name(&r.data).as_deref() == Some(a[3].as_str())
                || u64::from_str_radix(a[3].trim_start_matches("0x"), 16).ok() == Some(r.handle)
        })
        .unwrap_or_else(|| fail(format!("no alignment '{}'", a[3])));
    let mut rec2 = rec.clone();
    if clear {
        let (d, hb) = clear_design_speeds(&rec2).unwrap_or_else(|e| fail(e));
        rec2.data = d;
        rec2.handle_bits = hb;
    }
    let moved = if clear {
        set_start_station(&rec2, station)
    } else {
        move_start_station(&rec2, station)
    };
    let (d, hb) = moved.unwrap_or_else(|| fail("start station not found"));
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
        .unwrap();
    println!(
        "[{:X}] start station {:?}, design speeds {:?}, first interval station {:?}",
        r.handle,
        start_station(&r),
        civil3drust::aecc_design_speed::design_speeds(&r)
            .map(|v| v.iter().map(|e| (e.station, e.speed)).collect::<Vec<_>>()),
        civil3drust::aecc_design_speed::first_interval_station(&r)
    );
}
