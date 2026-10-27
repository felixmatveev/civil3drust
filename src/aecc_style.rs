//! Display components of an `AECC_ALIGNMENT_STYLE` record.
//!
//! A style's data stream holds 17 display-component blocks: 16 of 123 bits
//! (one 125, likely a longer encoding of a value inside it) and a last,
//! longer one (135 bits in S24). Each block contains a run of sixteen 1
//! bits followed by `01`; the component's **visible** flag is the single bit
//! 47 bits before that run (docs/alignment-object.md, "Alignment style:
//! display components"; S21 → S22, where turning the line component on in
//! the owner's `TEST STYLE` changed only this bit and the modified-time
//! stamp).
//!
//! The 17 blocks are the style's three view directions, in the order of
//! Civil 3D's Display tab (owner's screenshots): **Plan** — 0 Line,
//! 1 Curve, 2 Spiral, 3 Arrow, 4 Line Extensions, 5 Curve Extensions,
//! 6 Tangent Extensions, 7 Warning Symbol; **Model** — the same eight
//! (8–15); **Section** — 16 Line Marker in Section (the longer last
//! block). Confirmed by samples: 0 (S21 → S22) and 1 (S97 → S98, curve
//! turned on in Plan); the rest follows the order shown in Civil 3D and fits
//! every sample style (e.g. Basic `11100001 00000000 1`: plan line, curve,
//! spiral and warning symbol, nothing in model, the section marker).
//!
//! Locating blocks by the marker run is a heuristic (D-020): it is checked
//! against every alignment style in S1–S24 (always exactly 17 hits), and
//! [`components`] refuses a record with any other count.

use crate::raw;

const MARKER: [u8; 18] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1];
/// Bits from the visible flag to the marker run.
const FLAG_BEFORE_MARKER: usize = 47;
/// Display components found in every sample style.
pub const COMPONENT_COUNT: usize = 17;

/// View directions of an alignment style's display components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Plan,
    Model,
    Section,
}

/// Component names per view, in stored order (Civil 3D's Display tab).
pub const PLAN_MODEL_COMPONENTS: [&str; 8] = [
    "Line",
    "Curve",
    "Spiral",
    "Arrow",
    "Line Extensions",
    "Curve Extensions",
    "Tangent Extensions",
    "Warning Symbol",
];
/// The section view's only component.
pub const SECTION_COMPONENT: &str = "Line Marker in Section";

/// View and name of component `index` (0..17).
pub fn component_name(index: usize) -> Option<(View, &'static str)> {
    match index {
        0..=7 => Some((View::Plan, PLAN_MODEL_COMPONENTS[index])),
        8..=15 => Some((View::Model, PLAN_MODEL_COMPONENTS[index - 8])),
        16 => Some((View::Section, SECTION_COMPONENT)),
        _ => None,
    }
}

/// Index of a component given as `view:name` (case-insensitive, spaces
/// optional, e.g. `plan:curve`, `model:line`, `section`), or as a plain
/// number.
pub fn component_index(spec: &str) -> Option<usize> {
    if let Ok(i) = spec.parse::<usize>() {
        return (i < COMPONENT_COUNT).then_some(i);
    }
    let norm = |t: &str| t.to_ascii_lowercase().replace([' ', '_', '-'], "");
    let spec = norm(spec);
    if spec == "section" || spec == norm(&format!("section:{SECTION_COMPONENT}")) {
        return Some(16);
    }
    let (view, name) = spec.split_once(':')?;
    let base = match view {
        "plan" => 0,
        "model" => 8,
        _ => return None,
    };
    PLAN_MODEL_COMPONENTS
        .iter()
        .position(|n| norm(n) == name)
        .map(|i| base + i)
}

/// One display component of a style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    pub index: usize,
    /// Bit offset of the visible flag.
    pub flag_bit: usize,
    pub visible: bool,
}

fn marker_hits(data: &[u8]) -> Vec<usize> {
    let n = raw::bit_len(data);
    let mut hits = Vec::new();
    let mut b = FLAG_BEFORE_MARKER;
    while b + MARKER.len() <= n {
        if MARKER
            .iter()
            .enumerate()
            .all(|(i, &m)| raw::get_bit(data, b + i) == m)
        {
            hits.push(b);
            b += MARKER.len();
        } else {
            b += 1;
        }
    }
    hits
}

/// The display components of a style record, or `None` if the record
/// doesn't show exactly [`COMPONENT_COUNT`] blocks.
pub fn components(data: &[u8]) -> Option<Vec<Component>> {
    let hits = marker_hits(data);
    if hits.len() != COMPONENT_COUNT {
        return None;
    }
    Some(
        hits.iter()
            .enumerate()
            .map(|(index, &h)| {
                let flag_bit = h - FLAG_BEFORE_MARKER;
                Component {
                    index,
                    flag_bit,
                    visible: raw::get_bit(data, flag_bit) == 1,
                }
            })
            .collect(),
    )
}

/// Visible flags as a string of `0`/`1`, component 0 first.
pub fn flags_string(data: &[u8]) -> Option<String> {
    components(data).map(|c| {
        c.iter()
            .map(|c| if c.visible { '1' } else { '0' })
            .collect()
    })
}

/// Set component `index`'s visible flag in place. Returns the previous
/// value, or `None` if the components can't be located.
pub fn set_component_visible(data: &mut [u8], index: usize, visible: bool) -> Option<bool> {
    let c = *components(data)?.get(index)?;
    let byte = c.flag_bit / 8;
    let mask = 1u8 << (7 - c.flag_bit % 8);
    if visible {
        data[byte] |= mask;
    } else {
        data[byte] &= !mask;
    }
    Some(c.visible)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic record: 17 blocks of 123 bits, flag 47 bits before the
    /// marker, marker-free filler elsewhere.
    fn record(flags: u32) -> Vec<u8> {
        let mut bits = vec![0u8; 30];
        for i in 0..17 {
            let mut block = vec![0u8; 123];
            block[0] = ((flags >> i) & 1) as u8;
            block[47..65].copy_from_slice(&MARKER);
            bits.extend(block);
        }
        bits.extend([0u8; 40]);
        let mut out = vec![0u8; bits.len().div_ceil(8)];
        for (i, b) in bits.iter().enumerate() {
            out[i / 8] |= b << (7 - i % 8);
        }
        out
    }

    #[test]
    fn reads_and_sets_flags() {
        let mut d = record(0b0000_0000_1000_0111);
        assert_eq!(flags_string(&d).unwrap(), "11100001000000000");
        assert_eq!(set_component_visible(&mut d, 0, false), Some(true));
        assert_eq!(set_component_visible(&mut d, 15, true), Some(false));
        assert_eq!(flags_string(&d).unwrap(), "01100001000000010");
        assert_eq!(set_component_visible(&mut d, 17, true), None);
        assert_eq!(component_index("plan:curve"), Some(1));
        assert_eq!(component_index("Model:Tangent Extensions"), Some(14));
        assert_eq!(component_index("section"), Some(16));
        assert_eq!(component_index("16"), Some(16));
        assert_eq!(component_index("plan:nothing"), None);
        assert_eq!(component_name(9), Some((View::Model, "Curve")));
    }

    #[test]
    fn refuses_unexpected_block_count() {
        let d = record(0)[..100].to_vec();
        assert!(components(&d).is_none());
    }
}
