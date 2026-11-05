//! Station-anchored data of an alignment (S243): the interval block after
//! the element records, station equations and masking regions.
//!
//! The interval block (`docs/alignment-object.md`, "The interval block")
//! is not specific to design speeds: every object that sits at a station
//! has one interval there, found by its GUID — a design speed (its table
//! entry carries the GUID), a station equation (one interval) and a masking
//! region (one at its start, one at its end). After the intervals:
//!
//! ```text
//! BL n_eq, n_eq × { 18 bits (BS 1214 in S243, unknown), BD raw station,
//!                   GUID, BD station back, BD station ahead, B increasing },
//! BL 1, BL n, n × { BL 1, BD station, BD speed, GUID }     (design speeds)
//! BD 100, BL 0, element-ID list, BL type, BL 2, 2, 2,
//! BL n_mask, n_mask × { BD start station, BD end station, B not masked,
//!                       GUID start, GUID end },
//! BL implied-PI display, …
//! ```
//!
//! The `BL 0` that every earlier sample has before `BL 1, BL n` is the
//! empty equation list, and the `0` of `BL 2, 2, 2, 0` the empty masking
//! list. Each interval has two strings in the string stream, each design
//! speed one (its comment), each masking region one and each station
//! equation two (S243: `9 + 2 per interval + 1 per speed + 1 per region +
//! 2 per equation + 3 per non-line element`, + 1 user string in the
//! Properties form). A region's one string is its comment (S246).
//!
//! The Masking tab's "Lock To Start" / "Lock To End" (only on the first
//! and last region, owner) are not stored as flags: a locked region keeps
//! its stations (0 and the alignment's end) and intervals like any other
//! (S246), and Civil 3D shows the lock again whenever the first region
//! starts at the alignment's start or the last one ends at its end
//! (owner). civil3drust derives the locks the same way when it reads a
//! drawing; after that a lock is model data (D-068): a locked region uses
//! the alignment's start or end, whatever station is recorded, and an
//! unlocked one keeps its recorded station without gaining a lock.

use crate::aecc_elements::decode_elements;
use crate::raw::{self, RawRecord};

/// One interval: a station anchor (GUID, kind, station) and its bit range.
#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub guid: [u8; 16],
    /// What the interval belongs to: [`KIND_SPEED`], [`KIND_EQUATION`] or
    /// [`KIND_MASK`] (S243).
    pub kind: i32,
    pub station: f64,
    pub start_bit: usize,
    pub end_bit: usize,
}

/// A station equation (S243 FE5: back 190, ahead 310 at raw station 190).
#[derive(Debug, Clone, PartialEq)]
pub struct StationEquation {
    /// Raw Station Back: the station without any equation.
    pub raw_station: f64,
    /// Station Back: for every equation but the first, relative to the
    /// one before (S247: 190 − 110 + 109 = 189).
    pub back: f64,
    /// Station Ahead.
    pub ahead: f64,
    /// Increase/Decrease: the stations after the equation run up (bit 1)
    /// or down (bit 0; S248).
    pub increasing: bool,
    /// The 18 bits before the raw station, read as a BS: 1214 in every
    /// equation sampled (S243, S247, S248). A fixed marker (owner; the
    /// same bits open most Civil 3D objects, MIST18).
    pub unknown: u16,
    pub guid: [u8; 16],
}

/// A masking region (S243 FDF: 2+90.30 to 3+34.11; S246: four regions).
#[derive(Debug, Clone, PartialEq)]
pub struct MaskRegion {
    pub start: f64,
    pub end: f64,
    /// The Masking tab's "Mask" check box: the stored bit is 0 when it is
    /// checked, 1 when not (S246 region 2).
    pub masked: bool,
    /// "Lock To Start" (first region only): derived on reading, see
    /// [`MaskRegion::range`].
    pub lock_to_start: bool,
    /// "Lock To End" (last region only).
    pub lock_to_end: bool,
    pub start_guid: [u8; 16],
    pub end_guid: [u8; 16],
}

impl MaskRegion {
    /// The region's stations on an alignment running from `start` to
    /// `end`: a locked side takes the alignment's own station, an unlocked
    /// side the recorded one (D-068).
    pub fn range(&self, start: f64, end: f64) -> (f64, f64) {
        (
            if self.lock_to_start {
                start
            } else {
                self.start
            },
            if self.lock_to_end { end } else { self.end },
        )
    }
}

/// How close a recorded station must be to the alignment's start or end
/// to read as locked (S246: the locked end is 2.7e-10 past the decoded
/// length).
pub const LOCK_TOLERANCE: f64 = 1e-6;

/// Set the locks as Civil 3D shows them: the first region locked to the
/// start when it starts at `start`, the last locked to the end when it
/// ends at `end`.
pub fn derive_locks(regions: &mut [MaskRegion], start: f64, end: f64) {
    let n = regions.len();
    for (i, r) in regions.iter_mut().enumerate() {
        r.lock_to_start = i == 0 && (r.start - start).abs() <= LOCK_TOLERANCE;
        r.lock_to_end = i + 1 == n && (r.end - end).abs() <= LOCK_TOLERANCE;
    }
}

fn guid(data: &[u8], bit: usize) -> Option<[u8; 16]> {
    if bit + 128 > raw::bit_len(data) {
        return None;
    }
    let mut g = [0u8; 16];
    for (i, b) in g.iter_mut().enumerate() {
        *b = raw::get_byte(data, bit + 8 * i);
    }
    Some(g)
}

/// Interval kinds: the BL after the GUID (`B 1, BL kind, BD 0`; S243).
pub const KIND_SPEED: i32 = 2;
pub const KIND_EQUATION: i32 = 3;
pub const KIND_MASK: i32 = 4;

/// The interval block after the element records: its start (the `BL 0`
/// ×3), the intervals, and the bit after the block.
pub fn intervals(rec: &RawRecord) -> Option<(usize, Vec<Interval>, usize)> {
    let d = decode_elements(rec).ok()?;
    let start = d.records_end_bit()?;
    intervals_at(&rec.data, start).map(|(v, end)| (start, v, end))
}

/// The interval block starting at `start` (see [`intervals`]).
pub fn intervals_at(data: &[u8], start: usize) -> Option<(Vec<Interval>, usize)> {
    let mut q = start;
    for _ in 0..3 {
        let (v, nq) = raw::read_bl(data, q)?;
        if v != 0 {
            return None;
        }
        q = nq;
    }
    let (n, mut q) = raw::read_bl(data, q)?;
    if !(0..=10_000).contains(&n) {
        return None;
    }
    let mut out = Vec::new();
    for _ in 0..n {
        let s = q;
        let g = guid(data, q)?;
        q += 128;
        // `B 1, BL kind, BD 0` (13 bits; the "constant" of earlier
        // samples, which only had speeds).
        if raw::get_bit(data, q) != 1 {
            return None;
        }
        let (kind, p) = raw::read_bl(data, q + 1)?;
        let (zero, p) = raw::read_bd(data, p)?;
        if !(KIND_SPEED..=KIND_MASK).contains(&kind) || zero != 0.0 {
            return None;
        }
        let (station, p) = raw::read_bd(data, p)?;
        let (_, p) = raw::read_bd(data, p)?;
        let (_, mut p) = raw::read_bd(data, p)?;
        for _ in 0..3 {
            p = raw::read_bl(data, p)?.1;
        }
        q = p + 6 * 64;
        if q > raw::bit_len(data) || !station.is_finite() {
            return None;
        }
        out.push(Interval {
            guid: g,
            kind,
            station,
            start_bit: s,
            end_bit: q,
        });
    }
    Some((out, q))
}

/// The station-equation list right after the interval block: the
/// equations and the bit after the list (where the design-speed table's
/// `BL 1, BL n` starts).
pub fn station_equations_at(data: &[u8], at: usize) -> Option<(Vec<StationEquation>, usize)> {
    let (n, mut q) = raw::read_bl(data, at)?;
    if !(0..=1000).contains(&n) {
        return None;
    }
    let mut out = Vec::new();
    for _ in 0..n {
        // 18 bits: `00` + 16 bits in S243 (a BS); not decoded.
        if raw::get_bit(data, q) != 0 || raw::get_bit(data, q + 1) != 0 {
            return None;
        }
        let unknown =
            u16::from(raw::get_byte(data, q + 2)) | u16::from(raw::get_byte(data, q + 10)) << 8;
        q += 18;
        let (raw_station, p) = raw::read_bd(data, q)?;
        let g = guid(data, p)?;
        let (back, p) = raw::read_bd(data, p + 128)?;
        let (ahead, p) = raw::read_bd(data, p)?;
        let increasing = raw::get_bit(data, p) == 1;
        q = p + 1;
        if ![raw_station, back, ahead].iter().all(|v| v.is_finite()) {
            return None;
        }
        out.push(StationEquation {
            raw_station,
            back,
            ahead,
            increasing,
            unknown,
            guid: g,
        });
    }
    Some((out, q))
}

/// The alignment's station equations (empty when it has none).
pub fn station_equations(rec: &RawRecord) -> Option<Vec<StationEquation>> {
    let (_, _, end) = intervals(rec)?;
    station_equations_at(&rec.data, end).map(|x| x.0)
}

/// The masking list at `at` (the BL after `BL 2, 2, 2`): the regions and
/// the bit after the list (the implied-PI display BL).
pub fn masking_at(data: &[u8], at: usize) -> Option<(Vec<MaskRegion>, usize)> {
    let (n, mut q) = raw::read_bl(data, at)?;
    if !(0..=1000).contains(&n) {
        return None;
    }
    let mut out = Vec::new();
    for _ in 0..n {
        let (start, p) = raw::read_bd(data, q)?;
        let (end, p) = raw::read_bd(data, p)?;
        let masked = raw::get_bit(data, p) == 0;
        let start_guid = guid(data, p + 1)?;
        let end_guid = guid(data, p + 129)?;
        q = p + 257;
        if !(start.is_finite() && end.is_finite()) {
            return None;
        }
        out.push(MaskRegion {
            start,
            end,
            masked,
            lock_to_start: false,
            lock_to_end: false,
            start_guid,
            end_guid,
        });
    }
    Some((out, q))
}

/// The station Civil 3D shows at `raw` (start station + distance along
/// the alignment) with the station equations applied: past an equation's
/// Raw Station Back the stations continue from its Station Ahead, up or
/// down by its Increase/Decrease (S247, S248). Station Back is not used:
/// it is derived from the equations before it, and when the start station
/// moves Civil 3D shifts the raw station but keeps the stored back (S249:
/// raw 115, back 110). Exactly at an equation the
/// station ahead is returned.
pub fn display_station(raw: f64, equations: &[StationEquation]) -> f64 {
    equations
        .iter()
        .filter(|e| e.raw_station <= raw)
        .max_by(|a, b| a.raw_station.total_cmp(&b.raw_station))
        .map_or(raw, |e| {
            let d = raw - e.raw_station;
            if e.increasing {
                e.ahead + d
            } else {
                e.ahead - d
            }
        })
}

/// Station Back as Civil 3D shows it: the station reached just before the
/// equation through the ones before it (for the first, its raw station).
pub fn station_back(index: usize, equations: &[StationEquation]) -> Option<f64> {
    let e = equations.get(index)?;
    let before: Vec<StationEquation> = equations
        .iter()
        .filter(|x| x.raw_station < e.raw_station)
        .cloned()
        .collect();
    Some(display_station(e.raw_station, &before))
}
