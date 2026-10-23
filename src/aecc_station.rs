//! Start station of an alignment (O4, S9, S82).
//!
//! The start station (the station of the reference point when it is the
//! alignment's start) is a BD that ends 20 bits before the end of section
//! 2. Civil 3D removes all design speeds when the start station changes
//! (S82, owner checked); civil3drust keeps them and shifts their stations
//! with the start (D-031, [`move_start_station`]).

use crate::aecc_alignment;
use crate::aecc_build::Bits;
use crate::aecc_criteria::section2_end;
use crate::raw::{self, RawRecord};

/// Bits between the end of the start-station BD and the end of section 2.
pub const AFTER_STATION: usize = 20;

/// The start station.
pub fn start_station(rec: &RawRecord) -> Option<f64> {
    let e = section2_end(rec).ok()?;
    aecc_alignment::start_station(&rec.data, e + 21)
}

/// Rewrite the start station (the record grows or shrinks by 64 bits when
/// the value changes between 0/1 and other values). Returns the new bytes
/// and handle-stream size.
pub fn set_start_station(rec: &RawRecord, station: f64) -> Option<(Vec<u8>, usize)> {
    if !station.is_finite() {
        return None;
    }
    let old = start_station(rec)?;
    let e = section2_end(rec).ok()?;
    let bd_end = e - AFTER_STATION;
    let bd_start = if old == 0.0 || old == 1.0 {
        bd_end - 2
    } else {
        bd_end - 66
    };
    let data = &rec.data;
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, bd_start);
    w.bd(station);
    let new_end = w.len();
    w.copy(data, bd_end, total);
    let new_data_end = data_end + new_end - bd_end;
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    Some((bytes, hb))
}

/// Change the start station and shift the design speeds with it (D-031).
/// Alignments without design speeds stay without.
pub fn move_start_station(rec: &RawRecord, station: f64) -> Option<(Vec<u8>, usize)> {
    let old = start_station(rec)?;
    let (data, hb) = crate::aecc_design_speed::shift_design_speeds(rec, station - old).ok()?;
    let r = RawRecord {
        data,
        handle_bits: hb,
        ..rec.clone()
    };
    set_start_station(&r, station)
}
