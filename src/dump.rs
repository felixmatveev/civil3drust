//! A plain-text view of a DWG: what acadrust and civil3drust decode from
//! it, written as RON (Rust Object Notation; D-065).
//!
//! The file reads like
//!
//! ```text
//! Drawing(
//!     format: 1,
//!     source: "S24_ZERO_CIVIL.dwg",
//!     version: "AC1032",
//!     entities: [
//!         Line(h: "2A1", layer: "0", start: (0.0, 0.0, 0.0), end: (0.0, 100.0, 0.0)),
//!         Aecc(h: "3F0", class: "AECC_ALIGNMENT", layer: "C-ROAD"),
//!     ],
//!     alignments: [ ... ],
//!     classes: [("AECC_ALIGNMENT", 1), ...],
//! )
//! ```
//!
//! and parses back into the same [`Drawing`] ([`from_ron`]). It is a
//! readable *view*: model-space entities in compact form, decoded
//! alignments, and a count of the records civil3drust keeps as raw bytes.
//! With `raw` set, [`dump`] also lists every such record with its bytes in
//! hex ([`Record`]), which is the part a later RON → DWG writer needs
//! (FUTURE.md, FRT-2). Handles are hex strings, as elsewhere in
//! civil3drust; floats are written in their shortest exact form, so they
//! read back bit for bit.

use crate::aecc_alignment::Linked;
use crate::aecc_alignment_refs::{alignment_refs, alignment_styles};
use crate::aecc_elements::{decode_elements, Element, ResolvedElement, Segment};
use crate::raw::{self, Opened, RawRecord};
use acadrust::document::CadDocument;
use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Version of the dump layout; bump it when a field changes meaning.
pub const FORMAT: u32 = 1;

/// One drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    pub format: u32,
    /// File name the dump was made from.
    pub source: String,
    /// DWG version, e.g. `AC1032`.
    pub version: String,
    /// Model-space entities, sorted by handle.
    pub entities: Vec<Entity>,
    /// Decoded `AECC_ALIGNMENT` records, sorted by handle.
    pub alignments: Vec<Alignment>,
    /// Records kept as raw bytes (AECC and other classes acadrust does not
    /// model), counted by DXF class name, sorted by name.
    pub classes: Vec<(String, usize)>,
    /// Those records with their bytes; only with `raw`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<Record>,
}

/// A model-space entity in compact form. Types without a compact form are
/// [`Entity::Other`], named by acadrust's type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Entity {
    Line {
        h: String,
        layer: String,
        start: (f64, f64, f64),
        end: (f64, f64, f64),
    },
    Arc {
        h: String,
        layer: String,
        center: (f64, f64, f64),
        radius: f64,
        /// Radians, counter-clockwise from the X axis.
        start_angle: f64,
        end_angle: f64,
    },
    Circle {
        h: String,
        layer: String,
        center: (f64, f64, f64),
        radius: f64,
    },
    Point {
        h: String,
        layer: String,
        at: (f64, f64, f64),
    },
    /// LWPOLYLINE: vertices as (x, y, bulge).
    Polyline {
        h: String,
        layer: String,
        closed: bool,
        elevation: f64,
        vertices: Vec<(f64, f64, f64)>,
    },
    Text {
        h: String,
        layer: String,
        at: (f64, f64, f64),
        height: f64,
        rotation: f64,
        value: String,
    },
    MText {
        h: String,
        layer: String,
        at: (f64, f64, f64),
        height: f64,
        value: String,
    },
    Insert {
        h: String,
        layer: String,
        block: String,
        at: (f64, f64, f64),
        rotation: f64,
        scale: (f64, f64, f64),
    },
    /// A Civil 3D (or other AEC) entity kept as raw bytes.
    Aecc {
        h: String,
        class: String,
        layer: String,
    },
    Other {
        h: String,
        kind: String,
        layer: String,
    },
}

impl Entity {
    pub fn handle(&self) -> &str {
        match self {
            Self::Line { h, .. }
            | Self::Arc { h, .. }
            | Self::Circle { h, .. }
            | Self::Point { h, .. }
            | Self::Polyline { h, .. }
            | Self::Text { h, .. }
            | Self::MText { h, .. }
            | Self::Insert { h, .. }
            | Self::Aecc { h, .. }
            | Self::Other { h, .. } => h,
        }
    }
}

/// An alignment as civil3drust decodes it (see `examples/alignments`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alignment {
    pub h: String,
    pub name: Option<String>,
    /// Alignment style: its name, or `#<handle>` when the style record has
    /// no plain name (the template default style, S24).
    pub style: Option<String>,
    /// Stored start station; `None` when not found (stations then run
    /// from 0).
    pub start_station: Option<f64>,
    /// Length of the chain in `elements`.
    pub length: f64,
    /// The alignment proper, in path order.
    pub elements: Vec<AlignmentElement>,
    /// Pieces left disconnected by deleting an interior element (S100).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detached: Vec<Vec<AlignmentElement>>,
    /// Groups held only by the layout JSON (D-059, D-061).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<LayoutGroup>,
    /// Set when the record could not be decoded; the other fields are then
    /// empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One piece of an alignment, with the ID of the element record that
/// holds it (a record with several pieces repeats its ID).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlignmentElement {
    Line {
        id: i32,
        start: (f64, f64),
        end: (f64, f64),
    },
    Arc {
        id: i32,
        start: (f64, f64),
        end: (f64, f64),
        center: (f64, f64),
        radius: f64,
        /// Turns left (counter-clockwise).
        ccw: bool,
        /// How the element record defines it (free, fixed, floating, ...).
        method: String,
    },
    Spiral {
        id: i32,
        start: (f64, f64),
        end: (f64, f64),
        length: f64,
        /// `None` is an infinite radius (the line end of the spiral).
        start_radius: Option<f64>,
        radius: Option<f64>,
        ccw: bool,
        kind: String,
        /// Sideways shift from the parent, positive to the left (the spiral
        /// of an offset alignment, S255).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset: Option<f64>,
    },
}

/// A spiral group stored only as typed parameters of the layout JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutGroup {
    pub id: i32,
    pub constraint_type: i32,
    pub previous: i32,
    pub next: i32,
    pub params: Vec<(String, f64)>,
    pub locked: bool,
    pub solved: bool,
}

/// A record kept as raw bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub h: String,
    pub class: String,
    pub entity: bool,
    /// Size of the handle stream at the end of `data`, in bits.
    pub handle_bits: usize,
    /// Record bytes in hex, space-separated.
    pub data: String,
}

fn hx(h: u64) -> String {
    format!("{h:X}")
}

fn p3(v: &acadrust::types::Vector3) -> (f64, f64, f64) {
    (v.x, v.y, v.z)
}

fn p2(p: crate::Point2) -> (f64, f64) {
    (p.x, p.y)
}

fn finite(r: f64) -> Option<f64> {
    r.is_finite().then_some(r)
}

/// acadrust's name for an entity type: the enum variant.
fn kind_name(e: &EntityType) -> String {
    let dbg = format!("{e:?}");
    dbg.split(['(', ' ', '{']).next().unwrap_or("?").to_string()
}

fn entity(e: &EntityType) -> Entity {
    let c = e.common();
    let h = hx(c.handle.value());
    let layer = c.layer.clone();
    match e {
        EntityType::Line(l) => Entity::Line {
            h,
            layer,
            start: p3(&l.start),
            end: p3(&l.end),
        },
        EntityType::Arc(a) => Entity::Arc {
            h,
            layer,
            center: p3(&a.center),
            radius: a.radius,
            start_angle: a.start_angle,
            end_angle: a.end_angle,
        },
        EntityType::Circle(a) => Entity::Circle {
            h,
            layer,
            center: p3(&a.center),
            radius: a.radius,
        },
        EntityType::Point(p) => Entity::Point {
            h,
            layer,
            at: p3(&p.location),
        },
        EntityType::LwPolyline(p) => Entity::Polyline {
            h,
            layer,
            closed: p.is_closed,
            elevation: p.elevation,
            vertices: p
                .vertices
                .iter()
                .map(|v| (v.location.x, v.location.y, v.bulge))
                .collect(),
        },
        EntityType::Text(t) => Entity::Text {
            h,
            layer,
            at: p3(&t.insertion_point),
            height: t.height,
            rotation: t.rotation,
            value: t.value.clone(),
        },
        EntityType::MText(t) => Entity::MText {
            h,
            layer,
            at: p3(&t.insertion_point),
            height: t.height,
            value: t.value.clone(),
        },
        EntityType::Insert(i) => Entity::Insert {
            h,
            layer,
            block: i.block_name.clone(),
            at: p3(&i.insert_point),
            rotation: i.rotation,
            scale: (i.x_scale(), i.y_scale(), i.z_scale()),
        },
        EntityType::Unknown(u) => Entity::Aecc {
            h,
            class: u.dxf_name.clone(),
            layer,
        },
        other => Entity::Other {
            h,
            kind: kind_name(other),
            layer,
        },
    }
}

fn element(e: &ResolvedElement) -> AlignmentElement {
    let id = e.id();
    match &e.segment {
        Segment::Line { start, end } => AlignmentElement::Line {
            id,
            start: p2(*start),
            end: p2(*end),
        },
        Segment::Arc {
            start,
            end,
            geometry: g,
        } => AlignmentElement::Arc {
            id,
            start: p2(*start),
            end: p2(*end),
            center: p2(g.center),
            radius: g.radius,
            ccw: g.counter_clockwise,
            method: match &e.record {
                Element::Arc(a) => a.def.describe().to_string(),
                _ => "?".to_string(),
            },
        },
        Segment::Spiral {
            start,
            end,
            geometry: g,
        } => AlignmentElement::Spiral {
            id,
            start: p2(*start),
            end: p2(*end),
            length: g.curve_length(),
            start_radius: finite(g.start_radius),
            radius: finite(g.radius),
            ccw: g.counter_clockwise,
            kind: format!("{:?}", g.spiral_type),
            offset: (g.offset != 0.0).then_some(g.offset),
        },
    }
}

fn alignment(rec: &RawRecord, styles: &[(u64, String)]) -> Alignment {
    let style = alignment_refs(rec).map(|r| {
        let h = r.style.handle;
        styles
            .iter()
            .find(|(sh, n)| *sh == h && n != "(no name stored)")
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| format!("#{h:X}"))
    });
    match decode_elements(rec) {
        Ok(d) => Alignment {
            h: hx(d.handle),
            name: d.name.clone(),
            style,
            start_station: d.stored_start_station,
            length: d.length(),
            elements: d.elements.iter().map(element).collect(),
            detached: d
                .detached
                .iter()
                .map(|piece| piece.iter().map(element).collect())
                .collect(),
            groups: d
                .layout_groups
                .iter()
                .map(|g| LayoutGroup {
                    id: g.id,
                    constraint_type: g.constraint_type,
                    previous: g.previous,
                    next: g.next,
                    params: g.params.clone(),
                    locked: g.locked,
                    solved: g.solved,
                })
                .collect(),
            error: None,
        },
        Err(e) => Alignment {
            h: hx(rec.handle),
            name: crate::aecc_alignment::record_name(&rec.data),
            style,
            start_station: None,
            length: 0.0,
            elements: Vec::new(),
            detached: Vec::new(),
            groups: Vec::new(),
            error: Some(e.to_string()),
        },
    }
}

/// Every record acadrust keeps as raw bytes (AECC and other unmodelled
/// classes), sorted by handle.
pub fn raw_records(opened: &Opened) -> Vec<RawRecord> {
    raw_records_of(&opened.doc)
}

/// [`raw_records`] for a bare document.
pub fn raw_records_of(doc: &CadDocument) -> Vec<RawRecord> {
    let mut out = Vec::new();
    for e in doc.entities() {
        if let EntityType::Unknown(u) = e {
            if let Some(d) = &u.raw_dwg_data {
                out.push(RawRecord {
                    handle: u.common.handle.value(),
                    class: u.dxf_name.clone(),
                    is_entity: true,
                    data: d.clone(),
                    handle_bits: u.dwg_handle_bits.max(0) as usize,
                });
            }
        }
    }
    for (h, o) in &doc.objects {
        if let ObjectType::Unknown {
            type_name,
            raw_dwg_data: Some(d),
            raw_dwg_handle_bits,
            ..
        } = o
        {
            out.push(RawRecord {
                handle: h.value(),
                class: type_name.clone(),
                is_entity: false,
                data: d.clone(),
                handle_bits: (*raw_dwg_handle_bits).max(0) as usize,
            });
        }
    }
    out.sort_by_key(|r| r.handle);
    out
}

/// The raw record with handle `handle`, of any class.
pub fn raw_record(doc: &CadDocument, handle: u64) -> Option<RawRecord> {
    raw_records_of(doc).into_iter().find(|r| r.handle == handle)
}

/// One decoded alignment, as in the dump.
pub fn alignment_of(doc: &CadDocument, handle: u64) -> Option<Alignment> {
    let rec = raw::records(doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == handle)?;
    Some(alignment(&rec, &alignment_styles(doc)))
}

/// One alignment as RON, an element per line.
pub fn alignment_ron(a: &Alignment) -> String {
    let cfg = ron::ser::PrettyConfig::new()
        .depth_limit(2)
        .separator(" ")
        .indentor("    ")
        .struct_names(true);
    ron::ser::to_string_pretty(a, cfg).expect("dump values serialize")
}

/// Read one alignment back from [`alignment_ron`] text.
pub fn alignment_from_ron(text: &str) -> Result<Alignment, ron::error::SpannedError> {
    ron::from_str(text)
}

/// A raw record as it appears in the dump.
pub fn record_of(rec: &RawRecord) -> Record {
    Record {
        h: hx(rec.handle),
        class: rec.class.clone(),
        entity: rec.is_entity,
        handle_bits: rec.handle_bits,
        data: hex_bytes(&rec.data),
    }
}

/// One record as RON, its bytes 16 to a line inside the string (RON
/// strings may span lines).
pub fn record_ron(rec: &RawRecord) -> String {
    let mut data = String::new();
    for row in rec.data.chunks(16) {
        data.push_str("\n    ");
        data.push_str(&hex_bytes(row));
    }
    format!(
        "Record(\n    h: {:?},\n    class: {:?},\n    entity: {},\n    handle_bits: {},\n    data: \"{}\n    \",\n)\n",
        hx(rec.handle),
        rec.class,
        rec.is_entity,
        rec.handle_bits,
        data
    )
}

/// Read one record back from [`record_ron`] text: the record and its
/// bytes.
pub fn record_from_ron(text: &str) -> Result<(Record, Vec<u8>), String> {
    let r: Record = ron::from_str(text).map_err(|e| e.to_string())?;
    let bytes = record_bytes(&r)?;
    Ok((r, bytes))
}

/// The bytes of a dumped record (hex, any whitespace between bytes).
pub fn record_bytes(r: &Record) -> Result<Vec<u8>, String> {
    r.data
        .split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).map_err(|_| format!("bad byte '{b}'")))
        .collect()
}

fn hex_bytes(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 3);
    for (i, b) in data.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{b:02X}"));
    }
    s
}

/// Build the dump of an opened drawing. `source` is the name written in
/// the file; with `raw`, every raw record is listed with its bytes.
pub fn dump(opened: &Opened, source: &str, raw: bool) -> Drawing {
    let doc = &opened.doc;
    let mut entities: Vec<(u64, Entity)> = doc
        .model_space_entities()
        .map(|e| (e.common().handle.value(), entity(e)))
        .collect();
    entities.sort_by_key(|(h, _)| *h);

    let styles = alignment_styles(doc);
    let alignments = raw::records(doc, "AECC_ALIGNMENT")
        .iter()
        .map(|r| alignment(r, &styles))
        .collect();

    let records = raw_records(opened);
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for r in &records {
        *classes.entry(r.class.clone()).or_default() += 1;
    }

    Drawing {
        format: FORMAT,
        source: source.to_string(),
        version: doc.version.as_str().to_string(),
        entities: entities.into_iter().map(|(_, e)| e).collect(),
        alignments,
        classes: classes.into_iter().collect(),
        records: if raw {
            records
                .iter()
                .map(|r| Record {
                    h: hx(r.handle),
                    class: r.class.clone(),
                    entity: r.is_entity,
                    handle_bits: r.handle_bits,
                    data: hex_bytes(&r.data),
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}

/// One value on one line, `name: value` spaced for reading.
fn one_line<T: Serialize>(v: &T) -> String {
    let cfg = ron::ser::PrettyConfig::new()
        .depth_limit(0)
        .separator(" ")
        .struct_names(false);
    ron::ser::to_string_pretty(v, cfg).expect("dump values serialize")
}

/// Write `items` as a RON list, one item per line, or one item per block
/// when `nested` (its own lists one entry per line).
fn list<T: Serialize>(out: &mut String, name: &str, items: &[T], nested: bool) {
    if items.is_empty() {
        out.push_str(&format!("    {name}: [],\n"));
        return;
    }
    out.push_str(&format!("    {name}: [\n"));
    for it in items {
        let text = if nested {
            let cfg = ron::ser::PrettyConfig::new()
                .depth_limit(2)
                .separator(" ")
                .indentor("    ")
                .struct_names(false);
            ron::ser::to_string_pretty(it, cfg).expect("dump values serialize")
        } else {
            one_line(it)
        };
        for line in text.lines() {
            out.push_str("        ");
            out.push_str(line);
            out.push('\n');
        }
        // The last line closes the item; give it its list comma.
        out.pop();
        out.push_str(",\n");
    }
    out.push_str("    ],\n");
}

/// The dump as RON text: one entity, element or record per line.
pub fn to_ron(d: &Drawing) -> String {
    let mut out = String::new();
    out.push_str("// civil3drust dump (see src/dump.rs); handles are hex strings\n");
    out.push_str("Drawing(\n");
    out.push_str(&format!("    format: {},\n", d.format));
    out.push_str(&format!("    source: {},\n", one_line(&d.source)));
    out.push_str(&format!("    version: {},\n", one_line(&d.version)));
    list(&mut out, "entities", &d.entities, false);
    list(&mut out, "alignments", &d.alignments, true);
    list(&mut out, "classes", &d.classes, false);
    if !d.records.is_empty() {
        list(&mut out, "records", &d.records, false);
    }
    out.push_str(")\n");
    out
}

/// Read a dump back.
pub fn from_ron(text: &str) -> Result<Drawing, ron::error::SpannedError> {
    ron::from_str(text)
}

#[cfg(test)]
mod tests {

    #[test]
    fn record_ron_reads_back() {
        let rec = RawRecord {
            handle: 0x2A3,
            class: "AECC_ALIGNMENT".into(),
            is_entity: true,
            data: (0..=40u8).collect(),
            handle_bits: 77,
        };
        let text = record_ron(&rec);
        let (r, bytes) = record_from_ron(&text).unwrap();
        assert_eq!(bytes, rec.data);
        assert_eq!(r.h, "2A3");
        assert_eq!(r.handle_bits, 77);
        assert!(r.entity);
    }
    use super::*;

    #[test]
    fn compact_entity_reads_back() {
        let d = Drawing {
            format: FORMAT,
            source: "x.dwg".into(),
            version: "AC1032".into(),
            entities: vec![Entity::Line {
                h: "2A1".into(),
                layer: "0".into(),
                start: (0.0, 0.0, 0.0),
                end: (0.1, 100.0, 0.0),
            }],
            alignments: vec![Alignment {
                h: "3F0".into(),
                name: Some("A".into()),
                style: None,
                start_station: Some(0.0),
                length: 100.0,
                elements: vec![AlignmentElement::Spiral {
                    id: 1,
                    start: (0.0, 0.0),
                    end: (1.0, 2.0),
                    length: 3.0,
                    start_radius: None,
                    radius: Some(50.0),
                    ccw: true,
                    kind: "Clothoid".into(),
                    offset: None,
                }],
                detached: Vec::new(),
                groups: Vec::new(),
                error: None,
            }],
            classes: vec![("AECC_ALIGNMENT".into(), 1)],
            records: Vec::new(),
        };
        let text = to_ron(&d);
        assert!(text.contains(
            "Line(h: \"2A1\", layer: \"0\", start: (0.0, 0.0, 0.0), end: (0.1, 100.0, 0.0)),"
        ));
        assert_eq!(from_ron(&text).unwrap(), d);
    }
}
