//! Proxy-entity graphics embedded in AECC entity records.
//!
//! The display cache AutoCAD draws when Civil 3D isn't loaded. Format per the
//! public ODA *Open Design Specification for .dwg files*: `[i32 total
//! bytes][i32 chunk count]`, then chunks of `[i32 chunk bytes][i32
//! opcode][body]`, all raw little-endian at whatever bit offset the stream
//! starts. Chunk bytes include the 8-byte chunk header.

use crate::raw;

pub const OP_CIRCLE: i32 = 2;
pub const OP_ARC: i32 = 4;
pub const OP_POLYLINE: i32 = 6;
pub const OP_POLYGON: i32 = 7;
pub const OP_UNICODE_TEXT: i32 = 36;

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// Bit offset of the chunk header.
    pub bit: usize,
    /// Size in bytes, header included.
    pub size: usize,
    pub opcode: i32,
}

impl Chunk {
    /// Bit offset of the body (after size and opcode).
    pub fn body(&self) -> usize {
        self.bit + 64
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Graphics {
    pub start_bit: usize,
    pub size: usize,
    pub chunks: Vec<Chunk>,
}

impl Graphics {
    pub fn end_bit(&self) -> usize {
        self.start_bit + self.size * 8
    }

    /// True for the placeholder image Civil 3D writes before it has drawn an
    /// object: a text chunk and an extents box (docs/alignment-object.md).
    pub fn is_placeholder(&self) -> bool {
        self.chunks.iter().any(|c| c.opcode == OP_UNICODE_TEXT)
    }
}

/// Locate the graphics stream: the first start offset within `search_bits`
/// whose chunk walk lands exactly on the declared total size.
pub fn find(buf: &[u8], search_bits: usize) -> Option<Graphics> {
    let nbits = raw::bit_len(buf);
    for start in 0..search_bits.min(nbits.saturating_sub(64)) {
        let size = raw::get_i32(buf, start);
        let count = raw::get_i32(buf, start + 32);
        if size < 16 || !(1..=100_000).contains(&count) {
            continue;
        }
        let size = size as usize;
        let end = start + size * 8;
        if end > nbits {
            continue;
        }
        let mut chunks = Vec::with_capacity(count as usize);
        let mut bit = start + 64;
        let mut ok = true;
        for _ in 0..count {
            if bit + 64 > end {
                ok = false;
                break;
            }
            let cs = raw::get_i32(buf, bit);
            let op = raw::get_i32(buf, bit + 32);
            if cs < 8 || cs % 4 != 0 || !(0..200).contains(&op) {
                ok = false;
                break;
            }
            chunks.push(Chunk {
                bit,
                size: cs as usize,
                opcode: op,
            });
            bit += cs as usize * 8;
        }
        if ok && bit == end {
            return Some(Graphics {
                start_bit: start,
                size,
                chunks,
            });
        }
    }
    None
}

/// Vertices of a polyline or polygon chunk (`[i32 n][n × 3 doubles]`), as
/// `(bit offset of X, [x, y, z])`.
pub fn poly_vertices(buf: &[u8], c: &Chunk) -> Vec<(usize, [f64; 3])> {
    let n = raw::get_i32(buf, c.body());
    if n <= 0 || 8 + 4 + n as usize * 24 > c.size {
        return Vec::new();
    }
    (0..n as usize)
        .map(|i| {
            let b = c.body() + 32 + i * 192;
            (
                b,
                [
                    raw::get_f64(buf, b),
                    raw::get_f64(buf, b + 64),
                    raw::get_f64(buf, b + 128),
                ],
            )
        })
        .collect()
}

/// Circle chunk center: `(bit offset of X, [x, y, z])`.
pub fn circle_center(buf: &[u8], c: &Chunk) -> (usize, [f64; 3]) {
    let b = c.body();
    (
        b,
        [
            raw::get_f64(buf, b),
            raw::get_f64(buf, b + 64),
            raw::get_f64(buf, b + 128),
        ],
    )
}
