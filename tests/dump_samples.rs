//! The RON dump (D-065) of sample drawings reads back to the same value
//! and shows the decoded alignment.

use civil3drust::dump::{self, AlignmentElement};
use civil3drust::raw;

#[test]
fn s24_dump_reads_back() {
    let opened = raw::open("samples/S24_ZERO_CIVIL.dwg").unwrap();
    let d = dump::dump(&opened, "S24_ZERO_CIVIL.dwg", true);
    assert_eq!(d.version, "AC1032");
    assert_eq!(d.alignments.len(), 1);
    let a = &d.alignments[0];
    assert_eq!(a.name.as_deref(), Some("Alignment - (1)"));
    assert_eq!(a.style.as_deref(), Some("TEST STYLE"));
    assert_eq!(a.elements.len(), 4);
    assert!(matches!(
        a.elements[1],
        AlignmentElement::Arc { radius, ccw: true, .. } if radius == 50.0
    ));
    assert!(d
        .records
        .iter()
        .any(|r| r.class == "AECC_ALIGNMENT" && r.h == a.h));

    let text = dump::to_ron(&d);
    assert_eq!(dump::from_ron(&text).unwrap(), d);
}

#[test]
fn every_sample_dumps() {
    for entry in std::fs::read_dir("samples").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "dwg") {
            let opened = raw::open(&path).unwrap();
            let d = dump::dump(&opened, "x.dwg", false);
            let text = dump::to_ron(&d);
            assert_eq!(dump::from_ron(&text).unwrap(), d, "{}", path.display());
        }
    }
}
