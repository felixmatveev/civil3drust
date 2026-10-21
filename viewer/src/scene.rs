//! Everything the viewer draws, extracted from a DWG once at load time.
//!
//! Three layers of geometry, each with its own visibility switch:
//!
//! * **Decoded** — Civil 3D objects civil3drust understands (alignments for
//!   now), drawn from the decoded element records, i.e. from civil3drust's
//!   own reading of the format.
//! * **Proxy graphics** — the display cache Civil 3D stores inside every
//!   AECC entity, walked with `civil3drust::proxy_graphics`. This is what
//!   Civil 3D itself drew last time it saved the file, so overlaying it on
//!   the decoded geometry is a visual check of the decoder.
//! * **Context** — plain AutoCAD entities in model space (lines, polylines,
//!   circles, arcs), drawn in grey so the Civil 3D objects have something
//!   around them.

use acadrust::entities::EntityType;
use civil3drust::aecc_elements::{decode_elements, Segment};
use civil3drust::{proxy_graphics as pg, raw};
use std::collections::BTreeMap;
use std::f64::consts::TAU;
use std::path::Path;

pub type P3 = [f64; 3];

/// One drawable primitive in world coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Prim {
    /// Connected line segments.
    Line { pts: Vec<P3>, closed: bool },
    /// A point marker (PI, start/end), drawn at a fixed screen size.
    Marker { at: P3 },
    /// Text in world units: baseline start, direction (unit, in the
    /// drawing plane) and height.
    Text {
        at: P3,
        dir: P3,
        height: f64,
        text: String,
    },
}

/// A primitive with an AutoCAD color index (`None` = the layer's default).
#[derive(Debug, Clone, PartialEq)]
pub struct Colored {
    pub prim: Prim,
    pub aci: Option<i32>,
}

/// The decoded display cache of one entity.
#[derive(Debug, Clone, Default)]
pub struct ProxyGfx {
    pub prims: Vec<Colored>,
    /// Civil 3D's "not drawn yet" image (a class-name text and a box).
    pub placeholder: bool,
    pub chunk_count: usize,
    /// Opcodes walked but not drawn, with counts (attribute opcodes
    /// such as layer and linetype included).
    pub skipped: BTreeMap<i32, usize>,
}

/// What civil3drust decoded from the record, if anything.
#[derive(Debug, Clone)]
pub enum Decoded {
    /// Not a class civil3drust decodes yet.
    None,
    /// Decoding was tried and failed.
    Failed(String),
    Alignment(AlignmentView),
}

#[derive(Debug, Clone)]
pub struct AlignmentView {
    pub name: Option<String>,
    pub start_station: f64,
    pub length: f64,
    pub tangents: usize,
    pub arcs: usize,
    /// Geometry: path, element-boundary markers, station ticks and labels.
    pub prims: Vec<Prim>,
}

/// One custom (non-AutoCAD) entity in model space: an AECC object, or any
/// other class acadrust keeps as unknown.
#[derive(Debug, Clone)]
pub struct Object {
    pub handle: u64,
    pub class: String,
    pub name: Option<String>,
    pub decoded: Decoded,
    pub proxy: Option<ProxyGfx>,
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub path: String,
    pub failsafe: Option<String>,
    pub objects: Vec<Object>,
    /// Plain AutoCAD entities in model space.
    pub context: Vec<Colored>,
    /// Model-space entities of other kinds, by type, not drawn.
    pub context_skipped: BTreeMap<String, usize>,
}

impl Scene {
    pub fn load(path: impl AsRef<Path>) -> Result<Scene, String> {
        let path = path.as_ref();
        let opened = raw::open(path).map_err(|e| e.to_string())?;
        let doc = &opened.doc;
        let mut scene = Scene {
            path: path.display().to_string(),
            failsafe: opened.failsafe_reason.clone(),
            ..Default::default()
        };
        // Decoded alignments, by handle.
        let mut alignments: BTreeMap<u64, Decoded> = BTreeMap::new();
        for rec in raw::records(doc, "AECC_ALIGNMENT") {
            let d = match decode_elements(&rec) {
                Ok(d) => Decoded::Alignment(alignment_view(&d)),
                Err(e) => Decoded::Failed(e.to_string()),
            };
            alignments.insert(rec.handle, d);
        }
        for e in doc.model_space_entities() {
            match e {
                EntityType::Unknown(u) => {
                    let Some(data) = &u.raw_dwg_data else {
                        continue;
                    };
                    let handle = u.common.handle.value();
                    let decoded = alignments.remove(&handle).unwrap_or(Decoded::None);
                    let name = match &decoded {
                        Decoded::Alignment(a) => a.name.clone(),
                        _ => raw::object_name(data),
                    };
                    scene.objects.push(Object {
                        handle,
                        class: u.dxf_name.clone(),
                        name,
                        decoded,
                        proxy: pg::find(data, 8192).map(|g| proxy_prims(data, &g)),
                    });
                }
                other => context_prims(other, &mut scene),
            }
        }
        scene
            .objects
            .sort_by(|a, b| a.class.cmp(&b.class).then(a.handle.cmp(&b.handle)));
        Ok(scene)
    }

    /// Classes present, with object counts.
    pub fn classes(&self) -> BTreeMap<String, usize> {
        let mut m = BTreeMap::new();
        for o in &self.objects {
            *m.entry(o.class.clone()).or_default() += 1;
        }
        m
    }
}

/// Points along an arc from `start`, `sweep` radians (positive = left).
fn arc_points(center: P3, radius: f64, start_angle: f64, sweep: f64) -> Vec<P3> {
    let n = ((sweep.abs() / TAU * 96.0).ceil() as usize).clamp(4, 256);
    (0..=n)
        .map(|i| {
            let a = start_angle + sweep * i as f64 / n as f64;
            [
                center[0] + radius * a.cos(),
                center[1] + radius * a.sin(),
                center[2],
            ]
        })
        .collect()
}

fn p3(p: civil3drust::Point2) -> P3 {
    [p.x, p.y, 0.0]
}

/// Station as Civil 3D shows it in US units: `12+34.56`.
pub fn station_text(sta: f64) -> String {
    let sign = if sta < 0.0 { "-" } else { "" };
    let s = sta.abs();
    let hundreds = (s / 100.0).floor();
    format!("{sign}{}+{:05.2}", hundreds as i64, s - hundreds * 100.0)
}

fn alignment_view(d: &civil3drust::aecc_elements::DecodedElements) -> AlignmentView {
    let start = d.stored_start_station.unwrap_or(0.0);
    let length = d.length();
    let mut path: Vec<P3> = Vec::new();
    let mut prims = Vec::new();
    let mut arcs = 0;
    for e in &d.elements {
        let pts = match e.segment {
            Segment::Line { start, end } => vec![p3(start), p3(end)],
            Segment::Arc {
                start, geometry: g, ..
            } => {
                arcs += 1;
                let a0 = (start.y - g.center.y).atan2(start.x - g.center.x);
                let sweep = if g.counter_clockwise {
                    g.sweep
                } else {
                    -g.sweep
                };
                arc_points(p3(g.center), g.radius, a0, sweep)
            }
        };
        if path.is_empty() {
            path.extend(pts);
        } else {
            path.extend(pts.into_iter().skip(1));
        }
        prims.push(Prim::Marker {
            at: p3(e.segment.start()),
        });
    }
    if let Some(last) = d.elements.last() {
        prims.push(Prim::Marker {
            at: p3(last.segment.end()),
        });
    }
    // Station ticks every 100 units (every 10 if the alignment is short),
    // perpendicular to the path, with labels on the left.
    let step = [
        10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0,
    ]
    .into_iter()
    .find(|s| length / s <= 12.0)
    .unwrap_or(10000.0);
    let tick = (length / 60.0).clamp(0.5, 10.0);
    let mut sta = (start / step).ceil() * step;
    while sta <= start + length + 1e-9 {
        if let Some((p, dir)) = point_on_path(&path, sta - start) {
            let left = [-dir[1], dir[0], 0.0];
            let a = [p[0] - left[0] * tick, p[1] - left[1] * tick, 0.0];
            let b = [p[0] + left[0] * tick, p[1] + left[1] * tick, 0.0];
            prims.push(Prim::Line {
                pts: vec![a, b],
                closed: false,
            });
            prims.push(Prim::Text {
                at: [
                    b[0] + left[0] * tick * 0.3,
                    b[1] + left[1] * tick * 0.3,
                    0.0,
                ],
                dir: left,
                height: tick,
                text: station_text(sta),
            });
        }
        sta += step;
    }
    // Direction arrow at mid-length.
    if let Some((p, dir)) = point_on_path(&path, length / 2.0) {
        let s = tick * 1.5;
        let left = [-dir[1], dir[0]];
        let back = [p[0] - dir[0] * s, p[1] - dir[1] * s];
        prims.push(Prim::Line {
            pts: vec![
                [
                    back[0] + left[0] * s * 0.5,
                    back[1] + left[1] * s * 0.5,
                    0.0,
                ],
                p,
                [
                    back[0] - left[0] * s * 0.5,
                    back[1] - left[1] * s * 0.5,
                    0.0,
                ],
            ],
            closed: false,
        });
    }
    prims.insert(
        0,
        Prim::Line {
            pts: path,
            closed: false,
        },
    );
    AlignmentView {
        name: d.name.clone(),
        start_station: start,
        length,
        tangents: d.elements.len() - arcs,
        arcs,
        prims,
    }
}

/// Point and unit direction at distance `s` along a polyline.
fn point_on_path(path: &[P3], s: f64) -> Option<(P3, P3)> {
    let mut left = s;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        if len <= 0.0 {
            continue;
        }
        if left <= len + 1e-9 {
            let t = (left / len).clamp(0.0, 1.0);
            let dir = [(b[0] - a[0]) / len, (b[1] - a[1]) / len, 0.0];
            return Some((
                [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, 0.0],
                dir,
            ));
        }
        left -= len;
    }
    None
}

// ---------------------------------------------------------------------------
// Proxy graphics

fn get3(buf: &[u8], bit: usize) -> P3 {
    [
        raw::get_f64(buf, bit),
        raw::get_f64(buf, bit + 64),
        raw::get_f64(buf, bit + 128),
    ]
}

/// UTF-16LE string starting at `bit`, up to a NUL or `max_bytes`.
/// Returns the string and its byte length including the NUL.
fn utf16_at(buf: &[u8], bit: usize, max_bytes: usize) -> (String, usize) {
    let mut units = Vec::new();
    let mut i = 0;
    while i + 2 <= max_bytes {
        let u = u16::from_le_bytes([
            raw::get_byte(buf, bit + i * 8),
            raw::get_byte(buf, bit + i * 8 + 8),
        ]);
        i += 2;
        if u == 0 {
            break;
        }
        units.push(u);
    }
    (String::from_utf16_lossy(&units), i)
}

fn finite(p: &P3) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() < 1e12)
}

/// Unit vector `u` such that (u, n × u) spans the plane with normal `n`
/// (AutoCAD's arbitrary-axis algorithm, from the public DXF reference).
fn ocs_x(n: P3) -> P3 {
    let ax = if n[0].abs() < 1.0 / 64.0 && n[1].abs() < 1.0 / 64.0 {
        [n[2], 0.0, -n[0]] // Wy × N
    } else {
        [-n[1], n[0], 0.0] // Wz × N
    };
    norm(ax)
}

fn norm(v: P3) -> P3 {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [1.0, 0.0, 0.0]
    }
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Circle or arc in the plane with normal `n`, starting at direction
/// `start` (in that plane), `sweep` radians counter-clockwise about `n`.
fn circle3d(c: P3, r: f64, n: P3, start: P3, sweep: f64) -> Vec<P3> {
    let n = norm(n);
    let u = norm(start);
    let v = cross(n, u);
    let k = ((sweep.abs() / TAU * 96.0).ceil() as usize).clamp(8, 256);
    (0..=k)
        .map(|i| {
            let a = sweep * i as f64 / k as f64;
            let (s, co) = a.sin_cos();
            [
                c[0] + r * (u[0] * co + v[0] * s),
                c[1] + r * (u[1] * co + v[1] * s),
                c[2] + r * (u[2] * co + v[2] * s),
            ]
        })
        .collect()
}

/// Opcodes that only set attributes for following primitives.
const ATTRIBUTE_OPS: &[i32] = &[14, 16, 18, 20, 21, 22, 23, 24, 25, 26, 34];

/// Walk a display cache into primitives. Layouts follow the public ODA
/// specification; see docs/viewer.md for which were checked on samples.
pub fn proxy_prims(buf: &[u8], g: &pg::Graphics) -> ProxyGfx {
    let mut out = ProxyGfx {
        placeholder: g.is_placeholder(),
        chunk_count: g.chunks.len(),
        ..Default::default()
    };
    let mut aci: Option<i32> = None;
    for c in &g.chunks {
        let b = c.body();
        let body_bytes = c.size - 8;
        let mut push = |prim: Prim, aci: Option<i32>| out.prims.push(Colored { prim, aci });
        let mut drawn = true;
        match c.opcode {
            pg::OP_POLYLINE | pg::OP_POLYGON => {
                let pts: Vec<P3> = pg::poly_vertices(buf, c)
                    .into_iter()
                    .map(|(_, p)| p)
                    .collect();
                if pts.len() >= 2 && pts.iter().all(finite) {
                    push(
                        Prim::Line {
                            pts,
                            closed: c.opcode == pg::OP_POLYGON,
                        },
                        aci,
                    );
                }
            }
            pg::OP_CIRCLE if body_bytes >= 56 => {
                let center = get3(buf, b);
                let r = raw::get_f64(buf, b + 192);
                let n = get3(buf, b + 256);
                if finite(&center) && r.is_finite() && r > 0.0 {
                    push(
                        Prim::Line {
                            pts: circle3d(center, r, n, ocs_x(n), TAU),
                            closed: true,
                        },
                        aci,
                    );
                }
            }
            pg::OP_ARC if body_bytes >= 84 => {
                // center, radius, normal, start vector, sweep, arc type
                let center = get3(buf, b);
                let r = raw::get_f64(buf, b + 192);
                let n = get3(buf, b + 256);
                let sv = get3(buf, b + 448);
                let sweep = raw::get_f64(buf, b + 640);
                if finite(&center) && r > 0.0 && sweep.is_finite() {
                    push(
                        Prim::Line {
                            pts: circle3d(center, r, n, sv, sweep),
                            closed: false,
                        },
                        aci,
                    );
                }
            }
            pg::OP_UNICODE_TEXT if body_bytes > 96 => {
                // position, normal, direction, height, width factor,
                // oblique, then the NUL-terminated text.
                let at = get3(buf, b);
                let dir = get3(buf, b + 384);
                let height = raw::get_f64(buf, b + 576);
                let (text, _) = utf16_at(buf, b + 768, body_bytes - 96);
                if finite(&at) && height.is_finite() && height > 0.0 {
                    push(
                        Prim::Text {
                            at,
                            dir: norm(dir),
                            height,
                            text,
                        },
                        aci,
                    );
                }
            }
            OP_UNICODE_TEXT2 if body_bytes > 72 => {
                // position, normal, direction, text (NUL-terminated, padded
                // to 4 bytes), RL length, RL raw, height, width factor, …
                let at = get3(buf, b);
                let dir = get3(buf, b + 384);
                let (text, used) = utf16_at(buf, b + 576, body_bytes - 72);
                let padded = used.div_ceil(4) * 4;
                let hbit = b + 576 + (padded + 8) * 8;
                let height = if 72 + padded + 16 <= body_bytes {
                    raw::get_f64(buf, hbit)
                } else {
                    f64::NAN
                };
                if finite(&at) && height.is_finite() && height > 0.0 {
                    push(
                        Prim::Text {
                            at,
                            dir: norm(dir),
                            height,
                            text,
                        },
                        aci,
                    );
                }
            }
            OP_COLOR if body_bytes >= 4 => {
                let v = raw::get_i32(buf, b);
                aci = (1..=255).contains(&v).then_some(v);
                drawn = false;
            }
            _ => drawn = false,
        }
        if !drawn {
            *out.skipped.entry(c.opcode).or_default() += 1;
        }
    }
    out
}

pub const OP_COLOR: i32 = 14;
pub const OP_UNICODE_TEXT2: i32 = 38;

/// Short name of a proxy-graphics opcode (public ODA specification).
pub fn opcode_name(op: i32) -> &'static str {
    match op {
        1 => "extents",
        2 => "circle",
        3 => "circle 3pt",
        4 => "arc",
        5 => "arc 3pt",
        6 => "polyline",
        7 => "polygon",
        8 => "mesh",
        9 => "shell",
        10 => "text",
        11 => "text2",
        12 => "xline",
        13 => "ray",
        14 => "color",
        16 => "layer",
        18 => "linetype",
        20 => "marker",
        21 => "fill on",
        36 => "unicode text",
        38 => "unicode text2",
        _ if ATTRIBUTE_OPS.contains(&op) => "attribute",
        _ => "?",
    }
}

// ---------------------------------------------------------------------------
// Plain AutoCAD context entities

fn context_prims(e: &EntityType, scene: &mut Scene) {
    let mut push = |prim| {
        scene.context.push(Colored {
            prim,
            aci: entity_aci(e),
        })
    };
    let v3 = |v: &acadrust::types::Vector3| [v.x, v.y, v.z];
    match e {
        EntityType::Line(l) => push(Prim::Line {
            pts: vec![v3(&l.start), v3(&l.end)],
            closed: false,
        }),
        EntityType::Circle(c) => push(Prim::Line {
            pts: circle3d(
                v3(&c.center),
                c.radius,
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                TAU,
            ),
            closed: true,
        }),
        EntityType::Arc(a) => {
            let sweep = (a.end_angle - a.start_angle).rem_euclid(TAU);
            push(Prim::Line {
                pts: arc_points(v3(&a.center), a.radius, a.start_angle, sweep),
                closed: false,
            })
        }
        EntityType::LwPolyline(p) => {
            let z = p.elevation;
            let n = p.vertices.len();
            let mut pts: Vec<P3> = Vec::new();
            let segs = if p.is_closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let a = &p.vertices[i];
                let b = &p.vertices[(i + 1) % n];
                let (pa, pb) = (
                    [a.location.x, a.location.y, z],
                    [b.location.x, b.location.y, z],
                );
                if pts.is_empty() {
                    pts.push(pa);
                }
                if a.bulge.abs() > 1e-12 {
                    pts.extend(bulge_points(pa, pb, a.bulge).into_iter().skip(1));
                } else {
                    pts.push(pb);
                }
            }
            if n == 1 {
                pts.push([p.vertices[0].location.x, p.vertices[0].location.y, z]);
            }
            if pts.len() >= 2 {
                push(Prim::Line { pts, closed: false })
            }
        }
        other => {
            let name = format!("{other:?}");
            let kind = name
                .split(['(', ' ', '{'])
                .next()
                .unwrap_or("?")
                .to_string();
            *scene.context_skipped.entry(kind).or_default() += 1;
        }
    }
}

/// Arc points for a polyline segment with a bulge (tan of a quarter of
/// the included angle, positive = counter-clockwise).
fn bulge_points(a: P3, b: P3, bulge: f64) -> Vec<P3> {
    let theta = 4.0 * bulge.atan();
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let chord = (dx * dx + dy * dy).sqrt();
    if chord <= 0.0 {
        return vec![a, b];
    }
    let r = chord / (2.0 * (theta / 2.0).sin()).abs();
    // Centre: from the chord midpoint along the left normal (for ccw).
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let h = r * (theta / 2.0).cos().abs();
    let sgn = if (theta.abs() <= std::f64::consts::PI) == (theta > 0.0) {
        1.0
    } else {
        -1.0
    };
    let c = [
        mid[0] - dy / chord * h * sgn,
        mid[1] + dx / chord * h * sgn,
        a[2],
    ];
    let a0 = (a[1] - c[1]).atan2(a[0] - c[0]);
    arc_points(c, r, a0, theta)
}

fn entity_aci(e: &EntityType) -> Option<i32> {
    use acadrust::types::Color;
    let common = match e {
        EntityType::Line(x) => &x.common,
        EntityType::Circle(x) => &x.common,
        EntityType::Arc(x) => &x.common,
        EntityType::LwPolyline(x) => &x.common,
        _ => return None,
    };
    match common.color {
        Color::Index(i) => Some(i as i32),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_format() {
        assert_eq!(station_text(0.0), "0+00.00");
        assert_eq!(station_text(1234.5), "12+34.50");
        assert_eq!(station_text(100.0), "1+00.00");
    }

    #[test]
    fn bulge_half_circle() {
        // Bulge 1 = 180° counter-clockwise: from (0,0) to (2,0) the arc
        // passes below the chord, through (1,-1).
        let pts = bulge_points([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 1.0);
        let last = pts.last().unwrap();
        assert!((last[0] - 2.0).abs() < 1e-9 && last[1].abs() < 1e-9);
        assert!(pts.iter().any(|p| (p[1] + 1.0).abs() < 1e-3));
    }

    #[test]
    fn path_point() {
        let path = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]];
        let (p, d) = point_on_path(&path, 15.0).unwrap();
        assert!((p[0] - 10.0).abs() < 1e-12 && (p[1] - 5.0).abs() < 1e-12);
        assert!((d[1] - 1.0).abs() < 1e-12);
    }
}
