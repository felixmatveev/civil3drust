//! Site edits (docs/site-object.md): create, rename, describe and delete
//! a site, and move a non-crossing alignment into a site and back.
//!
//! Each writer was proved in Civil 3D 2024 (no RECOVER, clean AUDIT, the
//! re-save reads back to the same model): create S339/S340 → S347/S348,
//! rename/describe S336/S337 → S349/S350, delete S338 → S346, move in
//! S332/S345 → S342/S351, move out S333/S334 → S341/S343 (D-079).
//!
//! Three records make a site: the site (`AECC_PARCEL`, strings
//! `AeccImpFace`, `""`, name (optional), description, user), its container
//! (`AECC_FEATURE_STYLE`, handle site + 1: nodes, edge entries, faces) and
//! the site list (`AECC_PARCEL_NODE`: keys in hash order, names, site
//! references). A member alignment points at the container twice and
//! stores the ID of its edge there. Caches (site graphics) are dropped,
//! not written; Civil 3D rebuilds them.
//!
//! Not handled: members that cross or touch (crossing nodes, parcels),
//! feature lines, spirals, deleting a site with members.

use crate::aecc_build::Bits;
use crate::aecc_elements::{decode_elements, Segment};
use crate::raw::{self, RawRecord};
use crate::{handle_refs, proxy_graphics, string_stream};
use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
use acadrust::Handle;
use std::fmt;

/// Why a site edit was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteEditError(pub String);

impl fmt::Display for SiteEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "site: {}", self.0)
    }
}

impl std::error::Error for SiteEditError {}

impl From<String> for SiteEditError {
    fn from(s: String) -> Self {
        SiteEditError(s)
    }
}

impl From<&str> for SiteEditError {
    fn from(s: &str) -> Self {
        SiteEditError(s.to_string())
    }
}

type Res<T> = Result<T, SiteEditError>;

macro_rules! say {
    ($o:expr, $($a:tt)*) => {{
        $o.push_str(&format!($($a)*));
        $o.push('\n');
    }};
}
/// Points of a path and the bulge of the segment starting at each.
type Path = (Vec<(f64, f64)>, Vec<Option<f64>>);

/// A site made by [`create_site`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatedSite {
    pub site: u64,
    pub container: u64,
    pub key: u32,
}

/// What a move changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    pub alignment: u64,
    pub site: u64,
    /// The site's container.
    pub container: u64,
    /// The companion emptied (move in) or refilled (move out).
    pub companion: u64,
}

const SITE: &str = "AECC_PARCEL";
const LIST: &str = "AECC_PARCEL_NODE";
const CONT: &str = "AECC_FEATURE_STYLE";
/// Bits of the site record's fixed block between graphics and tail.
const SITE_BLOCK: usize = 760;
/// Offset of the key's four bytes in that block (a raw BL: `00` before them).
const SITE_KEY: usize = 108;
/// Constant bits between a site's tail and its strings.
const SITE_END: usize = 19;
/// Constant bits between the site list's keys and its strings.
const LIST_END: usize = 13;

fn rec(doc: &acadrust::CadDocument, class: &str, h: u64) -> Res<RawRecord> {
    raw::records(doc, class)
        .into_iter()
        .find(|r| r.handle == h)
        .ok_or(format!("no {class} {h:X}").into())
}

fn put(doc: &mut acadrust::CadDocument, h: u64, data: Vec<u8>, hb: usize) -> Res<()> {
    let (data, hb) = handle_refs::trim_padding(&data, hb, h);
    if raw::set_record(doc, h, data, hb) {
        Ok(())
    } else {
        Err(format!("could not write {h:X}").into())
    }
}

fn bl(d: &[u8], p: &mut usize) -> Res<i32> {
    let (v, q) = raw::read_bl(d, *p).ok_or(format!("BL at bit {p}"))?;
    *p = q;
    Ok(v)
}

fn bd(d: &[u8], p: &mut usize) -> Res<f64> {
    let (v, q) = raw::read_bd(d, *p).ok_or(format!("BD at bit {p}"))?;
    *p = q;
    Ok(v)
}

/// The data stream rebuilt as `w` (everything before the strings),
/// followed by the old strings and a new handle stream.
fn finish(mut w: Bits, old: &RawRecord, refs: &[(u8, u64)]) -> Res<(Vec<u8>, usize)> {
    let d = &old.data;
    let ss = string_stream::parse(d, old.handle_bits).map_err(|e| e.to_string())?;
    w.copy(d, ss.start, ss.data_end);
    let data_end = w.len();
    for &(code, h) in refs {
        let (code, v) = match code {
            8 if h + 1 == old.handle => (8, 0),
            8 | 0xC => (0xC, old.handle - h),
            c => (c, h),
        };
        let n = if v == 0 {
            0
        } else {
            8 - v.leading_zeros() as usize / 8
        };
        w.byte((code << 4) | n as u8);
        for i in (0..n).rev() {
            w.byte((v >> (8 * i)) as u8);
        }
    }
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - data_end;
    Ok((bytes, hb))
}

fn refs_of(r: &RawRecord) -> Vec<(u8, u64)> {
    handle_refs::of_record(r)
        .iter()
        .map(|x| (x.code, x.handle))
        .collect()
}

// ---------------------------------------------------------------- topology

/// One vertex of an edge (see [`crate::aecc_topology::Vertex`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub kind: i32,
    pub flags: i32,
    pub node: i32,
    pub a: i32,
    pub b: i32,
    pub bulge: Option<f64>,
}

/// One live edge entry: a member's polyline through the nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: i32,
    pub flags: i32,
    pub vertices: Vec<Vertex>,
    pub after: Vec<i32>,
}

/// One face: its ID, flags and loops (first node of a boundary, k).
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub id: i32,
    pub flags: i32,
    pub loops: Vec<(i32, i32)>,
}

/// A site container or companion record, decoded to its last value
/// (docs/site-object.md, "Container and companion"): every one of the
/// container records of S101–S112 and S313–S353 parses and re-encodes to
/// the same bits.
#[derive(Clone, Debug, PartialEq)]
pub struct Topology {
    pub head_end: usize,
    pub node_next: i32,
    pub nodes: Vec<(i32, f64, f64, f64)>,
    pub edge_next: i32,
    pub edges: Vec<Option<Edge>>,
    pub face_next: i32,
    pub faces: Vec<Face>,
    /// X, six counter pairs, six lists (as stored).
    pub tail: Vec<i32>,
    /// Handle stream: owner; hard nulls of `after` values; members of the
    /// live edges; face objects; style references.
    pub owner: (u8, u64),
    pub after_refs: Vec<(u8, u64)>,
    pub members: Vec<u64>,
    pub face_refs: Vec<u64>,
    pub styles: Vec<(u8, u64)>,
}

impl Topology {
    /// Decode a container or companion record.
    pub fn parse(r: &RawRecord) -> Res<Topology> {
        let d = &r.data;
        let ss = string_stream::parse(d, r.handle_bits).map_err(|e| e.to_string())?;
        // The constant run ends with the raw BL 0x80000001.
        let mark: Vec<u8> = [0u8, 0]
            .into_iter()
            .chain(
                [0x01u8, 0, 0, 0x80]
                    .iter()
                    .flat_map(|b| (0..8).rev().map(move |k| (b >> k) & 1)),
            )
            .collect();
        let head_end = (40..ss.start.saturating_sub(34))
            .find(|&b| (0..34).all(|k| raw::get_bit(d, b + k) == mark[k]))
            .ok_or("no constant run")?
            + 34;
        let mut p = head_end;
        let node_next = bl(d, &mut p)?;
        let n = bl(d, &mut p)?;
        let mut id = bl(d, &mut p)?;
        let mut nodes = Vec::new();
        for _ in 0..n {
            let (x, y, z) = (bd(d, &mut p)?, bd(d, &mut p)?, bd(d, &mut p)?);
            p += 3;
            nodes.push((id, x, y, z));
            id = bl(d, &mut p)?;
        }
        let edge_next = bl(d, &mut p)?;
        let slots = bl(d, &mut p)?;
        let mut edges = Vec::new();
        for _ in 0..slots {
            match bl(d, &mut p)? {
                2 => edges.push(None),
                0 => {
                    let id = bl(d, &mut p)?;
                    let flags = bl(d, &mut p)?;
                    let n = bl(d, &mut p)?;
                    bl(d, &mut p)?;
                    let mut vertices = Vec::new();
                    for _ in 0..n {
                        let mut v = Vertex {
                            kind: bl(d, &mut p)?,
                            flags: bl(d, &mut p)?,
                            node: bl(d, &mut p)?,
                            a: bl(d, &mut p)?,
                            b: bl(d, &mut p)?,
                            bulge: None,
                        };
                        if v.kind == 1 {
                            v.bulge = Some(bd(d, &mut p)?);
                            bl(d, &mut p)?;
                        }
                        vertices.push(v);
                    }
                    let m = bl(d, &mut p)?;
                    let after = (0..m).map(|_| bl(d, &mut p)).collect::<Res<_>>()?;
                    for _ in 0..3 {
                        bl(d, &mut p)?;
                    }
                    edges.push(Some(Edge {
                        id,
                        flags,
                        vertices,
                        after,
                    }));
                }
                s => return Err(format!("edge slot state {s}").into()),
            }
        }
        let face_next = bl(d, &mut p)?;
        let n = bl(d, &mut p)?;
        let mut faces = Vec::new();
        for _ in 0..n {
            bl(d, &mut p)?;
            let id = bl(d, &mut p)?;
            let flags = bl(d, &mut p)?;
            let k = bl(d, &mut p)?;
            let loops = (0..k)
                .map(|_| Ok((bl(d, &mut p)?, bl(d, &mut p)?)))
                .collect::<Res<_>>()?;
            faces.push(Face { id, flags, loops });
        }
        let mut tail = Vec::new();
        while p < ss.start {
            tail.push(bl(d, &mut p)?);
        }
        if p != ss.start || tail.len() < 19 {
            return Err("tail does not end at the strings".into());
        }
        let refs = refs_of(r);
        let live = edges.iter().flatten().count();
        let after: usize = edges.iter().flatten().map(|e| e.after.len()).sum();
        let x = tail[0] as usize;
        if refs.len() != 1 + after + 2 * live + 2 * faces.len() + x {
            return Err(format!("{} references do not fit the graph", refs.len()).into());
        }
        let m0 = 1 + after;
        let f0 = m0 + 2 * live;
        Ok(Topology {
            head_end,
            node_next,
            nodes,
            edge_next,
            edges,
            face_next,
            faces,
            tail,
            owner: refs[0],
            after_refs: refs[1..m0].to_vec(),
            members: (0..live).map(|i| refs[m0 + 2 * i].1).collect(),
            face_refs: (0..(refs.len() - f0 - x) / 2)
                .map(|i| refs[f0 + 2 * i].1)
                .collect(),
            styles: refs[refs.len() - x..].to_vec(),
        })
    }

    /// The record's bytes and handle-stream size for this graph: the
    /// head and strings of `old`, nodes in Civil 3D's order.
    pub fn write(&self, old: &RawRecord) -> Res<(Vec<u8>, usize)> {
        let mut w = Bits::default();
        w.copy(&old.data, 0, self.head_end);
        // Nodes in Civil 3D's order: ID mod 8, then ID.
        let mut nodes = self.nodes.clone();
        nodes.sort_by_key(|n| (n.0 & 7, n.0));
        w.bl(self.node_next);
        w.bl(nodes.len() as i32);
        w.bl(nodes.first().map_or(0, |n| n.0));
        for (i, n) in nodes.iter().enumerate() {
            w.bd(n.1);
            w.bd(n.2);
            w.bd(n.3);
            w.bits(0b010, 3);
            w.bl(nodes.get(i + 1).map_or(0, |m| m.0));
        }
        w.bl(self.edge_next);
        w.bl(self.edges.len() as i32);
        for e in &self.edges {
            let Some(e) = e else {
                w.bl(2);
                continue;
            };
            for v in [0, e.id, e.flags, e.vertices.len() as i32, 0] {
                w.bl(v);
            }
            for v in &e.vertices {
                for x in [v.kind, v.flags, v.node, v.a, v.b] {
                    w.bl(x);
                }
                if let Some(b) = v.bulge {
                    w.bd(b);
                    w.bl(0);
                }
            }
            w.bl(e.after.len() as i32);
            for v in e.after.iter().chain(&[0, 0, 0]) {
                w.bl(*v);
            }
        }
        w.bl(self.face_next);
        w.bl(self.faces.len() as i32);
        for f in &self.faces {
            for v in [f.id, f.id, f.flags, f.loops.len() as i32] {
                w.bl(v);
            }
            for l in &f.loops {
                w.bl(l.0);
                w.bl(l.1);
            }
        }
        for v in &self.tail {
            w.bl(*v);
        }
        let mut refs = vec![self.owner];
        refs.extend(&self.after_refs);
        for h in self.members.iter().chain(&self.face_refs) {
            refs.push((4, *h));
            refs.push((4, 0));
        }
        refs.extend(&self.styles);
        finish(w, old, &refs)
    }

    /// Index of the live edge in slot `slot` among the member references.
    pub fn member_index(&self, slot: usize) -> usize {
        self.edges[..slot].iter().flatten().count()
    }

    /// Add an edge through new nodes at `pts` (bulge per segment); returns
    /// the first node's ID. `faced`: vertices carry face 1 on both sides.
    fn add_edge(
        &mut self,
        pts: &[(f64, f64)],
        bulges: &[Option<f64>],
        flags: i32,
        faced: bool,
    ) -> i32 {
        let first = self.node_next;
        let n = pts.len();
        let vertices = (0..n)
            .map(|i| {
                let last = i + 1 == n;
                let bulge = if last { None } else { bulges[i] };
                Vertex {
                    kind: if last { 2 } else { bulge.is_some() as i32 },
                    flags: match (faced, last) {
                        (false, _) => 10,
                        (true, false) => 50,
                        (true, true) => 2,
                    },
                    node: first + i as i32,
                    a: (faced && !last) as i32,
                    b: (faced && !last) as i32,
                    bulge,
                }
            })
            .collect();
        for (i, p) in pts.iter().enumerate() {
            self.nodes.push((first + i as i32, p.0, p.1, 0.0));
        }
        self.node_next += n as i32;
        // An emptied companion gets edge ID 1 again (Civil 3D never reuses
        // an ID, but it never refills a companion either).
        let refill = self.edge_next == 2 && self.edges.iter().all(|s| s.is_none());
        let e = Some(Edge {
            id: if refill { 1 } else { self.edge_next },
            flags,
            vertices,
            after: Vec::new(),
        });
        if refill {
            self.edges = vec![e];
        } else {
            self.edges.push(e);
            self.edge_next += 1;
        }
        first
    }

    /// Free `slot`: its nodes, loops and member reference go.
    fn remove_edge(&mut self, slot: usize) -> Res<()> {
        let e = self.edges[slot].clone().ok_or("slot already free")?;
        let ids: Vec<i32> = e.vertices.iter().map(|v| v.node).collect();
        let shared = self
            .edges
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != slot)
            .filter_map(|(_, e)| e.as_ref())
            .any(|o| o.vertices.iter().any(|v| ids.contains(&v.node)));
        if shared || self.faces.len() > 1 || !e.after.is_empty() {
            return Err("the edge crosses another, bounds a parcel or has elevation points".into());
        }
        let m = self.member_index(slot);
        self.members.remove(m);
        self.edges[slot] = None;
        self.nodes.retain(|n| !ids.contains(&n.0));
        for f in &mut self.faces {
            f.loops.retain(|l| !ids.contains(&l.0));
        }
        Ok(())
    }

    /// A one-line summary (for reports).
    pub fn describe(&self) -> String {
        let edges: Vec<String> = self
            .edges
            .iter()
            .map(|e| match e {
                None => "free".into(),
                Some(e) => format!(
                    "{}:0x{:X}[{}]",
                    e.id,
                    e.flags,
                    e.vertices
                        .iter()
                        .map(|v| format!("{}:{}:{}/{}", v.node, v.flags, v.a, v.b))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
            })
            .collect();
        format!(
            "nodes {} (next {}), edges (next {}) {:?}, faces (next {}) {:?}, tail {:?}, members {:X?}, faces→{:X?}, {} styles",
            self.nodes.len(),
            self.node_next,
            self.edge_next,
            edges,
            self.face_next,
            self.faces.iter().map(|f| (f.id, f.flags, f.loops.clone())).collect::<Vec<_>>(),
            self.tail,
            self.members,
            self.face_refs,
            self.styles.len()
        )
    }
}

/// Points and bulges of an alignment's elements, in path order.
fn alignment_path(r: &RawRecord) -> Res<Path> {
    let d = decode_elements(r).map_err(|e| format!("{e:?}"))?;
    let mut pts = Vec::new();
    let mut bulges = Vec::new();
    for e in &d.elements {
        let s = e.segment.start();
        pts.push((s.x, s.y));
        bulges.push(match e.segment {
            Segment::Arc { geometry: g, .. } => {
                let b = (g.sweep / 4.0).tan();
                Some(if g.counter_clockwise { b } else { -b })
            }
            Segment::Line { .. } => None,
            _ => return Err("alignments with spirals are not handled".into()),
        });
    }
    let e = d.elements.last().ok_or("no elements")?.segment.end();
    pts.push((e.x, e.y));
    Ok((pts, bulges))
}

fn segments_touch(a: [(f64, f64); 2], b: [(f64, f64); 2]) -> bool {
    let o = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
        (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
    };
    let (d1, d2) = (o(b[0], b[1], a[0]), o(b[0], b[1], a[1]));
    let (d3, d4) = (o(a[0], a[1], b[0]), o(a[0], a[1], b[1]));
    d1 * d2 <= 0.0 && d3 * d4 <= 0.0
}

// -------------------------------------------------------------- site record

/// Start of the site record's fixed block: the end of the proxy graphics,
/// or of the has-graphics flag when they were stripped.
fn site_block(d: &[u8]) -> Res<usize> {
    let eed = crate::aecc_entity::eed_bit(d).ok_or("site: object header")?;
    if raw::get_bit(d, eed + 2) == 0 {
        return Ok(eed + 3);
    }
    proxy_graphics::find(d, 4096)
        .map(|g| g.end_bit())
        .ok_or("site: no graphics".into())
}

struct SiteTail {
    start: usize,
    p: [f64; 3],
    flags: i32,
    entries: Vec<([f64; 3], [i32; 3])>,
    a: i32,
    rest: Vec<i32>,
}

fn site_tail(r: &RawRecord) -> Res<SiteTail> {
    let d = &r.data;
    let ss = string_stream::parse(d, r.handle_bits).map_err(|e| e.to_string())?;
    let start = site_block(d)? + SITE_BLOCK;
    let mut q = start;
    let p = [bd(d, &mut q)?, bd(d, &mut q)?, bd(d, &mut q)?];
    let flags = bl(d, &mut q)?;
    let n = bl(d, &mut q)?;
    let mut entries = Vec::new();
    for _ in 0..n {
        let pt = [bd(d, &mut q)?, bd(d, &mut q)?, bd(d, &mut q)?];
        entries.push((pt, [bl(d, &mut q)?, bl(d, &mut q)?, bl(d, &mut q)?]));
    }
    let a = bl(d, &mut q)?;
    let mut rest = Vec::new();
    while q < ss.start - SITE_END {
        rest.push(bl(d, &mut q)?);
    }
    if q != ss.start - SITE_END || !(3..=4).contains(&rest.len()) {
        return Err("site tail not in the known form".into());
    }
    Ok(SiteTail {
        start,
        p,
        flags,
        entries,
        a,
        rest,
    })
}

fn write_site_tail(r: &RawRecord, t: &SiteTail, refs: &[(u8, u64)]) -> Res<(Vec<u8>, usize)> {
    let d = &r.data;
    let ss = string_stream::parse(d, r.handle_bits).map_err(|e| e.to_string())?;
    let mut w = Bits::default();
    w.copy(d, 0, t.start);
    t.p.iter().for_each(|v| w.bd(*v));
    w.bl(t.flags);
    w.bl(t.entries.len() as i32);
    for (pt, v) in &t.entries {
        pt.iter().for_each(|x| w.bd(*x));
        v.iter().for_each(|x| w.bl(*x));
    }
    w.bl(t.a);
    t.rest.iter().for_each(|v| w.bl(*v));
    w.copy(d, ss.start - SITE_END, ss.start);
    finish(w, r, refs)
}

/// Set the site's extent point from its container's nodes.
fn refresh_site(doc: &mut acadrust::CadDocument, site: u64, topo: &Topology) -> Res<()> {
    let r = rec(doc, SITE, site)?;
    let mut t = site_tail(&r)?;
    if let Some(first) = topo.nodes.first() {
        let (x, y) = topo
            .nodes
            .iter()
            .fold((first.1, first.2), |m, n| (m.0.min(n.1), m.1.max(n.2)));
        t.p = [x, y, 0.0];
        t.flags = 122;
        t.a = 1;
    } else {
        // The last member left: Civil 3D zeroes the point (S106).
        t.p = [0.0; 3];
    }
    let (d, hb) = write_site_tail(&r, &t, &refs_of(&r))?;
    put(doc, site, d, hb)
}

fn container_of(doc: &acadrust::CadDocument, site: u64) -> Res<u64> {
    refs_of(&rec(doc, SITE, site)?)
        .iter()
        .find(|r| r.0 == 3 && r.1 != 0)
        .map(|r| r.1)
        .ok_or(format!("{site:X} is not a site (no container)").into())
}

// ---------------------------------------------------------------- site list

struct SiteList {
    rec: RawRecord,
    list_bit: usize,
    keys: Vec<u32>,
    names: Vec<String>,
    tree: (u8, u64),
    sites: Vec<u64>,
    last: u64,
    styles: Vec<(u8, u64)>,
}

fn site_list(doc: &acadrust::CadDocument) -> Res<SiteList> {
    let r = raw::records(doc, LIST)
        .into_iter()
        .next()
        .ok_or("no site list")?;
    let d = &r.data;
    let ss = string_stream::parse(d, r.handle_bits).map_err(|e| e.to_string())?;
    let end = ss.start - LIST_END;
    let (list_bit, keys) = (0..64usize)
        .find_map(|n| {
            let nb = if n == 0 { 2 } else { 10 };
            let at = end.checked_sub(34 * n + nb + 2)?;
            let mut p = at;
            if bl(d, &mut p).ok()? != 0 || bl(d, &mut p).ok()? != n as i32 {
                return None;
            }
            let keys: Vec<u32> = (0..n)
                .map(|_| {
                    (raw::get_bit(d, p) == 0 && raw::get_bit(d, p + 1) == 0)
                        .then(|| bl(d, &mut p).ok().map(|v| v as u32))
                        .flatten()
                })
                .collect::<Option<_>>()?;
            Some((at, keys))
        })
        .ok_or("site list: key list not found")?;
    let refs = refs_of(&r);
    let n = keys.len();
    if refs.len() < n + 2 {
        return Err("site list: too few references".into());
    }
    Ok(SiteList {
        list_bit,
        names: ss.strings.iter().skip(1).map(|s| s.text.clone()).collect(),
        tree: refs[0],
        sites: refs[1..=n].iter().map(|x| x.1).collect(),
        last: refs[n + 1].1,
        styles: refs[n + 2..].to_vec(),
        keys,
        rec: r,
    })
}

/// The list in Civil 3D's order: key mod 8, then key — a hash table's
/// order, as for the container's nodes (every list of S103–S348).
fn sorted(l: &SiteList) -> (Vec<u32>, Vec<u64>, Vec<String>) {
    let mut names = l.names.iter();
    let mut e: Vec<(u32, u64, Option<&String>)> = l
        .keys
        .iter()
        .zip(&l.sites)
        .map(|(k, s)| (*k, *s, if named(*k) { names.next() } else { None }))
        .collect();
    e.sort_by_key(|x| (x.0 & 7, x.0));
    (
        e.iter().map(|x| x.0).collect(),
        e.iter().map(|x| x.1).collect(),
        e.iter().filter_map(|x| x.2.cloned()).collect(),
    )
}

fn write_site_list(doc: &mut acadrust::CadDocument, l: &SiteList) -> Res<()> {
    let (keys, sites, names) = sorted(l);
    let d = &l.rec.data;
    let ss = string_stream::parse(d, l.rec.handle_bits).map_err(|e| e.to_string())?;
    let mut w = Bits::default();
    w.copy(d, 0, l.list_bit);
    w.bl(0);
    w.bl(keys.len() as i32);
    for k in &keys {
        w.bits(0, 2);
        k.to_le_bytes().iter().for_each(|b| w.byte(*b));
    }
    w.copy(d, ss.start - LIST_END, ss.start);
    let mut refs = vec![l.tree];
    refs.extend(sites.iter().map(|h| (4, *h)));
    refs.push((4, l.last));
    refs.extend(&l.styles);
    let (mut data, mut hb) = finish(w, &l.rec, &refs)?;
    // Strings: "" + the names of the named sites, in list order.
    let old = ss.strings.len() - 1;
    for i in (names.len()..old).rev() {
        (data, hb) = string_stream::remove(&data, hb, i + 1).map_err(|e| e.to_string())?;
    }
    for i in old..names.len() {
        (data, hb) = string_stream::insert_empty(&data, hb, i + 1).map_err(|e| e.to_string())?;
    }
    for (i, n) in names.iter().enumerate() {
        (data, hb) = string_stream::replace(&data, hb, i + 1, n).map_err(|e| e.to_string())?;
    }
    put(doc, l.rec.handle, data, hb)
}

fn named(key: u32) -> bool {
    key & 0x8000_0000 != 0
}

/// Position of site `i` among the named sites (its string index − 1).
fn name_index(l: &SiteList, i: usize) -> usize {
    l.keys[..i].iter().filter(|k| named(**k)).count()
}

/// A key above every key of the site list and the alignment registry.
fn new_key(doc: &acadrust::CadDocument, l: &SiteList) -> u32 {
    let reg = raw::records(doc, CONT)
        .into_iter()
        .filter_map(|r| crate::aecc_registry::Registry::parse(&r).ok())
        .flat_map(|r| r.entries.into_iter().map(|e| e.key))
        .collect::<Vec<_>>();
    let max = l
        .keys
        .iter()
        .chain(&reg)
        .filter(|k| named(**k))
        .map(|k| k & 0x7FFF_FFFF)
        .max()
        .unwrap_or(0);
    0x8000_0000 | (max + 1)
}

fn set_site_key(r: &RawRecord, key: u32) -> Res<Vec<u8>> {
    let at = site_block(&r.data)? + SITE_KEY;
    let mut d = r.data.clone();
    if raw::get_bit(&d, at - 2) != 0 || raw::get_bit(&d, at - 1) != 0 {
        return Err("site key is not a raw BL".into());
    }
    for (i, b) in key.to_le_bytes().iter().enumerate() {
        raw::set_byte(&mut d, at + 8 * i, *b);
    }
    Ok(d)
}

/// Name (and description) strings of a site record; an unnamed record
/// (four strings) gets the name string inserted.
fn set_site_strings(r: &RawRecord, name: &str, description: Option<&str>) -> Res<(Vec<u8>, usize)> {
    let e = |e: string_stream::StringStreamError| e.to_string();
    let ss = string_stream::parse(&r.data, r.handle_bits).map_err(e)?;
    let (mut d, mut hb) = (r.data.clone(), r.handle_bits);
    match ss.strings.len() {
        4 => (d, hb) = string_stream::insert_empty(&d, hb, 2).map_err(e)?,
        5 => {}
        n => return Err(format!("site record with {n} strings").into()),
    }
    (d, hb) = string_stream::replace(&d, hb, 2, name).map_err(e)?;
    if let Some(t) = description {
        (d, hb) = string_stream::replace(&d, hb, 3, t).map_err(e)?;
    }
    // Civil 3D pads the handle stream with 1-bits, which can read as one
    // more reference once the record's length changes.
    handle_refs::keep_refs(&d, hb, r.handle, refs_of(r).len()).ok_or("site references".into())
}

// ----------------------------------------------------------------- commands

/// Rename a site and/or set its description (S336/S349). The name is
/// always stored as a string with a high-bit key, also over a built-in
/// name (S337/S350, D-078).
pub fn rename_site(
    doc: &mut acadrust::CadDocument,
    site: u64,
    name: Option<&str>,
    description: Option<&str>,
) -> Res<()> {
    let mut l = site_list(doc)?;
    let i = l
        .sites
        .iter()
        .position(|h| *h == site)
        .ok_or("site not in the site list")?;
    let mut r = rec(doc, SITE, site)?;
    let Some(name) = name else {
        let n = string_stream::parse(&r.data, r.handle_bits)
            .map_err(|e| e.to_string())?
            .strings
            .len();
        let (d, hb) =
            string_stream::replace(&r.data, r.handle_bits, n - 2, description.unwrap_or(""))
                .map_err(|e| e.to_string())?;
        let (d, hb) =
            handle_refs::keep_refs(&d, hb, site, refs_of(&r).len()).ok_or("site references")?;
        return put(doc, site, d, hb);
    };
    let ni = name_index(&l, i);
    if named(l.keys[i]) {
        l.names[ni] = name.to_string();
    } else {
        l.keys[i] = new_key(doc, &l);
        l.names.insert(ni, name.to_string());
        r.data = set_site_key(&r, l.keys[i])?;
    }
    let (d, hb) = set_site_strings(&r, name, description)?;
    put(doc, site, d, hb)?;
    write_site_list(doc, &l)
}

/// Delete an empty site: its entity, container and list entry (S338/S346,
/// as Civil 3D's S331).
pub fn delete_site(doc: &mut acadrust::CadDocument, site: u64) -> Res<()> {
    let cont = container_of(doc, site)?;
    let t = Topology::parse(&rec(doc, CONT, cont)?)?;
    if t.edges.iter().any(|e| e.is_some()) {
        return Err("the site has members; move them out first".into());
    }
    let mut l = site_list(doc)?;
    let i = l
        .sites
        .iter()
        .position(|h| *h == site)
        .ok_or("site not in the site list")?;
    if named(l.keys[i]) {
        l.names.remove(name_index(&l, i));
    }
    l.keys.remove(i);
    l.sites.remove(i);
    if l.last == site {
        l.last = 0;
    }
    write_site_list(doc, &l)?;
    doc.remove_entity(Handle::new(site))
        .ok_or("site entity not removable")?;
    // acadrust removes the objects the entity owns; make sure of it.
    doc.objects.remove(&Handle::new(cont));
    Ok(())
}

/// Create an empty site named `name` by cloning the first site's record
/// and container (S339/S347, S340/S348): new handles, a new high-bit key,
/// no graphics unless the template is itself empty, the template's style
/// references.
pub fn create_site(doc: &mut acadrust::CadDocument, name: &str) -> Res<CreatedSite> {
    if name.is_empty() {
        return Err("a site needs a name".into());
    }
    let mut l = site_list(doc)?;
    let template = *l.sites.first().ok_or("no site to clone (needs one site)")?;
    let t_cont = container_of(doc, template)?;
    let site = doc.allocate_handle().value();
    let cont = doc.allocate_handle().value();
    if cont != site + 1 {
        return Err("handles not consecutive".into());
    }
    let key = new_key(doc, &l);

    // Site record: empty tail, own key, name, no description; the graphics
    // (a cache) are dropped unless the template is itself empty.
    let mut r = rec(doc, SITE, template)?;
    let mut tail = site_tail(&r)?;
    if tail.flags != 99 {
        let (d, hb) = crate::aecc_build::strip_graphics(&r.data, r.handle_bits)
            .map_err(|e| format!("{e:?}"))?;
        (r.data, r.handle_bits) = (d, hb);
        tail = site_tail(&r)?;
    }
    tail.p = [0.0; 3];
    tail.flags = 99;
    tail.entries.clear();
    tail.a = 0;
    tail.rest = vec![1, 1, 1];
    let mut refs = refs_of(&r);
    if refs.len() != 11 {
        return Err("template site has parcels (13 references)".into());
    }
    refs[6].1 = cont;
    (r.data, r.handle_bits) = write_site_tail(&r, &tail, &refs)?;
    r.data = set_site_key(&r, key)?;
    (r.data, r.handle_bits) = set_site_strings(&r, name, Some(""))?;
    let (site_data, site_hb) =
        handle_refs::set_own_handle(&r.data, r.handle_bits, site).ok_or("site: own handle")?;

    // Container: no nodes, no slots, no face; the template's styles.
    let c = rec(doc, CONT, t_cont)?;
    let mut topo = Topology::parse(&c)?;
    let x = topo.tail[0];
    topo = Topology {
        node_next: 1,
        nodes: Vec::new(),
        edge_next: 1,
        edges: Vec::new(),
        face_next: 2,
        faces: Vec::new(),
        tail: std::iter::once(x).chain([1; 12]).chain([0; 6]).collect(),
        after_refs: Vec::new(),
        members: Vec::new(),
        face_refs: Vec::new(),
        ..topo
    };
    // `finish` writes own−1 against the template's handle: same relation.
    let (cd, chb) = topo.write(&c)?;
    let (cont_data, cont_hb) =
        handle_refs::set_own_handle(&cd, chb, cont).ok_or("container: own handle")?;

    let mut e = doc
        .entities()
        .find(|e| e.common().handle.value() == template)
        .cloned()
        .ok_or("template entity")?;
    if let EntityType::Unknown(u) = &mut e {
        u.common.handle = Handle::new(site);
        u.common.owner_handle = Handle::NULL;
        u.raw_dwg_data = Some(site_data);
        u.dwg_handle_bits = site_hb as i64;
    }
    doc.add_entity(e).map_err(|e| e.to_string())?;
    let Some(ObjectType::Unknown {
        type_name,
        raw_dxf_codes,
        raw_dwg_version,
        ..
    }) = doc.objects.get(&Handle::new(t_cont)).cloned()
    else {
        return Err("template container is not a raw object".into());
    };
    doc.objects.insert(
        Handle::new(cont),
        ObjectType::Unknown {
            type_name,
            handle: Handle::new(cont),
            owner: Handle::new(site),
            raw_dxf_codes,
            raw_dwg_data: Some(cont_data),
            raw_dwg_handle_bits: cont_hb as i64,
            raw_dwg_version,
        },
    );

    // Appended here; `write_site_list` puts the list in key order.
    l.keys.push(key);
    l.sites.push(site);
    l.names.push(name.to_string());
    l.last = site;
    write_site_list(doc, &l)?;
    Ok(CreatedSite {
        site,
        container: cont,
        key,
    })
}

/// Create an empty site named by one of Civil 3D's built-in names: the
/// ID as key and no name string, as Civil 3D stores such a name (MIST22;
/// S352/S353). Civil 3D shows the name it has for the ID; an ID it does
/// not know is untested.
pub fn create_site_with_builtin_name(
    doc: &mut acadrust::CadDocument,
    key: u32,
    description: &str,
) -> Res<CreatedSite> {
    if named(key) {
        return Err("a built-in name ID has no high bit".into());
    }
    if site_list(doc)?.keys.contains(&key) {
        return Err(format!("{key:08X} is already a site key").into());
    }
    let mut c = create_site(doc, "T")?;
    let site = c.site;
    let mut l = site_list(doc)?;
    let i = l.sites.iter().position(|h| *h == site).ok_or("new site")?;
    let ni = name_index(&l, i);
    l.names.remove(ni);
    l.keys[i] = key;
    let mut r = rec(doc, SITE, site)?;
    let n = refs_of(&r).len();
    let e = |e: string_stream::StringStreamError| e.to_string();
    (r.data, r.handle_bits) =
        string_stream::replace(&r.data, r.handle_bits, 3, description).map_err(e)?;
    (r.data, r.handle_bits) = string_stream::remove(&r.data, r.handle_bits, 2).map_err(e)?;
    (r.data, r.handle_bits) =
        handle_refs::keep_refs(&r.data, r.handle_bits, site, n).ok_or("site references")?;
    r.data = set_site_key(&r, key)?;
    put(doc, site, r.data, r.handle_bits)?;
    write_site_list(doc, &l)?;
    c.key = key;
    Ok(c)
}

/// Bit of the alignment's edge ID (`BL`): the ID of its edge in the site
/// container or companion, 133 bits after MIST1 and the optional MIST2
/// (98 bits later in the Alignment Properties form); checked against its
/// present value.
fn edge_id_bit(d: &[u8], id: i32) -> Res<usize> {
    use crate::aecc_build as b;
    let after_key =
        b::geometry_key_bit(d).ok_or("alignment: MIST1 not found")? + b::GEOMETRY_KEY_BITS;
    // 133 bits after MIST1 (and MIST2 when present): S324, S326, S344.
    let candidates: Vec<usize> = match b::resave_field_bit(d) {
        Some(q) => vec![q + 44 + 133],
        None => vec![after_key + 133, after_key + 98 + 133],
    };
    candidates
        .into_iter()
        .find(|&q| {
            raw::get_bit(d, q) == 0
                && raw::get_bit(d, q + 1) == 1
                && raw::read_bl(d, q).map(|x| x.0) == Some(id)
        })
        .ok_or(format!("alignment: edge ID {id} not where expected").into())
}

/// The alignment's two container references and its edge ID.
fn alignment_refs(
    doc: &mut acadrust::CadDocument,
    r: &RawRecord,
    target: u64,
    edge: (i32, i32),
) -> Res<()> {
    let mut t: Vec<u64> = refs_of(r).iter().map(|x| x.1).collect();
    t[2] = target;
    t[3] = target;
    if !(1..256).contains(&edge.1) {
        return Err("edge ID over 255 not handled".into());
    }
    let mut data = r.data.clone();
    let at = edge_id_bit(&data, edge.0)?;
    raw::set_byte(&mut data, at + 2, edge.1 as u8);
    let (d, hb) = handle_refs::rewrite_refs(&data, r.handle_bits, r.handle, &t)
        .ok_or("alignment references")?;
    put(doc, r.handle, d, hb)
}

/// Move a siteless alignment that crosses nothing into a site, as Civil 3D
/// does (S323; S332/S342, S345/S351): its edge goes to the site container
/// with new nodes and a loop of the site's face, the companion is emptied
/// and kept, the alignment's two container references and its edge ID
/// change, the site's extent point is refreshed. Refused when the
/// alignment would cross or touch a member, and for spirals.
pub fn move_into_site(doc: &mut acadrust::CadDocument, al: u64, site: u64) -> Res<Moved> {
    move_in(doc, al, site, true, PathFrom::Elements)
}

/// Where an experimental move takes the alignment's path from.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum PathFrom {
    /// Decoded from the alignment's elements (lines and arcs only).
    Elements,
    /// The edge of the alignment's own companion, vertex for vertex
    /// (spirals as Civil 3D's arc pieces; S368/S369).
    Companion,
    /// Given points and the bulge of the segment starting at each.
    Points(Vec<(f64, f64)>, Vec<Option<f64>>),
}

/// [`move_into_site`] for experiments (S370, S371): `check` = the
/// crossing test, `path` = where the edge's geometry comes from.
#[doc(hidden)]
pub fn move_into_site_with(
    doc: &mut acadrust::CadDocument,
    al: u64,
    site: u64,
    check: bool,
    path: PathFrom,
) -> Res<Moved> {
    move_in(doc, al, site, check, path)
}

/// [`move_into_site`] without the crossing test, for the crossing
/// experiments (`examples/exp_site_cross.rs`, S362/S363): the caller makes
/// the shared nodes and merges the loops. Not for use outside experiments.
#[doc(hidden)]
pub fn move_into_site_unchecked(doc: &mut acadrust::CadDocument, al: u64, site: u64) -> Res<Moved> {
    move_in(doc, al, site, false, PathFrom::Elements)
}

/// The companion's edge as points and bulges.
pub(crate) fn companion_path(t: &Topology) -> Res<Path> {
    let e = t.edges.iter().flatten().next().ok_or("empty companion")?;
    let mut pts = Vec::new();
    let mut bulges = Vec::new();
    for v in &e.vertices {
        let n = t
            .nodes
            .iter()
            .find(|n| n.0 == v.node)
            .ok_or("vertex without node")?;
        pts.push((n.1, n.2));
        bulges.push(v.bulge);
    }
    bulges.pop();
    Ok((pts, bulges))
}

fn move_in(
    doc: &mut acadrust::CadDocument,
    al: u64,
    site: u64,
    check: bool,
    from: PathFrom,
) -> Res<Moved> {
    let r = rec(doc, "AECC_ALIGNMENT", al)?;
    let comp_h = refs_of(&r)[2].1;
    let comp = rec(doc, CONT, comp_h)?;
    let mut ct = Topology::parse(&comp)?;
    if ct.owner.0 == 8 || ct.members != [al] {
        return Err("the alignment is not siteless".into());
    }
    let cont_h = container_of(doc, site)?;
    let cont = rec(doc, CONT, cont_h)?;
    let mut t = Topology::parse(&cont)?;
    if t.faces.len() > 1 {
        return Err(
            "the site has parcels: which face the alignment lies in is not worked out".into(),
        );
    }
    let (pts, bulges) = match from {
        PathFrom::Elements => alignment_path(&r)?,
        PathFrom::Companion => companion_path(&ct)?,
        PathFrom::Points(p, b) => (p, b),
    };
    // Crossing anything already in the site is not handled.
    let chords = |pts: &[(f64, f64)]| -> Vec<[(f64, f64); 2]> {
        pts.windows(2).map(|w| [w[0], w[1]]).collect()
    };
    let existing: Vec<[(f64, f64); 2]> = t
        .edges
        .iter()
        .flatten()
        .flat_map(|e| {
            let p: Vec<(f64, f64)> = e
                .vertices
                .iter()
                .filter_map(|v| t.nodes.iter().find(|n| n.0 == v.node))
                .map(|n| (n.1, n.2))
                .collect();
            chords(&p)
        })
        .collect();
    let arcs = bulges.iter().any(|b| b.is_some())
        || t.edges
            .iter()
            .flatten()
            .any(|e| e.vertices.iter().any(|v| v.bulge.is_some()));
    if check && !existing.is_empty() && arcs {
        return Err("arcs in a site that already has members: crossing test not written".into());
    }
    if check
        && chords(&pts)
            .iter()
            .any(|s| existing.iter().any(|o| segments_touch(*s, *o)))
    {
        return Err("the alignment crosses or touches a member of the site".into());
    }
    // Container: nodes, edge (as Civil 3D writes a moved alignment), loop.
    let first = t.add_edge(&pts, &bulges, 0x810, true);
    let ids = (
        ct.edges.iter().flatten().next().map_or(1, |e| e.id),
        t.edges.iter().flatten().last().map_or(1, |e| e.id),
    );
    t.members.push(al);
    match t.faces.first_mut() {
        Some(f) => f.loops.push((first, 0)),
        None => {
            t.faces.push(Face {
                id: 1,
                flags: 14,
                loops: vec![(first, 0)],
            });
            t.face_refs.push(site);
        }
    }
    let (d, hb) = t.write(&cont)?;
    put(doc, cont_h, d, hb)?;
    // Companion: emptied, kept in the registry.
    ct.nodes.clear();
    ct.edges.iter_mut().for_each(|e| *e = None);
    ct.members.clear();
    ct.faces.iter_mut().for_each(|f| f.loops.clear());
    let (d, hb) = ct.write(&comp)?;
    put(doc, comp_h, d, hb)?;
    alignment_refs(doc, &r, cont_h, ids)?;
    refresh_site(doc, site, &t)?;
    Ok(Moved {
        alignment: al,
        site,
        container: cont_h,
        companion: comp_h,
    })
}

/// Move an alignment out of its site, back to siteless (S333/S341,
/// S334/S343): its edge entry is freed, its nodes and loop go, and an
/// emptied companion of the registry is refilled with it (Civil 3D makes
/// a new companion; refilling is the smaller change and Civil 3D accepts
/// it). Refused when its edge shares a node, bounds a parcel or no
/// emptied companion exists.
pub fn move_out_of_site(doc: &mut acadrust::CadDocument, al: u64) -> Res<Moved> {
    let r = rec(doc, "AECC_ALIGNMENT", al)?;
    let cont_h = refs_of(&r)[2].1;
    let cont = rec(doc, CONT, cont_h)?;
    let mut t = Topology::parse(&cont)?;
    if t.owner.0 != 8 {
        return Err("the alignment is not in a site".into());
    }
    let site = cont_h - 1;
    let m = t
        .members
        .iter()
        .position(|h| *h == al)
        .ok_or("alignment not among the container's members")?;
    let slot = (0..t.edges.len())
        .find(|&s| t.edges[s].is_some() && t.member_index(s) == m)
        .ok_or("member slot")?;
    let old_id = t.edges[slot].as_ref().map_or(1, |e| e.id);
    t.remove_edge(slot)?;
    let (d, hb) = t.write(&cont)?;
    put(doc, cont_h, d, hb)?;
    // An emptied companion of the registry is refilled (Civil 3D makes a
    // new one and leaves the emptied ones; this is the smaller change).
    let (comp, mut ct) = raw::records(doc, CONT)
        .into_iter()
        .filter_map(|c| Topology::parse(&c).ok().map(|t| (c, t)))
        .find(|(_, t)| {
            t.owner.0 != 8 && t.styles.is_empty() && t.nodes.is_empty() && t.members.is_empty()
        })
        .ok_or("no emptied companion to refill")?;
    let (pts, bulges) = alignment_path(&r)?;
    let faced = !ct.faces.is_empty();
    let first = ct.add_edge(&pts, &bulges, if faced { 0x810 } else { 0x10 }, faced);
    ct.members.push(al);
    if let Some(f) = ct.faces.first_mut() {
        f.loops = vec![(first, 0)];
    }
    let (d, hb) = ct.write(&comp)?;
    put(doc, comp.handle, d, hb)?;
    let new_id = ct.edges.iter().flatten().last().map_or(1, |e| e.id);
    alignment_refs(doc, &r, comp.handle, (old_id, new_id))?;
    refresh_site(doc, site, &t)?;
    Ok(Moved {
        alignment: al,
        site,
        container: cont_h,
        companion: comp.handle,
    })
}

/// Sites, containers, companions and each alignment's edge, as text (for
/// reports and experiments).
pub fn describe(doc: &acadrust::CadDocument) -> Res<String> {
    let mut out = String::new();
    let l = site_list(doc)?;
    say!(
        out,
        "site list {:X}: keys {:08X?}, names {:?}, sites {:X?}, last {:X}",
        l.rec.handle,
        l.keys,
        l.names,
        l.sites,
        l.last
    );
    for r in raw::records(doc, SITE) {
        let s: Vec<String> = string_stream::parse(&r.data, r.handle_bits)
            .map(|s| s.strings.into_iter().map(|x| x.text).collect())
            .unwrap_or_default();
        match site_tail(&r) {
            Ok(t) => say!(
                out,
                "face {:X} {:?}: P {:?}, flags {}, {} entr., a {}, rest {:?}",
                r.handle,
                &s[2.min(s.len())..],
                t.p,
                t.flags,
                t.entries.len(),
                t.a,
                t.rest
            ),
            Err(e) => say!(out, "face {:X} {:?}: {e}", r.handle, s),
        }
    }
    let mut edge_of = std::collections::HashMap::new();
    for r in raw::records(doc, CONT) {
        if let Ok(t) = Topology::parse(&r) {
            for (e, m) in t.edges.iter().flatten().zip(&t.members) {
                edge_of.insert(*m, (r.handle, e.id));
            }
        }
    }
    for r in raw::records(doc, "AECC_ALIGNMENT") {
        match edge_of.get(&r.handle) {
            Some((c, id)) => say!(
                out,
                "alignment {:X}: edge {id} of {c:X}, edge ID field {}",
                r.handle,
                if edge_id_bit(&r.data, *id).is_ok() {
                    "matches"
                } else {
                    "DIFFERS"
                }
            ),
            None => say!(out, "alignment {:X}: in no container", r.handle),
        }
    }
    for r in raw::records(doc, CONT) {
        match Topology::parse(&r) {
            Ok(t) => say!(out, "{:X}: {}", r.handle, t.describe()),
            Err(e) => say!(out, "{:X}: {e}", r.handle),
        }
    }
    Ok(out)
}
