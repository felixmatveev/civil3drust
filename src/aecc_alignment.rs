//! Decoding `AECC_ALIGNMENT` records into [`Alignment`] geometry.
//!
//! Only the parts documented in `docs/alignment-object.md` are used. The
//! design geometry of a tangent-only alignment is a list of **tangent
//! element records** in the record's data section, each:
//!
//! ```text
//! BL 0x04020008   constant header (meaning unknown; same in every sample)
//! BL 0
//! BL id           element ID, 1-based
//! BL before       ID of the previous element, 0 if first
//! BL after        ID of the next element, 0 if last
//! RD start X, RD start Y, RD end X, RD end Y
//! ```
//!
//! (BL = bit-coded long, RD = raw double, as in the public ODA DWG
//! specification.) Elements are chained by `before`/`after`, which matches
//! the `EntityBefore`/`EntityAfter` properties of alignment entities in
//! Autodesk's public Civil 3D API documentation.
//!
//! The **start station** is a bit-coded double ("BD": code `10` = 0.0,
//! `01` = 1.0, `00` + raw double) that ends 41 bits before the first
//! tangent's header. The first 17 of those 41 bits have been identical in
//! every sample, and are checked before the station is trusted.
//!
//! Arc elements share the header and the id/before/after fields but carry
//! extra parameters and three points; they are decoded (read-only) in
//! [`crate::aecc_elements`]. [`decode`] handles tangent-only alignments and
//! reports [`DecodeError::HasCurves`] otherwise.
//!
//! Not yet decoded: spirals, and everything else in the record.

use crate::alignment::{Alignment, AlignmentError, Point2};
use crate::raw::{self, RawRecord};
use std::fmt;

/// Header value that starts every alignment element record seen so far
/// (tangents and arcs).
pub const ELEMENT_HEADER: i32 = 0x0402_0008;

/// Former name of [`ELEMENT_HEADER`].
pub const TANGENT_HEADER: i32 = ELEMENT_HEADER;

/// An element record linked to its neighbours by ID.
pub trait Linked {
    fn id(&self) -> i32;
    fn before(&self) -> i32;
    fn after(&self) -> i32;
}

impl Linked for TangentRecord {
    fn id(&self) -> i32 {
        self.id
    }
    fn before(&self) -> i32 {
        self.before
    }
    fn after(&self) -> i32 {
        self.after
    }
}

/// One tangent element record found in the data.
#[derive(Debug, Clone, PartialEq)]
pub struct TangentRecord {
    /// Bit offset of the header BL.
    pub header_bit: usize,
    /// Bit offset of the first of the four coordinate doubles.
    pub coords_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub start: Point2,
    pub end: Point2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodeError {
    NoTangents,
    /// Records exist but their before/after links don't form one chain.
    BrokenChain(String),
    /// The chain's tangents don't meet end to start.
    Discontinuous {
        after_id: i32,
    },
    Geometry(AlignmentError),
    /// The alignment contains arc elements; use
    /// [`crate::aecc_elements::decode_elements`] to read it.
    HasCurves {
        arcs: usize,
    },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTangents => write!(f, "no element records found"),
            Self::BrokenChain(why) => write!(f, "element links don't form a chain: {why}"),
            Self::Discontinuous { after_id } => {
                write!(
                    f,
                    "tangent {after_id} does not end where the next one starts"
                )
            }
            Self::Geometry(e) => write!(f, "{e}"),
            Self::HasCurves { arcs } => write!(
                f,
                "contains {arcs} arc element(s); only tangent-only alignments can be edited"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Find every tangent element record in `data` (from bit `from` on).
///
/// A record is accepted only if the five BLs parse, the IDs are small and
/// non-negative, the four doubles are finite, and the tangent has length.
pub fn tangent_records(data: &[u8], from: usize) -> Vec<TangentRecord> {
    let mut out = Vec::new();
    let mut bit = from;
    let n = raw::bit_len(data);
    while bit + 34 <= n {
        if let Some(rec) = record_at(data, bit) {
            bit = rec.coords_bit + 256;
            out.push(rec);
        } else {
            bit += 1;
        }
    }
    out
}

pub(crate) fn record_at(data: &[u8], bit: usize) -> Option<TangentRecord> {
    // Cheap pre-check: code 00 followed by the header's first byte (0x08).
    if raw::get_bit(data, bit) != 0
        || raw::get_bit(data, bit + 1) != 0
        || raw::get_byte(data, bit + 2) != 0x08
    {
        return None;
    }
    let (h, k) = raw::read_bl(data, bit)?;
    if h != ELEMENT_HEADER {
        return None;
    }
    let (zero, k) = raw::read_bl(data, k)?;
    let (id, k) = raw::read_bl(data, k)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let small = |v: i32| (0..100_000).contains(&v);
    if zero != 0 || id < 1 || !small(id) || !small(before) || !small(after) {
        return None;
    }
    if k + 256 > raw::bit_len(data) {
        return None;
    }
    let c: Vec<f64> = (0..4).map(|i| raw::get_f64(data, k + 64 * i)).collect();
    if c.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let start = Point2::new(c[0], c[1]);
    let end = Point2::new(c[2], c[3]);
    if start.distance_to(end) <= f64::EPSILON {
        return None;
    }
    Some(TangentRecord {
        header_bit: bit,
        coords_bit: k,
        id,
        before,
        after,
        start,
        end,
    })
}

/// Order element records by their before/after links, first element first.
pub fn chain<T: Linked + Clone>(records: &[T]) -> Result<Vec<T>, DecodeError> {
    if records.is_empty() {
        return Err(DecodeError::NoTangents);
    }
    let firsts: Vec<&T> = records.iter().filter(|r| r.before() == 0).collect();
    if firsts.len() != 1 {
        return Err(DecodeError::BrokenChain(format!(
            "{} elements have no predecessor",
            firsts.len()
        )));
    }
    let mut ordered = vec![firsts[0].clone()];
    while ordered.last().map_or(0, |r| r.after()) != 0 {
        let cur = ordered.last().expect("non-empty");
        let next = records
            .iter()
            .find(|r| r.id() == cur.after())
            .ok_or_else(|| {
                DecodeError::BrokenChain(format!(
                    "element {} points to missing {}",
                    cur.id(),
                    cur.after()
                ))
            })?;
        if next.before() != cur.id() {
            return Err(DecodeError::BrokenChain(format!(
                "element {} says it follows {}, not {}",
                next.id(),
                next.before(),
                cur.id()
            )));
        }
        if ordered.len() > records.len() {
            return Err(DecodeError::BrokenChain("cycle".into()));
        }
        ordered.push(next.clone());
    }
    if ordered.len() != records.len() {
        return Err(DecodeError::BrokenChain(format!(
            "chain has {} of {} elements",
            ordered.len(),
            records.len()
        )));
    }
    Ok(ordered)
}

/// A decoded alignment record.
#[derive(Debug, Clone)]
pub struct DecodedAlignment {
    pub handle: u64,
    /// Tangent records in chain order.
    pub tangents: Vec<TangentRecord>,
    /// The start station as stored, if it could be located (see
    /// [`start_station`]). `alignment.start_station` falls back to 0.
    pub stored_start_station: Option<f64>,
    pub alignment: Alignment,
}

/// Bits between the start-station BD and the first tangent header.
pub const STATION_GAP_BITS: usize = 41;

/// The leading part of that gap, identical in every sample so far.
const STATION_GAP_PREFIX: [u8; 17] = [0, 0, 0, 0, 2, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0];

/// Read the start station stored before the tangent whose header is at
/// `first_header_bit` (the first element of the chain).
///
/// Returns `None` if the 41-bit gap doesn't have its usual prefix or no
/// plausible bit-coded double ends where the gap begins.
pub fn start_station(data: &[u8], first_header_bit: usize) -> Option<f64> {
    let gap = first_header_bit.checked_sub(STATION_GAP_BITS)?;
    // 2 in the prefix table means "varies between samples".
    let prefix_ok = STATION_GAP_PREFIX
        .iter()
        .enumerate()
        .all(|(i, &want)| want == 2 || raw::get_bit(data, gap + i) == want);
    if !prefix_ok {
        return None;
    }
    // Full double: code 00 then 64 bits, ending at `gap`.
    if let Some(start) = gap.checked_sub(66) {
        if raw::get_bit(data, start) == 0 && raw::get_bit(data, start + 1) == 0 {
            let v = raw::get_f64(data, start + 2);
            if v.is_finite() && (v == 0.0 || (1e-9..1e9).contains(&v.abs())) {
                return Some(v);
            }
        }
    }
    match (
        raw::get_bit(data, gap.checked_sub(2)?),
        raw::get_bit(data, gap - 1),
    ) {
        (1, 0) => Some(0.0),
        (0, 1) => Some(1.0),
        _ => None,
    }
}

/// Name from the string stream (see [`raw::object_name`]).
pub fn record_name(data: &[u8]) -> Option<String> {
    raw::object_name(data)
}

/// Decode a tangent-only alignment record.
///
/// Searches the whole record: the proxy-graphics section never contains the
/// tangent header pattern followed by valid links in the samples studied.
/// The station-reference cache repeats tangent coordinates but without the
/// header, so it is not picked up.
pub fn decode(rec: &RawRecord) -> Result<DecodedAlignment, DecodeError> {
    let ordered = match chain(&tangent_records(&rec.data, 0)) {
        Ok(o) => o,
        Err(e) => {
            let arcs = crate::aecc_elements::element_records(&rec.data)
                .iter()
                .filter(|el| matches!(el, crate::aecc_elements::Element::Arc(_)))
                .count();
            return Err(if arcs > 0 {
                DecodeError::HasCurves { arcs }
            } else {
                e
            });
        }
    };
    for w in ordered.windows(2) {
        if w[0].end.distance_to(w[1].start) > 1e-6 * (1.0 + w[0].end.x.abs().max(w[0].end.y.abs()))
        {
            return Err(DecodeError::Discontinuous { after_id: w[0].id });
        }
    }
    let mut pis: Vec<Point2> = ordered.iter().map(|t| t.start).collect();
    pis.push(ordered.last().expect("non-empty").end);
    let name = record_name(&rec.data).unwrap_or_else(|| format!("{:X}", rec.handle));
    let stored_start_station = start_station(&rec.data, ordered[0].header_bit);
    let alignment = Alignment::new(name, stored_start_station.unwrap_or(0.0), pis)
        .map_err(DecodeError::Geometry)?;
    Ok(DecodedAlignment {
        handle: rec.handle,
        tangents: ordered,
        stored_start_station,
        alignment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal MSB-first bit writer for building synthetic records.
    struct Bits(Vec<u8>, usize);
    impl Bits {
        fn new() -> Self {
            Bits(Vec::new(), 0)
        }
        fn bit(&mut self, b: u8) {
            if self.1.is_multiple_of(8) {
                self.0.push(0);
            }
            if b != 0 {
                let i = self.1 / 8;
                self.0[i] |= 1 << (7 - self.1 % 8);
            }
            self.1 += 1;
        }
        fn byte(&mut self, v: u8) {
            for i in (0..8).rev() {
                self.bit((v >> i) & 1);
            }
        }
        fn bl(&mut self, v: i32) {
            if v == 0 {
                self.bit(1);
                self.bit(0);
            } else if (1..256).contains(&v) {
                self.bit(0);
                self.bit(1);
                self.byte(v as u8);
            } else {
                self.bit(0);
                self.bit(0);
                v.to_le_bytes().iter().for_each(|b| self.byte(*b));
            }
        }
        fn rd(&mut self, v: f64) {
            v.to_le_bytes().iter().for_each(|b| self.byte(*b));
        }
        fn bd(&mut self, v: f64) {
            if v == 0.0 {
                self.bit(1);
                self.bit(0);
            } else if v == 1.0 {
                self.bit(0);
                self.bit(1);
            } else {
                self.bit(0);
                self.bit(0);
                self.rd(v);
            }
        }
        /// Start station plus the 41-bit gap as seen in sample S5.
        fn station(&mut self, v: f64) {
            self.bd(v);
            for c in "00001111000100000001010000001100000000010".bytes() {
                self.bit(c - b'0');
            }
        }
        fn tangent(&mut self, id: i32, before: i32, after: i32, a: (f64, f64), b: (f64, f64)) {
            self.bl(TANGENT_HEADER);
            self.bl(0);
            self.bl(id);
            self.bl(before);
            self.bl(after);
            self.rd(a.0);
            self.rd(a.1);
            self.rd(b.0);
            self.rd(b.1);
        }
    }

    fn record(data: Vec<u8>) -> RawRecord {
        RawRecord {
            handle: 0xABC,
            class: "AECC_ALIGNMENT".into(),
            is_entity: true,
            data,
        }
    }

    #[test]
    fn reads_bl_forms() {
        let mut w = Bits::new();
        w.bl(0);
        w.bl(3);
        w.bl(TANGENT_HEADER);
        let (a, k) = raw::read_bl(&w.0, 0).unwrap();
        let (b, k) = raw::read_bl(&w.0, k).unwrap();
        let (c, k) = raw::read_bl(&w.0, k).unwrap();
        assert_eq!((a, b, c, k), (0, 3, TANGENT_HEADER, 2 + 10 + 34));
    }

    #[test]
    fn decodes_three_tangents_stored_out_of_order() {
        // Same geometry as sample S5, with records stored 3, 1, 2 and junk
        // bits in between, at odd bit offsets.
        let mut w = Bits::new();
        for _ in 0..5 {
            w.bit(1);
        }
        w.tangent(3, 2, 0, (200.0, 50.0), (300.0, 50.0));
        w.byte(0x5A);
        w.tangent(1, 0, 2, (0.0, 0.0), (100.0, 0.0));
        w.bit(0);
        w.bit(1);
        w.bit(1);
        w.tangent(2, 1, 3, (100.0, 0.0), (200.0, 50.0));
        // A cache record: tangent coordinates without the header.
        w.rd(0.0);
        w.rd(0.0);
        w.rd(100.0);
        w.rd(0.0);
        let d = decode(&record(w.0)).unwrap();
        assert_eq!(
            d.tangents.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            d.alignment.pis(),
            &[
                Point2::new(0.0, 0.0),
                Point2::new(100.0, 0.0),
                Point2::new(200.0, 50.0),
                Point2::new(300.0, 50.0)
            ]
        );
        assert!((d.alignment.length() - (100.0 + 50f64.hypot(100.0) + 100.0)).abs() < 1e-9);
    }

    #[test]
    fn reads_start_station_in_all_bd_forms() {
        for sta in [0.0, 1.0, 100.0, 1000.0, 2000.5] {
            let mut w = Bits::new();
            w.byte(0xFF);
            w.station(sta);
            w.tangent(1, 0, 0, (0.0, 0.0), (100.0, 0.0));
            let d = decode(&record(w.0)).unwrap();
            assert_eq!(d.stored_start_station, Some(sta));
            assert_eq!(d.alignment.start_station, sta);
            assert_eq!(d.alignment.end_station(), sta + 100.0);
        }
    }

    #[test]
    fn unknown_station_when_gap_does_not_match() {
        let mut w = Bits::new();
        w.bd(100.0);
        for _ in 0..41 {
            w.bit(1);
        }
        w.tangent(1, 0, 0, (0.0, 0.0), (100.0, 0.0));
        let d = decode(&record(w.0)).unwrap();
        assert_eq!(d.stored_start_station, None);
        assert_eq!(d.alignment.start_station, 0.0);
    }

    #[test]
    fn rejects_broken_links() {
        let mut w = Bits::new();
        w.tangent(1, 0, 2, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 5, 0, (100.0, 0.0), (200.0, 0.0));
        assert!(matches!(
            decode(&record(w.0)),
            Err(DecodeError::BrokenChain(_))
        ));
    }

    #[test]
    fn rejects_discontinuous_chain() {
        let mut w = Bits::new();
        w.tangent(1, 0, 2, (0.0, 0.0), (100.0, 0.0));
        w.tangent(2, 1, 0, (101.0, 0.0), (200.0, 0.0));
        assert_eq!(
            decode(&record(w.0)).unwrap_err(),
            DecodeError::Discontinuous { after_id: 1 }
        );
    }

    #[test]
    fn no_records_is_an_error() {
        assert_eq!(
            decode(&record(vec![0u8; 64])).unwrap_err(),
            DecodeError::NoTangents
        );
    }
}
