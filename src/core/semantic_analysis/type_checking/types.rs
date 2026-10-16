use std::fmt;

use crate::core::common::handle_collections::handle_impl;

#[salsa::interned(debug)]
pub(in crate::core) struct Ty<'db> {
    #[returns(ref)]
    pub(in crate::core) kind: TyKind<'db>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(in crate::core) enum TyKind<'db> {
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
pub(in crate::core) enum SignedIntTy {
    I32,
    I64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(in crate::core) enum UnsignedIntTy {
    U32,
    U64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(in crate::core) enum InferenceVariable {
    General(GeneralVariableHandle),
    Integer(IntegerVariableHandle),
}

pub(in crate::core) struct TyDisplay<'db> {
    ty: Ty<'db>,
    db: &'db dyn crate::Db,
}

handle_impl!(pub(in crate::core) GeneralVariableHandle);
handle_impl!(pub(in crate::core) IntegerVariableHandle);

impl<'db> Ty<'db> {
    pub(in crate::core) fn unit(db: &'db dyn crate::Db) -> Self {
        Self::tuple(db, Box::new([]))
    }

    pub(in crate::core) fn bottom(db: &'db dyn crate::Db) -> Self {
        Self::new(db, TyKind::Bottom)
    }

    pub(in crate::core) fn bool(db: &'db dyn crate::Db) -> Self {
        Self::new(db, TyKind::Bool)
    }

    pub(in crate::core) fn error(db: &'db dyn crate::Db) -> Self {
        Self::new(db, TyKind::Error)
    }

    pub(in crate::core) fn signed(db: &'db dyn crate::Db, ty: SignedIntTy) -> Self {
        Self::new(db, TyKind::Signed(ty))
    }

    pub(in crate::core) fn unsigned(db: &'db dyn crate::Db, ty: UnsignedIntTy) -> Self {
        Self::new(db, TyKind::Unsigned(ty))
    }

    pub(in crate::core) fn tuple(db: &'db dyn crate::Db, elements: Box<[Ty<'db>]>) -> Self {
        Self::new(db, TyKind::Tuple(elements))
    }

    pub(in crate::core) fn function(
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

    pub(in crate::core) fn to_primitive(db: &'db dyn crate::Db, name: &str) -> Option<Self> {
        Some(match name {
            "Bottom" => Self::bottom(db),
            "Bool" => Self::bool(db),
            "I32" => Self::signed(db, SignedIntTy::I32),
            "I64" => Self::signed(db, SignedIntTy::I64),
            "U32" => Self::unsigned(db, UnsignedIntTy::U32),
            "U64" => Self::unsigned(db, UnsignedIntTy::U64),
            _ => return None,
        })
    }

    pub(in crate::core) fn children(
        self,
        db: &'db dyn crate::Db,
    ) -> impl Iterator<Item = Ty<'db>> + 'db {
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

    pub(in crate::core) fn is_poisoned(self, db: &'db dyn crate::Db) -> bool {
        matches!(self.kind(db), TyKind::Error)
            || self.children(db).any(|child| child.is_poisoned(db))
    }

    pub(in crate::core) fn map_children(
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

    pub(in crate::core) fn display(self, db: &'db dyn crate::Db) -> TyDisplay<'db> {
        TyDisplay { ty: self, db }
    }
}

impl fmt::Display for TyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let db = self.db;
        match self.ty.kind(db) {
            TyKind::Bottom => f.write_str("Bottom"),
            TyKind::Bool => f.write_str("Bool"),
            TyKind::Signed(SignedIntTy::I32) => f.write_str("I32"),
            TyKind::Signed(SignedIntTy::I64) => f.write_str("I64"),
            TyKind::Unsigned(UnsignedIntTy::U32) => f.write_str("U32"),
            TyKind::Unsigned(UnsignedIntTy::U64) => f.write_str("U64"),
            TyKind::Tuple(elements) => {
                f.write_str("(")?;
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
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
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", parameter.display(db))?;
                }
                write!(f, ") -> {}", r#return.display(db))
            }
            TyKind::InferenceVariable(InferenceVariable::General(_)) => f.write_str("{unknown}"),
            TyKind::InferenceVariable(InferenceVariable::Integer(_)) => f.write_str("{integer}"),
            TyKind::Error => f.write_str("Error"),
        }
    }
}
