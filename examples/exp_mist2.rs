//! MIST2 experiment: change the optional section-2 field `BL 0x0D00D773,
//! BL n` of one alignment, for the owner to open in Civil 3D.
//!
//! Modes: `zero-first` (first BL → 0), `zero-second` (second BL → 0),
//! `zero-both`, `remove` (drop the 44 bits; section 2 back to 413 bits).
//! Zeroed values keep their encoding so the rest of the record stays in
//! place.
//!
//! Usage: cargo run --release --example exp_mist2 -- in.dwg out.dwg <alignment handle> <mode>

use civil3drust::aecc_build::{geometry_key_bit, Bits, GEOMETRY_KEY_BITS};
use civil3drust::raw;

const FIRST: u32 = 0x0D00_D773;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        fail("usage: exp_mist2 in.dwg out.dwg <alignment handle> zero-first|zero-second|zero-both|remove");
    }
    let handle = u64::from_str_radix(&a[3], 16).unwrap_or_else(|_| fail("bad handle"));
    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        fail(format!("read in failsafe mode ({r})"));
    }
    let rec = raw::records(&opened.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == handle)
        .unwrap_or_else(|| fail("alignment not found"));
    let d = &rec.data;
    let key = geometry_key_bit(d).unwrap_or_else(|| fail("MIST1 not found"));
    // MIST2 follows MIST1 directly, or 98 bits later in the Alignment
    // Properties forms (docs/mysteries.md).
    let at = [0usize, 98]
        .iter()
        .map(|o| key + GEOMETRY_KEY_BITS + o)
        .find(|&q| raw::read_bl(d, q).map(|x| x.0 as u32) == Some(FIRST) && raw::get_bit(d, q) == 0)
        .unwrap_or_else(|| fail("MIST2 not found (record never re-saved by Civil 3D?)"));
    let (_, second_at) = raw::read_bl(d, at).unwrap();
    let (second, end) = raw::read_bl(d, second_at).unwrap();
    if (raw::get_bit(d, second_at), raw::get_bit(d, second_at + 1)) != (0, 1) || end - at != 44 {
        fail("MIST2 not in the expected 44-bit form");
    }
    println!("MIST2 at bit {at}: BL {FIRST:#010x}, BL {second}");

    let (data, hb) = match a[4].as_str() {
        "zero-first" | "zero-second" | "zero-both" => {
            let mut data = d.clone();
            if a[4] != "zero-second" {
                for i in 0..4 {
                    raw::set_byte(&mut data, at + 2 + 8 * i, 0);
                }
            }
            if a[4] != "zero-first" {
                raw::set_byte(&mut data, second_at + 2, 0);
            }
            (data, rec.handle_bits)
        }
        "remove" => {
            let total = raw::bit_len(d);
            let data_end = total - rec.handle_bits;
            let mut w = Bits::default();
            w.copy(d, 0, at);
            w.copy(d, end, data_end);
            let new_data_end = w.len();
            w.copy(d, data_end, total);
            let bytes = w.into_bytes();
            let hb = bytes.len() * 8 - new_data_end;
            (bytes, hb)
        }
        m => fail(format!("unknown mode {m}")),
    };
    if !raw::set_record(&mut opened.doc, handle, data, hb) {
        fail("could not replace the record");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    println!("{}: wrote {}", a[4], a[2]);
}
