//! Civil 3D API dumps (`C3DUMP`, `tools/c3d_api_dump`): read, filter,
//! compare (D-080).
//!
//! A dump is the add-in's plain text: a few header lines, then one entry
//! per object — `[HANDLE] …` followed by indented `name: value` lines.
//! Here it is parsed into objects keyed by handle, so that
//!
//! - a filtered part can be written out again in the same format;
//! - two dumps can be compared property by property (only the changes are
//!   reported, which is what gets passed on for analysis);
//! - a noise list can drop what changes on every save.
//!
//! Dumps next to a drawing: the manual one is `<drawing>.api.txt`, the
//! ones written on save are `c3dump/<drawing>.<yyyyMMdd-HHmmss>.api.txt`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// One dumped object.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub handle: u64,
    /// DXF class name (`AECC_ALIGNMENT`, `LAYER`, …).
    pub class: String,
    /// The entry's first line without indentation.
    pub title: String,
    /// The object it was listed under (a profile under its alignment).
    pub parent: Option<u64>,
    /// Properties in dump order; collection items are flattened to
    /// `PVIs[0].Station`.
    pub props: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Dump {
    pub path: String,
    /// `drawing: …`, `written: …`, `config: …`, …
    pub header: Vec<String>,
    pub objects: BTreeMap<u64, Object>,
}

/// `[HEX]` at the start of `s`: the handle and the rest.
fn bracket_handle(s: &str) -> Option<(u64, &str)> {
    let rest = s.strip_prefix('[')?;
    let end = rest.find(']')?;
    let hex = &rest[..end];
    if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let after = &rest[end + 1..];
    // `GetPoints() [3]: …` is a property, not an entry.
    if after.starts_with(':') {
        return None;
    }
    Some((u64::from_str_radix(hex, 16).ok()?, after))
}

/// An entry line: `[8D6A] …` or `Profile [8D72] …`.
fn entry_line(t: &str) -> Option<(Option<&str>, u64, &str)> {
    if let Some((h, rest)) = bracket_handle(t) {
        return Some((None, h, rest.trim()));
    }
    let (word, rest) = t.split_once(' ')?;
    if word.is_empty() || !word.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let (h, rest) = bracket_handle(rest)?;
    Some((Some(word), h, rest.trim()))
}

/// Split `name: value` at the first `: ` outside brackets (an error text
/// such as `<EntityNotFoundException: …>` inside a name has its own).
fn split_prop(t: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let b = t.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'<' => depth += 1,
            b')' | b'>' => depth = (depth - 1).max(0),
            b':' if depth == 0 => {
                if i + 1 == b.len() {
                    return Some((&t[..i], ""));
                }
                if b[i + 1] == b' ' {
                    return Some((&t[..i], &t[i + 2..]));
                }
            }
            _ => {}
        }
    }
    t.split_once(": ")
}

/// Keys whose text carries a value: `GetIds() [3]` and
/// `Entity 1 (before 0, after 2)` become `GetIds()` and `Entity 1`, the
/// rest moves into the value.
fn normalise(key: &str, value: &str) -> (String, String) {
    if let Some(open) = key.rfind(" [") {
        let n = &key[open + 2..];
        if let Some(n) = n.strip_suffix(']') {
            if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) {
                return (
                    key[..open].to_string(),
                    format!("[{n}] {value}").trim_end().to_string(),
                );
            }
        }
    }
    if key.starts_with("Entity ") {
        if let Some(open) = key.find(" (") {
            return (
                key[..open].to_string(),
                format!("{}: {value}", &key[open + 1..]),
            );
        }
    }
    (key.to_string(), value.to_string())
}

pub fn parse(text: &str) -> Dump {
    let mut d = Dump::default();
    let mut in_header = true;
    let mut generic = false;
    // The entry being read, its indentation, and the open collection items
    // as (indentation, key prefix).
    let mut cur: Option<(Object, usize)> = None;
    let mut top: Option<u64> = None;
    let mut items: Vec<(usize, String)> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    fn finish(d: &mut Dump, cur: &mut Option<(Object, usize)>) {
        if let Some((o, _)) = cur.take() {
            d.objects.entry(o.handle).or_insert(o);
        }
    }
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            in_header = false;
            continue;
        }
        if in_header {
            d.header.push(t.to_string());
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if let Some((label, handle, rest)) = entry_line(t) {
            finish(&mut d, &mut cur);
            items.clear();
            seen.clear();
            let class = match label {
                Some("Profile") => "AECC_PROFILE".to_string(),
                Some("ProfileView") => "AECC_PROFILE_VIEW".to_string(),
                Some(l) => l.to_string(),
                None if !generic => "AECC_ALIGNMENT".to_string(),
                None => rest.split_whitespace().next().unwrap_or("").to_string(),
            };
            let parent = if indent > 0 { top } else { None };
            if indent == 0 {
                top = Some(handle);
            }
            cur = Some((
                Object {
                    handle,
                    class,
                    title: t.to_string(),
                    parent,
                    props: Vec::new(),
                },
                indent,
            ));
            continue;
        }
        if indent == 0 {
            // `== Objects`, `objects: 12`.
            finish(&mut d, &mut cur);
            top = None;
            if t.starts_with("== ") {
                generic = true;
            } else {
                d.header.push(t.to_string());
            }
            continue;
        }
        let Some((o, _)) = &mut cur else { continue };
        while items.last().is_some_and(|(i, _)| *i >= indent) {
            items.pop();
        }
        let prefix: String = items.iter().map(|(_, p)| p.as_str()).collect();
        match split_prop(t) {
            Some((k, v)) => {
                let (k, v) = normalise(k, v);
                let mut key = format!("{prefix}{k}");
                let n = seen.entry(key.clone()).or_insert(0);
                *n += 1;
                if *n > 1 {
                    key = format!("{key} #{n}");
                }
                o.props.push((key, v));
            }
            // `PVIs[0] ProfilePVI`: its properties follow, deeper.
            None => {
                let name = t.split_whitespace().next().unwrap_or(t);
                items.push((indent, format!("{name}.")));
            }
        }
    }
    finish(&mut d, &mut cur);
    d
}

pub fn load(path: impl AsRef<Path>) -> Result<Dump, String> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut d = parse(&String::from_utf8_lossy(&bytes));
    d.path = path.display().to_string();
    if d.objects.is_empty() {
        return Err(format!("{}: no objects (not a C3DUMP file?)", d.path));
    }
    Ok(d)
}

/// Case-insensitive match with `*` wildcards.
pub fn glob(pattern: &str, s: &str) -> bool {
    fn go(p: &[u8], s: &[u8]) -> bool {
        match p.split_first() {
            None => s.is_empty(),
            Some((b'*', rest)) => (0..=s.len()).any(|i| go(rest, &s[i..])),
            Some((c, rest)) => s
                .split_first()
                .is_some_and(|(d, s)| c.eq_ignore_ascii_case(d) && go(rest, s)),
        }
    }
    go(pattern.as_bytes(), s.as_bytes())
}

/// Which objects to keep. Empty parts pass everything.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// DXF class patterns (`*` wildcard).
    pub classes: Vec<String>,
    pub handles: Vec<u64>,
    /// Case-insensitive substring of the entry's first line.
    pub text: String,
}

impl Filter {
    pub fn new(classes: &str, handles: &str, text: &str) -> Result<Filter, String> {
        let list = |s: &str| -> Vec<String> {
            s.split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect()
        };
        let handles = list(handles)
            .iter()
            .map(|h| {
                u64::from_str_radix(h.trim_start_matches('#'), 16)
                    .map_err(|_| format!("not a handle: {h}"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Filter {
            classes: list(classes),
            handles,
            text: text.trim().to_ascii_lowercase(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty() && self.handles.is_empty() && self.text.is_empty()
    }

    pub fn class_passes(&self, class: &str) -> bool {
        self.classes.is_empty() || self.classes.iter().any(|p| glob(p, class))
    }

    pub fn passes(&self, handle: u64, class: &str, title: &str) -> bool {
        self.class_passes(class)
            && (self.handles.is_empty() || self.handles.contains(&handle))
            && (self.text.is_empty() || title.to_ascii_lowercase().contains(&self.text))
    }

    fn object(&self, o: &Object) -> bool {
        self.passes(o.handle, &o.class, &o.title)
    }

    pub fn describe(&self) -> String {
        if self.is_empty() {
            return "none".into();
        }
        let mut parts = Vec::new();
        if !self.classes.is_empty() {
            parts.push(format!("classes {}", self.classes.join(",")));
        }
        if !self.handles.is_empty() {
            let h: Vec<String> = self.handles.iter().map(|h| format!("{h:X}")).collect();
            parts.push(format!("handles {}", h.join(",")));
        }
        if !self.text.is_empty() {
            parts.push(format!("text \"{}\"", self.text));
        }
        parts.join("; ")
    }
}

/// What changes on every save and is left out of comparisons. One rule
/// per line; `#` starts a comment:
///
/// ```text
/// prop <class pattern> <property pattern>     an API property
/// record <#HANDLE | label pattern>            a DWG record
/// ```
#[derive(Debug, Clone, Default)]
pub struct Noise {
    pub props: Vec<(String, String)>,
    pub record_handles: BTreeSet<u64>,
    pub record_labels: Vec<String>,
}

impl Noise {
    pub fn parse(text: &str) -> Result<Noise, String> {
        let mut n = Noise::default();
        for (i, line) in text.lines().enumerate() {
            // A comment is a line starting with `#`, or ` # ` to the end.
            let line = line.split(" # ").next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (kind, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            let rest = rest.trim();
            let bad = || format!("noise list line {}: {line}", i + 1);
            match kind {
                "prop" => {
                    let (c, k) = rest.split_once(char::is_whitespace).ok_or_else(bad)?;
                    n.props.push((c.to_string(), k.trim().to_string()));
                }
                "record" if rest.starts_with('#') => {
                    n.record_handles
                        .insert(u64::from_str_radix(&rest[1..], 16).map_err(|_| bad())?);
                }
                "record" if !rest.is_empty() => n.record_labels.push(rest.to_string()),
                _ => return Err(bad()),
            }
        }
        Ok(n)
    }

    pub fn load(path: &str) -> Result<Noise, String> {
        Noise::parse(&std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?)
    }

    pub fn rules(&self) -> usize {
        self.props.len() + self.record_handles.len() + self.record_labels.len()
    }

    pub fn prop(&self, class: &str, key: &str) -> bool {
        self.props
            .iter()
            .any(|(c, k)| glob(c, class) && glob(k, key))
    }

    pub fn record(&self, handle: u64, label: &str) -> bool {
        self.record_handles.contains(&handle) || self.record_labels.iter().any(|p| glob(p, label))
    }
}

fn write_object(out: &mut String, o: &Object) {
    let _ = writeln!(out, "\n{}", o.title);
    for (k, v) in &o.props {
        let _ = writeln!(out, "  {k}: {v}");
    }
}

/// The objects passing `filter`, in the dump's own format (it can be read
/// back and compared), in handle order.
pub fn export(d: &Dump, filter: &Filter) -> String {
    let mut out = String::new();
    for h in d.header.iter().filter(|h| !h.starts_with("objects:")) {
        let _ = writeln!(out, "{h}");
    }
    let _ = writeln!(out, "source: {}", d.path);
    let _ = writeln!(out, "filter: {}", filter.describe());
    let _ = writeln!(out, "\n== Objects");
    let mut n = 0;
    for o in d.objects.values().filter(|o| filter.object(o)) {
        // The class is spelled out for entries the add-in writes without
        // one (alignments, profiles), so that it survives a round trip.
        let mut o = o.clone();
        let rest = entry_line(&o.title).map_or("", |(_, _, r)| r);
        if rest.split_whitespace().next() != Some(o.class.as_str()) {
            o.title = format!("[{:X}] {} {rest}", o.handle, o.class);
        }
        write_object(&mut out, &o);
        n += 1;
    }
    let _ = writeln!(out, "\nobjects: {n} of {}", d.objects.len());
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Changed,
    OnlyA,
    OnlyB,
}

/// One object that differs between two dumps.
#[derive(Debug, Clone)]
pub struct Change {
    pub kind: Kind,
    pub handle: u64,
    pub class: String,
    pub title: String,
    /// `~ name: a  →  b`, `- name: a`, `+ name: b`.
    pub lines: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Diff {
    pub changes: Vec<Change>,
    pub same: usize,
    /// Objects left out by the filter.
    pub filtered: usize,
    /// Property changes dropped by the noise list, and the objects whose
    /// only changes they were.
    pub noise_props: usize,
    pub noise_objects: usize,
}

/// Compare two dumps object by object (matched by handle).
pub fn diff(a: &Dump, b: &Dump, filter: &Filter, noise: &Noise) -> Diff {
    let mut d = Diff::default();
    let handles: BTreeSet<u64> = a.objects.keys().chain(b.objects.keys()).copied().collect();
    for h in handles {
        let (oa, ob) = (a.objects.get(&h), b.objects.get(&h));
        let any = oa.or(ob).unwrap();
        if !oa.is_some_and(|o| filter.object(o)) && !ob.is_some_and(|o| filter.object(o)) {
            d.filtered += 1;
            continue;
        }
        let mut c = Change {
            kind: Kind::Changed,
            handle: h,
            class: any.class.clone(),
            title: any.title.clone(),
            lines: Vec::new(),
        };
        match (oa, ob) {
            (Some(x), Some(y)) => {
                let mut hidden = 0;
                if x.title != y.title {
                    c.lines
                        .push(format!("~ entry: {}  →  {}", x.title, y.title));
                    c.title = y.title.clone();
                }
                let (ma, mb): (BTreeMap<_, _>, BTreeMap<_, _>) = (
                    x.props.iter().map(|(k, v)| (k, v)).collect(),
                    y.props.iter().map(|(k, v)| (k, v)).collect(),
                );
                // A's order, then what only B has.
                for (k, va) in &x.props {
                    let line = match mb.get(k) {
                        Some(vb) if *vb == va => continue,
                        Some(vb) => format!("~ {k}: {va}  →  {vb}"),
                        None => format!("- {k}: {va}"),
                    };
                    if noise.prop(&x.class, k) {
                        hidden += 1;
                    } else {
                        c.lines.push(line);
                    }
                }
                for (k, vb) in &y.props {
                    if ma.contains_key(k) {
                        continue;
                    }
                    if noise.prop(&y.class, k) {
                        hidden += 1;
                    } else {
                        c.lines.push(format!("+ {k}: {vb}"));
                    }
                }
                d.noise_props += hidden;
                if c.lines.is_empty() {
                    if hidden > 0 {
                        d.noise_objects += 1;
                    } else {
                        d.same += 1;
                    }
                    continue;
                }
            }
            (Some(x), None) => {
                c.kind = Kind::OnlyA;
                c.lines = x.props.iter().map(|(k, v)| format!("- {k}: {v}")).collect();
            }
            (None, Some(y)) => {
                c.kind = Kind::OnlyB;
                c.lines = y.props.iter().map(|(k, v)| format!("+ {k}: {v}")).collect();
            }
            (None, None) => unreachable!(),
        }
        d.changes.push(c);
    }
    d
}

/// Properties listed for an object that is in one dump only.
const MAX_ONE_SIDED: usize = 60;

/// The lines of one change, indented; an object in one dump only is cut
/// short (`--api <dump> --handle <h>` has all of it).
pub fn write_lines(out: &mut String, c: &Change, indent: &str) {
    let max = if c.kind == Kind::Changed {
        usize::MAX
    } else {
        MAX_ONE_SIDED
    };
    for l in c.lines.iter().take(max) {
        let _ = writeln!(out, "{indent}{l}");
    }
    if c.lines.len() > max {
        let _ = writeln!(
            out,
            "{indent}… {} more properties (--api <dump> --handle {:X})",
            c.lines.len() - max,
            c.handle
        );
    }
}

pub fn kind_text(k: Kind) -> &'static str {
    match k {
        Kind::Changed => "changed",
        Kind::OnlyA => "only in A",
        Kind::OnlyB => "only in B",
    }
}

/// The summary line of a comparison.
pub fn summary(d: &Diff, a: &Dump, b: &Dump) -> String {
    let count = |k: Kind| d.changes.iter().filter(|c| c.kind == k).count();
    let mut s = format!(
        "API objects (matched by handle): A {}, B {}; identical {}; changed {}; only in A {}; only in B {}",
        a.objects.len(),
        b.objects.len(),
        d.same,
        count(Kind::Changed),
        count(Kind::OnlyA),
        count(Kind::OnlyB)
    );
    if d.filtered > 0 {
        let _ = write!(s, "; {} left out by the filter", d.filtered);
    }
    if d.noise_props > 0 {
        let _ = write!(
            s,
            "; noise list dropped {} property change(s), {} object(s) had no other",
            d.noise_props, d.noise_objects
        );
    }
    s
}

/// The changes between two dumps as text (only what differs).
pub fn diff_report(a: &Dump, b: &Dump, filter: &Filter, noise: &Noise) -> String {
    let d = diff(a, b, filter, noise);
    let mut out = String::new();
    let _ = writeln!(out, "Civil 3D API dump comparison (civil3drust workbench)");
    let _ = writeln!(out, "A: {}", a.path);
    let _ = writeln!(out, "B: {}", b.path);
    for (name, x) in [("A", a), ("B", b)] {
        for h in x.header.iter().filter(|h| h.starts_with("written:")) {
            let _ = writeln!(out, "{name} {h}");
        }
    }
    if !filter.is_empty() {
        let _ = writeln!(out, "filter: {}", filter.describe());
    }
    let _ = writeln!(out, "{}", summary(&d, a, b));
    for c in &d.changes {
        let _ = writeln!(out, "\n== {}: {}", kind_text(c.kind), c.title);
        write_lines(&mut out, c, "   ");
    }
    out
}

/// A noise list from two dumps of a drawing saved twice without a change:
/// every property that differs.
pub fn noise_from(a: &Dump, b: &Dump) -> Vec<String> {
    let d = diff(a, b, &Filter::default(), &Noise::default());
    let mut rules = BTreeSet::new();
    for c in d.changes.iter().filter(|c| c.kind == Kind::Changed) {
        for l in &c.lines {
            let key = l[2..].split(": ").next().unwrap_or("");
            if key != "entry" && !key.is_empty() {
                rules.insert(format!("prop {} {key}", c.class));
            }
        }
    }
    rules.into_iter().collect()
}

fn stem(dwg: &Path) -> String {
    dwg.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The dumps written on save for `dwg` (`c3dump/<stem>.<stamp>.api.txt`
/// next to it), oldest first.
pub fn history(dwg: &Path) -> Vec<PathBuf> {
    let dir = dwg.parent().unwrap_or(Path::new(".")).join("c3dump");
    let prefix = format!("{}.", stem(dwg).to_ascii_lowercase());
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            n.strip_prefix(&prefix)
                .and_then(|r| r.strip_suffix(".api.txt"))
                // Only the time stamp between the name and the suffix
                // (`S1.dwg` must not pick up `S1.copy.….api.txt`).
                .is_some_and(|stamp| !stamp.is_empty() && !stamp.contains('.'))
        })
        .collect();
    v.sort();
    v
}

/// The dump that belongs to `dwg`: the newest of the manual one next to
/// it and the last one written on save.
pub fn find_for(dwg: &Path) -> Option<PathBuf> {
    let manual = dwg.with_file_name(format!("{}.api.txt", stem(dwg)));
    let mut found: Vec<PathBuf> = history(dwg).pop().into_iter().collect();
    if manual.is_file() {
        found.push(manual);
    }
    found
        .into_iter()
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "drawing: C:\\x\\S1.dwg
written: 2026-11-12 21:09:03Z
config: classes control

[8D6A] Alignment - (1)
  AlignmentType: Centerline
  Description:\x20
  Length: 210.2
  DesignSpeed #1: station 0, value 60, comment \"\"
  Entity 1 (before <EntityNotFoundException: NoPrevAlignmentEntity,>, after 2): Line, Constraint1 Fixed
  Entity 2 (before 1, after <EntityNotFoundException: NoNextAlignmentEntity,>): Line, Constraint1 Fixed

  Profile [8D72] Profile
    AlignmentId: 8D6A
    Name: prpfile1
    PVIs[0] ProfilePVI
      Elevation: 10
      Station: 0
    PVIs[1] ProfilePVI
      Elevation: 12.5
      Station: 100

== Objects

[A1C] AECC_GRADING_GROUP DBObject owner 8B
  Name: Grading Group
  DateModified: 2/23/2006 10:26:49 PM
  GetReactors() [2]: ImpRXObject, ImpRXObject

[8D58] AECC_FEATURE_STYLE Site owner 8B
  Name: A
  GetAlignmentIds() [0]:\x20

objects: 4
";

    #[test]
    fn parses_entries_and_properties() {
        let d = parse(A);
        assert_eq!(d.header.len(), 4, "{:?}", d.header);
        assert_eq!(d.objects.len(), 4);
        let al = &d.objects[&0x8D6A];
        assert_eq!(al.class, "AECC_ALIGNMENT");
        assert_eq!(al.parent, None);
        let get = |o: &Object, k: &str| {
            o.props
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| panic!("no {k} in {:?}", o.props))
        };
        assert_eq!(get(al, "Description"), "");
        assert_eq!(
            get(al, "DesignSpeed #1"),
            "station 0, value 60, comment \"\""
        );
        assert!(get(al, "Entity 1").ends_with("after 2): Line, Constraint1 Fixed"));
        assert!(get(al, "Entity 2").starts_with("(before 1,"));
        let p = &d.objects[&0x8D72];
        assert_eq!(p.class, "AECC_PROFILE");
        assert_eq!(p.parent, Some(0x8D6A));
        assert_eq!(get(p, "PVIs[1].Elevation"), "12.5");
        assert_eq!(get(p, "Name"), "prpfile1");
        let g = &d.objects[&0xA1C];
        assert_eq!(g.class, "AECC_GRADING_GROUP");
        assert_eq!(get(g, "GetReactors()"), "[2] ImpRXObject, ImpRXObject");
        assert_eq!(d.objects[&0x8D58].parent, None);
    }

    fn edited() -> String {
        A.replace("Elevation: 12.5", "Elevation: 14")
            .replace("10:26:49 PM", "10:26:50 PM")
            .replace(
                "  GetAlignmentIds() [0]: \n",
                "  GetAlignmentIds() [1]: 8D6A\n\n[8D7B] AECC_FEATURE_STYLE Site owner 8B\n  Name: NEW\n",
            )
    }

    #[test]
    fn diff_lists_only_changes() {
        let (a, b) = (parse(A), parse(&edited()));
        let d = diff(&a, &b, &Filter::default(), &Noise::default());
        let by: BTreeMap<u64, &Change> = d.changes.iter().map(|c| (c.handle, c)).collect();
        assert_eq!(d.same, 1, "the alignment is unchanged");
        assert_eq!(by[&0x8D72].lines, ["~ PVIs[1].Elevation: 12.5  →  14"]);
        assert_eq!(by[&0x8D58].lines, ["~ GetAlignmentIds(): [0]  →  [1] 8D6A"]);
        assert_eq!(by[&0x8D7B].kind, Kind::OnlyB);
        assert_eq!(by[&0xA1C].lines.len(), 1);

        let noise = Noise::parse("# every save\nprop AECC_GRADING* Date*\n").unwrap();
        let d = diff(&a, &b, &Filter::default(), &noise);
        assert!(d.changes.iter().all(|c| c.handle != 0xA1C));
        assert_eq!((d.noise_props, d.noise_objects), (1, 1));

        let f = Filter::new("AECC_PROFILE*", "", "").unwrap();
        let d = diff(&a, &b, &f, &Noise::default());
        assert_eq!(d.changes.len(), 1);
        assert_eq!(d.changes[0].handle, 0x8D72);

        let text = diff_report(&a, &b, &Filter::default(), &Noise::default());
        assert!(!text.contains("AlignmentType"), "{text}");
    }

    #[test]
    fn export_reads_back() {
        let d = parse(A);
        let f = Filter::new("AECC_PROFILE,AECC_ALIGNMENT", "", "").unwrap();
        let back = parse(&export(&d, &f));
        assert_eq!(back.objects.len(), 2);
        for (h, o) in &back.objects {
            assert_eq!(o.class, d.objects[h].class);
            assert_eq!(o.props, d.objects[h].props);
        }
        let f = Filter::new("", "#8d58", "").unwrap();
        assert_eq!(parse(&export(&d, &f)).objects.len(), 1);
    }

    #[test]
    fn noise_rules() {
        let n = Noise::parse("record #1F\nrecord AECC_DISP_REP_*\nprop * DateModified\n").unwrap();
        assert_eq!(n.rules(), 3);
        assert!(n.record(0x1F, "LAYER : 0"));
        assert!(n.record(0x99, "AECC_DISP_REP_ALIGNMENT : #99"));
        assert!(!n.record(0x99, "AECC_ALIGNMENT : a"));
        assert!(n.prop("ANY", "DateModified"));
        assert!(Noise::parse("bogus line").is_err());
        let (a, b) = (parse(A), parse(&edited()));
        let rules = noise_from(&a, &b);
        assert!(
            rules.contains(&"prop AECC_GRADING_GROUP DateModified".to_string()),
            "{rules:?}"
        );
    }

    #[test]
    fn glob_matches() {
        assert!(glob("aecc_*node", "AECC_ALIGNMENT_NODE"));
        assert!(glob("*", ""));
        assert!(!glob("AECC_PROFILE", "AECC_PROFILE_VIEW"));
    }

    #[test]
    fn finds_the_dumps_of_a_drawing() {
        let dir = std::env::temp_dir().join(format!("c3dump_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("c3dump")).unwrap();
        let dwg = dir.join("S1.dwg");
        assert_eq!(find_for(&dwg), None);
        for n in [
            "S1.20261113-090000.api.txt",
            "S1.20261113-101500.api.txt",
            "S1.copy.20261113-110000.api.txt",
            "S10.20261113-120000.api.txt",
        ] {
            std::fs::write(dir.join("c3dump").join(n), A).unwrap();
        }
        let h = history(&dwg);
        let names: Vec<String> = h
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            ["S1.20261113-090000.api.txt", "S1.20261113-101500.api.txt"]
        );
        assert_eq!(find_for(&dwg), h.last().cloned());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn real_samples_parse() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples");
        let d = load(dir.join("S339_S322_exp_create_site.api.txt")).unwrap();
        assert_eq!(d.objects[&0x8D6A].class, "AECC_ALIGNMENT");
        let sites: Vec<&Object> = d
            .objects
            .values()
            .filter(|o| o.title.contains(" Site "))
            .collect();
        assert_eq!(sites.len(), 2, "sites A and NEW SITE");
        let p = load(dir.join("S306_profile_curve_length_150.api.txt")).unwrap();
        assert!(p
            .objects
            .values()
            .any(|o| o.class == "AECC_PROFILE"
                && o.props.iter().any(|(k, _)| k.starts_with("PVIs[0]."))));
    }
}
