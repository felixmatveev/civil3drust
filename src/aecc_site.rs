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
//!   samples): data `BL 0, BL n, n × BL key` (raw), 13 constant bits;
//!   strings `""` + the names of the sites that store one; handle stream:
//!   tree node, the `n` sites (soft pointers, in key order), one more
//!   soft pointer (the site created last, or null), then the
//!   label-collector styles (docs/site-object.md, "Site list").
//! - **Names.** A key with the high bit (`0x8000_0000 | id`) belongs to a
//!   name stored as a string in the site record and the list. A key
//!   without it is the ID of one of Civil 3D's built-in names (a property
//!   name such as `A`): the record then has no name string, and the text
//!   is not in the drawing (MIST22). [`pooled_name`] knows the IDs our
//!   samples have shown.
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
    /// The name: the stored string, or for a pooled name the text of
    /// [`pooled_name`] (empty when the ID is not known).
    pub name: String,
    /// The site's key in the sites collection.
    pub name_key: u32,
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

/// Bits between the collection's keys and its strings.
const LIST_TAIL_BITS: usize = 13;

/// True for the key of a name stored as a string; false for the ID of a
/// built-in (pooled) name.
pub fn key_has_string(key: u32) -> bool {
    key & 0x8000_0000 != 0
}

/// Text of a built-in name ID, as far as our samples show it. Civil 3D
/// stores a site name it already has among its built-in names as that
/// name's ID alone (S321: the site the API calls `A` has key
/// `0x0200D214`, the ID the `R/3<A<R` design check uses for the spiral
/// value A, and no name string).
pub fn pooled_name(key: u32) -> Option<&'static str> {
    match key {
        0x0200_D214 => Some("A"),
        _ => None,
    }
}

/// The collection's keys, in its order: `BL 0, BL n, n × raw BL` ending
/// 13 bits before the strings.
pub fn site_keys(node: &RawRecord) -> Option<Vec<u32>> {
    let d = &node.data;
    let ss = string_stream::parse(d, node.handle_bits).ok()?;
    let end = ss.start.checked_sub(LIST_TAIL_BITS)?;
    (0..=255usize).find_map(|n| {
        let count_bits = if n == 0 { 2 } else { 10 };
        let at = end.checked_sub(34 * n + count_bits + 2)?;
        let (zero, p) = raw::read_bl(d, at)?;
        let (count, mut p) = raw::read_bl(d, p)?;
        if zero != 0 || count != n as i32 {
            return None;
        }
        let mut keys = Vec::with_capacity(n);
        for _ in 0..n {
            if raw::get_bit(d, p) != 0 || raw::get_bit(d, p + 1) != 0 {
                return None;
            }
            let (k, q) = raw::read_bl(d, p)?;
            keys.push(k as u32);
            p = q;
        }
        Some(keys)
    })
}

/// Handles and keys of the sites listed by the sites collection, in its
/// order.
pub fn site_entries(doc: &CadDocument) -> Vec<(u64, u32)> {
    let Some(node) = raw::records(doc, SITES_CLASS).into_iter().next() else {
        return Vec::new();
    };
    let Some(keys) = site_keys(&node) else {
        return Vec::new();
    };
    handle_refs::of_record(&node)
        .iter()
        .filter(|r| r.code == CODE_SOFT_PTR)
        .skip(1) // the tree node
        .map(|r| r.handle)
        .zip(keys)
        .collect()
}

/// Handles of the sites listed by the sites collection, in its order.
pub fn site_handles(doc: &CadDocument) -> Vec<u64> {
    site_entries(doc).into_iter().map(|e| e.0).collect()
}

/// The drawing's sites, in the collection's order.
pub fn sites(doc: &CadDocument) -> Vec<Site> {
    let faces = raw::records(doc, SITE_CLASS);
    site_entries(doc)
        .into_iter()
        .filter_map(|(h, key)| {
            let rec = faces.iter().find(|r| r.handle == h)?;
            let s = strings(rec);
            if s.first().map(String::as_str) != Some("AeccImpFace") {
                return None;
            }
            let container = handle_refs::of_record(rec)
                .iter()
                .find(|r| r.code == CODE_HARD_OWNER)
                .map(|r| r.handle);
            // Strings: AeccImpFace, "", [name], description, user. The
            // name is absent when it is a pooled one.
            let (name, description) = if s.len() >= 5 {
                (s[2].clone(), s[3].clone())
            } else {
                (
                    pooled_name(key).unwrap_or_default().to_string(),
                    s.get(2).cloned().unwrap_or_default(),
                )
            };
            Some(Site {
                handle: h,
                name,
                name_key: key,
                description,
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

    #[test]
    fn keys_and_pooled_names() {
        assert!(key_has_string(0x8000_0AFB));
        assert!(!key_has_string(0x0200_D214));
        assert_eq!(pooled_name(0x0200_D214), Some("A"));
        assert_eq!(pooled_name(0x0000_0101), None);
    }
}
