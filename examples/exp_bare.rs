//! Experiment S67: an alignment alone in a plain AutoCAD drawing.
//!
//! Puts one alignment record into a drawing that Civil 3D never saved (no
//! AECC classes, styles, registry or settings): the `AECC_ALIGNMENT` class
//! entry is added, the companion references and the style reference are
//! null. The question is whether Civil 3D builds its own drawing settings
//! on open (as it does when saving such a drawing, S64 → S65) and accepts the
//! alignment, assigns a style, or removes it (RECOVER).
//!
//! Usage: cargo run --release --example exp_bare -- donor.dwg base.dwg plain.dwg out.dwg
//!
//! `donor.dwg` provides the template alignment, `base.dwg` a Civil 3D drawing
//! the record is first built in (with the donor's style), `plain.dwg` the
//! AutoCAD drawing to put it in.

use acadrust::entities::EntityType;
use acadrust::types::Handle;
use civil3drust::aecc_create::{create_alignment, ensure_class, set_object_type, CreateOptions};
use civil3drust::alignment::Point2;
use civil3drust::{handle_refs, raw};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        eprintln!("usage: exp_bare <donor.dwg> <base.dwg> <plain.dwg> <out.dwg>");
        std::process::exit(2);
    }
    let donor = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let mut base = raw::open(&a[2]).unwrap_or_else(|e| fail(e));
    let mut plain = raw::open(&a[3]).unwrap_or_else(|e| fail(e));
    let pis = [Point2::new(0.0, 0.0), Point2::new(500.0, 0.0)];
    let opts = CreateOptions {
        label_group: false,
        parcel_entry: false,
        ..Default::default()
    };
    let c = create_alignment(&mut base.doc, &donor.doc, "Bare", &pis, &[], 1, opts)
        .unwrap_or_else(|e| fail(e));
    let rec = raw::records(&base.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == c.alignment)
        .unwrap();
    let new_h = plain.doc.allocate_handle().value();
    let (d, hb) = handle_refs::set_own_handle(&rec.data, rec.handle_bits, new_h)
        .unwrap_or_else(|| fail("own handle"));
    let n_refs = handle_refs::of_record(&raw::RawRecord {
        data: d.clone(),
        handle_bits: hb,
        handle: new_h,
        ..rec.clone()
    })
    .len();
    // layer "0" (10), everything else null.
    let mut targets = vec![0u64; n_refs];
    targets[0] = 0x10;
    let (mut d, hb) =
        handle_refs::rewrite_refs(&d, hb, new_h, &targets).unwrap_or_else(|| fail("refs"));
    let class =
        ensure_class(&mut plain.doc, &donor.doc, "AECC_ALIGNMENT").unwrap_or_else(|e| fail(e));
    set_object_type(&mut d, class).unwrap_or_else(|e| fail(e));
    let mut e = base
        .doc
        .entities()
        .find(|e| e.common().handle.value() == c.alignment)
        .cloned()
        .unwrap();
    if let EntityType::Unknown(u) = &mut e {
        u.common.handle = Handle::new(new_h);
        u.common.owner_handle = Handle::NULL;
        u.raw_dwg_data = Some(d);
        u.dwg_handle_bits = hb as i64;
    }
    plain.doc.add_entity(e).unwrap_or_else(|e| fail(e));
    acadrust::DwgWriter::write_to_file(&a[4], &plain.doc).unwrap_or_else(|e| fail(e));
    let re = raw::open(&a[4]).unwrap_or_else(|e| fail(format!("does not read back: {e}")));
    for r in raw::records(&re.doc, "AECC_ALIGNMENT") {
        println!("[{:X}] {} bytes, class {class}", r.handle, r.data.len());
    }
}
