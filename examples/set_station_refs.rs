//! Write station equations and masking regions (S243–S248; D-068).
//!
//! Usage:
//!   set_station_refs in.dwg out.dwg <alignment> equations "raw:ahead[:dec][:comment] ..."
//!   set_station_refs in.dwg out.dwg <alignment> masking "start-end[:off][:lockstart][:lockend][:comment] ..."
//!
//! Raw stations are start station + distance along the alignment. An empty
//! list ("") removes them all. Comments may not contain ':' or spaces.

use civil3drust::aecc_station_refs::{
    set_masking_regions, set_station_equations, EquationSpec, MaskSpec,
};
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 6 {
        fail("usage: set_station_refs in.dwg out.dwg <alignment> equations|masking \"...\"");
    }
    let mut doc = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let rec = raw::records(&doc.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| {
            raw::object_name(&r.data).as_deref() == Some(a[3].as_str())
                || u64::from_str_radix(&a[3], 16).ok() == Some(r.handle)
        })
        .unwrap_or_else(|| fail(format!("no alignment {}", a[3])));
    let num = |s: &str| -> f64 { s.parse().unwrap_or_else(|_| fail(format!("number: {s}"))) };
    let seed = 0x5EED_0250u64 ^ rec.handle;
    let (bytes, hb) = match a[4].as_str() {
        "equations" => {
            let specs: Vec<EquationSpec> = a[5]
                .split_whitespace()
                .map(|t| {
                    let p: Vec<&str> = t.split(':').collect();
                    if p.len() < 2 {
                        fail(format!("equation: {t}"));
                    }
                    let dec = p.get(2) == Some(&"dec");
                    let comment = p[2..].iter().find(|x| **x != "dec" && **x != "inc");
                    EquationSpec {
                        raw_station: num(p[0]),
                        ahead: num(p[1]),
                        increasing: !dec,
                        comment: comment.map(|c| c.to_string()).unwrap_or_default(),
                    }
                })
                .collect();
            set_station_equations(&rec, &specs, seed)
        }
        "masking" => {
            let specs: Vec<MaskSpec> = a[5]
                .split_whitespace()
                .map(|t| {
                    let p: Vec<&str> = t.split(':').collect();
                    let (s, e) = p[0]
                        .split_once('-')
                        .unwrap_or_else(|| fail(format!("region: {t}")));
                    let flags = &p[1..];
                    MaskSpec {
                        start: num(s),
                        end: num(e),
                        masked: !flags.contains(&"off"),
                        lock_to_start: flags.contains(&"lockstart"),
                        lock_to_end: flags.contains(&"lockend"),
                        comment: flags
                            .iter()
                            .find(|x| !["off", "lockstart", "lockend"].contains(x))
                            .map(|c| c.to_string())
                            .unwrap_or_default(),
                    }
                })
                .collect();
            set_masking_regions(&rec, &specs, seed)
        }
        other => fail(format!("unknown kind {other}")),
    }
    .unwrap_or_else(|e| fail(e));
    if !raw::set_record(&mut doc.doc, rec.handle, bytes, hb) {
        fail("could not write");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &doc.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
