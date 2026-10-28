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
//! and the list follows the next-IDs, not the path order. What follows the
//! node list (edges, faces) is not decoded yet.

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
    if !(1..=100_000).contains(&n) || head < 1 || head >= next_free {
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
