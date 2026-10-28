//! Sites and which site an alignment belongs to (O7: S101–S103;
//! docs/alignment-object.md, "Sites").
//!
//! Layout found so far (read-only):
//!
//! - A **site** is a record of class `AECC_PARCEL` (C++ `AeccDbFace`; its
//!   first string is `AeccImpFace`), strings: `AeccImpFace`, `""`, name,
//!   description, created-by. It hard-owns one **container** record
//!   (class `AECC_FEATURE_STYLE`, first reference `own−1` back to the
//!   site).
//! - The sites collection is the `AECC_PARCEL_NODE` record (BBC in the
//!   samples): data `BL 0, BL n, n × RL key` (keys with the high bit set,
//!   like the alignment registry), strings `""` + the site names, handle
//!   stream: tree node, the `n` sites (soft pointers, in the order of the
//!   names), one more soft pointer to a site (the last one created in the
//!   samples), then three label-collector styles.
//! - A **siteless** alignment's two companion references (hard pointers 2
//!   and 3 of its handle stream) point to its companion, owned by the
//!   alignment registry (BC0, owned by `AECC_ALIGNMENT_PARCEL_NODE`).
//! - An alignment **in a site** points both references at the site's
//!   container instead; the container then holds the companion's body
//!   (the geometry points) and soft pointers to the alignment and the
//!   site. The old companion stays in the registry, emptied to its
//!   117-byte form (ID string and owner reference only), as the companions
//!   of deleted alignments do (S101).

use crate::handle_refs::{self, HandleRef};
use crate::raw::{self, RawRecord};
use crate::string_stream;
use acadrust::document::CadDocument;

/// Class of site records (and, presumably, parcels: both are faces).
pub const SITE_CLASS: &str = "AECC_PARCEL";
/// Class of the sites collection.
pub const SITES_CLASS: &str = "AECC_PARCEL_NODE";
/// Class of site containers and alignment companions.
pub const CONTAINER_CLASS: &str = "AECC_FEATURE_STYLE";

const CODE_HARD_OWNER: u8 = 3;
const CODE_SOFT_PTR: u8 = 4;
const CODE_HARD_PTR: u8 = 5;

/// A site of the drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub handle: u64,
    pub name: String,
    pub description: String,
    /// The container record the site owns (holds its alignments).
    pub container: Option<u64>,
}

/// Where an alignment lives.
#[derive(Debug, Clone, PartialEq)]
pub enum Membership {
    /// Siteless: the companion record (owned by the alignment registry).
    Siteless { companion: u64 },
    /// In a site: the site and its container.
    InSite { site: u64, container: u64 },
    /// The references don't point to a record of a known form.
    Unknown,
}

fn strings(rec: &RawRecord) -> Vec<String> {
    string_stream::parse(&rec.data, rec.handle_bits)
        .map(|s| s.strings.into_iter().map(|x| x.text).collect())
        .unwrap_or_default()
}

/// Handles of the sites listed by the sites collection, in its order.
pub fn site_handles(doc: &CadDocument) -> Vec<u64> {
    let Some(node) = raw::records(doc, SITES_CLASS).into_iter().next() else {
        return Vec::new();
    };
    let n = strings(&node).len().saturating_sub(1);
    handle_refs::of_record(&node)
        .iter()
        .filter(|r| r.code == CODE_SOFT_PTR)
        .skip(1) // the tree node
        .take(n)
        .map(|r| r.handle)
        .collect()
}

/// The drawing's sites, in the collection's order.
pub fn sites(doc: &CadDocument) -> Vec<Site> {
    let faces = raw::records(doc, SITE_CLASS);
    site_handles(doc)
        .into_iter()
        .filter_map(|h| {
            let rec = faces.iter().find(|r| r.handle == h)?;
            let s = strings(rec);
            if s.first().map(String::as_str) != Some("AeccImpFace") {
                return None;
            }
            let container = handle_refs::of_record(rec)
                .iter()
                .find(|r| r.code == CODE_HARD_OWNER)
                .map(|r| r.handle);
            Some(Site {
                handle: h,
                name: s.get(2).cloned().unwrap_or_default(),
                description: s.get(3).cloned().unwrap_or_default(),
                container,
            })
        })
        .collect()
}

/// The two companion references of an alignment record: references 2 and
/// 3 of its handle stream (layer, null, companion, companion, null, style,
/// null), hard pointers.
pub fn companion_refs(alignment: &RawRecord) -> Vec<HandleRef> {
    handle_refs::of_record(alignment)
        .into_iter()
        .skip(2)
        .take(2)
        .filter(|r| r.code == CODE_HARD_PTR)
        .collect()
}

/// Which site (if any) the alignment record belongs to.
pub fn membership(doc: &CadDocument, alignment: &RawRecord) -> Membership {
    let containers = raw::records(doc, CONTAINER_CLASS);
    let target = companion_refs(alignment)
        .iter()
        .map(|r| r.handle)
        .find(|h| containers.iter().any(|c| c.handle == *h));
    let Some(target) = target else {
        return Membership::Unknown;
    };
    if let Some(site) = sites(doc).into_iter().find(|s| s.container == Some(target)) {
        return Membership::InSite {
            site: site.handle,
            container: target,
        };
    }
    Membership::Siteless { companion: target }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_constants() {
        assert_eq!(SITE_CLASS, "AECC_PARCEL");
        assert_eq!(SITES_CLASS, "AECC_PARCEL_NODE");
    }
}
