//! Feature lines (`AECC_FEATURE_LINE`, class name `AeccImpFeatureLine`) —
//! read-only (S313–S317; docs/feature-line-object.md).
//!
//! A feature line keeps no geometry of its own: its record holds the
//! description (twice), the name, a bounding-box placeholder as graphics,
//! and references (layer, null, the site container twice, null, the
//! feature line style). Its points are an **edge of the site's topology**,
//! the same graph that holds the site's alignments
//! ([`crate::aecc_topology`]): nodes with x, y and z (the elevation), an
//! edge through them, arcs as bulges (tan(Δ/4)) on the vertex where they
//! start. A feature line is therefore always in a site.

use acadrust::document::CadDocument;

use crate::aecc_site;
use crate::aecc_topology::{self, Graph};
use crate::handle_refs;
use crate::raw::{self, RawRecord};
use crate::string_stream;

/// Class of a feature line record.
pub const CLASS: &str = "AECC_FEATURE_LINE";
/// Vertex flag bit seen on an elevation point (a vertex added by "Insert
/// Elevation Point", S316: 0x400400).
pub const ELEVATION_POINT_FLAG: i32 = 0x40_0000;

/// One feature line record.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureLine {
    pub handle: u64,
    pub name: String,
    pub description: String,
    /// The site container (`AECC_FEATURE_STYLE`) whose graph holds the
    /// points.
    pub container: Option<u64>,
}

/// A feature line vertex: a topology node with the vertex's flags, and
/// the bulge (tan(Δ/4), positive to the left) of the arc that starts at
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeatureVertex {
    pub node: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub flags: i32,
    pub bulge: Option<f64>,
}

impl FeatureVertex {
    /// An elevation point (S316) rather than a PI.
    pub fn is_elevation_point(&self) -> bool {
        self.flags & ELEVATION_POINT_FLAG != 0
    }
}

/// Read a feature line record.
pub fn read(rec: &RawRecord) -> FeatureLine {
    let strings: Vec<String> = string_stream::parse(&rec.data, rec.handle_bits)
        .map(|s| s.strings.into_iter().map(|x| x.text).collect())
        .unwrap_or_default();
    let texts: Vec<&String> = strings.iter().skip(1).filter(|s| !s.is_empty()).collect();
    // Description twice, then the name; with no description, only the name.
    let (description, name) = match texts.as_slice() {
        [d, _, n, ..] => ((*d).clone(), (*n).clone()),
        [n] => (String::new(), (*n).clone()),
        _ => (
            String::new(),
            texts.last().map(|s| (*s).clone()).unwrap_or_default(),
        ),
    };
    let container = handle_refs::of_record(rec).get(2).map(|r| r.handle);
    FeatureLine {
        handle: rec.handle,
        name,
        description,
        container,
    }
}

/// The drawing's feature lines.
pub fn feature_lines(doc: &CadDocument) -> Vec<FeatureLine> {
    raw::records(doc, CLASS).iter().map(read).collect()
}

/// The site container's graph and the index of `line`'s edge in it. The
/// container lists its objects (alignments, feature lines) as soft
/// pointers in edge order, besides a soft pointer back to its site.
pub fn edge_of(doc: &CadDocument, line: &FeatureLine) -> Option<(Graph, usize)> {
    let c = line.container?;
    let rec = raw::records(doc, aecc_site::CONTAINER_CLASS)
        .into_iter()
        .find(|r| r.handle == c)?;
    let g = aecc_topology::graph(&rec.data)?;
    let members: Vec<u64> = handle_refs::of_record(&rec)
        .iter()
        .filter(|r| r.code == 4 && r.handle != 0)
        .filter(|r| raw::class_of(doc, r.handle).as_deref() != Some(aecc_site::SITE_CLASS))
        .map(|r| r.handle)
        .collect();
    let i = members.iter().position(|&h| h == line.handle)?;
    (i < g.edges.len()).then_some((g, i))
}

/// The feature line's vertices, in order.
pub fn vertices(doc: &CadDocument, line: &FeatureLine) -> Option<Vec<FeatureVertex>> {
    let (g, i) = edge_of(doc, line)?;
    g.edges[i]
        .vertices
        .iter()
        .map(|v| {
            let n = g.nodes.nodes.iter().find(|n| n.id == v.node)?;
            Some(FeatureVertex {
                node: n.id,
                x: n.x,
                y: n.y,
                z: n.z,
                flags: v.flags,
                bulge: v.bulge,
            })
        })
        .collect()
}
