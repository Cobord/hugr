//! Dataflow types

use std::borrow::Cow;

#[cfg(feature = "pyo3")]
use pyo3::prelude::*;
use smol_str::SmolStr;

use super::{custom::CustomType, Signature};
use crate::resource::ResourceSet;

/// A type that represents concrete data.
///
/// TODO: Derive pyclass
///
/// TODO: Compare performance vs flattening this into a single enum
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SimpleType {
    Classic(ClassicType),
    Quantum(QuantumType),
}

/// A type that represents concrete classical data.
///
/// Uses `Box`es on most variants to reduce the memory footprint.
///
/// TODO: Derive pyclass
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ClassicType {
    Variable(SmolStr),
    Nat,
    Int,
    #[default]
    Bit,
    Graph(Box<(ResourceSet, Signature)>),
    Pair(Box<(ClassicType, ClassicType)>),
    List(Box<ClassicType>),
    Map(Box<(ClassicType, ClassicType)>),
    Struct(Box<TypeRow>),
    /// An opaque operation that can be downcasted by the extensions that define it.
    Opaque(Box<CustomType>),
}

/// A type that represents concrete quantum data.
///
/// TODO: Derive pyclass
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum QuantumType {
    #[default]
    Qubit,
    Money,
    Array(Box<QuantumType>, usize),
}

impl SimpleType {
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::Quantum(_))
    }

    pub fn is_classical(&self) -> bool {
        matches!(self, Self::Classic(_))
    }
}

impl Default for SimpleType {
    fn default() -> Self {
        Self::Quantum(Default::default())
    }
}

impl From<ClassicType> for SimpleType {
    fn from(typ: ClassicType) -> Self {
        Self::Classic(typ)
    }
}

impl From<QuantumType> for SimpleType {
    fn from(typ: QuantumType) -> Self {
        Self::Quantum(typ)
    }
}

/// List of types, used for function signatures.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
#[non_exhaustive]
pub struct TypeRow {
    /// The datatypes in the row.
    pub types: Cow<'static, [SimpleType]>,
}

#[cfg_attr(feature = "pyo3", pymethods)]
impl TypeRow {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.types.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    #[inline(always)]
    pub fn purely_linear(&self) -> bool {
        self.types.iter().all(|typ| typ.is_linear())
    }

    #[inline(always)]
    pub fn purely_classical(&self) -> bool {
        !self.types.iter().all(SimpleType::is_classical)
    }
}
impl TypeRow {
    pub fn new(types: impl Into<Cow<'static, [SimpleType]>>) -> Self {
        Self {
            types: types.into(),
        }
    }

    pub fn empty() -> Self {
        static EMPTY: &[SimpleType] = &[];
        Self::new(EMPTY)
    }

    /// Iterator over the types in the row.
    pub fn iter(&self) -> impl Iterator<Item = &SimpleType> {
        self.types.iter()
    }
}

impl Default for TypeRow {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> From<T> for TypeRow
where
    T: Into<Cow<'static, [SimpleType]>>,
{
    fn from(types: T) -> Self {
        Self::new(types.into())
    }
}
