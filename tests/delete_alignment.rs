//! Deleting an alignment (S228 → S229, owner deleted `Alignment - (1)` in
//! Civil 3D): the alignment and its label groups go, the companion stays,
//! emptied as Civil 3D empties it.

use civil3drust::aecc_delete::delete_alignment;
use civil3drust::raw;

const F9E: u64 = 0xF9E;

#[test]
fn delete_like_civil3d() {
    let mut a = raw::open("samples/S228_S24_spiral_type_template.dwg").unwrap();
    let b = raw::open("samples/S229_S228_alignment_deleted.dwg").unwrap();
    let d = delete_alignment(&mut a.doc, F9E).unwrap();
    let mut groups = d.label_groups.clone();
    groups.sort_unstable();
    assert_eq!(groups, vec![0xFA3, 0xFA4]);
    assert_eq!(d.companion, 0xF9D);
    // Same alignments left as in Civil 3D's file.
    let handles = |doc: &acadrust::document::CadDocument| -> Vec<u64> {
        raw::records(doc, "AECC_ALIGNMENT")
            .iter()
            .map(|r| r.handle)
            .collect()
    };
    assert_eq!(handles(&a.doc), handles(&b.doc));
    // The companion: Civil 3D's emptied form, apart from the object
    // header bits a save rewrites (68–84) and the handle stream's padding
    // after its one reference (24 bits).
    let comp = |doc: &acadrust::document::CadDocument| {
        raw::records(doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == 0xF9D)
            .unwrap()
    };
    let (x, y) = (comp(&a.doc), comp(&b.doc));
    assert_eq!(x.data.len(), y.data.len());
    assert_eq!(x.handle_bits, y.handle_bits);
    let differ: Vec<usize> = (0..raw::bit_len(&x.data))
        .filter(|&i| raw::get_bit(&x.data, i) != raw::get_bit(&y.data, i))
        .collect();
    let pad = raw::bit_len(&x.data) - x.handle_bits + 24;
    assert!(
        differ.iter().all(|&i| (68..84).contains(&i) || i >= pad),
        "differing bits {differ:?}"
    );
    // Written and read back.
    let path = std::env::temp_dir().join("civil3drust-delete.dwg");
    acadrust::DwgWriter::write_to_file(&path, &a.doc).unwrap();
    let r = raw::open(&path).unwrap();
    assert_eq!(handles(&r.doc), handles(&b.doc));
    for (class, h) in [
        ("AECC_ALIGNMENT_STAEQU_LABEL_GROUP", 0xFA3),
        ("AECC_ALIGNMENT_DESIGNSPEED_LABEL_GROUP", 0xFA4),
    ] {
        assert!(raw::records(&r.doc, class).iter().all(|x| x.handle != h));
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn refuses_unknown_alignment() {
    let mut a = raw::open("samples/S228_S24_spiral_type_template.dwg").unwrap();
    assert!(delete_alignment(&mut a.doc, 0x1).is_err());
}

#[test]
fn civil3d_accepts_the_deletion() {
    // S230 (written by civil3drust) opened clean in Civil 3D; its re-save
    // S231 reads back to the same alignments.
    use civil3drust::aecc_elements::decode_elements;
    let a = raw::open("samples/S230_S228_alignment_deleted_by_civil3drust.dwg").unwrap();
    let b = raw::open("samples/S231_S230_civ3d_resaved.dwg").unwrap();
    let rb = raw::records(&b.doc, "AECC_ALIGNMENT");
    let ra = raw::records(&a.doc, "AECC_ALIGNMENT");
    assert_eq!(ra.len(), 7);
    assert_eq!(ra.len(), rb.len());
    for r in ra {
        let q = rb.iter().find(|q| q.handle == r.handle).unwrap();
        assert_eq!(raw::object_name(&r.data), raw::object_name(&q.data));
        let (x, y) = (decode_elements(&r).unwrap(), decode_elements(q).unwrap());
        assert!((x.length() - y.length()).abs() < 1e-9);
    }
}
