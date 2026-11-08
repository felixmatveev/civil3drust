//! The hex pane: the selected record and its related records as blocks of
//! bytes (D-073). Raw records come from the open document (edits show
//! before saving, changed bits against the opened file in red); records
//! acadrust parses come from the file on disk.

use crate::filemap::{self, Field};
use crate::state::{Relation, Workbench};
use civil3drust::raw;
use eframe::egui::{self, text::LayoutJob, Color32, FontId, TextFormat};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    /// Raw record in the open document (current, possibly edited).
    Document,
    /// Record acadrust parses: bytes as stored in the file on disk.
    File,
    /// Not found (file map still loading, or no such record).
    Missing,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Document => "document",
            Source::File => "file",
            Source::Missing => "missing",
        }
    }
}

/// One record shown in the pane.
#[derive(Debug, Clone)]
pub struct Block {
    pub handle: u64,
    pub class: String,
    pub name: Option<String>,
    /// How it relates to the selection, as text.
    pub relation: String,
    pub source: Source,
    pub bytes: Vec<u8>,
    pub handle_bits: usize,
    /// Per byte: differs from the opened file.
    pub changed: Vec<bool>,
    /// Per byte: a field starts in it.
    pub field_start: Vec<bool>,
    pub fields: Vec<Field>,
}

impl Block {
    pub fn title(&self) -> String {
        let name = self
            .name
            .as_deref()
            .filter(|n| !n.is_empty())
            .map(|n| format!(" \"{n}\""))
            .unwrap_or_default();
        let changed = self.changed.iter().filter(|c| **c).count();
        format!(
            "[{:X}] {}{name} — {} — {} bytes, {}{}",
            self.handle,
            self.class,
            self.relation,
            self.bytes.len(),
            self.source.label(),
            if changed > 0 {
                format!(", {changed} byte(s) changed")
            } else {
                String::new()
            }
        )
    }

    /// The field at or before `bit` (fields are kept in bit order).
    pub fn field_at(&self, bit: usize) -> Option<&Field> {
        self.fields
            .iter()
            .filter(|f| f.bit.is_some_and(|b| b <= bit))
            .max_by_key(|f| f.bit)
    }
}

fn changed_bytes(old: Option<&Vec<u8>>, new: &[u8]) -> Vec<bool> {
    let mut c = vec![false; new.len()];
    let Some(old) = old else {
        return vec![true; new.len()];
    };
    if old.len() == new.len() {
        for (s, e) in raw::changed_bit_ranges(old, new, 0) {
            for b in c.iter_mut().take(e.div_ceil(8)).skip(s / 8) {
                *b = true;
            }
        }
        return c;
    }
    // Different sizes: bytes inserted or removed somewhere. Mark what lies
    // between the common start and the common end.
    let n = old.len().min(new.len());
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(n - prefix)
        .take_while(|(a, b)| a == b)
        .count();
    for b in c.iter_mut().take(new.len() - suffix).skip(prefix) {
        *b = true;
    }
    c
}

/// Build the block for `handle`.
pub fn block(wb: &Workbench, handle: u64, relation: &str) -> Block {
    let mut b = Block {
        handle,
        class: wb.class_of(handle),
        name: None,
        relation: relation.to_string(),
        source: Source::Missing,
        bytes: Vec::new(),
        handle_bits: 0,
        changed: Vec::new(),
        field_start: Vec::new(),
        fields: Vec::new(),
    };
    if let Some(r) = wb.index.records.get(&handle) {
        let (name, mut fields) = filemap::civil_fields(&r.rec);
        fields.sort_by_key(|f| f.bit.unwrap_or(0));
        b.name = name.or_else(|| r.name.clone());
        b.source = Source::Document;
        b.bytes = r.rec.data.clone();
        b.handle_bits = r.rec.handle_bits;
        b.changed = changed_bytes(wb.baseline.get(&handle), &b.bytes);
        b.fields = fields;
    } else if let (Some(span), Some(bytes)) = (wb.file_span(handle), wb.file_bytes(handle)) {
        b.name = span.name.clone();
        b.source = Source::File;
        b.bytes = bytes.to_vec();
        b.handle_bits = span.handle_bits;
        b.changed = vec![false; b.bytes.len()];
        let mut fields = span.fields.clone();
        fields.sort_by_key(|f| f.bit.unwrap_or(0));
        b.fields = fields;
    }
    b.field_start = vec![false; b.bytes.len()];
    for f in &b.fields {
        if let Some(bit) = f.bit {
            if let Some(x) = b.field_start.get_mut(bit / 8) {
                *x = true;
            }
        }
    }
    b
}

pub fn relation_text(r: &Relation) -> String {
    match r {
        Relation::Selected => "selected".into(),
        Relation::RefBy { from, depth: 1 } => format!("referenced by {from:X}"),
        Relation::RefBy { from, depth } => format!("referenced by {from:X} (hop {depth})"),
        Relation::RefersTo { to } => format!("refers to {to:X}"),
    }
}

/// Values readable at `bit` in `bytes`, for the cursor panel.
pub fn decode_at(bytes: &[u8], bit: usize) -> Vec<(String, String)> {
    let mut v = Vec::new();
    let total = raw::bit_len(bytes);
    if bit >= total {
        return v;
    }
    let bits: String = (bit..(bit + 16).min(total))
        .map(|i| {
            if raw::get_bit(bytes, i) == 1 {
                '1'
            } else {
                '0'
            }
        })
        .collect();
    v.push(("bits".into(), bits));
    let rc = raw::get_byte(bytes, bit);
    v.push(("RC".into(), format!("{rc} (0x{rc:02X})")));
    if bit + 16 <= total {
        let s = raw::get_byte(bytes, bit) as u16 | (raw::get_byte(bytes, bit + 8) as u16) << 8;
        v.push(("RS".into(), format!("{}", s as i16)));
    }
    if bit + 32 <= total {
        v.push(("RL".into(), format!("{}", raw::get_i32(bytes, bit))));
    }
    if bit + 64 <= total {
        v.push(("RD".into(), format!("{}", raw::get_f64(bytes, bit))));
    }
    if let Some((x, next)) = raw::read_bd(bytes, bit) {
        v.push(("BD".into(), format!("{x}  ({} bits)", next - bit)));
    }
    if let Some((x, next)) = raw::read_bl(bytes, bit) {
        v.push(("BL".into(), format!("{x}  ({} bits)", next - bit)));
    }
    v
}

/// What the pane needs to remember between frames.
pub struct HexState {
    pub depth: usize,
    pub back_refs: bool,
    pub class_filter: String,
    pub collapsed: std::collections::HashSet<u64>,
    /// (block handle, bit).
    pub cursor: Option<(u64, usize)>,
    pub scroll_to: Option<(u64, usize)>,
    /// Cached blocks and the key they were built for.
    cache: Option<(CacheKey, Vec<Block>)>,
    pub goto: String,
}

impl Default for HexState {
    fn default() -> Self {
        HexState {
            depth: 1,
            back_refs: true,
            class_filter: String::new(),
            collapsed: Default::default(),
            cursor: None,
            scroll_to: None,
            cache: None,
            goto: String::new(),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
struct CacheKey {
    generation: u64,
    selected: u64,
    depth: usize,
    back_refs: bool,
    filemap: bool,
}

enum Line {
    Header(usize),
    Row(usize, usize),
}

/// Action requested by the pane.
pub enum HexAction {
    Select(u64),
}

const COLS: usize = 16;
const PREFIX: usize = 7; // "000000 "

/// Draw the pane. `generation` changes whenever the document does.
pub fn ui(
    ui: &mut egui::Ui,
    st: &mut HexState,
    wb: &Workbench,
    selected: Option<u64>,
    generation: u64,
) -> Option<HexAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.strong("Hex");
        ui.separator();
        ui.label("refs depth");
        for d in 0..=2 {
            ui.selectable_value(&mut st.depth, d, d.to_string());
        }
        ui.checkbox(&mut st.back_refs, "records referencing it");
        ui.add(
            egui::TextEdit::singleline(&mut st.class_filter)
                .hint_text("hide classes (comma list)")
                .desired_width(170.0),
        );
        ui.separator();
        let r = ui.add(
            egui::TextEdit::singleline(&mut st.goto)
                .hint_text("go to handle")
                .desired_width(90.0),
        );
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(h) = civil3drust::commands::parse_handle(&st.goto) {
                action = Some(HexAction::Select(h));
            }
        }
        if wb.filemap.is_none() {
            ui.weak("(file map loading: parsed records not shown yet)");
        }
    });
    let Some(sel) = selected else {
        ui.weak("Select an object (view, list, or go to handle).");
        return action;
    };
    let key = CacheKey {
        generation,
        selected: sel,
        depth: st.depth,
        back_refs: st.back_refs,
        filemap: wb.filemap.is_some(),
    };
    if st.cache.as_ref().map(|c| c.0) != Some(key) {
        let blocks = wb
            .related(sel, st.depth)
            .into_iter()
            .filter(|r| st.back_refs || !matches!(r.relation, Relation::RefersTo { .. }))
            .map(|r| block(wb, r.handle, &relation_text(&r.relation)))
            .collect();
        st.cache = Some((key, blocks));
    }
    let hidden: Vec<String> = st
        .class_filter
        .split(',')
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    let cache = st.cache.take();
    let blocks: Vec<&Block> = cache
        .as_ref()
        .map(|c| c.1.iter())
        .into_iter()
        .flatten()
        .filter(|b| {
            b.handle == sel
                || !hidden
                    .iter()
                    .any(|h| b.class.to_ascii_uppercase().contains(h))
        })
        .collect();

    let mut lines = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        lines.push(Line::Header(i));
        if !st.collapsed.contains(&b.handle) {
            for r in 0..b.bytes.len().div_ceil(COLS) {
                lines.push(Line::Row(i, r));
            }
        }
    }

    let font = FontId::monospace(12.5);
    let row_h = ui.ctx().fonts_mut(|f| f.row_height(&font)) + 2.0;
    let char_w = ui.ctx().fonts_mut(|f| f.glyph_width(&font, '0'));
    let dark = ui.visuals().dark_mode;
    let col_off = if dark {
        Color32::from_gray(110)
    } else {
        Color32::from_gray(140)
    };
    let col_byte = ui.visuals().text_color();
    let col_changed = Color32::from_rgb(240, 80, 70);
    let col_field = if dark {
        Color32::from_rgb(120, 180, 255)
    } else {
        Color32::from_rgb(20, 90, 200)
    };
    let cursor_bg = if dark {
        Color32::from_rgb(80, 80, 30)
    } else {
        Color32::from_rgb(255, 240, 150)
    };

    let side_w = 330.0_f32.min(ui.available_width() * 0.4);
    ui.horizontal_top(|ui| {
        let left_w = ui.available_width() - side_w - 8.0;
        ui.vertical(|ui| {
            ui.set_width(left_w);
            let mut sa = egui::ScrollArea::vertical()
                .id_salt("hex_rows")
                .auto_shrink([false, false]);
            if let Some((h, bit)) = st.scroll_to.take() {
                if let Some(pos) = lines.iter().position(|l| match l {
                    Line::Row(i, r) => blocks[*i].handle == h && *r == bit / 8 / COLS,
                    _ => false,
                }) {
                    sa = sa.vertical_scroll_offset((pos as f32 - 3.0).max(0.0) * row_h);
                }
            }
            sa.show_rows(ui, row_h, lines.len(), |ui, range| {
                for line in &lines[range] {
                    match *line {
                        Line::Header(i) => {
                            let b = blocks[i];
                            let open = !st.collapsed.contains(&b.handle);
                            ui.horizontal(|ui| {
                                ui.set_height(row_h);
                                let arrow = if open { "▼" } else { "▶" };
                                if ui.small_button(arrow).clicked() {
                                    if open {
                                        st.collapsed.insert(b.handle);
                                    } else {
                                        st.collapsed.remove(&b.handle);
                                    }
                                }
                                let t = egui::RichText::new(b.title()).strong();
                                let t = if b.handle == sel {
                                    t.color(col_field)
                                } else {
                                    t
                                };
                                ui.label(t);
                                if b.handle != sel && ui.small_button("select").clicked() {
                                    action = Some(HexAction::Select(b.handle));
                                }
                            });
                        }
                        Line::Row(i, r) => {
                            let b = blocks[i];
                            let start = r * COLS;
                            let end = (start + COLS).min(b.bytes.len());
                            let cur = st.cursor.filter(|c| c.0 == b.handle).map(|c| c.1 / 8);
                            let hs =
                                raw::bit_len(&b.bytes) - b.handle_bits.min(raw::bit_len(&b.bytes));
                            let mut job = LayoutJob::default();
                            let fmt = |color: Color32, bg: Color32, underline: bool| {
                                let mut f = TextFormat::simple(font.clone(), color);
                                f.background = bg;
                                if underline {
                                    f.underline = egui::Stroke::new(1.0, color);
                                }
                                f
                            };
                            job.append(
                                &format!("{:06X} ", start),
                                0.0,
                                fmt(col_off, Color32::TRANSPARENT, false),
                            );
                            for k in start..start + COLS {
                                if k >= end {
                                    job.append(
                                        "   ",
                                        0.0,
                                        fmt(col_byte, Color32::TRANSPARENT, false),
                                    );
                                    continue;
                                }
                                let color = if b.changed[k] {
                                    col_changed
                                } else if k * 8 >= hs && b.handle_bits > 0 {
                                    col_off
                                } else if b.field_start[k] {
                                    col_field
                                } else {
                                    col_byte
                                };
                                let bg = if cur == Some(k) {
                                    cursor_bg
                                } else {
                                    Color32::TRANSPARENT
                                };
                                job.append(
                                    &format!("{:02X}", b.bytes[k]),
                                    0.0,
                                    fmt(color, bg, b.field_start[k]),
                                );
                                job.append(" ", 0.0, fmt(col_byte, Color32::TRANSPARENT, false));
                            }
                            job.append(" ", 0.0, fmt(col_byte, Color32::TRANSPARENT, false));
                            for k in start..end {
                                let c = b.bytes[k];
                                let ch = if (0x20..0x7F).contains(&c) {
                                    c as char
                                } else {
                                    '·'
                                };
                                let color = if b.changed[k] { col_changed } else { col_off };
                                let bg = if cur == Some(k) {
                                    cursor_bg
                                } else {
                                    Color32::TRANSPARENT
                                };
                                job.append(&ch.to_string(), 0.0, fmt(color, bg, false));
                            }
                            let resp = ui.add(egui::Label::new(job).sense(egui::Sense::click()));
                            if resp.clicked() {
                                if let Some(p) = resp.interact_pointer_pos() {
                                    let col = ((p.x - resp.rect.left()) / char_w) as usize;
                                    let byte = if (PREFIX..PREFIX + COLS * 3).contains(&col) {
                                        Some((col - PREFIX) / 3)
                                    } else if col > PREFIX + COLS * 3 {
                                        Some(col - PREFIX - COLS * 3 - 1)
                                    } else {
                                        None
                                    };
                                    if let Some(k) = byte.map(|x| start + x).filter(|k| *k < end) {
                                        st.cursor = Some((b.handle, k * 8));
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.set_width(side_w);
            egui::ScrollArea::vertical()
                .id_salt("hex_side")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(a) = side_panel(ui, st, &blocks, sel) {
                        action = Some(a);
                    }
                });
        });
    });
    drop(blocks);
    st.cache = cache;
    action
}

fn side_panel(
    ui: &mut egui::Ui,
    st: &mut HexState,
    blocks: &[&Block],
    sel: u64,
) -> Option<HexAction> {
    let mut action = None;
    let cur_handle = st.cursor.map(|c| c.0).unwrap_or(sel);
    let b = blocks
        .iter()
        .find(|b| b.handle == cur_handle)
        .or(blocks.first())?;
    if b.source == Source::Missing {
        ui.weak("No bytes for this record (file map not loaded, or not in the file).");
    }
    if let Some((h, bit)) = st.cursor.filter(|c| c.0 == b.handle) {
        ui.strong(format!("Cursor [{h:X}] byte {} · bit {bit}", bit / 8));
        let mut bit_edit = bit;
        ui.horizontal(|ui| {
            ui.label("bit");
            if ui
                .add(
                    egui::DragValue::new(&mut bit_edit)
                        .range(0..=raw::bit_len(&b.bytes).saturating_sub(1)),
                )
                .changed()
            {
                st.cursor = Some((h, bit_edit));
            }
            if ui.small_button("−1").clicked() {
                st.cursor = Some((h, bit.saturating_sub(1)));
            }
            if ui.small_button("+1").clicked() {
                st.cursor = Some((h, bit + 1));
            }
        });
        if b.handle_bits > 0 {
            let hs = raw::bit_len(&b.bytes) - b.handle_bits.min(raw::bit_len(&b.bytes));
            if bit >= hs {
                ui.weak(format!("in the handle stream (from bit {hs})"));
            }
        }
        if let Some(f) = b.field_at(bit) {
            ui.label(format!(
                "in/after field: {} = {} (bit {})",
                f.label,
                f.value,
                f.bit.unwrap_or(0)
            ));
        }
        egui::Grid::new("decode_at")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for (k, v) in decode_at(&b.bytes, bit) {
                    ui.monospace(k);
                    ui.monospace(v);
                    ui.end_row();
                }
            });
        ui.separator();
    } else {
        ui.weak("Click a byte to decode values at it.");
        ui.separator();
    }
    ui.strong(format!("Fields of [{:X}] {}", b.handle, b.class));
    ui.small("blue/underlined bytes start a field · red changed since opened · grey handle stream");
    egui::Grid::new("fields")
        .num_columns(3)
        .striped(true)
        .show(ui, |ui| {
            for f in &b.fields {
                match f.bit {
                    Some(bit) => {
                        if ui
                            .link(format!("{bit}"))
                            .on_hover_text("go to bit")
                            .clicked()
                        {
                            st.cursor = Some((b.handle, bit));
                            st.scroll_to = Some((b.handle, bit));
                        }
                    }
                    None => {
                        ui.label("");
                    }
                }
                ui.label(&f.label);
                ui.horizontal(|ui| {
                    ui.monospace(&f.value);
                    if let Some(t) = f.target {
                        if ui
                            .small_button("→")
                            .on_hover_text("select target")
                            .clicked()
                        {
                            action = Some(HexAction::Select(t));
                        }
                    }
                });
                ui.end_row();
            }
        });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_marks_bytes() {
        let old = vec![0u8, 0, 0, 0];
        assert_eq!(
            changed_bytes(Some(&old), &[0, 1, 0, 0]),
            vec![false, true, false, false]
        );
        // Two bytes inserted after the first.
        assert_eq!(
            changed_bytes(Some(&vec![1, 2, 3]), &[1, 8, 9, 2, 3]),
            vec![false, true, true, false, false]
        );
        assert_eq!(changed_bytes(None, &[1, 2]), vec![true, true]);
    }

    #[test]
    fn decode_values() {
        let mut buf = vec![0u8; 16];
        raw::set_f64(&mut buf, 3, 12.5);
        let v = decode_at(&buf, 3);
        assert!(v.iter().any(|(k, x)| k == "RD" && x == "12.5"), "{v:?}");
        assert!(decode_at(&buf, 999).is_empty());
    }
}
