use std::collections::HashMap;

use crate::common::string_interner::{StringInterner, Symbol};
use crate::semantic_analysis::unification_table::{IntVarId, TypeVarId};

#[derive(Clone, Hash, Eq, PartialEq)]
pub(crate) enum Ty {
    Unit,
    Bottom,
    Bool,
    Signed(SignedIntTy),
    Unsigned(UnsignedIntTy),
    Function {
        parameter_type_ids: Vec<TypeId>,
        return_type_id: TypeId,
    },
    Infer(InferTy),
    Error,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub(crate) enum SignedIntTy {
    I32,
    I64,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub(crate) enum UnsignedIntTy {
    U32,
    U64,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) enum InferTy {
    TyVar(TypeVarId),
    IntVar(IntVarId),
}

pub(crate) struct TypeInterner {
    types: Vec<Ty>,
    handles: HashMap<Ty, TypeId>,
    pub(crate) unit_id: TypeId,
    pub(crate) bottom_id: TypeId,
    pub(crate) bool_id: TypeId,
    pub(crate) u32_id: TypeId,
    pub(crate) u64_id: TypeId,
    pub(crate) i32_id: TypeId,
    pub(crate) i64_id: TypeId,
    pub(crate) error_id: TypeId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TypeId(pub(crate) u32);

impl TypeInterner {
    pub(crate) fn new() -> Self {
        let mut ti = Self {
            types: Vec::new(),
            handles: HashMap::new(),
            unit_id: TypeId(0),
            bottom_id: TypeId(0),
            bool_id: TypeId(0),
            u32_id: TypeId(0),
            u64_id: TypeId(0),
            i32_id: TypeId(0),
            i64_id: TypeId(0),
            error_id: TypeId(0),
        };
        ti.unit_id = ti.intern(Ty::Unit);
        ti.bottom_id = ti.intern(Ty::Bottom);
        ti.bool_id = ti.intern(Ty::Bool);
        ti.u32_id = ti.intern(Ty::Unsigned(UnsignedIntTy::U32));
        ti.u64_id = ti.intern(Ty::Unsigned(UnsignedIntTy::U64));
        ti.i32_id = ti.intern(Ty::Signed(SignedIntTy::I32));
        ti.i64_id = ti.intern(Ty::Signed(SignedIntTy::I64));
        ti.error_id = ti.intern(Ty::Error);
        ti
    }

    pub(crate) fn intern(&mut self, ty: Ty) -> TypeId {
        if let Some(&ty_id) = self.handles.get(&ty) {
            return ty_id;
        }
        let ty_id = TypeId(self.types.len() as u32);
        self.types.push(ty.clone());
        self.handles.insert(ty, ty_id);
        ty_id
    }

    pub(crate) fn resolve(&self, type_id: TypeId) -> Option<&Ty> {
        self.types.get(type_id.0 as usize)
    }

    pub(crate) fn is_zero_sized(&self, type_id: TypeId) -> bool {
        type_id == self.unit_id
    }

    pub(crate) fn is_zero_sized_or_bottom(&self, type_id: TypeId) -> bool {
        type_id == self.unit_id || type_id == self.bottom_id
    }

    pub(crate) fn is_unsigned(&self, type_id: TypeId) -> bool {
        matches!(self.resolve(type_id), Some(Ty::Unsigned(_)))
    }

    pub(crate) fn builtin_type_id(
        &self,
        symbol: Symbol,
        string_interner: &StringInterner,
    ) -> Option<TypeId> {
        if symbol == string_interner.unit_symbol {
            return Some(self.unit_id);
        }
        if symbol == string_interner.bottom_symbol {
            return Some(self.bottom_id);
        }
        if symbol == string_interner.bool_symbol {
            return Some(self.bool_id);
        }
        if symbol == string_interner.i32_symbol {
            return Some(self.i32_id);
        }
        if symbol == string_interner.i64_symbol {
            return Some(self.i64_id);
        }
        if symbol == string_interner.u32_symbol {
            return Some(self.u32_id);
        }
        if symbol == string_interner.u64_symbol {
            return Some(self.u64_id);
        }
        None
    }

    pub(crate) fn as_func(&self, type_id: TypeId) -> Option<(&[TypeId], TypeId)> {
        match self.resolve(type_id)? {
            Ty::Function {
                parameter_type_ids,
                return_type_id,
            } => Some((parameter_type_ids, *return_type_id)),
            _ => None,
        }
    }

    pub(crate) fn to_string(&self, type_id: TypeId) -> String {
        match self.resolve(type_id).unwrap() {
            Ty::Unit => "()".to_string(),
            Ty::Bottom => "Bottom".to_string(),
            Ty::Bool => "Bool".to_string(),
            Ty::Signed(SignedIntTy::I32) => "I32".to_string(),
            Ty::Signed(SignedIntTy::I64) => "I64".to_string(),
            Ty::Unsigned(UnsignedIntTy::U32) => "U32".to_string(),
            Ty::Unsigned(UnsignedIntTy::U64) => "U64".to_string(),
            Ty::Function {
                parameter_type_ids,
                return_type_id,
            } => {
                let parameters: Vec<String> = parameter_type_ids
                    .iter()
                    .map(|parameter_type_id| self.to_string(*parameter_type_id))
                    .collect();
                let return_value = self.to_string(*return_type_id);
                format!("({}) -> {}", parameters.join(", "), return_value)
            }
            Ty::Infer(InferTy::IntVar(_)) => "Int".to_string(),
            Ty::Infer(InferTy::TyVar(_)) => "unknown".to_string(),
            Ty::Error => "Error".to_string(),
        }
    }
}
