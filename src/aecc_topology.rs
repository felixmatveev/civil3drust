//! The topology stored in alignment companions and site containers
//! (`AECC_FEATURE_STYLE` records; docs/alignment-object.md, "Topology
//! nodes"). Read-only, first part: the **node list**.
//!
//! Found in S25, S44, S48, S49, S102–S106 (companions and site
//! containers):
//!
//! ```text
//! BL next free node ID, BL node count n, BL ID of the first node,
//! n × { BD x, BD y, BD z, 3 bits 010, BL ID of the next node (0 = last) }
//! ```
//!
//! Nodes are the end points of the alignment's elements (start, PC/PT,
//! end), one per point; a site container holds the nodes of all its
//! alignments. IDs are allocated from a counter that keeps growing across
//! edits (S48 → S49: 39–43, then 12–18 after Civil 3D rebuilt the graph),
//! and the list follows the next-IDs, not the path order.
//!
//! After the nodes comes the **edge list**. An alignment is one edge, a
//! polyline through its nodes:
//!
//! ```text
//! BL next free edge ID (2), BL edge count (1), BL 0, BL first edge ID (1),
//! per edge: BL flags, BL vertex count n, BL 0,
//!   n × { BL kind, BL vertex flags, BL node ID, BL a, BL b,
//!         if kind = 1: BD bulge, BL 0 }
//! ```
//!
//! kind 0 = a tangent starts here, 1 = an arc starts here (bulge
//! tan(Δ/4), positive turning left), 2 = the last vertex. Edge flags:
//! 0x10, + 0x800 in S49/S106 (both with arcs, but S98's F9D has an arc
//! with 0x10 alone), + 0x20 in S25/S44. Vertex flags 10 with a = b = 0 in S48/S49; 2, 50 or 178 with
//! a = b = 1 in records Civil 3D made by other routes (S25, S44, S106) —
//! meanings unknown. A companion then ends with a constant-looking tail
//! (`BL 0 ×4, BL 2`, one face-like entry `BL 1, 1, 1, 7` + two BLs, `BL 0`
//! ×2, six `BL 1, BL 100` pairs, six `BL 0`) and its strings. Site
//! containers hold one edge per alignment, in the order of the
//! container's alignment references (S104, S112; [`graph`]).
//!
//! **Faces (S112).** Where alignments in a site cross, the crossing points
//! become nodes too (vertex flags 1072) and are vertices of both edges;
//! the vertex fields a and b are the IDs of the faces on the two sides of
//! the segment starting there (1 = the site's outer face; 0 at the last
//! vertex), and each closed region becomes a face and a parcel record
//! (`AECC_PARCEL`, referenced by the container after the site). The face
//! list after the edges (`BL next free face ID, BL count, BL 1, BL first
//! face ID`, then per face flags and boundary references) is only partly
//! read.
//!
//! [`rebuild_companion`] rewrites a siteless companion's nodes and edge
//! from an alignment's geometry, for companions in the uniform form of
//! S48/S49/S98 (vertex flags 10, a = b = 0), keeping their edge flags and
//! tail (D-040).

use crate::alignment::Point2;
use crate::raw;

/// One topology node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Node {
    pub id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Node {
    pub fn point(&self) -> Point2 {
        Point2::new(self.x, self.y)
    }
}

/// A decoded node list.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeList {
    /// Bit offset of the first BL (next free ID).
    pub start_bit: usize,
    /// Bit offset just after the last node.
    pub end_bit: usize,
    pub next_free_id: i32,
    pub nodes: Vec<Node>,
}

const NODE_FLAGS: [u8; 3] = [0, 1, 0];

/// Parse a node list at `bit`, checking its consistency: IDs positive and
/// below the next free ID, linked without repeats, the last next-ID 0,
/// finite coordinates, flags `010`.
pub fn node_list_at(data: &[u8], bit: usize) -> Option<NodeList> {
    let (next_free, p) = raw::read_bl(data, bit)?;
    let (n, p) = raw::read_bl(data, p)?;
    let (head, mut p) = raw::read_bl(data, p)?;
    // IDs are small counters; a loose check here lets a misaligned start
    // (a 4-byte BL read from the constant run) pass (S49).
    if !(1..=100_000).contains(&n) || next_free > 1_000_000 || n >= next_free {
        return None;
    }
    if head < 1 || head >= next_free {
        return None;
    }
    let mut nodes = Vec::with_capacity(n as usize);
    let mut id = head;
    for i in 0..n {
        let (x, q) = raw::read_bd(data, p)?;
        let (y, q) = raw::read_bd(data, q)?;
        let (z, q) = raw::read_bd(data, q)?;
        if ![x, y, z].iter().all(|v| v.is_finite() && v.abs() < 1e12) {
            return None;
        }
        if (0..3).any(|k| raw::get_bit(data, q + k) != NODE_FLAGS[k]) {
            return None;
        }
        let (next, q) = raw::read_bl(data, q + 3)?;
        let last = i + 1 == n;
        if last != (next == 0) || (!last && (next < 1 || next >= next_free)) {
            return None;
        }
        if nodes.iter().any(|m: &Node| m.id == id) {
            return None;
        }
        nodes.push(Node { id, x, y, z });
        id = next;
        p = q;
    }
    Some(NodeList {
        start_bit: bit,
        end_bit: p,
        next_free_id: next_free,
        nodes,
    })
}

/// Find the node list in a companion or site container record: the first
/// offset after the object header where [`node_list_at`] succeeds.
pub fn node_list(data: &[u8]) -> Option<NodeList> {
    let n = raw::bit_len(data);
    (66..n.saturating_sub(40)).find_map(|b| node_list_at(data, b))
}

/// One vertex of an edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// 0 tangent starts here, 1 arc starts here, 2 last vertex.
    pub kind: i32,
    pub flags: i32,
    pub node: i32,
    pub a: i32,
    pub b: i32,
    /// tan(Δ/4) of the arc starting here (kind 1), positive to the left.
    pub bulge: Option<f64>,
}

/// One edge (an alignment's polyline through the nodes).
#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub flags: i32,
    pub vertices: Vec<Vertex>,
}

/// A companion's decoded graph (single-edge records).
#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub nodes: NodeList,
    pub edges: Vec<Edge>,
    /// Bit offset just after the last edge.
    pub edges_end: usize,
}

fn edge_at(data: &[u8], bit: usize) -> Option<(Edge, usize)> {
    let (flags, p) = raw::read_bl(data, bit)?;
    let (n, p) = raw::read_bl(data, p)?;
    let (zero, mut p) = raw::read_bl(data, p)?;
    if !(1..=100_000).contains(&n) || zero != 0 {
        return None;
    }
    let mut vertices = Vec::with_capacity(n as usize);
    for i in 0..n {
        let (kind, q) = raw::read_bl(data, p)?;
        let (vflags, q) = raw::read_bl(data, q)?;
        let (node, q) = raw::read_bl(data, q)?;
        let (a, q) = raw::read_bl(data, q)?;
        let (b, mut q) = raw::read_bl(data, q)?;
        let last = i + 1 == n;
        if !(0..=2).contains(&kind) || last != (kind == 2) {
            return None;
        }
        let bulge = if kind == 1 {
            let (v, r) = raw::read_bd(data, q)?;
            let (z, r) = raw::read_bl(data, r)?;
            if !v.is_finite() || z != 0 {
                return None;
            }
            q = r;
            Some(v)
        } else {
            None
        };
        vertices.push(Vertex {
            kind,
            flags: vflags,
            node,
            a,
            b,
            bulge,
        });
        p = q;
    }
    Some((Edge { flags, vertices }, p))
}

/// Decode the graph of a companion or site container: node list, then
/// the edge list (`BL next free edge ID, BL count, BL 0, BL first edge
/// ID`), edges one after another. Each edge ends with `BL 0 ×4`; between
/// edges come `BL 0` and the next edge's ID (S104, S112).
pub fn graph(data: &[u8]) -> Option<Graph> {
    let nodes = node_list(data)?;
    let p = nodes.end_bit;
    let (next_free, p) = raw::read_bl(data, p)?;
    let (count, p) = raw::read_bl(data, p)?;
    let (zero, p) = raw::read_bl(data, p)?;
    let (head, mut p) = raw::read_bl(data, p)?;
    if !(1..=10_000).contains(&count) || count >= next_free || zero != 0 || head != 1 {
        return None;
    }
    let mut edges = Vec::new();
    let mut end = p;
    for k in 0..count {
        let (edge, q) = edge_at(data, p)?;
        if edge
            .vertices
            .iter()
            .any(|v| !nodes.nodes.iter().any(|n| n.id == v.node))
        {
            return None;
        }
        edges.push(edge);
        let mut q = q;
        for _ in 0..4 {
            let (z, r) = raw::read_bl(data, q)?;
            if z != 0 {
                return None;
            }
            q = r;
        }
        end = q;
        if k + 1 < count {
            let (z, r) = raw::read_bl(data, q)?;
            let (next, r) = raw::read_bl(data, r)?;
            if z != 0 || next < 1 || next >= next_free {
                return None;
            }
            q = r;
        }
        p = q;
    }
    Some(Graph {
        nodes,
        edges,
        edges_end: end,
    })
}

/// Decode a companion's graph: node list, then a one-edge edge list.
/// `edges_end` is the bit after the edge's vertices (before `BL 0 ×4`).
pub fn companion_graph(data: &[u8]) -> Option<Graph> {
    let nodes = node_list(data)?;
    let p = nodes.end_bit;
    let (next_free, p) = raw::read_bl(data, p)?;
    let (count, p) = raw::read_bl(data, p)?;
    let (zero, p) = raw::read_bl(data, p)?;
    let (head, p) = raw::read_bl(data, p)?;
    if (next_free, count, zero, head) != (2, 1, 0, 1) {
        return None;
    }
    let (edge, end) = edge_at(data, p)?;
    if edge
        .vertices
        .iter()
        .any(|v| !nodes.nodes.iter().any(|n| n.id == v.node))
    {
        return None;
    }
    Some(Graph {
        nodes,
        edges: vec![edge],
        edges_end: end,
    })
}

/// Points of an edge's vertices, in order.
pub fn edge_points(g: &Graph, e: &Edge) -> Vec<Point2> {
    e.vertices
        .iter()
        .filter_map(|v| g.nodes.nodes.iter().find(|n| n.id == v.node))
        .map(|n| n.point())
        .collect()
}

/// Vertex flags of the uniform form (S48, S49).
pub const VERTEX_PLAIN: i32 = 10;

/// Rewrite the graph of a siteless companion record (data + handle-stream
/// size) for an alignment made of `elements` (in path order): nodes at
/// the element end points (IDs 1… in path order), one edge through them
/// with a bulge per arc. The object header, key and constant run before
/// the nodes, the edge flags, the tail after the edge, the strings and
/// the handle stream are kept. Only companions in the uniform form
/// (vertex flags 10, a = b = 0) are rewritten; `None` otherwise. Returns
/// the new bytes and handle-stream size.
pub fn rebuild_companion(
    data: &[u8],
    handle_bits: usize,
    elements: &[crate::aecc_elements::ResolvedElement],
) -> Option<(Vec<u8>, usize)> {
    use crate::aecc_build::Bits;
    use crate::aecc_elements::Segment;
    if elements.is_empty() {
        return None;
    }
    // The old graph decides the parts we keep: edge flags and the tail
    // after the edge (its face-like entry goes with the number of handle
    // references; writing S49's tail into a three-reference companion made
    // Civil 3D discard it, S108 → S109).
    let old = companion_graph(data)?;
    let old_edge = old.edges.first()?;
    if old_edge
        .vertices
        .iter()
        .any(|v| (v.flags, v.a, v.b) != (VERTEX_PLAIN, 0, 0))
    {
        return None; // other vertex forms: meaning unknown
    }
    let nodes = old.nodes.clone();
    let ss = crate::string_stream::parse(data, handle_bits).ok()?;
    if ss.start <= nodes.start_bit {
        return None;
    }
    // Vertices: each element's start, then the last element's end.
    let mut pts: Vec<Point2> = elements.iter().map(|e| e.segment.start()).collect();
    pts.push(elements.last()?.segment.end());
    let n = pts.len() as i32;
    let mut w = Bits::default();
    w.copy(data, 0, nodes.start_bit);
    w.bl(n + 1);
    w.bl(n);
    w.bl(1);
    for (i, p) in pts.iter().enumerate() {
        w.bd(p.x);
        w.bd(p.y);
        w.bd(0.0);
        w.bits(0b010, 3);
        w.bl(if i as i32 + 1 == n { 0 } else { i as i32 + 2 });
    }
    for v in [2, 1, 0, 1] {
        w.bl(v);
    }
    // Edge flags as before: S98's companion has an arc with flags 0x10, so
    // 0x800 is not simply "has arcs".
    w.bl(old_edge.flags);
    w.bl(n);
    w.bl(0);
    for i in 0..n as usize {
        let arc = elements.get(i).and_then(|e| match e.segment {
            Segment::Arc { geometry, .. } => Some(geometry),
            _ => None,
        });
        let kind = if i + 1 == n as usize {
            2
        } else if arc.is_some() {
            1
        } else {
            0
        };
        w.bl(kind);
        w.bl(VERTEX_PLAIN);
        w.bl(i as i32 + 1);
        w.bl(0);
        w.bl(0);
        if let Some(g) = arc {
            let b = (g.sweep / 4.0).tan();
            w.bd(if g.counter_clockwise { b } else { -b });
            w.bl(0);
        }
    }
    // Tail as it was.
    w.copy(data, old.edges_end, ss.start);
    let total = raw::bit_len(data);
    let data_end = total - handle_bits;
    let new_data_end = data_end + w.len() - ss.start;
    w.copy(data, ss.start, total);
    let bytes = w.into_bytes();
    let hb = bytes.len() * 8 - new_data_end;
    Some((bytes, hb))
}

/// Rewrite the companion of the siteless alignment `alignment` in `doc`
/// from the alignment's geometry ([`rebuild_companion`]). Returns
/// `Ok(false)` when the alignment is in a site (its graph is the site's)
/// or its companion has no graph of the known form, which is then left as
/// it is.
pub fn refresh_companion(
    doc: &mut acadrust::document::CadDocument,
    alignment: u64,
) -> Result<bool, String> {
    use crate::aecc_site::{membership, Membership};
    let rec = raw::records(doc, "AECC_ALIGNMENT")
        .into_iter()
        .find(|r| r.handle == alignment)
        .ok_or_else(|| format!("no alignment {alignment:X}"))?;
    let Membership::Siteless { companion } = membership(doc, &rec) else {
        return Ok(false);
    };
    let Ok(d) = crate::aecc_elements::decode_elements(&rec) else {
        // No elements (an empty alignment, S113) or not decodable: leave
        // the companion as it is.
        return Ok(false);
    };
    let comp = raw::records(doc, "AECC_FEATURE_STYLE")
        .into_iter()
        .find(|r| r.handle == companion)
        .ok_or_else(|| format!("no companion {companion:X}"))?;
    if companion_graph(&comp.data).is_none() {
        return Ok(false);
    }
    let Some((bytes, hb)) = rebuild_companion(&comp.data, comp.handle_bits, &d.elements) else {
        return Ok(false);
    };
    let (bytes, hb) = crate::handle_refs::trim_padding(&bytes, hb, companion);
    if !raw::set_record(doc, companion, bytes, hb) {
        return Err(format!("could not write companion {companion:X}"));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aecc_build::Bits;

    #[test]
    fn parses_a_synthetic_list() {
        let mut w = Bits::default();
        w.bits(0b1011, 4); // filler
        w.bl(4);
        w.bl(2);
        w.bl(3);
        for (x, next) in [(10.0, 1), (20.0, 0)] {
            w.bd(x);
            w.bd(0.0);
            w.bd(0.0);
            w.bits(0b010, 3);
            w.bl(next);
        }
        let b = w.into_bytes();
        let l = node_list_at(&b, 4).unwrap();
        assert_eq!(l.next_free_id, 4);
        let ids: Vec<i32> = l.nodes.iter().map(|n| n.id).collect();
        assert_eq!(ids, [3, 1]);
        assert_eq!(l.nodes[1].x, 20.0);
    }
}
