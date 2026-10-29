use std::fmt;
use std::ops::RangeInclusive;

use crate::core::common::handle_collections::handle_impl;

#[salsa::interned(debug)]
pub(crate) struct Ty<'db> {
    #[returns(ref)]
    pub(crate) kind: TyKind<'db>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum TyKind<'db> {
    Bottom,
    Bool,
    Signed(SignedIntTy),
    Unsigned(UnsignedIntTy),
    Tuple(Box<[Ty<'db>]>),
    Function {
        parameters: Box<[Ty<'db>]>,
        r#return: Ty<'db>,
    },
    InferenceVariable(InferenceVariable),
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum SignedIntTy {
    I8,
    I16,
    I32,
    I64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum UnsignedIntTy {
    U8,
    U16,
    U32,
    U64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum InferenceVariable {
    General(GeneralVariableHandle),
    Integer(IntegerVariableHandle),
}

pub(crate) struct TyDisplay<'db> {
    ty: Ty<'db>,
    db: &'db dyn crate::Db,
}

handle_impl!(pub(crate) GeneralVariableHandle);
handle_impl!(pub(crate) IntegerVariableHandle);

impl<'db> Ty<'db> {
    pub(crate) fn unit(db: &'db dyn crate::Db) -> Self {
        Self::tuple(db, Box::new([]))
    }

    pub(crate) fn bottom(db: &'db dyn crate::Db) -> Self {
        Self::new(db, TyKind::Bottom)
    }

    pub(crate) fn bool(db: &'db dyn crate::Db) -> Self {
        Self::new(db, TyKind::Bool)
    }

    pub(crate) fn error(db: &'db dyn crate::Db) -> Self {
        Self::new(db, TyKind::Error)
    }

    pub(crate) fn signed(db: &'db dyn crate::Db, ty: SignedIntTy) -> Self {
        Self::new(db, TyKind::Signed(ty))
    }

    pub(crate) fn unsigned(db: &'db dyn crate::Db, ty: UnsignedIntTy) -> Self {
        Self::new(db, TyKind::Unsigned(ty))
    }

    pub(crate) fn tuple(db: &'db dyn crate::Db, elements: Box<[Ty<'db>]>) -> Self {
        Self::new(db, TyKind::Tuple(elements))
    }

    pub(crate) fn function(
        db: &'db dyn crate::Db,
        parameters: Box<[Ty<'db>]>,
        r#return: Ty<'db>,
    ) -> Self {
        Self::new(
            db,
            TyKind::Function {
                parameters,
                r#return,
            },
        )
    }

    pub(crate) fn to_primitive(db: &'db dyn crate::Db, name: &str) -> Option<Self> {
        Some(match name {
            "Bottom" => Self::bottom(db),
            "Bool" => Self::bool(db),
            "I8" => Self::signed(db, SignedIntTy::I8),
            "I16" => Self::signed(db, SignedIntTy::I16),
            "I32" => Self::signed(db, SignedIntTy::I32),
            "I64" => Self::signed(db, SignedIntTy::I64),
            "U8" => Self::unsigned(db, UnsignedIntTy::U8),
            "U16" => Self::unsigned(db, UnsignedIntTy::U16),
            "U32" => Self::unsigned(db, UnsignedIntTy::U32),
            "U64" => Self::unsigned(db, UnsignedIntTy::U64),
            _ => return None,
        })
    }

    /// Returns the values that the integer type `self` can hold, or `None` if it isn't one.
    pub(crate) fn integer_range(self, db: &'db dyn crate::Db) -> Option<RangeInclusive<i128>> {
        let (min, max): (i128, i128) = match self.kind(db) {
            TyKind::Signed(SignedIntTy::I8) => (i8::MIN.into(), i8::MAX.into()),
            TyKind::Signed(SignedIntTy::I16) => (i16::MIN.into(), i16::MAX.into()),
            TyKind::Signed(SignedIntTy::I32) => (i32::MIN.into(), i32::MAX.into()),
            TyKind::Signed(SignedIntTy::I64) => (i64::MIN.into(), i64::MAX.into()),
            TyKind::Unsigned(UnsignedIntTy::U8) => (0, u8::MAX.into()),
            TyKind::Unsigned(UnsignedIntTy::U16) => (0, u16::MAX.into()),
            TyKind::Unsigned(UnsignedIntTy::U32) => (0, u32::MAX.into()),
            TyKind::Unsigned(UnsignedIntTy::U64) => (0, u64::MAX.into()),
            _ => return None,
        };
        Some(min..=max)
    }

    /// Returns an integer type that can hold `value`, to suggest instead of the integer type
    /// `self`.
    ///
    /// Like rustc, the unsigned type of the same size comes first, then the bigger types of the
    /// same signedness, from smallest to biggest.
    pub(crate) fn wider_integer_type(self, db: &'db dyn crate::Db, value: i128) -> Option<Self> {
        let signed = |ty| Self::signed(db, ty);
        let unsigned = |ty| Self::unsigned(db, ty);
        let candidates = match self.kind(db) {
            TyKind::Signed(SignedIntTy::I8) => vec![
                unsigned(UnsignedIntTy::U8),
                signed(SignedIntTy::I16),
                signed(SignedIntTy::I32),
                signed(SignedIntTy::I64),
            ],
            TyKind::Signed(SignedIntTy::I16) => vec![
                unsigned(UnsignedIntTy::U16),
                signed(SignedIntTy::I32),
                signed(SignedIntTy::I64),
            ],
            TyKind::Signed(SignedIntTy::I32) => {
                vec![unsigned(UnsignedIntTy::U32), signed(SignedIntTy::I64)]
            }
            TyKind::Unsigned(UnsignedIntTy::U8) => vec![
                unsigned(UnsignedIntTy::U16),
                unsigned(UnsignedIntTy::U32),
                unsigned(UnsignedIntTy::U64),
            ],
            TyKind::Unsigned(UnsignedIntTy::U16) => {
                vec![unsigned(UnsignedIntTy::U32), unsigned(UnsignedIntTy::U64)]
            }
            TyKind::Signed(SignedIntTy::I64) | TyKind::Unsigned(UnsignedIntTy::U32) => {
                vec![unsigned(UnsignedIntTy::U64)]
            }
            // no integer type is bigger than `U64`, and the rest aren't integer types
            TyKind::Unsigned(UnsignedIntTy::U64)
            | TyKind::Bottom
            | TyKind::Bool
            | TyKind::Tuple(_)
            | TyKind::Function { .. }
            | TyKind::InferenceVariable(_)
            | TyKind::Error => Vec::new(),
        };
        candidates.into_iter().find(|candidate| {
            candidate
                .integer_range(db)
                .is_some_and(|range| range.contains(&value))
        })
    }

    pub(crate) fn children(self, db: &'db dyn crate::Db) -> impl Iterator<Item = Ty<'db>> + 'db {
        let (children, last): (&'db [Ty<'db>], Option<Ty<'db>>) = match self.kind(db) {
            TyKind::Tuple(elements) => (elements, None),
            TyKind::Function {
                parameters,
                r#return,
            } => (parameters, Some(*r#return)),
            TyKind::Bottom
            | TyKind::Bool
            | TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::InferenceVariable(_)
            | TyKind::Error => (&[], None),
        };
        children.iter().copied().chain(last)
    }

    pub(crate) fn is_poisoned(self, db: &'db dyn crate::Db) -> bool {
        matches!(self.kind(db), TyKind::Error)
            || self.children(db).any(|child| child.is_poisoned(db))
    }

    pub(crate) fn map_children(
        self,
        db: &'db dyn crate::Db,
        mut f: impl FnMut(Ty<'db>) -> Ty<'db>,
    ) -> Ty<'db> {
        match self.kind(db) {
            TyKind::Tuple(elements) => {
                Ty::tuple(db, elements.iter().map(|&element| f(element)).collect())
            }
            TyKind::Function {
                parameters,
                r#return,
            } => {
                let parameters = parameters.iter().map(|&parameter| f(parameter)).collect();
                Ty::function(db, parameters, f(*r#return))
            }
            TyKind::Bottom
            | TyKind::Bool
            | TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::InferenceVariable(_)
            | TyKind::Error => self,
        }
    }

    pub(crate) fn display(self, db: &'db dyn crate::Db) -> TyDisplay<'db> {
        TyDisplay { ty: self, db }
    }
}

impl fmt::Display for TyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let db = self.db;
        match self.ty.kind(db) {
            TyKind::Bottom => f.write_str("Bottom"),
            TyKind::Bool => f.write_str("Bool"),
            TyKind::Signed(SignedIntTy::I8) => f.write_str("I8"),
            TyKind::Signed(SignedIntTy::I16) => f.write_str("I16"),
            TyKind::Signed(SignedIntTy::I32) => f.write_str("I32"),
            TyKind::Signed(SignedIntTy::I64) => f.write_str("I64"),
            TyKind::Unsigned(UnsignedIntTy::U8) => f.write_str("U8"),
            TyKind::Unsigned(UnsignedIntTy::U16) => f.write_str("U16"),
            TyKind::Unsigned(UnsignedIntTy::U32) => f.write_str("U32"),
            TyKind::Unsigned(UnsignedIntTy::U64) => f.write_str("U64"),
            TyKind::Tuple(elements) => {
                f.write_str("(")?;
                for (index, element) in elements.iter().enumerate() {
                    if 0 < index {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", element.display(db))?;
                }
                if elements.len() == 1 {
                    f.write_str(",")?;
                }
                f.write_str(")")
            }
            TyKind::Function {
                parameters,
                r#return,
            } => {
                f.write_str("(")?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if 0 < index {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", parameter.display(db))?;
                }
                write!(f, ") -> {}", r#return.display(db))
            }
            TyKind::InferenceVariable(InferenceVariable::General(_)) => f.write_str("{unknown}"),
            TyKind::InferenceVariable(InferenceVariable::Integer(_)) => f.write_str("{integer}"),
            TyKind::Error => f.write_str("{error}"),
        }
    }
}
