use crate::core::common::handle_collections::handle_impl;

#[salsa::interned(debug)]
pub(crate) struct Ty<'db> {
    #[returns(ref)]
    pub(crate) kind: TyKind<'db>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum TyKind<'db> {
    Unit,
    Bottom,
    Bool,
    Signed(SignedIntTy),
    Unsigned(UnsignedIntTy),
    Function {
        parameters: Vec<Ty<'db>>,
        r#return: Ty<'db>,
    },
    InferenceVariable(InferenceVariable),
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum SignedIntTy {
    I32,
    I64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum UnsignedIntTy {
    U32,
    U64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, salsa::SalsaValue)]
pub(crate) enum InferenceVariable {
    General(GeneralVariableHandle),
    Integer(IntegerVariableHandle),
}

handle_impl!(pub(crate) GeneralVariableHandle);
handle_impl!(pub(crate) IntegerVariableHandle);

impl<'db> Ty<'db> {
    pub(crate) fn unit(db: &'db dyn crate::Db) -> Self {
        Self::new(db, TyKind::Unit)
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

    pub(crate) fn function(
        db: &'db dyn crate::Db,
        parameters: Vec<Ty<'db>>,
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
            "Unit" => Self::unit(db),
            "Bottom" => Self::bottom(db),
            "Bool" => Self::bool(db),
            "I32" => Self::signed(db, SignedIntTy::I32),
            "I64" => Self::signed(db, SignedIntTy::I64),
            "U32" => Self::unsigned(db, UnsignedIntTy::U32),
            "U64" => Self::unsigned(db, UnsignedIntTy::U64),
            _ => return None,
        })
    }

    pub(crate) fn is_poisoned(self, db: &'db dyn crate::Db) -> bool {
        match self.kind(db) {
            TyKind::Error => true,
            TyKind::Function {
                parameters,
                r#return,
            } => {
                parameters.iter().any(|parameter| parameter.is_poisoned(db))
                    || r#return.is_poisoned(db)
            }
            _ => false,
        }
    }

    pub(crate) fn display(self, db: &'db dyn crate::Db) -> String {
        match self.kind(db) {
            TyKind::Unit => "()".to_string(),
            TyKind::Bottom => "Bottom".to_string(),
            TyKind::Bool => "Bool".to_string(),
            TyKind::Signed(SignedIntTy::I32) => "I32".to_string(),
            TyKind::Signed(SignedIntTy::I64) => "I64".to_string(),
            TyKind::Unsigned(UnsignedIntTy::U32) => "U32".to_string(),
            TyKind::Unsigned(UnsignedIntTy::U64) => "U64".to_string(),
            TyKind::Function {
                parameters,
                r#return,
            } => {
                let parameters: Vec<String> = parameters
                    .iter()
                    .map(|parameter| parameter.display(db))
                    .collect();
                format!("({}) -> {}", parameters.join(", "), r#return.display(db))
            }
            TyKind::InferenceVariable(InferenceVariable::General(_)) => "{unknown}".to_string(),
            TyKind::InferenceVariable(InferenceVariable::Integer(_)) => "{integer}".to_string(),
            TyKind::Error => "Error".to_string(),
        }
    }
}
