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
    if zero != 0 || one != 1 || !(0..=1000).contains(&n) {
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
    let (end, identity) = identity(rec)?;
    let data = &rec.data;
    // No design speeds (S1, S9, S82 FB6/FB9): no intervals (four BL 0)
    // and an empty table right after them.
    if (0..EMPTY_INTERVALS.len()).all(|i| raw::get_bit(data, end + i) == EMPTY_INTERVALS[i]) {
        if let Some((v, q)) = parse_at(data, end + EMPTY_INTERVALS.len()) {
            if v.is_empty() {
                return Ok((end + EMPTY_INTERVALS.len(), q, v));
            }
        }
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

/// The interval block of an alignment without design speeds: BL 0 ×4.
const EMPTY_INTERVALS: [u8; 8] = [1, 0, 1, 0, 1, 0, 1, 0];

/// End of the element records and the identity GUID (copy 1, 16 bits
/// after them).
fn identity(rec: &RawRecord) -> Result<(usize, [u8; 16]), SpeedError> {
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
    let mut id = [0u8; 16];
    for (i, b) in id.iter_mut().enumerate() {
        *b = raw::get_byte(&rec.data, end + 16 + 8 * i);
    }
    Ok((end, id))
}

/// The alignment's design speeds, in stored order.
pub fn design_speeds(rec: &RawRecord) -> Result<Vec<DesignSpeed>, SpeedError> {
    locate(rec).map(|x| x.2)
}

/// Rewrite the design-speed table with `(station, speed)` entries. The
/// first entry gets the identity GUID, the others keep the old entries'
/// GUIDs in order or get random version-4 GUIDs from `seed`. Returns the new
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
    if old.is_empty() {
        return Err(err(
            "the alignment has no design speeds; adding them needs the interval block, \
             which is not written yet",
        ));
    }
    let data = &rec.data;
    // The first entry gets the identity GUID; the others reuse the old
    // entries' GUIDs in order, then fresh ones. (Other copies of entry
    // GUIDs exist outside the table, S74 `mixed speed`, so the identity is
    // taken from copy 1, not searched by value.)
    let (_, keep) = identity(rec)?;
    let mut others = old.iter().map(|e| e.id).filter(|id| *id != keep);
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
            others
                .next()
                .unwrap_or_else(|| guid_v4(seed.wrapping_add(i as u64)))
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

/// Remove all design speeds, as Civil 3D does when the start station
/// changes (S82): the interval block after the elements becomes four BL 0,
/// the table gets `n = 0`, and the three strings per speed (comment and
/// the interval's two) are removed from the end of the string stream.
pub fn clear_design_speeds(rec: &RawRecord) -> Result<(Vec<u8>, usize), SpeedError> {
    let (_, stop, old) = locate(rec)?;
    let (end, _) = identity(rec)?;
    let data = &rec.data;
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, end);
    for b in EMPTY_INTERVALS {
        w.bit(b);
    }
    w.bl(0);
    w.bl(1);
    w.bl(0);
    let new_stop = w.len();
    w.copy(data, stop, total);
    let new_data_end = data_end + new_stop - stop;
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    let ss = string_stream::parse(&bytes, hb).map_err(|e| err(e.to_string()))?;
    let count = ss
        .strings
        .len()
        .checked_sub(3 * old.len())
        .ok_or_else(|| err("string stream too short"))?;
    string_stream::set_string_count(&bytes, hb, count).map_err(|e| err(e.to_string()))
}

/// Bit offset of the first interval's station BD after the element
/// records: BL 0 ×3, BL n (here n ≥ 1, 10 bits), GUID, 13 constant bits.
const FIRST_INTERVAL_STATION: usize = 16 + 128 + 13;

/// Replace bits `[from, to)` of a record with `with`, keeping the handle
/// stream; returns the new bytes and handle-stream size.
fn splice(data: &[u8], handle_bits: usize, from: usize, to: usize, with: Bits) -> (Vec<u8>, usize) {
    let total = raw::bit_len(data);
    let data_end = total - handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, from);
    let inserted = with.len();
    let bytes_with = with.into_bytes();
    w.copy(&bytes_with, 0, inserted);
    w.copy(data, to, total);
    let new_data_end = data_end + from + inserted - to;
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    (bytes, hb)
}

/// Shift every design-speed station by `delta`, keeping order, speeds and
/// GUIDs; the first interval's station after the elements is shifted too
/// (its distance and length stay). This is what civil3drust does when the
/// start station changes (D-031); Civil 3D instead removes the speeds
/// (S82). Civil 3D rebuilds the other intervals on save (S77 → S79).
pub fn shift_design_speeds(rec: &RawRecord, delta: f64) -> Result<(Vec<u8>, usize), SpeedError> {
    let (start, stop, old) = locate(rec)?;
    if old.is_empty() || delta == 0.0 {
        return Ok((rec.data.clone(), rec.handle_bits));
    }
    // Table first (later in the record), then the interval station.
    let mut t = Bits::default();
    t.bl(0);
    t.bl(1);
    t.bl(old.len() as i32);
    for e in &old {
        t.bl(1);
        t.bd(e.station + delta);
        t.bd(e.speed);
        for b in e.id {
            t.byte(b);
        }
    }
    let (data, hb) = splice(&rec.data, rec.handle_bits, start, stop, t);
    let (end, _) = identity(rec)?;
    let at = end + FIRST_INTERVAL_STATION;
    let (station, after) = raw::read_bd(&data, at).ok_or_else(|| err("interval station"))?;
    let mut s = Bits::default();
    s.bd(station + delta);
    Ok(splice(&data, hb, at, after, s))
}

/// The first interval's station (None without design speeds).
pub fn first_interval_station(rec: &RawRecord) -> Option<f64> {
    if locate(rec).ok()?.2.is_empty() {
        return None;
    }
    let (end, _) = identity(rec).ok()?;
    raw::read_bd(&rec.data, end + FIRST_INTERVAL_STATION).map(|x| x.0)
}
