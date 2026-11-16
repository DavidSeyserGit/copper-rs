//! What one grid cell stores, and which stored value means "never observed".

use crate::CuSemanticClassId;

use cu29::units::si::f32::Length as Length32;
use cu29::units::si::f32::Ratio;
use cu29::units::si::length::meter;
use cu29::units::si::ratio::ratio;

use core::fmt::Debug;

/// Compile-time description of a grid layer's cell representation.
///
/// A cell is `CHANNELS` consecutive [`CuGridLayer::Cell`] values. `decode`
/// and `encode` are the only place the layer's "never observed" sentinel is
/// spelled out; consumers see `Option<Value>`.
pub trait CuGridLayer: Copy + Debug + Default + Send + Sync + 'static {
    /// Stored element type.
    type Cell: Copy + Debug + Default + Send + Sync + 'static;
    /// Typed value of one observed cell.
    type Value;

    /// Stored elements per cell.
    const CHANNELS: usize = 1;
    const TYPE_PATH: &'static str;
    const SHORT_TYPE_PATH: &'static str;

    /// Reads one cell. Returns `None` when the cell was never observed.
    ///
    /// `cell` holds exactly `CHANNELS` elements.
    fn decode(cell: &[Self::Cell]) -> Option<Self::Value>;

    /// Writes one cell; `None` marks it never observed.
    ///
    /// `cell` holds exactly `CHANNELS` elements.
    fn encode(value: Option<Self::Value>, cell: &mut [Self::Cell]);
}

/// Occupancy probability in 255 steps. A stored `0` is never observed;
/// `1..=255` span 0 % to 100 % linearly, so a zeroed buffer is an unobserved
/// map.
#[derive(Debug, Default, Clone, Copy)]
pub struct CuOccupancyLayer;

impl CuOccupancyLayer {
    /// Stored steps between 0 % and 100 %.
    const STEPS: f32 = 254.0;
}

impl CuGridLayer for CuOccupancyLayer {
    type Cell = u8;
    type Value = Ratio;

    const TYPE_PATH: &'static str = "cu_map_payloads::CuOccupancyGridMap";
    const SHORT_TYPE_PATH: &'static str = "CuOccupancyGridMap";

    fn decode(cell: &[u8]) -> Option<Ratio> {
        (cell[0] != 0).then(|| Ratio::new::<ratio>(f32::from(cell[0] - 1) / Self::STEPS))
    }

    fn encode(value: Option<Ratio>, cell: &mut [u8]) {
        cell[0] = match value.map(|value| value.get::<ratio>()) {
            // Rounds to nearest: the clamped value is non-negative, so
            // adding one half before the truncating cast is enough.
            Some(value) if value.is_finite() => {
                1 + (value.clamp(0.0, 1.0) * Self::STEPS + 0.5) as u8
            }
            _ => 0,
        };
    }
}

/// Semantic class per cell, resolved through a
/// [`CuSemanticLabelSet`](crate::CuSemanticLabelSet).
#[derive(Debug, Default, Clone, Copy)]
pub struct CuLabelLayer;

impl CuGridLayer for CuLabelLayer {
    type Cell = u16;
    type Value = CuSemanticClassId;

    const TYPE_PATH: &'static str = "cu_map_payloads::CuLabelGridMap";
    const SHORT_TYPE_PATH: &'static str = "CuLabelGridMap";

    fn decode(cell: &[u16]) -> Option<CuSemanticClassId> {
        let id = CuSemanticClassId(cell[0]);
        (!id.is_unknown()).then_some(id)
    }

    fn encode(value: Option<CuSemanticClassId>, cell: &mut [u16]) {
        cell[0] = value.unwrap_or(CuSemanticClassId::UNKNOWN).0;
    }
}

/// Surface height above the map frame's XY plane. NaN is never observed.
#[derive(Debug, Default, Clone, Copy)]
pub struct CuElevationLayer;

impl CuGridLayer for CuElevationLayer {
    type Cell = Length32;
    type Value = Length32;

    const TYPE_PATH: &'static str = "cu_map_payloads::CuElevationGridMap";
    const SHORT_TYPE_PATH: &'static str = "CuElevationGridMap";

    fn decode(cell: &[Length32]) -> Option<Length32> {
        (!cell[0].get::<meter>().is_nan()).then_some(cell[0])
    }

    fn encode(value: Option<Length32>, cell: &mut [Length32]) {
        cell[0] = value.unwrap_or_else(|| Length32::new::<meter>(f32::NAN));
    }
}

/// Unitless traversal cost; larger is worse. NaN is never observed.
///
/// The scale belongs to the planner that consumes the layer.
#[derive(Debug, Default, Clone, Copy)]
pub struct CuCostLayer;

impl CuGridLayer for CuCostLayer {
    type Cell = f32;
    type Value = f32;

    const TYPE_PATH: &'static str = "cu_map_payloads::CuCostGridMap";
    const SHORT_TYPE_PATH: &'static str = "CuCostGridMap";

    fn decode(cell: &[f32]) -> Option<f32> {
        (!cell[0].is_nan()).then_some(cell[0])
    }

    fn encode(value: Option<f32>, cell: &mut [f32]) {
        cell[0] = value.unwrap_or(f32::NAN);
    }
}

/// A `D`-dimensional feature vector per cell, such as a fused vision-language
/// embedding. A cell whose first element is NaN is never observed.
#[derive(Debug, Default, Clone, Copy)]
pub struct CuFeatureLayer<const D: usize>;

impl<const D: usize> CuGridLayer for CuFeatureLayer<D> {
    type Cell = f32;
    type Value = [f32; D];

    const CHANNELS: usize = D;
    const TYPE_PATH: &'static str = "cu_map_payloads::CuFeatureGridMap";
    const SHORT_TYPE_PATH: &'static str = "CuFeatureGridMap";

    fn decode(cell: &[f32]) -> Option<[f32; D]> {
        if cell.first().is_none_or(|first| first.is_nan()) {
            return None;
        }
        let mut value = [0.0; D];
        value.copy_from_slice(cell);
        Some(value)
    }

    fn encode(value: Option<[f32; D]>, cell: &mut [f32]) {
        match value {
            Some(value) => cell.copy_from_slice(&value),
            None => cell.fill(f32::NAN),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occupancy_keeps_unknown_apart_from_free() {
        assert_eq!(CuOccupancyLayer::decode(&[0]), None);
        assert_eq!(
            CuOccupancyLayer::decode(&[1]),
            Some(Ratio::new::<ratio>(0.0))
        );
        assert_eq!(
            CuOccupancyLayer::decode(&[255]),
            Some(Ratio::new::<ratio>(1.0))
        );

        let mut cell = [7u8];
        CuOccupancyLayer::encode(None, &mut cell);
        assert_eq!(cell, [0]);
        CuOccupancyLayer::encode(Some(Ratio::new::<ratio>(0.0)), &mut cell);
        assert_eq!(cell, [1]);
        CuOccupancyLayer::encode(Some(Ratio::new::<ratio>(0.5)), &mut cell);
        assert_eq!(cell, [128]);
        CuOccupancyLayer::encode(Some(Ratio::new::<ratio>(2.5)), &mut cell);
        assert_eq!(cell, [255]);
        CuOccupancyLayer::encode(Some(Ratio::new::<ratio>(-1.0)), &mut cell);
        assert_eq!(cell, [1]);
        CuOccupancyLayer::encode(Some(Ratio::new::<ratio>(f32::NAN)), &mut cell);
        assert_eq!(cell, [0]);
    }

    #[test]
    fn test_occupancy_roundtrips_every_stored_step() {
        for raw in 0..=u8::MAX {
            let mut cell = [0u8];
            CuOccupancyLayer::encode(CuOccupancyLayer::decode(&[raw]), &mut cell);
            assert_eq!(cell, [raw]);
        }
    }

    #[test]
    fn test_label_zero_is_a_class() {
        assert_eq!(CuLabelLayer::decode(&[0]), Some(CuSemanticClassId(0)));
        assert_eq!(CuLabelLayer::decode(&[u16::MAX]), None);
    }

    #[test]
    fn test_feature_roundtrip() {
        let mut cell = [0.0f32; 3];
        CuFeatureLayer::<3>::encode(None, &mut cell);
        assert_eq!(CuFeatureLayer::<3>::decode(&cell), None);
        CuFeatureLayer::<3>::encode(Some([0.1, 0.2, 0.3]), &mut cell);
        assert_eq!(CuFeatureLayer::<3>::decode(&cell), Some([0.1, 0.2, 0.3]));
    }
}
