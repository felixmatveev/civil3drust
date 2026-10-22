//! Handle references in a record's handle stream.
//!
//! The handle stream is the last [`RawRecord::handle_bits`] bits of the
//! record. Each reference is, per the public ODA DWG specification (and
//! acadrust's `read_handle_reference`), one byte `CODE(4) | COUNTER(4)`
//! followed by `COUNTER` handle bytes, most significant first.
//!
//! Codes 2–5 carry an absolute handle (2 soft owner, 3 hard owner,
//! 4 soft pointer, 5 hard pointer). Codes 6, 8, 0xA and 0xC are relative to
//! the referencing object's own handle (+1, −1, +offset, −offset).
//!
//! For `AECC_ALIGNMENT` the reference list is documented in
//! docs/alignment-object.md ("Handle stream", S18/S19).

use crate::raw::{self, RawRecord};

/// One decoded handle reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandleRef {
    /// Bit offset of the code/counter byte within the record.
    pub bit: usize,
    /// Reference code (high nibble).
    pub code: u8,
    /// Number of handle bytes that follow.
    pub counter: u8,
    /// The stored value (absolute handle or offset, depending on `code`).
    pub raw: u64,
    /// The referenced handle, resolved against the owning record's handle.
    pub handle: u64,
}

impl HandleRef {
    /// Size of the reference in bits.
    pub fn bits(&self) -> usize {
        8 + 8 * self.counter as usize
    }
}

/// Decode references from `data[from..to]` (bit offsets); `own` is the
/// record's handle, used for relative codes. Stops at the first reference
/// that would run past `to`.
pub fn parse(data: &[u8], from: usize, to: usize, own: u64) -> Vec<HandleRef> {
    let mut out = Vec::new();
    let mut bit = from;
    while bit + 8 <= to {
        let form = raw::get_byte(data, bit);
        let (code, counter) = (form >> 4, form & 0x0F);
        let len = 8 + 8 * counter as usize;
        if bit + len > to || counter > 8 {
            break;
        }
        let mut v: u64 = 0;
        for i in 0..counter as usize {
            v = (v << 8) | raw::get_byte(data, bit + 8 + 8 * i) as u64;
        }
        let handle = match code {
            0x6 => own.wrapping_add(1),
            0x8 => own.wrapping_sub(1),
            0xA => own.wrapping_add(v),
            0xC => own.wrapping_sub(v),
            _ => v,
        };
        out.push(HandleRef {
            bit,
            code,
            counter,
            raw: v,
            handle,
        });
        bit += len;
    }
    out
}

/// All references in `rec`'s handle stream (empty if its size is unknown).
pub fn of_record(rec: &RawRecord) -> Vec<HandleRef> {
    let total = raw::bit_len(&rec.data);
    if rec.handle_bits == 0 || rec.handle_bits > total {
        return Vec::new();
    }
    parse(&rec.data, total - rec.handle_bits, total, rec.handle)
}

/// Rewrite the absolute reference `r` in `data` to point at `handle`.
/// The new handle must fit in the same number of bytes (`r.counter`), so
/// the record size is unchanged; returns false (and changes nothing)
/// otherwise or if `r` is relative.
pub fn retarget(data: &mut [u8], r: &HandleRef, handle: u64) -> bool {
    if !(2..=5).contains(&r.code) || r.counter == 0 {
        return false;
    }
    let n = r.counter as usize;
    if n < 8 && handle >> (8 * n) != 0 {
        return false;
    }
    // Civil 3D writes the minimal byte count; keep that invariant.
    if n > 1 && handle >> (8 * (n - 1)) == 0 {
        return false;
    }
    for i in 0..n {
        let byte = (handle >> (8 * (n - 1 - i))) as u8;
        raw::set_byte(data, r.bit + 8 + 8 * i, byte);
    }
    true
}

/// Rewrite reference `r` (absolute or relative, codes 2–5, 0xA, 0xC) in a
/// record whose own handle is `own` so that it points at `target`, keeping
/// its code and byte count. Returns false (and changes nothing) if the new
/// stored value would need a different number of bytes, or the code can't
/// express `target` (6/8 only mean own±1).
pub fn retarget_any(data: &mut [u8], r: &HandleRef, own: u64, target: u64) -> bool {
    let value = match r.code {
        2..=5 => return retarget(data, r, target),
        0x6 => return target == own.wrapping_add(1),
        0x8 => return target == own.wrapping_sub(1),
        0xA if target > own => target - own,
        0xC if target < own => own - target,
        _ => return false,
    };
    let n = r.counter as usize;
    if n == 0 || (n < 8 && value >> (8 * n) != 0) || (n > 1 && value >> (8 * (n - 1)) == 0) {
        return false;
    }
    for i in 0..n {
        raw::set_byte(data, r.bit + 8 + 8 * i, (value >> (8 * (n - 1 - i))) as u8);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The S18 alignment's handle stream (bits 5932..6048), byte-aligned
    /// here: 5:10 4:0 5:8C5B 5:8C5B 4:0 5:8DE7 5:0.
    const S18: [u8; 14] = [
        0x51, 0x10, 0x40, 0x52, 0x8C, 0x5B, 0x52, 0x8C, 0x5B, 0x40, 0x52, 0x8D, 0xE7, 0x50,
    ];

    #[test]
    fn parses_s18_stream() {
        let refs = parse(&S18, 0, S18.len() * 8, 0x8C5C);
        let v: Vec<(u8, u64)> = refs.iter().map(|r| (r.code, r.handle)).collect();
        assert_eq!(
            v,
            [
                (5, 0x10),
                (4, 0),
                (5, 0x8C5B),
                (5, 0x8C5B),
                (4, 0),
                (5, 0x8DE7),
                (5, 0)
            ]
        );
        assert_eq!(refs[5].bit, 80);
    }

    #[test]
    fn relative_codes() {
        let d = [0x60, 0x80, 0xA1, 0x05, 0xC1, 0x05];
        let h: Vec<u64> = parse(&d, 0, 48, 100).iter().map(|r| r.handle).collect();
        assert_eq!(h, [101, 99, 105, 95]);
    }

    #[test]
    fn retarget_same_size_only() {
        let mut d = S18;
        let refs = parse(&d, 0, d.len() * 8, 0x8C5C);
        assert!(retarget(&mut d, &refs[5], 0x8DFE));
        assert_eq!(parse(&d, 0, d.len() * 8, 0x8C5C)[5].handle, 0x8DFE);
        assert!(!retarget(&mut d, &refs[5], 0x42)); // fits 1 byte: not minimal
        assert!(!retarget(&mut d, &refs[5], 0x12345));
        assert!(!retarget(&mut d, &refs[1], 0x10)); // zero-length null ref
    }

    #[test]
    fn retarget_relative() {
        // own-off (C.2): own 0xFB7, offset 0x3F7 -> 0xBC0
        let mut d = [0xC2, 0x03, 0xF7];
        let r = parse(&d, 0, 24, 0xFB7)[0];
        assert_eq!(r.handle, 0xBC0);
        assert!(retarget_any(&mut d, &r, 0xFC0, 0xBC0));
        assert_eq!(parse(&d, 0, 24, 0xFC0)[0].handle, 0xBC0);
        assert!(!retarget_any(&mut d, &r, 0xBD0, 0xBC0)); // offset 0x10: 1 byte
    }
}
