//! The workbench's document state, independent of the GUI (D-073).
//!
//! The open document is the only source of truth. The scene (what is
//! drawn) and the record index (who references whom) are rebuilt from it
//! after every change; nothing edits them directly. Every change runs on a
//! copy of the document under `catch_unwind`, so an error or a panic
//! leaves the document as it was.

use crate::filemap;
use crate::scene::Scene;
use acadrust::document::CadDocument;
use civil3drust::commands::{self, quote, CommandInfo};
use civil3drust::dump::{self, AlignmentElement};
use civil3drust::handle_refs;
use civil3drust::raw::{self, Opened, RawRecord};
use std::collections::{BTreeMap, HashMap};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// One raw record and its references.
#[derive(Debug, Clone)]
pub struct RecordInfo {
    pub rec: RawRecord,
    pub name: Option<String>,
    /// (bit, code, target) for each handle reference in the record.
    pub refs: Vec<(usize, u8, u64)>,
}

/// Raw records by handle, with back-references.
#[derive(Debug, Default)]
pub struct RecordIndex {
    pub records: BTreeMap<u64, RecordInfo>,
    /// target -> records referencing it.
    pub back: HashMap<u64, Vec<u64>>,
}

impl RecordIndex {
    pub fn build(doc: &CadDocument) -> RecordIndex {
        let mut idx = RecordIndex::default();
        for rec in dump::raw_records_of(doc) {
            let refs: Vec<(usize, u8, u64)> = handle_refs::of_record(&rec)
                .into_iter()
                .filter(|r| r.handle != 0)
                .map(|r| (r.bit, r.code, r.handle))
                .collect();
            for (_, _, t) in &refs {
                let v = idx.back.entry(*t).or_default();
                if !v.contains(&rec.handle) {
                    v.push(rec.handle);
                }
            }
            let name = raw::object_name(&rec.data);
            idx.records
                .insert(rec.handle, RecordInfo { rec, name, refs });
        }
        idx
    }
}

/// How a record in the hex pane relates to the selection.
#[derive(Debug, Clone, PartialEq)]
pub enum Relation {
    Selected,
    /// Referenced by `from` (depth 1 = by the selection).
    RefBy {
        from: u64,
        depth: usize,
    },
    /// References `to`.
    RefersTo {
        to: u64,
    },
}

/// A record to show for the selection.
#[derive(Debug, Clone)]
pub struct Related {
    pub handle: u64,
    pub relation: Relation,
}

/// One undo step: the whole document before a change.
pub struct Snapshot {
    pub doc: CadDocument,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogKind {
    Command,
    Ok,
    Error,
    Info,
}

pub struct Workbench {
    pub path: PathBuf,
    pub opened: Opened,
    pub scene: Scene,
    pub index: RecordIndex,
    /// Raw record bytes as opened, for the changed-bits colouring.
    pub baseline: HashMap<u64, Vec<u8>>,
    pub undo: Vec<Snapshot>,
    pub redo: Vec<Snapshot>,
    pub log: Vec<(LogKind, String)>,
    /// Changed since opened or last saved.
    pub dirty: bool,
    /// The file's record map (bytes of records acadrust parses); `None`
    /// while it loads or when mapping failed.
    pub filemap: Option<filemap::Loaded>,
    /// The Civil 3D API dump that belongs to the drawing (`C3DDUMP`,
    /// D-080), when there is one next to it.
    pub api: Option<crate::apidump::Dump>,
}

const UNDO_LIMIT: usize = 50;

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic".into()
    }
}

/// Run `f` on a copy of `doc`; the copy comes back only if `f` succeeded
/// without panicking.
fn on_copy<T>(
    doc: &CadDocument,
    f: impl FnOnce(&mut CadDocument) -> Result<T, String>,
) -> Result<(CadDocument, T), String> {
    let mut copy = doc.clone();
    match catch_unwind(AssertUnwindSafe(|| f(&mut copy))) {
        Ok(Ok(v)) => Ok((copy, v)),
        Ok(Err(e)) => Err(e),
        Err(p) => Err(format!("panicked: {}", panic_text(p))),
    }
}

/// True if `p` is inside a `samples` directory (never written to).
pub fn in_samples(p: &Path) -> bool {
    p.parent()
        .and_then(|d| d.file_name())
        .is_some_and(|n| n == "samples")
}

impl Workbench {
    /// Open a drawing (the file map is loaded separately, see
    /// [`filemap::load`]).
    pub fn open(path: impl AsRef<Path>) -> Result<Workbench, String> {
        let path = path.as_ref().to_path_buf();
        let opened = catch_unwind(|| raw::open(&path))
            .map_err(|p| format!("panicked while reading: {}", panic_text(p)))?
            .map_err(|e| e.to_string())?;
        let mut wb = Workbench {
            scene: Scene::default(),
            index: RecordIndex::default(),
            baseline: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            log: Vec::new(),
            dirty: false,
            filemap: None,
            api: None,
            path,
            opened,
        };
        wb.rebuild();
        wb.baseline = wb
            .index
            .records
            .iter()
            .map(|(h, r)| (*h, r.rec.data.clone()))
            .collect();
        wb.info(format!(
            "opened {} ({} raw records)",
            wb.path.display(),
            wb.index.records.len()
        ));
        wb.load_api();
        if let Some(r) = &wb.opened.failsafe_reason {
            wb.log.push((
                LogKind::Error,
                format!(
                    "read in failsafe mode: records acadrust could not parse are missing ({r})"
                ),
            ));
        }
        Ok(wb)
    }

    /// Pick up the API dump next to the drawing (the manual
    /// `<drawing>.api.txt` or the last one in `c3dump/`, whichever is
    /// newer).
    pub fn load_api(&mut self) {
        self.api = None;
        let Some(p) = crate::apidump::find_for(&self.path) else {
            return;
        };
        match crate::apidump::load(&p) {
            Ok(d) => {
                self.info(format!(
                    "API dump {} ({} objects)",
                    p.display(),
                    d.objects.len()
                ));
                self.api = Some(d);
            }
            Err(e) => self.log.push((LogKind::Error, format!("API dump: {e}"))),
        }
    }

    pub fn doc(&self) -> &CadDocument {
        &self.opened.doc
    }

    fn info(&mut self, s: String) {
        self.log.push((LogKind::Info, s));
    }

    /// Rebuild everything derived from the document.
    pub fn rebuild(&mut self) {
        let path = self.path.display().to_string();
        self.scene = match catch_unwind(AssertUnwindSafe(|| Scene::from_doc(&path, &self.opened))) {
            Ok(s) => s,
            Err(p) => {
                self.log.push((
                    LogKind::Error,
                    format!("drawing the scene panicked: {}", panic_text(p)),
                ));
                Scene {
                    path,
                    ..Default::default()
                }
            }
        };
        self.index = RecordIndex::build(&self.opened.doc);
    }

    /// Replace the document with `doc`, keeping the old one for undo.
    fn commit(&mut self, doc: CadDocument, label: String) {
        let old = std::mem::replace(&mut self.opened.doc, doc);
        self.undo.push(Snapshot { doc: old, label });
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        self.rebuild();
    }

    /// Run one command line (see `civil3drust::commands`). `@` as a word
    /// stands for `selected` (a hex handle). Reporting commands leave the
    /// document alone and add no undo step.
    pub fn run(&mut self, line: &str, selected: Option<u64>) -> Result<String, String> {
        let line = expand_selection(line, selected)?;
        self.log.push((LogKind::Command, line.clone()));
        let edits = commands::tokenize(&line)?
            .first()
            .and_then(|n| commands::find(n))
            .is_none_or(|c| c.edits);
        let r = on_copy(&self.opened.doc, |d| commands::run_line(d, &line));
        match r {
            Ok((doc, report)) => {
                if edits {
                    self.commit(doc, line);
                }
                self.log.push((LogKind::Ok, report.clone()));
                Ok(report)
            }
            Err(e) => {
                self.log.push((LogKind::Error, e.clone()));
                Err(e)
            }
        }
    }

    /// Run several command lines as one undo step, all or nothing.
    pub fn run_batch(&mut self, lines: &[String], label: &str) -> Result<String, String> {
        for l in lines {
            self.log.push((LogKind::Command, l.clone()));
        }
        let r = on_copy(&self.opened.doc, |d| {
            let mut out = Vec::new();
            for l in lines {
                out.push(commands::run_line(d, l).map_err(|e| format!("{l}: {e}"))?);
            }
            Ok(out.join("\n"))
        });
        match r {
            Ok((doc, report)) => {
                self.commit(doc, label.to_string());
                self.log.push((LogKind::Ok, report.clone()));
                Ok(report)
            }
            Err(e) => {
                self.log
                    .push((LogKind::Error, format!("nothing changed: {e}")));
                Err(e)
            }
        }
    }

    /// Run a command with an external `c3dr` (a newer library build): the
    /// document goes out as a temporary DWG and comes back read with
    /// `raw::open`. `info` is that `c3dr`'s spec (`c3dr --spec`).
    pub fn run_external(
        &mut self,
        exe: &Path,
        info: &[CommandInfo],
        line: &str,
        selected: Option<u64>,
    ) -> Result<String, String> {
        let line = expand_selection(line, selected)?;
        self.log
            .push((LogKind::Command, format!("[{}] {line}", exe.display())));
        let r = self.external(exe, info, &line);
        match &r {
            Ok(t) => self.log.push((LogKind::Ok, t.clone())),
            Err(e) => self.log.push((LogKind::Error, e.clone())),
        }
        r
    }

    fn external(&mut self, exe: &Path, info: &[CommandInfo], line: &str) -> Result<String, String> {
        let words = commands::tokenize(line)?;
        let (name, rest) = words.split_first().ok_or("empty command")?;
        let edits = info
            .iter()
            .find(|c| &c.name == name)
            .ok_or_else(|| format!("{} has no command '{name}'", exe.display()))?
            .edits;
        let dir = std::env::temp_dir().join(format!("c3dr-workbench-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let input = dir.join("in.dwg");
        let output = dir.join("out.dwg");
        let _ = std::fs::remove_file(&output);
        acadrust::DwgWriter::write_to_file(&input, &self.opened.doc)
            .map_err(|e| format!("writing the temporary drawing: {e}"))?;
        let mut cmd = std::process::Command::new(exe);
        cmd.arg(name).arg(&input);
        if edits {
            cmd.arg(&output);
        }
        cmd.args(rest);
        let out = cmd
            .output()
            .map_err(|e| format!("running {}: {e}", exe.display()))?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !out.status.success() {
            return Err(format!(
                "{}{}",
                String::from_utf8_lossy(&out.stderr).trim(),
                if text.is_empty() {
                    String::new()
                } else {
                    format!("\n{text}")
                }
            ));
        }
        if edits {
            let back = raw::open(&output).map_err(|e| format!("reading c3dr's output: {e}"))?;
            self.commit(back.doc, format!("[external] {line}"));
        }
        Ok(text)
    }

    pub fn undo(&mut self) -> Option<String> {
        let s = self.undo.pop()?;
        let cur = std::mem::replace(&mut self.opened.doc, s.doc);
        self.redo.push(Snapshot {
            doc: cur,
            label: s.label.clone(),
        });
        self.dirty = true;
        self.rebuild();
        self.info(format!("undo: {}", s.label));
        Some(s.label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let s = self.redo.pop()?;
        let cur = std::mem::replace(&mut self.opened.doc, s.doc);
        self.undo.push(Snapshot {
            doc: cur,
            label: s.label.clone(),
        });
        self.dirty = true;
        self.rebuild();
        self.info(format!("redo: {}", s.label));
        Some(s.label)
    }

    /// Records shown for `handle`: itself, what it references (to
    /// `depth` hops), and what references it.
    pub fn related(&self, handle: u64, depth: usize) -> Vec<Related> {
        let mut out = vec![Related {
            handle,
            relation: Relation::Selected,
        }];
        let mut seen = vec![handle];
        let mut frontier = vec![handle];
        for d in 1..=depth {
            let mut next = Vec::new();
            for h in frontier {
                let targets: Vec<u64> = match self.index.records.get(&h) {
                    Some(r) => r.refs.iter().map(|x| x.2).collect(),
                    None => Vec::new(),
                };
                for t in targets {
                    if !seen.contains(&t) {
                        seen.push(t);
                        next.push(t);
                        out.push(Related {
                            handle: t,
                            relation: Relation::RefBy { from: h, depth: d },
                        });
                    }
                }
            }
            frontier = next;
        }
        if let Some(b) = self.index.back.get(&handle) {
            for &f in b {
                if !seen.contains(&f) {
                    seen.push(f);
                    out.push(Related {
                        handle: f,
                        relation: Relation::RefersTo { to: handle },
                    });
                }
            }
        }
        out
    }

    /// Class name of any handle (raw record, acadrust object or table entry).
    pub fn class_of(&self, handle: u64) -> String {
        if let Some(r) = self.index.records.get(&handle) {
            return r.rec.class.clone();
        }
        if let Some(s) = self.file_span(handle) {
            return s.class.clone();
        }
        raw::class_of(self.doc(), handle).unwrap_or_else(|| "?".into())
    }

    /// The record's frame in the file map, for records acadrust parses.
    pub fn file_span(&self, handle: u64) -> Option<&filemap::Span> {
        let pane = self.filemap.as_ref()?.panes.first()?;
        pane.find_handle(handle).map(|i| &pane.spans[i])
    }

    /// Bytes of the record's data in the file on disk.
    pub fn file_bytes(&self, handle: u64) -> Option<&[u8]> {
        let pane = self.filemap.as_ref()?.panes.first()?;
        let s = &pane.spans[pane.find_handle(handle)?];
        pane.bytes.get(s.data_start..s.data_start + s.data_len)
    }

    /// Handles whose raw bytes differ from the baseline (or are new).
    pub fn changed(&self) -> Vec<u64> {
        let mut v: Vec<u64> = self
            .index
            .records
            .iter()
            .filter(|(h, r)| self.baseline.get(h) != Some(&r.rec.data))
            .map(|(h, _)| *h)
            .collect();
        v.extend(
            self.baseline
                .keys()
                .filter(|h| !self.index.records.contains_key(h)),
        );
        v.sort_unstable();
        v
    }

    /// The selected object as RON: Model (decoded alignment) when there is
    /// one, else `None`.
    pub fn model_ron(&self, handle: u64) -> Option<String> {
        let a = dump::alignment_of(self.doc(), handle)?;
        Some(dump::alignment_ron(&a))
    }

    pub fn raw_ron(&self, handle: u64) -> Option<String> {
        self.index
            .records
            .get(&handle)
            .map(|r| dump::record_ron(&r.rec))
    }

    /// Apply edited Raw RON to record `handle`.
    pub fn apply_raw_ron(&mut self, handle: u64, text: &str) -> Result<String, String> {
        let r = self.apply_raw_inner(handle, text);
        match &r {
            Ok(s) => self.log.push((LogKind::Ok, s.clone())),
            Err(e) => self.log.push((LogKind::Error, format!("raw apply: {e}"))),
        }
        r
    }

    fn apply_raw_inner(&mut self, handle: u64, text: &str) -> Result<String, String> {
        let (rec, bytes) = dump::record_from_ron(text)?;
        let h = commands::parse_handle(&rec.h).ok_or("bad handle in h")?;
        if h != handle {
            return Err(format!("h is {h:X}, the selection is {handle:X}"));
        }
        let cur = self.index.records.get(&handle).ok_or("not a raw record")?;
        if cur.rec.class != rec.class || cur.rec.is_entity != rec.entity {
            return Err("class and entity can't be changed".into());
        }
        if rec.handle_bits > bytes.len() * 8 {
            return Err("handle_bits is larger than the record".into());
        }
        if bytes == cur.rec.data && rec.handle_bits == cur.rec.handle_bits {
            return Ok("no change".into());
        }
        let msg = format!(
            "[{handle:X}] raw bytes replaced: {} -> {} bytes, handle stream {} -> {} bits",
            cur.rec.data.len(),
            bytes.len(),
            cur.rec.handle_bits,
            rec.handle_bits
        );
        let hb = rec.handle_bits;
        let (doc, ()) = on_copy(&self.opened.doc, |d| {
            if raw::set_record(d, handle, bytes, hb) {
                Ok(())
            } else {
                Err("could not replace the record".into())
            }
        })?;
        self.commit(doc, format!("raw edit {handle:X}"));
        Ok(msg)
    }

    /// Apply edited Model RON: translated into commands, run as one step.
    pub fn apply_model_ron(&mut self, handle: u64, text: &str) -> Result<String, String> {
        let lines = match self.model_commands(handle, text) {
            Ok(l) => l,
            Err(e) => {
                self.log.push((LogKind::Error, format!("model apply: {e}")));
                return Err(e);
            }
        };
        if lines.is_empty() {
            self.info("model apply: no change".into());
            return Ok("no change".into());
        }
        self.run_batch(&lines, &format!("model edit {handle:X}"))
    }

    /// The commands that turn the current alignment into the one in
    /// `text`. Errors name what can't be expressed.
    pub fn model_commands(&self, handle: u64, text: &str) -> Result<Vec<String>, String> {
        let new = dump::alignment_from_ron(text).map_err(|e| e.to_string())?;
        let cur = dump::alignment_of(self.doc(), handle).ok_or("not an alignment")?;
        model_diff(&cur, &new)
    }

    /// Write the document to `path`, read it back and compare raw records.
    pub fn save_as(&mut self, path: &Path) -> Result<String, String> {
        let r = self.save_inner(path);
        match &r {
            Ok(s) => self.log.push((LogKind::Ok, s.clone())),
            Err(e) => self.log.push((LogKind::Error, format!("save: {e}"))),
        }
        r
    }

    fn save_inner(&mut self, path: &Path) -> Result<String, String> {
        let same = |a: &Path, b: &Path| {
            a == b
                || matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)),
                    (Ok(x), Ok(y)) if x == y)
        };
        if same(path, &self.path) {
            return Err("Save As only: choose a new file, not the one that is open".into());
        }
        if in_samples(path) {
            return Err("refusing to write into samples/".into());
        }
        let doc = &self.opened.doc;
        catch_unwind(AssertUnwindSafe(|| {
            acadrust::DwgWriter::write_to_file(path, doc)
        }))
        .map_err(|p| format!("writer panicked: {}", panic_text(p)))?
        .map_err(|e| e.to_string())?;
        let back = raw::open(path).map_err(|e| format!("written file does not read back: {e}"))?;
        let mine: BTreeMap<u64, (Vec<u8>, usize)> = self
            .index
            .records
            .iter()
            .map(|(h, r)| (*h, (r.rec.data.clone(), r.rec.handle_bits)))
            .collect();
        let theirs: BTreeMap<u64, (Vec<u8>, usize)> = dump::raw_records_of(&back.doc)
            .into_iter()
            .map(|r| (r.handle, (r.data, r.handle_bits)))
            .collect();
        let differ: Vec<String> = mine
            .iter()
            .filter(|(h, v)| theirs.get(h) != Some(v))
            .map(|(h, _)| format!("{h:X}"))
            .collect();
        let extra = theirs.keys().filter(|h| !mine.contains_key(h)).count();
        self.dirty = false;
        self.path = path.to_path_buf();
        // The dump described the drawing that was opened; the saved copy
        // has its own once Civil 3D has saved it.
        self.api = None;
        let mut msg = format!(
            "saved {}: {} raw records read back",
            path.display(),
            theirs.len()
        );
        if differ.is_empty() && extra == 0 {
            msg.push_str(", all match");
        } else {
            msg.push_str(&format!(
                "; {} differ ({}), {extra} new on reading",
                differ.len(),
                differ.join(" ")
            ));
        }
        if let Some(r) = &back.failsafe_reason {
            msg.push_str(&format!("; read back only in failsafe mode ({r})"));
        }
        Ok(msg)
    }
}

/// Replace each word `@` with the selected handle.
pub fn expand_selection(line: &str, selected: Option<u64>) -> Result<String, String> {
    let words = commands::tokenize(line)?;
    if !words.iter().any(|w| w == "@") {
        return Ok(line.trim().to_string());
    }
    let h = selected.ok_or("@ needs a selection")?;
    Ok(words
        .iter()
        .map(|w| if w == "@" { format!("{h:X}") } else { quote(w) })
        .collect::<Vec<_>>()
        .join(" "))
}

fn fmt_num(v: f64) -> String {
    let s = format!("{v:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
}

/// Intersection of the infinite lines through (a, b) and (c, d).
fn intersect(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> Option<(f64, f64)> {
    let r = (b.0 - a.0, b.1 - a.1);
    let s = (d.0 - c.0, d.1 - c.1);
    let den = r.0 * s.1 - r.1 * s.0;
    let scale = (r.0.hypot(r.1) * s.0.hypot(s.1)).max(1e-300);
    if den.abs() / scale < 1e-12 {
        return None;
    }
    let t = ((c.0 - a.0) * s.1 - (c.1 - a.1) * s.0) / den;
    Some((a.0 + t * r.0, a.1 + t * r.1))
}

/// Commands that turn `cur` into `new` (Model RON apply, D-073): name,
/// style, start station, line geometry (as PIs) and free-curve radii.
pub fn model_diff(cur: &dump::Alignment, new: &dump::Alignment) -> Result<Vec<String>, String> {
    let h = &cur.h;
    if new.h != cur.h {
        return Err("h (the handle) can't be changed".into());
    }
    if new.detached != cur.detached || new.groups != cur.groups || new.error != cur.error {
        return Err("detached pieces, layout groups and error are read-only; use Raw".into());
    }
    let mut out = Vec::new();
    if new.name != cur.name {
        let n = new.name.as_deref().ok_or("name can't be removed")?;
        out.push(format!("set_name {h} {}", quote(n)));
    }
    if new.style != cur.style {
        let s = new.style.as_deref().ok_or("style can't be removed")?;
        out.push(format!(
            "set_style {h} {}",
            quote(s.trim_start_matches('#'))
        ));
    }
    if new.start_station != cur.start_station {
        let s = new.start_station.ok_or("start_station can't be removed")?;
        out.push(format!("set_start {h} {}", fmt_num(s)));
    }
    if new.elements.len() != cur.elements.len() {
        return Err("element count changed: use the curve / remove commands".into());
    }
    let mut lines_moved = false;
    let mut radii = Vec::new();
    for (i, (a, b)) in cur.elements.iter().zip(&new.elements).enumerate() {
        use AlignmentElement as E;
        match (a, b) {
            (
                E::Line { id, start, end },
                E::Line {
                    id: id2,
                    start: s2,
                    end: e2,
                },
            ) => {
                if id != id2 {
                    return Err(format!("element {i}: id can't be changed"));
                }
                if !near(*start, *s2) || !near(*end, *e2) {
                    lines_moved = true;
                }
            }
            (
                E::Arc {
                    id,
                    radius,
                    ccw,
                    method,
                    ..
                },
                E::Arc {
                    id: id2,
                    radius: r2,
                    ccw: c2,
                    method: m2,
                    ..
                },
            ) => {
                if id != id2 || ccw != c2 || method != m2 {
                    return Err(format!("element {i}: only an arc's radius can be changed"));
                }
                if (radius - r2).abs() > 1e-12 {
                    radii.push((*id, *r2));
                }
            }
            (a, b) if a == b => {}
            (E::Spiral { .. }, E::Spiral { .. }) => {
                return Err(format!("element {i}: spirals are read-only here; use Raw"));
            }
            _ => return Err(format!("element {i}: kind can't be changed")),
        }
    }
    if lines_moved {
        let lines: Vec<((f64, f64), (f64, f64))> = new
            .elements
            .iter()
            .filter_map(|e| match e {
                AlignmentElement::Line { start, end, .. } => Some((*start, *end)),
                _ => None,
            })
            .collect();
        let first_is_line = matches!(new.elements.first(), Some(AlignmentElement::Line { .. }));
        let last_is_line = matches!(new.elements.last(), Some(AlignmentElement::Line { .. }));
        if !first_is_line || !last_is_line {
            return Err("moving lines needs an alignment that starts and ends with a line".into());
        }
        let mut pis = vec![lines[0].0];
        for w in lines.windows(2) {
            let p = intersect(w[0].0, w[0].1, w[1].0, w[1].1)
                .ok_or("two consecutive lines are parallel: no PI between them")?;
            pis.push(p);
        }
        pis.push(lines[lines.len() - 1].1);
        let text: Vec<String> = pis
            .iter()
            .map(|p| format!("{},{}", fmt_num(p.0), fmt_num(p.1)))
            .collect();
        out.push(format!("set_pis {h} {}", quote(&text.join(" "))));
    }
    for (id, r) in radii {
        out.push(format!("set_radius {h} {id} {}", fmt_num(r)));
    }
    if out.is_empty() && new.length != cur.length {
        return Err("length is derived from the elements; change those instead".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn al() -> dump::Alignment {
        dump::Alignment {
            h: "2A".into(),
            name: Some("A".into()),
            style: Some("S".into()),
            start_station: Some(0.0),
            length: 200.0,
            elements: vec![
                AlignmentElement::Line {
                    id: 1,
                    start: (0.0, 0.0),
                    end: (90.0, 0.0),
                },
                AlignmentElement::Arc {
                    id: 3,
                    start: (90.0, 0.0),
                    end: (100.0, 10.0),
                    center: (90.0, 10.0),
                    radius: 10.0,
                    ccw: true,
                    method: "free".into(),
                },
                AlignmentElement::Line {
                    id: 2,
                    start: (100.0, 10.0),
                    end: (100.0, 100.0),
                },
            ],
            detached: vec![],
            groups: vec![],
            error: None,
        }
    }

    #[test]
    fn diff_name_style_station() {
        let mut n = al();
        n.name = Some("New name".into());
        n.style = Some("#1F64".into());
        n.start_station = Some(1000.0);
        assert_eq!(
            model_diff(&al(), &n).unwrap(),
            vec![
                "set_name 2A \"New name\"",
                "set_style 2A 1F64",
                "set_start 2A 1000"
            ]
        );
        assert!(model_diff(&al(), &al()).unwrap().is_empty());
    }

    #[test]
    fn diff_lines_become_pis() {
        let mut n = al();
        if let AlignmentElement::Line { end, .. } = &mut n.elements[2] {
            *end = (100.0, 150.0);
        }
        assert_eq!(
            model_diff(&al(), &n).unwrap(),
            vec!["set_pis 2A \"0,0 100,0 100,150\""]
        );
    }

    #[test]
    fn diff_radius_and_refusals() {
        let mut n = al();
        if let AlignmentElement::Arc { radius, .. } = &mut n.elements[1] {
            *radius = 25.5;
        }
        assert_eq!(model_diff(&al(), &n).unwrap(), vec!["set_radius 2A 3 25.5"]);
        let mut n = al();
        n.elements.pop();
        assert!(model_diff(&al(), &n).is_err());
        let mut n = al();
        n.h = "2B".into();
        assert!(model_diff(&al(), &n).is_err());
        let mut n = al();
        n.length = 5.0;
        assert!(model_diff(&al(), &n).is_err());
    }

    #[test]
    fn selection_placeholder() {
        assert_eq!(
            expand_selection("set_start @ 100", Some(0x2A3)).unwrap(),
            "set_start 2A3 100"
        );
        assert_eq!(
            expand_selection("set_name @ \"two words\"", Some(1)).unwrap(),
            "set_name 1 \"two words\""
        );
        assert!(expand_selection("x @", None).is_err());
        assert_eq!(
            expand_selection(" alignments ", None).unwrap(),
            "alignments"
        );
    }

    #[test]
    fn numbers() {
        assert_eq!(fmt_num(100.0), "100");
        assert_eq!(fmt_num(-0.25), "-0.25");
        assert_eq!(fmt_num(0.0), "0");
    }

    fn sample(name: &str) -> Workbench {
        Workbench::open(format!("{}/../samples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn model_ron_round_trip_and_apply() {
        let mut wb = sample("S11_100-200-300_r50.dwg");
        let text = wb.model_ron(0x8C5C).unwrap();
        assert!(wb.model_commands(0x8C5C, &text).unwrap().is_empty());
        let edited = text.replace("\"Alignment - (2)\"", "\"Renamed\"");
        assert_ne!(edited, text);
        wb.apply_model_ron(0x8C5C, &edited).unwrap();
        assert_eq!(
            dump::alignment_of(wb.doc(), 0x8C5C)
                .unwrap()
                .name
                .as_deref(),
            Some("Renamed")
        );
        assert_eq!(wb.undo.len(), 1);
        assert!(wb.changed().contains(&0x8C5C));
        wb.undo();
        assert_eq!(
            dump::alignment_of(wb.doc(), 0x8C5C)
                .unwrap()
                .name
                .as_deref(),
            Some("Alignment - (2)")
        );
        assert!(wb.changed().is_empty());
        wb.redo();
        assert!(wb.changed().contains(&0x8C5C));
    }

    #[test]
    fn model_ron_moves_lines_through_set_pis() {
        let mut wb = sample("S11_100-200-300_r50.dwg");
        let mut a = dump::alignment_of(wb.doc(), 0x8C5C).unwrap();
        // Last tangent 100 long -> 150 long, same direction.
        if let Some(AlignmentElement::Line { end, .. }) = a.elements.last_mut() {
            end.0 += 50.0;
        }
        let text = dump::alignment_ron(&a);
        let cmds = wb.model_commands(0x8C5C, &text).unwrap();
        assert_eq!(cmds.len(), 1, "{cmds:?}");
        assert!(cmds[0].starts_with("set_pis 8C5C "), "{cmds:?}");
        wb.apply_model_ron(0x8C5C, &text).unwrap();
        let after = dump::alignment_of(wb.doc(), 0x8C5C).unwrap();
        assert!(
            (after.length - (a.length + 50.0)).abs() < 1e-6,
            "{}",
            after.length
        );
    }

    #[test]
    fn raw_ron_apply_and_refusals() {
        let mut wb = sample("S11_100-200-300_r50.dwg");
        let text = wb.raw_ron(0x8C5C).unwrap();
        assert_eq!(wb.apply_raw_ron(0x8C5C, &text).unwrap(), "no change");
        assert!(wb.undo.is_empty());
        let mut r = wb.index.records[&0x8C5C].rec.clone();
        r.data[40] ^= 0xFF;
        let edited = dump::record_ron(&r);
        wb.apply_raw_ron(0x8C5C, &edited).unwrap();
        assert_eq!(wb.index.records[&0x8C5C].rec.data, r.data);
        assert_eq!(wb.changed(), vec![0x8C5C]);
        assert!(wb.apply_raw_ron(0x8C5B, &edited).is_err());
        assert!(wb.apply_raw_ron(0x8C5C, "Record(").is_err());
    }

    #[test]
    fn failed_command_changes_nothing() {
        let mut wb = sample("S11_100-200-300_r50.dwg");
        assert!(wb.run("set_length 8C5C 9 1", None).is_err());
        assert!(wb.run("remove 8C5C 99", None).is_err());
        assert!(wb.undo.is_empty() && !wb.dirty && wb.changed().is_empty());
        wb.run("alignments", None).unwrap();
        assert!(wb.undo.is_empty() && !wb.dirty);
        wb.run("set_start @ 100", Some(0x8C5C)).unwrap();
        assert_eq!(wb.undo.len(), 1);
    }

    #[test]
    fn save_as_checks_and_refuses() {
        let mut wb = sample("S11_100-200-300_r50.dwg");
        wb.run("set_length 8C5C 2 150", None).unwrap();
        let open = wb.path.clone();
        assert!(wb.save_as(&open).is_err());
        let s = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/x.dwg");
        assert!(wb.save_as(&s).is_err());
        let out = std::env::temp_dir().join(format!("wb_save_{}.dwg", std::process::id()));
        let msg = wb.save_as(&out).unwrap();
        assert!(msg.contains("all match"), "{msg}");
        assert!(!wb.dirty);
        let back = Workbench::open(&out).unwrap();
        assert_eq!(
            back.index.records[&0x8C5C].rec.data,
            wb.index.records[&0x8C5C].rec.data
        );
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn related_records() {
        let wb = sample("S25_ZERO_CIVIL_second_alignment.dwg");
        let r = wb.related(0xFB8, 1);
        assert_eq!(r[0].relation, Relation::Selected);
        assert!(r.iter().any(|x| x.handle == 0xFA9), "style referenced");
        assert!(r
            .iter()
            .any(|x| matches!(x.relation, Relation::RefersTo { to: 0xFB8 })));
        assert_eq!(
            wb.related(0xFB8, 0)
                .iter()
                .filter(|x| matches!(x.relation, Relation::RefBy { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn external_c3dr() {
        let exe = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/c3dr");
        if !exe.exists() {
            eprintln!("skipped: build c3dr first (cargo build --bin c3dr)");
            return;
        }
        let out = std::process::Command::new(&exe)
            .arg("--spec")
            .output()
            .unwrap();
        let spec: Vec<CommandInfo> = ron::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
        let mut wb = sample("S11_100-200-300_r50.dwg");
        let report = wb.run_external(&exe, &spec, "alignments", None).unwrap();
        assert!(report.contains("8C5C"), "{report}");
        assert!(wb.undo.is_empty());
        wb.run_external(&exe, &spec, "set_length @ 2 150", Some(0x8C5C))
            .unwrap();
        assert_eq!(wb.undo.len(), 1);
        let a = dump::alignment_of(wb.doc(), 0x8C5C).unwrap();
        assert!((a.length - 349.5756).abs() < 1e-3, "{}", a.length);
        assert!(wb
            .run_external(&exe, &spec, "set_length 8C5C 9 1", None)
            .is_err());
        assert_eq!(wb.undo.len(), 1);
    }
}
