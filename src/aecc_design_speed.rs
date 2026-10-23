//! Design speeds of an alignment (O2, S74).
//!
//! The "post-cache block" of an `AECC_ALIGNMENT` record starts with the
//! design-speed table:
//!
//! ```text
//! BL 0, BL 1, BL n,
//! n × { BL 1, BD start station, BD speed, 16-byte ID (GUID) },
//! BD 100, …                      (rest of the block, alignment type, …)
//! ```
//!
//! Speeds are stored in the drawing's speed unit (mph in the samples). Every
//! alignment has at least one entry: Civil 3D's default is 60 at the start
//! station, which is why "BD 60" looked like a constant (S24–S73). Each entry
//! has a comment string in the record's string stream (S76); one entry
//! carries the alignment's identity GUID ("GUID copy 2"); the others have
//! their own random GUIDs (S74 `mixed speed`). See
//! `docs/alignment-object.md`, "Design speeds".

use crate::aecc_build::{guid_v4, Bits};
use crate::aecc_elements::{decode_elements, Element};
use crate::raw::{self, RawRecord};
use crate::string_stream;
use std::fmt;

/// One design-speed entry.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignSpeed {
    pub station: f64,
    pub speed: f64,
    pub id: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedError(pub String);

impl fmt::Display for SpeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "design speeds: {}", self.0)
    }
}

impl std::error::Error for SpeedError {}

fn err(m: impl Into<String>) -> SpeedError {
    SpeedError(m.into())
}

/// The table at `start`, if it parses there and is followed by BD 100.
/// Returns the entries and the bit offset after the last entry.
fn parse_at(data: &[u8], start: usize) -> Option<(Vec<DesignSpeed>, usize)> {
    let (zero, q) = raw::read_bl(data, start)?;
    let (one, q) = raw::read_bl(data, q)?;
    let (n, mut q) = raw::read_bl(data, q)?;
    if zero != 0 || one != 1 || !(1..=1000).contains(&n) {
        return None;
    }
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let (flag, p) = raw::read_bl(data, q)?;
        let (station, p) = raw::read_bd(data, p)?;
        let (speed, p) = raw::read_bd(data, p)?;
        if flag != 1 || !station.is_finite() || !speed.is_finite() || speed <= 0.0 {
            return None;
        }
        if p + 128 > raw::bit_len(data) {
            return None;
        }
        let mut id = [0u8; 16];
        for (i, b) in id.iter_mut().enumerate() {
            *b = raw::get_byte(data, p + 8 * i);
        }
        out.push(DesignSpeed { station, speed, id });
        q = p + 128;
    }
    match raw::read_bd(data, q) {
        Some((100.0, _)) => Some((out, q)),
        _ => None,
    }
}

/// Bit range of the table and its entries. The table is searched after the
/// element records; it must hold the identity GUID (copy 1, 16 bits after
/// the last element record) in one entry.
pub fn locate(rec: &RawRecord) -> Result<(usize, usize, Vec<DesignSpeed>), SpeedError> {
    let d = decode_elements(rec).map_err(|e| err(e.to_string()))?;
    let end = d
        .elements
        .iter()
        .map(|e| match &e.record {
            Element::Tangent(t) => t.coords_bit + 256,
            Element::Arc(a) => a.end_bit,
        })
        .max()
        .ok_or_else(|| err("no elements"))?;
    let data = &rec.data;
    let mut identity = [0u8; 16];
    for (i, b) in identity.iter_mut().enumerate() {
        *b = raw::get_byte(data, end + 16 + 8 * i);
    }
    let data_end = raw::bit_len(data) - rec.handle_bits;
    (end + 144..data_end.saturating_sub(200))
        .find_map(|s| {
            parse_at(data, s)
                .filter(|(v, _)| v.iter().any(|e| e.id == identity))
                .map(|(v, q)| (s, q, v))
        })
        .ok_or_else(|| err("design-speed table not found"))
}

/// The alignment's design speeds, in stored order.
pub fn design_speeds(rec: &RawRecord) -> Result<Vec<DesignSpeed>, SpeedError> {
    locate(rec).map(|x| x.2)
}

/// Rewrite the design-speed table with `(station, speed)` entries. The
/// entry holding the identity GUID keeps it and becomes the first entry;
/// the others get random version-4 GUIDs from `seed`. Returns the new
/// bytes and handle-stream size (the record grows or shrinks).
pub fn set_design_speeds(
    rec: &RawRecord,
    speeds: &[(f64, f64)],
    seed: u64,
) -> Result<(Vec<u8>, usize), SpeedError> {
    if speeds.is_empty() {
        return Err(err("at least one design speed is required"));
    }
    if speeds
        .iter()
        .any(|(st, sp)| !st.is_finite() || !(sp.is_finite() && *sp > 0.0))
    {
        return Err(err("stations must be finite and speeds positive"));
    }
    let (start, stop, old) = locate(rec)?;
    let data = &rec.data;
    // The identity GUID is the entry whose ID Civil 3D also stores after
    // the element records; locate() guarantees one exists.
    let keep = old
        .iter()
        .map(|e| e.id)
        .find(|id| {
            // copy 1 is found by value anywhere before the table
            (0..start.saturating_sub(128))
                .any(|b| (0..16).all(|i| raw::get_byte(data, b + 8 * i) == id[i]))
        })
        .unwrap_or(old[0].id);
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, start);
    w.bl(0);
    w.bl(1);
    w.bl(speeds.len() as i32);
    for (i, (station, speed)) in speeds.iter().enumerate() {
        w.bl(1);
        w.bd(*station);
        w.bd(*speed);
        let id = if i == 0 {
            keep
        } else {
            guid_v4(seed.wrapping_add(i as u64))
        };
        for b in id {
            w.byte(b);
        }
    }
    let new_stop = w.len();
    w.copy(data, stop, total);
    let new_data_end = data_end + new_stop - stop;
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    // Each entry has a comment string (empty here) in the string stream:
    // with too few strings Civil 3D reads the comment from the bits before
    // the stream (S75 -> S76). The other strings around them are empty too,
    // so adding or removing empty strings at the end is enough.
    let ss = string_stream::parse(&bytes, hb).map_err(|e| err(e.to_string()))?;
    let count = (ss.strings.len() + speeds.len())
        .checked_sub(old.len())
        .ok_or_else(|| err("string stream too short"))?;
    string_stream::set_string_count(&bytes, hb, count).map_err(|e| err(e.to_string()))
}
