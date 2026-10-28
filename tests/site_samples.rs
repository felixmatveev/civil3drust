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

fn member(p: &str, h: u64) -> Membership {
    let o = open(p);
    let rec = raw::records(&o.doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap();
    membership(&o.doc, &rec)
}

#[test]
fn two_alignments_share_the_site_container_s104_s106() {
    let in_test_site = Membership::InSite {
        site: 0x100D,
        container: 0x100E,
    };
    let s104 = "samples/S104_second_alignment_in_site.dwg";
    assert_eq!(member(s104, 0xF9E), in_test_site);
    assert_eq!(member(s104, 0x103A), in_test_site);
    // S105: TEST moved to "TEST 2".
    let s105 = "samples/S105_test_moved_to_test2.dwg";
    assert_eq!(
        member(s105, 0xF9E),
        Membership::InSite {
            site: 0x1068,
            container: 0x1069
        }
    );
    assert_eq!(member(s105, 0x103A), in_test_site);
    // S106: back to siteless with a new companion (the emptied F9D is
    // not reused).
    let s106 = "samples/S106_test_back_to_siteless.dwg";
    assert_eq!(
        member(s106, 0xF9E),
        Membership::Siteless { companion: 0x108E }
    );
}

#[test]
fn created_siteless_next_to_sites_s107() {
    let p = "samples/S107_S104_created_siteless.dwg";
    assert_eq!(
        member(p, 0x107D),
        Membership::Siteless { companion: 0x107C }
    );
    assert_eq!(sites(&open(p).doc).len(), 2);
}
