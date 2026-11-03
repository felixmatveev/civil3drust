//! MIST3 experiment: drop the "date modified" block (`BL 0x1600101F` +
//! 64-bit FILETIME) of one alignment in the Alignment Properties form, for
//! the owner to open in Civil 3D.
//!
//! Modes:
//! - `drop-stamp`: remove the 98-bit block only; everything else as is.
//! - `drop-stamp-mist2`: also remove MIST2 and set the Properties-form
//!   marker to "MIST2 absent" (`0100000001` → `0011000001`, as S159).
//! - `plain`: turn the record into the plain form Civil 3D writes for an
//!   alignment never edited in Alignment Properties: section-2 bits 115 up
//!   to the MIST1 anchor become `BL 0x1148, BL 0` (S24, S155), and the
//!   block and MIST2 are removed.
//!
//! Usage: cargo run --release --example exp_mist3 -- in.dwg out.dwg <alignment handle> <mode>

use civil3drust::aecc_build::{geometry_key_bit, resave_field_bit, Bits, GEOMETRY_KEY_BITS};
use civil3drust::raw;

const STAMP_TAG: u32 = 0x1600_101F;
const ANCHOR_BITS: usize = 14;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        fail(
            "usage: exp_mist3 in.dwg out.dwg <alignment handle> drop-stamp|drop-stamp-mist2|plain",
        );
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
    let s2 = civil3drust::aecc_entity::entity_common(d)
        .unwrap_or_else(|| fail("no common entity data"))
        .start_bit;
    let key = geometry_key_bit(d).unwrap_or_else(|| fail("MIST1 not found"));
    let stamp = key + GEOMETRY_KEY_BITS;
    if raw::get_bit(d, stamp) != 0
        || raw::get_bit(d, stamp + 1) != 0
        || raw::read_bl(d, stamp).map(|x| x.0 as u32) != Some(STAMP_TAG)
    {
        fail("no date-modified block after MIST1 (not the Properties form?)");
    }
    let stamp_end = stamp + 34 + 64;
    let ticks = (0..8).fold(0u64, |v, i| {
        v | (raw::get_byte(d, stamp + 34 + 8 * i) as u64) << (8 * i)
    });
    println!("date-modified block at bit {stamp}: FILETIME {ticks}");
    let mist2 = resave_field_bit(d).filter(|&q| q == stamp_end);
    let marker = s2 + 115;
    let marker_bits: String = (marker..marker + 10)
        .map(|b| char::from(b'0' + raw::get_bit(d, b)))
        .collect();

    // Pieces to cut (start, end) and what to write instead, in order.
    let mut cuts: Vec<(usize, usize, Bits)> = vec![];
    match a[4].as_str() {
        "drop-stamp" => cuts.push((stamp, stamp_end, Bits::default())),
        "drop-stamp-mist2" => {
            let q = mist2.unwrap_or_else(|| fail("no MIST2 after the block"));
            if marker_bits != "0100000001" {
                fail(format!("unexpected marker {marker_bits}"));
            }
            let mut m = Bits::default();
            for c in "0011000001".bytes() {
                m.bit(c - b'0');
            }
            cuts.push((marker, marker + 10, m));
            cuts.push((stamp, q + 44, Bits::default()));
        }
        "plain" => {
            let anchor = key - ANCHOR_BITS;
            let mut m = Bits::default();
            m.bl(0x1148);
            m.bl(0);
            cuts.push((marker, anchor, m));
            let end = mist2.map_or(stamp_end, |q| q + 44);
            cuts.push((stamp, end, Bits::default()));
        }
        m => fail(format!("unknown mode {m}")),
    }
    let total = raw::bit_len(d);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    let mut at = 0;
    for (from, to, with) in cuts {
        w.copy(d, at, from);
        w.0.extend(with.0);
        at = to;
    }
    w.copy(d, at, data_end);
    let new_data_end = w.len();
    w.copy(d, data_end, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    println!("record {} -> {} bits of data", data_end, new_data_end);
    if !raw::set_record(&mut opened.doc, handle, bytes, hb) {
        fail("could not replace the record");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    println!("{}: wrote {}", a[4], a[2]);
}
