//! Standardized map payloads for Copper.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod geometry;

pub use geometry::CuGridGeometry;
