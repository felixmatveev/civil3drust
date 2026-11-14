//! Site writers (`aecc_site_edit`, D-079) on the site study samples
//! (docs/site-object.md): the library reproduces the experiment drawings
//! Civil 3D accepted, and its results read back.

use civil3drust::aecc_site::{membership, sites, Membership};
use civil3drust::aecc_site_edit::{self as edit, Topology};
use civil3drust::commands::run_line;
use civil3drust::raw::{self, RawRecord};

const CONT: &str = "AECC_FEATURE_STYLE";

fn open(name: &str) -> raw::Opened {
    raw::open(format!("{}/samples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn samples(prefixes: &[&str]) -> Vec<String> {
    let dir = format!("{}/samples", env!("CARGO_MANIFEST_DIR"));
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".dwg") && prefixes.iter().any(|p| n.starts_with(p)))
        .collect();
    v.sort();
    v
}

/// The Civil records of a drawing, by handle.
fn civil(doc: &acadrust::CadDocument) -> Vec<RawRecord> {
    let mut v: Vec<RawRecord> = [CONT, "AECC_PARCEL", "AECC_PARCEL_NODE", "AECC_ALIGNMENT"]
        .iter()
        .flat_map(|c| raw::records(doc, c))
        .collect();
    v.sort_by_key(|r| r.handle);
    v
}

/// Every container and companion of the site samples decodes, and
/// re-encodes to a record that decodes to the same graph.
#[test]
fn topology_round_trips_on_all_site_samples() {
    let mut prefixes: Vec<String> = (101..=112)
        .chain(313..=353)
        .map(|n| format!("S{n}_"))
        .collect();
    prefixes.sort();
    let p: Vec<&str> = prefixes.iter().map(String::as_str).collect();
    let (mut seen, mut with_free) = (0, 0);
    for name in samples(&p) {
        let o = open(&name);
        for r in raw::records(&o.doc, CONT) {
            // The registry (owner of the companions) is not a graph.
            let Ok(t) = Topology::parse(&r) else {
                // Not graphs: the registry that owns the companions, and
                // the S108 experiment (companions written by civil3drust
                // in an older form) with its recovered copy S109.
                assert!(
                    civil3drust::aecc_registry::Registry::parse(&r).is_ok()
                        || name.starts_with("S108_")
                        || name.starts_with("S109_"),
                    "{name} {:X}",
                    r.handle
                );
                continue;
            };
            let (data, hb) = t.write(&r).unwrap();
            let again = Topology::parse(&RawRecord {
                data,
                handle_bits: hb,
                ..r.clone()
            })
            .unwrap_or_else(|e| panic!("{name} {:X}: {e}", r.handle));
            assert_eq!(t, again, "{name} {:X}", r.handle);
            seen += 1;
            with_free += t.edges.iter().any(|e| e.is_none()) as usize;
        }
    }
    assert!(seen > 150, "{seen}");
    assert!(with_free > 20, "{with_free}");
}

/// `aecc_topology::graph` skips freed edge entries and reports edge IDs.
#[test]
fn graph_reads_containers_with_freed_entries() {
    // S343: Civil 3D's save of S334 — edge 1 freed, edge 2 live.
    let o = open("S343_S334_resaved.dwg");
    let s = &sites(&o.doc)[0];
    let c = raw::records(&o.doc, CONT)
        .into_iter()
        .find(|r| Some(r.handle) == s.container)
        .unwrap();
    let g = civil3drust::aecc_topology::graph(&c.data).unwrap();
    assert_eq!(g.edges.len(), 1);
    assert_eq!(g.edges[0].id, 2);
}

fn same_civil_records(ours: &acadrust::CadDocument, sample: &str) {
    let theirs = open(sample);
    let (a, b) = (civil(ours), civil(&theirs.doc));
    assert_eq!(a.len(), b.len(), "{sample}: record count");
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.handle, y.handle, "{sample}");
        // Trailing padding of the handle stream is not compared.
        let trim = |r: &RawRecord| {
            civil3drust::handle_refs::trim_padding(&r.data, r.handle_bits, r.handle)
        };
        assert_eq!(trim(x), trim(y), "{sample}: record {:X}", x.handle);
    }
}

/// The library calls give the records of the experiment drawings Civil 3D
/// accepted.
#[test]
fn library_reproduces_accepted_experiments() {
    let mut o = open("S334_S324_exp_move_out.dwg");
    edit::move_into_site(&mut o.doc, 0x8D6A, 0x8D58).unwrap();
    same_civil_records(&o.doc, "S345_S334_exp_move_in_edge_id.dwg");

    let mut o = open("S331_site_second_site_deleted.dwg");
    edit::rename_site(
        &mut o.doc,
        0x8D58,
        Some("RENAMED"),
        Some("written by civil3drust"),
    )
    .unwrap();
    same_civil_records(&o.doc, "S336_S331_exp_rename.dwg");

    let mut o = open("S322_site_alignment_siteless.dwg");
    edit::rename_site(&mut o.doc, 0x8D58, Some("A"), None).unwrap();
    same_civil_records(&o.doc, "S337_S322_exp_name_unnamed.dwg");

    let mut o = open("S322_site_alignment_siteless.dwg");
    edit::delete_site(&mut o.doc, 0x8D58).unwrap();
    same_civil_records(&o.doc, "S338_S322_exp_delete_site.dwg");

    let mut o = open("S322_site_alignment_siteless.dwg");
    edit::create_site(&mut o.doc, "NEW SITE").unwrap();
    same_civil_records(&o.doc, "S339_S322_exp_create_site.dwg");

    let mut o = open("S331_site_second_site_deleted.dwg");
    edit::create_site(&mut o.doc, "A SITE").unwrap();
    // S340 was written with the list in another order (accepted; Civil 3D
    // put it in key order on save, S348): the same sites, in S348's order.
    let names = |d: &acadrust::CadDocument| -> Vec<String> {
        sites(d).into_iter().map(|s| s.name).collect()
    };
    assert_eq!(names(&o.doc), names(&open("S348_S340_resaved.dwg").doc));
    assert_eq!(names(&o.doc), ["B", "A SITE"]);

    let mut o = open("S321_site_empty_site.dwg");
    for key in [0x101u32, 0x79D, 0x200D216] {
        edit::create_site_with_builtin_name(&mut o.doc, key, &format!("id {key:08X}")).unwrap();
    }
    let s = sites(&o.doc);
    assert_eq!(s.len(), 4);
    assert!(s
        .iter()
        .any(|x| x.name == "Radius in" && x.description == "id 0200D216"));
}

/// A chain of site commands; the result reads back.
#[test]
fn site_commands_chain() {
    let mut o = open("S324_site_second_alignment_in_site.dwg");
    for l in [
        "sites",
        "site_move_out 8D6A",
        "site_create Second",
        "site_move_in 8D6A Second",
        "site_rename A Main --description \"first site\"",
        "site_rename Second - --description \"second\"",
    ] {
        let r = run_line(&mut o.doc, l);
        assert!(r.is_ok(), "{l}: {r:?}");
    }
    let s = sites(&o.doc);
    let get = |n: &str| {
        s.iter()
            .find(|x| x.name == n)
            .unwrap_or_else(|| panic!("{n}"))
    };
    assert_eq!(get("Main").description, "first site");
    assert_eq!(get("Second").description, "second");
    let site_of = |doc: &acadrust::CadDocument, h: u64| {
        let r = raw::records(doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        match membership(doc, &r) {
            Membership::InSite { site, .. } => Some(site),
            _ => None,
        }
    };
    assert_eq!(site_of(&o.doc, 0x8D6A), Some(get("Second").handle));
    assert_eq!(site_of(&o.doc, 0x8D8C), Some(get("Main").handle));
    // Each alignment's stored edge ID is the ID of its edge.
    let text = edit::describe(&o.doc).unwrap();
    assert!(!text.contains("DIFFERS"), "{text}");
    assert_eq!(text.matches("edge ID field matches").count(), 2, "{text}");

    // Refusals: a site with members, a duplicate name, an unknown site.
    assert!(run_line(&mut o.doc, "site_delete Second").is_err());
    assert!(run_line(&mut o.doc, "site_create Main").is_err());
    assert!(run_line(&mut o.doc, "site_rename Nowhere X").is_err());
    assert!(run_line(&mut o.doc, "site_move_in 8D6A Main").is_err());

    run_line(&mut o.doc, "site_move_out 8D6A").unwrap();
    run_line(&mut o.doc, "site_delete Second").unwrap();
    assert_eq!(sites(&o.doc).len(), 1);
    assert_eq!(site_of(&o.doc, 0x8D6A), None);

    // The written drawing reads back the same.
    let path = std::env::temp_dir().join("civil3drust_site_chain.dwg");
    acadrust::DwgWriter::write_to_file(&path, &o.doc).unwrap();
    let back = raw::open(&path).unwrap();
    assert_eq!(sites(&back.doc), sites(&o.doc));
    assert_eq!(civil(&back.doc), civil(&o.doc));
    let _ = std::fs::remove_file(path);
}

/// Crossing members are written without crossing points: Civil 3D adds
/// them (S366 → S367). The library's move of S355 is S366 record for
/// record (its edge copied from the companion). Sites with parcels are
/// still refused.
#[test]
fn crossing_moves() {
    let mut o = open("S355_S354_cross_second_out.dwg");
    run_line(&mut o.doc, "site_move_in 8C58 \"TEST SITE\"").unwrap();
    // The container equals S366's but for the last digits of one node
    // (the companion's −50 where S366 decoded −49.999…).
    let theirs = open("S366_S355_exp_cross_no_node.dwg");
    let topo = |d: &acadrust::CadDocument| {
        let r = raw::records(d, CONT)
            .into_iter()
            .find(|r| r.handle == 0x8C43)
            .unwrap();
        Topology::parse(&r).unwrap()
    };
    let (mut a, b) = (topo(&o.doc), topo(&theirs.doc));
    assert_eq!(a.nodes.len(), b.nodes.len());
    for (x, y) in a.nodes.iter().zip(&b.nodes) {
        assert!(x.0 == y.0 && (x.1 - y.1).abs() < 1e-9 && (x.2 - y.2).abs() < 1e-9);
    }
    a.nodes = b.nodes.clone();
    assert_eq!(a, b);

    // Out of Civil 3D's own crossing (S354, S356, S358, S360): the shared
    // node stays for the remaining edge, one loop per part.
    for name in [
        "S354_site_cross.dwg",
        "S356_site_tee.dwg",
        "S358_site_end_to_end.dwg",
        "S360_site_arc_cross.dwg",
    ] {
        let mut o = open(name);
        let m =
            edit::move_out_of_site(&mut o.doc, 0x8C58).unwrap_or_else(|e| panic!("{name}: {e}"));
        let c = raw::records(&o.doc, CONT)
            .into_iter()
            .find(|r| r.handle == m.container)
            .unwrap();
        let t = Topology::parse(&c).unwrap();
        assert_eq!(t.members, [0x8C4F], "{name}");
        let live: Vec<_> = t.edges.iter().flatten().collect();
        assert_eq!(live.len(), 1, "{name}");
        assert!(
            live[0]
                .vertices
                .iter()
                .all(|v| t.nodes.iter().any(|n| n.0 == v.node)),
            "{name}"
        );
        assert_eq!(t.faces[0].loops.len(), 1, "{name}");
        assert!(
            live[0]
                .vertices
                .iter()
                .any(|v| v.node == t.faces[0].loops[0].0),
            "{name}"
        );
        // The alignment's companion holds the edge it had in the site.
        let comp = raw::records(&o.doc, CONT)
            .into_iter()
            .find(|r| r.handle == m.companion)
            .unwrap();
        let ct = Topology::parse(&comp).unwrap();
        assert_eq!(ct.members, [0x8C58], "{name}");
        assert_eq!(
            edit::describe(&o.doc).unwrap().matches("DIFFERS").count(),
            0,
            "{name}"
        );
    }

    // S326: a closed region (parcel) in the site.
    let mut o = open("S326_site_closed_region_parcel.dwg");
    let site = sites(&o.doc)[0].handle;
    let al = raw::records(&o.doc, "AECC_ALIGNMENT")[0].handle;
    assert!(edit::move_out_of_site(&mut o.doc, al).is_err());
    assert!(edit::delete_site(&mut o.doc, site).is_err());
    assert!(edit::delete_site_with_members(&mut o.doc, site).is_err());
    // A site with parcels is no template for a new site either.
    assert!(edit::create_site(&mut o.doc, "Other").is_err());
}

/// Spiral alignments move with their companion's edge (S370 accepted);
/// moving out gives the companion the container's edge.
#[test]
fn spiral_moves() {
    let mut o = open("S369_site_spiral_out.dwg");
    edit::move_into_site(&mut o.doc, 0x8C49, 0x8C42).unwrap();
    same_civil_records(&o.doc, "S370_S369_exp_spiral_companion.dwg");

    let mut o = open("S368_site_spiral_in.dwg");
    let m = edit::move_out_of_site(&mut o.doc, 0x8C49).unwrap();
    let comp = raw::records(&o.doc, CONT)
        .into_iter()
        .find(|r| r.handle == m.companion)
        .unwrap();
    let e = Topology::parse(&comp).unwrap();
    let e = e.edges.iter().flatten().next().unwrap();
    assert_eq!(e.vertices.len(), 24);
    assert_eq!(e.vertices.iter().filter(|v| v.bulge.is_some()).count(), 21);
}

/// A free spiral-curve-spiral between two lines is fitted at the lines'
/// intersection (S368: the first line's stored end is not the PI). The
/// decoded points are those of Civil 3D's site container.
#[test]
fn free_scs_is_fitted_between_lines() {
    let o = open("S368_site_spiral_in.dwg");
    let r = &raw::records(&o.doc, "AECC_ALIGNMENT")[0];
    let d = civil3drust::aecc_elements::decode_elements(r).unwrap();
    let ends: Vec<(f64, f64)> = d
        .elements
        .iter()
        .map(|e| (e.segment.end().x, e.segment.end().y))
        .collect();
    let want = [
        (318.2213, 0.0),
        (356.0390, -6.0618),
        (362.7239, -10.4066),
        (383.6184, -42.5039),
        (550.1067, -417.1977),
    ];
    assert_eq!(ends.len(), want.len());
    for (g, w) in ends.iter().zip(want) {
        assert!(
            (g.0 - w.0).abs() < 1e-3 && (g.1 - w.1).abs() < 1e-3,
            "{g:?} vs {w:?}"
        );
    }
}

/// Deleting a site with its members gives S375 (Civil 3D's delete of
/// S374): the same records, the labeling record's alignment reference
/// nulled. Only the class node Civil 3D re-writes under new handles on
/// any save differs.
#[test]
fn delete_site_with_members_matches_civil3d() {
    let mut o = open("S374_site_with_members.dwg");
    let r = run_line(&mut o.doc, "site_delete \"TEST SITE\"");
    assert!(
        r.is_err(),
        "without --with-members a site with members stays"
    );
    run_line(&mut o.doc, "site_delete \"TEST SITE\" --with-members").unwrap();
    let theirs = open("S375_S374_site_deleted.dwg");
    let classes = [
        "AECC_ALIGNMENT",
        "AECC_PARCEL",
        "AECC_PARCEL_NODE",
        "AECC_FEATURE_LINE",
        "AECC_ALIGNMENT_VAGEOMPOINT_LABELING",
        "AECC_ALIGNMENT_STATION_LABEL_GROUP",
        "AECC_ALIGNMENT_GEOMPOINT_LABEL_GROUP",
    ];
    for c in classes {
        let trim = |r: &RawRecord| {
            (
                r.handle,
                civil3drust::handle_refs::trim_padding(&r.data, r.handle_bits, r.handle),
            )
        };
        let a: Vec<_> = raw::records(&o.doc, c).iter().map(trim).collect();
        let b: Vec<_> = raw::records(&theirs.doc, c).iter().map(trim).collect();
        assert_eq!(a, b, "{c}");
    }
    assert!(sites(&o.doc).is_empty());
}

/// Deleting a siteless alignment nulls the `*_LABELING` references to it
/// (as S375), instead of leaving them dangling.
#[test]
fn delete_alignment_nulls_labeling_refs() {
    let mut o = open("S369_site_spiral_out.dwg");
    let refs = |doc: &acadrust::CadDocument| -> Vec<u64> {
        raw::records(doc, "AECC_ALIGNMENT_VAGEOMPOINT_LABELING")
            .iter()
            .flat_map(civil3drust::handle_refs::of_record)
            .map(|x| x.handle)
            .collect()
    };
    assert!(
        refs(&o.doc).contains(&0x8C49),
        "the sample has a labeling record for 8C49"
    );
    civil3drust::aecc_delete::delete_alignment(&mut o.doc, 0x8C49).unwrap();
    assert!(!refs(&o.doc).contains(&0x8C49));
    assert!(!raw::records(&o.doc, "AECC_ALIGNMENT_VAGEOMPOINT_LABELING").is_empty());
}
