//! Experiment S67: an alignment alone in a plain AutoCAD drawing.
//!
//! Puts one alignment record into a drawing that Civil 3D never saved (no
//! AECC classes, styles, registry or settings): the `AECC_ALIGNMENT` class
//! entry is added, the companion references and the style reference are
//! null. The question is whether Civil 3D builds its own drawing settings
//! on open (as it does when saving such a drawing, S64 → S65) and accepts the
//! alignment, assigns a style, or removes it (RECOVER).
//!
//! Usage: cargo run --release --example exp_bare -- donor.dwg base.dwg plain.dwg out.dwg [style-hex]
//!
//! With `style-hex` (experiment S70) the alignment style with that handle in
//! `base.dwg` is copied too and the alignment points at it. The style is
//! owned by a new dictionary `CIVIL3DRUST_STYLES` in the named object
//! dictionary (S73; with no owner, S70, Civil 3D dropped it); its soft
//! pointer to the default style is nulled; layers and linetypes missing in `plain.dwg` become layer 0 /
//! Continuous.
//!
//! `donor.dwg` provides the template alignment, `base.dwg` a Civil 3D drawing
//! the record is first built in (with the donor's style), `plain.dwg` the
//! AutoCAD drawing to put it in.

use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
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
    if a.len() != 5 && a.len() != 6 {
        eprintln!("usage: exp_bare <donor.dwg> <base.dwg> <plain.dwg> <out.dwg> [style-hex]");
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
    let style = a
        .get(5)
        .map(|h| copy_style(&base, &mut plain, u64::from_str_radix(h, 16).unwrap()));
    // layer "0" (10), style (reference 5) if copied, everything else null.
    let mut targets = vec![0u64; n_refs];
    targets[0] = 0x10;
    if let Some(st) = style {
        targets[5] = st;
    }
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

/// Copy the alignment style `h` of `base` into `plain` with a new handle.
fn copy_style(base: &raw::Opened, plain: &mut raw::Opened, h: u64) -> u64 {
    let rec = raw::records(&base.doc, "AECC_ALIGNMENT_STYLE")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap_or_else(|| fail(format!("no alignment style {h:X} in base")));
    let new_h = plain.doc.allocate_handle().value();
    // Owner: a new dictionary in the named object dictionary (S73). Without
    // an owner Civil 3D drops the style on open (S70 -> S71/S72).
    let dict_h = plain.doc.allocate_handle();
    let nod = plain.doc.header.named_objects_dict_handle;
    match plain.doc.objects.get_mut(&nod) {
        Some(ObjectType::Dictionary(d)) => {
            d.entries.push(("CIVIL3DRUST_STYLES".into(), dict_h));
        }
        _ => fail("plain drawing has no named object dictionary"),
    }
    let mut dict = acadrust::objects::Dictionary::new();
    dict.handle = dict_h;
    dict.owner = nod;
    dict.hard_owner = true;
    dict.entries.push(("TEST STYLE".into(), Handle::new(new_h)));
    plain
        .doc
        .objects
        .insert(dict_h, ObjectType::Dictionary(dict));
    let layers: Vec<u64> = plain.doc.layers.iter().map(|l| l.handle.value()).collect();
    let ltypes: Vec<u64> = plain
        .doc
        .line_types
        .iter()
        .map(|l| l.handle.value())
        .collect();
    let base_layers: Vec<u64> = base.doc.layers.iter().map(|l| l.handle.value()).collect();
    let targets: Vec<u64> = handle_refs::of_record(&rec)
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let t = r.handle;
            if i == 0 {
                dict_h.value() // owner reference
            } else if t == 0 || r.code == 2 || r.code == 4 || r.code >= 6 {
                0 // owner, soft pointers (style collection, default style)
            } else if layers.contains(&t) || ltypes.contains(&t) {
                t
            } else if base_layers.contains(&t) {
                0x10
            } else if raw::class_of(&base.doc, t).is_none() {
                0x16 // a linetype missing in plain: Continuous
            } else {
                0
            }
        })
        .collect();
    let (d, hb) = handle_refs::set_own_handle(&rec.data, rec.handle_bits, new_h)
        .unwrap_or_else(|| fail("style own handle"));
    let (mut d, hb) =
        handle_refs::rewrite_refs(&d, hb, new_h, &targets).unwrap_or_else(|| fail("style refs"));
    let class =
        ensure_class(&mut plain.doc, &base.doc, "AECC_ALIGNMENT_STYLE").unwrap_or_else(|e| fail(e));
    set_object_type(&mut d, class).unwrap_or_else(|e| fail(e));
    let obj = base
        .doc
        .objects
        .iter()
        .find(|(k, _)| k.value() == h)
        .map(|(_, o)| o.clone())
        .unwrap();
    let ObjectType::Unknown {
        type_name,
        raw_dxf_codes,
        raw_dwg_version,
        ..
    } = obj
    else {
        fail("style is not a raw object");
    };
    plain.doc.objects.insert(
        Handle::new(new_h),
        ObjectType::Unknown {
            type_name,
            handle: Handle::new(new_h),
            owner: dict_h,
            raw_dxf_codes,
            raw_dwg_data: Some(d),
            raw_dwg_handle_bits: hb as i64,
            raw_dwg_version,
        },
    );
    println!("style {h:X} copied as {new_h:X} (class {class})");
    new_h
}
