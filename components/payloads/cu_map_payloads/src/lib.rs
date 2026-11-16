//! Standardized map payloads for Copper.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod geometry;
mod layer;
mod semantics;

pub use geometry::CuGridGeometry;
pub use layer::CuCostLayer;
pub use layer::CuElevationLayer;
pub use layer::CuFeatureLayer;
pub use layer::CuGridLayer;
pub use layer::CuLabelLayer;
pub use layer::CuOccupancyLayer;
pub use semantics::CuSemanticClass;
pub use semantics::CuSemanticClassId;
pub use semantics::CuSemanticLabelSet;
