//! Experiment S80: which parts switch design criteria on (O3)?
//!
//! Builds one drawing with four created alignments:
//!
//! - `C0 control`  — unchanged
//! - `C1 flags`    — the three section-2 criteria flags set
//! - `C2 checkset` — flags + last reference to a design check set
//! - `C3 full`     — flags + check set + the five strings Civil 3D stored in
//!   S79 (criteria file, its `<DATADIR>` form, superelevation table, lanes,
//!   attainment method)
//!
//! Usage: cargo run --release --example exp_criteria -- donor.dwg target.dwg out.dwg <check-set-hex>

use acadrust::entities::EntityType;
use civil3drust::aecc_create::{create_alignment, CreateOptions};
use civil3drust::aecc_criteria::{criteria_flags, set_criteria_flags};
use civil3drust::alignment::Point2;
use civil3drust::{handle_refs, raw, string_stream};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

/// The strings Civil 3D wrote for criteria on in S79 (index after the name).
const S79_STRINGS: [&str; 5] = [
    r"C:\ProgramData\Autodesk\C3D 2024\enu\Data\Corridor Design Standards\Imperial\_Autodesk Civil 3D Imperial (2011) Roadway Design Standards.xml",
    r"<DATADIR>Corridor Design Standards\Imperial\_Autodesk Civil 3D Imperial (2011) Roadway Design Standards.xml",
    "AASHTO 2011 US Customary eMax 4%",
    "2 Lane",
    "AASHTO 2011 Crowned Roadway",
];

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        eprintln!("usage: exp_criteria <donor.dwg> <target.dwg> <out.dwg> <check-set-hex>");
        std::process::exit(2);
    }
    let check_set = u64::from_str_radix(&a[4], 16).unwrap_or_else(|_| fail("check set hex"));
    let donor = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let mut t = raw::open(&a[2]).unwrap_or_else(|e| fail(e));
    if raw::class_of(&t.doc, check_set).as_deref() != Some("AECC_ALIGNMENT_DESIGN_CHECK_SET") {
        fail(format!(
            "{check_set:X} is not a design check set in the target"
        ));
    }
    for (k, name) in ["C0 control", "C1 flags", "C2 checkset", "C3 full"]
        .iter()
        .enumerate()
    {
        let y = -100.0 * (k as f64 + 1.0);
        let pis = [Point2::new(0.0, y), Point2::new(500.0, y)];
        let c = create_alignment(
            &mut t.doc,
            &donor.doc,
            name,
            &pis,
            &[],
            40 + k as u64,
            CreateOptions::default(),
        )
        .unwrap_or_else(|e| fail(e));
        let rec = raw::records(&t.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == c.alignment)
            .unwrap();
        let mut d = rec.data.clone();
        let mut hb = rec.handle_bits;
        if k >= 1 {
            set_criteria_flags(&rec, &mut d, [true; 3]).unwrap_or_else(|e| fail(e));
        }
        if k >= 2 {
            let r = raw::RawRecord {
                data: d.clone(),
                handle_bits: hb,
                ..rec.clone()
            };
            let mut targets: Vec<u64> = handle_refs::of_record(&r)
                .iter()
                .map(|x| x.handle)
                .collect();
            *targets.last_mut().unwrap() = check_set;
            (d, hb) = handle_refs::rewrite_refs(&d, hb, rec.handle, &targets)
                .unwrap_or_else(|| fail("refs"));
        }
        if k >= 3 {
            let ss = string_stream::parse(&d, hb).unwrap_or_else(|e| fail(e));
            let name_i = ss
                .strings
                .iter()
                .rposition(|s| s.text == *name)
                .unwrap_or_else(|| fail("name string"));
            for (i, s) in S79_STRINGS.iter().enumerate() {
                (d, hb) =
                    string_stream::replace(&d, hb, name_i + 1 + i, s).unwrap_or_else(|e| fail(e));
            }
        }
        let (d, hb) = handle_refs::trim_padding(&d, hb, rec.handle);
        for e in t.doc.entities_mut() {
            if let EntityType::Unknown(u) = e {
                if u.common.handle.value() == rec.handle {
                    u.raw_dwg_data = Some(d.clone());
                    u.dwg_handle_bits = hb as i64;
                }
            }
        }
    }
    acadrust::DwgWriter::write_to_file(&a[3], &t.doc).unwrap_or_else(|e| fail(e));
    let re = raw::open(&a[3]).unwrap_or_else(|e| fail(format!("does not read back: {e}")));
    for r in raw::records(&re.doc, "AECC_ALIGNMENT") {
        let refs = handle_refs::of_record(&r);
        let ss = string_stream::parse(&r.data, r.handle_bits).unwrap();
        println!(
            "[{:X}] {:<12} flags {:?} last ref {:X} strings {}",
            r.handle,
            raw::object_name(&r.data).unwrap_or_default(),
            criteria_flags(&r).unwrap_or_else(|e| fail(e)),
            refs.last().map(|x| x.handle).unwrap_or(0),
            ss.strings.len()
        );
    }
}
