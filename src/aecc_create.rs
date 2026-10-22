//! Creating an alignment by transplanting Civil 3D's own records.
//!
//! Creating an alignment in Civil 3D adds (S24 → S25, docs/alignment-object.md
//! "Creating an alignment"):
//!
//! - the `AECC_ALIGNMENT` entity,
//! - its companion `AECC_FEATURE_STYLE` object, which carries a 32-hex-digit
//!   ID string,
//! - an `AECC_ALIGNMENT_STAEQU_LABEL_GROUP` entity pointing at the alignment,
//! - an entry in the companion registry (`AECC_FEATURE_STYLE`, BC0 in
//!   S24/S25): a soft-owner reference and a copy of the ID string.
//!
//! The registry's entry layout isn't decoded yet, so [`create_alignment`]
//! takes a **donor** drawing that is the target plus one alignment created
//! in Civil 3D (S25 for target S24). It copies the donor's four records
//! into the target and makes them a new alignment:
//!
//! - fresh handles (companion, alignment, label group, allocated in that
//!   order, as Civil 3D did), patched in each record's own-handle field
//!   (bit 26) and in every reference between them;
//! - a new ID string in the companion and the registry;
//! - the requested name (any length, via [`string_stream::set_name`]) and
//!   PIs (same count as the donor's, via [`set_pis`]).
//!
//! The label group and the parcel entry (companion + registry entry) can
//! be left out ([`CreateOptions`]) to test whether Civil 3D requires them;
//! their handles are still allocated, so all variants use the same
//! handles. Without the parcel entry the alignment's two companion
//! references become null references, which shrinks its handle stream
//! (`handle_refs::null_refs`).
//!
//! Only the alignment record can change size (name). The style, layer and
//! label style are the donor's and must exist in the target.

use crate::aecc_alignment_edit::{set_pis, EditError};
use crate::aecc_alignment_refs::alignment_refs;
use crate::aecc_elements::decode_elements;
use crate::alignment::Point2;
use crate::handle_refs::{self, HandleRef};
use crate::raw::{self, RawRecord};
use crate::string_stream;
use acadrust::document::CadDocument;
use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
use acadrust::types::Handle;
use std::fmt;

const COMPANION: &str = "AECC_FEATURE_STYLE";
const ALIGNMENT: &str = "AECC_ALIGNMENT";
const LABEL_GROUP: &str = "AECC_ALIGNMENT_STAEQU_LABEL_GROUP";
/// Bit offset of the own-handle reference (`0x02` + two handle bytes) in
/// every record studied.
const OWN_HANDLE_REF: usize = 18;

#[derive(Debug, Clone, PartialEq)]
pub enum CreateError {
    /// The donor/target pair doesn't have the expected shape.
    Donor(String),
    /// The target lacks an object the donor's records point at.
    MissingInTarget(&'static str, u64),
    /// The new name must have the donor name's length (for now).
    NameLength {
        want: usize,
        got: usize,
    },
    /// A new handle or offset doesn't fit the stored byte count.
    HandleSize(u64),
    Geometry(EditError),
}

impl fmt::Display for CreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Donor(why) => write!(f, "donor drawing: {why}"),
            Self::MissingInTarget(what, h) => write!(f, "target has no {what} {h:X}"),
            Self::NameLength { .. } => write!(f, "name must not be empty"),
            Self::HandleSize(h) => write!(f, "handle {h:X} doesn't fit the stored byte count"),
            Self::Geometry(e) => write!(f, "geometry: {e}"),
        }
    }
}

impl std::error::Error for CreateError {}

/// Which optional records [`create_alignment`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateOptions {
    /// The station-equation label group (optional: S27 → S29).
    pub label_group: bool,
    /// The parcel-topology entry: companion object plus registry entry. When
    /// false, the alignment's two companion references are nulled.
    pub parcel_entry: bool,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            label_group: true,
            parcel_entry: true,
        }
    }
}

/// What [`create_alignment`] added.
#[derive(Debug, Clone, PartialEq)]
pub struct Created {
    pub alignment: u64,
    /// 0 when created without the parcel entry.
    pub companion: u64,
    /// 0 when created without a label group.
    pub label_group: u64,
    pub registry: u64,
    pub id_string: String,
    pub name: String,
}

fn donor_err(msg: impl Into<String>) -> CreateError {
    CreateError::Donor(msg.into())
}

fn record(doc: &CadDocument, class: &str, handle: u64) -> Option<RawRecord> {
    raw::records(doc, class)
        .into_iter()
        .find(|r| r.handle == handle)
}

/// Soft-owner (code 2) entries of a registry record.
fn owned(rec: &RawRecord) -> Vec<u64> {
    handle_refs::of_record(rec)
        .iter()
        .filter(|r| r.code == 2)
        .map(|r| r.handle)
        .collect()
}

fn registry(doc: &CadDocument) -> Option<RawRecord> {
    raw::records(doc, COMPANION)
        .into_iter()
        .find(|r| !owned(r).is_empty())
}

fn patch_own_handle(data: &mut [u8], old: u64, new: u64) -> Result<(), CreateError> {
    let stored = ((raw::get_byte(data, OWN_HANDLE_REF + 8) as u64) << 8)
        | raw::get_byte(data, OWN_HANDLE_REF + 16) as u64;
    if raw::get_byte(data, OWN_HANDLE_REF) != 0x02 || stored != old {
        return Err(donor_err(format!(
            "record {old:X}: own handle not at the expected position"
        )));
    }
    if !(0x100..=0xFFFF).contains(&new) {
        return Err(CreateError::HandleSize(new));
    }
    raw::set_byte(data, OWN_HANDLE_REF + 8, (new >> 8) as u8);
    raw::set_byte(data, OWN_HANDLE_REF + 16, new as u8);
    Ok(())
}

/// Point every reference in `refs` that targets `from` at `to`.
fn retarget_all(
    data: &mut [u8],
    refs: &[HandleRef],
    own: u64,
    from: u64,
    to: u64,
) -> Result<usize, CreateError> {
    let mut n = 0;
    for r in refs.iter().filter(|r| r.handle == from) {
        if !handle_refs::retarget_any(data, r, own, to) {
            return Err(CreateError::HandleSize(to));
        }
        n += 1;
    }
    Ok(n)
}

/// Overwrite every occurrence of the UTF-16 string `old` with `new` (same
/// length). Returns the number of occurrences.
fn replace_string(data: &mut [u8], old: &str, new: &str) -> usize {
    debug_assert_eq!(old.len(), new.len());
    let hits: Vec<usize> = raw::utf16_strings(data, old.len())
        .into_iter()
        .filter(|(_, s)| s == old)
        .map(|(b, _)| b)
        .collect();
    for &b in &hits {
        for (i, c) in new.bytes().enumerate() {
            raw::set_byte(data, b + 16 * i, c);
            raw::set_byte(data, b + 16 * i + 8, 0);
        }
    }
    hits.len()
}

/// A new 32-hex-digit ID string, distinct from `avoid`.
pub fn new_id_string(seed: u64, avoid: &[String]) -> String {
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    loop {
        let mut s = String::new();
        for _ in 0..2 {
            // splitmix64
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            s.push_str(&format!("{z:016x}"));
        }
        if !avoid.contains(&s) {
            return s;
        }
    }
}

fn id_string_of(rec: &RawRecord) -> Option<String> {
    raw::utf16_strings(&rec.data, 32)
        .into_iter()
        .map(|(_, s)| s)
        .find(|s| s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Add a new alignment to `target`, built from the one alignment `donor`
/// has that `target` lacks. `seed` feeds the ID string.
pub fn create_alignment(
    target: &mut CadDocument,
    donor: &CadDocument,
    name: &str,
    pis: &[Point2],
    seed: u64,
    opts: CreateOptions,
) -> Result<Created, CreateError> {
    let with_label_group = opts.label_group;
    // --- Identify the donor's new records. ---
    let d_reg = registry(donor).ok_or_else(|| donor_err("no companion registry"))?;
    let t_reg = record(target, COMPANION, d_reg.handle).ok_or(CreateError::MissingInTarget(
        "companion registry",
        d_reg.handle,
    ))?;
    let (d_owned, t_owned) = (owned(&d_reg), owned(&t_reg));
    let extra: Vec<u64> = d_owned
        .iter()
        .copied()
        .filter(|h| !t_owned.contains(h))
        .collect();
    if extra.len() != 1 || d_owned.len() != t_owned.len() + 1 {
        return Err(donor_err(format!(
            "registry must list exactly one more companion than the target's (donor {d_owned:X?}, target {t_owned:X?})"
        )));
    }
    let d_comp_h = extra[0];
    let d_comp = record(donor, COMPANION, d_comp_h)
        .ok_or_else(|| donor_err(format!("companion {d_comp_h:X} missing")))?;
    let d_align = raw::records(donor, ALIGNMENT)
        .into_iter()
        .find(|r| alignment_refs(r).is_some_and(|x| x.companion.handle == d_comp_h))
        .ok_or_else(|| donor_err("no alignment uses the new companion"))?;
    let d_label = raw::records(donor, LABEL_GROUP)
        .into_iter()
        .find(|r| {
            handle_refs::of_record(r)
                .iter()
                .any(|x| x.code == 4 && x.handle == d_align.handle)
        })
        .ok_or_else(|| donor_err("no station-equation label group for the alignment"))?;
    let refs = alignment_refs(&d_align).unwrap();

    // --- Everything the copies point at must exist in the target. ---
    if raw::class_of(target, refs.style.handle).as_deref() != Some("AECC_ALIGNMENT_STYLE") {
        return Err(CreateError::MissingInTarget(
            "alignment style",
            refs.style.handle,
        ));
    }
    if !target
        .layers
        .iter()
        .any(|l| l.handle.value() == refs.layer.handle)
    {
        return Err(CreateError::MissingInTarget("layer", refs.layer.handle));
    }
    let new_set = [d_comp_h, d_align.handle, d_label.handle];
    for r in handle_refs::of_record(&d_label)
        .iter()
        .chain(handle_refs::of_record(&d_comp).iter())
    {
        let known = r.handle == 0
            || new_set.contains(&r.handle)
            || r.handle == d_reg.handle
            || target.layers.iter().any(|l| l.handle.value() == r.handle)
            || raw::class_of(target, r.handle).is_some();
        if !known {
            return Err(CreateError::MissingInTarget("object", r.handle));
        }
    }

    // --- Name and ID string. ---
    let old_name =
        raw::object_name(&d_align.data).ok_or_else(|| donor_err("alignment has no name"))?;
    if name.is_empty() {
        return Err(CreateError::NameLength { want: 1, got: 0 });
    }
    let old_id = id_string_of(&d_comp).ok_or_else(|| donor_err("companion has no ID string"))?;
    let existing_ids: Vec<String> = raw::records(target, COMPANION)
        .iter()
        .filter_map(id_string_of)
        .chain([old_id.clone()])
        .collect();
    let new_id = new_id_string(seed, &existing_ids);

    // --- New handles (Civil 3D order: companion, alignment, label). ---
    let comp_h = target.allocate_handle().value();
    let align_h = target.allocate_handle().value();
    let label_h = target.allocate_handle().value();

    // Companion.
    let mut comp = d_comp.data.clone();
    patch_own_handle(&mut comp, d_comp_h, comp_h)?;
    let crefs = handle_refs::of_record(&d_comp);
    retarget_all(&mut comp, &crefs, comp_h, d_reg.handle, d_reg.handle)?;
    retarget_all(&mut comp, &crefs, comp_h, d_align.handle, align_h)?;
    if replace_string(&mut comp, &old_id, &new_id) != 1 {
        return Err(donor_err("companion ID string not found once"));
    }

    // Alignment.
    let mut align = d_align.data.clone();
    patch_own_handle(&mut align, d_align.handle, align_h)?;
    let arefs = handle_refs::of_record(&d_align);
    if retarget_all(&mut align, &arefs, align_h, d_comp_h, comp_h)? != 2 {
        return Err(donor_err("alignment doesn't reference its companion twice"));
    }
    let rec = RawRecord {
        handle: align_h,
        class: ALIGNMENT.into(),
        is_entity: true,
        data: align.clone(),
        handle_bits: d_align.handle_bits,
    };
    let decoded = decode_elements(&rec).map_err(|e| donor_err(e.to_string()))?;
    set_pis(&mut align, &decoded, pis).map_err(CreateError::Geometry)?;
    // Name last among data edits: it can change the record size.
    let (bytes, mut align_hb) =
        string_stream::set_name(&align, d_align.handle_bits, &old_name, name)
            .map_err(|e| donor_err(e.to_string()))?;
    align = bytes;
    if !opts.parcel_entry {
        let idx: Vec<usize> = handle_refs::of_record(&d_align)
            .iter()
            .enumerate()
            .filter(|(_, r)| r.handle == d_comp_h)
            .map(|(i, _)| i)
            .collect();
        let (bytes, hb) = handle_refs::null_refs(&align, align_hb, align_h, &idx);
        align = bytes;
        align_hb = hb;
    }

    // Label group.
    let mut label = d_label.data.clone();
    patch_own_handle(&mut label, d_label.handle, label_h)?;
    let lrefs = handle_refs::of_record(&d_label);
    if retarget_all(&mut label, &lrefs, label_h, d_align.handle, align_h)? != 1 {
        return Err(donor_err(
            "label group doesn't reference the alignment once",
        ));
    }

    // Registry: the donor's version, with the new companion and ID string.
    let mut reg = d_reg.data.clone();
    let rrefs = handle_refs::of_record(&d_reg);
    if retarget_all(&mut reg, &rrefs, d_reg.handle, d_comp_h, comp_h)? != 1 {
        return Err(donor_err("registry doesn't own the companion once"));
    }
    if replace_string(&mut reg, &old_id, &new_id) != 1 {
        return Err(donor_err("registry ID string not found once"));
    }

    // --- Insert into the target. ---
    if opts.parcel_entry {
        let d_obj = donor
            .objects
            .iter()
            .find(|(h, _)| h.value() == d_comp_h)
            .map(|(_, o)| o.clone())
            .unwrap();
        let ObjectType::Unknown {
            type_name,
            raw_dxf_codes,
            raw_dwg_version,
            ..
        } = d_obj
        else {
            return Err(donor_err("companion is not a raw object"));
        };
        target.objects.insert(
            Handle::new(comp_h),
            ObjectType::Unknown {
                type_name,
                handle: Handle::new(comp_h),
                owner: Handle::new(d_reg.handle),
                raw_dxf_codes,
                raw_dwg_data: Some(comp),
                raw_dwg_handle_bits: d_comp.handle_bits as i64,
                raw_dwg_version,
            },
        );
        if let Some(ObjectType::Unknown {
            raw_dwg_data,
            raw_dwg_handle_bits,
            ..
        }) = target
            .objects
            .iter_mut()
            .find(|(h, _)| h.value() == d_reg.handle)
            .map(|(_, o)| o)
        {
            *raw_dwg_data = Some(reg);
            *raw_dwg_handle_bits = d_reg.handle_bits as i64;
        }
    }

    let mut entities = vec![(d_align.handle, align_h, align, align_hb)];
    if with_label_group {
        entities.push((d_label.handle, label_h, label, d_label.handle_bits));
    }
    for (src, new_h, data, hb) in entities {
        let mut e = donor
            .entities()
            .find(|e| e.common().handle.value() == src)
            .cloned()
            .ok_or_else(|| donor_err(format!("entity {src:X} missing")))?;
        if let EntityType::Unknown(u) = &mut e {
            u.common.handle = Handle::new(new_h);
            u.common.owner_handle = Handle::NULL;
            u.raw_dwg_data = Some(data);
            u.dwg_handle_bits = hb as i64;
        }
        target.add_entity(e).map_err(|e| donor_err(e.to_string()))?;
    }

    Ok(Created {
        alignment: align_h,
        companion: if opts.parcel_entry { comp_h } else { 0 },
        label_group: if with_label_group { label_h } else { 0 },
        registry: d_reg.handle,
        id_string: new_id,
        name: name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_strings_are_hex_and_distinct() {
        let a = new_id_string(1, &[]);
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        let b = new_id_string(1, std::slice::from_ref(&a));
        assert_ne!(a, b);
    }

    #[test]
    fn own_handle_patch() {
        let mut d = vec![0u8; 8];
        raw::set_byte(&mut d, 18, 0x02);
        raw::set_byte(&mut d, 26, 0x0F);
        raw::set_byte(&mut d, 34, 0xB8);
        patch_own_handle(&mut d, 0xFB8, 0x1234).unwrap();
        assert_eq!(raw::get_byte(&d, 26), 0x12);
        assert_eq!(raw::get_byte(&d, 34), 0x34);
        assert!(patch_own_handle(&mut d, 0xFB8, 0x1235).is_err());
        assert_eq!(
            patch_own_handle(&mut d, 0x1234, 0x12345),
            Err(CreateError::HandleSize(0x12345))
        );
    }
}
