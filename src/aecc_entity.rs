//! The common entity data at the start of an AECC entity's "section 2"
//! (docs/alignment-object.md, "Section 2: common entity data").
//!
//! After the object header and the proxy graphics, every `AECC_ALIGNMENT`
//! record continues with the common entity data of the public ODA DWG
//! specification (R2010+ layout):
//!
//! ```text
//! BB entity mode, BL reactor count, B no-xdictionary flag, B has-DS-data,
//! BS colour (ENC), BD linetype scale, BB linetype flags,
//! BB plot-style flags, BB material flags, RC shadow flags,
//! B B B visual-style flags, BS invisibility, RC lineweight
//! ```
//!
//! The class's own data starts right after (with `BL 80, BL 0, BL 80,
//! BL 80, BL 0` in every sample). Checked on all 249 alignment records of
//! S1–S112: 242 have colour, linetype and lineweight BYLAYER; S18–S23's
//! alignment has them BYBLOCK; S1's has an extension dictionary (its
//! handle stream starts with one more reference).

use crate::aecc_build::read_bll;
use crate::raw;

/// Bit of the EED size field: after the object type (BOT: `00`/`01` + 1
/// byte, `10` + 2 bytes) and the own handle (code/counter byte + counter
/// bytes). 42 for class numbers ≥ 756 and 2-byte handles; S68's alignment
/// (class 522, from S64's AutoCAD-made class table) starts 8 bits earlier.
pub fn eed_bit(data: &[u8]) -> Option<usize> {
    let type_bits = match (raw::get_bit(data, 0), raw::get_bit(data, 1)) {
        (1, 0) => 18,
        (0, _) => 10,
        _ => return None,
    };
    let counter = raw::get_byte(data, type_bits) & 0x0F;
    Some(type_bits + 8 + 8 * counter as usize)
}

/// Colour index values.
pub const COLOR_BYBLOCK: u16 = 0;
pub const COLOR_BYLAYER: u16 = 256;
/// DWG lineweight codes.
pub const LINEWEIGHT_BYLAYER: u8 = 29;
pub const LINEWEIGHT_BYBLOCK: u8 = 30;

/// Common entity data of an AECC entity record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityCommon {
    /// Bit offset of the first field (the end of the proxy graphics).
    pub start_bit: usize,
    pub mode: u8,
    pub reactors: i32,
    pub has_xdictionary: bool,
    pub has_ds_data: bool,
    /// Colour index (BS of the ENC; 256 = BYLAYER, 0 = BYBLOCK).
    pub color: u16,
    pub linetype_scale: f64,
    /// 0 BYLAYER, 1 BYBLOCK, 2 CONTINUOUS, 3 handle.
    pub linetype_flags: u8,
    pub plotstyle_flags: u8,
    pub material_flags: u8,
    pub shadow_flags: u8,
    pub invisible: bool,
    pub lineweight: u8,
    /// Bit offset just after the common data (the class's own data).
    pub end_bit: usize,
    /// Bit offset of the invisibility BS (for in-place edits).
    pub invisibility_bit: usize,
}

fn read_bs(d: &[u8], p: usize) -> Option<(u16, usize)> {
    match (raw::get_bit(d, p), raw::get_bit(d, p + 1)) {
        (0, 0) => Some((
            u16::from(raw::get_byte(d, p + 2)) | u16::from(raw::get_byte(d, p + 10)) << 8,
            p + 18,
        )),
        (0, 1) => Some((u16::from(raw::get_byte(d, p + 2)), p + 10)),
        (1, 0) => Some((0, p + 2)),
        _ => Some((256, p + 2)),
    }
}

fn bb(d: &[u8], p: usize) -> u8 {
    raw::get_bit(d, p) << 1 | raw::get_bit(d, p + 1)
}

/// Read the common entity data of an AECC entity record. `None` if the
/// record has extended entity data (not seen in the samples) or the fields
/// don't parse.
pub fn entity_common(data: &[u8]) -> Option<EntityCommon> {
    let eed = eed_bit(data)?;
    if (raw::get_bit(data, eed), raw::get_bit(data, eed + 1)) != (1, 0) {
        return None;
    }
    let start = if raw::get_bit(data, eed + 2) == 1 {
        let (size, gstart) = read_bll(data, eed + 3);
        gstart + 8 * size as usize
    } else {
        eed + 3
    };
    if start + 37 > raw::bit_len(data) {
        return None;
    }
    let mut p = start;
    let mode = bb(data, p);
    p += 2;
    let (reactors, q) = raw::read_bl(data, p)?;
    p = q;
    let has_xdictionary = raw::get_bit(data, p) == 0;
    let has_ds_data = raw::get_bit(data, p + 1) == 1;
    p += 2;
    let (color, q) = read_bs(data, p)?;
    p = q;
    let (linetype_scale, q) = raw::read_bd(data, p)?;
    p = q;
    let linetype_flags = bb(data, p);
    let plotstyle_flags = bb(data, p + 2);
    let material_flags = bb(data, p + 4);
    p += 6;
    let shadow_flags = raw::get_byte(data, p);
    p += 8 + 3;
    let invisibility_bit = p;
    let (invis, q) = read_bs(data, p)?;
    p = q;
    let lineweight = raw::get_byte(data, p);
    p += 8;
    Some(EntityCommon {
        start_bit: start,
        mode,
        reactors,
        has_xdictionary,
        has_ds_data,
        color: color & 0x1FF,
        linetype_scale,
        linetype_flags,
        plotstyle_flags,
        material_flags,
        shadow_flags,
        invisible: invis & 1 == 1,
        lineweight,
        end_bit: p,
        invisibility_bit,
    })
}
