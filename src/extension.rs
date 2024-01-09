//! Extensions
//!
//! TODO: YAML declaration and parsing. This should be similar to a plugin
//! system (outside the `types` module), which also parses nested [`OpDef`]s.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use smol_str::SmolStr;
use thiserror::Error;

use crate::hugr::IdentList;
use crate::ops;
use crate::ops::custom::{ExtensionOp, OpaqueOp};
use crate::types::type_param::{check_type_args, TypeArgError};
use crate::types::type_param::{TypeArg, TypeParam};
use crate::types::{check_typevar_decl, CustomType, PolyFuncType, Substitution, TypeBound};

#[allow(dead_code)]
mod infer;
#[cfg(feature = "extension_inference")]
pub use infer::infer_extensions;
pub use infer::{ExtensionSolution, InferExtensionError};

mod op_def;
pub use op_def::{
    CustomSignatureFunc, CustomValidator, OpDef, SignatureFromArgs, SignatureFunc,
    ValidateJustArgs, ValidateTypeArgs,
};
mod type_def;
pub use type_def::{TypeDef, TypeDefBound};
mod const_fold;
pub mod prelude;
pub mod simple_op;
pub mod validate;
pub use const_fold::{ConstFold, ConstFoldResult};
pub use prelude::{PRELUDE, PRELUDE_REGISTRY};

/// Extension Registries store extensions to be looked up e.g. during validation.
#[derive(Clone, Debug)]
pub struct ExtensionRegistry(BTreeMap<ExtensionId, Extension>);

impl ExtensionRegistry {
    /// Gets the Extension with the given name
    pub fn get(&self, name: &str) -> Option<&Extension> {
        self.0.get(name)
    }

    /// Makes a new ExtensionRegistry, validating all the extensions in it
    pub fn try_new(
        value: impl IntoIterator<Item = Extension>,
    ) -> Result<Self, (ExtensionId, SignatureError)> {
        let mut exts = BTreeMap::new();
        for ext in value.into_iter() {
            let prev = exts.insert(ext.name.clone(), ext);
            if let Some(prev) = prev {
                panic!("Multiple extensions with same name: {}", prev.name)
            };
        }
        // Note this potentially asks extensions to validate themselves against other extensions that
        // may *not* be valid themselves yet. It'd be better to order these respecting dependencies,
        // or at least to validate the types first - which we don't do at all yet:
        // TODO https://github.com/CQCL/hugr/issues/624. However, parametrized types could be
        // cyclically dependent, so there is no perfect solution, and this is at least simple.
        let res = ExtensionRegistry(exts);
        for ext in res.0.values() {
            ext.validate(&res).map_err(|e| (ext.name().clone(), e))?;
        }
        Ok(res)
    }
}

impl IntoIterator for ExtensionRegistry {
    type Item = (ExtensionId, Extension);

    type IntoIter = <BTreeMap<ExtensionId, Extension> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// An Extension Registry containing no extensions.
pub const EMPTY_REG: ExtensionRegistry = ExtensionRegistry(BTreeMap::new());

/// An error that can occur in computing the signature of a node.
/// TODO: decide on failure modes
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum SignatureError {
    /// Name mismatch
    #[error("Definition name ({0}) and instantiation name ({1}) do not match.")]
    NameMismatch(SmolStr, SmolStr),
    /// Extension mismatch
    #[error("Definition extension ({0:?}) and instantiation extension ({1:?}) do not match.")]
    ExtensionMismatch(ExtensionId, ExtensionId),
    /// When the type arguments of the node did not match the params declared by the OpDef
    #[error("Type arguments of node did not match params declared by definition: {0}")]
    TypeArgMismatch(#[from] TypeArgError),
    /// Invalid type arguments
    #[error("Invalid type arguments for operation")]
    InvalidTypeArgs,
    /// The Extension Registry did not contain an Extension referenced by the Signature
    #[error("Extension '{0}' not found")]
    ExtensionNotFound(ExtensionId),
    /// The Extension was found in the registry, but did not contain the Type(Def) referenced in the Signature
    #[error("Extension '{exn}' did not contain expected TypeDef '{typ}'")]
    ExtensionTypeNotFound { exn: ExtensionId, typ: SmolStr },
    /// The bound recorded for a CustomType doesn't match what the TypeDef would compute
    #[error("Bound on CustomType ({actual}) did not match TypeDef ({expected})")]
    WrongBound {
        actual: TypeBound,
        expected: TypeBound,
    },
    /// A Type Variable's cache of its declared kind is incorrect
    #[error("Type Variable claims to be {cached:?} but actual declaration {actual:?}")]
    TypeVarDoesNotMatchDeclaration {
        actual: TypeParam,
        cached: TypeParam,
    },
    /// A type variable that was used has not been declared
    #[error("Type variable {idx} was not declared ({num_decls} in scope)")]
    FreeTypeVar { idx: usize, num_decls: usize },
    /// The type stored in a [LeafOp::TypeApply] is not what we compute from the
    /// [ExtensionRegistry].
    ///
    /// [LeafOp::TypeApply]: crate::ops::LeafOp::TypeApply
    #[error("Incorrect result of type application - cached {cached} but expected {expected}")]
    TypeApplyIncorrectCache {
        cached: PolyFuncType,
        expected: PolyFuncType,
    },
}

/// Concrete instantiations of types and operations defined in extensions.
trait CustomConcrete {
    fn def_name(&self) -> &SmolStr;
    fn type_args(&self) -> &[TypeArg];
    fn parent_extension(&self) -> &ExtensionId;
}

impl CustomConcrete for OpaqueOp {
    fn def_name(&self) -> &SmolStr {
        self.name()
    }

    fn type_args(&self) -> &[TypeArg] {
        self.args()
    }

    fn parent_extension(&self) -> &ExtensionId {
        self.extension()
    }
}

impl CustomConcrete for CustomType {
    fn def_name(&self) -> &SmolStr {
        self.name()
    }

    fn type_args(&self) -> &[TypeArg] {
        self.args()
    }

    fn parent_extension(&self) -> &ExtensionId {
        self.extension()
    }
}

/// Type-parametrised functionality shared between [`TypeDef`] and [`OpDef`].
trait TypeParametrised {
    /// The concrete object built by binding type arguments to parameters
    type Concrete: CustomConcrete;
    /// The extension-unique name.
    fn name(&self) -> &SmolStr;
    /// Type parameters.
    fn params(&self) -> &[TypeParam];
    /// The parent extension.
    fn extension(&self) -> &ExtensionId;
    /// Check provided type arguments are valid against parameters.
    fn check_args_impl(&self, args: &[TypeArg]) -> Result<(), SignatureError> {
        check_type_args(args, self.params()).map_err(SignatureError::TypeArgMismatch)
    }
}

/// A constant value provided by a extension.
/// Must be an instance of a type available to the extension.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ExtensionValue {
    extension: ExtensionId,
    name: SmolStr,
    typed_value: ops::Const,
}

impl ExtensionValue {
    /// Returns a reference to the typed value of this [`ExtensionValue`].
    pub fn typed_value(&self) -> &ops::Const {
        &self.typed_value
    }

    /// Returns a reference to the name of this [`ExtensionValue`].
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Returns a reference to the extension this [`ExtensionValue`] belongs to.
    pub fn extension(&self) -> &ExtensionId {
        &self.extension
    }
}

/// A unique identifier for a extension.
///
/// The actual [`Extension`] is stored externally.
pub type ExtensionId = IdentList;

/// A extension is a set of capabilities required to execute a graph.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Extension {
    /// Unique identifier for the extension.
    pub name: ExtensionId,
    /// Other extensions defining types used by this extension.
    /// That is, an upper-bound on the types that can be returned by
    /// computing the signature of any operation in this extension,
    /// for any possible [TypeArg].
    pub extension_reqs: ExtensionSet,
    /// Types defined by this extension.
    types: HashMap<SmolStr, TypeDef>,
    /// Static values defined by this extension.
    values: HashMap<SmolStr, ExtensionValue>,
    /// Operation declarations with serializable definitions.
    // Note: serde will serialize this because we configure with `features=["rc"]`.
    // That will clone anything that has multiple references, but each
    // OpDef should appear exactly once in this map (keyed by its name),
    // and the other references to the OpDef are from ExternalOp's in the Hugr
    // (which are serialized as OpaqueOp's i.e. Strings).
    operations: HashMap<SmolStr, Arc<op_def::OpDef>>,
}

impl Extension {
    /// Creates a new extension with the given name.
    pub fn new(name: ExtensionId) -> Self {
        Self::new_with_reqs(name, Default::default())
    }

    /// Creates a new extension with the given name and requirements.
    pub fn new_with_reqs(name: ExtensionId, extension_reqs: ExtensionSet) -> Self {
        Self {
            name,
            extension_reqs,
            types: Default::default(),
            values: Default::default(),
            operations: Default::default(),
        }
    }

    /// Allows read-only access to the operations in this Extension
    pub fn get_op(&self, op_name: &str) -> Option<&Arc<op_def::OpDef>> {
        self.operations.get(op_name)
    }

    /// Allows read-only access to the types in this Extension
    pub fn get_type(&self, type_name: &str) -> Option<&type_def::TypeDef> {
        self.types.get(type_name)
    }

    /// Allows read-only access to the values in this Extension
    pub fn get_value(&self, type_name: &str) -> Option<&ExtensionValue> {
        self.values.get(type_name)
    }

    /// Returns the name of the extension.
    pub fn name(&self) -> &ExtensionId {
        &self.name
    }

    /// Iterator over the operations of this [`Extension`].
    pub fn operations(&self) -> impl Iterator<Item = (&SmolStr, &Arc<OpDef>)> {
        self.operations.iter()
    }

    /// Iterator over the types of this [`Extension`].
    pub fn types(&self) -> impl Iterator<Item = (&SmolStr, &TypeDef)> {
        self.types.iter()
    }

    /// Add a named static value to the extension.
    pub fn add_value(
        &mut self,
        name: impl Into<SmolStr>,
        typed_value: ops::Const,
    ) -> Result<&mut ExtensionValue, ExtensionBuildError> {
        let extension_value = ExtensionValue {
            extension: self.name.clone(),
            name: name.into(),
            typed_value,
        };
        match self.values.entry(extension_value.name.clone()) {
            Entry::Occupied(_) => Err(ExtensionBuildError::OpDefExists(extension_value.name)),
            Entry::Vacant(ve) => Ok(ve.insert(extension_value)),
        }
    }

    /// Instantiate an [`ExtensionOp`] which references an [`OpDef`] in this extension.
    pub fn instantiate_extension_op(
        &self,
        op_name: &str,
        args: impl Into<Vec<TypeArg>>,
        ext_reg: &ExtensionRegistry,
    ) -> Result<ExtensionOp, SignatureError> {
        let op_def = self.get_op(op_name).expect("Op not found.");
        ExtensionOp::new(op_def.clone(), args, ext_reg)
    }

    // Validates against a registry, which we can assume includes this extension itself.
    // (TODO deal with the registry itself containing invalid extensions!)
    fn validate(&self, all_exts: &ExtensionRegistry) -> Result<(), SignatureError> {
        // We should validate TypeParams of TypeDefs too - https://github.com/CQCL/hugr/issues/624
        for op_def in self.operations.values() {
            op_def.validate(all_exts)?;
        }
        Ok(())
    }
}

impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// An error that can occur in computing the signature of a node.
/// TODO: decide on failure modes
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ExtensionBuildError {
    /// Existing [`OpDef`]
    #[error("Extension already has an op called {0}.")]
    OpDefExists(SmolStr),
    /// Existing [`TypeDef`]
    #[error("Extension already has an type called {0}.")]
    TypeDefExists(SmolStr),
}

/// A set of extensions identified by their unique [`ExtensionId`].
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtensionSet(BTreeSet<ExtensionId>);

impl ExtensionSet {
    /// Creates a new empty extension set.
    pub const fn new() -> Self {
        Self(BTreeSet::new())
    }

    /// Adds a extension to the set.
    pub fn insert(&mut self, extension: &ExtensionId) {
        self.0.insert(extension.clone());
    }

    /// Adds a type var (which must have been declared as a [TypeParam::Extensions]) to this set
    pub fn insert_type_var(&mut self, idx: usize) {
        // Represent type vars as string representation of DeBruijn index.
        // This is not a legal IdentList or ExtensionId so should not conflict.
        self.0
            .insert(ExtensionId::new_unchecked(idx.to_string().as_str()));
    }

    /// Returns `true` if the set contains the given extension.
    pub fn contains(&self, extension: &ExtensionId) -> bool {
        self.0.contains(extension)
    }

    /// Returns `true` if the set is a subset of `other`.
    pub fn is_subset(&self, other: &Self) -> bool {
        self.0.is_subset(&other.0)
    }

    /// Returns `true` if the set is a superset of `other`.
    pub fn is_superset(&self, other: &Self) -> bool {
        self.0.is_superset(&other.0)
    }

    /// Create a extension set with a single element.
    pub fn singleton(extension: &ExtensionId) -> Self {
        let mut set = Self::new();
        set.insert(extension);
        set
    }

    /// An ExtensionSet containing a single type variable
    /// (which must have been declared as a [TypeParam::Extensions])
    pub fn type_var(idx: usize) -> Self {
        let mut set = Self::new();
        set.insert_type_var(idx);
        set
    }

    /// Returns the union of two extension sets.
    pub fn union(mut self, other: &Self) -> Self {
        self.0.extend(other.0.iter().cloned());
        self
    }

    /// Returns the union of an arbitrary collection of [ExtensionSet]s
    pub fn union_over(sets: impl IntoIterator<Item = Self>) -> Self {
        // `union` clones the receiver, which we do not need to do here
        let mut res = ExtensionSet::new();
        for s in sets {
            res.0.extend(s.0)
        }
        res
    }

    /// The things in other which are in not in self
    pub fn missing_from(&self, other: &Self) -> Self {
        ExtensionSet::from_iter(other.0.difference(&self.0).cloned())
    }

    /// Iterate over the contained ExtensionIds
    pub fn iter(&self) -> impl Iterator<Item = &ExtensionId> {
        self.0.iter()
    }

    /// True if this set contains no [ExtensionId]s
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn validate(&self, params: &[TypeParam]) -> Result<(), SignatureError> {
        self.iter()
            .filter_map(as_typevar)
            .try_for_each(|var_idx| check_typevar_decl(params, var_idx, &TypeParam::Extensions))
    }

    pub(crate) fn substitute(&self, t: &impl Substitution) -> Self {
        Self::from_iter(self.0.iter().flat_map(|e| match as_typevar(e) {
            None => vec![e.clone()],
            Some(i) => match t.apply_var(i, &TypeParam::Extensions) {
                TypeArg::Extensions{es} => es.iter().cloned().collect::<Vec<_>>(),
                _ => panic!("value for type var was not extension set - type scheme should be validated first"),
            },
        }))
    }
}

fn as_typevar(e: &ExtensionId) -> Option<usize> {
    // Type variables are represented as radix-10 numbers, which are illegal
    // as standard ExtensionIds. Hence if an ExtensionId starts with a digit,
    // we assume it must be a type variable, and fail fast if it isn't.
    match e.chars().next() {
        Some(c) if c.is_ascii_digit() => Some(str::parse(e).unwrap()),
        _ => None,
    }
}

impl Display for ExtensionSet {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_list().entries(self.0.iter()).finish()
    }
}

impl FromIterator<ExtensionId> for ExtensionSet {
    fn from_iter<I: IntoIterator<Item = ExtensionId>>(iter: I) -> Self {
        Self(BTreeSet::from_iter(iter))
    }
}
