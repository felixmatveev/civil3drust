//! Sites (O7: S101–S103).

use civil3drust::aecc_site::{membership, sites, Membership};
use civil3drust::raw;

fn open(p: &str) -> raw::Opened {
    raw::open(p).unwrap()
}

#[test]
fn sites_are_listed_with_names_and_containers() {
    assert!(sites(&open("samples/S24_ZERO_CIVIL.dwg").doc).is_empty());
    let s = sites(&open("samples/S101_empty_site.dwg").doc);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].handle, s[0].name.as_str()), (0x100D, "TEST SITE"));
    assert_eq!(s[0].description, "TEST SITE DECRIPTION\r\nNEW LINE");
    assert_eq!(s[0].container, Some(0x100E));
    let s = sites(&open("samples/S103_second_site.dwg").doc);
    let names: Vec<(u64, &str)> = s.iter().map(|x| (x.handle, x.name.as_str())).collect();
    assert_eq!(names, [(0x1015, "TEST2"), (0x100D, "TEST SITE")]);
    assert_eq!(s[0].container, Some(0x1016));
}

#[test]
fn alignment_membership_follows_the_companion_references() {
    for (p, want) in [
        (
            "samples/S101_empty_site.dwg",
            Membership::Siteless { companion: 0xF9D },
        ),
        (
            "samples/S102_alignment_in_site.dwg",
            Membership::InSite {
                site: 0x100D,
                container: 0x100E,
            },
        ),
        (
            "samples/S103_second_site.dwg",
            Membership::InSite {
                site: 0x100D,
                container: 0x100E,
            },
        ),
    ] {
        let o = open(p);
        let rec = raw::records(&o.doc, "AECC_ALIGNMENT").remove(0);
        assert_eq!(membership(&o.doc, &rec), want, "{p}");
    }
}
