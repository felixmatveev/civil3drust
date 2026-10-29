//! Loading a file into hex panes with a record overlay.
//!
//! A DWG stores its objects in the `AcDb:AcDbObjects` section. From R2004
//! on, sections are compressed in pages, so the raw file bytes cannot be
//! mapped to objects; the overlay is shown on the decompressed section
//! (acadrust's `get_section_buffer`). In R2000 (AC1015) files the section
//! is stored as is, so the raw-file pane gets the overlay too.
//!
//! Record positions come from the handle map (`AcDb:Handles`, acadrust's
//! `read_handles`). Each record is framed per the public ODA spec as
//! `MS(size) [R2010+: MC(handle bits)] data[size] CRC16`.
//!
//! Each record is then classified by who decodes it:
//! - [`Origin::Acad`]: acadrust parses it into a typed entity/object/table
//!   entry;
//! - [`Origin::Civil`]: an AECC record (acadrust keeps it opaque; civil3drust
//!   reads it);
//! - [`Origin::Opaque`]: some other class acadrust keeps as raw bytes;
//! - [`Origin::Unmapped`]: in the handle map but not in acadrust's document.

use acadrust::document::CadDocument;
use acadrust::entities::EntityType;
use acadrust::io::dwg::crc::crc16;
use acadrust::io::dwg::dwg_reader::DwgReader;
use acadrust::io::dwg::dwg_stream_readers::handle_reader::read_handles;
use acadrust::objects::ObjectType;
use acadrust::tables::TableEntry;
use acadrust::types::Handle;
use civil3drust::aecc_elements::{decode_elements, Segment};
use civil3drust::raw::{self, RawRecord};
use civil3drust::{aecc_style, handle_refs, inventory, proxy_graphics, string_stream};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    Acad,
    Civil,
    Opaque,
    Unmapped,
}

impl Origin {
    pub fn describe(self) -> &'static str {
        match self {
            Origin::Acad => "decoded by acadrust",
            Origin::Civil => "AECC record (civil3drust)",
            Origin::Opaque => "kept as raw bytes by acadrust",
            Origin::Unmapped => "in handle map, not in acadrust's document",
        }
    }
}

/// One decoded value shown in a record's detail window.
#[derive(Debug, Clone)]
pub struct Field {
    pub label: String,
    pub value: String,
    /// Bit offset inside the record data, when the value has a location.
    pub bit: Option<usize>,
    /// Handle this value refers to (clicking jumps there).
    pub target: Option<u64>,
}

impl Field {
    fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Field {
            label: label.into(),
            value: value.into(),
            bit: None,
            target: None,
        }
    }
    fn at(mut self, bit: usize) -> Self {
        self.bit = Some(bit);
        self
    }
    fn to(mut self, handle: u64) -> Self {
        self.target = Some(handle);
        self
    }
}

/// One object record located in a pane.
#[derive(Debug, Clone)]
pub struct Span {
    /// Pane offset of the record frame (the MS size field).
    pub start: usize,
    /// Whole frame length: size fields + data + CRC.
    pub len: usize,
    /// Pane offset of the record data (what acadrust calls raw_dwg_data).
    pub data_start: usize,
    pub data_len: usize,
    /// Size of the handle stream in bits (R2010+), 0 otherwise.
    pub handle_bits: usize,
    pub handle: u64,
    pub type_code: Option<u16>,
    pub class: String,
    pub name: Option<String>,
    pub origin: Origin,
    pub crc_stored: u16,
    pub crc_ok: bool,
    pub fields: Vec<Field>,
}

impl Span {
    pub fn end(&self) -> usize {
        self.start + self.len
    }

    /// Text drawn in the overlay: `CLASS : name` or `CLASS : #HANDLE`.
    pub fn label(&self) -> String {
        match &self.name {
            Some(n) if !n.is_empty() => format!("{} : {}", self.class, n),
            _ => format!("{} : #{:X}", self.class, self.handle),
        }
    }

    /// Pane offset of a bit inside the record data.
    pub fn byte_of_bit(&self, bit: usize) -> usize {
        self.data_start + bit / 8
    }

    fn shifted(&self, delta: usize) -> Span {
        let mut s = self.clone();
        s.start += delta;
        s.data_start += delta;
        s
    }
}

/// One byte buffer shown in the hex view.
pub struct Pane {
    pub title: String,
    pub bytes: Vec<u8>,
    pub spans: Vec<Span>,
    /// Span index per byte (`u32::MAX` = none).
    owner: Vec<u32>,
    pub note: String,
}

impl Pane {
    pub fn new(
        title: impl Into<String>,
        bytes: Vec<u8>,
        mut spans: Vec<Span>,
        note: impl Into<String>,
    ) -> Pane {
        spans.sort_by_key(|s| s.start);
        let mut owner = vec![u32::MAX; bytes.len()];
        for (i, s) in spans.iter().enumerate() {
            let end = s.end().min(bytes.len());
            for o in &mut owner[s.start.min(end)..end] {
                *o = i as u32;
            }
        }
        Pane {
            title: title.into(),
            bytes,
            spans,
            owner,
            note: note.into(),
        }
    }

    pub fn span_at(&self, off: usize) -> Option<usize> {
        match self.owner.get(off) {
            Some(&i) if i != u32::MAX => Some(i as usize),
            _ => None,
        }
    }

    pub fn find_handle(&self, handle: u64) -> Option<usize> {
        self.spans.iter().position(|s| s.handle == handle)
    }
}

pub struct Loaded {
    pub path: String,
    pub version: String,
    pub panes: Vec<Pane>,
    pub warnings: Vec<String>,
    pub doc: Option<CadDocument>,
}

/// What the document knows about one handle.
struct Known {
    class: String,
    name: Option<String>,
    origin: Origin,
    fields: Vec<Field>,
}

pub fn load(path: impl AsRef<Path>) -> Result<Loaded, String> {
    let path = path.as_ref();
    let raw_bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut loaded = Loaded {
        path: path.display().to_string(),
        version: String::new(),
        panes: Vec::new(),
        warnings: Vec::new(),
        doc: None,
    };
    let is_dwg = raw_bytes.starts_with(b"AC10") || raw_bytes.starts_with(b"AC1.");
    if !is_dwg {
        loaded.version = "not a DWG".into();
        loaded.panes.push(Pane::new(
            "Raw file",
            raw_bytes,
            Vec::new(),
            "No record overlay: only DWG files are mapped.",
        ));
        return Ok(loaded);
    }
    loaded.version = String::from_utf8_lossy(&raw_bytes[..6.min(raw_bytes.len())]).into_owned();

    match load_dwg(path, &raw_bytes, &mut loaded) {
        Ok(()) => {}
        Err(e) => {
            loaded.warnings.push(format!("DWG mapping failed: {e}"));
            loaded.panes.push(Pane::new(
                "Raw file",
                raw_bytes,
                Vec::new(),
                "No record overlay (mapping failed).",
            ));
        }
    }
    Ok(loaded)
}

fn load_dwg(path: &Path, raw_bytes: &[u8], loaded: &mut Loaded) -> Result<(), String> {
    let mut reader = DwgReader::from_file(path).map_err(|e| e.to_string())?;
    let info = reader.read_file_header().map_err(|e| e.to_string())?;
    let r2010 = info.version.r2010_plus();

    // Every section we can name, in file order where known.
    let mut names: Vec<String> = if !info.section_locators.is_empty() {
        let mut v: Vec<_> = info.section_locators.iter().collect();
        v.sort_by_key(|(_, (off, _))| *off);
        v.into_iter().map(|(n, _)| n.clone()).collect()
    } else {
        info.section_descriptors
            .iter()
            .map(|s| s.name.clone())
            .collect()
    };
    names.retain(|n| !n.is_empty());

    let objects = reader
        .get_section_buffer("AcDb:AcDbObjects", &info)
        .map_err(|e| format!("objects section: {e}"))?;
    let handles = reader
        .get_section_buffer("AcDb:Handles", &info)
        .map_err(|e| format!("handles section: {e}"))?;
    let mut map = read_handles(&handles).map_err(|e| format!("handle map: {e}"))?;
    let base = info.objects_base_offset;
    if base != 0 {
        for v in map.values_mut() {
            *v -= base;
        }
    }

    // The parsed document: who decodes what.
    let (doc, known) = match raw::open(path) {
        Ok(o) => {
            if let Some(r) = &o.failsafe_reason {
                loaded
                    .warnings
                    .push(format!("acadrust needed failsafe mode ({r}); some records are missing from its document"));
            }
            let known = index_doc(&o.doc);
            (Some(o.doc), known)
        }
        Err(e) => {
            loaded
                .warnings
                .push(format!("acadrust could not read the document: {e}"));
            (None, HashMap::new())
        }
    };
    let class_numbers: HashMap<u16, String> = doc
        .as_ref()
        .map(|d| {
            d.classes
                .iter()
                .map(|c| (c.class_number as u16, c.dxf_name.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Frame every record listed in the handle map.
    let mut spans = Vec::with_capacity(map.len());
    let mut bad = 0usize;
    for (&handle, &off) in &map {
        let Some(mut span) = frame(&objects, off, r2010, handle) else {
            bad += 1;
            continue;
        };
        let type_class = span.type_code.map(|t| type_name(t, &class_numbers));
        match known.get(&handle) {
            Some(k) => {
                span.class = k.class.clone();
                span.name = k.name.clone();
                span.origin = k.origin;
                span.fields = k.fields.clone();
            }
            None => {
                span.class = type_class.clone().unwrap_or_else(|| "?".into());
                span.origin = Origin::Unmapped;
            }
        }
        if let Some(t) = span.type_code {
            let tc = type_class.unwrap_or_default();
            span.fields
                .insert(0, Field::new("DWG type code", format!("{t} ({tc})")));
        }
        spans.push(span);
    }
    if bad > 0 {
        loaded.warnings.push(format!(
            "{bad} handle-map entries point outside the objects section"
        ));
    }

    // AC15: the objects section sits in the file unchanged at `base`.
    let raw_spans: Vec<Span> = if base > 0 && !info.is_ac18_format {
        spans.iter().map(|s| s.shifted(base as usize)).collect()
    } else {
        Vec::new()
    };
    let raw_note = if raw_spans.is_empty() {
        "Raw file bytes. Sections are compressed in this version, so records are mapped in the AcDb:AcDbObjects pane."
    } else {
        "Raw file bytes; the objects section is stored uncompressed (R2000), so records are mapped here too."
    };

    let n_spans = spans.len();
    loaded.panes.push(Pane::new(
        "AcDb:AcDbObjects",
        objects,
        spans,
        format!("Decompressed objects section, {n_spans} records from the handle map."),
    ));
    loaded.panes.push(Pane::new(
        "Raw file",
        raw_bytes.to_vec(),
        raw_spans,
        raw_note,
    ));
    loaded.panes.push(Pane::new(
        "AcDb:Handles",
        handles,
        Vec::new(),
        "Handle map: pages of (handle delta, offset delta) modular chars.",
    ));
    for n in names {
        if n == "AcDb:AcDbObjects" || n == "AcDb:Handles" {
            continue;
        }
        match reader.get_section_buffer(&n, &info) {
            Ok(b) if !b.is_empty() => {
                loaded
                    .panes
                    .push(Pane::new(n.clone(), b, Vec::new(), "Decompressed section."))
            }
            Ok(_) => {}
            Err(e) => loaded.warnings.push(format!("{n}: {e}")),
        }
    }
    loaded.doc = doc;
    Ok(())
}

/// Read the frame of the record at `off` in the objects section.
fn frame(buf: &[u8], off: i64, r2010: bool, handle: u64) -> Option<Span> {
    let start = usize::try_from(off).ok()?;
    let mut pos = start;
    let size = read_modular(buf, &mut pos, 2, 15)?;
    let handle_bits = if r2010 {
        read_modular(buf, &mut pos, 1, 7)?
    } else {
        0
    };
    let data_start = pos;
    let crc_at = data_start.checked_add(size)?;
    if crc_at + 2 > buf.len() {
        return None;
    }
    let crc_stored = u16::from_le_bytes([buf[crc_at], buf[crc_at + 1]]);
    let crc_ok = crc16(0xC0C1, &buf[start..crc_at]) == crc_stored;
    let data = &buf[data_start..crc_at];
    Some(Span {
        start,
        len: crc_at + 2 - start,
        data_start,
        data_len: size,
        handle_bits,
        handle,
        type_code: read_type(data, r2010),
        class: String::new(),
        name: None,
        origin: Origin::Unmapped,
        crc_stored,
        crc_ok,
        fields: Vec::new(),
    })
}

/// Modular short (width 2, 15 payload bits) or modular char (1, 7).
fn read_modular(buf: &[u8], pos: &mut usize, width: usize, bits: u32) -> Option<usize> {
    let mut value = 0usize;
    let mut shift = 0u32;
    loop {
        let w = buf.get(*pos..*pos + width)?;
        let word = if width == 2 {
            u16::from_le_bytes([w[0], w[1]]) as usize
        } else {
            w[0] as usize
        };
        value |= (word & ((1 << bits) - 1)).checked_shl(shift)?;
        *pos += width;
        if word & (1 << bits) == 0 {
            return Some(value);
        }
        shift += bits;
        if shift > 48 {
            return None;
        }
    }
}

/// Object type at the start of the record data: OT (R2010+) or BS.
fn read_type(data: &[u8], r2010: bool) -> Option<u16> {
    if raw::bit_len(data) < 18 {
        return None;
    }
    let b0 = raw::get_bit(data, 0);
    let b1 = raw::get_bit(data, 1);
    let byte = |bit| raw::get_byte(data, bit) as u16;
    Some(if r2010 {
        match (b0, b1) {
            (0, 0) => byte(2),
            (0, 1) => byte(2) + 0x1F0,
            _ => byte(2) | byte(10) << 8,
        }
    } else {
        match (b0, b1) {
            (0, 0) => byte(2) | byte(10) << 8,
            (0, 1) => byte(2),
            (1, 0) => 0,
            _ => 256,
        }
    })
}

/// DWG class name for a type code (fixed types from the public ODA spec,
/// custom classes from the drawing's class table).
fn type_name(code: u16, classes: &HashMap<u16, String>) -> String {
    let fixed = match code {
        1 => "TEXT",
        2 => "ATTRIB",
        3 => "ATTDEF",
        4 => "BLOCK",
        5 => "ENDBLK",
        6 => "SEQEND",
        7 => "INSERT",
        8 => "MINSERT",
        10 => "VERTEX_2D",
        11 => "VERTEX_3D",
        12 => "VERTEX_MESH",
        13 => "VERTEX_PFACE",
        14 => "VERTEX_PFACE_FACE",
        15 => "POLYLINE_2D",
        16 => "POLYLINE_3D",
        17 => "ARC",
        18 => "CIRCLE",
        19 => "LINE",
        20 => "DIMENSION_ORDINATE",
        21 => "DIMENSION_LINEAR",
        22 => "DIMENSION_ALIGNED",
        23 => "DIMENSION_ANG3PT",
        24 => "DIMENSION_ANG2LN",
        25 => "DIMENSION_RADIUS",
        26 => "DIMENSION_DIAMETER",
        27 => "POINT",
        28 => "3DFACE",
        29 => "POLYLINE_PFACE",
        30 => "POLYLINE_MESH",
        31 => "SOLID",
        32 => "TRACE",
        33 => "SHAPE",
        34 => "VIEWPORT",
        35 => "ELLIPSE",
        36 => "SPLINE",
        37 => "REGION",
        38 => "3DSOLID",
        39 => "BODY",
        40 => "RAY",
        41 => "XLINE",
        42 => "DICTIONARY",
        43 => "OLEFRAME",
        44 => "MTEXT",
        45 => "LEADER",
        46 => "TOLERANCE",
        47 => "MLINE",
        48 => "BLOCK_CONTROL",
        49 => "BLOCK_HEADER",
        50 => "LAYER_CONTROL",
        51 => "LAYER",
        52 => "STYLE_CONTROL",
        53 => "STYLE",
        56 => "LTYPE_CONTROL",
        57 => "LTYPE",
        60 => "VIEW_CONTROL",
        61 => "VIEW",
        62 => "UCS_CONTROL",
        63 => "UCS",
        64 => "VPORT_CONTROL",
        65 => "VPORT",
        66 => "APPID_CONTROL",
        67 => "APPID",
        68 => "DIMSTYLE_CONTROL",
        69 => "DIMSTYLE",
        70 => "VX_CONTROL",
        71 => "VX_TABLE_RECORD",
        72 => "GROUP",
        73 => "MLINESTYLE",
        74 => "OLE2FRAME",
        76 => "LONG_TRANSACTION",
        77 => "LWPOLYLINE",
        78 => "HATCH",
        79 => "XRECORD",
        80 => "ACDBPLACEHOLDER",
        81 => "VBA_PROJECT",
        82 => "LAYOUT",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_string();
    }
    classes
        .get(&code)
        .cloned()
        .unwrap_or_else(|| format!("type {code}"))
}

/// A `fmt::Write` that stops after the first word, so taking an enum
/// variant name from `Debug` does not format the whole object.
struct FirstWord(String);
impl std::fmt::Write for FirstWord {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        for c in s.chars() {
            if c.is_alphanumeric() || c == '_' {
                self.0.push(c);
            } else {
                return Err(std::fmt::Error);
            }
        }
        Ok(())
    }
}

fn variant_name(v: &impl std::fmt::Debug) -> String {
    let mut w = FirstWord(String::new());
    let _ = write!(w, "{v:?}");
    w.0
}

fn index_doc(doc: &CadDocument) -> HashMap<u64, Known> {
    let mut m: HashMap<u64, Known> = HashMap::new();
    let acad = |class: String, name: Option<String>| Known {
        class,
        name,
        origin: Origin::Acad,
        fields: Vec::new(),
    };

    for e in doc.entities() {
        let h = e.common().handle.value();
        if let EntityType::Unknown(u) = e {
            let origin = if inventory::is_aecc(&u.dxf_name, "") {
                Origin::Civil
            } else {
                Origin::Opaque
            };
            m.insert(
                h,
                Known {
                    class: u.dxf_name.clone(),
                    name: None,
                    origin,
                    fields: Vec::new(),
                },
            );
        } else {
            let mut k = acad(e.as_entity().entity_type().to_ascii_uppercase(), None);
            k.fields
                .push(Field::new("Layer", e.as_entity().layer().to_string()));
            m.insert(h, k);
        }
    }
    for (h, o) in &doc.objects {
        let k = match o {
            ObjectType::Unknown { type_name, .. } => Known {
                class: type_name.clone(),
                name: None,
                origin: if inventory::is_aecc(type_name, "") {
                    Origin::Civil
                } else {
                    Origin::Opaque
                },
                fields: Vec::new(),
            },
            other => acad(variant_name(other).to_ascii_uppercase(), None),
        };
        m.insert(h.value(), k);
    }

    macro_rules! table {
        ($t:expr, $control:literal, $entry:literal) => {
            m.insert($t.handle().value(), acad($control.into(), None));
            for e in $t.iter() {
                m.insert(
                    e.handle().value(),
                    acad($entry.into(), Some(e.name().to_string())),
                );
            }
        };
    }
    table!(doc.layers, "LAYER_CONTROL", "LAYER");
    table!(doc.line_types, "LTYPE_CONTROL", "LTYPE");
    table!(doc.text_styles, "STYLE_CONTROL", "STYLE");
    table!(doc.block_records, "BLOCK_CONTROL", "BLOCK_HEADER");
    table!(doc.dim_styles, "DIMSTYLE_CONTROL", "DIMSTYLE");
    table!(doc.app_ids, "APPID_CONTROL", "APPID");
    table!(doc.views, "VIEW_CONTROL", "VIEW");
    table!(doc.vports, "VPORT_CONTROL", "VPORT");
    table!(doc.ucss, "UCS_CONTROL", "UCS");

    // civil3drust: everything it can say about AECC records.
    for class in inventory::aecc_classes(doc) {
        for rec in raw::records(doc, &class.dxf_name) {
            if let Some(k) = m.get_mut(&rec.handle) {
                let (name, fields) = civil_fields(&rec);
                k.name = name;
                k.fields = fields;
            }
        }
    }
    m
}

fn civil_fields(rec: &RawRecord) -> (Option<String>, Vec<Field>) {
    let data = &rec.data;
    let mut f = Vec::new();
    let mut name = raw::object_name(data);

    f.push(Field::new(
        "Record data",
        format!(
            "{} bytes, handle stream {} bits",
            data.len(),
            rec.handle_bits
        ),
    ));
    if rec.handle_bits > 0 {
        f.push(
            Field::new(
                "Handle stream start",
                format!("bit {}", raw::bit_len(data) - rec.handle_bits),
            )
            .at(raw::bit_len(data) - rec.handle_bits),
        );
    }

    if rec.class == "AECC_ALIGNMENT" {
        match decode_elements(rec) {
            Ok(d) => {
                if d.name.is_some() {
                    name = d.name.clone();
                }
                if let Some(s) = d.stored_start_station {
                    f.push(Field::new("Start station", format!("{s:.4}")));
                }
                f.push(Field::new("Length", format!("{:.4}", d.length())));
                for (i, el) in d.elements.iter().enumerate() {
                    let bit = el.record.header_bit();
                    let v = match &el.segment {
                        Segment::Line { start, end } => format!(
                            "line ({:.3}, {:.3}) → ({:.3}, {:.3}), L={:.4}",
                            start.x,
                            start.y,
                            end.x,
                            end.y,
                            el.segment.length()
                        ),
                        Segment::Arc {
                            start,
                            end,
                            geometry,
                        } => format!(
                            "arc ({:.3}, {:.3}) → ({:.3}, {:.3}), R={:.4}, L={:.4}",
                            start.x, start.y, end.x, end.y, geometry.radius, geometry.length
                        ),
                        Segment::Spiral {
                            start,
                            end,
                            geometry,
                        } => format!(
                            "spiral ({:.3}, {:.3}) → ({:.3}, {:.3}), into R={:.4}, L={:.4}",
                            start.x, start.y, end.x, end.y, geometry.radius, geometry.length
                        ),
                    };
                    f.push(Field::new(format!("Element {i}"), v).at(bit));
                }
            }
            Err(e) => f.push(Field::new("Alignment decode", format!("failed: {e}"))),
        }
    }
    if rec.class == "AECC_ALIGNMENT_STYLE" {
        if let Some(flags) = aecc_style::flags_string(data) {
            f.push(Field::new("Component visibility", flags));
        }
    }

    if let Ok(ss) = string_stream::parse(data, rec.handle_bits) {
        f.push(Field::new("String stream", format!("{} strings", ss.strings.len())).at(ss.start));
        for (i, s) in ss.strings.iter().enumerate() {
            f.push(Field::new(format!("String {i}"), format!("{:?}", s.text)).at(s.bit));
        }
    }
    if rec.is_entity {
        if let Some(g) = proxy_graphics::find(data, 8192) {
            f.push(
                Field::new(
                    "Proxy graphics",
                    format!(
                        "{} bytes, {} chunks{}",
                        g.size,
                        g.chunks.len(),
                        if g.is_placeholder() {
                            " (placeholder)"
                        } else {
                            ""
                        }
                    ),
                )
                .at(g.start_bit),
            );
        }
    }
    for r in handle_refs::of_record(rec) {
        f.push(
            Field::new(format!("Ref code {}", r.code), format!("#{:X}", r.handle))
                .at(r.bit)
                .to(r.handle),
        );
    }
    (name, f)
}

/// Full acadrust view of one handle, for the detail window.
pub fn dump(doc: &CadDocument, handle: u64) -> String {
    let h = Handle::new(handle);
    if let Some(e) = doc.get_entity(h) {
        return match e {
            EntityType::Unknown(u) => format!(
                "UnknownEntity {{ dxf_name: {:?}, dwg_type_code: {}, layer: {:?}, owner: {:?}, raw bytes: {} }}",
                u.dxf_name,
                u.dwg_type_code,
                u.common.layer,
                u.common.owner_handle,
                u.raw_dwg_data.as_ref().map_or(0, Vec::len)
            ),
            other => format!("{other:#?}"),
        };
    }
    if let Some(o) = doc.objects.get(&h) {
        return match o {
            ObjectType::Unknown {
                type_name, owner, ..
            } => format!("Unknown object {{ type_name: {type_name:?}, owner: {owner:?} }}"),
            other => format!("{other:#?}"),
        };
    }
    macro_rules! find {
        ($t:expr) => {
            if $t.handle() == h {
                return format!("table control, {} entries", $t.len());
            }
            if let Some(e) = $t.iter().find(|e| e.handle() == h) {
                return format!("{e:#?}");
            }
        };
    }
    find!(doc.layers);
    find!(doc.line_types);
    find!(doc.text_styles);
    find!(doc.block_records);
    find!(doc.dim_styles);
    find!(doc.app_ids);
    find!(doc.views);
    find!(doc.vports);
    find!(doc.ucss);
    "(acadrust has no object with this handle)".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modular_short() {
        let mut p = 0;
        assert_eq!(read_modular(&[0x2A, 0x00], &mut p, 2, 15), Some(42));
        assert_eq!(p, 2);
        // 0x8001 continues; second word 0x0001 → 1 | 1 << 15
        let mut p = 0;
        assert_eq!(
            read_modular(&[0x01, 0x80, 0x01, 0x00], &mut p, 2, 15),
            Some(1 | 1 << 15)
        );
    }

    #[test]
    fn type_codes() {
        // BS 01 + byte 19 (LINE): bits 01 00010011 → 0x44 0xC0
        assert_eq!(read_type(&[0x44, 0xC0, 0, 0], false), Some(19));
        // OT 01 + byte 0x10 → 0x1F0 + 0x10 = 512
        assert_eq!(read_type(&[0x44, 0x00, 0, 0], true), Some(0x1F0 + 0x10));
    }
}
