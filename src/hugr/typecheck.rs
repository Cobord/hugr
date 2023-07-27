//! Simple type checking - takes a hugr and some extra info and checks whether
//! the types at the sources of each wire match those of the targets

use lazy_static::lazy_static;

use std::collections::HashSet;

use crate::hugr::*;

// For static typechecking
use crate::ops::ConstValue;
use crate::types::{ClassicRow, ClassicType, Container, HashableType, PrimType, TypeRow};

use crate::ops::constant::{HugrIntValueStore, HugrIntWidthStore, HUGR_MAX_INT_WIDTH};

/// Errors that arise from typechecking constants
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ConstTypeError {
    /// This case hasn't been implemented. Possibly because we don't have value
    /// constructors to check against it
    #[error("Unimplemented: there are no constants of type {0}")]
    Unimplemented(ClassicType),
    /// The value exceeds the max value of its `I<n>` type
    /// E.g. checking 300 against I8
    #[error("Const int {1} too large for type I{0}")]
    IntTooLarge(HugrIntWidthStore, HugrIntValueStore),
    /// Width (n) of an `I<n>` type doesn't fit into a HugrIntWidthStore
    #[error("Int type too large: I{0}")]
    IntWidthTooLarge(HugrIntWidthStore),
    /// The width of an integer type wasn't a power of 2
    #[error("The int type I{0} is invalid, because {0} is not a power of 2")]
    IntWidthInvalid(HugrIntWidthStore),
    /// Expected width (packed with const int) doesn't match type
    #[error("Type mismatch for int: expected I{0}, but found I{1}")]
    IntWidthMismatch(HugrIntWidthStore, HugrIntWidthStore),
    /// Found a Var type constructor when we're checking a const val
    #[error("Type of a const value can't be Var")]
    ConstCantBeVar,
    /// The length of the tuple value doesn't match the length of the tuple type
    #[error("Tuple of wrong length")]
    TupleWrongLength,
    /// Tag for a sum value exceeded the number of variants
    #[error("Tag of Sum value is invalid")]
    InvalidSumTag,
    /// A mismatch between the type expected and the actual type of the constant
    #[error("Type mismatch for const - expected {0}, found {1}")]
    TypeMismatch(ClassicType, ClassicType),
    /// A mismatch between the embedded type and the type we're checking
    /// against, as above, but for rows instead of simple types
    #[error("Type mismatch for const - expected {0}, found {1}")]
    TypeRowMismatch(ClassicRow, ClassicRow),
}

lazy_static! {
    static ref VALID_WIDTHS: HashSet<HugrIntWidthStore> =
        HashSet::from_iter((0..8).map(|a| HugrIntWidthStore::pow(2, a)));
}

/// Per the spec, valid widths for integers are 2^n for all n in [0,7]
fn check_valid_width(width: HugrIntWidthStore) -> Result<(), ConstTypeError> {
    if width > HUGR_MAX_INT_WIDTH {
        return Err(ConstTypeError::IntWidthTooLarge(width));
    }

    if VALID_WIDTHS.contains(&width) {
        Ok(())
    } else {
        Err(ConstTypeError::IntWidthInvalid(width))
    }
}

fn map_vals<T: PrimType, T2: PrimType>(
    container: Container<T>,
    f: &impl Fn(T) -> T2,
) -> Container<T2> {
    fn map_row<T: PrimType, T2: PrimType>(
        row: TypeRow<T>,
        f: &impl Fn(T) -> T2,
    ) -> Box<TypeRow<T2>> {
        Box::new(TypeRow::from(
            row.into_owned().into_iter().map(f).collect::<Vec<T2>>(),
        ))
    }
    match container {
        Container::List(elem) => Container::List(Box::new(f(*elem))),
        Container::Map(kv) => {
            let (k, v) = *kv;
            Container::Map(Box::new((k, f(v))))
        }
        Container::Tuple(elems) => Container::Tuple(map_row(*elems, f)),
        Container::Sum(variants) => Container::Sum(map_row(*variants, f)),
        Container::Array(elem, sz) => Container::Array(Box::new(f(*elem)), sz),
        Container::Alias(s) => Container::Alias(s),
        Container::Opaque(custom) => Container::Opaque(custom),
    }
}

/// Typecheck a constant value
pub fn typecheck_const(typ: &ClassicType, val: &ConstValue) -> Result<(), ConstTypeError> {
    match (typ, val) {
        (ClassicType::Hashable(HashableType::Int(exp_width)), ConstValue::Int { value, width }) => {
            // Check that the types make sense
            check_valid_width(*exp_width)?;
            check_valid_width(*width)?;
            // Check that the terms make sense against the types
            if exp_width == width {
                let max_value = if *width == HUGR_MAX_INT_WIDTH {
                    HugrIntValueStore::MAX
                } else {
                    HugrIntValueStore::pow(2, *width as u32) - 1
                };
                if value <= &max_value {
                    Ok(())
                } else {
                    Err(ConstTypeError::IntTooLarge(*width, *value))
                }
            } else {
                Err(ConstTypeError::IntWidthMismatch(*exp_width, *width))
            }
        }
        (ClassicType::F64, ConstValue::F64(_)) => Ok(()),
        (ty @ ClassicType::Container(c), tm) => match (c, tm) {
            (Container::Tuple(row), ConstValue::Tuple(xs)) => {
                if row.len() != xs.len() {
                    return Err(ConstTypeError::TupleWrongLength);
                }
                for (ty, tm) in row.iter().zip(xs.iter()) {
                    typecheck_const(ty, tm)?
                }
                Ok(())
            }
            (Container::Tuple(_), _) => {
                Err(ConstTypeError::TypeMismatch(ty.clone(), tm.const_type()))
            }
            (Container::Sum(row), ConstValue::Sum { tag, variants, val }) => {
                if tag > &row.len() {
                    return Err(ConstTypeError::InvalidSumTag);
                }
                if **row != *variants {
                    return Err(ConstTypeError::TypeRowMismatch(
                        *row.clone(),
                        variants.clone(),
                    ));
                }
                let ty = variants.get(*tag).unwrap();
                typecheck_const(ty, val.as_ref())
            }
            (Container::Sum(_), _) => {
                Err(ConstTypeError::TypeMismatch(ty.clone(), tm.const_type()))
            }
            (Container::Opaque(ty), ConstValue::Opaque(ty_act, _val)) => {
                if ty_act != ty {
                    return Err(ConstTypeError::TypeMismatch(
                        ty.clone().into(),
                        ty_act.clone().into(),
                    ));
                }
                Ok(())
            }
            _ => Err(ConstTypeError::Unimplemented(ty.clone())),
        },
        (ClassicType::Hashable(HashableType::Container(c)), tm) => {
            // Here we deliberately build malformed Container-of-Hashable types
            // (rather than Hashable-of-Container) in order to reuse logic above
            typecheck_const(
                &ClassicType::Container(map_vals(c.clone(), &ClassicType::Hashable)),
                tm,
            )
        }
        (ty @ ClassicType::Graph(_), _) => Err(ConstTypeError::Unimplemented(ty.clone())),
        (ty @ ClassicType::Hashable(HashableType::String), _) => {
            Err(ConstTypeError::Unimplemented(ty.clone()))
        }
        (ClassicType::Hashable(HashableType::Variable(_)), _) => {
            Err(ConstTypeError::ConstCantBeVar)
        }
        (ty, _) => Err(ConstTypeError::TypeMismatch(ty.clone(), val.const_type())),
    }
}

#[cfg(test)]
mod test {
    use cool_asserts::assert_matches;

    use crate::{classic_row, types::ClassicType};

    use super::*;

    #[test]
    fn test_typecheck_const() {
        const INT: ClassicType = ClassicType::int::<64>();
        typecheck_const(&INT, &ConstValue::i64(3)).unwrap();
        assert_eq!(
            typecheck_const(&HashableType::Int(32).into(), &ConstValue::i64(3)),
            Err(ConstTypeError::IntWidthMismatch(32, 64))
        );
        typecheck_const(&ClassicType::F64, &ConstValue::F64(17.4)).unwrap();
        assert_eq!(
            typecheck_const(&ClassicType::F64, &ConstValue::i64(5)),
            Err(ConstTypeError::TypeMismatch(
                ClassicType::F64,
                ClassicType::i64()
            ))
        );
        let tuple_ty = ClassicType::new_tuple(classic_row![INT, ClassicType::F64,]);
        typecheck_const(
            &tuple_ty,
            &ConstValue::Tuple(vec![ConstValue::i64(7), ConstValue::F64(5.1)]),
        )
        .unwrap();
        assert_matches!(
            typecheck_const(
                &tuple_ty,
                &ConstValue::Tuple(vec![ConstValue::F64(4.8), ConstValue::i64(2)])
            ),
            Err(ConstTypeError::TypeMismatch(_, _))
        );
        assert_eq!(
            typecheck_const(
                &tuple_ty,
                &ConstValue::Tuple(vec![
                    ConstValue::i64(5),
                    ConstValue::F64(3.3),
                    ConstValue::i64(2)
                ])
            ),
            Err(ConstTypeError::TupleWrongLength)
        );
    }
}
