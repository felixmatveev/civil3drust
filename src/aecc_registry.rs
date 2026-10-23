//! The alignment companion registry ("companion list").
//!
//! Every alignment has a companion object (`AECC_FEATURE_STYLE`, its
//! parcel-topology entry) and the drawing has one registry object of the
//! same class, owned by the `AECC_ALIGNMENT_PARCEL_NODE`, listing every
//! companion (docs/alignment-object.md, "The companion registry"; S24, S25,
//! S44, S48 with 1–4 entries).
//!
//! Per entry the registry stores, always in the same order:
//!
//! - in the data stream, after a BL 0 and a BL **count**: one bit-coded
//!   long per entry, a **key** `0x8000_0000 | n` (always the full 32-bit
//!   form) — the same value is stored in the companion;
//! - in the string stream, after one empty string: the companion's
//!   32-hex-digit **ID string**;
//! - in the handle stream, after the `own−1` reference to the parcel node:
//!   a **soft-owner** reference to the companion.
//!
//! Entries are sorted by ID string (S48: `07ce…`, `37d6…`, `475c…`,
//! `b4c9…`). [`Registry::with_entry`] rebuilds the record with an entry
//! added; [`Registry::without_entry`] with one removed.

use crate::aecc_build::Bits;
use crate::handle_refs;
use crate::raw::{self, RawRecord};
use crate::string_stream;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    Layout(String),
    Duplicate(String),
    NotFound(u64),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layout(w) => write!(f, "companion registry layout: {w}"),
            Self::Duplicate(w) => write!(f, "companion registry already has {w}"),
            Self::NotFound(h) => write!(f, "companion {h:X} not in the registry"),
        }
    }
}

impl std::error::Error for RegistryError {}

fn layout(m: impl Into<String>) -> RegistryError {
    RegistryError::Layout(m.into())
}

/// One registry entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: u32,
    pub companion: u64,
    pub id: String,
}

/// Bits between the key list and the string stream, the same in every
/// registry in the samples (S24–S66, 0–7 entries).
pub const TAIL_BITS: usize = 249;

/// A parsed registry record.
#[derive(Debug, Clone, PartialEq)]
pub struct Registry {
    pub handle: u64,
    pub entries: Vec<Entry>,
    /// Bit offset of the BL 0 before the count.
    pub list_bit: usize,
    /// Bit offset just after the last key.
    pub list_end: usize,
}

/// Find `BL 0, BL n, n × (00 + RL with the high bit set)` in the data
/// stream before `limit`.
fn find_list(data: &[u8], n: usize, limit: usize) -> Option<(usize, usize, Vec<u32>)> {
    'start: for b in 0..limit.saturating_sub(12) {
        if raw::get_bit(data, b) != 1 || raw::get_bit(data, b + 1) != 0 {
            continue;
        }
        let Some((count, mut p)) = raw::read_bl(data, b + 2) else {
            continue;
        };
        if count as usize != n || raw::get_bit(data, b + 2) != 0 || raw::get_bit(data, b + 3) != 1 {
            continue;
        }
        let mut keys = Vec::with_capacity(n);
        for _ in 0..n {
            if raw::get_bit(data, p) != 0 || raw::get_bit(data, p + 1) != 0 {
                continue 'start;
            }
            let v = raw::get_i32(data, p + 2) as u32;
            if v & 0x8000_0000 == 0 {
                continue 'start;
            }
            keys.push(v);
            p += 34;
        }
        if p <= limit {
            return Some((b, p, keys));
        }
    }
    None
}

impl Registry {
    /// Parse `rec` as a registry (a record whose handle stream has
    /// soft-owner references).
    pub fn parse(rec: &RawRecord) -> Result<Registry, RegistryError> {
        let refs = handle_refs::of_record(rec);
        let owned: Vec<u64> = refs
            .iter()
            .filter(|r| r.code == 2)
            .map(|r| r.handle)
            .collect();
        let ss =
            string_stream::parse(&rec.data, rec.handle_bits).map_err(|e| layout(e.to_string()))?;
        if owned.is_empty() {
            // Empty registry (a Civil 3D drawing without alignments, S65):
            // the list is `10 10` (BL 0, count 0), followed by the same
            // 249-bit tail as every registry in S24–S63.
            let only_empty = ss.strings.len() == 1 && ss.strings[0].text.is_empty();
            let list_end = ss
                .start
                .checked_sub(TAIL_BITS)
                .ok_or_else(|| layout("too short"))?;
            let list_bit = list_end.checked_sub(4).ok_or_else(|| layout("too short"))?;
            let bits: Vec<u8> = (list_bit..list_end)
                .map(|b| raw::get_bit(&rec.data, b))
                .collect();
            if !only_empty || bits != [1, 0, 1, 0] {
                return Err(layout("no soft-owner references"));
            }
            return Ok(Registry {
                handle: rec.handle,
                entries: Vec::new(),
                list_bit,
                list_end,
            });
        }
        let ids: Vec<String> = ss.strings.iter().skip(1).map(|s| s.text.clone()).collect();
        if ss.strings.first().map(|s| s.text.as_str()) != Some("") || ids.len() != owned.len() {
            return Err(layout(format!(
                "{} ID strings for {} entries",
                ids.len(),
                owned.len()
            )));
        }
        let (list_bit, list_end, keys) = find_list(&rec.data, owned.len(), ss.start)
            .ok_or_else(|| layout("key list not found"))?;
        let entries = keys
            .into_iter()
            .zip(owned)
            .zip(ids)
            .map(|((key, companion), id)| Entry { key, companion, id })
            .collect();
        Ok(Registry {
            handle: rec.handle,
            entries,
            list_bit,
            list_end,
        })
    }

    /// A key not used by any entry: the largest key + 1.
    pub fn next_key(&self) -> u32 {
        self.entries
            .iter()
            .map(|e| e.key)
            .max()
            .map(|k| k + 1)
            .unwrap_or(0x8000_0001)
    }

    /// Rebuild `rec` with `entries` (sorted here by ID string).
    fn rebuild(
        &self,
        rec: &RawRecord,
        mut entries: Vec<Entry>,
    ) -> Result<(Vec<u8>, usize), RegistryError> {
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        let data = &rec.data;
        let ss = string_stream::parse(data, rec.handle_bits).map_err(|e| layout(e.to_string()))?;
        let total = raw::bit_len(data);
        let refs = handle_refs::of_record(rec);
        let mut w = Bits::default();
        // Data: header, BL 0, count, keys, rest up to the strings.
        w.copy(data, 0, self.list_bit);
        w.bl(0);
        w.bl(entries.len() as i32);
        for e in &entries {
            w.bits(0b00, 2);
            w.rl(e.key as i32);
        }
        w.copy(data, self.list_end, ss.start);
        // Strings: "", then the IDs.
        let start = w.len();
        w.bits(0b10, 2);
        for e in &entries {
            let units: Vec<u16> = e.id.encode_utf16().collect();
            w.bl_short(units.len());
            for u in units {
                w.byte((u & 0xFF) as u8);
                w.byte((u >> 8) as u8);
            }
        }
        let size = w.len() - start;
        if size & 0x8000 != 0 {
            return Err(layout("string stream too large"));
        }
        w.byte((size & 0xFF) as u8);
        w.byte((size >> 8) as u8);
        w.bit(1);
        let data_end = w.len();
        // Handles: non-owner references before the owners, then owners in
        // entry order, then any references after them.
        // An empty registry has no owners yet: they go after its references.
        let first_owner = refs.iter().position(|r| r.code == 2).unwrap_or(refs.len());
        let last_owner = refs
            .iter()
            .rposition(|r| r.code == 2)
            .unwrap_or(refs.len().wrapping_sub(1));
        for r in &refs[..first_owner] {
            w.copy(data, r.bit, r.bit + r.bits());
        }
        for e in &entries {
            encode_abs(&mut w, 2, e.companion);
        }
        for r in &refs[last_owner + 1..] {
            w.copy(data, r.bit, r.bit + r.bits());
        }
        let _ = total;
        let bytes = w.into_bytes();
        let hb = bytes.len() * 8 - data_end;
        Ok((bytes, hb))
    }

    /// The record with `entry` added.
    pub fn with_entry(
        &self,
        rec: &RawRecord,
        entry: Entry,
    ) -> Result<(Vec<u8>, usize), RegistryError> {
        if let Some(e) = self
            .entries
            .iter()
            .find(|e| e.companion == entry.companion || e.key == entry.key || e.id == entry.id)
        {
            return Err(RegistryError::Duplicate(format!(
                "an entry like {:X} / {:X} / {}",
                e.companion, e.key, e.id
            )));
        }
        let mut entries = self.entries.clone();
        entries.push(entry);
        self.rebuild(rec, entries)
    }

    /// The record with the entry for `companion` removed.
    pub fn without_entry(
        &self,
        rec: &RawRecord,
        companion: u64,
    ) -> Result<(Vec<u8>, usize), RegistryError> {
        let entries: Vec<Entry> = self
            .entries
            .iter()
            .filter(|e| e.companion != companion)
            .cloned()
            .collect();
        if entries.len() == self.entries.len() {
            return Err(RegistryError::NotFound(companion));
        }
        if entries.is_empty() {
            return Err(layout("removing the last entry is not supported yet"));
        }
        self.rebuild(rec, entries)
    }
}

/// Absolute handle reference with the minimal byte count.
pub fn encode_abs(w: &mut Bits, code: u8, handle: u64) {
    let n = if handle == 0 {
        0
    } else {
        8 - handle.leading_zeros() as usize / 8
    };
    w.byte((code << 4) | n as u8);
    for i in (0..n).rev() {
        w.byte((handle >> (8 * i)) as u8);
    }
}

/// The key stored in a companion record (the first `00` + RL with the high
/// bit set in the data stream), as (bit offset of the RL, key).
pub fn companion_key(rec: &RawRecord) -> Option<(usize, u32)> {
    let end = raw::bit_len(&rec.data).checked_sub(rec.handle_bits)?;
    (40..end.min(400).saturating_sub(34)).find_map(|b| {
        let v = raw::get_i32(&rec.data, b + 2) as u32;
        (raw::get_bit(&rec.data, b) == 0
            && raw::get_bit(&rec.data, b + 1) == 0
            && v & 0x8000_0000 != 0
            && v & 0x7FFF_0000 == 0)
            .then_some((b + 2, v))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_abs_minimal() {
        let mut w = Bits::default();
        encode_abs(&mut w, 2, 0xFB7);
        encode_abs(&mut w, 5, 0);
        encode_abs(&mut w, 4, 0x10);
        assert_eq!(w.into_bytes(), [0x22, 0x0F, 0xB7, 0x50, 0x41, 0x10]);
    }
}
