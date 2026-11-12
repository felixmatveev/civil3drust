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

/// Sites whose name is one of Civil 3D's built-in names store no name
/// string (MIST22): they are still listed, with their key, and the name
/// comes from the IDs our samples have shown.
#[test]
fn pooled_site_names() {
    // S321: one site, which the API calls `A`; no name string.
    let s = sites(&open("samples/S321_site_empty_site.dwg").doc);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].name.as_str(), s[0].name_key), ("A", 0x0200_D214));
    assert_eq!(s[0].description, "");
    // S329: a second pooled site, whose ID is not known, before `B`.
    let s = sites(&open("samples/S329_site_second_site_fl_moved.dwg").doc);
    assert_eq!(s.len(), 2);
    assert_eq!((s[0].name.as_str(), s[0].name_key), ("", 0x0000_0101));
    assert_eq!((s[1].name.as_str(), s[1].name_key), ("B", 0x8000_0AFB));
    // S323: the alignment is found in the pooled-name site.
    let o = open("samples/S323_site_alignment_into_site.dwg");
    let a = &civil3drust::raw::records(&o.doc, "AECC_ALIGNMENT")[0];
    assert!(matches!(
        membership(&o.doc, a),
        Membership::InSite { site: 0x8D58, .. }
    ));
    // S337 (civil3drust: a literal `A`) and S350 (Civil 3D's save of it,
    // pooled again) read the same.
    for p in [
        "samples/S337_S322_exp_name_unnamed.dwg",
        "samples/S350_S337_resaved.dwg",
    ] {
        let s = sites(&open(p).doc);
        assert_eq!(s.len(), 1, "{p}");
        assert_eq!(s[0].name, "A", "{p}");
    }
    // S336/S349: name and description strings.
    let s = sites(&open("samples/S349_S336_resaved.dwg").doc);
    assert_eq!(s[0].name, "RENAMED");
    assert_eq!(s[0].description, "written by civil3drust");
    // S338/S346: no site left.
    assert!(sites(&open("samples/S346_S338_resaved.dwg").doc).is_empty());
}
