//! Creating an alignment in S24 from donor S25 (aecc_create).

use civil3drust::aecc_alignment_refs::alignment_refs;
use civil3drust::aecc_create::{create_alignment, CreateOptions};
use civil3drust::aecc_elements::decode_elements;
use civil3drust::alignment::Point2;
use civil3drust::{handle_refs, raw};

#[test]
fn create_in_s24_from_s25() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [Point2::new(0.0, -100.0), Point2::new(200.0, -100.0)];
    let c = create_alignment(
        &mut t.doc,
        &donor.doc,
        "civil3drust - 1",
        &pis,
        7,
        CreateOptions::default(),
    )
    .unwrap();
    assert_eq!(c.alignment, c.companion + 1);
    assert_eq!(c.label_group, c.alignment + 1);

    // Write and read back.
    let dir = std::env::temp_dir().join("civil3drust-create-test.dwg");
    acadrust::DwgWriter::write_to_file(&dir, &t.doc).unwrap();
    let re = raw::open(&dir).unwrap();
    let aligns = raw::records(&re.doc, "AECC_ALIGNMENT");
    assert_eq!(aligns.len(), 2);
    let a = aligns.iter().find(|r| r.handle == c.alignment).unwrap();
    let d = decode_elements(a).unwrap();
    assert_eq!(d.name.as_deref(), Some("civil3drust - 1"));
    assert!((d.length() - 200.0).abs() < 1e-9);
    let refs = alignment_refs(a).unwrap();
    assert_eq!(refs.companion.handle, c.companion);

    let comp = raw::records(&re.doc, "AECC_FEATURE_STYLE")
        .into_iter()
        .find(|r| r.handle == c.companion)
        .unwrap();
    let crefs = handle_refs::of_record(&comp);
    assert_eq!(crefs[0].handle, c.registry);
    assert_eq!(crefs[1].handle, c.alignment);
    let reg = raw::records(&re.doc, "AECC_FEATURE_STYLE")
        .into_iter()
        .find(|r| r.handle == c.registry)
        .unwrap();
    let owned: Vec<u64> = handle_refs::of_record(&reg)
        .iter()
        .filter(|r| r.code == 2)
        .map(|r| r.handle)
        .collect();
    assert_eq!(owned, [c.companion, 0xF9D]);
    for rec in [&comp, &reg] {
        assert!(raw::utf16_strings(&rec.data, 32)
            .iter()
            .any(|(_, s)| s == &c.id_string));
    }
    let label = raw::records(&re.doc, "AECC_ALIGNMENT_STAEQU_LABEL_GROUP")
        .into_iter()
        .find(|r| r.handle == c.label_group)
        .unwrap();
    assert!(handle_refs::of_record(&label)
        .iter()
        .any(|r| r.code == 4 && r.handle == c.alignment));
    let _ = std::fs::remove_file(dir);
}

#[test]
fn refuses_empty_name_and_wrong_pi_count() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
    assert!(create_alignment(
        &mut t.doc,
        &donor.doc,
        "",
        &pis,
        1,
        CreateOptions::default()
    )
    .is_err());
    let three = [pis[0], pis[1], Point2::new(1.0, 1.0)];
    assert!(create_alignment(
        &mut t.doc,
        &donor.doc,
        "civil3drust - 1",
        &three,
        1,
        CreateOptions::default()
    )
    .is_err());
}

#[test]
fn create_without_parcel_entry() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [Point2::new(0.0, -100.0), Point2::new(200.0, -100.0)];
    let opts = CreateOptions {
        label_group: false,
        parcel_entry: false,
    };
    let c = create_alignment(&mut t.doc, &donor.doc, "civil3drust - 1", &pis, 7, opts).unwrap();
    assert_eq!((c.companion, c.label_group), (0, 0));
    let dir = std::env::temp_dir().join("civil3drust-create-noparcel.dwg");
    acadrust::DwgWriter::write_to_file(&dir, &t.doc).unwrap();
    let re = raw::open(&dir).unwrap();
    let a = raw::records(&re.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == c.alignment)
        .unwrap();
    let d = decode_elements(&a).unwrap();
    assert!((d.length() - 200.0).abs() < 1e-9);
    let refs: Vec<(u8, u64)> = handle_refs::of_record(&a)
        .iter()
        .map(|r| (r.code, r.handle))
        .collect();
    assert_eq!(
        refs,
        [
            (5, 0x10),
            (4, 0),
            (5, 0),
            (5, 0),
            (4, 0),
            (5, 0xFA9),
            (5, 0)
        ]
    );
    // Registry unchanged from S24: still owns only F9D.
    let reg = raw::records(&re.doc, "AECC_FEATURE_STYLE")
        .into_iter()
        .find(|r| r.handle == 0xBC0)
        .unwrap();
    let owned: Vec<u64> = handle_refs::of_record(&reg)
        .iter()
        .filter(|r| r.code == 2)
        .map(|r| r.handle)
        .collect();
    assert_eq!(owned, [0xF9D]);
    assert_eq!(raw::records(&re.doc, "AECC_FEATURE_STYLE").len(), 2);
    let _ = std::fs::remove_file(dir);
}

#[test]
fn create_with_one_letter_name() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [Point2::new(0.0, -100.0), Point2::new(200.0, -100.0)];
    let c = create_alignment(
        &mut t.doc,
        &donor.doc,
        "B",
        &pis,
        3,
        CreateOptions::default(),
    )
    .unwrap();
    let dir = std::env::temp_dir().join("civil3drust-create-b.dwg");
    acadrust::DwgWriter::write_to_file(&dir, &t.doc).unwrap();
    let re = raw::open(&dir).unwrap();
    let a = raw::records(&re.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == c.alignment)
        .unwrap();
    let d = decode_elements(&a).unwrap();
    assert_eq!(d.name.as_deref(), Some("B"));
    assert!((d.length() - 200.0).abs() < 1e-9);
    assert_eq!(alignment_refs(&a).unwrap().style.handle, 0xFA9);
    let _ = std::fs::remove_file(dir);
}

#[test]
fn rename_s24_alignment_any_length() {
    use civil3drust::string_stream;
    let s24 = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let rec = raw::records(&s24.doc, "AECC_ALIGNMENT").remove(0);
    for name in ["A", "Centerline of 19th Avenue NE - proposed realignment"] {
        let (d, hb) =
            string_stream::set_name(&rec.data, rec.handle_bits, "Alignment - (1)", name).unwrap();
        let r2 = raw::RawRecord {
            data: d,
            handle_bits: hb,
            ..rec.clone()
        };
        assert_eq!(raw::object_name(&r2.data).as_deref(), Some(name));
        let dec = decode_elements(&r2).unwrap();
        assert!((dec.length() - 411.378981575051).abs() < 1e-6);
        let refs: Vec<u64> = handle_refs::of_record(&r2)
            .iter()
            .map(|r| r.handle)
            .collect();
        let refs0: Vec<u64> = handle_refs::of_record(&rec)
            .iter()
            .map(|r| r.handle)
            .collect();
        assert_eq!(refs, refs0);
        assert_eq!(
            string_stream::parse(&r2.data, hb).unwrap().strings.len(),
            15
        );
    }
}
