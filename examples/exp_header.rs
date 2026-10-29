//! MIST7 experiment: replace the element header (`0x04020008`, the first
//! BL of every element record) of one alignment with another 32-bit value,
//! for the owner to open in Civil 3D. With `keep` the drawing is written
//! unchanged (the control).
//!
//! Usage: cargo run --release --example exp_header -- in.dwg out.dwg <alignment handle> <hex value|keep> [element id]

use civil3drust::aecc_alignment::ELEMENT_HEADER;
use civil3drust::aecc_elements::decode_elements;
use civil3drust::raw;

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 {
        fail("usage: exp_header in.dwg out.dwg <alignment handle> <hex value|keep> [element id]");
    }
    let handle = u64::from_str_radix(&a[3], 16).unwrap_or_else(|_| fail("bad handle"));
    let value = if a[4] == "keep" {
        None
    } else {
        Some(
            u32::from_str_radix(a[4].trim_start_matches("0x"), 16)
                .unwrap_or_else(|_| fail("bad hex value")),
        )
    };
    let only: Option<i32> = a
        .get(5)
        .map(|s| s.parse().unwrap_or_else(|_| fail("bad id")));

    let mut opened = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &opened.failsafe_reason {
        fail(format!("read in failsafe mode ({r})"));
    }
    let rec = raw::records(&opened.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == handle)
        .unwrap_or_else(|| fail("alignment not found"));
    let d = decode_elements(&rec).unwrap_or_else(|e| fail(e));
    if let Some(v) = value {
        let data = raw::record_data_mut(&mut opened.doc, handle)
            .unwrap_or_else(|| fail("record bytes missing"));
        let mut n = 0;
        for e in &d.elements {
            let id = civil3drust::aecc_alignment::Linked::id(&e.record);
            if only.is_some_and(|o| o != id) {
                continue;
            }
            let b = e.record.header_bit();
            // Full-form BL: code `00`, then 4 bytes, low byte first.
            let (old, _) = raw::read_bl(data, b).unwrap_or_else(|| fail("header not readable"));
            if old != ELEMENT_HEADER || raw::get_bit(data, b) != 0 || raw::get_bit(data, b + 1) != 0
            {
                fail(format!("element {id}: header is not 0x04020008"));
            }
            for (i, byte) in v.to_le_bytes().iter().enumerate() {
                raw::set_byte(data, b + 2 + 8 * i, *byte);
            }
            println!("element {id} at bit {b}: 0x{old:08X} -> 0x{v:08X}");
            n += 1;
        }
        if n == 0 {
            fail("no element changed");
        }
    } else {
        println!("control: written unchanged");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &opened.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
