//! Station-anchored data of an alignment (S243): the interval block after
//! the element records, station equations and masking regions.
//!
//! The interval block (`docs/alignment-object.md`, "The interval block")
//! is not specific to design speeds: every object that sits at a station
//! has one interval there, found by its GUID — a design speed (its table
//! entry carries the GUID), a station equation (one interval) and a masking
//! region (one at its start, one at its end). In a production drawing
//! (checked locally) alignments carry hundreds of intervals of other kinds
//! (labels and other objects anchored to stations; kinds up to ~700 and
//! one near 0x7FFF per alignment), and the three BLs that open the block
//! are small counters (1–3, 0–1, 0) instead of 0. After the intervals:
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
        if !(0..=1_000_000).contains(&v) {
            return None;
        }
        q = nq;
    }
    let (n, mut q) = raw::read_bl(data, q)?;
    if !(0..=100_000).contains(&n) {
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
        if !(0..=0xFFFF).contains(&kind) || zero != 0.0 {
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

/// Why a station-reference edit was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StationRefError(pub String);

impl std::fmt::Display for StationRefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "station references: {}", self.0)
    }
}

impl std::error::Error for StationRefError {}

fn fail(m: impl Into<String>) -> StationRefError {
    StationRefError(m.into())
}

/// A station equation to write. Station Back is derived from the
/// equations before it, as Civil 3D does (S247).
#[derive(Debug, Clone, PartialEq)]
pub struct EquationSpec {
    /// Raw Station Back: start station + distance along the alignment.
    pub raw_station: f64,
    pub ahead: f64,
    pub increasing: bool,
    pub comment: String,
}

/// A masking region to write. A locked side takes the alignment's start or
/// end (D-068); only the first region may be locked to the start and only
/// the last to the end.
#[derive(Debug, Clone, PartialEq)]
pub struct MaskSpec {
    pub start: f64,
    pub end: f64,
    pub masked: bool,
    pub lock_to_start: bool,
    pub lock_to_end: bool,
    pub comment: String,
}

/// The equation marker (`00` + `BE 04`; MIST18, a fixed marker).
const EQUATION_MARKER: [u8; 2] = [0xBE, 0x04];

/// Where the station-anchored parts of a record are.
struct Layout {
    d: crate::aecc_elements::DecodedElements,
    start: f64,
    end: f64,
    /// Interval block: start bit, intervals, end bit (= equation list).
    block: usize,
    intervals: Vec<Interval>,
    eq_start: usize,
    equations: Vec<StationEquation>,
    eq_end: usize,
    mask_start: usize,
    regions: Vec<MaskRegion>,
    mask_end: usize,
    speeds: usize,
    strings: usize,
}

impl Layout {
    fn read(rec: &RawRecord) -> Result<Self, StationRefError> {
        let data = &rec.data;
        let d = decode_elements(rec).map_err(|e| fail(e.to_string()))?;
        if !d.detached.is_empty() || d.layout_groups.iter().any(|g| !g.solved) {
            return Err(fail("detached pieces or an unsolved JSON group"));
        }
        let start = crate::aecc_station::start_station(rec).ok_or_else(|| fail("start station"))?;
        let end = start + d.length();
        let block = d.records_end_bit().ok_or_else(|| fail("no elements"))?;
        let (intervals, eq_start) =
            intervals_at(data, block).ok_or_else(|| fail("interval block not readable"))?;
        let (equations, eq_end) = station_equations_at(data, eq_start)
            .ok_or_else(|| fail("station-equation list not readable"))?;
        let mask_start =
            crate::aecc_build::masking_bit(rec, data).map_err(|e| fail(e.to_string()))?;
        let (mut regions, mask_end) =
            masking_at(data, mask_start).ok_or_else(|| fail("masking list not readable"))?;
        derive_locks(&mut regions, start, end);
        let speeds = crate::aecc_design_speed::design_speeds(rec)
            .map_err(|e| fail(e.to_string()))?
            .len();
        let total = crate::string_stream::parse(data, rec.handle_bits)
            .map_err(|e| fail(e.to_string()))?
            .strings
            .len();
        // The tail is counted from the end of the strings the rule
        // requires, not from the last string: Civil 3D can leave surplus
        // empty strings after them (S253: 15 after an edit).
        let required = crate::aecc_element_edit::required_string_count(rec)
            .map_err(|e| fail(e.to_string()))?;
        let user = usize::from(raw::section2_form(data) == Some(raw::Section2Form::Properties));
        let strings = (required + user).min(total);
        Ok(Layout {
            d,
            start,
            end,
            block,
            intervals,
            eq_start,
            equations,
            eq_end,
            mask_start,
            regions,
            mask_end,
            speeds,
            strings,
        })
    }

    /// The interval block with the intervals of `kind` replaced by `new`.
    fn interval_block(
        &self,
        data: &[u8],
        kind: i32,
        new: &[(f64, [u8; 16])],
    ) -> Result<crate::aecc_build::Bits, StationRefError> {
        let mut body = crate::aecc_build::Bits::default();
        let mut n = 0;
        for i in self.intervals.iter().filter(|i| i.kind != kind) {
            body.copy(data, i.start_bit, i.end_bit);
            n += 1;
        }
        for (station, id) in new {
            crate::aecc_design_speed::write_interval(
                &self.d, self.start, *station, kind, id, &mut body,
            )
            .map_err(|e| fail(e.to_string()))?;
            n += 1;
        }
        let mut w = crate::aecc_build::Bits::default();
        // The opening BLs as stored (0 in samples, counters in production).
        let mut q = self.block;
        for _ in 0..3 {
            let (v, nq) = raw::read_bl(data, q).ok_or_else(|| fail("interval head"))?;
            w.bl(v);
            q = nq;
        }
        w.bl(n);
        let len = body.len();
        let bytes = body.into_bytes();
        w.copy(&bytes, 0, len);
        Ok(w)
    }
}

/// A random v4 GUID from `seed` that is not in `used` (and add it): two
/// anchors with one GUID are merged by Civil 3D (S250 → S251: a masking
/// region given an equation's GUIDs was moved onto the equation's
/// stations).
pub(crate) fn fresh_guid(seed: &mut u64, used: &mut Vec<[u8; 16]>) -> [u8; 16] {
    loop {
        let g = crate::aecc_build::guid_v4(*seed);
        *seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        if !used.contains(&g) {
            used.push(g);
            return g;
        }
    }
}

/// Every GUID the record's anchors use (intervals, equations, regions);
/// empty when they can't be read.
pub(crate) fn record_guids(rec: &RawRecord) -> Vec<[u8; 16]> {
    Layout::read(rec).map(|l| l.guids()).unwrap_or_else(|_| {
        intervals(rec)
            .map(|(_, v, _)| v.iter().map(|i| i.guid).collect())
            .unwrap_or_default()
    })
}

impl Layout {
    /// Every GUID the record's anchors use: intervals, equations, regions.
    fn guids(&self) -> Vec<[u8; 16]> {
        self.intervals
            .iter()
            .map(|i| i.guid)
            .chain(self.equations.iter().map(|e| e.guid))
            .chain(self.regions.iter().flat_map(|r| [r.start_guid, r.end_guid]))
            .collect()
    }
}

/// Copy `data` with the bit ranges in `edits` (sorted, not overlapping)
/// replaced; returns bytes and handle-stream size.
fn splice_all(
    data: &[u8],
    handle_bits: usize,
    edits: Vec<(usize, usize, crate::aecc_build::Bits)>,
) -> (Vec<u8>, usize) {
    let total = raw::bit_len(data);
    let data_end = total - handle_bits;
    let mut w = crate::aecc_build::Bits::default();
    let mut at = 0;
    let mut delta: isize = 0;
    for (from, to, with) in edits {
        w.copy(data, at, from);
        let n = with.len();
        let bytes = with.into_bytes();
        w.copy(&bytes, 0, n);
        delta += n as isize - (to - from) as isize;
        at = to;
    }
    w.copy(data, at, total);
    let new_data_end = (data_end as isize + delta) as usize;
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    (bytes, hb)
}

/// Replace `remove` strings at `index` by `insert` (empty strings are
/// inserted for `""`).
fn edit_strings(
    data: Vec<u8>,
    hb: usize,
    index: usize,
    remove: usize,
    insert: &[String],
) -> Result<(Vec<u8>, usize), StationRefError> {
    use crate::string_stream as ss;
    let e = |x: ss::StringStreamError| fail(x.to_string());
    let (mut data, mut hb) = (data, hb);
    for _ in 0..remove {
        (data, hb) = ss::remove(&data, hb, index).map_err(e)?;
    }
    for (k, text) in insert.iter().enumerate() {
        (data, hb) = ss::insert_empty(&data, hb, index + k).map_err(e)?;
        if !text.is_empty() {
            (data, hb) = ss::replace(&data, hb, index + k, text).map_err(e)?;
        }
    }
    Ok((data, hb))
}

/// Rewrite the alignment's station equations (S243, S247, S248): their
/// intervals (kind 3), the equation list with the fixed marker, Raw
/// Station Back, Station Back (derived), Station Ahead and
/// Increase/Decrease, and their strings (comment, number; before the
/// speeds' comments). Old equations' GUIDs are reused in order; new ones
/// come from `seed`. Everything else is kept.
pub fn set_station_equations(
    rec: &RawRecord,
    equations: &[EquationSpec],
    seed: u64,
) -> Result<(Vec<u8>, usize), StationRefError> {
    let l = Layout::read(rec)?;
    let data = &rec.data;
    let mut sorted = equations.to_vec();
    sorted.sort_by(|a, b| a.raw_station.total_cmp(&b.raw_station));
    for (i, e) in sorted.iter().enumerate() {
        if !(e.raw_station > l.start && e.raw_station < l.end) || !e.ahead.is_finite() {
            return Err(fail(format!(
                "equation at {} must lie inside the alignment ({} to {})",
                e.raw_station, l.start, l.end
            )));
        }
        if i > 0 && (e.raw_station - sorted[i - 1].raw_station).abs() < 1e-9 {
            return Err(fail("two equations at one station"));
        }
    }
    let mut used = l.guids();
    let mut seed = seed;
    let ids: Vec<[u8; 16]> = (0..sorted.len())
        .map(|i| match l.equations.get(i) {
            Some(e) => e.guid,
            None => fresh_guid(&mut seed, &mut used),
        })
        .collect();
    let mut eqs: Vec<StationEquation> = sorted
        .iter()
        .zip(&ids)
        .map(|(e, g)| StationEquation {
            raw_station: e.raw_station,
            back: 0.0,
            ahead: e.ahead,
            increasing: e.increasing,
            unknown: u16::from_le_bytes(EQUATION_MARKER),
            guid: *g,
        })
        .collect();
    for i in 0..eqs.len() {
        eqs[i].back = station_back(i, &eqs).unwrap_or(eqs[i].raw_station);
    }
    let anchors: Vec<(f64, [u8; 16])> = eqs.iter().map(|e| (e.raw_station, e.guid)).collect();
    let block = l.interval_block(data, KIND_EQUATION, &anchors)?;
    let mut list = crate::aecc_build::Bits::default();
    list.bl(eqs.len() as i32);
    for e in &eqs {
        list.bits(0, 2);
        list.byte(EQUATION_MARKER[0]);
        list.byte(EQUATION_MARKER[1]);
        list.bd(e.raw_station);
        for b in e.guid {
            list.byte(b);
        }
        list.bd(e.back);
        list.bd(e.ahead);
        list.bit(u8::from(e.increasing));
    }
    let (bytes, hb) = splice_all(
        data,
        rec.handle_bits,
        vec![(l.block, l.eq_start, block), (l.eq_start, l.eq_end, list)],
    );
    // Strings, counted from the end: the masking regions' comments, the
    // speeds' comments, then the equations' (comment, number) pairs (S246,
    // S248). Two empty strings per interval go with them.
    let old = l.equations.len();
    let at = l
        .strings
        .checked_sub(l.regions.len() + l.speeds + 2 * old)
        .ok_or_else(|| fail("string stream too short"))?;
    let mut pairs = Vec::new();
    for (i, e) in sorted.iter().enumerate() {
        pairs.push(e.comment.clone());
        pairs.push((i + 1).to_string());
    }
    let (bytes, hb) = edit_strings(bytes, hb, at, 2 * old, &pairs)?;
    let (bytes, hb) = adjust_empties(bytes, hb, at, 2 * old, 2 * eqs.len())?;
    Ok(crate::handle_refs::trim_padding(&bytes, hb, rec.handle))
}

/// Remove `remove` empty strings just before string `before` and insert
/// `add` empty strings there: the interval strings, whose place among the
/// empty strings is not known (they are always empty).
fn adjust_empties(
    data: Vec<u8>,
    hb: usize,
    before: usize,
    remove: usize,
    add: usize,
) -> Result<(Vec<u8>, usize), StationRefError> {
    let ss = crate::string_stream::parse(&data, hb).map_err(|e| fail(e.to_string()))?;
    let from = before
        .checked_sub(remove)
        .ok_or_else(|| fail("string stream too short"))?;
    if ss.strings[from..before].iter().any(|x| !x.text.is_empty()) {
        return Err(fail("the interval strings are not where expected"));
    }
    edit_strings(data, hb, from, remove, &vec![String::new(); add])
}

/// Rewrite the alignment's masking regions (S243, S246): their intervals
/// (kind 4, one at each end), the masking list (stations, the inverted
/// Mask bit, the two GUIDs) and their comments (the last strings). A
/// locked side is written as the alignment's start or end (D-068). Old
/// regions' GUIDs are reused in order; new ones come from `seed`.
pub fn set_masking_regions(
    rec: &RawRecord,
    regions: &[MaskSpec],
    seed: u64,
) -> Result<(Vec<u8>, usize), StationRefError> {
    let l = Layout::read(rec)?;
    let data = &rec.data;
    let n = regions.len();
    let mut spans = Vec::new();
    for (i, r) in regions.iter().enumerate() {
        if (r.lock_to_start && i != 0) || (r.lock_to_end && i + 1 != n) {
            return Err(fail(
                "only the first region locks to the start, the last to the end",
            ));
        }
        let s = if r.lock_to_start { l.start } else { r.start };
        let e = if r.lock_to_end { l.end } else { r.end };
        if s.partial_cmp(&e) != Some(std::cmp::Ordering::Less)
            || s < l.start - 1e-9
            || e > l.end + 1e-9
        {
            return Err(fail(format!(
                "region {s} to {e} must lie inside the alignment ({} to {})",
                l.start, l.end
            )));
        }
        if spans.last().is_some_and(|&(_, pe): &(f64, f64)| s < pe) {
            return Err(fail("regions must be in station order and not overlap"));
        }
        spans.push((s, e));
    }
    let mut used = l.guids();
    let mut seed = seed;
    let ids: Vec<([u8; 16], [u8; 16])> = (0..n)
        .map(|i| match l.regions.get(i) {
            Some(r) => (r.start_guid, r.end_guid),
            None => (
                fresh_guid(&mut seed, &mut used),
                fresh_guid(&mut seed, &mut used),
            ),
        })
        .collect();
    let anchors: Vec<(f64, [u8; 16])> = spans
        .iter()
        .zip(&ids)
        .flat_map(|(&(s, e), &(a, b))| [(s, a), (e, b)])
        .collect();
    let block = l.interval_block(data, KIND_MASK, &anchors)?;
    let mut list = crate::aecc_build::Bits::default();
    list.bl(n as i32);
    for ((r, &(s, e)), (a, b)) in regions.iter().zip(&spans).zip(&ids) {
        list.bd(s);
        list.bd(e);
        list.bit(u8::from(!r.masked));
        for x in a.iter().chain(b) {
            list.byte(*x);
        }
    }
    let (bytes, hb) = splice_all(
        data,
        rec.handle_bits,
        vec![
            (l.block, l.eq_start, block),
            (l.mask_start, l.mask_end, list),
        ],
    );
    // Strings: the regions' comments are the last ones (S246); their four
    // interval strings go before the speeds' and equations' strings.
    let old = l.regions.len();
    let tail = l
        .strings
        .checked_sub(old)
        .ok_or_else(|| fail("string stream too short"))?;
    let comments: Vec<String> = regions.iter().map(|r| r.comment.clone()).collect();
    let (bytes, hb) = edit_strings(bytes, hb, tail, old, &comments)?;
    let at = tail
        .checked_sub(l.speeds + 2 * l.equations.len())
        .ok_or_else(|| fail("string stream too short"))?;
    let (bytes, hb) = adjust_empties(bytes, hb, at, 4 * old, 4 * n)?;
    Ok(crate::handle_refs::trim_padding(&bytes, hb, rec.handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aecc_build::Bits;

    /// The interval block of a production drawing (checked locally): the
    /// three opening BLs are small counters, not 0, and intervals of other
    /// objects carry kinds well above 4 (one per alignment near 0x7FFF).
    #[test]
    fn interval_block_with_counters_and_foreign_kinds() {
        let mut w = Bits::default();
        for v in [2, 1, 0, 2] {
            w.bl(v);
        }
        for (k, st) in [(695, 151.397), (0x7FFA, 2000.0)] {
            for b in 0..16u8 {
                w.byte(b);
            }
            w.bit(1);
            w.bl(k);
            w.bd(0.0);
            w.bd(st);
            w.bd(1.0);
            w.bd(2.0);
            for v in [5, 4, 6] {
                w.bl(v);
            }
            for v in [0.0; 6] {
                w.rd(v);
            }
        }
        w.bl(0);
        let n = w.len();
        let bytes = w.into_bytes();
        let (v, end) = intervals_at(&bytes, 0).unwrap();
        assert_eq!(v.iter().map(|i| i.kind).collect::<Vec<_>>(), [695, 0x7FFA]);
        assert_eq!(v[1].station, 2000.0);
        assert_eq!(end + 2, n);
    }
}
