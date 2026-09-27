use crate::core::common::handle_collections::handle_impl;

handle_impl!(pub(crate) GeneralVariableHandle);
handle_impl!(pub(crate) IntegerVariableHandle);

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
        return_type: Ty<'db>,
    },
    UnificationVariable(UnificationVariable),
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
pub(crate) enum UnificationVariable {
    General(GeneralVariableHandle),
    Integer(IntegerVariableHandle),
}

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
        return_type: Ty<'db>,
    ) -> Self {
        Self::new(
            db,
            TyKind::Function {
                parameters,
                return_type,
            },
        )
    }

    pub(crate) fn primitive(db: &'db dyn crate::Db, name: &str) -> Option<Self> {
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
                return_type,
            } => {
                let parameters: Vec<String> = parameters
                    .iter()
                    .map(|parameter| parameter.display(db))
                    .collect();
                format!("({}) -> {}", parameters.join(", "), return_type.display(db))
            }
            TyKind::UnificationVariable(UnificationVariable::General(_)) => "unknown".to_string(),
            TyKind::UnificationVariable(UnificationVariable::Integer(_)) => "Int".to_string(),
            TyKind::Error => "Error".to_string(),
        }
    }
}
