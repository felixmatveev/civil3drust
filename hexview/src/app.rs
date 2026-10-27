//! The window: offset | hex | decoded columns, a record list, an inspector
//! and per-record detail windows.

use crate::model::{self, Loaded, Origin, Pane, Span};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};

const ROW: usize = 16;

pub fn run(path: Option<String>, goto: Option<String>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("civil3drust hexview")
            .with_inner_size([1400.0, 860.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "civil3drust hexview",
        options,
        Box::new(move |_cc| {
            let mut app = App::default();
            if let Some(p) = path {
                app.open(p);
            }
            app.pending_goto = goto;
            Ok(Box::new(app))
        }),
    )
}

pub fn color(origin: Origin, dark: bool) -> Color32 {
    match (origin, dark) {
        (Origin::Acad, true) => Color32::from_rgb(110, 165, 255),
        (Origin::Acad, false) => Color32::from_rgb(20, 80, 200),
        (Origin::Civil, true) => Color32::from_rgb(255, 105, 100),
        (Origin::Civil, false) => Color32::from_rgb(200, 30, 30),
        (Origin::Opaque, true) => Color32::from_rgb(230, 170, 70),
        (Origin::Opaque, false) => Color32::from_rgb(170, 110, 0),
        (Origin::Unmapped, _) => Color32::from_gray(140),
    }
}

/// Something a click asked for; applied after drawing.
enum Action {
    Cursor(usize),
    Jump(usize),
    Open(usize),
    Handle(u64),
}

#[derive(Default)]
struct App {
    loaded: Option<Loaded>,
    loading: Option<(String, Receiver<Result<Loaded, String>>)>,
    error: Option<String>,
    pane: usize,
    cursor: Option<usize>,
    selected: Option<usize>,
    scroll_to: Option<usize>,
    /// Open detail windows: (pane, span index).
    details: Vec<(usize, usize)>,
    dumps: HashMap<u64, String>,
    goto: String,
    filter: String,
    actions: Vec<Action>,
    /// `--goto` from the command line, applied once the file is loaded.
    pending_goto: Option<String>,
}

impl App {
    fn open(&mut self, path: String) {
        let (tx, rx) = channel();
        let p = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(model::load(&p));
        });
        self.loading = Some((path, rx));
        self.error = None;
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let Some((path, rx)) = &self.loading else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(l)) => {
                self.loaded = Some(l);
                self.pane = 0;
                self.cursor = None;
                self.selected = None;
                self.details.clear();
                self.dumps.clear();
                self.loading = None;
                if let Some(g) = self.pending_goto.take() {
                    self.goto = g;
                    self.do_goto();
                }
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

    fn pane(&self) -> Option<&Pane> {
        self.loaded.as_ref().and_then(|l| l.panes.get(self.pane))
    }

    fn apply_actions(&mut self) {
        for a in std::mem::take(&mut self.actions) {
            match a {
                Action::Cursor(o) => self.cursor = Some(o),
                Action::Jump(o) => {
                    self.cursor = Some(o);
                    self.scroll_to = Some(o);
                }
                Action::Open(i) => self.open_span(i),
                Action::Handle(h) => {
                    if let Some(i) = self.pane().and_then(|p| p.find_handle(h)) {
                        let start = self.pane().unwrap().spans[i].start;
                        self.cursor = Some(start);
                        self.scroll_to = Some(start);
                        self.open_span(i);
                    }
                }
            }
        }
    }

    fn open_span(&mut self, i: usize) {
        self.selected = Some(i);
        let key = (self.pane, i);
        if !self.details.contains(&key) {
            self.details.push(key);
        }
    }

    fn do_goto(&mut self) {
        let t = self.goto.trim().to_string();
        if let Some(h) = t.strip_prefix('#') {
            if let Ok(h) = u64::from_str_radix(h, 16) {
                self.actions.push(Action::Handle(h));
            }
        } else if let Ok(o) = usize::from_str_radix(t.trim_start_matches("0x"), 16) {
            self.actions.push(Action::Jump(o));
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let mut goto_now = false;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Open…").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Drawing", &["dwg", "DWG", "dxf", "DXF"])
                    .add_filter("Any", &["*"])
                    .pick_file()
                {
                    self.open(p.display().to_string());
                }
            }
            if let Some(l) = &self.loaded {
                ui.label(format!("{}  [{}]", l.path, l.version));
                ui.separator();
                let titles: Vec<String> = l
                    .panes
                    .iter()
                    .map(|p| format!("{} ({} B)", p.title, p.bytes.len()))
                    .collect();
                let mut pane = self.pane;
                egui::ComboBox::from_id_salt("pane")
                    .selected_text(titles.get(pane).cloned().unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for (i, t) in titles.iter().enumerate() {
                            ui.selectable_value(&mut pane, i, t);
                        }
                    });
                if pane != self.pane {
                    self.pane = pane;
                    self.cursor = None;
                    self.selected = None;
                    self.scroll_to = Some(0);
                }
                ui.separator();
                ui.label("Go to");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.goto)
                        .desired_width(110.0)
                        .hint_text("hex offset / #handle"),
                );
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    goto_now = true;
                }
                ui.separator();
                let dark = ui.visuals().dark_mode;
                if let Some(p) = l.panes.get(self.pane) {
                    for o in [
                        Origin::Acad,
                        Origin::Civil,
                        Origin::Opaque,
                        Origin::Unmapped,
                    ] {
                        let n = p.spans.iter().filter(|s| s.origin == o).count();
                        if n > 0 {
                            let name = match o {
                                Origin::Acad => "acadrust",
                                Origin::Civil => "civil3drust",
                                Origin::Opaque => "opaque",
                                Origin::Unmapped => "unmapped",
                            };
                            ui.colored_label(color(o, dark), format!("■ {name} {n}"))
                                .on_hover_text(o.describe());
                        }
                    }
                }
            } else if self.loading.is_none() {
                ui.label("Open or drop a DWG file.");
            }
            if let Some((p, _)) = &self.loading {
                ui.spinner();
                ui.label(format!("loading {p}"));
            }
        });
        if goto_now {
            self.do_goto();
        }
    }

    fn record_list(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text("filter class / name / handle")
                .desired_width(f32::INFINITY),
        );
        let f = self.filter.to_ascii_lowercase();
        let Some(p) = self.pane() else { return };
        let hits: Vec<usize> = p
            .spans
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                f.is_empty()
                    || s.label().to_ascii_lowercase().contains(&f)
                    || format!("{:x}", s.handle).contains(f.trim_start_matches('#'))
            })
            .map(|(i, _)| i)
            .collect();
        ui.label(format!("{} records", hits.len()));
        let dark = ui.visuals().dark_mode;
        let row_h = ui.text_style_height(&egui::TextStyle::Body) + 2.0;
        let mut acts = Vec::new();
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_h, hits.len(), |ui, range| {
                for &i in &hits[range] {
                    let s = &p.spans[i];
                    let text = egui::RichText::new(format!("{:>8X}  {}", s.start, s.label()))
                        .monospace()
                        .color(color(s.origin, dark));
                    let r = ui.selectable_label(self.selected == Some(i), text);
                    if r.clicked() {
                        acts.push(Action::Jump(s.start));
                        acts.push(Action::Open(i));
                    }
                }
            });
        self.actions.extend(acts);
    }

    fn inspector(&self, ui: &mut egui::Ui) {
        let (Some(p), Some(c)) = (self.pane(), self.cursor) else {
            ui.label("Click a byte to inspect it.");
            return;
        };
        let b = &p.bytes;
        let get = |n: usize| -> Option<Vec<u8>> { b.get(c..c + n).map(<[u8]>::to_vec) };
        let mut parts = vec![format!("@0x{c:X}")];
        if let Some(v) = get(1) {
            parts.push(format!("u8 {}  i8 {}  bits {:08b}", v[0], v[0] as i8, v[0]));
        }
        if let Some(v) = get(2) {
            let a = [v[0], v[1]];
            parts.push(format!(
                "u16 {}  i16 {}",
                u16::from_le_bytes(a),
                i16::from_le_bytes(a)
            ));
        }
        if let Some(v) = get(4) {
            let a = [v[0], v[1], v[2], v[3]];
            parts.push(format!(
                "u32 {}  i32 {}  f32 {}",
                u32::from_le_bytes(a),
                i32::from_le_bytes(a),
                num(f32::from_le_bytes(a) as f64)
            ));
        }
        if let Some(v) = get(8) {
            let a: [u8; 8] = v.try_into().unwrap();
            parts.push(format!(
                "i64 {}  f64 {}",
                i64::from_le_bytes(a),
                num(f64::from_le_bytes(a))
            ));
        }
        ui.horizontal_wrapped(|ui| {
            ui.monospace(parts.join("   "));
        });
        if let Some(i) = p.span_at(c) {
            let s = &p.spans[i];
            let where_ = if c < s.data_start {
                "size field".to_string()
            } else if c >= s.data_start + s.data_len {
                "CRC".to_string()
            } else {
                let k = c - s.data_start;
                format!(
                    "data byte {k}, bit {} (civil3drust bit offsets are relative to the data start)",
                    k * 8
                )
            };
            ui.monospace(format!("in {}  —  {where_}", s.label()));
        }
    }

    fn hex_view(&mut self, ui: &mut egui::Ui) {
        let Some(l) = &self.loaded else {
            if let Some(e) = &self.error {
                ui.colored_label(Color32::RED, e);
            }
            return;
        };
        let Some(p) = l.panes.get(self.pane) else {
            return;
        };
        let font = FontId::monospace(13.0);
        let cw = ui.ctx().fonts_mut(|f| f.glyph_width(&font, '0'));
        let row_h = (font.size * 1.35).round();
        let dark = ui.visuals().dark_mode;
        let text_col = ui.visuals().text_color();
        let faint = ui.visuals().weak_text_color();
        let x_hex = cw * 10.0;
        let x_txt = x_hex + cw * (ROW as f32 * 3.0 + 2.0);
        let width = x_txt + cw * (ROW as f32 + 1.0);
        let rows = p.bytes.len().div_ceil(ROW);

        // show_rows strides rows by row height + item spacing; keep it exact.
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut area = egui::ScrollArea::both()
            .auto_shrink([false, false])
            .id_salt(("hex", self.pane));
        if let Some(o) = self.scroll_to.take() {
            let y = (o / ROW) as f32 * row_h - 4.0 * row_h;
            area = area.vertical_scroll_offset(y.max(0.0));
        }
        let mut acts = Vec::new();
        let (cursor, selected) = (self.cursor, self.selected);
        area.show_rows(ui, row_h, rows, |ui, range| {
            // label chars per span, built once per frame for visible spans
            let mut labels: HashMap<usize, Vec<char>> = HashMap::new();
            for row in range {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::click());
                let painter = ui.painter_at(rect);
                let base = row * ROW;
                let at = |x: f32| Pos2::new(rect.left() + x, rect.top() + 1.0);
                painter.text(
                    at(0.0),
                    Align2::LEFT_TOP,
                    format!("{base:08X}"),
                    font.clone(),
                    faint,
                );
                let hex_x =
                    |i: usize| x_hex + cw * (i as f32 * 3.0 + if i >= 8 { 1.0 } else { 0.0 });
                for i in 0..ROW {
                    let off = base + i;
                    let Some(&b) = p.bytes.get(off) else { break };
                    let span_i = p.span_at(off);
                    let span = span_i.map(|k| &p.spans[k]);
                    let hx = hex_x(i);
                    let tx = x_txt + cw * i as f32;
                    // background tint
                    if let Some(s) = span {
                        let mut c = color(s.origin, dark);
                        let framing = off < s.data_start || off >= s.data_start + s.data_len;
                        let alpha = if span_i == selected {
                            90
                        } else if framing {
                            60
                        } else {
                            28
                        };
                        c = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha);
                        let pad = if i == 7 { cw * 2.0 } else { cw * 3.0 };
                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(rect.left() + hx - cw * 0.5, rect.top()),
                                Vec2::new(pad, row_h),
                            ),
                            0.0,
                            c,
                        );
                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(rect.left() + tx, rect.top()),
                                Vec2::new(cw, row_h),
                            ),
                            0.0,
                            c,
                        );
                        if off == s.start {
                            let x = rect.left() + tx;
                            painter.line_segment(
                                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                                Stroke::new(1.5, color(s.origin, dark)),
                            );
                        }
                    }
                    if cursor == Some(off) {
                        let st = Stroke::new(1.0, text_col);
                        painter.rect_stroke(
                            Rect::from_min_size(
                                Pos2::new(rect.left() + hx - 1.0, rect.top()),
                                Vec2::new(cw * 2.0 + 2.0, row_h),
                            ),
                            0.0,
                            st,
                            egui::StrokeKind::Inside,
                        );
                        painter.rect_stroke(
                            Rect::from_min_size(
                                Pos2::new(rect.left() + tx, rect.top()),
                                Vec2::new(cw, row_h),
                            ),
                            0.0,
                            st,
                            egui::StrokeKind::Inside,
                        );
                    }
                    let hex_col = match span {
                        Some(s) => color(s.origin, dark),
                        None if b == 0 => faint,
                        None => text_col,
                    };
                    painter.text(
                        at(hx),
                        Align2::LEFT_TOP,
                        format!("{b:02X}"),
                        font.clone(),
                        hex_col,
                    );
                    // decoded column
                    let (ch, col) = match (span_i, span) {
                        (Some(k), Some(s)) => {
                            let chars = labels
                                .entry(k)
                                .or_insert_with(|| s.label().chars().collect());
                            let idx = off - s.start;
                            (
                                chars.get(idx).copied().unwrap_or('·'),
                                color(s.origin, dark),
                            )
                        }
                        _ if (0x20..0x7F).contains(&b) => (b as char, text_col),
                        _ => ('.', faint),
                    };
                    painter.text(at(tx), Align2::LEFT_TOP, ch.to_string(), font.clone(), col);
                }

                // hit test
                let pick = |pos: Pos2| -> Option<usize> {
                    let x = pos.x - rect.left();
                    let i = if x >= x_txt {
                        ((x - x_txt) / cw) as usize
                    } else if x >= x_hex - cw * 0.5 {
                        let mut c = (x - x_hex + cw * 0.5) / cw;
                        if c >= 25.0 {
                            c -= 1.0;
                        }
                        (c / 3.0) as usize
                    } else {
                        return None;
                    };
                    (i < ROW && base + i < p.bytes.len()).then_some(base + i)
                };
                if let Some(pos) = resp.interact_pointer_pos().filter(|_| resp.clicked()) {
                    if let Some(off) = pick(pos) {
                        acts.push(Action::Cursor(off));
                        if let Some(k) = p.span_at(off) {
                            acts.push(Action::Open(k));
                        }
                    }
                }
                if let Some(pos) = resp.hover_pos() {
                    if let Some(k) = pick(pos).and_then(|o| p.span_at(o)) {
                        let s = &p.spans[k];
                        resp.on_hover_text(format!(
                            "{}\nhandle #{:X} — {}\nframe 0x{:X}..0x{:X}, data {} B",
                            s.label(),
                            s.handle,
                            s.origin.describe(),
                            s.start,
                            s.end(),
                            s.data_len
                        ));
                    }
                }
            }
        });
        self.actions.extend(acts);
    }

    fn detail_windows(&mut self, ctx: &egui::Context) {
        let Some(l) = &self.loaded else { return };
        let dark = ctx.global_style().visuals.dark_mode;
        let right = ctx.content_rect().right();
        let mut keep = Vec::new();
        let mut acts = Vec::new();
        for &(pi, si) in &self.details {
            let Some(s) = l.panes.get(pi).and_then(|p| p.spans.get(si)) else {
                continue;
            };
            let mut open = true;
            let title = egui::RichText::new(s.label()).color(color(s.origin, dark));
            egui::Window::new(title)
                .id(egui::Id::NULL.with(("detail", pi, s.handle)))
                .open(&mut open)
                .default_size([460.0, 420.0])
                .pivot(Align2::RIGHT_TOP)
                .default_pos([
                    right - 16.0 - 24.0 * keep.len() as f32,
                    48.0 + 24.0 * keep.len() as f32,
                ])
                .resizable(true)
                .show(ctx, |ui| {
                    detail_body(ui, l, pi, s, &mut self.dumps, &mut acts);
                });
            if open {
                keep.push((pi, si));
            }
        }
        self.details = keep;
        self.actions.extend(acts);
    }
}

/// Compact float text: plain for everyday magnitudes, exponent otherwise.
fn num(v: f64) -> String {
    if v == 0.0 || (1e-4..1e12).contains(&v.abs()) {
        let t = format!("{v:.6}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    } else if v.is_finite() {
        format!("{v:.4e}")
    } else {
        format!("{v}")
    }
}

fn detail_body(
    ui: &mut egui::Ui,
    l: &Loaded,
    pane: usize,
    s: &Span,
    dumps: &mut HashMap<u64, String>,
    acts: &mut Vec<Action>,
) {
    let same_pane = |acts: &mut Vec<Action>, a: Action| acts.push(a);
    ui.horizontal(|ui| {
        ui.label(s.origin.describe());
        if ui.small_button("go to").clicked() {
            same_pane(acts, Action::Jump(s.start));
        }
    });
    egui::Grid::new(("hdr", pane, s.handle))
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            let mut row = |k: &str, v: String| {
                ui.label(k);
                ui.monospace(v);
                ui.end_row();
            };
            row("Handle", format!("#{:X}", s.handle));
            row("Class", s.class.clone());
            if let Some(n) = &s.name {
                row("Name", n.clone());
            }
            row(
                "Frame",
                format!("0x{:X} .. 0x{:X} ({} B)", s.start, s.end(), s.len),
            );
            row("Data", format!("0x{:X}, {} B", s.data_start, s.data_len));
            if s.handle_bits > 0 {
                row("Handle stream", format!("{} bits", s.handle_bits));
            }
            row(
                "CRC",
                format!(
                    "{:04X} {}",
                    s.crc_stored,
                    if s.crc_ok { "ok" } else { "MISMATCH" }
                ),
            );
        });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt(("fields", pane, s.handle))
        .max_height(260.0)
        .show(ui, |ui| {
            egui::Grid::new(("fld", pane, s.handle))
                .num_columns(3)
                .striped(true)
                .show(ui, |ui| {
                    for f in &s.fields {
                        ui.label(&f.label);
                        ui.monospace(&f.value);
                        ui.horizontal(|ui| {
                            if let Some(bit) = f.bit {
                                let at = s.byte_of_bit(bit);
                                if ui
                                    .small_button(format!("bit {bit}"))
                                    .on_hover_text(format!(
                                        "pane byte 0x{at:X}, bit {} in it",
                                        bit % 8
                                    ))
                                    .clicked()
                                {
                                    acts.push(Action::Jump(at));
                                }
                            }
                            if let Some(h) = f.target {
                                if ui.small_button("open").clicked() {
                                    acts.push(Action::Handle(h));
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    if let Some(doc) = &l.doc {
        egui::CollapsingHeader::new("acadrust view")
            .id_salt(("dump", pane, s.handle))
            .show(ui, |ui| {
                let text = dumps
                    .entry(s.handle)
                    .or_insert_with(|| model::dump(doc, s.handle));
                egui::ScrollArea::both()
                    .id_salt(("dumps", pane, s.handle))
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(text.as_str()).monospace())
                                .selectable(true),
                        );
                    });
            });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.open(p.display().to_string());
        }
        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(3.0);
            self.top_bar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("inspector").show(ui, |ui| {
            ui.add_space(2.0);
            self.inspector(ui);
            if let Some(l) = &self.loaded {
                if let Some(p) = l.panes.get(self.pane) {
                    ui.weak(&p.note);
                }
                for w in &l.warnings {
                    ui.colored_label(Color32::from_rgb(220, 150, 0), w);
                }
            }
            ui.add_space(2.0);
        });
        if self.pane().is_some_and(|p| !p.spans.is_empty()) {
            egui::Panel::left("records")
                .resizable(true)
                .default_size(300.0)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    self.record_list(ui);
                });
        }
        egui::CentralPanel::default().show(ui, |ui| self.hex_view(ui));
        self.detail_windows(&ctx);
        self.apply_actions();
    }
}
