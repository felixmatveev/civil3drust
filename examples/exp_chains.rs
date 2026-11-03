//! Experiment: generate alignments with every element kind civil3drust can
//! encode, for the owner to open in Civil 3D.
//!
//! For each alignment of the source drawings (Civil 3D-made samples), a new
//! alignment is created in the target with `create_alignment` (template:
//! the target's alignment `--template`), and its element list is replaced
//! by the source's element records, re-encoded from their decoded fields
//! (`aecc_encode`, D-054), with three strings per curve, spiral or group
//! record (D-056), and its companion is rebuilt. Each source becomes its
//! own alignment, so a rejected one only costs itself.
//!
//! Usage: cargo run --release --example exp_chains -- target.dwg out.dwg <template hex> source.dwg[:HANDLE,…|:-HANDLE,…] ... [--plain-members] [--json|--json-tweak]
//!
//! A source may name the alignments to take (`S177.dwg:FC8,FCB`) or to
//! leave out (`S177.dwg:-FCE,FD1`). `--plain-members` writes group-member
//! spirals as ordinary spirals (header type 0x04, no start point).
//! `--json` writes the layout JSON (composed from the elements,
//! `aecc_layout`) into alignments with group-member spirals;
//! `--json-tweak` also adds 20 to each group's `Spiral1Length` in the JSON
//! only (S197, S198).

use civil3drust::aecc_alignment::Linked;
use civil3drust::aecc_create::{create_alignment, CreateOptions};
use civil3drust::aecc_element_edit::replace_elements;
use civil3drust::aecc_elements::{decode_elements, element_records, Element};
use civil3drust::aecc_layout::{compose, set_layout, Json};
use civil3drust::alignment::Point2;
use civil3drust::raw::{self, RawRecord};
use civil3drust::{aecc_design_speed, aecc_topology};

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

/// Add 20 to every group's `Spiral1Length` in the JSON (the binary
/// pieces keep the old length).
fn tweak_spiral1(j: &mut Json) {
    if let Some(Json::Arr(objs)) = j.get_mut("RGObjects") {
        for o in objs {
            if let Some(Json::Num(v)) = o
                .get_mut("ConstraintParameters")
                .and_then(|p| p.get_mut("Spiral1Length"))
            {
                *v += 20.0;
            }
        }
    }
}

fn main() {
    let mut a: Vec<String> = std::env::args().collect();
    let plain_members = a.iter().any(|x| x == "--plain-members");
    let json = a.iter().any(|x| x == "--json" || x == "--json-tweak");
    let tweak = a.iter().any(|x| x == "--json-tweak");
    a.retain(|x| !x.starts_with("--"));
    if a.len() < 5 {
        fail("usage: exp_chains target.dwg out.dwg <template hex> source.dwg ...");
    }
    let template = u64::from_str_radix(&a[3], 16).unwrap_or_else(|_| fail("bad template handle"));
    let mut target = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let donor = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let mut seed = 0x5EED_0185u64;
    let (mut ok, mut skipped) = (0, 0);
    for arg in &a[4..] {
        let (src, filter) = match arg.split_once(".dwg:") {
            Some((f, h)) => (format!("{f}.dwg"), Some(h.to_string())),
            None => (arg.clone(), None),
        };
        let (exclude, handles): (bool, Vec<u64>) = match &filter {
            Some(h) => (
                h.starts_with('-'),
                h.trim_start_matches('-')
                    .split(',')
                    .filter_map(|x| u64::from_str_radix(x, 16).ok())
                    .collect(),
            ),
            None => (true, Vec::new()),
        };
        let tag = src
            .rsplit('/')
            .next()
            .unwrap_or(&src)
            .split('_')
            .next()
            .unwrap_or("S?")
            .to_string();
        let s = raw::open(&src).unwrap_or_else(|e| fail(e));
        for rec in raw::records(&s.doc, "AECC_ALIGNMENT") {
            if handles.contains(&rec.handle) == exclude {
                continue;
            }
            let src_name = raw::object_name(&rec.data).unwrap_or_default();
            let label = format!("{tag} {:X} {src_name}", rec.handle);
            let Ok(d) = decode_elements(&rec) else {
                println!("skip  {label}: no elements / does not decode");
                skipped += 1;
                continue;
            };
            if !d.detached.is_empty() {
                println!("skip  {label}: detached pieces");
                skipped += 1;
                continue;
            }
            let mut els = element_records(&rec.data);
            if plain_members {
                for e in &mut els {
                    if let Element::Spiral(s) = e {
                        s.member = None;
                    }
                }
            }
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let name: String = label.chars().take(120).collect();
            let opts = CreateOptions {
                template: Some(template),
                ..CreateOptions::default()
            };
            let pis = [Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)];
            let created =
                create_alignment(&mut target.doc, &donor.doc, &name, &pis, &[], seed, opts)
                    .unwrap_or_else(|e| fail(format!("{label}: create: {e:?}")));
            let h = created.alignment;
            let mut new = raw::records(&target.doc, "AECC_ALIGNMENT")
                .into_iter()
                .find(|r| r.handle == h)
                .unwrap();
            // The layout JSON, composed from the element records, for
            // alignments with group-member spirals.
            let members = els
                .iter()
                .any(|e| matches!(e, Element::Spiral(s) if s.member.is_some()));
            if json && members {
                let locked = civil3drust::aecc_build::element_header_flags(&new).is_some();
                let mut j = compose(&els, locked).unwrap_or_else(|e| fail(format!("{label}: {e}")));
                if tweak {
                    tweak_spiral1(&mut j);
                }
                let (b, hb) =
                    set_layout(&new, &j).unwrap_or_else(|e| fail(format!("{label}: {e}")));
                new = RawRecord {
                    data: b,
                    handle_bits: hb,
                    ..new
                };
                println!(
                    "      {label}: layout JSON written{}",
                    if tweak { " (Spiral1Length + 20)" } else { "" }
                );
            }
            // Speeds on spirals can't be written yet: drop them if needed.
            let (bytes, hb) = match replace_elements(&new, &els, seed) {
                Ok((b, hb, _)) => (b, hb),
                Err(e) => {
                    let (b, hb) = aecc_design_speed::clear_design_speeds(&new)
                        .unwrap_or_else(|e2| fail(format!("{label}: {e:?}; clear speeds: {e2:?}")));
                    let cleared = RawRecord {
                        data: b,
                        handle_bits: hb,
                        ..new.clone()
                    };
                    let (b, hb, _) = replace_elements(&cleared, &els, seed).unwrap_or_else(|e2| {
                        fail(format!("{label}: {e:?}; without speeds: {e2:?}"))
                    });
                    println!("      {label}: design speeds dropped ({e:?})");
                    (b, hb)
                }
            };
            let r = RawRecord {
                data: bytes,
                handle_bits: hb,
                ..new.clone()
            };
            if !raw::set_record(&mut target.doc, h, r.data.clone(), r.handle_bits) {
                fail(format!("{label}: could not write the record"));
            }
            let companion = aecc_topology::refresh_companion(&mut target.doc, h).unwrap_or(false);
            let check = raw::records(&target.doc, "AECC_ALIGNMENT")
                .into_iter()
                .find(|x| x.handle == h)
                .and_then(|x| decode_elements(&x).ok())
                .map(|x| {
                    format!(
                        "{} elements, length {:.4} (source {:.4})",
                        x.record_count(),
                        x.length(),
                        d.length()
                    )
                })
                .unwrap_or_else(|| "DOES NOT DECODE".into());
            println!(
                "ok    {label} -> {h:X}: {} records ({}), {check}, companion {}",
                els.len(),
                els.iter()
                    .map(|e| e.id().to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                if companion { "rebuilt" } else { "kept" }
            );
            ok += 1;
        }
    }
    acadrust::DwgWriter::write_to_file(&a[2], &target.doc).unwrap_or_else(|e| fail(e));
    println!(
        "wrote {} ({ok} alignments, {skipped} sources skipped)",
        a[2]
    );
}
