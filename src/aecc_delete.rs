//! Deleting an alignment, as Civil 3D does it (S228 → S229).
//!
//! Civil 3D removes the alignment entity and the label groups that point
//! at it (station equations, design speeds), and keeps the alignment's
//! companion and its registry entry, the companion emptied: no nodes, no
//! edge, no reference to the alignment. Alignments in a site are refused
//! (their site collection would change too; not sampled yet).

use crate::aecc_site::{membership, Membership};
use crate::handle_refs;
use crate::raw::{self, RawRecord};
use acadrust::document::CadDocument;
use acadrust::Handle;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteError(pub String);

impl fmt::Display for DeleteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "delete alignment: {}", self.0)
    }
}

impl std::error::Error for DeleteError {}

fn err(m: impl Into<String>) -> DeleteError {
    DeleteError(m.into())
}

/// What [`delete_alignment`] removed and changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deleted {
    pub alignment: u64,
    /// Label-group entities removed with it.
    pub label_groups: Vec<u64>,
    /// The companion, kept and emptied (0 when the alignment had none).
    pub companion: u64,
}

/// Delete the siteless alignment `handle` from `doc`.
pub fn delete_alignment(doc: &mut CadDocument, handle: u64) -> Result<Deleted, DeleteError> {
    let rec = raw::records(doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == handle)
        .ok_or_else(|| err(format!("no alignment {handle:X}")))?;
    let companion = match membership(doc, &rec) {
        Membership::Siteless { companion } => companion,
        _ => return Err(err("the alignment is in a site (not supported yet)")),
    };
    // Label groups pointing at the alignment (S229: FA3, FA4).
    let label_groups: Vec<u64> = crate::inventory::aecc_classes(doc)
        .into_iter()
        .filter(|c| c.dxf_name.ends_with("_LABEL_GROUP"))
        .flat_map(|c| raw::records(doc, &c.dxf_name))
        .filter(|r| refers_to(r, handle))
        .map(|r| r.handle)
        .collect();
    if companion != 0 {
        let comp = raw::records(doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == companion)
            .ok_or_else(|| err(format!("companion {companion:X} not found")))?;
        let (data, hb) = empty_companion(&comp)
            .ok_or_else(|| err(format!("companion {companion:X}: form not known")))?;
        if !raw::set_record(doc, companion, data, hb) {
            return Err(err("could not write the companion"));
        }
    }
    for h in label_groups.iter().chain(std::iter::once(&handle)) {
        doc.remove_entity(Handle::new(*h))
            .ok_or_else(|| err(format!("entity {h:X} not removable")))?;
    }
    Ok(Deleted {
        alignment: handle,
        label_groups,
        companion,
    })
}

fn refers_to(r: &RawRecord, target: u64) -> bool {
    let total = raw::bit_len(&r.data);
    handle_refs::parse(&r.data, total - r.handle_bits, total, r.handle)
        .iter()
        .any(|x| x.handle == target)
}

/// The companion as Civil 3D leaves it when its alignment is deleted
/// (S229): the node list keeps its next free ID with no nodes, the edge
/// list and the tail entry after it become `BL 2, 1, 2, 2`, and only the
/// first reference (the owner, the registry) stays. The rest (header, key,
/// the run before the strings, the ID string) is kept. `None` when the
/// companion is not in the known one-edge form.
pub fn empty_companion(rec: &RawRecord) -> Option<(Vec<u8>, usize)> {
    use crate::aecc_build::Bits;
    let data = &rec.data;
    let g = crate::aecc_topology::companion_graph(data)?;
    // After the edge: BL 0 ×4, then the tail entry `2, 1, 1, 1, 7`.
    let mut p = g.edges_end;
    let mut tail = Vec::new();
    for _ in 0..9 {
        let (v, q) = raw::read_bl(data, p)?;
        tail.push(v);
        p = q;
    }
    if tail != [0, 0, 0, 0, 2, 1, 1, 1, 7] {
        return None;
    }
    let (next_free, after_first) = raw::read_bl(data, g.nodes.start_bit)?;
    let total = raw::bit_len(data);
    let data_end = total - rec.handle_bits;
    let mut w = Bits::default();
    w.copy(data, 0, g.nodes.start_bit);
    w.bl(next_free);
    debug_assert!(after_first > g.nodes.start_bit);
    for v in [0, 0, 2, 1, 2, 2] {
        w.bl(v);
    }
    w.copy(data, p, data_end);
    let new_data_end = w.len();
    w.copy(data, data_end, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    handle_refs::keep_refs(&bytes, hb, rec.handle, 1)
}
