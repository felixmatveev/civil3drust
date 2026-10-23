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
    let mut refs = parse(&rec.data, total - rec.handle_bits, total, rec.handle);
    // A whole zero byte at the end is padding, not a reference (code 0 is
    // never used in these streams; S56 had one from accumulated padding).
    while refs.last().is_some_and(|r| r.code == 0 && r.counter == 0) {
        refs.pop();
    }
    refs
}

/// Drop padding after the last reference of the handle stream and re-pad
/// to a whole byte with zero bits. Returns the new bytes and
/// handle-stream size.
pub fn trim_padding(data: &[u8], handle_bits: usize, own: u64) -> (Vec<u8>, usize) {
    let total = raw::bit_len(data);
    let start = total - handle_bits;
    let mut refs = parse(data, start, total, own);
    while refs.last().is_some_and(|r| r.code == 0 && r.counter == 0) {
        refs.pop();
    }
    let end = refs.last().map(|r| r.bit + r.bits()).unwrap_or(start);
    let mut bits: Vec<u8> = (0..end).map(|b| raw::get_bit(data, b)).collect();
    while !bits.len().is_multiple_of(8) {
        bits.push(0);
    }
    let hb = bits.len() - start;
    (pack(&bits), hb)
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

/// Rebuild the handle stream of a record (`data`, with `handle_bits` bits
/// of handle stream at its end), replacing the references at `null_at`
/// (indices into [`parse`]'s list) with null references of the same code
/// (one byte, counter 0). The data stream is copied unchanged; the new
/// stream is padded with zero bits to a whole byte. Returns the new record
/// bytes and handle-stream size in bits.
pub fn null_refs(data: &[u8], handle_bits: usize, own: u64, null_at: &[usize]) -> (Vec<u8>, usize) {
    let total = raw::bit_len(data);
    let start = total - handle_bits;
    let refs = parse(data, start, total, own);
    let mut bits: Vec<u8> = (0..start).map(|b| raw::get_bit(data, b)).collect();
    for (i, r) in refs.iter().enumerate() {
        if null_at.contains(&i) {
            let b = r.code << 4;
            bits.extend((0..8).rev().map(|k| (b >> k) & 1));
        } else {
            bits.extend((r.bit..r.bit + r.bits()).map(|b| raw::get_bit(data, b)));
        }
    }
    while !bits.len().is_multiple_of(8) {
        bits.push(0);
    }
    let mut out = vec![0u8; bits.len() / 8];
    for (i, b) in bits.iter().enumerate() {
        out[i / 8] |= b << (7 - i % 8);
    }
    let hb = bits.len() - start;
    (out, hb)
}

/// Bit offset of the object's own handle (`CODE 0 | COUNTER` byte + bytes)
/// in every AECC record studied (after the 2-byte object type).
pub const OWN_HANDLE_BIT: usize = 18;

/// The object's own handle stored at [`OWN_HANDLE_BIT`].
pub fn own_handle(data: &[u8]) -> Option<u64> {
    let form = raw::get_byte(data, OWN_HANDLE_BIT);
    if form >> 4 != 0 || form & 0x0F > 8 {
        return None;
    }
    let n = (form & 0x0F) as usize;
    Some((0..n).fold(0u64, |v, i| {
        v << 8 | raw::get_byte(data, OWN_HANDLE_BIT + 8 + 8 * i) as u64
    }))
}

fn pack(bits: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, b) in bits.iter().enumerate() {
        out[i / 8] |= b << (7 - i % 8);
    }
    out
}

fn push_byte(out: &mut Vec<u8>, v: u8) {
    out.extend((0..8).rev().map(|k| (v >> k) & 1));
}

/// Rewrite the object's own handle (minimal byte count; the record size
/// can change). Returns the new bytes and handle-stream size.
pub fn set_own_handle(data: &[u8], handle_bits: usize, new: u64) -> Option<(Vec<u8>, usize)> {
    let form = raw::get_byte(data, OWN_HANDLE_BIT);
    if form >> 4 != 0 {
        return None;
    }
    let old_end = OWN_HANDLE_BIT + 8 + 8 * (form & 0x0F) as usize;
    let total = raw::bit_len(data);
    let data_end = total - handle_bits;
    let mut bits: Vec<u8> = (0..OWN_HANDLE_BIT).map(|b| raw::get_bit(data, b)).collect();
    let n = if new == 0 {
        0
    } else {
        8 - new.leading_zeros() as usize / 8
    };
    push_byte(&mut bits, n as u8);
    for i in (0..n).rev() {
        push_byte(&mut bits, (new >> (8 * i)) as u8);
    }
    let shift = bits.len() as isize - old_end as isize;
    bits.extend((old_end..total).map(|b| raw::get_bit(data, b)));
    let new_data_end = (data_end as isize + shift) as usize;
    // Drop old padding past the handle stream's references, re-pad.
    let refs = parse(&pack(&bits), new_data_end, bits.len(), new);
    if let Some(last) = refs.last() {
        bits.truncate(last.bit + last.bits());
    }
    while !bits.len().is_multiple_of(8) {
        bits.push(0);
    }
    let hb = bits.len() - new_data_end;
    Some((pack(&bits), hb))
}

/// Re-encode the whole handle stream of a record whose own handle is `own`:
/// reference `i` points at `targets[i]`, keeping its kind. Absolute codes
/// (2–5) use the minimal byte count; relative codes are recomputed (`6`/`8`
/// when the target is own ± 1 and the original used them, else `A`/`C`
/// with a minimal offset). Returns the new bytes and handle-stream size.
pub fn rewrite_refs(
    data: &[u8],
    handle_bits: usize,
    own: u64,
    targets: &[u64],
) -> Option<(Vec<u8>, usize)> {
    let total = raw::bit_len(data);
    let data_end = total - handle_bits;
    let refs = parse(data, data_end, total, own);
    if refs.len() != targets.len() {
        return None;
    }
    let mut bits: Vec<u8> = (0..data_end).map(|b| raw::get_bit(data, b)).collect();
    for (r, &t) in refs.iter().zip(targets) {
        let (code, value) = match r.code {
            2..=5 => (r.code, t),
            0x6 | 0x8 | 0xA | 0xC => {
                if r.code == 0x6 && t == own.wrapping_add(1) {
                    (0x6, 0)
                } else if r.code == 0x8 && t == own.wrapping_sub(1) {
                    (0x8, 0)
                } else if t > own {
                    (0xA, t - own)
                } else {
                    (0xC, own - t)
                }
            }
            _ => return None,
        };
        let n = if matches!(code, 0x6 | 0x8) || value == 0 {
            0
        } else {
            8 - value.leading_zeros() as usize / 8
        };
        push_byte(&mut bits, (code << 4) | n as u8);
        for i in (0..n).rev() {
            push_byte(&mut bits, (value >> (8 * i)) as u8);
        }
    }
    while !bits.len().is_multiple_of(8) {
        bits.push(0);
    }
    let hb = bits.len() - data_end;
    Some((pack(&bits), hb))
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

    #[test]
    fn null_refs_shrinks_stream() {
        // 4 data bits "1010", then refs 5:10 5.2:0FB7 5.0, padded.
        let mut bits = vec![1u8, 0, 1, 0];
        for byte in [0x51u8, 0x10, 0x52, 0x0F, 0xB7, 0x50] {
            bits.extend((0..8).rev().map(|k| (byte >> k) & 1));
        }
        while !bits.len().is_multiple_of(8) {
            bits.push(0);
        }
        let mut d = vec![0u8; bits.len() / 8];
        for (i, b) in bits.iter().enumerate() {
            d[i / 8] |= b << (7 - i % 8);
        }
        let hb = bits.len() - 4;
        let (out, nhb) = null_refs(&d, hb, 0xFB8, &[1]);
        let refs = parse(&out, 4, out.len() * 8, 0xFB8);
        let v: Vec<(u8, u64)> = refs.iter().map(|r| (r.code, r.handle)).collect();
        assert_eq!(v, [(5, 0x10), (5, 0), (5, 0)]);
        assert_eq!(out.len() * 8 - nhb, 4);
        assert_eq!(raw::get_bit(&out, 0), 1);
        assert_eq!(out.len(), d.len() - 2);
    }

    #[test]
    fn rewrite_and_own_handle() {
        // header: 18 bits, own handle 02 0F B8, 6 data bits, refs C.2 03F7, 4.2 0FB8, 5.0
        let mut bits = vec![0u8; 18];
        for b in [0x02u8, 0x0F, 0xB8] {
            bits.extend((0..8).rev().map(|k| (b >> k) & 1));
        }
        bits.extend([1, 0, 1, 1, 0, 1]);
        let de = bits.len();
        for b in [0xC2u8, 0x03, 0xF7, 0x42, 0x0F, 0xB8, 0x50] {
            bits.extend((0..8).rev().map(|k| (b >> k) & 1));
        }
        while !bits.len().is_multiple_of(8) {
            bits.push(0);
        }
        let d = pack(&bits);
        let hb = d.len() * 8 - de;
        assert_eq!(own_handle(&d), Some(0xFB7 + 1));
        let (d2, hb2) = set_own_handle(&d, hb, 0x12345).unwrap();
        assert_eq!(own_handle(&d2), Some(0x12345));
        let refs: Vec<u64> = parse(&d2, d2.len() * 8 - hb2, d2.len() * 8, 0x12345)
            .iter()
            .map(|r| r.raw)
            .collect();
        assert_eq!(refs, [0x3F7, 0xFB8, 0]); // stored values unchanged
        let (d3, hb3) = rewrite_refs(&d2, hb2, 0x12345, &[0xBC0, 0x12346, 0]).unwrap();
        let r3 = parse(&d3, d3.len() * 8 - hb3, d3.len() * 8, 0x12345);
        let got: Vec<(u8, u64)> = r3.iter().map(|r| (r.code, r.handle)).collect();
        assert_eq!(got, [(0xC, 0xBC0), (4, 0x12346), (5, 0)]);
        assert_eq!(raw::get_bit(&d3, 18 + 32), 1); // data bits kept
    }
}
