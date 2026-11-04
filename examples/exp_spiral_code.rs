//! Experiment (S233, S234): change the spiral-type code (MIST10) of fixed
//! spiral records, nothing else (the proxy graphics are left as they
//! were). Codes 263 and 264 were never seen in a sample; 264 is the
//! owner's guess for NSW Cubic Parabola. S233 (263 and 264) crashes Civil
//! 3D on open; S234 has 264 only.
//!
//! Usage: exp_spiral_code in.dwg out.dwg <alignment hex>:<code> ...

use civil3drust::aecc_alignment::read_header;
use civil3drust::aecc_build::Bits;
use civil3drust::aecc_elements::{element_records, Element};
use civil3drust::aecc_encode::encode_element;
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        fail("usage: exp_spiral_code in.dwg out.dwg <alignment hex>:<code> ...");
    }
    let mut doc = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    for spec in &a[3..] {
        let (h, code) = spec.split_once(':').unwrap_or_else(|| fail("hex:code"));
        let h = u64::from_str_radix(h, 16).unwrap_or_else(|_| fail("handle"));
        let code: i32 = code.parse().unwrap_or_else(|_| fail("code"));
        let rec = raw::records(&doc.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap_or_else(|| fail(format!("{h:X} not found")));
        let data = &rec.data;
        let total = raw::bit_len(data);
        let data_end = total - rec.handle_bits;
        let mut w = Bits::default();
        let mut at = 0;
        let mut n = 0;
        for el in element_records(data) {
            let Element::Spiral(mut s) = el else { continue };
            let hd = read_header(data, s.header_bit).unwrap_or_else(|| fail("header"));
            let from = s.header_bit - 11;
            w.copy(data, at, from);
            let old = s.mode;
            s.mode = code;
            let end = s.end_bit;
            encode_element(&mut w, &hd, &Element::Spiral(s));
            at = end;
            n += 1;
            println!("{h:X}: spiral code {old} -> {code}");
        }
        if n == 0 {
            fail(format!("{h:X}: no spiral record"));
        }
        w.copy(data, at, data_end);
        let new_end = w.len();
        w.copy(data, data_end, total);
        let bytes = w.into_bytes();
        let hb = bytes.len() * 8 - new_end;
        if !raw::set_record(&mut doc.doc, h, bytes, hb) {
            fail("could not write");
        }
    }
    acadrust::DwgWriter::write_to_file(&a[2], &doc.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
