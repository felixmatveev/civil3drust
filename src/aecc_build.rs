//! Rewriting an alignment's geometry with any number of elements.
//!
//! Layout used (docs/alignment-object.md, "Element list, cache and
//! graphics"; S24/S25/S44 and S1–S15):
//!
//! - **Graphics size** — after the object header, at bit 42: EED size `10`,
//!   has-graphics `1`, then (bit 45) a bit-coded long long (3-bit byte count +
//!   little-endian bytes) giving the proxy graphics size in bytes, then the
//!   graphics.
//! - **Identity** — a version-4 GUID, 16 bits after the last element
//!   record and again further on ([`set_identity`]).
//! - **Element list** — a bit-coded long element **count** (`01` + byte),
//!   then per element, in ID order, an 11-bit **kind prefix**
//!   (`00000000010` tangent, `00000000100` arc) and the element record.
//! - After the last record: 161 unchanged bits, the BD **station at the end
//!   of element 0**, then a **cache** of element 0: BL 1, BL 0, BL id of the
//!   element after it, and three raw points: element 0's start, its end,
//!   and the start again.
//!
//! - **Strings** — each free arc reads three empty strings from the string
//!   stream, so its string count is 12 + 3 × arcs for a plain alignment.
//!
//! [`set_geometry`] rebuilds the graphics (one 2-point polyline chunk per
//! tangent, in the donor's chunk format, after the donor's leading
//! attribute chunks), the element list and that cache from a PI list with
//! optional radii (free arcs), copying everything else.

use crate::aecc_alignment::TangentRecord;
use crate::aecc_alignment::ELEMENT_HEADER;
use crate::aecc_arc_edit::station_of;
use crate::aecc_elements::{
    decode_elements, resolve, ArcDef, ArcParams, ArcRecord, Element, Segment,
};
use crate::alignment::Point2;
use crate::proxy_graphics as pg;
use crate::raw::{self, RawRecord};
use crate::string_stream;
use std::fmt;

/// Kind prefixes before each element record.
pub const PREFIX_TANGENT: u16 = 0b000_0000_0010;
pub const PREFIX_ARC: u16 = 0b000_0000_0100;
/// Parameters of every free arc (fillet) Civil 3D wrote in S11–S16.
pub const FREE_ARC_PARAMS: ArcParams = ArcParams {
    p1: -1.0,
    p2: -1.0,
    n1: 1,
    p3: -1.0,
    p4: None,
    n2: 29,
    closed: false,
};
/// Bit of the EED-size field (`10`, no EED); has-graphics follows, then
/// the graphics size (BLL) at +3.
const EED_BIT_USUAL: usize = 42;

/// The EED bit of `data` ([`crate::aecc_entity::eed_bit`]; 42 for the
/// usual class numbers and handles).
fn eed(data: &[u8]) -> usize {
    crate::aecc_entity::eed_bit(data).unwrap_or(EED_BIT_USUAL)
}
const GAP_AFTER_ELEMENTS: usize = 161;

#[derive(Debug, Clone, PartialEq)]
pub enum BuildError {
    Layout(String),
    Geometry(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(w) => write!(f, "record layout: {w}"),
            Self::Geometry(w) => write!(f, "geometry: {w}"),
        }
    }
}

impl std::error::Error for BuildError {}

fn layout(msg: impl Into<String>) -> BuildError {
    BuildError::Layout(msg.into())
}

/// Bit writer (MSB first, as DWG streams are read).
#[derive(Default)]
pub struct Bits(pub Vec<u8>);

impl Bits {
    pub fn bit(&mut self, b: u8) {
        self.0.push(b & 1);
    }
    pub fn bits(&mut self, v: u64, n: usize) {
        for k in (0..n).rev() {
            self.0.push(((v >> k) & 1) as u8);
        }
    }
    pub fn byte(&mut self, v: u8) {
        self.bits(v as u64, 8);
    }
    pub fn copy(&mut self, data: &[u8], from: usize, to: usize) {
        self.0.extend((from..to).map(|b| raw::get_bit(data, b)));
    }
    /// Bit-coded long.
    pub fn bl(&mut self, v: i32) {
        if v == 0 {
            self.bits(0b10, 2);
        } else if (1..256).contains(&v) {
            self.bits(0b01, 2);
            self.byte(v as u8);
        } else {
            self.bits(0b00, 2);
            v.to_le_bytes().iter().for_each(|b| self.byte(*b));
        }
    }
    /// Bit-coded short (string lengths): `10` for 0, `01` + byte for
    /// 1–255, `00` + 16 bits little-endian otherwise.
    pub fn bl_short(&mut self, v: usize) {
        match v {
            0 => self.bits(0b10, 2),
            1..=255 => {
                self.bits(0b01, 2);
                self.byte(v as u8);
            }
            _ => {
                self.bits(0b00, 2);
                self.byte((v & 0xFF) as u8);
                self.byte((v >> 8) as u8);
            }
        }
    }
    /// Raw double.
    pub fn rd(&mut self, v: f64) {
        v.to_le_bytes().iter().for_each(|b| self.byte(*b));
    }
    /// Bit-coded double.
    pub fn bd(&mut self, v: f64) {
        if v == 1.0 {
            self.bits(0b01, 2);
        } else if v == 0.0 {
            self.bits(0b10, 2);
        } else {
            self.bits(0b00, 2);
            self.rd(v);
        }
    }
    /// Raw 32-bit little-endian integer.
    pub fn rl(&mut self, v: i32) {
        v.to_le_bytes().iter().for_each(|b| self.byte(*b));
    }
    /// Bit-coded long long: 3-bit byte count + little-endian bytes.
    pub fn bll(&mut self, v: u64) {
        let n = (8 - v.leading_zeros() as usize / 8).min(7);
        let n = if v == 0 { 0 } else { n };
        self.bits(n as u64, 3);
        for i in 0..n {
            self.byte((v >> (8 * i)) as u8);
        }
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Pad to a byte and pack.
    pub fn into_bytes(mut self) -> Vec<u8> {
        while !self.0.len().is_multiple_of(8) {
            self.0.push(0);
        }
        let mut out = vec![0u8; self.0.len() / 8];
        for (i, b) in self.0.iter().enumerate() {
            out[i / 8] |= b << (7 - i % 8);
        }
        out
    }
}

/// Read a bit-coded long long at `bit`: (value, next bit).
pub fn read_bll(data: &[u8], bit: usize) -> (u64, usize) {
    let n = (raw::get_bit(data, bit) as usize) << 2
        | (raw::get_bit(data, bit + 1) as usize) << 1
        | raw::get_bit(data, bit + 2) as usize;
    let mut v = 0u64;
    for i in 0..n {
        v |= (raw::get_byte(data, bit + 3 + 8 * i) as u64) << (8 * i);
    }
    (v, bit + 3 + 8 * n)
}

/// Encode a tangent record (without its prefix).
pub fn encode_tangent(w: &mut Bits, id: i32, before: i32, after: i32, a: Point2, b: Point2) {
    w.bits(0b00, 2);
    w.rl(ELEMENT_HEADER);
    w.bl(0);
    w.bl(id);
    w.bl(before);
    w.bl(after);
    for v in [a.x, a.y, b.x, b.y] {
        w.rd(v);
    }
}

/// Encode a free-arc record (without its prefix).
pub fn encode_free_arc(
    w: &mut Bits,
    flag: bool,
    params: &ArcParams,
    ids: (i32, i32, i32),
    radius: f64,
) {
    w.bits(0b00, 2);
    w.rl(ELEMENT_HEADER);
    w.bit(flag as u8);
    w.bd(params.p1);
    w.bd(params.p2);
    w.bl(params.n1);
    w.bd(params.p3);
    if let Some(p4) = params.p4 {
        w.bd(p4);
    }
    w.bl(params.n2);
    w.bit(params.closed as u8);
    w.bl(ids.0);
    w.bl(ids.1);
    w.bl(ids.2);
    w.bd(radius);
}

/// The element list Civil 3D would store for `pis` with optional free
/// arcs at interior PIs (`radii[k]` for PI `k + 1`): tangents get IDs
/// 1..n−1 in order, arcs the following IDs in order; records are listed by
/// ID and linked in path order (S11: T1 A4 T2 T3).
pub fn plan(pis: &[Point2], radii: &[Option<f64>]) -> Result<Vec<Element>, BuildError> {
    if pis.len() < 2 {
        return Err(BuildError::Geometry("need at least two PIs".into()));
    }
    let n_t = pis.len() - 1;
    if radii.len() > n_t.saturating_sub(1) {
        return Err(BuildError::Geometry(format!(
            "{} radii for {} interior PIs",
            radii.len(),
            n_t - 1
        )));
    }
    // Path order: T1, (A), T2, (A), ...
    let mut next_arc = n_t as i32 + 1;
    let mut path: Vec<(char, i32, usize)> = Vec::new(); // kind, id, index
    for k in 0..n_t {
        path.push(('T', k as i32 + 1, k));
        if let Some(Some(r)) = radii.get(k) {
            if !(r.is_finite() && *r > 0.0) {
                return Err(BuildError::Geometry(format!("bad radius {r}")));
            }
            path.push(('A', next_arc, k));
            next_arc += 1;
        }
    }
    let mut els: Vec<Element> = Vec::new();
    for (i, &(kind, id, k)) in path.iter().enumerate() {
        let before = if i == 0 { 0 } else { path[i - 1].1 };
        let after = path.get(i + 1).map(|p| p.1).unwrap_or(0);
        els.push(match kind {
            'T' => Element::Tangent(TangentRecord {
                header_bit: 0,
                kind: crate::aecc_alignment::KIND_FIXED,
                coords_bit: 0,
                id,
                before,
                after,
                start: pis[k],
                end: pis[k + 1],
                length: None,
                record_end: 0,
            }),
            _ => Element::Arc(ArcRecord {
                header_bit: 0,
                end_bit: 0,
                id,
                before,
                after,
                flag: false,
                params: FREE_ARC_PARAMS,
                def: ArcDef::Free {
                    radius_bit: 0,
                    radius: radii[k].unwrap(),
                },
            }),
        });
    }
    els.sort_by_key(crate::aecc_alignment::Linked::id);
    Ok(els)
}

/// What [`set_geometry`] wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryReport {
    pub elements: usize,
    pub length: f64,
    pub graphics_rebuilt: bool,
}

/// Rewrite the geometry of the alignment record `rec` as `pis` with free
/// arcs `radii` (see [`plan`]). Returns the new record bytes and
/// handle-stream size.
pub fn set_geometry(
    rec: &RawRecord,
    pis: &[Point2],
    radii: &[Option<f64>],
) -> Result<(Vec<u8>, usize, GeometryReport), BuildError> {
    let data = &rec.data;
    let eed_bit = eed(data);
    let decoded = decode_elements(rec).map_err(|e| layout(e.to_string()))?;
    if !decoded.detached.is_empty() {
        return Err(layout(
            "the alignment has detached pieces; rebuilding from PIs would drop them",
        ));
    }
    let start_station = decoded.stored_start_station.unwrap_or(0.0);

    // --- New elements, resolved and checked. ---
    let mut els = plan(pis, radii)?;
    let path = path_order(&els);
    let resolved = resolve(&path).map_err(|e| BuildError::Geometry(e.to_string()))?;
    for (rec_e, seg) in path.iter().zip(&resolved) {
        if let (Element::Tangent(t), Segment::Line { start, end }) = (rec_e, seg.segment) {
            let (dx, dy) = (t.end.x - t.start.x, t.end.y - t.start.y);
            if (end.x - start.x) * dx + (end.y - start.y) * dy < -1e-9 * (dx * dx + dy * dy) {
                return Err(BuildError::Geometry(format!(
                    "curves don't fit on tangent {}",
                    t.id
                )));
            }
        }
    }
    let length: f64 = resolved.iter().map(|e| e.segment.length()).sum();
    // Keep the donor's arc flag when it has an arc.
    if let Some(Element::Arc(a0)) = decoded
        .elements
        .iter()
        .map(|e| &e.record)
        .find(|e| matches!(e, Element::Arc(_)))
    {
        for e in &mut els {
            if let Element::Arc(a) = e {
                a.flag = a0.flag;
            }
        }
    }

    // --- Old layout. ---
    let g = pg::find(data, 4096).ok_or_else(|| layout("no proxy graphics"))?;
    let (gsize, gstart) = read_bll(data, eed_bit + 3);
    if raw::get_bit(data, eed_bit) != 1
        || raw::get_bit(data, eed_bit + 1) != 0
        || raw::get_bit(data, eed_bit + 2) != 1
        || gstart != g.start_bit
        || gsize as usize != g.size
    {
        return Err(layout("graphics size field not where expected"));
    }
    let bounds: Vec<(usize, usize)> = decoded
        .elements
        .iter()
        .map(|e| (e.record.header_bit(), e.record.end_bit()))
        .collect();
    let first = bounds.iter().map(|b| b.0).min().unwrap();
    let last_end = bounds.iter().map(|b| b.1).max().unwrap();
    let count_bit = first - 11 - 10;
    if raw::get_bit(data, count_bit) != 0
        || raw::get_bit(data, count_bit + 1) != 1
        || raw::get_byte(data, count_bit + 2) as usize != decoded.elements.len()
    {
        return Err(layout("element count not where expected"));
    }
    let end0_bit = last_end + GAP_AFTER_ELEMENTS;
    let old_end0 = station_of(&decoded.elements, 1, start_station);
    let plain_layout = decoded.elements.iter().all(|e| match &e.record {
        Element::Tangent(t) => t.kind == crate::aecc_alignment::KIND_FIXED,
        Element::Arc(a) => matches!(a.def, ArcDef::Free { .. }),
        Element::LineSpiral(_) => false,
    }) && raw::get_bit(data, end0_bit) == 0
        && raw::get_bit(data, end0_bit + 1) == 0
        && (raw::get_f64(data, end0_bit + 2) - old_end0).abs() <= 1e-9 * old_end0.abs().max(1.0);
    if !plain_layout {
        // Other templates (floating elements, no speed interval, edited
        // element lists): replace the element list in place and rebuild
        // the derived parts from the geometry.
        let (bytes, hb, rep) =
            crate::aecc_element_edit::replace_elements(rec, &els, length.to_bits())
                .map_err(|e| layout(e.to_string()))?;
        return Ok((
            bytes,
            hb,
            GeometryReport {
                elements: rep.elements,
                length: rep.length,
                graphics_rebuilt: rep.graphics_rebuilt,
            },
        ));
    }
    // Cache: BL 1, BL 0, BL after [, 00], 6 RD.
    let mut cb = end0_bit + 66;
    let (one, nb) = raw::read_bl(data, cb).ok_or_else(|| layout("cache"))?;
    let (zero, nb) = raw::read_bl(data, nb).ok_or_else(|| layout("cache"))?;
    let (_after, nb) = raw::read_bl(data, nb).ok_or_else(|| layout("cache"))?;
    if one != 1 || zero != 0 {
        return Err(layout("cache header not 1, 0"));
    }
    let cache_points = nb;
    let cache_end = cache_points + 6 * 64;
    cb = cache_end;

    // --- Write. ---
    let mut w = Bits::default();
    w.copy(data, 0, eed_bit + 3);
    let (gbits, graphics_rebuilt) = graphics_bits(data, &g, &resolved);
    w.bll((gbits.len() / 8) as u64);
    w.0.extend(gbits.0);
    w.copy(data, g.end_bit(), count_bit);
    // Element list.
    w.bl(els.len() as i32);
    for e in &els {
        match e {
            Element::Tangent(t) => {
                w.bits(PREFIX_TANGENT as u64, 11);
                encode_tangent(&mut w, t.id, t.before, t.after, t.start, t.end);
            }
            Element::Arc(a) => {
                w.bits(PREFIX_ARC as u64, 11);
                let ArcDef::Free { radius, .. } = a.def else {
                    unreachable!()
                };
                encode_free_arc(&mut w, a.flag, &a.params, (a.id, a.before, a.after), radius);
            }
            Element::LineSpiral(_) => unreachable!("not a plain layout"),
        }
    }
    // Gap, end-of-element-0 station, cache.
    w.copy(data, last_end, end0_bit);
    let path_first = resolved
        .iter()
        .find(|e| e.record.before() == 0)
        .ok_or_else(|| BuildError::Geometry("no first element".into()))?;
    let e0_len = path_first.segment.length();
    w.bd(start_station + e0_len);
    w.bl(1);
    w.bl(0);
    w.bl(path_first.record.after());
    let (s0, e0) = (path_first.segment.start(), path_first.segment.end());
    for v in [s0.x, s0.y, e0.x, e0.y, s0.x, s0.y] {
        w.rd(v);
    }
    // Rest of the data stream and the handle stream.
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    w.copy(data, cb, data_end);
    let new_data_end = w.len();
    w.copy(data, data_end, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    // Each free arc reads three (empty) strings from the string stream
    // (S10/S11: 15 strings, S48: 18 for two arcs, tangent-only: 12).
    let arcs = |v: &[Element]| v.iter().filter(|e| matches!(e, Element::Arc(_))).count();
    let old_count = string_stream::parse(data, rec.handle_bits)
        .map_err(|e| layout(e.to_string()))?
        .strings
        .len();
    let want = (old_count + 3 * arcs(&els))
        .checked_sub(
            3 * arcs(
                &decoded
                    .elements
                    .iter()
                    .map(|e| e.record.clone())
                    .collect::<Vec<_>>(),
            ),
        )
        .ok_or_else(|| layout("string count"))?;
    let (bytes, hb) =
        string_stream::set_string_count(&bytes, hb, want).map_err(|e| layout(e.to_string()))?;
    Ok((
        bytes,
        hb,
        GeometryReport {
            elements: els.len(),
            length,
            graphics_rebuilt,
        },
    ))
}

/// Proxy graphics for `resolved` in the donor format of `g` (found in
/// `data`): the donor's leading non-polyline chunks, then per element in
/// path order a 2-point polyline for each tangent and the attribute triple
/// after each arc (S24, S48). Placeholder or polyline-less graphics are
/// copied unchanged (second value `false`). Returns the graphics bytes as
/// bits (size field not included).
pub fn graphics_bits(
    data: &[u8],
    g: &pg::Graphics,
    resolved: &[crate::aecc_elements::ResolvedElement],
) -> (Bits, bool) {
    let poly = g.chunks.iter().find(|c| c.opcode == pg::OP_POLYLINE);
    let mut gbits = Bits::default();
    let graphics_rebuilt = poly.is_some() && !g.is_placeholder();
    if graphics_rebuilt {
        let lead: Vec<&pg::Chunk> = g
            .chunks
            .iter()
            .take_while(|c| c.opcode != pg::OP_POLYLINE)
            .collect();
        // Per element in path order: a polyline chunk for each tangent;
        // after each arc (not drawn by a lines-only style) the attribute
        // triple (layer, marker, colour) again — copies of the leading
        // chunks 4..7, as Civil 3D writes them (S24, S48).
        let triple: Vec<&pg::Chunk> = if lead.len() >= 7 {
            lead[4..7].to_vec()
        } else {
            Vec::new()
        };
        let mut body = Bits::default();
        let mut nchunks = lead.len();
        for e in resolved {
            match e.segment {
                Segment::Line { start: a, end: b } => {
                    body.rl(60);
                    body.rl(pg::OP_POLYLINE);
                    body.rl(2);
                    for v in [a.x, a.y, 0.0, b.x, b.y, 0.0] {
                        body.rd(v);
                    }
                    nchunks += 1;
                }
                _ => {
                    for c in &triple {
                        body.copy(data, c.bit, c.bit + c.size * 8);
                        nchunks += 1;
                    }
                }
            }
        }
        let lead_bytes: usize = lead.iter().map(|c| c.size).sum();
        gbits.rl((8 + lead_bytes + body.len() / 8) as i32);
        gbits.rl(nchunks as i32);
        for c in &lead {
            gbits.copy(data, c.bit, c.bit + c.size * 8);
        }
        gbits.0.extend(body.0);
    } else {
        gbits.copy(data, g.start_bit, g.end_bit());
    }
    (gbits, graphics_rebuilt)
}

/// Rewrite the proxy graphics of `rec` for `resolved` (see
/// [`graphics_bits`]). Records without graphics are returned unchanged.
pub fn replace_graphics(
    rec: &RawRecord,
    resolved: &[crate::aecc_elements::ResolvedElement],
) -> Result<(Vec<u8>, usize, bool), BuildError> {
    let data = &rec.data;
    let eed_bit = eed(data);
    if raw::get_bit(data, eed_bit + 2) == 0 {
        return Ok((data.clone(), rec.handle_bits, false));
    }
    let g = pg::find(data, 4096).ok_or_else(|| layout("no proxy graphics"))?;
    let (gsize, gstart) = read_bll(data, eed_bit + 3);
    if gstart != g.start_bit || gsize as usize != g.size {
        return Err(layout("graphics size field not where expected"));
    }
    let (gbits, rebuilt) = graphics_bits(data, &g, resolved);
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, eed_bit + 3);
    w.bll((gbits.len() / 8) as u64);
    w.0.extend(gbits.0);
    let new_data_end = data_end + w.len() - g.end_bit();
    w.copy(data, g.end_bit(), total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    Ok((bytes, hb, rebuilt))
}

/// Remove the proxy graphics: clear has-graphics (bit 44) and drop the
/// graphics size and stream. Civil 3D regenerates them on open and save
/// (S56 X2 → S57); other programs show nothing until then. Returns the new
/// bytes and handle-stream size.
pub fn strip_graphics(data: &[u8], handle_bits: usize) -> Result<(Vec<u8>, usize), BuildError> {
    let eed_bit = eed(data);
    let g = pg::find(data, 4096).ok_or_else(|| layout("no proxy graphics"))?;
    if raw::get_bit(data, eed_bit) != 1
        || raw::get_bit(data, eed_bit + 1) != 0
        || raw::get_bit(data, eed_bit + 2) != 1
    {
        return Err(layout("graphics flag not where expected"));
    }
    let total = raw::bit_len(data);
    let data_end = total - handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, eed_bit + 2);
    w.bit(0);
    let shift = g.end_bit() - w.len();
    w.copy(data, g.end_bit(), total);
    let new_data_end = data_end - shift;
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    Ok((bytes, hb))
}

/// Offset of the first copy of the alignment's 128-bit identity value
/// after the last element record. It is a random (version 4) GUID in
/// little-endian GUID layout — every value Civil 3D wrote has version
/// nibble 4 in byte 7 and variant bits `10` in byte 8 (S5, S24, S25, S44,
/// S47); stored twice, distinct per alignment, found nowhere else.
const IDENTITY_AFTER_ELEMENTS: usize = 16;

/// A version-4 GUID's 16 bytes in little-endian GUID layout, from `seed`.
pub fn guid_v4(seed: u64) -> [u8; 16] {
    let mut x = seed ^ 0xD1B5_4A32_D192_ED03;
    let mut out = [0u8; 16];
    for half in out.chunks_mut(8) {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        half.copy_from_slice(&z.to_le_bytes());
    }
    out[7] = (out[7] & 0x0F) | 0x40; // version 4 (high nibble of Data3)
    out[8] = (out[8] & 0x3F) | 0x80; // variant 10xx
    out
}

/// Give the alignment record a new identity value: a fresh version-4 GUID
/// from `seed`, written to both copies. Returns the bit offsets of the two
/// copies.
pub fn set_identity(
    rec: &RawRecord,
    data: &mut [u8],
    seed: u64,
) -> Result<(usize, usize), BuildError> {
    let (a, b) = identity_copies(rec, data)?;
    let fresh = guid_v4(seed);
    for pos in [a, b] {
        for (i, v) in fresh.iter().enumerate() {
            raw::set_byte(data, pos + 8 * i, *v);
        }
    }
    Ok((a, b))
}

/// Bit offsets of the two copies of the identity GUID in `data` (laid out
/// like `rec`): copy 1 right after the element records, copy 2 found by
/// value in the post-cache block.
fn identity_copies(rec: &RawRecord, data: &[u8]) -> Result<(usize, usize), BuildError> {
    let decoded = decode_elements(rec).map_err(|e| layout(e.to_string()))?;
    let end = decoded
        .records_end_bit()
        .ok_or_else(|| layout("no elements"))?;
    let a = end + IDENTITY_AFTER_ELEMENTS;
    let old: Vec<u8> = (0..16).map(|i| raw::get_byte(data, a + 8 * i)).collect();
    let n = raw::bit_len(data);
    let b = (a + 128..n.saturating_sub(128))
        .find(|&b| (0..16).all(|i| raw::get_byte(data, b + 8 * i) == old[i]))
        .ok_or_else(|| layout("second copy of the identity value not found"))?;
    Ok((a, b))
}

/// Alignment type values seen in samples (O1, S58/S59). Other values are
/// kept as numbers until a sample names them.
pub const TYPE_CENTERLINE: i32 = 1;
pub const TYPE_MISCELLANEOUS: i32 = 4;

/// Bit offset of the alignment-type BL. After the design-speed table the
/// record has BD 100, BL 0, BL n, n BDs (element IDs 1..n, present after
/// some saves), then the type BL (docs/alignment-object.md, "Alignment type").
fn type_bit(rec: &RawRecord, data: &[u8]) -> Result<usize, BuildError> {
    // After the design-speed table (O2): with one speed it ends with GUID
    // copy 2, with more speeds the identity GUID is in another entry (S79).
    let r = RawRecord {
        data: data.to_vec(),
        ..rec.clone()
    };
    let (_, end, _) = crate::aecc_design_speed::locate(&r).map_err(|e| layout(e.to_string()))?;
    let bad = || layout("alignment-type field not found");
    let (_, q) = raw::read_bd(data, end).ok_or_else(bad)?;
    let (_, q) = raw::read_bl(data, q).ok_or_else(bad)?;
    let (n, mut q) = raw::read_bl(data, q).ok_or_else(bad)?;
    if !(0..=10_000).contains(&n) {
        return Err(bad());
    }
    for _ in 0..n {
        q = raw::read_bd(data, q).ok_or_else(bad)?.1;
    }
    Ok(q)
}

/// Remove the element-ID list Civil 3D adds after geometry edits (BL n +
/// n BDs before the alignment type; S12, S48, S51). It is optional (S50)
/// and would be stale after elements are added or removed. Returns `None`
/// when the record has no list.
pub fn clear_element_id_list(rec: &RawRecord) -> Result<Option<(Vec<u8>, usize)>, BuildError> {
    let data = &rec.data;
    let (_, end, _) = crate::aecc_design_speed::locate(rec).map_err(|e| layout(e.to_string()))?;
    let bad = || layout("element-ID list not found");
    let (_, q) = raw::read_bd(data, end).ok_or_else(bad)?;
    let (_, list) = raw::read_bl(data, q).ok_or_else(bad)?;
    let (n, mut q) = raw::read_bl(data, list).ok_or_else(bad)?;
    if n == 0 {
        return Ok(None);
    }
    if !(0..=10_000).contains(&n) {
        return Err(bad());
    }
    for _ in 0..n {
        q = raw::read_bd(data, q).ok_or_else(bad)?.1;
    }
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, list);
    w.bl(0);
    let new_data_end = data_end - (q - w.len());
    w.copy(data, q, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    Ok(Some((bytes, hb)))
}

/// The alignment type (1 = Centerline, 4 = Miscellaneous; see
/// [`TYPE_CENTERLINE`]).
pub fn alignment_type(rec: &RawRecord) -> Result<i32, BuildError> {
    let q = type_bit(rec, &rec.data)?;
    raw::read_bl(&rec.data, q)
        .map(|x| x.0)
        .ok_or_else(|| layout("alignment-type field not found"))
}

/// Set the alignment type in place. Values 1..=255 share the stored size
/// (`01` + one byte), so the record keeps its length.
pub fn set_alignment_type(rec: &RawRecord, data: &mut [u8], value: i32) -> Result<(), BuildError> {
    if !(1..=255).contains(&value) {
        return Err(layout(format!(
            "alignment type {value} out of range 1..=255"
        )));
    }
    let q = type_bit(rec, data)?;
    if (raw::get_bit(data, q), raw::get_bit(data, q + 1)) != (0, 1) {
        return Err(layout("alignment-type field is not a one-byte BL"));
    }
    raw::set_byte(data, q + 2, value as u8);
    Ok(())
}

/// Elements in path order (following `after` from the one with
/// `before == 0`).
fn path_order(els: &[Element]) -> Vec<Element> {
    let id = |e: &Element| crate::aecc_alignment::Linked::id(e);
    let mut out = Vec::new();
    let mut cur = els.iter().find(|e| e.before() == 0);
    while let Some(e) = cur {
        out.push(e.clone());
        let next = e.after();
        cur = if next == 0 {
            None
        } else {
            els.iter().find(|x| id(x) == next)
        };
        if out.len() > els.len() {
            break;
        }
    }
    out
}

trait Links {
    fn before(&self) -> i32;
    fn after(&self) -> i32;
}

impl Links for Element {
    fn before(&self) -> i32 {
        crate::aecc_alignment::Linked::before(self)
    }
    fn after(&self) -> i32 {
        crate::aecc_alignment::Linked::after(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_ids_like_s11() {
        let pis = [
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(200.0, 50.0),
            Point2::new(300.0, 50.0),
        ];
        let els = plan(&pis, &[Some(50.0), None]).unwrap();
        let ids: Vec<(i32, i32, i32)> = els
            .iter()
            .map(|e| {
                (
                    crate::aecc_alignment::Linked::id(e),
                    crate::aecc_alignment::Linked::before(e),
                    crate::aecc_alignment::Linked::after(e),
                )
            })
            .collect();
        assert_eq!(ids, [(1, 0, 4), (2, 4, 3), (3, 2, 0), (4, 1, 2)]);
    }

    #[test]
    fn bll_round_trip() {
        for v in [0u64, 156, 216, 312, 70000] {
            let mut w = Bits::default();
            w.bll(v);
            let n = w.len();
            let b = w.into_bytes();
            assert_eq!(read_bll(&b, 0), (v, n));
        }
    }

    #[test]
    fn guid_v4_bits() {
        for seed in [0u64, 1, 42, u64::MAX] {
            let g = guid_v4(seed);
            assert_eq!(g[7] >> 4, 4);
            assert_eq!(g[8] >> 6, 0b10);
        }
        assert_ne!(guid_v4(1), guid_v4(2));
    }
}
