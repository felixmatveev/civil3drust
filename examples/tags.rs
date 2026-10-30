//! Catalog candidate sub-object tags in AECC records (docs/class-catalog.md).
//!
//! A candidate is a full-form BL (code `00` + 4 bytes) with a value of at
//! least 0x10000, outside the proxy graphics. Constant data repeats in
//! every record of a class, so what marks a sub-object type tag is reuse
//! across different classes (`0x04020008`: alignment elements, profile
//! styles, …). Scanning bit by bit also finds the same bytes one bit off,
//! so a real tag comes with shifted neighbours (e.g. `0x02010004` and
//! `0x08040010` around `0x04020008`); every hit is kept, and a value is
//! only taken as a tag once a test or a decoded block confirms it.
//!
//! Usage: cargo run --release --example tags -- [--min N] [--min-classes K] [--class AECC_X] a.dwg [b.dwg ...]
//!
//! Prints, per value seen in at least N records (default 5) of at least K
//! classes (default 3): the number of records and drawings, the classes it
//! occurs in, and for `AECC_ALIGNMENT` the section-2 offsets where it
//! occurs.

use civil3drust::raw;
use std::collections::{BTreeMap, BTreeSet};

struct Hit {
    bit: usize,
    value: u32,
}

fn hits(d: &[u8], end: usize) -> Vec<Hit> {
    let mut out = Vec::new();
    let from = civil3drust::proxy_graphics::find(d, 4096).map_or(0, |g| g.end_bit());
    for b in from..end.saturating_sub(34) {
        if raw::get_bit(d, b) != 0 || raw::get_bit(d, b + 1) != 0 {
            continue;
        }
        let v = (0..4).fold(0u32, |v, i| {
            v | (raw::get_byte(d, b + 2 + 8 * i) as u32) << (8 * i)
        });
        if v >= 0x1_0000 && v != u32::MAX {
            out.push(Hit { bit: b, value: v });
        }
    }
    out
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut min = 5usize;
    let mut min_classes = 3usize;
    let mut only: Option<String> = None;
    while let Some(i) = args
        .iter()
        .position(|a| a == "--min" || a == "--min-classes" || a == "--class")
    {
        let v = args.remove(i + 1);
        match args.remove(i).as_str() {
            "--min" => min = v.parse().expect("--min N"),
            "--min-classes" => min_classes = v.parse().expect("--min-classes K"),
            _ => only = Some(v),
        }
    }
    // Pass 1: every record's raw hits.
    struct Rec {
        file: usize,
        class: String,
        s2: Option<usize>,
        hits: Vec<Hit>,
    }
    let mut recs = Vec::new();
    for (fi, p) in args.iter().enumerate() {
        let Ok(o) = raw::open(p) else {
            eprintln!("skip {p}: not readable");
            continue;
        };
        let names: Vec<String> = o
            .doc
            .classes
            .iter()
            .filter(|c| c.dxf_name.starts_with("AECC"))
            .filter(|c| only.as_ref().is_none_or(|x| *x == c.dxf_name))
            .map(|c| c.dxf_name.clone())
            .collect();
        for c in names {
            for r in raw::records(&o.doc, &c) {
                let end = raw::bit_len(&r.data).saturating_sub(r.handle_bits);
                let s2 = if c == "AECC_ALIGNMENT" {
                    civil3drust::aecc_entity::entity_common(&r.data).map(|e| e.start_bit)
                } else {
                    None
                };
                recs.push(Rec {
                    file: fi,
                    class: c.clone(),
                    s2,
                    hits: hits(&r.data, end),
                });
            }
        }
    }
    #[derive(Default)]
    struct Stat {
        records: usize,
        files: BTreeSet<usize>,
        classes: BTreeMap<String, usize>,
        s2_offsets: BTreeSet<usize>,
    }
    let mut stats: BTreeMap<u32, Stat> = BTreeMap::new();
    for r in &recs {
        let kept: BTreeSet<(usize, u32)> = r.hits.iter().map(|h| (h.bit, h.value)).collect();
        let values: BTreeSet<u32> = kept.iter().map(|k| k.1).collect();
        for v in values {
            let s = stats.entry(v).or_default();
            s.records += 1;
            s.files.insert(r.file);
            *s.classes.entry(r.class.clone()).or_default() += 1;
        }
        if let Some(s2) = r.s2 {
            for (b, v) in kept {
                if b >= s2 {
                    stats.entry(v).or_default().s2_offsets.insert(b - s2);
                }
            }
        }
    }
    let mut rows: Vec<(&u32, &Stat)> = stats
        .iter()
        .filter(|(_, s)| s.records >= min && s.classes.len() >= min_classes)
        .collect();
    rows.sort_by_key(|(v, s)| (std::cmp::Reverse(s.records), **v));
    println!("{} records scanned in {} drawings", recs.len(), args.len());
    for (v, s) in rows {
        let classes: Vec<String> = s
            .classes
            .iter()
            .map(|(c, n)| format!("{}×{n}", c.trim_start_matches("AECC_")))
            .collect();
        let offs: Vec<String> = s.s2_offsets.iter().take(8).map(|o| o.to_string()).collect();
        println!(
            "{v:#010x}  {:5} records  {:3} drawings  {:3} classes  {}{}",
            s.records,
            s.files.len(),
            s.classes.len(),
            classes.join(" "),
            if offs.is_empty() {
                String::new()
            } else {
                format!("  | alignment section-2 offsets {}", offs.join(","))
            }
        );
    }
}
