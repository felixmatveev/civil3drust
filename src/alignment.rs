//! Tangent-only horizontal alignment geometry.
//!
//! This module is deliberately independent of the DWG format. It models an
//! alignment as an ordered list of PI (point of intersection) locations plus
//! a starting station. With no curves, each segment is simply the straight
//! tangent between consecutive PIs.
//!
//! Units are whatever the drawing uses (typically US survey feet or meters);
//! nothing here converts units.

use std::fmt;

/// A 2D point in drawing coordinates (X = easting, Y = northing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Point2) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }
}

/// One straight segment of an alignment, with its stationing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tangent {
    pub start: Point2,
    pub end: Point2,
    pub start_station: f64,
    pub end_station: f64,
}

impl Tangent {
    pub fn length(&self) -> f64 {
        self.start.distance_to(self.end)
    }

    /// Direction angle in radians, measured counter-clockwise from +X
    /// (the CAD convention, not a surveyor's bearing).
    pub fn direction(&self) -> f64 {
        (self.end.y - self.start.y).atan2(self.end.x - self.start.x)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentError {
    /// An alignment needs at least two PIs to have a segment.
    TooFewPoints(usize),
    /// Two consecutive PIs coincide, so the segment has no direction.
    ZeroLengthSegment(usize),
    SegmentOutOfRange {
        index: usize,
        count: usize,
    },
    InvalidLength(f64),
    StationOutOfRange(f64),
}

impl fmt::Display for AlignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewPoints(n) => write!(f, "alignment needs at least 2 PIs, got {n}"),
            Self::ZeroLengthSegment(i) => write!(f, "segment {i} has zero length"),
            Self::SegmentOutOfRange { index, count } => {
                write!(f, "segment {index} out of range (alignment has {count})")
            }
            Self::InvalidLength(l) => {
                write!(f, "segment length must be positive and finite, got {l}")
            }
            Self::StationOutOfRange(s) => write!(f, "station {s} is outside the alignment"),
        }
    }
}

impl std::error::Error for AlignmentError {}

/// A horizontal alignment made only of tangents.
#[derive(Debug, Clone, PartialEq)]
pub struct Alignment {
    pub name: String,
    pub start_station: f64,
    pis: Vec<Point2>,
}

impl Alignment {
    pub fn new(
        name: impl Into<String>,
        start_station: f64,
        pis: Vec<Point2>,
    ) -> Result<Self, AlignmentError> {
        if pis.len() < 2 {
            return Err(AlignmentError::TooFewPoints(pis.len()));
        }
        for (i, w) in pis.windows(2).enumerate() {
            if w[0].distance_to(w[1]) <= f64::EPSILON {
                return Err(AlignmentError::ZeroLengthSegment(i));
            }
        }
        Ok(Self {
            name: name.into(),
            start_station,
            pis,
        })
    }

    pub fn pis(&self) -> &[Point2] {
        &self.pis
    }

    pub fn segment_count(&self) -> usize {
        self.pis.len() - 1
    }

    pub fn length(&self) -> f64 {
        self.pis.windows(2).map(|w| w[0].distance_to(w[1])).sum()
    }

    pub fn end_station(&self) -> f64 {
        self.start_station + self.length()
    }

    /// All segments with their start/end stations.
    pub fn tangents(&self) -> Vec<Tangent> {
        let mut sta = self.start_station;
        self.pis
            .windows(2)
            .map(|w| {
                let len = w[0].distance_to(w[1]);
                let t = Tangent {
                    start: w[0],
                    end: w[1],
                    start_station: sta,
                    end_station: sta + len,
                };
                sta += len;
                t
            })
            .collect()
    }

    /// Point on the alignment at a given station.
    pub fn point_at_station(&self, station: f64) -> Result<Point2, AlignmentError> {
        let tol = 1e-9;
        if station < self.start_station - tol || station > self.end_station() + tol {
            return Err(AlignmentError::StationOutOfRange(station));
        }
        let tangents = self.tangents();
        let t = tangents
            .iter()
            .find(|t| station <= t.end_station + tol)
            .unwrap_or_else(|| tangents.last().expect("at least one tangent"));
        let frac = ((station - t.start_station) / t.length()).clamp(0.0, 1.0);
        Ok(Point2::new(
            t.start.x + frac * (t.end.x - t.start.x),
            t.start.y + frac * (t.end.y - t.start.y),
        ))
    }

    /// Change the length of segment `index`, keeping its direction.
    ///
    /// The end PI of that segment slides along the segment's direction, and
    /// every PI after it is translated by the same offset. Downstream
    /// segments therefore keep their directions and lengths; only stations
    /// downstream shift. See `DECISIONS.md` (D-003) for why.
    pub fn set_segment_length(
        &mut self,
        index: usize,
        new_length: f64,
    ) -> Result<(), AlignmentError> {
        let count = self.segment_count();
        if index >= count {
            return Err(AlignmentError::SegmentOutOfRange { index, count });
        }
        if !(new_length.is_finite() && new_length > 0.0) {
            return Err(AlignmentError::InvalidLength(new_length));
        }
        let a = self.pis[index];
        let b = self.pis[index + 1];
        let old_length = a.distance_to(b);
        let ux = (b.x - a.x) / old_length;
        let uy = (b.y - a.y) / old_length;
        let dx = ux * (new_length - old_length);
        let dy = uy * (new_length - old_length);
        for p in &mut self.pis[index + 1..] {
            p.x += dx;
            p.y += dy;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn l_shape() -> Alignment {
        // 100 ft east, then 50 ft north, starting at 10+00.
        Alignment::new(
            "L",
            1000.0,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(100.0, 0.0),
                Point2::new(100.0, 50.0),
            ],
        )
        .unwrap()
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(
            Alignment::new("x", 0.0, vec![Point2::new(0.0, 0.0)]),
            Err(AlignmentError::TooFewPoints(1))
        );
        assert_eq!(
            Alignment::new("x", 0.0, vec![Point2::new(1.0, 1.0), Point2::new(1.0, 1.0)]),
            Err(AlignmentError::ZeroLengthSegment(0))
        );
    }

    #[test]
    fn stationing() {
        let a = l_shape();
        assert!((a.length() - 150.0).abs() < EPS);
        assert!((a.end_station() - 1150.0).abs() < EPS);
        let t = a.tangents();
        assert!((t[1].start_station - 1100.0).abs() < EPS);
    }

    #[test]
    fn point_at_station() {
        let a = l_shape();
        let p = a.point_at_station(1125.0).unwrap();
        assert!((p.x - 100.0).abs() < EPS && (p.y - 25.0).abs() < EPS);
        assert!(a.point_at_station(999.0).is_err());
        assert!(a.point_at_station(1151.0).is_err());
    }

    #[test]
    fn lengthen_first_segment_shifts_downstream() {
        let mut a = l_shape();
        a.set_segment_length(0, 120.0).unwrap();
        assert_eq!(a.pis()[1], Point2::new(120.0, 0.0));
        assert_eq!(a.pis()[2], Point2::new(120.0, 50.0));
        // Downstream segment keeps its length; total grows by 20.
        assert!((a.tangents()[1].length() - 50.0).abs() < EPS);
        assert!((a.end_station() - 1170.0).abs() < EPS);
    }

    #[test]
    fn shorten_last_segment() {
        let mut a = l_shape();
        a.set_segment_length(1, 30.0).unwrap();
        assert!((a.pis()[2].y - 30.0).abs() < EPS);
        assert!((a.pis()[2].x - 100.0).abs() < EPS);
    }

    #[test]
    fn set_length_errors() {
        let mut a = l_shape();
        assert!(matches!(
            a.set_segment_length(2, 10.0),
            Err(AlignmentError::SegmentOutOfRange { .. })
        ));
        assert!(matches!(
            a.set_segment_length(0, 0.0),
            Err(AlignmentError::InvalidLength(_))
        ));
        assert!(matches!(
            a.set_segment_length(0, f64::NAN),
            Err(AlignmentError::InvalidLength(_))
        ));
    }
}
