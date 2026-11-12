//! Site write experiments (docs/site-object.md): first writers for the
//! site record, the site list and the site container, to be proved in
//! Civil 3D before they move into the library.
//!
//! Usage: cargo run --release --example exp_site -- <command> in.dwg out.dwg args...
//!
//!   rename   <site hex> <new name> [description]   name (and description) strings
//!   create   <name>                                 a new empty site, cloned from the first site
//!   delete   <site hex>                             an empty site, its container and list entry
//!   move-in  <alignment hex> <site hex>             a siteless alignment that crosses nothing
//!   move-out <alignment hex>                        back to siteless (an emptied companion is refilled)
//!   show                                            print sites, containers and companions (no out.dwg)

use acadrust::entities::EntityType;
use acadrust::objects::ObjectType;
use acadrust::Handle;
use civil3drust::aecc_build::Bits;
use civil3drust::aecc_elements::{decode_elements, Segment};
use civil3drust::raw::{self, RawRecord};
use civil3drust::{handle_refs, proxy_graphics, string_stream};

type Res<T> = Result<T, String>;
/// Points of a path and the bulge of the segment starting at each.
type Path = (Vec<(f64, f64)>, Vec<Option<f64>>);

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

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("error: {m}");
    std::process::exit(1);
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or_else(|_| fail(format!("not a handle: {s}")))
}

fn rec(doc: &acadrust::CadDocument, class: &str, h: u64) -> Res<RawRecord> {
    raw::records(doc, class)
        .into_iter()
        .find(|r| r.handle == h)
        .ok_or(format!("no {class} {h:X}"))
}

fn put(doc: &mut acadrust::CadDocument, h: u64, data: Vec<u8>, hb: usize) -> Res<()> {
    let (data, hb) = handle_refs::trim_padding(&data, hb, h);
    if raw::set_record(doc, h, data, hb) {
        Ok(())
    } else {
        Err(format!("could not write {h:X}"))
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

#[derive(Clone, Debug)]
struct Vertex {
    kind: i32,
    flags: i32,
    node: i32,
    a: i32,
    b: i32,
    bulge: Option<f64>,
}

#[derive(Clone, Debug)]
struct Edge {
    id: i32,
    flags: i32,
    vertices: Vec<Vertex>,
    after: Vec<i32>,
}

#[derive(Clone, Debug)]
struct Face {
    id: i32,
    flags: i32,
    loops: Vec<(i32, i32)>,
}

/// A container or companion record, decoded to its last value.
#[derive(Clone, Debug)]
struct Topo {
    head_end: usize,
    node_next: i32,
    nodes: Vec<(i32, f64, f64, f64)>,
    edge_next: i32,
    edges: Vec<Option<Edge>>,
    face_next: i32,
    faces: Vec<Face>,
    /// X, six counter pairs, six lists (as stored).
    tail: Vec<i32>,
    /// Handle stream: owner; hard nulls of `after` values; members of the
    /// live edges; face objects; style references.
    owner: (u8, u64),
    after_refs: Vec<(u8, u64)>,
    members: Vec<u64>,
    face_refs: Vec<u64>,
    styles: Vec<(u8, u64)>,
}

impl Topo {
    fn parse(r: &RawRecord) -> Res<Topo> {
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
                s => return Err(format!("edge slot state {s}")),
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
            return Err(format!("{} references do not fit the graph", refs.len()));
        }
        let m0 = 1 + after;
        let f0 = m0 + 2 * live;
        Ok(Topo {
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

    fn write(&self, old: &RawRecord) -> Res<(Vec<u8>, usize)> {
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
    fn member_index(&self, slot: usize) -> usize {
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

    fn describe(&self) -> String {
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
            _ => return Err("spirals are not handled by this experiment".into()),
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
    let eed = civil3drust::aecc_entity::eed_bit(d).ok_or("site: object header")?;
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
fn refresh_site(doc: &mut acadrust::CadDocument, site: u64, topo: &Topo) -> Res<()> {
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
        .ok_or(format!("{site:X} is not a site (no container)"))
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

fn write_site_list(doc: &mut acadrust::CadDocument, l: &SiteList) -> Res<()> {
    let d = &l.rec.data;
    let ss = string_stream::parse(d, l.rec.handle_bits).map_err(|e| e.to_string())?;
    let mut w = Bits::default();
    w.copy(d, 0, l.list_bit);
    w.bl(0);
    w.bl(l.keys.len() as i32);
    for k in &l.keys {
        w.bits(0, 2);
        k.to_le_bytes().iter().for_each(|b| w.byte(*b));
    }
    w.copy(d, ss.start - LIST_END, ss.start);
    let mut refs = vec![l.tree];
    refs.extend(l.sites.iter().map(|h| (4, *h)));
    refs.push((4, l.last));
    refs.extend(&l.styles);
    let (mut data, mut hb) = finish(w, &l.rec, &refs)?;
    // Strings: "" + the names of the named sites, in list order.
    let old = ss.strings.len() - 1;
    for i in (l.names.len()..old).rev() {
        (data, hb) = string_stream::remove(&data, hb, i + 1).map_err(|e| e.to_string())?;
    }
    for i in old..l.names.len() {
        (data, hb) = string_stream::insert_empty(&data, hb, i + 1).map_err(|e| e.to_string())?;
    }
    for (i, n) in l.names.iter().enumerate() {
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
        .filter_map(|r| civil3drust::aecc_registry::Registry::parse(&r).ok())
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
        n => return Err(format!("site record with {n} strings")),
    }
    (d, hb) = string_stream::replace(&d, hb, 2, name).map_err(e)?;
    if let Some(t) = description {
        (d, hb) = string_stream::replace(&d, hb, 3, t).map_err(e)?;
    }
    // Civil 3D pads the handle stream with 1-bits, which can read as one
    // more reference once the record's length changes.
    handle_refs::keep_refs(&d, hb, r.handle, refs_of(r).len()).ok_or("site references".into())
}

/// Where a site called `name` goes in the list: by name, spaces ignored,
/// unnamed sites first.
fn list_position(l: &SiteList, name: &str) -> usize {
    let norm = |s: &str| s.replace(' ', "");
    let mut names = l.names.iter();
    l.keys
        .iter()
        .position(|k| named(*k) && names.next().is_some_and(|n| norm(n) > norm(name)))
        .unwrap_or(l.keys.len())
}

// ----------------------------------------------------------------- commands

fn cmd_rename(doc: &mut acadrust::CadDocument, a: &[String]) -> Res<()> {
    let site = hex(&a[0]);
    let mut l = site_list(doc)?;
    let i = l
        .sites
        .iter()
        .position(|h| *h == site)
        .ok_or("site not in the site list")?;
    let mut r = rec(doc, SITE, site)?;
    let ni = name_index(&l, i);
    if named(l.keys[i]) {
        l.names[ni] = a[1].clone();
    } else {
        l.keys[i] = new_key(doc, &l);
        l.names.insert(ni, a[1].clone());
        r.data = set_site_key(&r, l.keys[i])?;
        println!("unnamed site: key {:08X}", l.keys[i]);
    }
    let (d, hb) = set_site_strings(&r, &a[1], a.get(2).map(String::as_str))?;
    put(doc, site, d, hb)?;
    write_site_list(doc, &l)
}

fn cmd_delete(doc: &mut acadrust::CadDocument, a: &[String]) -> Res<()> {
    let site = hex(&a[0]);
    let cont = container_of(doc, site)?;
    let t = Topo::parse(&rec(doc, CONT, cont)?)?;
    if t.edges.iter().any(|e| e.is_some()) {
        return Err("the site has members (not handled yet)".into());
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

fn cmd_create(doc: &mut acadrust::CadDocument, a: &[String]) -> Res<()> {
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
        let (d, hb) = civil3drust::aecc_build::strip_graphics(&r.data, r.handle_bits)
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
    (r.data, r.handle_bits) = set_site_strings(&r, &a[0], Some(""))?;
    let (site_data, site_hb) =
        handle_refs::set_own_handle(&r.data, r.handle_bits, site).ok_or("site: own handle")?;

    // Container: no nodes, no slots, no face; the template's styles.
    let c = rec(doc, CONT, t_cont)?;
    let mut topo = Topo::parse(&c)?;
    let x = topo.tail[0];
    topo = Topo {
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

    let i = list_position(&l, &a[0]);
    let ni = name_index(&l, i);
    l.keys.insert(i, key);
    l.sites.insert(i, site);
    l.names.insert(ni, a[0].clone());
    l.last = site;
    write_site_list(doc, &l)?;
    println!("site {site:X}, container {cont:X}, key {key:08X}");
    Ok(())
}

fn alignment_refs(doc: &mut acadrust::CadDocument, r: &RawRecord, target: u64) -> Res<()> {
    let mut t: Vec<u64> = refs_of(r).iter().map(|x| x.1).collect();
    t[2] = target;
    t[3] = target;
    let (d, hb) = handle_refs::rewrite_refs(&r.data, r.handle_bits, r.handle, &t)
        .ok_or("alignment references")?;
    put(doc, r.handle, d, hb)
}

fn cmd_move_in(doc: &mut acadrust::CadDocument, a: &[String]) -> Res<()> {
    let (al, site) = (hex(&a[0]), hex(&a[1]));
    let r = rec(doc, "AECC_ALIGNMENT", al)?;
    let comp_h = refs_of(&r)[2].1;
    let comp = rec(doc, CONT, comp_h)?;
    let mut ct = Topo::parse(&comp)?;
    if ct.owner.0 == 8 || ct.members != [al] {
        return Err("the alignment is not siteless".into());
    }
    let cont_h = container_of(doc, site)?;
    let cont = rec(doc, CONT, cont_h)?;
    let mut t = Topo::parse(&cont)?;
    let (pts, bulges) = alignment_path(&r)?;
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
    if !existing.is_empty() && arcs {
        return Err("arcs in a site that already has members: crossing test not written".into());
    }
    if chords(&pts)
        .iter()
        .any(|s| existing.iter().any(|o| segments_touch(*s, *o)))
    {
        return Err("the alignment crosses or touches a member of the site".into());
    }
    // Container: nodes, edge (as Civil 3D writes a moved alignment), loop.
    let first = t.add_edge(&pts, &bulges, 0x810, true);
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
    alignment_refs(doc, &r, cont_h)?;
    refresh_site(doc, site, &t)?;
    println!("{al:X} → site {site:X} (container {cont_h:X}); companion {comp_h:X} emptied");
    Ok(())
}

fn cmd_move_out(doc: &mut acadrust::CadDocument, a: &[String]) -> Res<()> {
    let al = hex(&a[0]);
    let r = rec(doc, "AECC_ALIGNMENT", al)?;
    let cont_h = refs_of(&r)[2].1;
    let cont = rec(doc, CONT, cont_h)?;
    let mut t = Topo::parse(&cont)?;
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
    t.remove_edge(slot)?;
    let (d, hb) = t.write(&cont)?;
    put(doc, cont_h, d, hb)?;
    // An emptied companion of the registry is refilled (Civil 3D makes a
    // new one and leaves the emptied ones; this is the smaller change).
    let (comp, mut ct) = raw::records(doc, CONT)
        .into_iter()
        .filter_map(|c| Topo::parse(&c).ok().map(|t| (c, t)))
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
    alignment_refs(doc, &r, comp.handle)?;
    refresh_site(doc, site, &t)?;
    println!(
        "{al:X} → siteless (companion {:X} refilled); slot {} of {cont_h:X} freed",
        comp.handle,
        slot + 1
    );
    Ok(())
}

fn show(doc: &acadrust::CadDocument) -> Res<()> {
    let l = site_list(doc)?;
    println!(
        "site list {:X}: keys {:08X?}, names {:?}, sites {:X?}, last {:X}",
        l.rec.handle, l.keys, l.names, l.sites, l.last
    );
    for r in raw::records(doc, SITE) {
        let s: Vec<String> = string_stream::parse(&r.data, r.handle_bits)
            .map(|s| s.strings.into_iter().map(|x| x.text).collect())
            .unwrap_or_default();
        match site_tail(&r) {
            Ok(t) => println!(
                "face {:X} {:?}: P {:?}, flags {}, {} entr., a {}, rest {:?}",
                r.handle,
                &s[2.min(s.len())..],
                t.p,
                t.flags,
                t.entries.len(),
                t.a,
                t.rest
            ),
            Err(e) => println!("face {:X} {:?}: {e}", r.handle, s),
        }
    }
    for r in raw::records(doc, CONT) {
        match Topo::parse(&r) {
            Ok(t) => println!("{:X}: {}", r.handle, t.describe()),
            Err(e) => println!("{:X}: {e}", r.handle),
        }
    }
    Ok(())
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        fail(
            "usage: exp_site <rename|create|delete|move-in|move-out|show> in.dwg [out.dwg args...]",
        );
    }
    let mut o = raw::open(&a[2]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &o.failsafe_reason {
        fail(format!("read in failsafe mode ({r})"));
    }
    if a[1] == "show" {
        return show(&o.doc).unwrap_or_else(|e| fail(e));
    }
    let args = a.get(4..).unwrap_or_default();
    let need = |n: usize| {
        if args.len() < n {
            fail(format!("{}: {n} argument(s) expected", a[1]));
        }
    };
    let r = match a[1].as_str() {
        "rename" => {
            need(2);
            cmd_rename(&mut o.doc, args)
        }
        "create" => {
            need(1);
            cmd_create(&mut o.doc, args)
        }
        "delete" => {
            need(1);
            cmd_delete(&mut o.doc, args)
        }
        "move-in" => {
            need(2);
            cmd_move_in(&mut o.doc, args)
        }
        "move-out" => {
            need(1);
            cmd_move_out(&mut o.doc, args)
        }
        c => fail(format!("unknown command {c}")),
    };
    r.unwrap_or_else(|e| fail(e));
    acadrust::DwgWriter::write_to_file(&a[3], &o.doc).unwrap_or_else(|e| fail(e));
    println!("wrote {}", a[3]);
}
