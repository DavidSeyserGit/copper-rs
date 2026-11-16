//! Standardized map payloads for Copper.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod geometry;
mod semantics;

pub use geometry::CuGridGeometry;
pub use semantics::CuSemanticClass;
pub use semantics::CuSemanticClassId;
pub use semantics::CuSemanticLabelSet;
