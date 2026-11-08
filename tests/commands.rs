//! The command registry (D-074) on sample drawings: every editing command
//! runs, the result decodes, and in-memory edits match the library calls
//! the examples make.

use civil3drust::aecc_elements::decode_elements;
use civil3drust::commands::run_line;
use civil3drust::raw;

fn open(name: &str) -> raw::Opened {
    raw::open(format!("{}/samples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn alignment(doc: &acadrust::CadDocument, h: u64) -> raw::RawRecord {
    raw::records(doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == h)
        .unwrap()
}

#[test]
fn set_length_matches_library_call() {
    let mut a = open("S11_100-200-300_r50.dwg");
    let mut b = open("S11_100-200-300_r50.dwg");
    run_line(&mut a.doc, "set_length 8C5C 2 150").unwrap();

    let rec = alignment(&b.doc, 0x8C5C);
    let d = decode_elements(&rec).unwrap();
    let data = raw::record_data_mut(&mut b.doc, 0x8C5C).unwrap();
    civil3drust::aecc_alignment_edit::set_tangent_length(data, &d, 1, 150.0).unwrap();
    civil3drust::aecc_topology::refresh_companion(&mut b.doc, 0x8C5C).unwrap();
    civil3drust::aecc_build::drop_record_caches(&mut b.doc, 0x8C5C);

    assert_eq!(alignment(&a.doc, 0x8C5C), alignment(&b.doc, 0x8C5C));
    let after = decode_elements(&alignment(&a.doc, 0x8C5C)).unwrap();
    assert!(
        (after.length() - 349.5756).abs() < 1e-3,
        "{}",
        after.length()
    );
}

#[test]
fn editing_commands_run_and_decode() {
    let lines = [
        "set_name FB8 \"Renamed\" --description \"two words\"",
        "set_start Renamed 1000",
        "set_speeds Renamed \"1000:50 1040:60\"",
        "set_pis Renamed \"0,0 120,10\"",
        "equations Renamed \"1050:2050\"",
        "masking Renamed \"1010-1020\"",
        "set_style Renamed 48C",
        "style_component FA9 plan:curve on",
    ];
    let mut o = open("S25_ZERO_CIVIL_second_alignment.dwg");
    for l in lines {
        let r = run_line(&mut o.doc, l);
        assert!(r.is_ok(), "{l}: {r:?}");
    }
    let rec = alignment(&o.doc, 0xFB8);
    assert_eq!(raw::object_name(&rec.data).as_deref(), Some("Renamed"));
    let d = decode_elements(&rec).unwrap();
    assert_eq!(d.stored_start_station, Some(1000.0));
    run_line(&mut o.doc, "delete Renamed").unwrap();
    assert!(raw::records(&o.doc, "AECC_ALIGNMENT")
        .iter()
        .all(|r| r.handle != 0xFB8));
}

#[test]
fn element_commands() {
    let mut o = open("S93_S92_civ3d_resaved.dwg");
    let r = run_line(&mut o.doc, "remove FB3 2").unwrap();
    assert!(r.contains("2 element(s)"), "{r}");
    assert!(run_line(&mut o.doc, "curve FB3 9 40").is_err());
}

#[test]
fn reports_and_errors() {
    let mut o = open("S25_ZERO_CIVIL_second_alignment.dwg");
    let before = raw::records(&o.doc, "AECC_ALIGNMENT");
    for l in [
        "help",
        "help create",
        "alignments",
        "styles",
        "elements F9E",
        "refs FB8",
    ] {
        let r = run_line(&mut o.doc, l);
        assert!(r.as_ref().is_ok_and(|t| !t.is_empty()), "{l}: {r:?}");
    }
    assert_eq!(before, raw::records(&o.doc, "AECC_ALIGNMENT"));
    assert!(run_line(&mut o.doc, "nope").is_err());
    assert!(run_line(&mut o.doc, "set_length F9E").is_err());
    assert!(run_line(&mut o.doc, "set_length NoSuch 1 2").is_err());
}
