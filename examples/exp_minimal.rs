//! Experiment S56: which parts of an alignment record can be empty?
//!
//! Builds one drawing from S24 with several created alignments; each one has
//! a single part emptied (docs/alignment-map.md, "Experiments"). If Civil 3D
//! rejects a part, RECOVER removes only that alignment, which identifies it.
//!
//! Usage: cargo run --release --example exp_minimal -- donor.dwg target.dwg out.dwg
//!
//! - `X0 control`     — unchanged (as S50/S53)
//! - `X1 zero-hash`   — the 66-bit varying field in section 2 set to zero bits
//! - `X2 no-graphics` — has-graphics bit cleared, proxy graphics removed
//! - `X3 zero-guid`   — both copies of the identity GUID zeroed
//! - `X4 no-label`    — no station-equation label group (known optional)
//! - `X5 minimal`     — X1 + X2 + X3 + X4 together

use acadrust::entities::EntityType;
use civil3drust::aecc_create::{create_alignment, CreateOptions};
use civil3drust::aecc_elements::{decode_elements, Element};
use civil3drust::alignment::Point2;
use civil3drust::{proxy_graphics as pg, raw};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn bits_of(d: &[u8]) -> Vec<u8> {
    (0..d.len() * 8).map(|i| raw::get_bit(d, i)).collect()
}

fn pack(b: &[u8]) -> Vec<u8> {
    let mut b = b.to_vec();
    while !b.len().is_multiple_of(8) {
        b.push(0);
    }
    let mut out = vec![0u8; b.len() / 8];
    for (i, x) in b.iter().enumerate() {
        out[i / 8] |= x << (7 - i % 8);
    }
    out
}

/// Section-2 start (graphics end) and the element-list end.
fn marks(rec: &raw::RawRecord) -> (usize, usize) {
    let g = pg::find(&rec.data, 4096).unwrap_or_else(|| fail("no graphics"));
    let d = decode_elements(rec).unwrap_or_else(|e| fail(e));
    let end = d
        .elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => t.end_bit(),
            Element::Arc(a) => a.end_bit,
        })
        .max()
        .unwrap();
    (g.end_bit(), end)
}

fn zero_hash(rec: &raw::RawRecord) -> (Vec<u8>, usize) {
    let (s2, _) = marks(rec);
    let mut d = rec.data.clone();
    for b in s2 + 165..s2 + 231 {
        let (i, m) = (b / 8, 1u8 << (7 - b % 8));
        d[i] &= !m;
    }
    (d, rec.handle_bits)
}

fn zero_guid(rec: &raw::RawRecord) -> (Vec<u8>, usize) {
    let (_, end) = marks(rec);
    let mut d = rec.data.clone();
    let a = end + 16;
    let first: Vec<u8> = (0..16).map(|i| raw::get_byte(&d, a + 8 * i)).collect();
    let n = d.len() * 8;
    let b = (a + 128..n - 128)
        .find(|&b| (0..16).all(|i| raw::get_byte(&d, b + 8 * i) == first[i]))
        .unwrap_or_else(|| fail("second GUID copy"));
    for pos in [a, b] {
        for i in 0..16 {
            raw::set_byte(&mut d, pos + 8 * i, 0);
        }
    }
    (d, rec.handle_bits)
}

fn no_graphics(rec: &raw::RawRecord) -> (Vec<u8>, usize) {
    let (s2, _) = marks(rec);
    let bits = bits_of(&rec.data);
    // bits 42..43 EED "10", bit 44 has-graphics, 45.. BLL size + graphics.
    assert_eq!((bits[42], bits[43], bits[44]), (1, 0, 1));
    let total = rec.data.len() * 8;
    let data_end = total - rec.handle_bits;
    let mut out = bits[..44].to_vec();
    out.push(0);
    out.extend_from_slice(&bits[s2..]);
    let shrink = s2 - 45;
    let new_data_end = data_end - shrink;
    let packed = pack(&out);
    let hb = packed.len() * 8 - new_data_end;
    (packed, hb)
}

fn apply(
    doc: &mut acadrust::document::CadDocument,
    h: u64,
    f: &dyn Fn(&raw::RawRecord) -> (Vec<u8>, usize),
) {
    let rec = raw::records(doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap();
    let (d, hb) = f(&rec);
    for e in doc.entities_mut() {
        if let EntityType::Unknown(u) = e {
            if u.common.handle.value() == h {
                u.raw_dwg_data = Some(d.clone());
                u.dwg_handle_bits = hb as i64;
            }
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 4 {
        eprintln!("usage: exp_minimal <donor.dwg> <target.dwg> <out.dwg>");
        std::process::exit(2);
    }
    let donor = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let mut t = raw::open(&a[2]).unwrap_or_else(|e| fail(e));
    type Edit = Vec<&'static dyn Fn(&raw::RawRecord) -> (Vec<u8>, usize)>;
    let cases: Vec<(&str, bool, Edit)> = vec![
        ("X0 control", true, vec![]),
        ("X1 zero-hash", true, vec![&zero_hash]),
        ("X2 no-graphics", true, vec![&no_graphics]),
        ("X3 zero-guid", true, vec![&zero_guid]),
        ("X4 no-label", false, vec![]),
        (
            "X5 minimal",
            false,
            vec![&zero_hash, &zero_guid, &no_graphics],
        ),
    ];
    for (k, (name, label, edits)) in cases.into_iter().enumerate() {
        let y = -100.0 * (k as f64 + 1.0);
        let pis = [
            Point2::new(500.0, y),
            Point2::new(600.0, y),
            Point2::new(700.0, y + 40.0),
            Point2::new(800.0, y + 40.0),
        ];
        let opts = CreateOptions {
            label_group: label,
            ..Default::default()
        };
        let c = create_alignment(
            &mut t.doc,
            &donor.doc,
            name,
            &pis,
            &[Some(30.0), None],
            100 + k as u64,
            opts,
        )
        .unwrap_or_else(|e| fail(e));
        for f in edits {
            apply(&mut t.doc, c.alignment, f);
        }
        println!("{name}: alignment {:X}", c.alignment);
    }
    acadrust::DwgWriter::write_to_file(&a[3], &t.doc).unwrap_or_else(|e| fail(e));
    let re = raw::open(&a[3]).unwrap_or_else(|e| fail(format!("does not read back: {e}")));
    for r in raw::records(&re.doc, "AECC_ALIGNMENT") {
        let d = decode_elements(&r).unwrap_or_else(|e| fail(e));
        println!(
            "  [{:X}] {:<16} {} B, length {:.3}, graphics {}",
            r.handle,
            d.name.clone().unwrap_or_default(),
            r.data.len(),
            d.length(),
            pg::find(&r.data, 4096).is_some()
        );
    }
}
