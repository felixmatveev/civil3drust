//! Design criteria (O3, S79, S80).

use civil3drust::aecc_build::alignment_type;
use civil3drust::aecc_criteria::{criteria, Criteria};
use civil3drust::raw;

fn recs(f: &str) -> Vec<raw::RawRecord> {
    raw::records(&raw::open(f).unwrap().doc, "AECC_ALIGNMENT")
}

#[test]
fn s79_flags_names_types() {
    let r = recs("samples/S79_O3_design_criteria.dwg");
    let on = r.iter().find(|x| x.handle == 0xFB3).unwrap();
    let off = r.iter().find(|x| x.handle == 0xFB6).unwrap();
    let all = Criteria {
        criteria_based: true,
        criteria_file: true,
        check_set: true,
        check_tangency: true,
    };
    assert_eq!(criteria(on).unwrap(), all);
    assert_eq!(criteria(off).unwrap(), Criteria::default());
    assert_eq!(raw::object_name(&on.data).as_deref(), Some("DESIGN ON"));
    assert_eq!(raw::object_name(&off.data).as_deref(), Some("DESING OFF"));
    // Two design speeds: the type follows the whole table.
    assert_eq!(alignment_type(off).unwrap(), 1);
}

#[test]
fn s81_c1_cleared_by_civil3d() {
    // C1 had only the flags; Civil 3D kept criteria-based design and
    // cleared the rest (owner's screenshot of the dialog matches).
    let r = recs("samples/S81_S80_civ3d_resaved.dwg");
    let c1 = r
        .iter()
        .find(|x| raw::object_name(&x.data).as_deref() == Some("C1 flags"))
        .unwrap();
    assert_eq!(
        criteria(c1).unwrap(),
        Criteria {
            criteria_based: true,
            criteria_file: false,
            check_set: false,
            check_tangency: false,
        }
    );
    // Every other sample alignment has the defaults.
    for f in [
        "samples/S24_ZERO_CIVIL.dwg",
        "samples/S58_O1_centerline.dwg",
    ] {
        for x in recs(f) {
            assert_eq!(criteria(&x).unwrap(), Criteria::default());
        }
    }
}
