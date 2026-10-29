//! Civil 3D 2027 drawings (S123 feet, S124 meters): the one-byte object
//! type form (class numbers 500–751) and speed entries with a zero GUID.

use civil3drust::aecc_create::{create_alignment, set_object_type, CreateOptions};
use civil3drust::aecc_design_speed::{design_speeds, entry_guid_bits, ZERO_GUID};
use civil3drust::aecc_elements::decode_elements;
use civil3drust::alignment::Point2;
use civil3drust::{handle_refs, raw};

const S123: &str = "samples/S123_c3d2027_feet_tangent.dwg";
const S124: &str = "samples/S124_c3d2027_meters_tangent.dwg";

/// S123's companion class (736) uses the `01` + byte form, so its own
/// handle starts at bit 10, not 18.
#[test]
fn own_handle_after_one_byte_object_type() {
    let o = raw::open(S123).unwrap();
    let c = &raw::records(&o.doc, "AECC_FEATURE_STYLE")[0];
    assert_eq!(handle_refs::object_type_bits(&c.data), Some(10));
    assert_eq!(handle_refs::own_handle(&c.data), Some(c.handle));
    let a = &raw::records(&o.doc, "AECC_ALIGNMENT")[0];
    assert_eq!(handle_refs::object_type_bits(&a.data), Some(18));
    assert_eq!(handle_refs::own_handle(&a.data), Some(a.handle));
}

/// The object type is written in AutoCAD's form for the number, switching
/// form (and record length) when needed, and reads back.
#[test]
fn object_type_forms() {
    let o = raw::open(S123).unwrap();
    let c = &raw::records(&o.doc, "AECC_FEATURE_STYLE")[0];
    let bits = raw::bit_len(&c.data);
    // Short form → long form (+8 bits) and back.
    let (long, hb) = set_object_type(&c.data, c.handle_bits, 1056).unwrap();
    assert_eq!(hb, c.handle_bits);
    assert_eq!(raw::bit_len(&long), bits + 8);
    assert_eq!(handle_refs::object_type_bits(&long), Some(18));
    assert_eq!(handle_refs::own_handle(&long), Some(c.handle));
    let (short, hb) = set_object_type(&long, hb, 736).unwrap();
    assert_eq!(hb, c.handle_bits);
    assert_eq!(short, c.data);
}

/// S123/S124 store a speed (0:60, 0:100) with no interval block and a
/// zero GUID in the table.
#[test]
fn speed_entry_with_zero_guid() {
    for f in [S123, S124] {
        let o = raw::open(f).unwrap();
        let a = &raw::records(&o.doc, "AECC_ALIGNMENT")[0];
        let s = design_speeds(a).unwrap();
        assert_eq!(s.len(), 1, "{f}");
        assert_eq!(s[0].id, ZERO_GUID, "{f}");
        assert_eq!(entry_guid_bits(a).unwrap().len(), 1);
    }
}

/// A 2027 drawing as its own donor: the companion keeps its short object
/// type, the speed gets a fresh GUID shared by the interval and the table,
/// and nothing else in the record is overwritten.
#[test]
fn create_with_2027_donor() {
    for f in [S123, S124] {
        let donor = raw::open(f).unwrap();
        let mut t = raw::open(f).unwrap();
        let pis = [
            Point2::new(0.0, -200.0),
            Point2::new(300.0, -200.0),
            Point2::new(300.0, -500.0),
            Point2::new(600.0, -500.0),
        ];
        let c = create_alignment(
            &mut t.doc,
            &donor.doc,
            "NEW",
            &pis,
            &[None, Some(50.0)],
            7,
            CreateOptions::default(),
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "civil3drust-2027-{}.dwg",
            if f == S123 { "ft" } else { "m" }
        ));
        acadrust::DwgWriter::write_to_file(&dir, &t.doc).unwrap();
        let re = raw::open(&dir).unwrap();
        let comp = raw::records(&re.doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == c.companion)
            .unwrap();
        assert_eq!(handle_refs::own_handle(&comp.data), Some(c.companion));
        let a = raw::records(&re.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == c.alignment)
            .unwrap();
        let d = decode_elements(&a).unwrap();
        assert_eq!(d.elements.len(), 4, "{f}");
        let s = design_speeds(&a).unwrap();
        assert_eq!(s.len(), 1, "{f}");
        assert_ne!(s[0].id, ZERO_GUID, "{f}");
        let end = d.records_end_bit().unwrap();
        let copy1: Vec<u8> = (0..16)
            .map(|i| raw::get_byte(&a.data, end + 16 + 8 * i))
            .collect();
        assert_eq!(copy1, s[0].id, "{f}");
        assert_eq!(civil3drust::aecc_build::alignment_type(&a).unwrap(), 1);
        assert_eq!(civil3drust::aecc_build::pi_display(&a).unwrap(), 1);
        let _ = std::fs::remove_file(dir);
    }
}
