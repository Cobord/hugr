//! Helper macros.

/// Helper macro for declaring downcast traits that can be cloned when inside a `Box<dyn $trait>`.
/// Implements `Clone` for `Box<dyn $trait>`.
macro_rules! impl_box_clone {
    ($trait:ident, $clone_trait:ident) => {
        /// Auto-implemented trait for cloning `Box<dyn $trait>`.
        pub trait $clone_trait {
            /// Clone the trait object into a Sized box.
            fn clone_box(&self) -> Box<dyn $trait>;
        }

        impl<T> $clone_trait for T
        where
            T: $trait + Clone,
        {
            fn clone_box(&self) -> Box<dyn $trait> {
                Box::new(self.clone())
            }
        }

        impl Clone for Box<dyn $trait> {
            fn clone(&self) -> Box<dyn $trait> {
                self.clone_box()
            }
        }
    };
}
pub(crate) use impl_box_clone;

/// Creates a [`TypeRow`] backed by statically defined data, avoiding allocations.
///
/// Example:
/// ```
/// # use hugr::macros::type_row;
/// # use hugr::types::{ClassicType, SimpleType, Signature, TypeRow};
/// const B: SimpleType = SimpleType::Classic(ClassicType::Bit);
/// let sig: Signature = Signature::new_df(type_row![B, B], type_row![B]);
/// ```
#[allow(unused_macros)]
#[macro_export]
macro_rules! type_row {
    ($($t:ident),*) => {
        {
            use $crate::types;
            static ROW: &[types::SimpleType] = &[$($t),*];
            let row: types::TypeRow = ROW.into();
            row
        }
    };
}
#[allow(unused_imports)]
pub use type_row;
