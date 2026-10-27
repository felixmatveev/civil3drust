//! Comparing the object records of two drawings.
//!
//! Both drawings are loaded the usual way ([`crate::model::load`]); the
//! records of their `AcDb:AcDbObjects` panes are matched by handle. A
//! record is *same* when its data bytes are identical, *changed* when they
//! differ, or present in only one drawing. For a changed record the report
//! gives:
//!
//! - data and handle-stream sizes on both sides;
//! - where the bits differ: the common prefix and suffix, and the differing
//!   window between them on each side (DWG data is a bit stream, so a field
//!   that grows shifts everything after it — prefix/suffix finds the edit,
//!   a byte-by-byte diff would not). When both records have the same
//!   length, the differing runs inside the window are listed one by one;
//! - which decoded fields changed (civil3drust's fields for AECC records:
//!   strings, elements, handle references, …).
//!
//! The report is plain text meant to be pasted into a conversation.

use crate::model::{Field, Loaded, Origin, Pane, Span};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Bits shown per differing window or run before cutting it short.
pub const MAX_BITS_SHOWN: usize = 512;
/// Differing regions listed per record.
const MAX_EDITS: usize = 24;
/// Shortest run of equal bits used to split a differing window.
const ANCHOR: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Changed,
    OnlyA,
    OnlyB,
}

/// One record that differs between the drawings.
#[derive(Debug, Clone)]
pub struct Entry {
    pub kind: Kind,
    pub handle: u64,
    pub origin: Origin,
    pub label: String,
    /// Index of the record in A's / B's objects pane.
    pub a: Option<usize>,
    pub b: Option<usize>,
}

pub struct Comparison {
    pub a_title: String,
    pub b_title: String,
    pub a_pane: Option<usize>,
    pub b_pane: Option<usize>,
    pub same: usize,
    pub entries: Vec<Entry>,
    pub note: Option<String>,
}

fn objects_pane(l: &Loaded) -> Option<usize> {
    l.panes.iter().position(|p| p.title == "AcDb:AcDbObjects")
}

fn data<'a>(p: &'a Pane, s: &Span) -> &'a [u8] {
    let end = (s.data_start + s.data_len).min(p.bytes.len());
    &p.bytes[s.data_start.min(end)..end]
}

/// Match the records of `a` and `b` by handle.
pub fn compare(a: &Loaded, b: &Loaded) -> Comparison {
    let mut c = Comparison {
        a_title: format!("{} [{}]", a.path, a.version),
        b_title: format!("{} [{}]", b.path, b.version),
        a_pane: objects_pane(a),
        b_pane: objects_pane(b),
        same: 0,
        entries: Vec::new(),
        note: None,
    };
    let (Some(pa), Some(pb)) = (c.a_pane, c.b_pane) else {
        c.note =
            Some("No record overlay in one of the files (not a DWG, or mapping failed).".into());
        return c;
    };
    let (pa, pb) = (&a.panes[pa], &b.panes[pb]);
    let mut by_handle: BTreeMap<u64, (Option<usize>, Option<usize>)> = BTreeMap::new();
    for (i, s) in pa.spans.iter().enumerate() {
        by_handle.entry(s.handle).or_default().0 = Some(i);
    }
    for (i, s) in pb.spans.iter().enumerate() {
        by_handle.entry(s.handle).or_default().1 = Some(i);
    }
    for (handle, (ia, ib)) in by_handle {
        let kind = match (ia, ib) {
            (Some(x), Some(y)) => {
                let (sa, sb) = (&pa.spans[x], &pb.spans[y]);
                if data(pa, sa) == data(pb, sb) && sa.handle_bits == sb.handle_bits {
                    c.same += 1;
                    continue;
                }
                Kind::Changed
            }
            (Some(_), None) => Kind::OnlyA,
            (None, Some(_)) => Kind::OnlyB,
            (None, None) => continue,
        };
        let s = ia
            .map(|i| &pa.spans[i])
            .or(ib.map(|i| &pb.spans[i]))
            .unwrap();
        c.entries.push(Entry {
            kind,
            handle,
            origin: s.origin,
            label: s.label(),
            a: ia,
            b: ib,
        });
    }
    c
}

fn bits(d: &[u8]) -> Vec<u8> {
    (0..d.len() * 8)
        .map(|i| (d[i / 8] >> (7 - i % 8)) & 1)
        .collect()
}

fn bit_text(b: &[u8]) -> String {
    let mut s: String = b
        .iter()
        .take(MAX_BITS_SHOWN)
        .map(|&x| if x == 1 { '1' } else { '0' })
        .collect();
    if b.len() > MAX_BITS_SHOWN {
        let _ = write!(s, "… (+{} bits)", b.len() - MAX_BITS_SHOWN);
    }
    s
}

/// One differing region: bit ranges in A and in B.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub a: std::ops::Range<usize>,
    pub b: std::ops::Range<usize>,
}

/// Differing regions of two bit strings. The common prefix and suffix are
/// removed; inside the remaining window the longest common run of at least
/// [`ANCHOR`] bits splits it in two, recursively. A field that grows (an
/// inserted string, a longer bit-coded value) shows as an insertion and the
/// shifted bits after it line up again, which a position-by-position diff
/// cannot do.
pub fn diff_bits(a: &[u8], b: &[u8]) -> Vec<Edit> {
    let mut out = Vec::new();
    split(a, b, 0, 0, 0, &mut out);
    out
}

fn split(a: &[u8], b: &[u8], oa: usize, ob: usize, depth: usize, out: &mut Vec<Edit>) {
    let p = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let max_s = a.len().min(b.len()) - p;
    let s = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(max_s)
        .take_while(|(x, y)| x == y)
        .count();
    let (a, b) = (&a[p..a.len() - s], &b[p..b.len() - s]);
    let (oa, ob) = (oa + p, ob + p);
    if a.is_empty() && b.is_empty() {
        return;
    }
    let whole = || Edit {
        a: oa..oa + a.len(),
        b: ob..ob + b.len(),
    };
    if a.len() < ANCHOR || b.len() < ANCHOR || depth > 40 {
        out.push(whole());
        return;
    }
    match longest_common(a, b) {
        Some((i, j, n)) if n >= ANCHOR => {
            split(&a[..i], &b[..j], oa, ob, depth + 1, out);
            split(
                &a[i + n..],
                &b[j + n..],
                oa + i + n,
                ob + j + n,
                depth + 1,
                out,
            );
        }
        _ => out.push(whole()),
    }
}

/// Longest common run (start in a, start in b, length) found through
/// 64-bit keys of b at every offset and of a every 16 bits.
fn longest_common(a: &[u8], b: &[u8]) -> Option<(usize, usize, usize)> {
    let key = |s: &[u8], i: usize| {
        s[i..i + ANCHOR]
            .iter()
            .fold(0u64, |k, &x| k << 1 | x as u64)
    };
    let mut index: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
    let mut k = 0u64;
    for (j, &x) in b.iter().enumerate() {
        k = k << 1 | x as u64;
        if j + 1 >= ANCHOR {
            let v = index.entry(k).or_default();
            if v.len() < 16 {
                v.push(j + 1 - ANCHOR);
            }
        }
    }
    let mut best: Option<(usize, usize, usize)> = None;
    let mut i = 0;
    while i + ANCHOR <= a.len() {
        if let Some(js) = index.get(&key(a, i)) {
            for &j in js {
                // extend backwards and forwards
                let mut s = 0;
                while i > s && j > s && a[i - s - 1] == b[j - s - 1] {
                    s += 1;
                }
                let (si, sj) = (i - s, j - s);
                let mut n = 0;
                while si + n < a.len() && sj + n < b.len() && a[si + n] == b[sj + n] {
                    n += 1;
                }
                if best.is_none_or(|(_, _, m)| n > m) {
                    best = Some((si, sj, n));
                }
            }
        }
        i += 16;
    }
    best
}

/// The decoded field of A that starts closest before `bit`, as
/// `, after <label> (bit n)`; empty when none is known.
fn near(fields: &[Field], bit: usize) -> String {
    fields
        .iter()
        .filter_map(|f| f.bit.filter(|&b| b <= bit).map(|b| (b, f)))
        .max_by_key(|(b, _)| *b)
        .map(|(b, f)| format!(", after {} at bit {b}", f.label))
        .unwrap_or_default()
}

/// Fields keyed by label (later duplicates get `#2`, `#3`, …).
fn field_map(fields: &[Field]) -> BTreeMap<String, (String, Option<usize>)> {
    let mut m = BTreeMap::new();
    for f in fields {
        let mut k = f.label.clone();
        let mut n = 2;
        while m.contains_key(&k) {
            k = format!("{} #{n}", f.label);
            n += 1;
        }
        m.insert(k, (f.value.clone(), f.bit));
    }
    m
}

/// The bit-level and field-level description of one changed record.
fn describe_change(out: &mut String, pa: &Pane, sa: &Span, pb: &Pane, sb: &Span) {
    let (da, db) = (data(pa, sa), data(pb, sb));
    let _ = writeln!(
        out,
        "   data: A {} B {} bytes; handle stream: A {} B {} bits; frame: A 0x{:X} B 0x{:X}",
        da.len(),
        db.len(),
        sa.handle_bits,
        sb.handle_bits,
        sa.start,
        sb.start
    );
    let (ba, bb) = (bits(da), bits(db));
    // The data stream and the handle stream are compared separately: the
    // handle stream sits at the end, so a change there would otherwise hide
    // the common suffix of the data.
    let ea = ba.len() - sa.handle_bits.min(ba.len());
    let eb = bb.len() - sb.handle_bits.min(bb.len());
    let edits = diff_bits(&ba[..ea], &bb[..eb]);
    if edits.is_empty() {
        let _ = writeln!(out, "   data stream: identical ({ea} bits)");
    } else {
        let delta = eb as i64 - ea as i64;
        let _ = writeln!(
            out,
            "   data stream: A {ea} B {eb} bits ({delta:+}); {} differing region(s)",
            edits.len()
        );
        for (k, e) in edits.iter().take(MAX_EDITS).enumerate() {
            let (wa, wb) = (&ba[e.a.clone()], &bb[e.b.clone()]);
            let what = if wa.len() == wb.len() {
                let n = wa.iter().zip(wb).filter(|(x, y)| x != y).count();
                format!("{n} of {} bits differ", wa.len())
            } else if wa.is_empty() {
                format!("{} bits inserted", wb.len())
            } else if wb.is_empty() {
                format!("{} bits removed", wa.len())
            } else {
                format!("{} bits replaced by {}", wa.len(), wb.len())
            };
            let _ = writeln!(
                out,
                "   [{}] A bits {}..{}  B bits {}..{}  ({what}){}",
                k + 1,
                e.a.start,
                e.a.end,
                e.b.start,
                e.b.end,
                near(&sa.fields, e.a.start)
            );
            let _ = writeln!(out, "       A: {}", bit_text(wa));
            let _ = writeln!(out, "       B: {}", bit_text(wb));
        }
        if edits.len() > MAX_EDITS {
            let _ = writeln!(out, "   … {} more regions", edits.len() - MAX_EDITS);
        }
    }
    let (ha, hb) = (&ba[ea..], &bb[eb..]);
    if ha != hb {
        let _ = writeln!(out, "   handle stream (A bit {ea}, B bit {eb}):");
        let _ = writeln!(out, "       A: {}", bit_text(ha));
        let _ = writeln!(out, "       B: {}", bit_text(hb));
    }
    // Decoded fields.
    let (fa, fb) = (field_map(&sa.fields), field_map(&sb.fields));
    let at = |b: &Option<usize>| b.map(|b| format!(" (bit {b})")).unwrap_or_default();
    // Only values are compared: a field that merely moved (its bit offset
    // shifted by an earlier change) is not listed.
    let mut lines = Vec::new();
    let mut moved = 0;
    for (k, (va, ba)) in &fa {
        match fb.get(k) {
            Some((vb, bb)) if vb == va => {
                if ba != bb {
                    moved += 1;
                }
            }
            Some((vb, bb)) => lines.push(format!("     ~ {k}: {va}{}  →  {vb}{}", at(ba), at(bb))),
            None => lines.push(format!("     - {k}: {va}{}", at(ba))),
        }
    }
    for (k, (vb, bb)) in &fb {
        if !fa.contains_key(k) {
            lines.push(format!("     + {k}: {vb}{}", at(bb)));
        }
    }
    if moved > 0 {
        lines.push(format!(
            "     ({moved} unchanged fields at shifted bit offsets not listed)"
        ));
    }
    if sa.name != sb.name {
        lines.insert(0, format!("     ~ name: {:?}  →  {:?}", sa.name, sb.name));
    }
    if !lines.is_empty() {
        let _ = writeln!(out, "   decoded fields (A → B):");
        for l in lines {
            let _ = writeln!(out, "{l}");
        }
    }
}

fn origin_name(o: Origin) -> &'static str {
    match o {
        Origin::Acad => "acadrust",
        Origin::Civil => "civil3drust",
        Origin::Opaque => "opaque",
        Origin::Unmapped => "unmapped",
    }
}

/// Whether an entry passes the text filter: a case-insensitive substring of
/// its label (class and name) or its hex handle; empty passes everything.
pub fn matches(e: &Entry, filter: &str) -> bool {
    let f = filter.trim().to_ascii_lowercase();
    f.is_empty()
        || e.label.to_ascii_lowercase().contains(&f)
        || format!("{:x}", e.handle) == f.trim_start_matches('#')
}

/// The text report for the entries whose origin passes `show` and whose
/// label or handle matches `filter`.
pub fn report(
    c: &Comparison,
    a: &Loaded,
    b: &Loaded,
    show: &dyn Fn(Origin) -> bool,
    filter: &str,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "DWG record comparison (civil3drust hexview)");
    let _ = writeln!(out, "A: {}", c.a_title);
    let _ = writeln!(out, "B: {}", c.b_title);
    if let Some(n) = &c.note {
        let _ = writeln!(out, "{n}");
        return out;
    }
    let (pa, pb) = (&a.panes[c.a_pane.unwrap()], &b.panes[c.b_pane.unwrap()]);
    let count = |k: Kind| c.entries.iter().filter(|e| e.kind == k).count();
    let _ = writeln!(
        out,
        "records (objects section, matched by handle): A {}, B {}; identical {}; changed {}; only in A {}; only in B {}",
        pa.spans.len(),
        pb.spans.len(),
        c.same,
        count(Kind::Changed),
        count(Kind::OnlyA),
        count(Kind::OnlyB)
    );
    let mut per: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for e in &c.entries {
        let k = match e.kind {
            Kind::Changed => 0,
            Kind::OnlyA => 1,
            Kind::OnlyB => 2,
        };
        per.entry(origin_name(e.origin)).or_default()[k] += 1;
    }
    for (o, [ch, oa, ob]) in &per {
        let _ = writeln!(out, "  {o}: changed {ch}, only A {oa}, only B {ob}");
    }
    let shown: Vec<&Entry> = c
        .entries
        .iter()
        .filter(|e| show(e.origin) && matches(e, filter))
        .collect();
    let hidden = c.entries.len() - shown.len();
    if !filter.trim().is_empty() {
        let _ = writeln!(out, "filter: \"{}\"", filter.trim());
    }
    if hidden > 0 {
        let _ = writeln!(
            out,
            "({hidden} differing records hidden by the filter or kind)"
        );
    }
    let _ = writeln!(
        out,
        "Bit offsets are relative to the record data start (as in civil3drust); bit 0 is the MSB of the first data byte."
    );
    for e in shown {
        let what = match e.kind {
            Kind::Changed => "changed",
            Kind::OnlyA => "only in A",
            Kind::OnlyB => "only in B",
        };
        let _ = writeln!(
            out,
            "\n== {what}: #{:X} {} ({})",
            e.handle,
            e.label,
            origin_name(e.origin)
        );
        match (e.a, e.b) {
            (Some(x), Some(y)) => {
                let (sa, sb) = (&pa.spans[x], &pb.spans[y]);
                if sa.label() != sb.label() {
                    let _ = writeln!(out, "   B label: {}", sb.label());
                }
                describe_change(&mut out, pa, sa, pb, sb);
            }
            (Some(x), None) | (None, Some(x)) => {
                let (p, s) = if e.a.is_some() {
                    (pa, &pa.spans[x])
                } else {
                    (pb, &pb.spans[x])
                };
                let d = data(p, s);
                let _ = writeln!(
                    out,
                    "   data {} bytes, handle stream {} bits",
                    d.len(),
                    s.handle_bits
                );
                for f in s.fields.iter().take(60) {
                    let _ = writeln!(out, "     {}: {}", f.label, f.value);
                }
            }
            (None, None) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pseudo(n: usize, seed: u64) -> Vec<u8> {
        let mut x = seed;
        (0..n)
            .map(|_| {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (x >> 63) as u8
            })
            .collect()
    }

    #[test]
    fn insertion_realigns() {
        let a = pseudo(2000, 7);
        let mut b = a.clone();
        let ins = pseudo(40, 99);
        b.splice(700..700, ins.iter().copied());
        b[1500] ^= 1; // a later in-place change (bit 1460 in A)
        let e = diff_bits(&a, &b);
        assert_eq!(e.len(), 2, "{e:?}");
        assert_eq!(e[0].a.len(), 0);
        assert_eq!(e[0].b.len(), 40);
        assert!(e[0].a.start.abs_diff(700) <= 2);
        assert_eq!(e[1].a, 1460..1461);
        assert_eq!(e[1].b, 1500..1501);
    }

    #[test]
    fn identical_has_no_edits() {
        let a = pseudo(500, 3);
        assert!(diff_bits(&a, &a).is_empty());
    }

    #[test]
    fn bit_order_is_msb_first() {
        assert_eq!(bits(&[0b1000_0001]), vec![1, 0, 0, 0, 0, 0, 0, 1]);
    }
}
