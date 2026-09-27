use crate::core::common::handle_collections::handle_disjoint_set::{HandleDisjointSet, UnifyValue};
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::semantic_analysis::type_checking::types::{
    GeneralVariableHandle, IntegerVariableHandle, Ty,
};

pub(crate) struct UnificationTable<'db> {
    general_unification_variables: HandleDisjointSet<GeneralVariableHandle, Option<Ty<'db>>>,
    integer_unification_variables: HandleDisjointSet<IntegerVariableHandle, Option<Ty<'db>>>,
}

pub(crate) struct TypeMismatch<'db> {
    pub(crate) expected: Ty<'db>,
    pub(crate) found: Ty<'db>,
}

impl<'db> UnificationTable<'db> {
    pub(crate) fn new() -> Self {
        Self {
            general_unification_variables: HandleDisjointSet::new(),
            integer_unification_variables: HandleDisjointSet::new(),
        }
    }
}

impl<'db> UnifyValue for Option<Ty<'db>> {
    type Error = TypeMismatch<'db>;

    fn unify(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        match (a, b) {
            (None, None) => Ok(None),
            (Some(t), None) | (None, Some(t)) => Ok(Some(*t)),
            (Some(t1), Some(t2)) if t1 == t2 => Ok(Some(*t1)),
            (Some(t1), Some(t2)) => Err(TypeMismatch {
                expected: *t1,
                found: *t2,
            }),
        }
    }
}
