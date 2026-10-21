//! The window (eframe/egui).

use crate::scene::{opcode_name, station_text, Decoded, Scene};
use crate::view::{self, Camera, Canvas, Layers, Mode, Rgba, S2};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Stroke, Vec2};
use std::sync::mpsc::{channel, Receiver};

pub fn run(path: Option<String>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("civil3drust viewer")
            .with_inner_size([1280.0, 820.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "civil3drust viewer",
        options,
        Box::new(move |_cc| {
            let mut app = App::default();
            if let Some(p) = path {
                app.open(p);
            }
            Ok(Box::new(app))
        }),
    )
}

#[derive(Default)]
struct App {
    scene: Option<Scene>,
    loading: Option<(String, Receiver<Result<Scene, String>>)>,
    error: Option<String>,
    layers: Layers,
    cam: Camera,
    /// Remembered plan and orbit cameras, so switching keeps each view.
    other_cam: Option<Camera>,
    fit_pending: bool,
    pick_lines: Vec<(u64, Vec<S2>)>,
    hover_world: Option<[f64; 2]>,
    filter: String,
}

impl App {
    fn open(&mut self, path: String) {
        let (tx, rx) = channel();
        let p = path.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Scene::load(&p));
        });
        self.loading = Some((path, rx));
        self.error = None;
    }

    fn poll_loading(&mut self, ctx: &egui::Context) {
        let Some((path, rx)) = &self.loading else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(scene)) => {
                self.scene = Some(scene);
                self.layers.selected = None;
                self.layers.hidden_classes.clear();
                self.other_cam = None;
                self.fit_pending = true;
                self.loading = None;
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

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("📂 Open DWG…").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("DWG drawing", &["dwg", "DWG"])
                    .pick_file()
                {
                    self.open(p.display().to_string());
                }
            }
            ui.separator();
            let mut mode = self.cam.mode;
            ui.selectable_value(&mut mode, Mode::Plan, "2D plan");
            ui.selectable_value(&mut mode, Mode::Orbit, "3D orbit");
            self.switch_mode(mode);
            if self.cam.mode == Mode::Orbit {
                ui.checkbox(&mut self.cam.perspective, "perspective");
            }
            if ui.button("⛶ Fit").clicked() {
                self.fit_pending = true;
            }
            ui.separator();
            ui.toggle_value(&mut self.layers.decoded, "Decoded")
                .on_hover_text("Geometry civil3drust decoded from the object records");
            ui.toggle_value(&mut self.layers.proxy, "Proxy graphics")
                .on_hover_text("The display cache Civil 3D saved inside each object");
            ui.toggle_value(&mut self.layers.placeholders, "Placeholders")
                .on_hover_text(
                    "Civil 3D's \"not drawn yet\" images (class name in a box at the origin)",
                );
            ui.toggle_value(&mut self.layers.context, "AutoCAD entities");
            ui.toggle_value(&mut self.layers.text, "Text");
            ui.toggle_value(&mut self.layers.grid, "Grid");
            if self.cam.mode == Mode::Orbit && self.layers.proxy {
                ui.separator();
                ui.label("Proxy lift");
                ui.add(
                    egui::DragValue::new(&mut self.layers.proxy_lift)
                        .speed(0.5)
                        .suffix(" u"),
                )
                .on_hover_text("Raise proxy graphics above the decoded geometry in 3D");
            }
            ui.separator();
            if ui
                .button(if self.layers.dark { "Light" } else { "Dark" })
                .on_hover_text("Light / dark")
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

    fn object_list(&mut self, ui: &mut egui::Ui) {
        let Some(scene) = &self.scene else {
            ui.label("No drawing open.");
            ui.small("Open a DWG, drop one on the window, or pass a path on the command line.");
            return;
        };
        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("filter by class or name"));
        ui.add_space(4.0);
        let filter = self.filter.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (class, count) in scene.classes() {
                let mut shown = !self.layers.hidden_classes.contains(&class);
                let objs: Vec<_> = scene
                    .objects
                    .iter()
                    .filter(|o| o.class == class)
                    .filter(|o| {
                        filter.is_empty()
                            || o.class.to_lowercase().contains(&filter)
                            || o.name
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .contains(&filter)
                    })
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
                        (Decoded::Alignment(_), _) => "◆",
                        (Decoded::Failed(_), _) => "✖",
                        (_, Some(g)) if g.placeholder => "□",
                        (_, Some(_)) => "▣",
                        _ => "·",
                    };
                    let label = format!(
                        "   {badge} {:X}  {}",
                        o.handle,
                        o.name.as_deref().unwrap_or("")
                    );
                    let sel = self.layers.selected == Some(o.handle);
                    if ui.selectable_label(sel, label).clicked() {
                        self.layers.selected = if sel { None } else { Some(o.handle) };
                    }
                }
            }
            ui.add_space(8.0);
            ui.small("◆ decoded   ▣ proxy graphics   □ placeholder only   ✖ decode failed");
        });
    }

    fn details(&self, ui: &mut egui::Ui) {
        let Some(scene) = &self.scene else { return };
        let Some(o) = self
            .layers
            .selected
            .and_then(|h| scene.objects.iter().find(|o| o.handle == h))
        else {
            ui.weak("Click an object in the view or the list.");
            return;
        };
        egui::Grid::new("details")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("Handle");
                ui.monospace(format!("{:X}", o.handle));
                ui.end_row();
                ui.label("Class");
                ui.monospace(&o.class);
                ui.end_row();
                if let Some(n) = &o.name {
                    ui.label("Name");
                    ui.label(n);
                    ui.end_row();
                }
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
                    None => {
                        ui.weak("none found");
                    }
                    Some(g) => {
                        ui.label(format!(
                            "{} chunk(s), {} drawn{}",
                            g.chunk_count,
                            g.prims.len(),
                            if g.placeholder { " (placeholder)" } else { "" }
                        ));
                    }
                }
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
            });
    }

    fn viewport(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = resp.rect;
        self.cam.viewport = [rect.min.x, rect.min.y, rect.width(), rect.height()];
        let pal = view::Palette::new(self.layers.dark);
        painter.rect_filled(rect, 0.0, c32(pal.bg));

        let Some(scene) = &self.scene else {
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
            self.cam.fit(view::bounds(scene, &self.layers));
            self.fit_pending = false;
        }

        // Navigation: left-drag pans in 2D and orbits in 3D; right or
        // middle drag pans in both; wheel zooms.
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
        self.pick_lines = view::draw(scene, &self.layers, &self.cam, &mut canvas);

        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                self.layers.selected = view::pick(&self.pick_lines, [p.x, p.y], 6.0);
            }
        }
        self.hover_world = match (self.cam.mode, resp.hover_pos()) {
            (Mode::Plan, Some(p)) => Some(self.cam.unproject_plan([p.x, p.y])),
            _ => None,
        };
        if self.cam.mode == Mode::Orbit {
            draw_axes(&painter, &self.cam, rect);
        }
        // Hover tooltip with the object under the cursor.
        if let Some(p) = resp.hover_pos() {
            if let Some(h) = view::pick(&self.pick_lines, [p.x, p.y], 6.0) {
                if let Some(o) = scene.objects.iter().find(|o| o.handle == h) {
                    let text = match &o.name {
                        Some(n) => format!("{} — {n}", o.class),
                        None => o.class.clone(),
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
            if let Some(s) = &self.scene {
                ui.label(&s.path);
                if s.failsafe.is_some() {
                    ui.colored_label(Color32::from_rgb(230, 170, 60), "⚠ read in failsafe mode")
                        .on_hover_text(s.failsafe.clone().unwrap_or_default());
                }
                ui.separator();
                ui.label(format!(
                    "{} custom object(s), {} AutoCAD primitive(s)",
                    s.objects.len(),
                    s.context.len()
                ));
            }
            if let Some(e) = &self.error {
                ui.colored_label(Color32::LIGHT_RED, e);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match self.hover_world {
                    Some([x, y]) => ui.monospace(format!("X {x:.3}  Y {y:.3}")),
                    None if self.cam.mode == Mode::Orbit => {
                        ui.weak("drag: orbit · right/middle drag or shift: pan · wheel: zoom · double-click: fit")
                    }
                    None => ui.weak("drag: pan · wheel: zoom · double-click: fit · click: select"),
                };
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_loading(&ctx);
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.open(p.display().to_string());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F)) && !ctx.egui_wants_keyboard_input() {
            self.fit_pending = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.layers.selected = None;
        }
        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(3.0);
            self.top_bar(ui);
            ui.add_space(2.0);
        });
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("objects")
            .resizable(true)
            .default_size(290.0)
            .min_size(220.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.heading("Objects");
                self.object_list(ui);
            });
        egui::Panel::right("details")
            .resizable(true)
            .default_size(300.0)
            .min_size(240.0)
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.heading("Selection");
                self.details(ui);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.viewport(ui));
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
        // egui places text by its top-left corner; DWG text by its
        // baseline start. Shift up by the ascent along the rotated up axis.
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
