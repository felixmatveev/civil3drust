//! Alignment elements of every decoded kind: tangents and arcs (read-only).
//!
//! An **arc element record** (sample S10, checked against a production
//! drawing) is laid out as:
//!
//! ```text
//! BL 0x04020008   header, same as tangents
//! B  1
//! BD p1           60.0 in every arc seen
//! BD p2           -1.0
//! BL n1           1
//! BD p3           -1.0
//! BL n2           2
//! B  0
//! BL id, BL before, BL after      as for tangents
//! RD start X, RD start Y
//! RD mid X,   RD mid Y            point on the arc halfway along it
//! RD end X,   RD end Y
//! ```
//!
//! (B = bit, BL = bit-coded long, BD = bit-coded double, RD = raw double,
//! per the public ODA DWG specification.) The meaning of p1–p3, n1, n2 is
//! unknown; they are kept as read. Centre, radius, direction and length
//! follow from the three points.

use crate::aecc_alignment::{self, chain, DecodeError, Linked, TangentRecord, ELEMENT_HEADER};
use crate::alignment::Point2;
use crate::raw::{self, RawRecord};

/// Parameters stored before an arc's IDs, meaning not yet known.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcParams {
    pub p1: f64,
    pub p2: f64,
    pub n1: i32,
    pub p3: f64,
    pub n2: i32,
}

/// One arc element record.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcRecord {
    pub header_bit: usize,
    /// Bit offset of start X (the first of six raw doubles).
    pub coords_bit: usize,
    pub id: i32,
    pub before: i32,
    pub after: i32,
    pub params: ArcParams,
    pub start: Point2,
    pub mid: Point2,
    pub end: Point2,
}

/// Circle geometry derived from an arc's three points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcGeometry {
    pub center: Point2,
    pub radius: f64,
    /// True if the arc turns left (counter-clockwise) from start to end.
    pub counter_clockwise: bool,
    /// Central angle in radians, always positive.
    pub sweep: f64,
    pub length: f64,
}

impl ArcRecord {
    /// Centre, radius, direction and length of the circle through the
    /// three stored points. `None` if they are collinear.
    pub fn geometry(&self) -> Option<ArcGeometry> {
        arc_through(self.start, self.mid, self.end)
    }
}

/// Circle through three points, travelling start → mid → end.
pub fn arc_through(a: Point2, m: Point2, b: Point2) -> Option<ArcGeometry> {
    let d = 2.0 * (a.x * (m.y - b.y) + m.x * (b.y - a.y) + b.x * (a.y - m.y));
    let scale = a.distance_to(b).max(a.distance_to(m)).max(1.0);
    if d.abs() <= 1e-12 * scale * scale {
        return None;
    }
    let sq = |p: Point2| p.x * p.x + p.y * p.y;
    let cx = (sq(a) * (m.y - b.y) + sq(m) * (b.y - a.y) + sq(b) * (a.y - m.y)) / d;
    let cy = (sq(a) * (b.x - m.x) + sq(m) * (a.x - b.x) + sq(b) * (m.x - a.x)) / d;
    let center = Point2::new(cx, cy);
    let radius = center.distance_to(a);
    let ccw = (m.x - a.x) * (b.y - a.y) - (m.y - a.y) * (b.x - a.x) > 0.0;
    let ang = |p: Point2| (p.y - cy).atan2(p.x - cx);
    let tau = std::f64::consts::TAU;
    let sweep = if ccw {
        (ang(b) - ang(a)).rem_euclid(tau)
    } else {
        (ang(a) - ang(b)).rem_euclid(tau)
    };
    Some(ArcGeometry {
        center,
        radius,
        counter_clockwise: ccw,
        sweep,
        length: radius * sweep,
    })
}

/// A decoded element of either kind.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Tangent(TangentRecord),
    Arc(ArcRecord),
}

impl Element {
    pub fn header_bit(&self) -> usize {
        match self {
            Self::Tangent(t) => t.header_bit,
            Self::Arc(a) => a.header_bit,
        }
    }
    pub fn start(&self) -> Point2 {
        match self {
            Self::Tangent(t) => t.start,
            Self::Arc(a) => a.start,
        }
    }
    pub fn end(&self) -> Point2 {
        match self {
            Self::Tangent(t) => t.end,
            Self::Arc(a) => a.end,
        }
    }
    /// Length along the element; `None` for a degenerate arc.
    pub fn length(&self) -> Option<f64> {
        match self {
            Self::Tangent(t) => Some(t.start.distance_to(t.end)),
            Self::Arc(a) => a.geometry().map(|g| g.length),
        }
    }
}

impl Linked for Element {
    fn id(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.id,
            Self::Arc(a) => a.id,
        }
    }
    fn before(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.before,
            Self::Arc(a) => a.before,
        }
    }
    fn after(&self) -> i32 {
        match self {
            Self::Tangent(t) => t.after,
            Self::Arc(a) => a.after,
        }
    }
}

fn arc_at(data: &[u8], bit: usize) -> Option<ArcRecord> {
    if raw::get_bit(data, bit) != 0
        || raw::get_bit(data, bit + 1) != 0
        || raw::get_byte(data, bit + 2) != 0x08
    {
        return None;
    }
    let (h, k) = raw::read_bl(data, bit)?;
    if h != ELEMENT_HEADER || raw::get_bit(data, k) != 1 {
        return None;
    }
    let (p1, k) = raw::read_bd(data, k + 1)?;
    let (p2, k) = raw::read_bd(data, k)?;
    let (n1, k) = raw::read_bl(data, k)?;
    let (p3, k) = raw::read_bd(data, k)?;
    let (n2, k) = raw::read_bl(data, k)?;
    if raw::get_bit(data, k) != 0 {
        return None;
    }
    let (id, k) = raw::read_bl(data, k + 1)?;
    let (before, k) = raw::read_bl(data, k)?;
    let (after, k) = raw::read_bl(data, k)?;
    let small = |v: i32| (0..100_000).contains(&v);
    if id < 1 || !small(id) || !small(before) || !small(after) || !small(n1) || !small(n2) {
        return None;
    }
    if ![p1, p2, p3].iter().all(|v| v.is_finite()) || k + 384 > raw::bit_len(data) {
        return None;
    }
    let c: Vec<f64> = (0..6).map(|i| raw::get_f64(data, k + 64 * i)).collect();
    if c.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let rec = ArcRecord {
        header_bit: bit,
        coords_bit: k,
        id,
        before,
        after,
        params: ArcParams { p1, p2, n1, p3, n2 },
        start: Point2::new(c[0], c[1]),
        mid: Point2::new(c[2], c[3]),
        end: Point2::new(c[4], c[5]),
    };
    rec.geometry()?; // three distinct, non-collinear points
    Some(rec)
}

/// Every tangent or arc element record in `data`, in storage order.
pub fn element_records(data: &[u8]) -> Vec<Element> {
    let mut out = Vec::new();
    let mut bit = 0;
    let n = raw::bit_len(data);
    while bit + 34 <= n {
        if let Some(t) = aecc_alignment::record_at(data, bit) {
            bit = t.coords_bit + 256;
            out.push(Element::Tangent(t));
        } else if let Some(a) = arc_at(data, bit) {
            bit = a.coords_bit + 384;
            out.push(Element::Arc(a));
        } else {
            bit += 1;
        }
    }
    out
}

/// An alignment decoded into elements of any supported kind.
#[derive(Debug, Clone)]
pub struct DecodedElements {
    pub handle: u64,
    pub name: Option<String>,
    pub stored_start_station: Option<f64>,
    /// Elements in chain order.
    pub elements: Vec<Element>,
}

impl DecodedElements {
    pub fn length(&self) -> f64 {
        self.elements.iter().filter_map(Element::length).sum()
    }
}

/// Decode an alignment of tangents and arcs (read-only).
pub fn decode_elements(rec: &RawRecord) -> Result<DecodedElements, DecodeError> {
    let elements = chain(&element_records(&rec.data))?;
    for w in elements.windows(2) {
        let (e, s) = (w[0].end(), w[1].start());
        if e.distance_to(s) > 1e-6 * (1.0 + e.x.abs().max(e.y.abs())) {
            return Err(DecodeError::Discontinuous {
                after_id: w[0].id(),
            });
        }
    }
    let stored_start_station = aecc_alignment::start_station(&rec.data, elements[0].header_bit());
    Ok(DecodedElements {
        handle: rec.handle,
        name: aecc_alignment::record_name(&rec.data),
        stored_start_station,
        elements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aecc_alignment::decode;

    struct Bits(Vec<u8>, usize);
    impl Bits {
        fn bit(&mut self, b: u8) {
            if self.1.is_multiple_of(8) {
                self.0.push(0);
            }
            if b != 0 {
                let i = self.1 / 8;
                self.0[i] |= 1 << (7 - self.1 % 8);
            }
            self.1 += 1;
        }
        fn byte(&mut self, v: u8) {
            for i in (0..8).rev() {
                self.bit((v >> i) & 1);
            }
        }
        fn bl(&mut self, v: i32) {
            if v == 0 {
                self.bit(1);
                self.bit(0);
            } else if (1..256).contains(&v) {
                self.bit(0);
                self.bit(1);
                self.byte(v as u8);
            } else {
                self.bit(0);
                self.bit(0);
                v.to_le_bytes().iter().for_each(|b| self.byte(*b));
            }
        }
        fn rd(&mut self, v: f64) {
            v.to_le_bytes().iter().for_each(|b| self.byte(*b));
        }
        fn bd(&mut self, v: f64) {
            self.bit(0);
            self.bit(0);
            self.rd(v);
        }
        fn tangent(&mut self, id: i32, before: i32, after: i32, a: (f64, f64), b: (f64, f64)) {
            for v in [ELEMENT_HEADER, 0, id, before, after] {
                self.bl(v);
            }
            for v in [a.0, a.1, b.0, b.1] {
                self.rd(v);
            }
        }
        /// Arc record laid out as in sample S10.
        fn arc(&mut self, ids: (i32, i32, i32), pts: [(f64, f64); 3]) {
            self.bl(ELEMENT_HEADER);
            self.bit(1);
            self.bd(60.0);
            self.bd(-1.0);
            self.bl(1);
            self.bd(-1.0);
            self.bl(2);
            self.bit(0);
            self.bl(ids.0);
            self.bl(ids.1);
            self.bl(ids.2);
            for p in pts {
                self.rd(p.0);
                self.rd(p.1);
            }
        }
    }

    fn record(data: Vec<u8>) -> RawRecord {
        RawRecord {
            handle: 7,
            class: "AECC_ALIGNMENT".into(),
            is_entity: true,
            data,
        }
    }

    const H: f64 = std::f64::consts::FRAC_1_SQRT_2 * 100.0; // 70.71…

    #[test]
    fn decodes_s10_quarter_arc() {
        let mut w = Bits(Vec::new(), 0);
        w.byte(0xA5);
        w.arc((1, 0, 0), [(0.0, 0.0), (H, 100.0 - H), (100.0, 100.0)]);
        let d = decode_elements(&record(w.0)).unwrap();
        assert_eq!(d.elements.len(), 1);
        let Element::Arc(a) = &d.elements[0] else {
            panic!("not an arc")
        };
        assert_eq!(
            a.params,
            ArcParams {
                p1: 60.0,
                p2: -1.0,
                n1: 1,
                p3: -1.0,
                n2: 2
            }
        );
        let g = a.geometry().unwrap();
        assert!(g.center.distance_to(Point2::new(0.0, 100.0)) < 1e-9);
        assert!((g.radius - 100.0).abs() < 1e-9);
        assert!(g.counter_clockwise);
        assert!((g.length - 50.0 * std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn decodes_tangent_arc_tangent_chain() {
        // East 100, quarter arc R=100 turning left, north 50.
        let mut w = Bits(Vec::new(), 0);
        w.tangent(3, 2, 0, (200.0, 100.0), (200.0, 150.0));
        w.arc(
            (2, 1, 3),
            [(100.0, 0.0), (100.0 + H, 100.0 - H), (200.0, 100.0)],
        );
        w.tangent(1, 0, 2, (0.0, 0.0), (100.0, 0.0));
        let rec = record(w.0);
        let d = decode_elements(&rec).unwrap();
        let kinds: Vec<&str> = d
            .elements
            .iter()
            .map(|e| match e {
                Element::Tangent(_) => "T",
                Element::Arc(_) => "A",
            })
            .collect();
        assert_eq!(kinds, ["T", "A", "T"]);
        assert!((d.length() - (150.0 + 50.0 * std::f64::consts::PI)).abs() < 1e-9);
        // The tangent-only decoder refuses it clearly.
        assert_eq!(
            decode(&rec).unwrap_err(),
            DecodeError::HasCurves { arcs: 1 }
        );
    }

    #[test]
    fn clockwise_arc_and_collinear_rejection() {
        let g = arc_through(
            Point2::new(0.0, 0.0),
            Point2::new(H, H - 100.0),
            Point2::new(100.0, -100.0),
        )
        .unwrap();
        assert!(!g.counter_clockwise);
        assert!(g.center.distance_to(Point2::new(0.0, -100.0)) < 1e-9);
        assert!(arc_through(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 2.0)
        )
        .is_none());
    }
}
