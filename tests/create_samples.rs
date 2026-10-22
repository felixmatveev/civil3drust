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
        &[],
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
fn refuses_empty_name_and_bad_geometry() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
    let d = CreateOptions::default();
    assert!(create_alignment(&mut t.doc, &donor.doc, "", &pis, &[], 1, d).is_err());
    let one = [Point2::new(0.0, 0.0)];
    assert!(create_alignment(&mut t.doc, &donor.doc, "X", &one, &[], 1, d).is_err());
    let three = [pis[0], Point2::new(10.0, 0.0), Point2::new(10.0, 10.0)];
    // R = 1000 needs a 1000-ft tangent on a 10-ft leg.
    assert!(create_alignment(&mut t.doc, &donor.doc, "X", &three, &[Some(1000.0)], 1, d).is_err());
}

#[test]
fn create_multi_pi_with_curves() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [
        Point2::new(0.0, -200.0),
        Point2::new(100.0, -200.0),
        Point2::new(200.0, -150.0),
        Point2::new(300.0, -150.0),
        Point2::new(400.0, -250.0),
    ];
    let radii = [Some(50.0), None, Some(30.0)];
    let c = create_alignment(
        &mut t.doc,
        &donor.doc,
        "C",
        &pis,
        &radii,
        9,
        CreateOptions::default(),
    )
    .unwrap();
    let dir = std::env::temp_dir().join("civil3drust-create-multi.dwg");
    acadrust::DwgWriter::write_to_file(&dir, &t.doc).unwrap();
    let re = raw::open(&dir).unwrap();
    let recs = raw::records(&re.doc, "AECC_ALIGNMENT");
    let a = recs.iter().find(|r| r.handle == c.alignment).unwrap();
    let d = decode_elements(a).unwrap();
    assert_eq!(d.name.as_deref(), Some("C"));
    let kinds: String = d
        .elements
        .iter()
        .map(|e| match e.segment {
            civil3drust::aecc_elements::Segment::Line { .. } => 'T',
            _ => 'A',
        })
        .collect();
    assert_eq!(kinds, "TATTAT");
    let s = d.elements[0].segment.start();
    let e = d.elements.last().unwrap().segment.end();
    assert_eq!((s.x, s.y, e.x, e.y), (0.0, -200.0, 400.0, -250.0));
    // Identity differs from the donor's alignment.
    let id_of = |r: &raw::RawRecord| -> Vec<u8> {
        let dd = decode_elements(r).unwrap();
        let end = dd
            .elements
            .iter()
            .map(|e| match &e.record {
                civil3drust::aecc_elements::Element::Tangent(t) => t.coords_bit + 256,
                civil3drust::aecc_elements::Element::Arc(a) => a.end_bit,
            })
            .max()
            .unwrap();
        (0..16)
            .map(|i| raw::get_byte(&r.data, end + 16 + 8 * i))
            .collect()
    };
    let dn = raw::records(&donor.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xFB8)
        .unwrap();
    let (mine, theirs) = (id_of(a), id_of(&dn));
    assert_ne!(mine[..14], theirs[..14]);
    assert_eq!(mine[14..], theirs[14..]);
    let _ = std::fs::remove_file(dir);
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
    let c = create_alignment(
        &mut t.doc,
        &donor.doc,
        "civil3drust - 1",
        &pis,
        &[],
        7,
        opts,
    )
    .unwrap();
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
        &[],
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

/// The two-tangent alignment civil3drust writes equals the one Civil 3D
/// wrote for the same polyline (S44 FF3) from the element count to the
/// string stream, except for the two copies of the identity value.
#[test]
fn two_tangents_match_civil3d_s44() {
    use civil3drust::{aecc_elements::Element, proxy_graphics as pg, string_stream};
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [
        Point2::new(0.0, -200.0),
        Point2::new(100.0, -200.0),
        Point2::new(200.0, -150.0),
    ];
    let c = create_alignment(
        &mut t.doc,
        &donor.doc,
        "T",
        &pis,
        &[],
        5,
        CreateOptions::default(),
    )
    .unwrap();
    let ours = raw::records(&t.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == c.alignment)
        .unwrap();
    let s44 = raw::open("samples/S44_S25_plus_two_tangent_alignment.dwg").unwrap();
    let theirs = raw::records(&s44.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0xFF3)
        .unwrap();
    let span = |r: &raw::RawRecord| -> (Vec<u8>, Vec<u8>, usize) {
        let g = pg::find(&r.data, 4096).unwrap();
        let d = decode_elements(r).unwrap();
        let bounds: Vec<(usize, usize)> = d
            .elements
            .iter()
            .map(|e| match &e.record {
                Element::Tangent(t) => (t.header_bit, t.coords_bit + 256),
                Element::Arc(a) => (a.header_bit, a.end_bit),
            })
            .collect();
        let first = bounds.iter().map(|b| b.0).min().unwrap() - 21;
        let end = bounds.iter().map(|b| b.1).max().unwrap();
        let ss = string_stream::parse(&r.data, r.handle_bits).unwrap();
        let bits = |a: usize, b: usize| {
            (a..b)
                .map(|i| raw::get_bit(&r.data, i))
                .collect::<Vec<u8>>()
        };
        (
            bits(g.start_bit, g.end_bit()),
            bits(first, ss.start),
            end - first,
        )
    };
    let (g1, mut e1, end1) = span(&ours);
    let (g2, mut e2, end2) = span(&theirs);
    assert_eq!(g1, g2);
    assert_eq!(end1, end2);
    assert_eq!(e1.len(), e2.len());
    // Mask the identity copies (after the elements, and 717 bits later).
    for v in [&mut e1, &mut e2] {
        for k in [end1 + 16, end1 + 733] {
            v[k..k + 128].iter_mut().for_each(|b| *b = 0);
        }
    }
    assert_eq!(e1, e2);
}
