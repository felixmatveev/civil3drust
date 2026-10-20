//! Compare AECC records between two DWG files at bit level.
//!
//! Usage:
//!   cargo run --release --example rawdiff -- a.dwg b.dwg            # summary of all AECC classes
//!   cargo run --release --example rawdiff -- a.dwg b.dwg AECC_ALIGNMENT   # detail for one class
//!
//! Records are matched by handle. For equal-size records the tool lists the
//! changed bit ranges and, for each, the raw double that best explains the
//! change (old -> new). Intended for controlled sample pairs, e.g. "same
//! alignment, segment 2 lengthened by 10 ft".

use civil3drust::inventory::aecc_classes;
use civil3drust::raw::{self, RawRecord};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: rawdiff <a.dwg> <b.dwg> [AECC_CLASS]");
        std::process::exit(2);
    }
    let a = open(&args[1]);
    let b = open(&args[2]);
    match args.get(3) {
        None => summary(&a, &b),
        Some(class) => detail(&a, &b, class),
    }
}

fn open(path: &str) -> acadrust::document::CadDocument {
    match raw::open(path) {
        Ok(o) => {
            if let Some(r) = o.failsafe_reason {
                eprintln!("note: {path} read in failsafe mode ({r})");
            }
            o.doc
        }
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            std::process::exit(1);
        }
    }
}

fn by_handle(recs: Vec<RawRecord>) -> BTreeMap<u64, RawRecord> {
    recs.into_iter().map(|r| (r.handle, r)).collect()
}

fn summary(a: &acadrust::document::CadDocument, b: &acadrust::document::CadDocument) {
    let mut classes: Vec<String> = aecc_classes(a).into_iter().map(|c| c.dxf_name).collect();
    for c in aecc_classes(b) {
        if !classes.contains(&c.dxf_name) {
            classes.push(c.dxf_name);
        }
    }
    classes.sort();
    println!(
        "{:<52} {:>6} {:>6} {:>8} {:>8} {:>7} {:>7}",
        "class", "a", "b", "same", "changed", "resized", "1-side"
    );
    let mut total_changed = 0;
    for class in classes {
        let ra = by_handle(raw::records(a, &class));
        let rb = by_handle(raw::records(b, &class));
        let (mut same, mut changed, mut resized, mut one_side) = (0, 0, 0, 0);
        for (h, x) in &ra {
            match rb.get(h) {
                None => one_side += 1,
                Some(y) if y.data == x.data => same += 1,
                Some(y) if y.data.len() == x.data.len() => changed += 1,
                Some(_) => resized += 1,
            }
        }
        one_side += rb.keys().filter(|h| !ra.contains_key(h)).count();
        if changed + resized + one_side > 0 {
            total_changed += 1;
            println!(
                "{:<52} {:>6} {:>6} {:>8} {:>8} {:>7} {:>7}",
                class,
                ra.len(),
                rb.len(),
                same,
                changed,
                resized,
                one_side
            );
        }
    }
    if total_changed == 0 {
        println!("(all AECC records identical)");
    }
}

/// A double is "plausible" if it is finite and not a tiny denormal-like
/// artifact of reading at the wrong offset.
fn plausible(v: f64) -> bool {
    v.is_finite() && (v == 0.0 || (1e-9..1e12).contains(&v.abs()))
}

/// Shorter decimal representations are more likely to be real field values.
fn ugliness(v: f64) -> usize {
    format!("{v}").len()
}

fn detail(a: &acadrust::document::CadDocument, b: &acadrust::document::CadDocument, class: &str) {
    let ra = by_handle(raw::records(a, class));
    let rb = by_handle(raw::records(b, class));
    for (h, x) in &ra {
        let Some(y) = rb.get(h) else {
            println!("[{h:X}] only in a ({} bytes)", x.data.len());
            continue;
        };
        let name = record_name(&x.data);
        if x.data == y.data {
            println!("[{h:X}] {name}: identical ({} bytes)", x.data.len());
            continue;
        }
        if x.data.len() != y.data.len() {
            println!(
                "[{h:X}] {name}: size {} -> {} bytes (bit-level alignment follows)",
                x.data.len(),
                y.data.len()
            );
            aligned_diff(&x.data, &y.data);
            continue;
        }
        let ranges = raw::changed_bit_ranges(&x.data, &y.data, 16);
        println!(
            "[{h:X}] {name}: {} bytes, {} changed bit range(s)",
            x.data.len(),
            ranges.len()
        );
        for (s, e) in ranges {
            // Candidate 64-bit fields covering the whole changed range.
            let lo = e.saturating_sub(64);
            let best = (lo..=s)
                .filter(|&k| k + 64 <= raw::bit_len(&x.data))
                .map(|k| (k, raw::get_f64(&x.data, k), raw::get_f64(&y.data, k)))
                .filter(|&(_, va, vb)| plausible(va) && plausible(vb) && va != vb)
                .min_by_key(|&(_, va, vb)| ugliness(va) + ugliness(vb));
            match best {
                Some((k, va, vb)) => {
                    println!("    bits {s:>7}..{e:<7}  double @ {k:>7}: {va} -> {vb}")
                }
                None => println!("    bits {s:>7}..{e:<7}  (no plausible double)"),
            }
        }
    }
    for (h, y) in &rb {
        if !ra.contains_key(h) {
            println!("[{h:X}] only in b ({} bytes)", y.data.len());
        }
    }
}

/// Best-effort name: the first string after the implementation class name.
fn record_name(data: &[u8]) -> String {
    let strs = raw::utf16_strings(data, 3);
    let mut after = false;
    for (_, s) in strs {
        if s.starts_with("AeccImp") {
            after = true;
        } else if after
            && s != "Tam"
            && !s.contains('\\')
            && !s.starts_with('<')
            && !s.contains('@')
        {
            return s;
        }
    }
    String::from("?")
}

/// Doubles in `[from, to)` that look like real field values, as
/// `(bit, value)`, skipping overlapping reads.
fn values_in(buf: &[u8], from: usize, to: usize) -> Vec<(usize, f64)> {
    let mut out: Vec<(usize, f64)> = Vec::new();
    let mut k = from.saturating_sub(63);
    while k + 64 <= raw::bit_len(buf) && k < to {
        let v = raw::get_f64(buf, k);
        if plausible(v) && v != 0.0 && ugliness(v) <= 12 {
            out.push((k, v));
            k += 64;
        } else {
            k += 1;
        }
    }
    out
}

/// Align two records of different size with a Myers diff over their bits.
/// Runs of at least 32 equal bits are treated as anchors; everything
/// between anchors is reported as a hunk with the values found on each side.
fn aligned_diff(a: &[u8], b: &[u8]) {
    let bits = |buf: &[u8]| -> Vec<u8> {
        (0..raw::bit_len(buf))
            .map(|i| (buf[i / 8] >> (7 - i % 8)) & 1)
            .collect()
    };
    let (ba, bb) = (bits(a), bits(b));
    let ops = similar::capture_diff_slices(similar::Algorithm::Myers, &ba, &bb);
    let mut anchors: Vec<(usize, usize, usize)> = Vec::new();
    for op in ops {
        if let similar::DiffOp::Equal {
            old_index,
            new_index,
            len,
        } = op
        {
            if len >= 32 {
                anchors.push((old_index, new_index, len));
            }
        }
    }
    let (mut pa, mut pb) = (0usize, 0usize);
    let mut ends = anchors.clone();
    ends.push((ba.len(), bb.len(), 0));
    let mut hunks = 0;
    for (oa, ob, len) in ends {
        if oa > pa || ob > pb {
            hunks += 1;
            let (la, lb) = (oa.saturating_sub(pa), ob.saturating_sub(pb));
            print!("    a[{pa}..{oa}] ({la} bits)  ->  b[{pb}..{ob}] ({lb} bits)");
            let va = values_in(a, pa, oa.max(pa + 1));
            let vb = values_in(b, pb, ob.max(pb + 1));
            if va.is_empty() && vb.is_empty() {
                println!();
            } else {
                println!();
                let fmt = |v: &[(usize, f64)]| {
                    v.iter()
                        .map(|(k, x)| format!("{x}@{k}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                if !va.is_empty() {
                    println!("        a values: {}", fmt(&va));
                }
                if !vb.is_empty() {
                    println!("        b values: {}", fmt(&vb));
                }
            }
        }
        pa = oa + len;
        pb = ob + len;
    }
    println!(
        "    {hunks} hunk(s); offset shift at end: {:+} bits",
        bb.len() as i64 - ba.len() as i64
    );
}
