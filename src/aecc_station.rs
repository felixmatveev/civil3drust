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

/// The ID of the first element of the path, stored twice in section 2
/// (S93 → S94, first tangent deleted: both 1 → 4): as a one-byte BL
/// followed by `10` right before the start-station BD, and as the BL
/// that ends section 2. Returns the two bit offsets and the stored ID,
/// if both are one-byte BLs holding the same value.
pub fn first_element_fields(rec: &RawRecord) -> Option<(usize, usize, i32)> {
    let data = &rec.data;
    let e = section2_end(rec).ok()?;
    let station = start_station(rec)?;
    let bd_len = if station == 0.0 || station == 1.0 {
        2
    } else {
        66
    };
    let bd_start = e - AFTER_STATION - bd_len;
    let a = bd_start.checked_sub(12)?;
    let b = e - 10;
    let one_byte = |bit: usize| raw::get_bit(data, bit) == 0 && raw::get_bit(data, bit + 1) == 1;
    if !one_byte(a)
        || !one_byte(b)
        || raw::get_bit(data, a + 10) != 1
        || raw::get_bit(data, a + 11) != 0
    {
        return None;
    }
    let (ia, ib) = (raw::get_byte(data, a + 2), raw::get_byte(data, b + 2));
    (ia == ib).then_some((a, b, ia as i32))
}

/// Set both copies of the first element's ID (see
/// [`first_element_fields`]) in place; IDs 1..=255 keep the field size.
pub fn set_first_element(rec: &RawRecord, data: &mut [u8], id: i32) -> Option<()> {
    if !(1..=255).contains(&id) {
        return None;
    }
    let (a, b, _) = first_element_fields(rec)?;
    raw::set_byte(data, a + 2, id as u8);
    raw::set_byte(data, b + 2, id as u8);
    Some(())
}
