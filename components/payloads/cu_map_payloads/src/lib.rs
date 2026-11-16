//! Standardized map payloads for Copper: metric grid geometry, typed grid
//! layers, and the class vocabulary semantic layers refer to.
//!
//! A map is a set of [`CuGridMap`] layers sharing one [`CuGridGeometry`]. Each
//! layer type fixes what a cell stores and which stored value means
//! "never observed", so consumers read `Option`s instead of sentinels.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod geometry;
#[cfg(feature = "std")]
mod grid;
mod layer;
mod semantics;

pub use geometry::CuGridGeometry;
#[cfg(feature = "std")]
pub use grid::CuCostGridMap;
#[cfg(feature = "std")]
pub use grid::CuElevationGridMap;
#[cfg(feature = "std")]
pub use grid::CuFeatureGridMap;
#[cfg(feature = "std")]
pub use grid::CuGridMap;
#[cfg(feature = "std")]
pub use grid::CuLabelGridMap;
#[cfg(feature = "std")]
pub use grid::CuOccupancyGridMap;
pub use layer::CuCostLayer;
pub use layer::CuElevationLayer;
pub use layer::CuFeatureLayer;
pub use layer::CuGridLayer;
pub use layer::CuLabelLayer;
pub use layer::CuOccupancyLayer;
pub use semantics::CuSemanticClass;
pub use semantics::CuSemanticClassId;
pub use semantics::CuSemanticLabelSet;
