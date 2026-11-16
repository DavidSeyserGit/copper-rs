//! Metric layout of a 2D grid map: where its cells sit in the map frame.

use bincode::{Decode, Encode};
use cu_spatial_payloads::{Point2d, Point2u};
use cu29::prelude::*;
use cu29::units::si::f32::Length as Length32;
use cu29::units::si::f64::Length as Length64;
use cu29::units::si::length::meter;
use serde::{Deserialize, Serialize};

/// Placement of a regular 2D grid in a metric frame.
///
/// The grid is axis-aligned in its frame: cell `x` grows along the frame's +X
/// and cell `y` along its +Y. A rotated grid is a grid published in a rotated
/// frame. `origin` is the LOWER-LEFT CORNER of cell `(0, 0)`, not its center
/// and not the center of the map.
///
/// `origin` is `f64` because georeferenced maps subtract coordinates in the
/// hundreds of kilometers, where `f32` resolves only centimeters.
#[derive(
    Default, Debug, Clone, Copy, PartialEq, Encode, Decode, Serialize, Deserialize, Reflect,
)]
pub struct CuGridGeometry {
    /// Cells along +X.
    pub width: u32,
    /// Cells along +Y.
    pub height: u32,
    /// Edge length of one square cell.
    pub resolution: Length32,
    /// Lower-left corner of cell `(0, 0)` in the map frame.
    pub origin: Point2d,
}

impl CuGridGeometry {
    pub const fn new(width: u32, height: u32, resolution: Length32, origin: Point2d) -> Self {
        Self {
            width,
            height,
            resolution,
            origin,
        }
    }

    /// A geometry is usable when its resolution is finite and positive and its
    /// origin is finite.
    pub fn is_valid(&self) -> bool {
        let resolution = self.resolution.get::<meter>();
        resolution.is_finite()
            && resolution > 0.0
            && self.origin.x.get::<meter>().is_finite()
            && self.origin.y.get::<meter>().is_finite()
    }

    pub fn cell_count(&self) -> usize {
        self.width as usize * self.height as usize
    }

    /// Row-major index of `cell`, or `None` outside the grid.
    pub fn index(&self, cell: Point2u) -> Option<usize> {
        (cell.x < self.width && cell.y < self.height)
            .then(|| cell.y as usize * self.width as usize + cell.x as usize)
    }

    /// The cell containing `point`, or `None` outside the grid.
    ///
    /// Cells are half-open: a point on a shared edge belongs to the cell with
    /// the larger index.
    pub fn cell_at(&self, point: Point2d) -> Option<Point2u> {
        let resolution = f64::from(self.resolution.get::<meter>());
        let x = (point.x - self.origin.x).get::<meter>() / resolution;
        let y = (point.y - self.origin.y).get::<meter>() / resolution;
        // The comparisons reject NaN and negatives, so the casts truncate
        // exactly like `floor`.
        (x >= 0.0 && y >= 0.0 && x < f64::from(self.width) && y < f64::from(self.height))
            .then(|| Point2u::new(x as u32, y as u32))
    }

    /// Center of `cell` in the map frame, or `None` outside the grid.
    pub fn cell_center(&self, cell: Point2u) -> Option<Point2d> {
        self.index(cell)?;
        let resolution = f64::from(self.resolution.get::<meter>());
        let offset = |index: u32| Length64::new::<meter>((f64::from(index) + 0.5) * resolution);
        Some(Point2d::new(
            self.origin.x + offset(cell.x),
            self.origin.y + offset(cell.y),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> CuGridGeometry {
        CuGridGeometry::new(
            4,
            3,
            Length32::new::<meter>(0.5),
            Point2d::from_meters(-1.0, 10.0),
        )
    }

    #[test]
    fn test_index_is_row_major() {
        let geometry = geometry();
        assert_eq!(geometry.index(Point2u::new(0, 0)), Some(0));
        assert_eq!(geometry.index(Point2u::new(3, 0)), Some(3));
        assert_eq!(geometry.index(Point2u::new(1, 2)), Some(9));
        assert_eq!(geometry.index(Point2u::new(4, 0)), None);
        assert_eq!(geometry.index(Point2u::new(0, 3)), None);
    }

    #[test]
    fn test_cell_at_uses_corner_origin() {
        let geometry = geometry();
        assert_eq!(
            geometry.cell_at(Point2d::from_meters(-1.0, 10.0)),
            Some(Point2u::new(0, 0))
        );
        assert_eq!(
            geometry.cell_at(Point2d::from_meters(-0.5, 10.49)),
            Some(Point2u::new(1, 0))
        );
        assert_eq!(
            geometry.cell_at(Point2d::from_meters(0.99, 11.49)),
            Some(Point2u::new(3, 2))
        );
    }

    #[test]
    fn test_cell_at_rejects_outside_points() {
        let geometry = geometry();
        assert_eq!(geometry.cell_at(Point2d::from_meters(-1.01, 10.0)), None);
        assert_eq!(geometry.cell_at(Point2d::from_meters(1.0, 10.0)), None);
        assert_eq!(geometry.cell_at(Point2d::from_meters(0.0, 11.5)), None);
        assert_eq!(geometry.cell_at(Point2d::from_meters(f64::NAN, 10.0)), None);
    }

    #[test]
    fn test_cell_center_roundtrips() {
        let geometry = geometry();
        let cell = Point2u::new(2, 1);
        let center = geometry.cell_center(cell).unwrap();
        assert_eq!(center, Point2d::from_meters(0.25, 10.75));
        assert_eq!(geometry.cell_at(center), Some(cell));
        assert_eq!(geometry.cell_center(Point2u::new(4, 0)), None);
    }

    #[test]
    fn test_invalid_geometry() {
        assert!(geometry().is_valid());
        assert!(!CuGridGeometry::default().is_valid());
        let mut geometry = geometry();
        geometry.origin = Point2d::from_meters(f64::INFINITY, 0.0);
        assert!(!geometry.is_valid());
    }
}
