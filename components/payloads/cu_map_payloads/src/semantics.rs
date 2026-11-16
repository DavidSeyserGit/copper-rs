//! The class vocabulary shared by a semantic map's producer and consumers.

use bincode::de::Decoder;
use bincode::error::DecodeError;
use bincode::{Decode, Encode};
use cu29::prelude::*;
use serde::{Deserialize, Serialize};

use alloc::format;

/// Identifier of a semantic class within a [`CuSemanticLabelSet`].
///
/// Dataset ids pass through unchanged, including `0`.
/// [`CuSemanticClassId::UNKNOWN`] is reserved for cells that were never
/// labeled.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Encode,
    Decode,
    Serialize,
    Deserialize,
    Reflect,
)]
pub struct CuSemanticClassId(pub u16);

impl CuSemanticClassId {
    /// Reserved id of a cell that was never labeled.
    pub const UNKNOWN: Self = Self(u16::MAX);

    pub const fn is_unknown(self) -> bool {
        self.0 == Self::UNKNOWN.0
    }
}

impl Default for CuSemanticClassId {
    fn default() -> Self {
        Self::UNKNOWN
    }
}

/// One entry of a semantic vocabulary.
///
/// The name is stored inline as UTF-8, so a class never allocates.
#[derive(Default, Debug, Clone, Encode, Serialize, Deserialize, Reflect)]
#[reflect(from_reflect = false)]
pub struct CuSemanticClass {
    pub id: CuSemanticClassId,
    name: CuArrayVec<u8, { CuSemanticClass::NAME_CAPACITY }>,
    /// Display color as sRGB with alpha.
    pub color: [u8; 4],
}

impl CuSemanticClass {
    /// Longest name a class can carry, in UTF-8 bytes.
    pub const NAME_CAPACITY: usize = 64;

    /// Builds a class. For open-vocabulary maps `name` is the text prompt.
    ///
    /// Errors when `name` is longer than [`Self::NAME_CAPACITY`] bytes.
    pub fn new(id: CuSemanticClassId, name: &str, color: [u8; 4]) -> CuResult<Self> {
        let mut bytes = CuArrayVec::default();
        bytes
            .0
            .try_extend_from_slice(name.as_bytes())
            .map_err(|_| {
                CuError::from(format!(
                    "Semantic class name is {} bytes, the limit is {}",
                    name.len(),
                    Self::NAME_CAPACITY
                ))
            })?;
        Ok(Self {
            id,
            name: bytes,
            color,
        })
    }

    /// Human-readable name. Empty when decoded bytes are not valid UTF-8.
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name.0).unwrap_or_default()
    }
}

// `CuArrayVec` decodes with the unit context only, which the derive cannot
// express.
impl Decode<()> for CuSemanticClass {
    fn decode<D: Decoder<Context = ()>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            id: Decode::decode(decoder)?,
            name: Decode::decode(decoder)?,
            color: Decode::decode(decoder)?,
        })
    }
}

impl PartialEq for CuSemanticClass {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name.0 == other.name.0 && self.color == other.color
    }
}

impl Eq for CuSemanticClass {}

/// The vocabulary a label layer's class ids refer to, holding up to `N`
/// classes inline.
///
/// The set changes rarely, so send it as a latched state next to the map
/// layers instead of with every map update.
#[derive(Default, Debug, Clone, Encode, Serialize, Deserialize, Reflect)]
#[reflect(from_reflect = false)]
pub struct CuSemanticLabelSet<const N: usize> {
    classes: CuArrayVec<CuSemanticClass, N>,
}

impl<const N: usize> CuSemanticLabelSet<N> {
    /// Adds `class`. Errors when the set already holds `N` classes or one with
    /// the same id.
    pub fn push(&mut self, class: CuSemanticClass) -> CuResult<()> {
        if self.class(class.id).is_some() {
            return Err(CuError::from(format!(
                "Semantic class id {} is already registered",
                class.id.0
            )));
        }
        self.classes
            .0
            .try_push(class)
            .map_err(|_| CuError::from(format!("Semantic label set is full at {N} classes")))
    }

    pub fn classes(&self) -> &[CuSemanticClass] {
        &self.classes.0
    }

    /// The class registered under `id`.
    pub fn class(&self, id: CuSemanticClassId) -> Option<&CuSemanticClass> {
        self.classes().iter().find(|class| class.id == id)
    }

    /// The class registered under `name`.
    pub fn class_named(&self, name: &str) -> Option<&CuSemanticClass> {
        self.classes().iter().find(|class| class.name() == name)
    }
}

impl<const N: usize> Decode<()> for CuSemanticLabelSet<N> {
    fn decode<D: Decoder<Context = ()>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            classes: Decode::decode(decoder)?,
        })
    }
}

impl<const N: usize> PartialEq for CuSemanticLabelSet<N> {
    fn eq(&self, other: &Self) -> bool {
        self.classes() == other.classes()
    }
}

impl<const N: usize> Eq for CuSemanticLabelSet<N> {}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> CuSemanticLabelSet<2> {
        let mut labels = CuSemanticLabelSet::default();
        labels
            .push(CuSemanticClass::new(CuSemanticClassId(0), "grass", [0, 255, 0, 255]).unwrap())
            .unwrap();
        labels
            .push(
                CuSemanticClass::new(CuSemanticClassId(7), "gravel", [128, 128, 128, 255]).unwrap(),
            )
            .unwrap();
        labels
    }

    #[test]
    fn test_default_class_id_is_unknown() {
        assert!(CuSemanticClassId::default().is_unknown());
        assert!(!CuSemanticClassId(0).is_unknown());
    }

    #[test]
    fn test_label_set_lookup() {
        let labels = labels();
        assert_eq!(labels.class(CuSemanticClassId(7)).unwrap().name(), "gravel");
        assert_eq!(
            labels.class_named("grass").unwrap().id,
            CuSemanticClassId(0)
        );
        assert!(labels.class(CuSemanticClassId::UNKNOWN).is_none());
    }

    #[test]
    fn test_label_set_rejects_overflow_and_duplicates() {
        let mut labels = labels();
        let rock = CuSemanticClass::new(CuSemanticClassId(9), "rock", [0; 4]).unwrap();
        assert!(labels.push(rock).is_err());

        let mut labels = CuSemanticLabelSet::<4>::default();
        let grass = CuSemanticClass::new(CuSemanticClassId(0), "grass", [0; 4]).unwrap();
        labels.push(grass.clone()).unwrap();
        assert!(labels.push(grass).is_err());
        assert_eq!(labels.classes().len(), 1);
    }

    #[test]
    fn test_class_name_capacity() {
        let longest = "a".repeat(CuSemanticClass::NAME_CAPACITY);
        let class = CuSemanticClass::new(CuSemanticClassId(1), &longest, [0; 4]).unwrap();
        assert_eq!(class.name(), longest);

        let too_long = "a".repeat(CuSemanticClass::NAME_CAPACITY + 1);
        assert!(CuSemanticClass::new(CuSemanticClassId(1), &too_long, [0; 4]).is_err());
    }

    #[test]
    fn test_label_set_bincode_roundtrip() {
        let labels = labels();
        let config = bincode::config::standard();
        let bytes = bincode::encode_to_vec(&labels, config).unwrap();
        let (decoded, used): (CuSemanticLabelSet<2>, usize) =
            bincode::decode_from_slice(&bytes, config).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(decoded, labels);
    }
}
