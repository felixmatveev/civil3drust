//! acadrust 0.6.7 reads every `AEC*` class (including all `AECC_*` Civil 3D
//! classes) into its Architecture types and re-encodes them on save. These
//! tests check that `raw::open` restores them to raw records (D-033), so a
//! future acadrust change that hides the records again fails loudly.

use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
use civil3drust::raw;

const SAMPLE: &str = "samples/S24_ZERO_CIVIL.dwg";

#[test]
fn open_leaves_no_architecture_records() {
    let doc = raw::open(SAMPLE).unwrap().doc;
    assert!(!doc
        .objects
        .values()
        .any(|o| matches!(o, ObjectType::Arch(_))));
    assert!(!doc.entities().any(|e| matches!(e, EntityType::Extended(x)
            if matches!(x.data, acadrust::entities::ExtendedEntityData::Arch(_)))));
}

#[test]
fn aecc_records_are_found() {
    let doc = raw::open(SAMPLE).unwrap().doc;
    assert!(!raw::records(&doc, "AECC_ALIGNMENT").is_empty());
    assert!(!raw::records(&doc, "AECC_ALIGNMENT_STYLE").is_empty());
}

#[test]
fn restore_keeps_the_bytes() {
    // Read without restoring, take the bytes from the Architecture types,
    // restore, and compare.
    let mut doc = acadrust::DwgReader::from_file(SAMPLE)
        .unwrap()
        .read()
        .unwrap();
    let mut before: Vec<(u64, Vec<u8>, i64)> = Vec::new();
    for (h, o) in &doc.objects {
        if let ObjectType::Arch(a) = o {
            if let Some(d) = &a.raw_dwg_data {
                before.push((h.value(), d.clone(), a.raw_dwg_handle_bits));
            }
        }
    }
    let n = raw::restore_raw_records(&mut doc);
    assert!(n >= before.len() && !before.is_empty());
    for (h, d, hb) in before {
        match doc.objects.get(&acadrust::types::Handle::new(h)) {
            Some(ObjectType::Unknown {
                raw_dwg_data: Some(r),
                raw_dwg_handle_bits,
                ..
            }) => {
                assert_eq!(r, &d, "bytes changed for {h:X}");
                assert_eq!(*raw_dwg_handle_bits, hb);
            }
            _ => panic!("{h:X} not restored"),
        }
    }
}
