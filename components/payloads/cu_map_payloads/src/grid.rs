//! Handle-backed grid layers.

use crate::CuCostLayer;
use crate::CuElevationLayer;
use crate::CuFeatureLayer;
use crate::CuGridGeometry;
use crate::CuGridLayer;
use crate::CuLabelLayer;
use crate::CuOccupancyLayer;

use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use cu_spatial_payloads::{Point2d, Point2u};
use cu29::prelude::*;
use serde::{Deserialize, Serialize, Serializer};

use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use core::marker::PhantomData;

/// One layer of a 2D grid map, backed by a Copper handle.
///
/// Cells are stored row-major from [`CuGridGeometry::origin`], +X first, with
/// [`CuGridLayer::CHANNELS`] consecutive elements per cell. Layers of the same
/// map share a geometry and travel as separate messages, so a consumer only
/// receives the layers it is wired to.
///
/// ```
/// use cu_map_payloads::{CuGridGeometry, CuLabelGridMap, CuSemanticClassId};
/// use cu_spatial_payloads::{Point2d, Point2u};
/// use cu29::units::si::f32::Length;
/// use cu29::units::si::length::meter;
///
/// let geometry = CuGridGeometry::new(
///     200,
///     200,
///     Length::new::<meter>(0.1),
///     Point2d::from_meters(-10.0, -10.0),
/// );
/// let mut labels = CuLabelGridMap::<Vec<u16>>::new_unobserved(geometry).unwrap();
/// assert_eq!(labels.get_at(Point2d::from_meters(0.0, 0.0)), None);
///
/// labels.set(Point2u::new(100, 100), Some(CuSemanticClassId(3))).unwrap();
/// assert_eq!(
///     labels.get_at(Point2d::from_meters(0.05, 0.05)),
///     Some(CuSemanticClassId(3))
/// );
/// ```
#[derive(Debug, Reflect)]
#[reflect(from_reflect = false, no_field_bounds, type_path = false)]
pub struct CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
{
    pub geometry: CuGridGeometry,
    pub buffer_handle: CuHandle<A>,
    #[reflect(ignore)]
    layer: PhantomData<L>,
}

pub type CuOccupancyGridMap<A> = CuGridMap<A, CuOccupancyLayer>;
pub type CuLabelGridMap<A> = CuGridMap<A, CuLabelLayer>;
pub type CuElevationGridMap<A> = CuGridMap<A, CuElevationLayer>;
pub type CuCostGridMap<A> = CuGridMap<A, CuCostLayer>;
pub type CuFeatureGridMap<A, const D: usize> = CuGridMap<A, CuFeatureLayer<D>>;

// Cloning a layer clones its handle, never the storage behind it.
impl<A, L> Clone for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            geometry: self.geometry,
            buffer_handle: self.buffer_handle.clone(),
            layer: PhantomData,
        }
    }
}

impl<A, L> Default for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            geometry: CuGridGeometry::default(),
            buffer_handle: CuHandle::default(),
            layer: PhantomData,
        }
    }
}

impl<A, L> TypePath for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
{
    fn type_path() -> &'static str {
        L::TYPE_PATH
    }

    fn short_type_path() -> &'static str {
        L::SHORT_TYPE_PATH
    }

    fn type_ident() -> Option<&'static str> {
        Some(L::SHORT_TYPE_PATH)
    }

    fn crate_name() -> Option<&'static str> {
        Some("cu_map_payloads")
    }

    fn module_path() -> Option<&'static str> {
        Some("cu_map_payloads")
    }
}

impl<A, L> Encode for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
    CuHandle<A>: Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Encode::encode(&self.geometry, encoder)?;
        Encode::encode(&self.buffer_handle, encoder)
    }
}

impl<A, L> Decode<()> for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
    CuHandle<A>: Decode<()>,
{
    fn decode<D: Decoder<Context = ()>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let geometry: CuGridGeometry = Decode::decode(decoder)?;
        let buffer_handle: CuHandle<A> = Decode::decode(decoder)?;
        Self::new(geometry, buffer_handle)
            .map_err(|error| DecodeError::OtherString(format!("{error}")))
    }
}

impl<A, L> Serialize for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
    CuHandle<A>: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct(L::SHORT_TYPE_PATH, 2)?;
        state.serialize_field("geometry", &self.geometry)?;
        state.serialize_field("handle", &self.buffer_handle)?;
        state.end()
    }
}

impl<'de, A, L> Deserialize<'de> for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
    CuHandle<A>: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire<H> {
            geometry: CuGridGeometry,
            handle: H,
        }

        let wire = Wire::<CuHandle<A>>::deserialize(deserializer)?;
        Self::new(wire.geometry, wire.handle).map_err(serde::de::Error::custom)
    }
}

impl<A, L> CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
{
    /// Wraps `buffer_handle` as a layer laid out by `geometry`.
    ///
    /// The buffer keeps its contents. Errors when the geometry is invalid or
    /// the buffer holds fewer than [`Self::required_elements`] elements.
    pub fn new(geometry: CuGridGeometry, buffer_handle: CuHandle<A>) -> CuResult<Self> {
        if !geometry.is_valid() {
            return Err(CuError::from(format!(
                "Grid geometry is invalid: {geometry:?}"
            )));
        }
        let required = Self::required_elements(&geometry);
        let available = buffer_handle.with_inner(|inner| inner.len());
        if available < required {
            return Err(CuError::from(format!(
                "Grid buffer holds {available} elements, geometry requires {required}"
            )));
        }
        Ok(Self {
            geometry,
            buffer_handle,
            layer: PhantomData,
        })
    }

    /// Elements a buffer needs to hold a layer laid out by `geometry`.
    pub fn required_elements(geometry: &CuGridGeometry) -> usize {
        geometry.cell_count() * L::CHANNELS
    }

    /// The value at `cell`. `None` outside the grid and for cells that were
    /// never observed.
    pub fn get(&self, cell: Point2u) -> Option<L::Value> {
        let start = self.geometry.index(cell)? * L::CHANNELS;
        self.buffer_handle
            .with_inner(|inner| L::decode(&inner[start..start + L::CHANNELS]))
    }

    /// The value of the cell containing `point`, given in the map frame.
    pub fn get_at(&self, point: Point2d) -> Option<L::Value> {
        self.get(self.geometry.cell_at(point)?)
    }

    /// Writes `value` to `cell`; `None` marks the cell never observed.
    pub fn set(&mut self, cell: Point2u, value: Option<L::Value>) -> CuResult<()> {
        let Some(index) = self.geometry.index(cell) else {
            return Err(CuError::from(format!(
                "Cell ({}, {}) is outside the {}x{} grid",
                cell.x, cell.y, self.geometry.width, self.geometry.height
            )));
        };
        let start = index * L::CHANNELS;
        self.buffer_handle
            .with_inner_mut(|inner| L::encode(value, &mut inner[start..start + L::CHANNELS]));
        Ok(())
    }

    /// Marks every cell never observed.
    pub fn clear(&mut self) {
        self.with_cells_mut(|cells, _| {
            for cell in cells.chunks_exact_mut(L::CHANNELS.max(1)) {
                L::encode(None, cell);
            }
        });
    }

    /// Accesses all stored elements under a single buffer lock.
    ///
    /// The slice covers exactly [`Self::required_elements`].
    pub fn with_cells<R>(&self, f: impl FnOnce(&[L::Cell], CuGridGeometry) -> R) -> R {
        let geometry = self.geometry;
        let required = Self::required_elements(&geometry);
        self.buffer_handle
            .with_inner(|inner| f(&inner[..required], geometry))
    }

    /// Mutably accesses all stored elements under a single buffer lock.
    ///
    /// The slice covers exactly [`Self::required_elements`].
    pub fn with_cells_mut<R>(&mut self, f: impl FnOnce(&mut [L::Cell], CuGridGeometry) -> R) -> R {
        let geometry = self.geometry;
        let required = Self::required_elements(&geometry);
        self.buffer_handle
            .with_inner_mut(|inner| f(&mut inner[..required], geometry))
    }

    pub fn payload_should_log(&self) -> bool {
        self.buffer_handle.payload_should_log()
    }

    pub fn apply_handle_content_policy(&self, mode: cu29::pool::HandleContent) {
        self.buffer_handle.apply_handle_content_policy(mode);
    }

    pub fn mark_touched(&self) {
        self.buffer_handle.mark_touched();
    }
}

impl<L> CuGridMap<Vec<L::Cell>, L>
where
    L: CuGridLayer,
    Vec<L::Cell>: ArrayLike<Element = L::Cell>,
{
    /// Allocates a detached layer with every cell never observed.
    ///
    /// Call this at startup; reuse the layer or a pool on the cyclic path.
    pub fn new_unobserved(geometry: CuGridGeometry) -> CuResult<Self> {
        let buffer = vec![L::Cell::default(); Self::required_elements(&geometry)];
        let mut map = Self::new(geometry, CuHandle::new_detached(buffer))?;
        map.clear();
        Ok(map)
    }
}

impl<A, L> cu29::pool::HandleContentAware for CuGridMap<A, L>
where
    L: CuGridLayer,
    A: ArrayLike<Element = L::Cell> + Send + Sync + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use cu29::units::si::f32::Length;
    use cu29::units::si::f32::Ratio;
    use cu29::units::si::length::meter;
    use cu29::units::si::ratio::ratio;

    use crate::CuSemanticClassId;

    fn geometry() -> CuGridGeometry {
        CuGridGeometry::new(
            3,
            2,
            Length::new::<meter>(0.5),
            Point2d::from_meters(1.0, 2.0),
        )
    }

    #[test]
    fn test_new_layer_is_unobserved() {
        let labels = CuLabelGridMap::<Vec<u16>>::new_unobserved(geometry()).unwrap();
        let occupancy = CuOccupancyGridMap::<Vec<u8>>::new_unobserved(geometry()).unwrap();
        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(labels.get(Point2u::new(x, y)), None);
                assert_eq!(occupancy.get(Point2u::new(x, y)), None);
            }
        }
    }

    #[test]
    fn test_set_and_get_by_cell_and_point() {
        let mut labels = CuLabelGridMap::<Vec<u16>>::new_unobserved(geometry()).unwrap();
        labels
            .set(Point2u::new(2, 1), Some(CuSemanticClassId(0)))
            .unwrap();

        assert_eq!(labels.get(Point2u::new(2, 1)), Some(CuSemanticClassId(0)));
        assert_eq!(
            labels.get_at(Point2d::from_meters(2.25, 2.75)),
            Some(CuSemanticClassId(0))
        );
        assert_eq!(labels.get_at(Point2d::from_meters(0.0, 0.0)), None);
        assert!(labels.set(Point2u::new(3, 0), None).is_err());
        labels.with_cells(|cells, _| assert_eq!(cells[5], 0));
    }

    #[test]
    fn test_feature_layer_interleaves_channels() {
        let mut features = CuFeatureGridMap::<Vec<f32>, 4>::new_unobserved(geometry()).unwrap();
        assert_eq!(
            CuFeatureGridMap::<Vec<f32>, 4>::required_elements(&geometry()),
            24
        );
        features
            .set(Point2u::new(1, 0), Some([1.0, 2.0, 3.0, 4.0]))
            .unwrap();

        assert_eq!(features.get(Point2u::new(0, 0)), None);
        assert_eq!(features.get(Point2u::new(1, 0)), Some([1.0, 2.0, 3.0, 4.0]));
        features.with_cells(|cells, _| assert_eq!(&cells[4..8], &[1.0, 2.0, 3.0, 4.0]));
    }

    #[test]
    fn test_new_rejects_short_buffer_and_invalid_geometry() {
        let short = CuHandle::new_detached(vec![0u8; 5]);
        assert!(CuOccupancyGridMap::new(geometry(), short).is_err());

        let buffer = CuHandle::new_detached(vec![0u8; 6]);
        assert!(CuOccupancyGridMap::new(CuGridGeometry::default(), buffer).is_err());
    }

    #[test]
    fn test_bincode_roundtrip() {
        let mut occupancy = CuOccupancyGridMap::<Vec<u8>>::new_unobserved(geometry()).unwrap();
        occupancy
            .set(Point2u::new(1, 1), Some(Ratio::new::<ratio>(1.0)))
            .unwrap();

        let config = bincode::config::standard();
        let bytes = bincode::encode_to_vec(&occupancy, config).unwrap();
        let (decoded, used): (CuOccupancyGridMap<Vec<u8>>, usize) =
            bincode::decode_from_slice(&bytes, config).unwrap();

        assert_eq!(used, bytes.len());
        assert_eq!(decoded.geometry, geometry());
        assert_eq!(
            decoded.get(Point2u::new(1, 1)),
            Some(Ratio::new::<ratio>(1.0))
        );
        assert_eq!(decoded.get(Point2u::new(0, 0)), None);
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut elevation = CuElevationGridMap::<Vec<Length>>::new_unobserved(geometry()).unwrap();
        elevation
            .set(Point2u::new(0, 1), Some(Length::new::<meter>(0.25)))
            .unwrap();
        // JSON has no NaN, so observe every cell before the text roundtrip.
        elevation.with_cells_mut(|cells, _| {
            for cell in cells.iter_mut().filter(|cell| cell.value.is_nan()) {
                *cell = Length::new::<meter>(0.0);
            }
        });

        let json = serde_json::to_string(&elevation).unwrap();
        let decoded: CuElevationGridMap<Vec<Length>> = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.geometry, geometry());
        assert_eq!(
            decoded.get(Point2u::new(0, 1)),
            Some(Length::new::<meter>(0.25))
        );
    }

    #[test]
    fn test_payload_bounds() {
        fn assert_payload<T: CuMsgPayload>() {}
        assert_payload::<CuOccupancyGridMap<Vec<u8>>>();
        assert_payload::<CuLabelGridMap<Vec<u16>>>();
        assert_payload::<CuElevationGridMap<Vec<Length>>>();
        assert_payload::<CuCostGridMap<Vec<f32>>>();
        assert_payload::<CuFeatureGridMap<Vec<f32>, 512>>();
        assert_payload::<crate::CuSemanticLabelSet<64>>();
        assert_payload::<CuGridGeometry>();
    }
}
