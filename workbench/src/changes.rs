//! What changed between two saves of a drawing: the DWG records and the
//! Civil 3D API values side by side, object by object (D-080).
//!
//! The record comparison ([`crate::compare`]) says *where* bits changed;
//! the API dump comparison ([`crate::apidump`]) says *what* Civil 3D
//! reports as changed. Both are matched by handle, so one report can list,
//! per object, the record's differing regions next to the properties that
//! moved. Only differences are written: the report is meant to be the one
//! file passed on for analysis.

use crate::apidump::{self, Dump, Filter, Noise};
use crate::compare::{self, Comparison, Entry};
use crate::filemap::{Loaded, Origin};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The DXF class of a record label (`CLASS : name`).
fn label_class(label: &str) -> &str {
    label.split(" : ").next().unwrap_or(label)
}

pub struct Inputs<'a> {
    pub a: &'a Loaded,
    pub b: &'a Loaded,
    /// The API dumps of A and B, when both exist.
    pub api: Option<(&'a Dump, &'a Dump)>,
    pub filter: &'a Filter,
    pub noise: &'a Noise,
    pub show: &'a dyn Fn(Origin) -> bool,
    /// One line per object instead of the bit regions and properties.
    pub brief: bool,
}

/// The combined report.
pub fn report(i: &Inputs) -> String {
    let c: Comparison = compare::compare(i.a, i.b);
    let mut out = String::new();
    let _ = writeln!(out, "Changes between two drawings (civil3drust workbench)");
    let _ = writeln!(out, "A: {}", c.a_title);
    let _ = writeln!(out, "B: {}", c.b_title);
    match i.api {
        Some((x, y)) => {
            let _ = writeln!(out, "A API dump: {}", x.path);
            let _ = writeln!(out, "B API dump: {}", y.path);
        }
        None => {
            let _ = writeln!(
                out,
                "API dumps: not found for both drawings; records only \
                 (C3DUMP, or `<drawing>.api.txt` / `c3dump/` next to each drawing)"
            );
        }
    }
    if !i.filter.is_empty() {
        let _ = writeln!(out, "filter: {}", i.filter.describe());
    }
    if i.noise.rules() > 0 {
        let _ = writeln!(out, "noise list: {} rule(s)", i.noise.rules());
    }
    if let Some(n) = &c.note {
        let _ = writeln!(out, "{n}");
        return out;
    }
    let (pa, pb) = (&i.a.panes[c.a_pane.unwrap()], &i.b.panes[c.b_pane.unwrap()]);

    // Records: kept, or left out by kind, filter or noise list.
    let mut records: BTreeMap<u64, &Entry> = BTreeMap::new();
    let (mut by_kind, mut by_filter, mut by_noise) = (0, 0, Vec::new());
    for e in &c.entries {
        if i.noise.record(e.handle, &e.label) {
            by_noise.push(e.handle);
        } else if !i.filter.passes(e.handle, label_class(&e.label), &e.label) {
            by_filter += 1;
        } else if !(i.show)(e.origin) {
            by_kind += 1;
        } else {
            records.insert(e.handle, e);
        }
    }
    let _ = writeln!(
        out,
        "DWG records (matched by handle): A {}, B {}; identical {}; differing {}",
        pa.spans.len(),
        pb.spans.len(),
        c.same,
        c.entries.len()
    );
    if by_kind + by_filter > 0 {
        let _ = writeln!(
            out,
            "  not shown: {by_kind} by kind (--show), {by_filter} by the filter"
        );
    }
    if !by_noise.is_empty() {
        let list: Vec<String> = by_noise.iter().take(40).map(|h| format!("{h:X}")).collect();
        let _ = writeln!(
            out,
            "  dropped by the noise list: {} ({}{})",
            by_noise.len(),
            list.join(" "),
            if by_noise.len() > 40 { " …" } else { "" }
        );
    }

    let api = i
        .api
        .map(|(x, y)| (apidump::diff(x, y, i.filter, i.noise), x, y));
    let mut api_changes: BTreeMap<u64, &apidump::Change> = BTreeMap::new();
    if let Some((d, x, y)) = &api {
        let _ = writeln!(out, "{}", apidump::summary(d, x, y));
        api_changes = d.changes.iter().map(|c| (c.handle, c)).collect();
    }
    let _ = writeln!(
        out,
        "Bit offsets are relative to the record data start; bit 0 is the MSB of the first data byte."
    );

    let mut handles: Vec<u64> = records.keys().chain(api_changes.keys()).copied().collect();
    handles.sort_unstable();
    handles.dedup();
    if i.brief {
        let _ = writeln!(
            out,
            "\n{} object(s); --handle <h,h> without --brief for the detail:",
            handles.len()
        );
        for h in handles {
            let rec = match records.get(&h) {
                Some(e) => compare::entry_brief(pa, pb, e),
                None => format!(
                    "record same #{h:X} {}",
                    api_changes.get(&h).map_or("", |c| c.class.as_str())
                ),
            };
            let api = match api_changes.get(&h) {
                Some(c) => format!(
                    "; API {}, {} line(s)",
                    apidump::kind_text(c.kind),
                    c.lines.len()
                ),
                None => String::new(),
            };
            let _ = writeln!(out, "{rec}{api}");
        }
        return out;
    }
    for h in handles {
        match records.get(&h) {
            Some(e) => compare::entry_text(&mut out, pa, pb, e),
            None => {
                let title = api_changes.get(&h).map_or("", |c| c.title.as_str());
                let _ = writeln!(out, "\n== record identical or not shown: {title}");
            }
        }
        match (api_changes.get(&h), &api) {
            (Some(c), _) => {
                let _ = writeln!(out, "   API ({}): {}", apidump::kind_text(c.kind), c.title);
                apidump::write_lines(&mut out, c, "     ");
            }
            (None, Some((_, x, y))) => {
                let known = x.objects.contains_key(&h) || y.objects.contains_key(&h);
                let _ = writeln!(
                    out,
                    "   API: {}",
                    if known {
                        "no change reported"
                    } else {
                        "object not in the dumps"
                    }
                );
            }
            (None, None) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// S336 (written by civil3drust) and S349 (Civil 3D's re-save of it),
    /// each with its API dump in `samples/`.
    #[test]
    fn records_and_api_side_by_side() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples");
        let (pa, pb) = (
            dir.join("S336_S331_exp_rename.dwg"),
            dir.join("S349_S336_resaved.dwg"),
        );
        let (a, b) = (
            crate::filemap::load(&pa).unwrap(),
            crate::filemap::load(&pb).unwrap(),
        );
        let (da, db) = (
            apidump::load(apidump::find_for(&pa).unwrap()).unwrap(),
            apidump::load(apidump::find_for(&pb).unwrap()).unwrap(),
        );
        let run = |filter: &Filter, noise: &Noise, brief: bool| {
            report(&Inputs {
                a: &a,
                b: &b,
                api: Some((&da, &db)),
                filter,
                noise,
                show: &|_| true,
                brief,
            })
        };
        // The site record: Civil 3D gave it a new key on its save, the API
        // reports the same site.
        let site = Filter::new("", "8D58", "").unwrap();
        let text = run(&site, &Noise::default(), false);
        assert!(text.contains("== changed: #8D58 AECC_PARCEL"), "{text}");
        assert!(text.contains("   API: no change reported"), "{text}");
        assert_eq!(text.matches("\n== ").count(), 1, "{text}");
        // The API change of this pair; its record did not change.
        let group = Filter::new("AECC_GRADING_GROUP", "", "").unwrap();
        let text = run(&group, &Noise::default(), false);
        assert!(
            text.contains("== record identical or not shown: [A1C]"),
            "{text}"
        );
        assert!(text.contains("~ GetReactors(): [2]"), "{text}");
        let noise = Noise::parse("prop AECC_GRADING_GROUP GetReactors()\n").unwrap();
        let text = run(&group, &noise, false);
        assert!(!text.contains("\n== "), "{text}");
        assert!(
            text.contains("noise list dropped 1 property change"),
            "{text}"
        );
        // A record on the noise list is counted, not shown.
        let noise = Noise::parse("record #8D58  # the site\n").unwrap();
        let text = run(&site, &noise, false);
        assert!(!text.contains("\n== "), "{text}");
        assert!(
            text.contains("dropped by the noise list: 1 (8D58)"),
            "{text}"
        );
        // One line per object.
        let brief = run(
            &Filter::new("AECC_PARCEL", "", "").unwrap(),
            &Noise::default(),
            true,
        );
        assert!(brief.contains("changed    #8D58 AECC_PARCEL"), "{brief}");
        assert!(!brief.contains("A bits"), "{brief}");
    }
}
