//! Experiment (S200): which section-2 form makes Civil 3D read an
//! alignment's JSON layout string.
//!
//! Every Civil 3D record with a JSON string is in the Alignment Properties
//! form with section-2 bits 115… = `BL 3, BL 0x01010034, BL 0x08010004`
//! (S178; `BL 0x43` with MIST2, S177/S181), where an alignment without it
//! has `BL 1, BL 0x08010004` (S177 FC8) or, in the plain form,
//! `BL 0x1148, BL 0` (S24). Generated alignments in the plain form with a
//! JSON string are rejected (S197, eWrongObjectType). Variants, applied to
//! plain-form alignments:
//!
//! - `A`: the donor's Properties-form region (115 up to MIST1) and its
//!   date-modified block after MIST1 (MIST3), copied bit for bit, plus the
//!   string the Properties form has after the JSON (the user who edited
//!   it, "felix" in S177/S178; written as "civil3drust");
//! - `B`: plain form with the first BL `0x1148 | 2 = 0x114A`;
//! - `C`: as B, with `BL 0x01010034` inserted after it.
//!
//! Usage: exp_json_form in.dwg out.dwg donor.dwg:HANDLE A:h,h B:h,h C:h,h

use civil3drust::aecc_build::{geometry_key_bit, Bits, GEOMETRY_KEY_BITS};
use civil3drust::aecc_entity::entity_common;
use civil3drust::raw::{self, RawRecord};
use civil3drust::string_stream;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

/// The section-2 region from bit 115 up to the start of MIST1's anchor
/// (14 bits before MIST1), and the bits after MIST1 up to `upto` bits.
fn region(data: &[u8]) -> (usize, usize) {
    let s2 = entity_common(data).unwrap().start_bit;
    let key = geometry_key_bit(data).unwrap();
    (s2 + 115, key - 14)
}

fn rewrite(rec: &RawRecord, variant: char, donor: &RawRecord) -> (Vec<u8>, usize) {
    let data = &rec.data;
    let (r0, r1) = region(data);
    let key = geometry_key_bit(data).unwrap();
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, r0);
    match variant {
        'A' => {
            let (d0, d1) = region(&donor.data);
            w.copy(&donor.data, d0, d1);
            w.copy(data, r1, key + GEOMETRY_KEY_BITS);
            // MIST3: BL 0x1600101F (34 bits) + 64-bit FILETIME.
            let dk = geometry_key_bit(&donor.data).unwrap() + GEOMETRY_KEY_BITS;
            w.copy(&donor.data, dk, dk + 98);
            w.copy(data, key + GEOMETRY_KEY_BITS, data_end);
        }
        'B' | 'C' => {
            let (v, k) = raw::read_bl(data, r0).unwrap();
            if v != 0x1148 {
                fail(format!("{:X}: not the plain form ({v:#x})", rec.handle));
            }
            w.bits(0, 2);
            for b in (0x114Au32).to_le_bytes() {
                w.byte(b);
            }
            if variant == 'C' {
                w.bits(0, 2);
                for b in (0x0101_0034u32).to_le_bytes() {
                    w.byte(b);
                }
            }
            w.copy(data, k, data_end);
        }
        _ => fail("variant must be A, B or C"),
    }
    let new_end = w.len();
    w.copy(data, data_end, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_end;
    (bytes, hb)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 {
        fail("usage: exp_json_form in.dwg out.dwg donor.dwg:HANDLE A:h,h B:h,h C:h,h");
    }
    let mut doc = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let (df, dh) = a[3]
        .rsplit_once(':')
        .unwrap_or_else(|| fail("donor.dwg:HANDLE"));
    let dh = u64::from_str_radix(dh, 16).unwrap_or_else(|_| fail("donor handle"));
    let dd = raw::open(df).unwrap_or_else(|e| fail(e));
    let donor = raw::records(&dd.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == dh)
        .unwrap_or_else(|| fail("donor alignment not found"));
    for spec in &a[4..] {
        let (v, hs) = spec
            .split_once(':')
            .unwrap_or_else(|| fail("variant:handles"));
        let variant = v.chars().next().unwrap();
        for h in hs.split(',') {
            let h = u64::from_str_radix(h, 16).unwrap_or_else(|_| fail("handle"));
            let rec = raw::records(&doc.doc, "AECC_ALIGNMENT")
                .into_iter()
                .find(|r| r.handle == h)
                .unwrap_or_else(|| fail(format!("{h:X} not found")));
            let (mut bytes, mut hb) = rewrite(&rec, variant, &donor);
            if variant == 'A' {
                // The Properties form's extra string after the JSON (slot 4).
                let (b, h2) = string_stream::insert_empty(&bytes, hb, 4).unwrap();
                (bytes, hb) = string_stream::replace(&b, h2, 4, "civil3drust").unwrap();
            }
            if !raw::set_record(&mut doc.doc, h, bytes, hb) {
                fail(format!("{h:X}: could not write"));
            }
            println!("{h:X}: variant {variant}");
        }
    }
    acadrust::DwgWriter::write_to_file(&a[2], &doc.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
