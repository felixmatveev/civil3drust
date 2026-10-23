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
    assert_eq!(owned.len(), 2);
    assert!(owned.contains(&c.companion) && owned.contains(&0xF9D));
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
    assert_ne!(mine, theirs);
    assert_eq!(mine[7] >> 4, 4, "version-4 GUID");
    assert_eq!(mine[8] >> 6, 0b10, "GUID variant");
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
        ..Default::default()
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

/// Five PIs with two free curves: civil3drust's record equals Civil 3D's
/// (S49) in graphics, elements and strings, except the identity GUID, the
/// 62-bit section-2 field and the last bit of the PC station (Civil 3D
/// rounds 88.1966011250105, we compute 88.19660112501052).
#[test]
fn curves_match_civil3d_s49() {
    use civil3drust::{aecc_elements::Element, proxy_graphics as pg, string_stream};
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [
        Point2::new(0.0, -200.0),
        Point2::new(100.0, -200.0),
        Point2::new(200.0, -150.0),
        Point2::new(300.0, -150.0),
        Point2::new(400.0, -250.0),
    ];
    let c = create_alignment(
        &mut t.doc,
        &donor.doc,
        "C",
        &pis,
        &[Some(50.0), None, Some(30.0)],
        5,
        CreateOptions::default(),
    )
    .unwrap();
    let ours = raw::records(&t.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == c.alignment)
        .unwrap();
    let s49 = raw::open("samples/S49_S48_C_with_curves.dwg").unwrap();
    let theirs = raw::records(&s49.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == 0x102B)
        .unwrap();
    let parts = |r: &raw::RawRecord| {
        let g = pg::find(&r.data, 4096).unwrap();
        let d = decode_elements(r).unwrap();
        let b: Vec<(usize, usize)> = d
            .elements
            .iter()
            .map(|e| match &e.record {
                Element::Tangent(t) => (t.header_bit, t.coords_bit + 256),
                Element::Arc(a) => (a.header_bit, a.end_bit),
            })
            .collect();
        let first = b.iter().map(|x| x.0).min().unwrap() - 21;
        let end = b.iter().map(|x| x.1).max().unwrap();
        let ss = string_stream::parse(&r.data, r.handle_bits).unwrap();
        let bits = |a: usize, z: usize| {
            (a..z)
                .map(|i| raw::get_bit(&r.data, i))
                .collect::<Vec<u8>>()
        };
        (
            bits(g.start_bit, g.end_bit()),
            bits(first, end),
            bits(ss.start, ss.size_field),
            ss.strings.len(),
        )
    };
    let (g1, e1, s1, n1) = parts(&ours);
    let (g2, e2, s2, n2) = parts(&theirs);
    assert_eq!(e1, e2, "element records");
    assert_eq!((s1, n1), (s2, n2), "string stream");
    assert_eq!(n1, 18);
    let gd: Vec<usize> = (0..g1.len()).filter(|&i| g1[i] != g2[i]).collect();
    assert_eq!(g1.len(), g2.len());
    assert!(gd.len() <= 2, "graphics differ beyond PC rounding: {gd:?}");
}

/// The donor no longer has to be the target plus one alignment: S25's
/// template into S44 (three registry entries) and S24 twice in a row.
#[test]
fn registry_entry_added_for_any_target() {
    use civil3drust::aecc_create::find_registry;
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S44_S25_plus_two_tangent_alignment.dwg").unwrap();
    let pis = [Point2::new(0.0, -300.0), Point2::new(150.0, -300.0)];
    let c = create_alignment(
        &mut t.doc,
        &donor.doc,
        "D",
        &pis,
        &[],
        11,
        CreateOptions::default(),
    )
    .unwrap();
    let (_, reg) = find_registry(&t.doc).unwrap();
    assert_eq!(reg.entries.len(), 4);
    let ids: Vec<&str> = reg.entries.iter().map(|e| e.id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
    let e = reg
        .entries
        .iter()
        .find(|e| e.companion == c.companion)
        .unwrap();
    assert_eq!(e.key, 0x8000_29F6); // largest key in S44 + 1
    assert_eq!(e.id, c.id_string);

    // Two alignments into S24, one after the other.
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let c1 = create_alignment(
        &mut t.doc,
        &donor.doc,
        "E1",
        &pis,
        &[],
        1,
        CreateOptions::default(),
    )
    .unwrap();
    let c2 = create_alignment(
        &mut t.doc,
        &donor.doc,
        "E2",
        &pis,
        &[],
        2,
        CreateOptions::default(),
    )
    .unwrap();
    let dir = std::env::temp_dir().join("civil3drust-two-created.dwg");
    acadrust::DwgWriter::write_to_file(&dir, &t.doc).unwrap();
    let re = raw::open(&dir).unwrap();
    let (_, reg) = find_registry(&re.doc).unwrap();
    assert_eq!(reg.entries.len(), 3);
    let keys: Vec<u32> = reg.entries.iter().map(|e| e.key).collect();
    assert!(keys.contains(&0x8000_0C77) && keys.contains(&0x8000_0C78));
    for c in [&c1, &c2] {
        let comp = raw::records(&re.doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == c.companion)
            .unwrap();
        let refs = handle_refs::of_record(&comp);
        assert_eq!(refs[0].handle, reg.handle);
        assert_eq!(refs[1].handle, c.alignment);
        assert_eq!(
            civil3drust::aecc_registry::companion_key(&comp).unwrap().1,
            c.registry_key
        );
    }
    let names: Vec<String> = raw::records(&re.doc, "AECC_ALIGNMENT")
        .iter()
        .filter_map(|r| raw::object_name(&r.data))
        .collect();
    assert!(names.contains(&"E1".to_string()) && names.contains(&"E2".to_string()));
    let _ = std::fs::remove_file(dir);
}

/// Adding an entry to a registry reproduces Civil 3D's registry exactly
/// (apart from padding bits): S24 → S25, S25 → S44, S44 → S48; removing it
/// gives back the original.
#[test]
fn registry_entries_match_civil3d() {
    use civil3drust::aecc_create::find_registry;
    let files = [
        "samples/S24_ZERO_CIVIL.dwg",
        "samples/S25_ZERO_CIVIL_second_alignment.dwg",
        "samples/S44_S25_plus_two_tangent_alignment.dwg",
        "samples/S48_S25_plus_C_no_curves.dwg",
    ];
    let regs: Vec<_> = files
        .iter()
        .map(|f| find_registry(&raw::open(f).unwrap().doc).unwrap())
        .collect();
    let content = |d: &[u8], hb: usize| -> Vec<u8> {
        // data stream + handle references, without trailing padding
        let total = d.len() * 8;
        let refs = handle_refs::parse(d, total - hb, total, 0);
        let end = refs.last().map(|r| r.bit + r.bits()).unwrap();
        (0..end).map(|i| raw::get_bit(d, i)).collect()
    };
    for w in regs.windows(2) {
        let ((ra, ga), (rb, gb)) = (&w[0], &w[1]);
        let new = gb
            .entries
            .iter()
            .find(|e| !ga.entries.iter().any(|x| x.companion == e.companion))
            .unwrap()
            .clone();
        let (d, hb) = ga.with_entry(ra, new.clone()).unwrap();
        assert_eq!(content(&d, hb), content(&rb.data, rb.handle_bits));
        let (d2, hb2) = gb.without_entry(rb, new.companion).unwrap();
        assert_eq!(content(&d2, hb2), content(&ra.data, ra.handle_bits));
    }
}

/// No stray padding byte after the handle stream of created records (S56
/// had one); the alignment references parse as the usual seven.
#[test]
fn created_handle_stream_has_no_padding_ref() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    for name in ["X0 control", "X", "A longer name to shift padding"] {
        let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
        let pis = [
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(200.0, 40.0),
        ];
        let c = create_alignment(
            &mut t.doc,
            &donor.doc,
            name,
            &pis,
            &[Some(30.0)],
            1,
            CreateOptions::default(),
        )
        .unwrap();
        let a = raw::records(&t.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == c.alignment)
            .unwrap();
        let total = a.data.len() * 8;
        let raw_refs = handle_refs::parse(&a.data, total - a.handle_bits, total, a.handle);
        assert_eq!(raw_refs.len(), 7, "{name}");
        assert!(
            a.handle_bits
                - (raw_refs.last().unwrap().bit + raw_refs.last().unwrap().bits()
                    - (total - a.handle_bits))
                < 8
        );
        assert_eq!(alignment_refs(&a).unwrap().companion.handle, c.companion);
    }
}

#[test]
fn create_without_graphics() {
    let donor = raw::open("samples/S25_ZERO_CIVIL_second_alignment.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [
        Point2::new(0.0, 0.0),
        Point2::new(100.0, 0.0),
        Point2::new(200.0, 40.0),
    ];
    let opts = CreateOptions {
        graphics: false,
        ..Default::default()
    };
    let c = create_alignment(&mut t.doc, &donor.doc, "NG", &pis, &[Some(30.0)], 1, opts).unwrap();
    let dir = std::env::temp_dir().join("civil3drust-no-graphics.dwg");
    acadrust::DwgWriter::write_to_file(&dir, &t.doc).unwrap();
    let re = raw::open(&dir).unwrap();
    let a = raw::records(&re.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == c.alignment)
        .unwrap();
    assert!(civil3drust::proxy_graphics::find(&a.data, 4096).is_none());
    assert_eq!(raw::get_bit(&a.data, 44), 0);
    let d = decode_elements(&a).unwrap();
    assert_eq!(d.name.as_deref(), Some("NG"));
    assert_eq!(alignment_refs(&a).unwrap().companion.handle, c.companion);
    let _ = std::fs::remove_file(dir);
}

/// O1: S58 (Centerline) and S59 (Miscellaneous) differ in the type BL after
/// GUID copy 2; every other Civil 3D record in the samples is a centerline.
#[test]
fn alignment_type_field() {
    use civil3drust::aecc_build::{alignment_type, TYPE_CENTERLINE, TYPE_MISCELLANEOUS};
    let ty = |f: &str| {
        let o = raw::open(f).unwrap();
        let r = raw::records(&o.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == 0x1089)
            .unwrap();
        alignment_type(&r).unwrap()
    };
    assert_eq!(ty("samples/S58_O1_centerline.dwg"), TYPE_CENTERLINE);
    assert_eq!(ty("samples/S59_O1_miscellaneous.dwg"), TYPE_MISCELLANEOUS);
    // Records with the element-ID list before the type (S51 FB3).
    let o = raw::open("samples/S51_S50_civ3d_resaved.dwg").unwrap();
    for r in raw::records(&o.doc, "AECC_ALIGNMENT") {
        assert_eq!(
            alignment_type(&r).unwrap(),
            TYPE_CENTERLINE,
            "{:X}",
            r.handle
        );
    }
}

#[test]
fn create_with_alignment_type() {
    use civil3drust::aecc_build::{alignment_type, TYPE_MISCELLANEOUS};
    let donor = raw::open("samples/S58_O1_centerline.dwg").unwrap();
    let mut t = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let pis = [Point2::new(0.0, -200.0), Point2::new(300.0, -200.0)];
    let opts = CreateOptions {
        alignment_type: Some(TYPE_MISCELLANEOUS),
        ..Default::default()
    };
    let c = create_alignment(&mut t.doc, &donor.doc, "M", &pis, &[], 7, opts).unwrap();
    let r = raw::records(&t.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == c.alignment)
        .unwrap();
    assert_eq!(alignment_type(&r).unwrap(), TYPE_MISCELLANEOUS);
}

/// S65 is a Civil 3D drawing that never had an alignment: an empty registry
/// and no `AECC_ALIGNMENT` class. Creating one adds the class (positional
/// number) and the first registry entry.
#[test]
fn create_in_drawing_without_alignments() {
    use civil3drust::aecc_create::find_registry;
    let donor = raw::open("samples/S58_O1_centerline.dwg").unwrap();
    let mut t = raw::open("samples/S65_S64_civ3d_resaved.dwg").unwrap();
    assert!(t.doc.classes.get_by_name("AECC_ALIGNMENT").is_none());
    let (_, reg) = find_registry(&t.doc).unwrap();
    assert!(reg.entries.is_empty());
    let pis = [Point2::new(0.0, 0.0), Point2::new(500.0, 0.0)];
    let opts = CreateOptions {
        label_group: false,
        style: Some(0x48C),
        ..Default::default()
    };
    let c = create_alignment(&mut t.doc, &donor.doc, "Minimal", &pis, &[], 3, opts).unwrap();
    let n = t.doc.classes.len() as i16 - 1;
    assert_eq!(
        t.doc
            .classes
            .get_by_name("AECC_ALIGNMENT")
            .unwrap()
            .class_number,
        500 + n
    );
    let path = std::env::temp_dir().join("civil3drust-s65-create.dwg");
    acadrust::DwgWriter::write_to_file(&path, &t.doc).unwrap();
    let re = raw::open(&path).unwrap();
    let a = raw::records(&re.doc, "AECC_ALIGNMENT");
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].handle, c.alignment);
    assert_eq!(decode_elements(&a[0]).unwrap().length(), 500.0);
    let (_, reg) = find_registry(&re.doc).unwrap();
    assert_eq!(reg.entries.len(), 1);
    assert_eq!(reg.entries[0].companion, c.companion);
}
