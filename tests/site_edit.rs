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

/// Moving in is refused where the alignment would cross a member, and
/// for a site with parcels.
#[test]
fn crossing_move_is_refused() {
    let mut o = open("S324_site_second_alignment_in_site.dwg");
    run_line(&mut o.doc, "site_move_out 8D6A").unwrap();
    // Lay 8D6A across the first segment of 8D8C, which stays in the site.
    let s = &sites(&o.doc)[0];
    let c = raw::records(&o.doc, CONT)
        .into_iter()
        .find(|r| Some(r.handle) == s.container)
        .unwrap();
    let g = civil3drust::aecc_topology::graph(&c.data).unwrap();
    let p = civil3drust::aecc_topology::edge_points(&g, &g.edges[0]);
    let (mx, my) = ((p[0].x + p[1].x) / 2.0, (p[0].y + p[1].y) / 2.0);
    let (dx, dy) = (p[1].x - p[0].x, p[1].y - p[0].y);
    let l = dx.hypot(dy);
    let (nx, ny) = (-dy / l, dx / l);
    let pis = format!(
        "{},{} {},{} {},{}",
        mx - 20.0 * nx,
        my - 20.0 * ny,
        mx + 20.0 * nx,
        my + 20.0 * ny,
        mx + 40.0 * nx + 10.0 * dx / l,
        my + 40.0 * ny + 10.0 * dy / l
    );
    run_line(&mut o.doc, &format!("set_pis 8D6A \"{pis}\"")).unwrap();
    let e = run_line(&mut o.doc, "site_move_in 8D6A A").unwrap_err();
    assert!(e.contains("crosses"), "{e}");

    // S326: a closed region (parcel) in the site.
    let mut o = open("S326_site_closed_region_parcel.dwg");
    let site = sites(&o.doc)[0].handle;
    let al = raw::records(&o.doc, "AECC_ALIGNMENT")[0].handle;
    // Its members cross each other: none can leave, none can enter.
    assert!(edit::move_out_of_site(&mut o.doc, al).is_err());
    assert!(edit::delete_site(&mut o.doc, site).is_err());
    // A site with parcels is no template for a new site either.
    assert!(edit::create_site(&mut o.doc, "Other").is_err());
}
