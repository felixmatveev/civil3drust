//! Creating an alignment from a template alignment.
//!
//! Creating an alignment in Civil 3D adds (S24 → S25, docs/alignment-object.md
//! "Creating an alignment"):
//!
//! - the `AECC_ALIGNMENT` entity,
//! - its companion `AECC_FEATURE_STYLE` object (parcel-topology entry) with
//!   a key and a 32-hex-digit ID string,
//! - optionally an `AECC_ALIGNMENT_STAEQU_LABEL_GROUP` entity,
//! - an entry in the drawing's companion registry ([`crate::aecc_registry`]).
//!
//! [`create_alignment`] copies a **template** alignment, its companion and
//! (if wanted) its label group from a donor drawing — any drawing with an
//! alignment made by Civil 3D (S25 in the tests) — and turns them into a new
//! alignment of the target:
//!
//! - fresh handles (companion, alignment, label group, in that order, as
//!   Civil 3D allocates them), written as each record's own handle and in
//!   every reference ([`handle_refs::set_own_handle`],
//!   [`handle_refs::rewrite_refs`]) — any byte length;
//! - the companion gets a new registry key and ID string, and is added to
//!   the target's registry, sorted by ID string;
//! - the alignment gets the requested name, PIs and free curves
//!   ([`aecc_build::set_geometry`]), a fresh identity GUID and the requested
//!   (or the donor's) alignment style.
//!
//! The label group is written only when the target has its label style; it
//! is optional (S27 → S29). Without the parcel entry (an experiment, S30)
//! the alignment's companion references are null.

use crate::aecc_alignment_refs::alignment_refs;
use crate::aecc_build;
use crate::aecc_registry::{companion_key, Entry, Registry};
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

#[derive(Debug, Clone, PartialEq)]
pub enum CreateError {
    /// The donor drawing doesn't have a usable template alignment.
    Donor(String),
    /// The target lacks an object the new records must point at.
    MissingInTarget(&'static str, u64),
    /// The name must not be empty.
    NameLength {
        want: usize,
        got: usize,
    },
    /// A record could not be rewritten.
    Layout(String),
    Geometry(String),
}

impl fmt::Display for CreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Donor(why) => write!(f, "donor drawing: {why}"),
            Self::MissingInTarget(what, h) => write!(f, "target has no {what} {h:X}"),
            Self::NameLength { .. } => write!(f, "name must not be empty"),
            Self::Layout(why) => write!(f, "record layout: {why}"),
            Self::Geometry(e) => write!(f, "geometry: {e}"),
        }
    }
}

impl std::error::Error for CreateError {}

/// Options for [`create_alignment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateOptions {
    /// Write the station-equation label group (if the target has its label
    /// style).
    pub label_group: bool,
    /// The parcel-topology entry: companion object plus registry entry.
    /// When false, the alignment's two companion references are nulled.
    pub parcel_entry: bool,
    /// Alignment style handle in the target; default: the template's.
    pub style: Option<u64>,
    /// Handle of the template alignment in the donor; default: the donor's
    /// alignment with the highest handle that has a companion.
    pub template: Option<u64>,
    /// Write proxy graphics (the display cache). Civil 3D regenerates them
    /// (S57), but AutoCAD and viewers without Civil 3D draw only these.
    pub graphics: bool,
    /// Alignment type value (1 = Centerline, 4 = Miscellaneous);
    /// default: the template's.
    pub alignment_type: Option<i32>,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            label_group: true,
            parcel_entry: true,
            style: None,
            template: None,
            graphics: true,
            alignment_type: None,
        }
    }
}

/// What [`create_alignment`] added.
#[derive(Debug, Clone, PartialEq)]
pub struct Created {
    pub alignment: u64,
    /// The companion's topology graph was rewritten for the new geometry.
    pub companion_rebuilt: bool,
    /// 0 when created without the parcel entry.
    pub companion: u64,
    /// 0 when no label group was written.
    pub label_group: u64,
    pub registry: u64,
    pub registry_key: u32,
    pub id_string: String,
    pub name: String,
    pub style: u64,
}

fn donor_err(msg: impl Into<String>) -> CreateError {
    CreateError::Donor(msg.into())
}

fn lay(msg: impl Into<String>) -> CreateError {
    CreateError::Layout(msg.into())
}

fn record(doc: &CadDocument, class: &str, handle: u64) -> Option<RawRecord> {
    raw::records(doc, class)
        .into_iter()
        .find(|r| r.handle == handle)
}

/// The drawing's companion registry.
pub fn find_registry(doc: &CadDocument) -> Option<(RawRecord, Registry)> {
    raw::records(doc, COMPANION)
        .into_iter()
        .find_map(|r| Registry::parse(&r).ok().map(|g| (r, g)))
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
        // Civil 3D's ID strings have a '4' at position 13 (S24, S25), like
        // a version-4 GUID written without dashes.
        s.replace_range(12..13, "4");
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

/// Set own handle then all references of `rec` (mapped by `map`).
fn rehandle(
    rec: &RawRecord,
    new: u64,
    map: impl Fn(&HandleRef) -> u64,
) -> Result<(Vec<u8>, usize), CreateError> {
    let refs = handle_refs::of_record(rec);
    let targets: Vec<u64> = refs.iter().map(&map).collect();
    let (d, hb) = handle_refs::set_own_handle(&rec.data, rec.handle_bits, new)
        .ok_or_else(|| lay(format!("record {:X}: own handle", rec.handle)))?;
    handle_refs::rewrite_refs(&d, hb, new, &targets)
        .ok_or_else(|| lay(format!("record {:X}: handle stream", rec.handle)))
}

/// The class number of `dxf_name` in `target`, adding the donor's class
/// entry when the target lacks it (a Civil 3D drawing that never had an
/// alignment, S65). DWG class numbers are positional (500 + index), so a
/// new entry is appended with the next number.
pub fn ensure_class(
    target: &mut CadDocument,
    donor: &CadDocument,
    dxf_name: &str,
) -> Result<i16, CreateError> {
    if let Some(c) = target.classes.get_by_name(dxf_name) {
        return Ok(c.class_number);
    }
    let mut c = donor
        .classes
        .get_by_name(dxf_name)
        .cloned()
        .ok_or_else(|| donor_err(format!("no class {dxf_name}")))?;
    let n = 500 + target.classes.len() as i16;
    if target.classes.iter().any(|x| x.class_number >= n) {
        return Err(lay("target class numbers are not positional"));
    }
    c.class_number = n;
    c.instance_count = 1;
    target.classes.push_preserving(c);
    Ok(n)
}

/// Write the object type (class number) at the start of a record in the
/// form AutoCAD uses (BOT): `00` + byte for 0–255, `01` + (n − 0x1F0) for
/// 496–751, `10` + RS otherwise. The template record may use the other
/// form (S123's companion class 736 vs S44's 1056), so the record can grow
/// or shrink by 8 bits. Returns the new bytes and handle-stream size.
pub fn set_object_type(
    data: &[u8],
    handle_bits: usize,
    class_number: i16,
) -> Result<(Vec<u8>, usize), CreateError> {
    let old = crate::handle_refs::object_type_bits(data)
        .ok_or_else(|| lay("object type form 11 is not valid"))?;
    let v = class_number as u16;
    let mut w = crate::aecc_build::Bits::default();
    match v {
        0..=0xFF => {
            w.bit(0);
            w.bit(0);
            w.byte(v as u8);
        }
        0x1F0..=0x2EF => {
            w.bit(0);
            w.bit(1);
            w.byte((v - 0x1F0) as u8);
        }
        _ => {
            w.bit(1);
            w.bit(0);
            w.byte((v & 0xFF) as u8);
            w.byte((v >> 8) as u8);
        }
    }
    let total = raw::bit_len(data);
    let data_end = total - handle_bits;
    let new_data_end = data_end - old + w.len();
    w.copy(data, old, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    Ok((bytes, hb))
}

fn is_layer(doc: &CadDocument, h: u64) -> bool {
    doc.layers.iter().any(|l| l.handle.value() == h)
}

/// Add a new alignment to `target`, built from a template alignment in
/// `donor`. `seed` feeds the ID string, the registry key choice is
/// deterministic (largest + 1), and the identity GUID.
pub fn create_alignment(
    target: &mut CadDocument,
    donor: &CadDocument,
    name: &str,
    pis: &[Point2],
    radii: &[Option<f64>],
    seed: u64,
    opts: CreateOptions,
) -> Result<Created, CreateError> {
    if name.is_empty() {
        return Err(CreateError::NameLength { want: 1, got: 0 });
    }
    // --- Template records in the donor. ---
    // Only siteless alignments: an alignment in a site points at the
    // site's container, not at a companion of its own (S102).
    let candidates: Vec<RawRecord> = raw::records(donor, ALIGNMENT)
        .into_iter()
        .filter(|r| alignment_refs(r).is_some_and(|x| x.companion.handle != 0))
        .filter(|r| {
            matches!(
                crate::aecc_site::membership(donor, r),
                crate::aecc_site::Membership::Siteless { .. }
            )
        })
        .collect();
    let d_align = match opts.template {
        Some(h) => candidates.into_iter().find(|r| r.handle == h),
        None => candidates.into_iter().max_by_key(|r| r.handle),
    }
    .ok_or_else(|| donor_err("no siteless alignment with a companion"))?;
    let refs = alignment_refs(&d_align).unwrap();
    let d_comp_h = refs.companion.handle;
    let d_comp = record(donor, COMPANION, d_comp_h)
        .ok_or_else(|| donor_err(format!("companion {d_comp_h:X} missing")))?;
    let d_label = raw::records(donor, LABEL_GROUP).into_iter().find(|r| {
        handle_refs::of_record(r)
            .iter()
            .any(|x| x.code == 4 && x.handle == d_align.handle)
    });

    // --- What the new records point at, in the target. ---
    let style = opts.style.unwrap_or(refs.style.handle);
    if raw::class_of(target, style).as_deref() != Some("AECC_ALIGNMENT_STYLE") {
        return Err(CreateError::MissingInTarget("alignment style", style));
    }
    if !is_layer(target, refs.layer.handle) {
        return Err(CreateError::MissingInTarget("layer", refs.layer.handle));
    }
    let (t_reg_rec, t_reg) =
        find_registry(target).ok_or(CreateError::MissingInTarget("companion registry", 0))?;
    // The label group's other references (layer, label style) must exist.
    let label = d_label.filter(|l| {
        opts.label_group
            && handle_refs::of_record(l).iter().all(|r| {
                r.handle == 0
                    || r.handle == d_align.handle
                    || is_layer(target, r.handle)
                    || raw::class_of(target, r.handle).is_some()
            })
    });

    // --- ID string, key, handles. ---
    let old_name =
        raw::object_name(&d_align.data).ok_or_else(|| donor_err("alignment has no name"))?;
    let old_id = id_string_of(&d_comp).ok_or_else(|| donor_err("companion has no ID string"))?;
    let existing_ids: Vec<String> = t_reg.entries.iter().map(|e| e.id.clone()).collect();
    let new_id = new_id_string(seed, &existing_ids);
    let key = t_reg.next_key();
    let comp_h = target.allocate_handle().value();
    let align_h = target.allocate_handle().value();
    let label_h = target.allocate_handle().value();

    // Companion: own handle, refs (registry, alignment), key, ID string.
    let (mut comp, comp_hb) = rehandle(&d_comp, comp_h, |r| {
        if r.handle == d_align.handle {
            align_h
        } else if r.code != 2 && r.code != 4 && r.handle != 0 {
            t_reg.handle // the own-offset reference to the registry
        } else {
            r.handle
        }
    })?;
    let crec = RawRecord {
        handle: comp_h,
        class: COMPANION.into(),
        is_entity: false,
        data: comp.clone(),
        handle_bits: comp_hb,
    };
    let (kbit, _) = companion_key(&crec).ok_or_else(|| lay("companion key not found"))?;
    for (i, b) in key.to_le_bytes().iter().enumerate() {
        raw::set_byte(&mut comp, kbit + 8 * i, *b);
    }
    if replace_string(&mut comp, &old_id, &new_id) != 1 {
        return Err(donor_err("companion ID string not found once"));
    }

    // A template's design check set is kept only if the target has the
    // same check set; otherwise the reference is nulled and the "use
    // design check set" box cleared (S80/S81: a set box without the
    // reference is cleared by Civil 3D anyway).
    let check_set_kept = refs.check_set.handle == 0
        || raw::class_of(target, refs.check_set.handle).as_deref()
            == Some("AECC_ALIGNMENT_DESIGN_CHECK_SET");

    // Alignment: own handle, refs (companion, style), geometry, identity,
    // name.
    let (align, ahb) = rehandle(&d_align, align_h, |r| {
        if r.handle == d_comp_h {
            if opts.parcel_entry {
                comp_h
            } else {
                0
            }
        } else if r.bit == refs.style.bit {
            style
        } else if r.bit == refs.check_set.bit && !check_set_kept {
            0
        } else {
            r.handle
        }
    })?;
    let rec = RawRecord {
        handle: align_h,
        class: ALIGNMENT.into(),
        is_entity: true,
        data: align,
        handle_bits: ahb,
    };
    // No PIs: an empty alignment, copied from an empty template (S113
    // `EMPTY`); its speed-table entry gets a fresh GUID.
    let (bytes, hb) = if pis.is_empty() {
        if !crate::aecc_elements::element_records(&rec.data).is_empty() {
            return Err(CreateError::Geometry(
                "no PIs given, but the template has elements".into(),
            ));
        }
        let mut d = rec.data.clone();
        let (entries, end) = crate::aecc_design_speed::empty_alignment_table(&rec)
            .ok_or_else(|| lay("empty template: design-speed table not found"))?;
        if entries.len() == 1 {
            let g = aecc_build::guid_v4(seed.rotate_left(23));
            for (i, v) in g.iter().enumerate() {
                raw::set_byte(&mut d, end - 128 + 8 * i, *v);
            }
        }
        (d, rec.handle_bits)
    } else {
        let (bytes, hb, _) = aecc_build::set_geometry(&rec, pis, radii)
            .map_err(|e| CreateError::Geometry(e.to_string()))?;
        (bytes, hb)
    };
    let mut align = bytes;
    // Never carry the template's MIST1 over to a new alignment (D-046).
    aecc_build::clear_geometry_key(&mut align);
    let mut rec = RawRecord {
        data: align.clone(),
        handle_bits: hb,
        ..rec
    };
    // A new alignment starts at station 0 (templates may start elsewhere,
    // S98 `Chain` at 2+20); speeds move with the start (D-031).
    if crate::aecc_station::start_station(&rec).is_some_and(|s| s != 0.0) {
        let (b, h) = crate::aecc_station::move_start_station(&rec, 0.0)
            .ok_or_else(|| lay("start station not found"))?;
        align = b.clone();
        rec = RawRecord {
            data: b,
            handle_bits: h,
            ..rec
        };
    }
    if !check_set_kept {
        let mut c = crate::aecc_criteria::criteria(&rec).map_err(|e| lay(e.to_string()))?;
        c.check_set = false;
        crate::aecc_criteria::set_criteria(&rec, &mut align, c).map_err(|e| lay(e.to_string()))?;
        rec.data = align.clone();
    }
    // The "identity" is the first speed interval's GUID (O4); an
    // alignment without intervals has none to renew.
    if crate::aecc_design_speed::first_interval_station(&rec).is_some() {
        aecc_build::set_identity(&rec, &mut align, seed.rotate_left(17))
            .map_err(|e| lay(e.to_string()))?;
    }
    if let Some(t) = opts.alignment_type {
        aecc_build::set_alignment_type(&rec, &mut align, t).map_err(|e| lay(e.to_string()))?;
    }
    let (align, align_hb) = string_stream::set_name(&align, rec.handle_bits, &old_name, name)
        .map_err(|e| lay(e.to_string()))?;
    // Each rewrite step keeps the previous padding; trim it so no stray
    // zero byte follows the last reference (S56).
    let (mut align, mut align_hb) = handle_refs::trim_padding(&align, align_hb, align_h);
    if !opts.graphics {
        (align, align_hb) =
            aecc_build::strip_graphics(&align, align_hb).map_err(|e| lay(e.to_string()))?;
    }

    // Label group: own handle, reference to the alignment.
    let label_rec = match &label {
        Some(l) => Some(rehandle(l, label_h, |r| {
            if r.handle == d_align.handle {
                align_h
            } else {
                r.handle
            }
        })?),
        None => None,
    };

    // --- Insert into the target. ---
    if opts.parcel_entry {
        let comp_class = ensure_class(target, donor, COMPANION)?;
        let (comp, comp_hb) = set_object_type(&comp, comp_hb, comp_class)?;
        let (reg, reg_hb) = t_reg
            .with_entry(
                &t_reg_rec,
                Entry {
                    key,
                    companion: comp_h,
                    id: new_id.clone(),
                },
            )
            .map_err(|e| lay(e.to_string()))?;
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
                owner: Handle::new(t_reg.handle),
                raw_dxf_codes,
                raw_dwg_data: Some(comp),
                raw_dwg_handle_bits: comp_hb as i64,
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
            .find(|(h, _)| h.value() == t_reg.handle)
            .map(|(_, o)| o)
        {
            *raw_dwg_data = Some(reg);
            *raw_dwg_handle_bits = reg_hb as i64;
        }
    }

    let mut entities = vec![(d_align.handle, align_h, align, align_hb, ALIGNMENT)];
    if let (Some(l), Some((data, hb))) = (&label, label_rec) {
        entities.push((l.handle, label_h, data, hb, LABEL_GROUP));
    }
    let wrote_label = entities.len() == 2;
    for (src, new_h, data, hb, class) in entities {
        let n = ensure_class(target, donor, class)?;
        let (data, hb) = set_object_type(&data, hb, n)?;
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
        target.add_entity(e).map_err(|e| lay(e.to_string()))?;
    }

    // The companion still holds the template's graph (S107); rebuild it
    // from the new geometry.
    let companion_rebuilt = opts.parcel_entry
        && crate::aecc_topology::refresh_companion(target, align_h).map_err(lay)?;

    Ok(Created {
        companion_rebuilt,
        alignment: align_h,
        companion: if opts.parcel_entry { comp_h } else { 0 },
        label_group: if wrote_label { label_h } else { 0 },
        registry: t_reg.handle,
        registry_key: key,
        id_string: new_id,
        name: name.to_string(),
        style,
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
}
