//! Experiment (S216): a spiral group written only as typed parameters.
//!
//! For each source alignment with a JSON layout (S177/S178), a new
//! alignment is created in the target from `<template hex>` (an alignment
//! in Civil 3D's JSON form, e.g. S178 11F8). Its element records are only
//! the source's fixed elements (no group pieces); its JSON
//! (`aecc_layout::layout`) holds those elements and the group with the
//! source's typed parameters, or the ones given. Civil 3D rebuilds group
//! pieces from the JSON when MIST1 does not match (S214 → S215); the
//! question is whether it also builds them when there are none.
//!
//! The records are written with `replace_elements`; the reader takes the
//! JSON group in place of the gap (`DecodedElements::layout_groups`).
//!
//! Each spec: `source.dwg:HANDLE:direct|group[:p1,p2,…]`
//! - `direct`: the two fixed elements linked to each other;
//! - `group`: linked to the group's ID, as in the JSON (no record has it);
//! - `end` / `start`: the fixed element after / before the group left
//!   out, the group at that end of the alignment (`NextId` / `PreviousId`
//!   0), the remaining element linked to the group's ID (S223; `layout`
//!   now refuses these, so they can no longer be generated);
//! - parameters: replace the source's typed ones (method order).
//!
//! Usage: exp_group_params target.dwg out.dwg <template hex> spec ...

use civil3drust::aecc_alignment::Linked;
use civil3drust::aecc_build::element_header_flags;
use civil3drust::aecc_create::{create_alignment, CreateOptions};
use civil3drust::aecc_element_edit::replace_elements;
use civil3drust::aecc_elements::{decode_elements, element_records, Element};
use civil3drust::aecc_layout::{group_parameters, layout, set_layout, Group, Json};
use civil3drust::alignment::Point2;
use civil3drust::raw::{self, RawRecord};
use civil3drust::{aecc_topology, string_stream};

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn num(j: &Json, k: &str) -> f64 {
    match j.get(k) {
        Some(Json::Num(v)) => *v,
        _ => fail(format!("JSON: no {k}")),
    }
}

fn set_links(e: &mut Element, before: i32, after: i32) {
    match e {
        Element::Tangent(t) => {
            t.before = before;
            t.after = after;
        }
        Element::Arc(a) => {
            a.before = before;
            a.after = after;
        }
        _ => fail("only fixed lines and curves"),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 {
        fail("usage: exp_group_params target.dwg out.dwg <template hex> source.dwg:H:direct|group|end|start[:p,…] ...");
    }
    let template = u64::from_str_radix(&a[3], 16).unwrap_or_else(|_| fail("template"));
    let mut target = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let donor = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let mut seed = 0x5EED_0216u64;
    for spec in &a[4..] {
        let parts: Vec<&str> = spec.split(':').collect();
        let (file, h, mode) = (parts[0], parts[1], parts[2]);
        let h = u64::from_str_radix(h, 16).unwrap_or_else(|_| fail("handle"));
        let s = raw::open(file).unwrap_or_else(|e| fail(e));
        let rec = raw::records(&s.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap_or_else(|| fail(format!("{h:X} not in {file}")));
        let text = string_stream::layout_json(&rec.data, rec.handle_bits)
            .unwrap_or_else(|| fail(format!("{h:X}: no JSON")));
        let j = Json::parse(&text).unwrap_or_else(|e| fail(e));
        let Some(Json::Arr(objs)) = j.get("RGObjects") else {
            fail("no RGObjects")
        };
        let g = objs
            .iter()
            .find(|o| num(o, "RGObjectType") == 7.0)
            .unwrap_or_else(|| fail("no group"));
        let method = match num(g, "ConstraintType") as i32 {
            2569 => "SSCSS",
            2561 => "SSCS",
            2565 => "SCSS",
            c => fail(format!("group type {c}")),
        };
        let names = group_parameters(method).unwrap();
        let typed: Vec<f64> = names
            .iter()
            .map(|n| num(g.get("ConstraintParameters").unwrap(), n))
            .collect();
        let params = match parts.get(3) {
            Some(p) => p
                .split(',')
                .map(|v| v.parse().unwrap_or_else(|_| fail("parameter")))
                .collect(),
            None => typed.clone(),
        };
        let (prev, next) = (num(g, "PreviousId") as i32, num(g, "NextId") as i32);
        let gid = num(g, "Id") as i32;
        // The fixed elements: the records whose IDs are fixed objects.
        let fixed_ids: Vec<i32> = objs
            .iter()
            .filter(|o| num(o, "RGObjectType") != 7.0)
            .map(|o| num(o, "Id") as i32)
            .collect();
        let mut fixed: Vec<Element> = element_records(&rec.data)
            .into_iter()
            .filter(|e| fixed_ids.contains(&e.id()))
            .collect();
        for e in &mut fixed {
            let (b, n) = (e.before(), e.after());
            let link = |x: i32| -> i32 {
                if fixed_ids.contains(&x) || x == 0 {
                    return x;
                }
                // A piece: link to the other fixed element or to the group.
                match mode {
                    "direct" => {
                        if e.id() == prev {
                            next
                        } else {
                            prev
                        }
                    }
                    _ => gid,
                }
            };
            let (nb, nn) = (link(b), link(n));
            set_links(e, nb, nn);
        }
        // Edge cases: the group at an end of the alignment (S219).
        let (gprev, gnext) = match mode {
            "end" => {
                fixed.retain(|e| e.id() != next);
                (prev, 0)
            }
            "start" => {
                // The remaining element is the first record (before 0);
                // the JSON's FirstId names the group.
                fixed.retain(|e| e.id() != prev);
                for e in &mut fixed {
                    if e.before() == gid {
                        let a = e.after();
                        set_links(e, 0, a);
                    }
                }
                (0, next)
            }
            _ => (prev, next),
        };
        // `layout` numbers the group after the highest fixed ID.
        let new_gid = fixed.iter().map(|e| e.id()).max().unwrap_or(0) + 1;
        for e in &mut fixed {
            let r = |x: i32| if x == gid { new_gid } else { x };
            let (b, n) = (r(e.before()), r(e.after()));
            set_links(e, b, n);
        }
        let name = format!(
            "G {mode} {} {h:X} {}",
            file.rsplit('/').next().unwrap().split('_').next().unwrap(),
            names
                .iter()
                .zip(&params)
                .map(|(n, v)| format!("{}={v}", n.replace("Length", "").replace("Arc", "")))
                .collect::<Vec<_>>()
                .join(" ")
        );
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let opts = CreateOptions {
            template: Some(template),
            ..CreateOptions::default()
        };
        let pis = [Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)];
        let created = create_alignment(&mut target.doc, &donor.doc, &name, &pis, &[], seed, opts)
            .unwrap_or_else(|e| fail(format!("{name}: create: {e:?}")));
        let nh = created.alignment;
        let new = raw::records(&target.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| r.handle == nh)
            .unwrap();
        let locked = element_header_flags(&new).is_some();
        let json = layout(
            &fixed,
            &[Group {
                method,
                locked,
                previous: gprev,
                next: gnext,
                params,
            }],
            locked,
        )
        .unwrap_or_else(|e| fail(e));
        let (b, hb) = set_layout(&new, &json).unwrap_or_else(|e| fail(e));
        let new = RawRecord {
            data: b,
            handle_bits: hb,
            ..new
        };
        let (b, hb, _) =
            replace_elements(&new, &fixed, seed).unwrap_or_else(|e| fail(format!("{name}: {e}")));
        if !raw::set_record(&mut target.doc, nh, b, hb) {
            fail("could not write");
        }
        aecc_topology::refresh_companion(&mut target.doc, nh).ok();
        let check = raw::records(&target.doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|x| x.handle == nh)
            .map(|x| match decode_elements(&x) {
                Ok(d) => format!(
                    "{} elements, {} JSON group(s), complete {}, length {:.4}",
                    d.elements.len(),
                    d.layout_groups.len(),
                    d.is_complete(),
                    d.length()
                ),
                Err(e) => format!("does not decode: {e}"),
            })
            .unwrap();
        println!("{nh:X} {name}: {check}");
    }
    acadrust::DwgWriter::write_to_file(&a[2], &target.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[2]);
}
