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
use crate::aecc_elements::{decode_elements, Element, Segment};
use crate::alignment::Point2;
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
        // (Also speeds without intervals, e.g. only on curves, S90.)
        if let Some((v, q)) = parse_at(data, end + EMPTY_INTERVALS.len()) {
            return Ok((end + EMPTY_INTERVALS.len(), q, v));
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

/// The design-speed table of an alignment with **no elements** (S113
/// `EMPTY`): found by scanning for the empty interval block (BL 0 ×4)
/// followed by a table. Returns the table's entries and the bit offset
/// after it.
pub fn empty_alignment_table(rec: &RawRecord) -> Option<(Vec<DesignSpeed>, usize)> {
    let data = &rec.data;
    let end = raw::bit_len(data).saturating_sub(rec.handle_bits);
    (0..end.saturating_sub(64)).find_map(|b| {
        if (0..EMPTY_INTERVALS.len()).all(|i| raw::get_bit(data, b + i) == EMPTY_INTERVALS[i]) {
            parse_at(data, b + EMPTY_INTERVALS.len()).filter(|(v, _)| !v.is_empty())
        } else {
            None
        }
    })
}

/// The interval block of an alignment without design speeds: BL 0 ×4.
const EMPTY_INTERVALS: [u8; 8] = [1, 0, 1, 0, 1, 0, 1, 0];

/// End of the element records and the identity GUID (copy 1, 16 bits
/// after them).
fn identity(rec: &RawRecord) -> Result<(usize, [u8; 16]), SpeedError> {
    let d = decode_elements(rec).map_err(|e| err(e.to_string()))?;
    let end = d.records_end_bit().ok_or_else(|| err("no elements"))?;
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

/// Rewrite the design speeds with `(station, speed)` entries: the interval
/// block after the elements (one interval per speed, computed from the
/// geometry) and the table. The entries keep the old GUIDs in order (the
/// first old interval's GUID first) and get random version-4 GUIDs from
/// `seed` after that. Works on alignments without design speeds too.
/// Speeds on curves get an interval too, with the point on the arc (S113). Speeds must lie within the alignment. Returns the new bytes and
/// handle-stream size.
pub fn set_design_speeds(
    rec: &RawRecord,
    speeds: &[(f64, f64)],
    seed: u64,
) -> Result<(Vec<u8>, usize), SpeedError> {
    if speeds.is_empty() {
        return clear_design_speeds(rec);
    }
    if speeds
        .iter()
        .any(|(st, sp)| !st.is_finite() || !(sp.is_finite() && *sp > 0.0))
    {
        return Err(err("stations must be finite and speeds positive"));
    }
    let (_, stop, old) = locate(rec)?;
    let (end, first) = identity(rec)?;
    let data = &rec.data;
    let old_intervals = old_interval_count(data, end)?;
    // GUIDs: old ones first (the first interval's, then the rest in table
    // order), then fresh ones.
    let mut ids: Vec<[u8; 16]> = Vec::new();
    if old_intervals > 0 {
        ids.push(first);
    }
    for e in &old {
        if !ids.contains(&e.id) {
            ids.push(e.id);
        }
    }
    let ids: Vec<[u8; 16]> = (0..speeds.len())
        .map(|i| {
            ids.get(i)
                .copied()
                .unwrap_or_else(|| guid_v4(seed.wrapping_add(i as u64)))
        })
        .collect();
    let mut w = Bits::default();
    let with_interval = write_intervals(rec, &mut w, speeds, &ids)?;
    w.bl(0);
    w.bl(1);
    w.bl(speeds.len() as i32);
    for ((station, speed), id) in speeds.iter().zip(&ids) {
        w.bl(1);
        w.bd(*station);
        w.bd(*speed);
        for b in id {
            w.byte(*b);
        }
    }
    let (bytes, hb) = splice(data, rec.handle_bits, end, stop, w);
    // Strings: three per speed with an interval (the speed's comment and
    // the interval's two), one per speed without (S76, S79, S82).
    let ss = string_stream::parse(&bytes, hb).map_err(|e| err(e.to_string()))?;
    let base = ss
        .strings
        .len()
        .checked_sub(3 * old_intervals + old.len().saturating_sub(old_intervals))
        .ok_or_else(|| err("string stream too short"))?;
    string_stream::set_string_count(&bytes, hb, base + 2 * with_interval + speeds.len())
        .map_err(|e| err(e.to_string()))
}

/// Number of intervals: after the elements come BL 0 ×3 and the count.
fn old_interval_count(data: &[u8], end: usize) -> Result<usize, SpeedError> {
    if (0..6).any(|i| raw::get_bit(data, end + i) != EMPTY_INTERVALS[i]) {
        return Err(err("interval block not where expected"));
    }
    let (n, _) = raw::read_bl(data, end + 6).ok_or_else(|| err("interval count"))?;
    usize::try_from(n).map_err(|_| err("interval count"))
}

/// Write the interval block: BL 0 ×3, BL n, then per speed its GUID, 13
/// constant bits, BD station, BD distance from the element start, BD length to the
/// end of the element, BL element ID, BL its before-ID, BL its after-ID (S115), and RD
/// element start x, y, element end x, y, point x, y (S24, S74, S79, S84);
/// the distance is measured from the element's start (S89). Returns the
/// number of intervals written (speeds on tangents).
fn write_intervals(
    rec: &RawRecord,
    w: &mut Bits,
    speeds: &[(f64, f64)],
    ids: &[[u8; 16]],
) -> Result<usize, SpeedError> {
    let d = decode_elements(rec).map_err(|e| err(e.to_string()))?;
    let start = crate::aecc_station::start_station(rec).unwrap_or(0.0);
    let total: f64 = d.elements.iter().map(|e| e.segment.length()).sum();
    let mut intervals = Bits::default();
    let mut count = 0;
    for ((station, _), id) in speeds.iter().zip(ids) {
        let dist = station - start;
        if dist < -1e-9 || dist > total + 1e-9 {
            return Err(err(format!(
                "speed station {station} is outside the alignment ({start} to {})",
                start + total
            )));
        }
        // The element containing the point (at an element boundary, the
        // next one, except at the very end).
        let mut acc = 0.0;
        let mut found = None;
        for (i, e) in d.elements.iter().enumerate() {
            let len = e.segment.length();
            let last = i + 1 == d.elements.len();
            if dist < acc + len - 1e-9 || last {
                found = Some((e, (dist - acc).clamp(0.0, len), len));
                break;
            }
            acc += len;
        }
        let (e, t, len) = found.ok_or_else(|| err("no elements"))?;
        // The interval stores the element's start and end and the point at
        // the station; on a curve the point lies on the arc. The third distance is the length to the element's end, as
        // on tangents (S113: 15.548 = 29.650 − 14.102); S89's curve
        // interval stores 84.63 there instead of 35.72 (not explained).
        let (a, b, p) = match e.segment {
            Segment::Line { start: a, end: b } => {
                let f = if len > 0.0 { t / len } else { 0.0 };
                (
                    a,
                    b,
                    Point2::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f),
                )
            }
            Segment::Arc {
                start: a,
                end: b,
                geometry: g,
            } => {
                let a0 = (a.y - g.center.y).atan2(a.x - g.center.x);
                let th = if g.counter_clockwise {
                    a0 + t / g.radius
                } else {
                    a0 - t / g.radius
                };
                let p = Point2::new(
                    g.center.x + g.radius * th.cos(),
                    g.center.y + g.radius * th.sin(),
                );
                (a, b, p)
            }
        };
        // The element's own ID and links (S115: Civil 3D rewrote our
        // (1, 0, next) to (4, 1, 3) for a speed on element 4; S113's
        // "2, 1, 3" on a curve is element 2, before 1, after 3).
        let (eid, before, after) = match &e.record {
            Element::Tangent(t) => (t.id, t.before, t.after),
            Element::Arc(x) => (x.id, x.before, x.after),
        };
        for byte in id {
            intervals.byte(*byte);
        }
        intervals.bits(INTERVAL_CONSTANT, 13);
        intervals.bd(*station);
        // Distance from the element's start (S89: 84.64 on its second
        // element), then the length to the element's end.
        intervals.bd(t);
        intervals.bd(len - t);
        intervals.bl(eid);
        intervals.bl(before);
        intervals.bl(after);
        for v in [a.x, a.y, b.x, b.y, p.x, p.y] {
            intervals.rd(v);
        }
        count += 1;
    }
    for _ in 0..3 {
        w.bl(0);
    }
    w.bl(count as i32);
    let n = intervals.len();
    let bytes = intervals.into_bytes();
    w.copy(&bytes, 0, n);
    Ok(count)
}

/// The 13 bits after each interval's GUID, the same in every sample.
const INTERVAL_CONSTANT: u64 = 0b1010000001010;

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
    if old_interval_count(&data, end)? == 0 {
        return Ok((data, hb));
    }
    let at = end + FIRST_INTERVAL_STATION;
    let (station, after) = raw::read_bd(&data, at).ok_or_else(|| err("interval station"))?;
    let mut s = Bits::default();
    s.bd(station + delta);
    Ok(splice(&data, hb, at, after, s))
}

/// The first interval's station (None without design speeds).
pub fn first_interval_station(rec: &RawRecord) -> Option<f64> {
    let (end, _) = identity(rec).ok()?;
    if old_interval_count(&rec.data, end).ok()? == 0 {
        return None;
    }
    raw::read_bd(&rec.data, end + FIRST_INTERVAL_STATION).map(|x| x.0)
}
