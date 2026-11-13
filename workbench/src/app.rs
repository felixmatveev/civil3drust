//! The window (eframe/egui): object list, view, hex pane, RON pane,
//! command form and console around one selection (D-073).

use crate::apidump;
use crate::filemap;
use crate::fileview::FileView;
use crate::hex::{self, HexAction, HexState};
use crate::scene::{opcode_name, station_text, Decoded};
use crate::state::{LogKind, Workbench};
use crate::view::{self, Camera, Canvas, Layers, Mode, Rgba, S2};
use civil3drust::commands::{self, quote, ArgKind, CommandInfo};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Stroke, Vec2};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

pub fn run(path: Option<String>, goto: Option<u64>, compare: Option<String>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("civil3drust workbench")
            .with_inner_size([1500.0, 950.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "civil3drust workbench",
        options,
        Box::new(move |_cc| {
            let mut app = App::default();
            if let Some(p) = path {
                app.open(p);
            }
            app.layers.selected = goto;
            if let (Some(p), Some(b)) = (&app.loading, compare) {
                app.fileview.open(p.0.clone());
                app.fileview.open_compare(b);
                app.central = Central::File;
            }
            Ok(Box::new(app))
        }),
    )
}

#[derive(PartialEq, Clone, Copy, Default)]
enum RightTab {
    #[default]
    Details,
    Ron,
    Form,
    /// The Civil 3D API dump next to the drawing (D-080).
    Api,
}

/// What the central area shows.
#[derive(PartialEq, Clone, Copy, Default)]
enum Central {
    /// The drawing (plan / 3D).
    #[default]
    View,
    /// The file on disk as hex, section by section (hexview, D-075).
    File,
}

#[derive(PartialEq, Clone, Copy, Default)]
enum RonMode {
    #[default]
    Model,
    Raw,
}

/// What a click in the view does.
#[derive(PartialEq, Clone, Copy, Default)]
enum Pick {
    #[default]
    Select,
    /// Put the clicked object's handle into form field i.
    Handle(usize),
    /// Append the clicked point to form field i.
    Points(usize),
}

#[derive(Default)]
struct RonPane {
    mode: RonMode,
    text: String,
    /// (handle, mode, generation) the text was loaded for.
    loaded_for: Option<(u64, RonMode, u64)>,
    edited: bool,
    preview: Option<Result<Vec<String>, String>>,
}

#[derive(Default)]
struct Form {
    command: String,
    values: Vec<String>,
}

#[derive(Default)]
struct External {
    enabled: bool,
    exe: String,
    spec: Option<Vec<CommandInfo>>,
    status: String,
}

#[derive(Default)]
struct App {
    wb: Option<Workbench>,
    loading: Option<(String, Receiver<Result<Workbench, String>>)>,
    filemap_rx: Option<Receiver<Option<filemap::Loaded>>>,
    error: Option<String>,
    /// Bumped on every document change (RON and hex caches key on it).
    generation: u64,
    layers: Layers,
    cam: Camera,
    other_cam: Option<Camera>,
    fit_pending: bool,
    pick_lines: Vec<(u64, Vec<S2>)>,
    hover_world: Option<[f64; 2]>,
    filter: String,
    show_other_records: bool,
    right: RightTab,
    ron: RonPane,
    form: Form,
    external: External,
    pick: Pick,
    hex: HexState,
    cmdline: String,
    history: Vec<String>,
    history_pos: Option<usize>,
    focus_cmdline: bool,
    confirm_discard: bool,
    central: Central,
    fileview: FileView,
    /// API tab: class patterns, text filter, and what the last export did.
    api_classes: String,
    api_text: String,
    api_selected_only: bool,
    api_status: String,
}

impl App {
    fn open(&mut self, path: String) {
        let (tx, rx) = channel();
        let p = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Workbench::open(&p));
        });
        self.loading = Some((path, rx));
        self.error = None;
    }

    fn load_filemap(&mut self) {
        let Some(wb) = &self.wb else { return };
        let path = wb.path.clone();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let loaded = std::panic::catch_unwind(|| filemap::load(&path))
                .ok()
                .and_then(Result::ok)
                .map(|mut l| {
                    l.doc = None;
                    l
                });
            let _ = tx.send(loaded);
        });
        self.filemap_rx = Some(rx);
    }

    fn poll(&mut self, ctx: &egui::Context) {
        if let Some((path, rx)) = &self.loading {
            match rx.try_recv() {
                Ok(Ok(wb)) => {
                    // Keep a selection that exists in the new drawing (reload, --goto).
                    if !self
                        .layers
                        .selected
                        .is_some_and(|h| wb.index.records.contains_key(&h))
                    {
                        self.layers.selected = None;
                    }
                    self.wb = Some(wb);
                    self.layers.hidden_classes.clear();
                    self.other_cam = None;
                    self.fit_pending = true;
                    self.loading = None;
                    self.changed();
                    self.load_filemap();
                }
                Ok(Err(e)) => {
                    self.error = Some(format!("{path}: {e}"));
                    self.loading = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint(),
                Err(_) => {
                    self.error = Some(format!("{path}: loader stopped"));
                    self.loading = None;
                }
            }
        }
        if let Some(rx) = &self.filemap_rx {
            match rx.try_recv() {
                Ok(fm) => {
                    if let Some(wb) = &mut self.wb {
                        if fm.is_none() {
                            wb.log.push((
                                LogKind::Error,
                                "file map failed: parsed records have no bytes in the hex pane"
                                    .into(),
                            ));
                        }
                        wb.filemap = fm;
                    }
                    self.filemap_rx = None;
                    self.generation += 1;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint(),
                Err(_) => self.filemap_rx = None,
            }
        }
    }

    /// The document changed.
    fn changed(&mut self) {
        self.generation += 1;
        self.ron.preview = None;
        self.confirm_discard = false;
        // A deleted selection disappears.
        if let (Some(wb), Some(h)) = (&self.wb, self.layers.selected) {
            if !wb.index.records.contains_key(&h) && wb.file_span(h).is_none() {
                self.layers.selected = None;
            }
        }
    }

    fn run_line(&mut self, line: &str) {
        let line = line.trim().to_string();
        if line.is_empty() {
            return;
        }
        if self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        self.history_pos = None;
        let sel = self.layers.selected;
        let Some(wb) = &mut self.wb else { return };
        let before = wb.undo.len();
        let ext = self
            .external
            .enabled
            .then_some(())
            .and(self.external.spec.as_ref());
        let _ = match ext {
            Some(spec) => wb.run_external(&PathBuf::from(&self.external.exe), spec, &line, sel),
            None => wb.run(&line, sel),
        };
        let edited = wb.undo.len() != before;
        // A command that created an object: select it.
        let created = match wb.log.last() {
            Some((LogKind::Ok, msg)) => msg
                .strip_prefix("created [")
                .and_then(|s| s.split(']').next())
                .and_then(commands::parse_handle),
            _ => None,
        };
        if edited {
            self.changed();
        }
        if created.is_some() {
            self.layers.selected = created;
        }
    }

    /// Point the File tab at the open drawing's file.
    fn sync_fileview(&mut self) {
        if let Some(wb) = &self.wb {
            let p = wb.path.display().to_string();
            if self.fileview.path() != Some(p.as_str()) {
                self.fileview.open(p);
            }
        }
    }

    fn switch_mode(&mut self, mode: Mode) {
        if self.cam.mode == mode {
            return;
        }
        let keep_viewport = self.cam.viewport;
        match self.other_cam.take() {
            Some(mut c) if c.mode == mode => {
                self.other_cam = Some(self.cam);
                c.viewport = keep_viewport;
                self.cam = c;
            }
            _ => {
                self.other_cam = Some(self.cam);
                self.cam.mode = mode;
                self.fit_pending = true;
            }
        }
    }

    fn save_as(&mut self) {
        let Some(wb) = &mut self.wb else { return };
        let stem = wb
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "drawing".into());
        let dir = wb
            .path
            .parent()
            .filter(|d| !crate::state::in_samples(&d.join("x")))
            .map(|d| d.to_path_buf());
        let mut dlg = rfd::FileDialog::new()
            .add_filter("DWG drawing", &["dwg", "DWG"])
            .set_file_name(format!("{stem}_edited.dwg"));
        if let Some(d) = dir {
            dlg = dlg.set_directory(d);
        }
        if let Some(p) = dlg.save_file() {
            if wb.save_as(&p).is_ok() {
                self.load_filemap();
                self.generation += 1;
            }
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("📂 Open…").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("DWG drawing", &["dwg", "DWG"])
                    .pick_file()
                {
                    self.open(p.display().to_string());
                }
            }
            let has = self.wb.is_some();
            if ui
                .add_enabled(has, egui::Button::new("💾 Save As…"))
                .on_hover_text(
                    "Write a copy, read it back and compare (never overwrites the open file)",
                )
                .clicked()
            {
                self.save_as();
            }
            let dirty = self.wb.as_ref().is_some_and(|w| w.dirty);
            let reload_label = if self.confirm_discard {
                "Discard changes?"
            } else {
                "⟳ Reload"
            };
            if ui
                .add_enabled(has, egui::Button::new(reload_label))
                .clicked()
            {
                if dirty && !self.confirm_discard {
                    self.confirm_discard = true;
                } else if let Some(p) = self.wb.as_ref().map(|w| w.path.display().to_string()) {
                    self.confirm_discard = false;
                    self.open(p);
                }
            }
            let (can_undo, can_redo) = self
                .wb
                .as_ref()
                .map_or((false, false), |w| (!w.undo.is_empty(), !w.redo.is_empty()));
            let undo_tip = self
                .wb
                .as_ref()
                .and_then(|w| w.undo.last())
                .map_or(String::new(), |s| s.label.clone());
            if ui
                .add_enabled(can_undo, egui::Button::new("↶ Undo"))
                .on_hover_text(undo_tip)
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(can_redo, egui::Button::new("↷ Redo"))
                .clicked()
            {
                self.redo();
            }
            ui.separator();
            let mut mode = self.cam.mode;
            ui.selectable_value(&mut mode, Mode::Plan, "2D");
            ui.selectable_value(&mut mode, Mode::Orbit, "3D");
            self.switch_mode(mode);
            if ui.button("⛶ Fit").clicked() {
                self.fit_pending = true;
            }
            ui.separator();
            ui.toggle_value(&mut self.layers.decoded, "Decoded");
            ui.toggle_value(&mut self.layers.proxy, "Proxy")
                .on_hover_text("Proxy graphics: Civil 3D's saved display cache");
            ui.toggle_value(&mut self.layers.placeholders, "Placeholders");
            ui.toggle_value(&mut self.layers.context, "AutoCAD");
            ui.toggle_value(&mut self.layers.text, "Text");
            ui.toggle_value(&mut self.layers.grid, "Grid");
            ui.add(
                egui::DragValue::new(&mut self.layers.z_scale)
                    .range(0.1..=100.0)
                    .speed(0.1)
                    .prefix("Z ×"),
            )
            .on_hover_text("Vertical exaggeration in 3D (profiles over their alignments)");
            ui.separator();
            if ui
                .button(if self.layers.dark { "Light" } else { "Dark" })
                .clicked()
            {
                self.layers.dark = !self.layers.dark;
                ui.ctx().set_visuals(if self.layers.dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
            }
        });
    }

    fn undo(&mut self) {
        if let Some(wb) = &mut self.wb {
            if wb.undo().is_some() {
                self.changed();
            }
        }
    }

    fn redo(&mut self) {
        if let Some(wb) = &mut self.wb {
            if wb.redo().is_some() {
                self.changed();
            }
        }
    }

    fn object_list(&mut self, ui: &mut egui::Ui) {
        let Some(wb) = &self.wb else {
            ui.label("No drawing open.");
            ui.small("Open a DWG, drop one on the window, or pass a path on the command line.");
            return;
        };
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text("filter by class, name or handle"),
        );
        ui.add_space(4.0);
        let filter = self.filter.to_lowercase();
        let changed = wb.changed();
        let matches = |class: &str, name: Option<&str>, h: u64| {
            filter.is_empty()
                || class.to_lowercase().contains(&filter)
                || name.unwrap_or("").to_lowercase().contains(&filter)
                || format!("{h:x}").contains(&filter)
        };
        let mut select = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (class, count) in wb.scene.classes() {
                let mut shown = !self.layers.hidden_classes.contains(&class);
                let objs: Vec<_> = wb
                    .scene
                    .objects
                    .iter()
                    .filter(|o| o.class == class && matches(&o.class, o.name.as_deref(), o.handle))
                    .collect();
                if objs.is_empty() {
                    continue;
                }
                let short = class.strip_prefix("AECC_").unwrap_or(&class).to_string();
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut shown, "").changed() {
                        if shown {
                            self.layers.hidden_classes.remove(&class);
                        } else {
                            self.layers.hidden_classes.insert(class.clone());
                        }
                    }
                    ui.strong(format!("{short} ({count})"))
                        .on_hover_text(&class);
                });
                for o in objs {
                    let badge = match (&o.decoded, &o.proxy) {
                        (Decoded::Alignment(_), _)
                        | (Decoded::Profile(_), _)
                        | (Decoded::FeatureLine(_), _) => "◆",
                        (Decoded::Failed(_), _) => "✖",
                        (_, Some(g)) if g.placeholder => "□",
                        (_, Some(_)) => "▣",
                        _ => "·",
                    };
                    let edit = if changed.contains(&o.handle) {
                        " ✎"
                    } else {
                        ""
                    };
                    let label = format!(
                        "   {badge} {:X}  {}{edit}",
                        o.handle,
                        o.name.as_deref().unwrap_or("")
                    );
                    let sel = self.layers.selected == Some(o.handle);
                    if ui.selectable_label(sel, label).clicked() {
                        select = Some(if sel { None } else { Some(o.handle) });
                    }
                }
            }
            ui.add_space(6.0);
            ui.checkbox(
                &mut self.show_other_records,
                "other raw records (styles, sites, …)",
            );
            if self.show_other_records {
                let in_scene: std::collections::HashSet<u64> =
                    wb.scene.objects.iter().map(|o| o.handle).collect();
                let mut by_class: std::collections::BTreeMap<&str, Vec<_>> = Default::default();
                for (h, r) in &wb.index.records {
                    if !in_scene.contains(h) && matches(&r.rec.class, r.name.as_deref(), *h) {
                        by_class.entry(r.rec.class.as_str()).or_default().push(r);
                    }
                }
                for (class, recs) in by_class {
                    egui::CollapsingHeader::new(format!("{class} ({})", recs.len()))
                        .id_salt(class)
                        .show(ui, |ui| {
                            for r in recs {
                                let h = r.rec.handle;
                                let edit = if changed.contains(&h) { " ✎" } else { "" };
                                let sel = self.layers.selected == Some(h);
                                let label =
                                    format!("{h:X}  {}{edit}", r.name.as_deref().unwrap_or(""));
                                if ui.selectable_label(sel, label).clicked() {
                                    select = Some(if sel { None } else { Some(h) });
                                }
                            }
                        });
                }
            }
            ui.add_space(8.0);
            ui.small("◆ decoded  ▣ proxy graphics  □ placeholder  ✖ decode failed  ✎ changed");
        });
        if let Some(s) = select {
            self.layers.selected = s;
        }
    }

    fn details(&self, ui: &mut egui::Ui) {
        let Some(wb) = &self.wb else { return };
        let Some(h) = self.layers.selected else {
            ui.weak("Click an object in the view or the list.");
            return;
        };
        let obj = wb.scene.objects.iter().find(|o| o.handle == h);
        let rec = wb.index.records.get(&h);
        egui::Grid::new("details")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Handle");
                ui.monospace(format!("{h:X}"));
                ui.end_row();
                ui.label("Class");
                ui.monospace(wb.class_of(h));
                ui.end_row();
                if let Some(n) = obj
                    .and_then(|o| o.name.clone())
                    .or(rec.and_then(|r| r.name.clone()))
                {
                    ui.label("Name");
                    ui.label(n);
                    ui.end_row();
                }
                if let Some(r) = rec {
                    ui.label("Record");
                    ui.label(format!(
                        "{} bytes, handle stream {} bits, {}",
                        r.rec.data.len(),
                        r.rec.handle_bits,
                        if r.rec.is_entity { "entity" } else { "object" }
                    ));
                    ui.end_row();
                    ui.label("References");
                    ui.label(format!(
                        "{} out, {} in",
                        r.refs.len(),
                        wb.index.back.get(&h).map_or(0, Vec::len)
                    ));
                    ui.end_row();
                    if wb.changed().contains(&h) {
                        ui.label("Changed");
                        ui.colored_label(Color32::from_rgb(240, 80, 70), "yes, since opened");
                        ui.end_row();
                    }
                }
                if let Some(o) = obj {
                    match &o.decoded {
                        Decoded::Alignment(a) => {
                            ui.label("Decoded");
                            ui.label(format!(
                                "alignment: {} tangent(s), {} arc(s)",
                                a.tangents, a.arcs
                            ));
                            ui.end_row();
                            ui.label("Length");
                            ui.label(format!("{:.4}", a.length));
                            ui.end_row();
                            ui.label("Stations");
                            ui.label(format!(
                                "{} – {}",
                                station_text(a.start_station),
                                station_text(a.start_station + a.length)
                            ));
                            ui.end_row();
                        }
                        Decoded::Profile(p) => {
                            ui.label("Decoded");
                            ui.label(format!(
                                "profile: {} entities, drawn in 3D over alignment {:X}",
                                p.entities, p.alignment
                            ));
                            ui.end_row();
                            ui.label("Stations");
                            ui.label(format!(
                                "{} – {}",
                                station_text(p.stations.0),
                                station_text(p.stations.1)
                            ));
                            ui.end_row();
                            ui.label("Elevations");
                            ui.label(format!("{:.3} – {:.3}", p.elevations.0, p.elevations.1));
                            ui.end_row();
                        }
                        Decoded::FeatureLine(f) => {
                            ui.label("Decoded");
                            ui.label(format!(
                                "feature line: {} vertices ({} elevation points), {} arc(s)",
                                f.vertices, f.elevation_points, f.arcs
                            ));
                            ui.end_row();
                            ui.label("Site");
                            ui.label(f.site.clone().unwrap_or_else(|| "?".into()));
                            ui.end_row();
                            ui.label("Elevations");
                            ui.label(format!("{:.3} – {:.3}", f.elevations.0, f.elevations.1));
                            ui.end_row();
                        }
                        Decoded::Failed(e) => {
                            ui.label("Decoded");
                            ui.colored_label(Color32::LIGHT_RED, e);
                            ui.end_row();
                        }
                        Decoded::None => {
                            ui.label("Decoded");
                            ui.weak("not a class civil3drust decodes yet");
                            ui.end_row();
                        }
                    }
                    ui.label("Proxy graphics");
                    match &o.proxy {
                        None => ui.weak("none found"),
                        Some(g) => ui.label(format!(
                            "{} chunk(s), {} drawn{}",
                            g.chunk_count,
                            g.prims.len(),
                            if g.placeholder { " (placeholder)" } else { "" }
                        )),
                    };
                    ui.end_row();
                    if let Some(g) = &o.proxy {
                        if !g.skipped.is_empty() {
                            ui.label("Not drawn");
                            let s: Vec<String> = g
                                .skipped
                                .iter()
                                .map(|(op, n)| format!("{op} {}×{n}", opcode_name(*op)))
                                .collect();
                            ui.label(s.join(", "));
                            ui.end_row();
                        }
                    }
                }
            });
        ui.add_space(8.0);
        ui.small("Use @ in a command for the selected handle, e.g. set_start @ 1000");
    }

    fn ron_pane(&mut self, ui: &mut egui::Ui) {
        let Some(wb) = &self.wb else { return };
        let Some(h) = self.layers.selected else {
            ui.weak("Select an object.");
            return;
        };
        let has_model = wb.model_ron(h).is_some();
        let has_raw = wb.index.records.contains_key(&h);
        if !has_model && self.ron.mode == RonMode::Model {
            self.ron.mode = RonMode::Raw;
        }
        ui.horizontal(|ui| {
            ui.add_enabled_ui(has_model, |ui| {
                ui.selectable_value(&mut self.ron.mode, RonMode::Model, "Model")
                    .on_hover_text("The decoded alignment; Apply turns changes into commands");
            });
            ui.selectable_value(&mut self.ron.mode, RonMode::Raw, "Raw")
                .on_hover_text("The record bytes; Apply replaces them as typed");
        });
        let key = (h, self.ron.mode, self.generation);
        let stale = self
            .ron
            .loaded_for
            .is_some_and(|k| k.0 == h && k.1 == self.ron.mode && k.2 != self.generation);
        if self.ron.loaded_for != Some(key) && !(self.ron.edited && stale) {
            self.ron.text = match self.ron.mode {
                RonMode::Model => wb.model_ron(h),
                RonMode::Raw => wb.raw_ron(h),
            }
            .unwrap_or_else(|| {
                if has_raw {
                    String::new()
                } else {
                    "// acadrust parses this record; it has no raw bytes to edit here".into()
                }
            });
            self.ron.loaded_for = Some(key);
            self.ron.edited = false;
            self.ron.preview = None;
        }
        if self.ron.edited && stale {
            ui.colored_label(
                Color32::from_rgb(230, 170, 60),
                "The document changed since this text was loaded; Revert to reload.",
            );
        }
        let mut do_apply = false;
        let mut do_revert = false;
        let mut do_preview = false;
        ui.horizontal(|ui| {
            do_apply = ui
                .add_enabled(has_raw && self.ron.edited, egui::Button::new("Apply"))
                .clicked();
            do_revert = ui.button("Revert").clicked();
            if self.ron.mode == RonMode::Model {
                do_preview = ui
                    .button("Preview")
                    .on_hover_text("Show the commands Apply would run")
                    .clicked();
            }
            if self.ron.edited {
                ui.weak("edited");
            }
        });
        if let Some(p) = &self.ron.preview {
            match p {
                Ok(lines) if lines.is_empty() => {
                    ui.weak("no change");
                }
                Ok(lines) => {
                    for l in lines {
                        ui.monospace(l);
                    }
                }
                Err(e) => {
                    ui.colored_label(Color32::LIGHT_RED, e);
                }
            }
        }
        // No wrapping: as wide as the longest line, scrolled sideways.
        let font = FontId::monospace(12.0);
        let char_w = ui.ctx().fonts_mut(|f| f.glyph_width(&font, '0'));
        let longest = self
            .ron
            .text
            .lines()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let width = (longest as f32 + 4.0) * char_w;
        egui::ScrollArea::both()
            .id_salt("ron_text")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let r = ui.add(
                    egui::TextEdit::multiline(&mut self.ron.text)
                        .code_editor()
                        .desired_width(width.max(ui.available_width()))
                        .desired_rows(30),
                );
                if r.changed() {
                    self.ron.edited = true;
                    self.ron.preview = None;
                }
            });
        let Some(wb) = &mut self.wb else { return };
        if do_preview {
            self.ron.preview = Some(wb.model_commands(h, &self.ron.text));
        }
        if do_revert {
            self.ron.loaded_for = None;
            self.ron.edited = false;
        }
        if do_apply {
            let r = match self.ron.mode {
                RonMode::Model => wb.apply_model_ron(h, &self.ron.text),
                RonMode::Raw => wb.apply_raw_ron(h, &self.ron.text),
            };
            match r {
                Ok(_) => {
                    self.ron.edited = false;
                    self.ron.loaded_for = None;
                    self.changed();
                }
                Err(e) => self.ron.preview = Some(Err(e)),
            }
        }
    }

    /// The Civil 3D API dump: the selected object's entry, and a filtered
    /// part of the dump saved to a file.
    fn api_pane(&mut self, ui: &mut egui::Ui) {
        let Some(wb) = &self.wb else { return };
        let Some(d) = &wb.api else {
            ui.weak(
                "No Civil 3D API dump next to this drawing.\n\
                 In Civil 3D: C3DUMP writes <drawing>.api.txt; with C3DCONFIG Auto on, \
                 every save writes one into c3dump\\ next to the drawing.",
            );
            return;
        };
        ui.small(&d.path);
        for h in d.header.iter().filter(|h| h.starts_with("written:")) {
            ui.small(h);
        }
        ui.horizontal(|ui| {
            ui.label("Classes");
            ui.add(
                egui::TextEdit::singleline(&mut self.api_classes)
                    .hint_text("AECC_ALIGNMENT,AECC_PROFILE*")
                    .desired_width(220.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Text");
            ui.add(egui::TextEdit::singleline(&mut self.api_text).desired_width(140.0));
            ui.checkbox(&mut self.api_selected_only, "selected only");
        });
        let handle = match (self.api_selected_only, self.layers.selected) {
            (true, Some(h)) => format!("{h:X}"),
            _ => String::new(),
        };
        let filter = apidump::Filter::new(&self.api_classes, &handle, &self.api_text);
        let mut export = false;
        ui.horizontal(|ui| match &filter {
            Ok(f) => {
                let n = d
                    .objects
                    .values()
                    .filter(|o| f.passes(o.handle, &o.class, &o.title))
                    .count();
                export = ui
                    .button("Save filtered…")
                    .on_hover_text("Write the matching objects to a file, in the dump's format")
                    .clicked();
                ui.weak(format!("{n} of {} objects", d.objects.len()));
            }
            Err(e) => {
                ui.colored_label(Color32::LIGHT_RED, e);
            }
        });
        if !self.api_status.is_empty() {
            ui.small(&self.api_status);
        }
        ui.separator();
        let text = match self.layers.selected {
            None => "Select an object to see its entry.".to_string(),
            Some(h) => match d.objects.get(&h) {
                None => format!("{h:X} is not in the dump (class not dumped, or created since)."),
                Some(o) => {
                    let mut t = format!("{}\n", o.title);
                    for (k, v) in &o.props {
                        t.push_str(&format!("  {k}: {v}\n"));
                    }
                    // What was listed under it (an alignment's profiles).
                    for c in d.objects.values().filter(|c| c.parent == Some(h)) {
                        t.push_str(&format!("\n{}  ({} properties)\n", c.title, c.props.len()));
                    }
                    t
                }
            },
        };
        egui::ScrollArea::both()
            .id_salt("api_text")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(text).monospace().size(12.0))
                        .wrap_mode(egui::TextWrapMode::Extend)
                        .selectable(true),
                );
            });
        if let (true, Ok(f)) = (export, &filter) {
            let stem = wb
                .path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "drawing".into());
            let mut dlg = rfd::FileDialog::new()
                .add_filter("API dump", &["txt"])
                .set_file_name(format!("{stem}.filtered.api.txt"));
            if let Some(dir) = wb.path.parent() {
                dlg = dlg.set_directory(dir);
            }
            if let Some(p) = dlg.save_file() {
                self.api_status = match std::fs::write(&p, apidump::export(d, f)) {
                    Ok(()) => format!("written: {}", p.display()),
                    Err(e) => format!("failed to write {}: {e}", p.display()),
                };
            }
        }
    }

    fn spec(&self) -> Vec<CommandInfo> {
        match (&self.external.spec, self.external.enabled) {
            (Some(s), true) => s.clone(),
            _ => commands::spec(),
        }
    }

    fn form_pane(&mut self, ui: &mut egui::Ui) {
        let spec = self.spec();
        if !spec.iter().any(|c| c.name == self.form.command) {
            self.form.command = spec
                .iter()
                .find(|c| c.edits)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            self.form.values.clear();
        }
        egui::ComboBox::from_id_salt("form_cmd")
            .selected_text(self.form.command.clone())
            .width(220.0)
            .show_ui(ui, |ui| {
                for c in &spec {
                    if ui
                        .selectable_label(c.name == self.form.command, &c.name)
                        .on_hover_text(&c.help)
                        .clicked()
                    {
                        self.form.command = c.name.clone();
                        self.form.values.clear();
                        self.pick = Pick::Select;
                    }
                }
            });
        let Some(cmd) = spec.iter().find(|c| c.name == self.form.command).cloned() else {
            return;
        };
        ui.label(&cmd.help);
        self.form.values.resize(cmd.args.len(), String::new());
        let sel = self.layers.selected;
        egui::Grid::new("form")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for (i, a) in cmd.args.iter().enumerate() {
                    let label = if a.optional {
                        format!("{} (optional)", a.name)
                    } else {
                        a.name.clone()
                    };
                    ui.label(label).on_hover_text(&a.help);
                    ui.horizontal(|ui| {
                        if a.kind == ArgKind::Flag {
                            let mut on = !self.form.values[i].is_empty();
                            if ui.checkbox(&mut on, &a.help).changed() {
                                self.form.values[i] = if on { "1".into() } else { String::new() };
                            }
                            return;
                        }
                        let v = &mut self.form.values[i];
                        if v.is_empty() && a.kind == ArgKind::Alignment && i == 0 {
                            if let Some(h) = sel {
                                *v = format!("{h:X}");
                            }
                        }
                        ui.add(
                            egui::TextEdit::singleline(v)
                                .hint_text(&a.help)
                                .desired_width(200.0),
                        );
                        match a.kind {
                            ArgKind::Alignment | ArgKind::AlignmentStyle | ArgKind::Handle => {
                                let on = self.pick == Pick::Handle(i);
                                if ui
                                    .selectable_label(on, "pick")
                                    .on_hover_text("click an object in the view")
                                    .clicked()
                                {
                                    self.pick = if on { Pick::Select } else { Pick::Handle(i) };
                                }
                                if let Some(h) = sel {
                                    if ui
                                        .small_button("sel")
                                        .on_hover_text("use the selection")
                                        .clicked()
                                    {
                                        self.form.values[i] = format!("{h:X}");
                                    }
                                }
                            }
                            ArgKind::Points | ArgKind::PointsRadii => {
                                let on = self.pick == Pick::Points(i);
                                if ui
                                    .selectable_label(on, "pick")
                                    .on_hover_text(
                                        "click points in the plan view; click again to stop",
                                    )
                                    .clicked()
                                {
                                    self.pick = if on { Pick::Select } else { Pick::Points(i) };
                                }
                                if ui.small_button("clear").clicked() {
                                    self.form.values[i].clear();
                                }
                            }
                            ArgKind::File if ui.small_button("…").clicked() => {
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("DWG drawing", &["dwg", "DWG"])
                                    .pick_file()
                                {
                                    self.form.values[i] = p.display().to_string();
                                }
                            }
                            _ => {}
                        }
                    });
                    ui.end_row();
                }
            });
        let line = build_line(&cmd, &self.form.values);
        ui.add_space(4.0);
        ui.monospace(&line);
        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                self.pick = Pick::Select;
                self.run_line(&line);
            }
            if ui.button("To command line").clicked() {
                self.cmdline = line.clone();
                self.focus_cmdline = true;
            }
        });
        ui.separator();
        ui.collapsing("External c3dr (newer library build)", |ui| {
            ui.small("Runs commands through another c3dr binary on a temporary copy of the drawing, so new library commands work without rebuilding the workbench.");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.external.exe).hint_text("path to c3dr").desired_width(220.0));
                if ui.small_button("…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                        self.external.exe = p.display().to_string();
                    }
                }
            });
            if ui.button("Load its command list").clicked() {
                self.load_external_spec();
            }
            ui.add_enabled_ui(self.external.spec.is_some(), |ui| {
                ui.checkbox(&mut self.external.enabled, "run commands with it");
            });
            if !self.external.status.is_empty() {
                ui.small(&self.external.status);
            }
        });
    }

    fn load_external_spec(&mut self) {
        let out = std::process::Command::new(&self.external.exe)
            .arg("--spec")
            .output();
        match out {
            Ok(o) if o.status.success() => {
                match ron::from_str::<Vec<CommandInfo>>(&String::from_utf8_lossy(&o.stdout)) {
                    Ok(s) => {
                        self.external.status = format!("{} commands", s.len());
                        self.external.spec = Some(s);
                    }
                    Err(e) => self.external.status = format!("can't read its spec: {e}"),
                }
            }
            Ok(o) => self.external.status = String::from_utf8_lossy(&o.stderr).into_owned(),
            Err(e) => self.external.status = format!("can't run it: {e}"),
        }
    }

    fn console(&mut self, ui: &mut egui::Ui) {
        let h = ui.available_height() - 30.0;
        egui::ScrollArea::vertical()
            .id_salt("log")
            .max_height(h.max(40.0))
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if let Some(wb) = &self.wb {
                    for (k, line) in &wb.log {
                        let (prefix, color) = match k {
                            LogKind::Command => ("> ", ui.visuals().strong_text_color()),
                            LogKind::Ok => ("  ", ui.visuals().text_color()),
                            LogKind::Error => ("! ", Color32::from_rgb(240, 90, 80)),
                            LogKind::Info => ("# ", ui.visuals().weak_text_color()),
                        };
                        for l in line.lines() {
                            ui.label(
                                egui::RichText::new(format!("{prefix}{l}"))
                                    .monospace()
                                    .color(color),
                            );
                        }
                    }
                }
            });
        ui.horizontal(|ui| {
            ui.monospace(if self.external.enabled { "c3dr ext>" } else { ">" });
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.cmdline)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("command (help lists them; @ = selection; Ctrl+click in view inserts x,y; Alt+click inserts a handle)")
                    .desired_width(f32::INFINITY),
            );
            if self.focus_cmdline {
                r.request_focus();
                self.focus_cmdline = false;
            }
            if r.has_focus() {
                let (up, down) = ui.input(|i| (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown)));
                if up && !self.history.is_empty() {
                    let p = self.history_pos.map_or(self.history.len() - 1, |p| p.saturating_sub(1));
                    self.history_pos = Some(p);
                    self.cmdline = self.history[p].clone();
                }
                if down {
                    if let Some(p) = self.history_pos {
                        if p + 1 < self.history.len() {
                            self.history_pos = Some(p + 1);
                            self.cmdline = self.history[p + 1].clone();
                        } else {
                            self.history_pos = None;
                            self.cmdline.clear();
                        }
                    }
                }
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let line = std::mem::take(&mut self.cmdline);
                self.run_line(&line);
                self.focus_cmdline = true;
            }
        });
    }

    fn viewport(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = resp.rect;
        self.cam.viewport = [rect.min.x, rect.min.y, rect.width(), rect.height()];
        let pal = view::Palette::new(self.layers.dark);
        painter.rect_filled(rect, 0.0, c32(pal.bg));

        let Some(wb) = &self.wb else {
            let msg = match (&self.loading, &self.error) {
                (Some((p, _)), _) => format!("Reading {p} …"),
                (_, Some(e)) => format!("Could not open: {e}"),
                _ => "Drop a .dwg here".to_string(),
            };
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                msg,
                FontId::proportional(18.0),
                c32(pal.context).gamma_multiply(1.6),
            );
            return;
        };
        if self.fit_pending {
            self.cam.fit(view::bounds(&wb.scene, &self.layers));
            self.fit_pending = false;
        }
        let d = resp.drag_delta();
        if d != Vec2::ZERO {
            let pan = resp.dragged_by(egui::PointerButton::Middle)
                || resp.dragged_by(egui::PointerButton::Secondary)
                || ui.input(|i| i.modifiers.shift);
            match (self.cam.mode, pan) {
                (Mode::Orbit, false) => self.cam.orbit(d.x, d.y),
                _ => self.cam.pan(d.x, d.y),
            }
        }
        if resp.hovered() {
            let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = (scroll as f64 / 300.0).exp() * zoom as f64;
            if (factor - 1.0).abs() > 1e-6 {
                let at = resp.hover_pos().map(|p| [p.x, p.y]);
                self.cam.zoom_at(factor, at);
            }
        }
        if resp.double_clicked() {
            self.fit_pending = true;
        }
        let mut canvas = PainterCanvas {
            painter: painter.with_clip_rect(rect),
        };
        self.pick_lines = view::draw(&wb.scene, &self.layers, &self.cam, &mut canvas);
        self.hover_world = match (self.cam.mode, resp.hover_pos()) {
            (Mode::Plan, Some(p)) => Some(self.cam.unproject_plan([p.x, p.y])),
            _ => None,
        };
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                let hit = view::pick(&self.pick_lines, [p.x, p.y], 6.0);
                let world =
                    (self.cam.mode == Mode::Plan).then(|| self.cam.unproject_plan([p.x, p.y]));
                let mods = ui.input(|i| i.modifiers);
                let pt = |w: [f64; 2]| format!("{:.3},{:.3}", w[0], w[1]);
                match self.pick {
                    Pick::Handle(i) => {
                        if let Some(h) = hit {
                            if let Some(v) = self.form.values.get_mut(i) {
                                *v = format!("{h:X}");
                            }
                            self.pick = Pick::Select;
                        }
                    }
                    Pick::Points(i) => {
                        if let (Some(w), Some(v)) = (world, self.form.values.get_mut(i)) {
                            if !v.is_empty() {
                                v.push(' ');
                            }
                            v.push_str(&pt(w));
                        }
                    }
                    Pick::Select if mods.command => {
                        if let Some(w) = world {
                            if !self.cmdline.is_empty() && !self.cmdline.ends_with(' ') {
                                self.cmdline.push(' ');
                            }
                            self.cmdline.push_str(&pt(w));
                            self.focus_cmdline = true;
                        }
                    }
                    Pick::Select if mods.alt => {
                        if let Some(h) = hit {
                            if !self.cmdline.is_empty() && !self.cmdline.ends_with(' ') {
                                self.cmdline.push(' ');
                            }
                            self.cmdline.push_str(&format!("{h:X}"));
                            self.focus_cmdline = true;
                        }
                    }
                    Pick::Select => self.layers.selected = hit,
                }
            }
        }
        if self.cam.mode == Mode::Orbit {
            draw_axes(&painter, &self.cam, rect);
        }
        if self.pick != Pick::Select {
            let msg = match self.pick {
                Pick::Handle(_) => "pick an object for the form",
                _ => "click points for the form (toggle 'pick' to stop)",
            };
            painter.text(
                rect.left_top() + Vec2::new(10.0, 10.0),
                Align2::LEFT_TOP,
                msg,
                FontId::proportional(14.0),
                c32(pal.selected),
            );
        }
        if let Some(p) = resp.hover_pos() {
            if let Some(h) = view::pick(&self.pick_lines, [p.x, p.y], 6.0) {
                if let Some(o) = wb.scene.objects.iter().find(|o| o.handle == h) {
                    let text = match &o.name {
                        Some(n) => format!("{:X} {} — {n}", o.handle, o.class),
                        None => format!("{:X} {}", o.handle, o.class),
                    };
                    painter.text(
                        p + Vec2::new(14.0, -10.0),
                        Align2::LEFT_BOTTOM,
                        text,
                        FontId::proportional(13.0),
                        c32(pal.selected),
                    );
                }
            }
        }
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(wb) = &self.wb {
                ui.label(wb.path.display().to_string());
                if wb.dirty {
                    ui.colored_label(Color32::from_rgb(230, 170, 60), "● modified");
                }
                if wb.opened.failsafe_reason.is_some() {
                    ui.colored_label(Color32::from_rgb(230, 170, 60), "⚠ failsafe read")
                        .on_hover_text(wb.opened.failsafe_reason.clone().unwrap_or_default());
                }
                ui.separator();
                ui.label(format!(
                    "{} raw records, {} changed",
                    wb.index.records.len(),
                    wb.changed().len()
                ));
            }
            if let Some(e) = &self.error {
                ui.colored_label(Color32::LIGHT_RED, e);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match self.hover_world {
                    Some([x, y]) => ui.monospace(format!("X {x:.3}  Y {y:.3}")),
                    None => ui.weak(
                        "drag: pan/orbit · wheel: zoom · dbl-click: fit · Ctrl+Z/Y: undo/redo",
                    ),
                };
            });
        });
    }
}

/// The command line for a form: positionals in order (stopping at the
/// first empty optional), then options.
fn build_line(cmd: &CommandInfo, values: &[String]) -> String {
    let mut parts = vec![cmd.name.clone()];
    let mut stop = false;
    for (a, v) in cmd.args.iter().zip(values) {
        if a.name.starts_with("--") {
            continue;
        }
        if v.is_empty() {
            if a.optional {
                stop = true;
                continue;
            }
            parts.push(format!("<{}>", a.name));
            continue;
        }
        if !stop {
            parts.push(quote(v));
        }
    }
    for (a, v) in cmd.args.iter().zip(values) {
        if !a.name.starts_with("--") || v.is_empty() {
            continue;
        }
        parts.push(a.name.clone());
        if a.kind != ArgKind::Flag {
            parts.push(quote(v));
        }
    }
    parts.join(" ")
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.open(p.display().to_string());
        }
        let typing = ctx.egui_wants_keyboard_input();
        if !typing {
            let (f, z, y, shift) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::F),
                    i.modifiers.command && i.key_pressed(egui::Key::Z),
                    i.modifiers.command && i.key_pressed(egui::Key::Y),
                    i.modifiers.shift,
                )
            });
            if f {
                self.fit_pending = true;
            }
            if z && shift || y {
                self.redo();
            } else if z {
                self.undo();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.pick != Pick::Select {
                self.pick = Pick::Select;
            } else {
                self.layers.selected = None;
            }
        }
        let title = match &self.wb {
            Some(wb) => format!(
                "{}{} — civil3drust workbench",
                if wb.dirty { "* " } else { "" },
                wb.path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
            None => "civil3drust workbench".into(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(3.0);
            self.top_bar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::bottom("console")
            .resizable(true)
            .default_size(170.0)
            .min_size(80.0)
            .show(ui, |ui| {
                ui.add_space(2.0);
                self.console(ui);
            });
        egui::Panel::bottom("hex")
            .resizable(true)
            .default_size(320.0)
            .min_size(200.0)
            .show(ui, |ui| {
                ui.add_space(2.0);
                if let Some(wb) = &self.wb {
                    match hex::ui(ui, &mut self.hex, wb, self.layers.selected, self.generation) {
                        Some(HexAction::Select(h)) => {
                            self.layers.selected = Some(h);
                            self.hex.cursor = None;
                        }
                        Some(HexAction::ShowInFile(h)) => {
                            self.central = Central::File;
                            self.sync_fileview();
                            self.fileview.goto_handle(h);
                        }
                        None => {}
                    }
                } else {
                    ui.weak("Hex: open a drawing.");
                }
            });
        egui::Panel::left("objects")
            .resizable(true)
            .default_size(280.0)
            .min_size(200.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.heading("Objects");
                self.object_list(ui);
            });
        egui::Panel::right("right")
            .resizable(true)
            .default_size(470.0)
            .min_size(280.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.right, RightTab::Details, "Details");
                    ui.selectable_value(&mut self.right, RightTab::Ron, "RON");
                    ui.selectable_value(&mut self.right, RightTab::Form, "Command form");
                    ui.selectable_value(&mut self.right, RightTab::Api, "API")
                        .on_hover_text("What Civil 3D's API reports for the object (C3DUMP)");
                });
                ui.separator();
                match self.right {
                    RightTab::Details => self.details(ui),
                    RightTab::Ron => self.ron_pane(ui),
                    RightTab::Api => self.api_pane(ui),
                    RightTab::Form => {
                        egui::ScrollArea::vertical()
                            .id_salt("form_scroll")
                            .show(ui, |ui| self.form_pane(ui));
                    }
                }
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    ui.selectable_value(&mut self.central, Central::View, "Drawing");
                    ui.selectable_value(&mut self.central, Central::File, "File (hex)")
                        .on_hover_text("The saved file section by section, with record outlines and comparison");
                    if self.central == Central::File {
                        if let Some(wb) = &self.wb {
                            if wb.dirty {
                                ui.weak("shows the file on disk; unsaved edits are not in it");
                            }
                        }
                    }
                });
                match self.central {
                    Central::View => self.viewport(ui),
                    Central::File => {
                        self.sync_fileview();
                        self.fileview.ui(ui);
                    }
                }
            });
    }
}

fn c32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0, c.1, c.2, c.3)
}

struct PainterCanvas {
    painter: egui::Painter,
}

impl Canvas for PainterCanvas {
    fn polyline(&mut self, pts: &[S2], closed: bool, color: Rgba, width: f32) {
        let mut v: Vec<Pos2> = pts.iter().map(|p| Pos2::new(p[0], p[1])).collect();
        if closed {
            v.push(v[0]);
        }
        self.painter.line(v, Stroke::new(width, c32(color)));
    }

    fn marker(&mut self, at: S2, color: Rgba, size: f32) {
        self.painter
            .circle_stroke(Pos2::new(at[0], at[1]), size, Stroke::new(1.3, c32(color)));
    }

    fn text(&mut self, at: S2, angle: f32, size: f32, text: &str, color: Rgba) {
        let galley =
            self.painter
                .layout_no_wrap(text.to_string(), FontId::proportional(size), c32(color));
        let up = Vec2::new(angle.sin(), -angle.cos()) * (size * 0.95);
        let pos = Pos2::new(at[0], at[1]) + up;
        self.painter
            .add(egui::epaint::TextShape::new(pos, galley, c32(color)).with_angle(angle));
    }
}

/// Small XYZ triad in the corner of the 3D view.
fn draw_axes(painter: &egui::Painter, cam: &Camera, rect: egui::Rect) {
    let mut c = *cam;
    c.perspective = false;
    c.target = [0.0; 3];
    c.px_per_unit = 1.0;
    let origin = rect.left_bottom() + Vec2::new(46.0, -46.0);
    c.viewport = [origin.x - 50.0, origin.y - 50.0, 100.0, 100.0];
    for (v, col, name) in [
        ([30.0, 0.0, 0.0], Color32::from_rgb(230, 80, 70), "X"),
        ([0.0, 30.0, 0.0], Color32::from_rgb(90, 200, 90), "Y"),
        ([0.0, 0.0, 30.0], Color32::from_rgb(90, 140, 240), "Z"),
    ] {
        if let (Some(a), Some(b)) = (c.project([0.0; 3]), c.project(v)) {
            let (a, b) = (Pos2::new(a[0], a[1]), Pos2::new(b[0], b[1]));
            painter.line_segment([a, b], Stroke::new(2.0, col));
            painter.text(b, Align2::CENTER_CENTER, name, FontId::monospace(11.0), col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_line() {
        let spec = commands::spec();
        let c = spec.iter().find(|c| c.name == "set_start").unwrap();
        let v = vec!["2A3".to_string(), "100".into(), "1".into()];
        assert_eq!(build_line(c, &v), "set_start 2A3 100 --clear-speeds");
        let c = spec.iter().find(|c| c.name == "set_name").unwrap();
        let v = vec!["A".to_string(), "two words".into(), String::new()];
        assert_eq!(build_line(c, &v), "set_name A \"two words\"");
        let v = vec!["A".to_string(), String::new(), String::new()];
        assert_eq!(build_line(c, &v), "set_name A <name>");
    }
}
