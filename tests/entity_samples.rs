//! Common entity data of alignment records (section 2 start).

use civil3drust::aecc_entity::{
    entity_common, COLOR_BYBLOCK, COLOR_BYLAYER, LINEWEIGHT_BYBLOCK, LINEWEIGHT_BYLAYER,
};
use civil3drust::raw;

#[test]
fn every_alignment_starts_section_2_with_common_entity_data() {
    let mut n = 0;
    let mut paths: Vec<_> = std::fs::read_dir("samples")
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "dwg"))
        .collect();
    paths.sort();
    for p in paths {
        let Ok(o) = raw::open(&p) else { continue };
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        for r in raw::records(&o.doc, "AECC_ALIGNMENT") {
            let Some(c) = entity_common(&r.data) else {
                // Records without graphics written by experiments are fine
                // too; anything else must parse.
                panic!("{name} {:X}: common data not parsed", r.handle)
            };
            // The class's data starts with BL 80.
            assert_eq!(
                raw::read_bl(&r.data, c.end_bit).map(|x| x.0),
                Some(80),
                "{name} {:X}",
                r.handle
            );
            assert_eq!(c.linetype_scale, 1.0);
            assert!(!c.invisible);
            let byblock = name.starts_with("S18")
                || name.starts_with("S19")
                || name.starts_with("S20")
                || name.starts_with("S21")
                || name.starts_with("S22")
                || name.starts_with("S23");
            if byblock && r.handle == 0x8C5C {
                assert_eq!((c.color, c.lineweight), (COLOR_BYBLOCK, LINEWEIGHT_BYBLOCK));
                assert_eq!(c.linetype_flags, 1);
            } else {
                assert_eq!(
                    (c.color, c.lineweight),
                    (COLOR_BYLAYER, LINEWEIGHT_BYLAYER),
                    "{name}"
                );
                assert_eq!(c.linetype_flags, 0);
            }
            // S1's alignment has an extension dictionary (extra reference).
            assert_eq!(c.has_xdictionary, name.starts_with("S1_"), "{name}");
            n += 1;
        }
    }
    assert!(n > 200, "{n}");
}
