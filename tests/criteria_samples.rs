//! Design criteria (O3, S79, S80).

use civil3drust::aecc_build::alignment_type;
use civil3drust::aecc_criteria::criteria_flags;
use civil3drust::raw;

fn recs(f: &str) -> Vec<raw::RawRecord> {
    raw::records(&raw::open(f).unwrap().doc, "AECC_ALIGNMENT")
}

#[test]
fn s79_flags_names_types() {
    let r = recs("samples/S79_O3_design_criteria.dwg");
    let on = r.iter().find(|x| x.handle == 0xFB3).unwrap();
    let off = r.iter().find(|x| x.handle == 0xFB6).unwrap();
    assert_eq!(criteria_flags(on).unwrap(), [true; 3]);
    assert_eq!(criteria_flags(off).unwrap(), [false; 3]);
    assert_eq!(raw::object_name(&on.data).as_deref(), Some("DESIGN ON"));
    assert_eq!(raw::object_name(&off.data).as_deref(), Some("DESING OFF"));
    // Two design speeds: the type follows the whole table.
    assert_eq!(alignment_type(off).unwrap(), 1);
}

#[test]
fn s80_generated() {
    let names: Vec<(String, [bool; 3])> = recs("samples/S80_S24_criteria_experiments.dwg")
        .iter()
        .map(|r| {
            (
                raw::object_name(&r.data).unwrap(),
                criteria_flags(r).unwrap(),
            )
        })
        .collect();
    assert!(names.contains(&("C0 control".into(), [false; 3])));
    assert!(names.contains(&("C1 flags".into(), [true; 3])));
    assert!(names.contains(&("C3 full".into(), [true; 3])));
}
