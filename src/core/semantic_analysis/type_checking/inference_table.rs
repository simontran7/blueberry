use crate::core::common::handle_collections::handle_disjoint_set::{HandleDisjointSet, UnifyValue};
use crate::core::semantic_analysis::type_checking::types::{
    GeneralVariableHandle, InferenceVariable, IntegerVariableHandle, Ty, TyKind,
};

pub(crate) struct InferenceTable<'db> {
    db: &'db dyn crate::Db,
    general_inference_variables: HandleDisjointSet<GeneralVariableHandle, Option<Ty<'db>>>,
    integer_inference_variables: HandleDisjointSet<IntegerVariableHandle, Option<Ty<'db>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TypeMismatch<'db> {
    pub(crate) expected: Ty<'db>,
    pub(crate) found: Ty<'db>,
}

impl<'db> InferenceTable<'db> {
    pub(crate) fn new(db: &'db dyn crate::Db) -> Self {
        Self {
            db,
            general_inference_variables: HandleDisjointSet::new(),
            integer_inference_variables: HandleDisjointSet::new(),
        }
    }

    /// Creates a fresh general inference variable, whose type is not yet known.
    pub(crate) fn new_general_inference_variable(&mut self) -> GeneralVariableHandle {
        self.general_inference_variables.make_set(None)
    }

    /// Creates a fresh integer inference variable, whose exact integer type is not yet known.
    pub(crate) fn new_integer_inference_variable(&mut self) -> IntegerVariableHandle {
        self.integer_inference_variables.make_set(None)
    }

    /// Makes `expected` and `found` the same type, recording any inference variables solved
    /// along the way. On a mismatch, returns the two (shallow resolved) types that disagree.
    pub(crate) fn unify(
        &mut self,
        expected: Ty<'db>,
        found: Ty<'db>,
    ) -> Result<(), TypeMismatch<'db>> {
        let expected = self.shallow_resolve(expected);
        let found = self.shallow_resolve(found);

        // Types are interned, so identical types are the same `Ty`.
        if expected == found {
            return Ok(());
        }

        use InferenceVariable::{General, Integer};
        match (expected.kind(self.db), found.kind(self.db)) {
            // An error type unifies with anything, to avoid a pile of follow-up errors.
            (TyKind::Error, _) | (_, TyKind::Error) => Ok(()),

            (TyKind::InferenceVariable(General(a)), TyKind::InferenceVariable(General(b))) => {
                self.general_inference_variables.union(*a, *b)
            }
            (TyKind::InferenceVariable(General(variable)), _) => {
                self.bind_general(*variable, found, expected, found)
            }
            (_, TyKind::InferenceVariable(General(variable))) => {
                self.bind_general(*variable, expected, expected, found)
            }

            (TyKind::InferenceVariable(Integer(a)), TyKind::InferenceVariable(Integer(b))) => {
                self.integer_inference_variables.union(*a, *b)
            }
            (
                TyKind::InferenceVariable(Integer(variable)),
                TyKind::Signed(_) | TyKind::Unsigned(_),
            ) => {
                self.integer_inference_variables
                    .set_value(*variable, Some(found));
                Ok(())
            }
            (
                TyKind::Signed(_) | TyKind::Unsigned(_),
                TyKind::InferenceVariable(Integer(variable)),
            ) => {
                self.integer_inference_variables
                    .set_value(*variable, Some(expected));
                Ok(())
            }

            (
                TyKind::Function {
                    parameters: expected_parameters,
                    return_type: expected_return,
                },
                TyKind::Function {
                    parameters: found_parameters,
                    return_type: found_return,
                },
            ) if expected_parameters.len() == found_parameters.len() => {
                for (&e, &f) in expected_parameters.iter().zip(found_parameters) {
                    self.unify(e, f)?;
                }
                self.unify(*expected_return, *found_return)
            }

            _ => Err(TypeMismatch { expected, found }),
        }
    }

    /// If `ty` is an inference variable that has been solved, returns what it was solved to
    /// (repeatedly, since a variable can be solved to another variable). Only looks at the
    /// outermost type: variables nested inside, e.g. in a function's parameters, are left as is.
    pub(crate) fn shallow_resolve(&mut self, ty: Ty<'db>) -> Ty<'db> {
        let mut ty = ty;
        loop {
            let solved = match ty.kind(self.db) {
                TyKind::InferenceVariable(InferenceVariable::General(variable)) => {
                    self.general_inference_variables.get_value(*variable)
                }
                TyKind::InferenceVariable(InferenceVariable::Integer(variable)) => {
                    self.integer_inference_variables.get_value(*variable)
                }
                _ => None,
            };
            match solved {
                Some(solved) => ty = solved,
                None => return ty,
            }
        }
    }

    /// Solves the general inference variable `variable` to `ty`, unless `ty` contains
    /// `variable` (the occurs check), since no finite type could satisfy that.
    fn bind_general(
        &mut self,
        variable: GeneralVariableHandle,
        ty: Ty<'db>,
        expected: Ty<'db>,
        found: Ty<'db>,
    ) -> Result<(), TypeMismatch<'db>> {
        if self.occurs(variable, ty) {
            return Err(TypeMismatch { expected, found });
        }
        self.general_inference_variables
            .set_value(variable, Some(ty));
        Ok(())
    }

    /// Returns whether the general inference variable `variable` appears anywhere inside `ty`.
    fn occurs(&mut self, variable: GeneralVariableHandle, ty: Ty<'db>) -> bool {
        let ty = self.shallow_resolve(ty);
        match ty.kind(self.db) {
            TyKind::InferenceVariable(InferenceVariable::General(other)) => self
                .general_inference_variables
                .is_connected(variable, *other),
            TyKind::Function {
                parameters,
                return_type,
            } => {
                parameters
                    .iter()
                    .any(|&parameter| self.occurs(variable, parameter))
                    || self.occurs(variable, *return_type)
            }
            _ => false,
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

#[cfg(test)]
mod tests {
    use super::{InferenceTable, TypeMismatch};
    use crate::core::db::BlueberryDatabase;
    use crate::core::semantic_analysis::type_checking::types::{
        InferenceVariable, SignedIntTy, Ty, TyKind, UnsignedIntTy,
    };

    fn general<'db>(db: &'db BlueberryDatabase, table: &mut InferenceTable<'db>) -> Ty<'db> {
        let variable = table.new_general_inference_variable();
        Ty::new(
            db,
            TyKind::InferenceVariable(InferenceVariable::General(variable)),
        )
    }

    fn integer<'db>(db: &'db BlueberryDatabase, table: &mut InferenceTable<'db>) -> Ty<'db> {
        let variable = table.new_integer_inference_variable();
        Ty::new(
            db,
            TyKind::InferenceVariable(InferenceVariable::Integer(variable)),
        )
    }

    #[test]
    fn test_unify_general_with_concrete() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let a = general(&db, &mut table);

        table.unify(a, Ty::bool(&db)).unwrap();

        assert_eq!(table.shallow_resolve(a), Ty::bool(&db));
    }

    #[test]
    fn test_unify_general_variables_then_concrete() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let a = general(&db, &mut table);
        let b = general(&db, &mut table);

        table.unify(a, b).unwrap();
        table.unify(b, Ty::bool(&db)).unwrap();

        assert_eq!(table.shallow_resolve(a), Ty::bool(&db));
    }

    #[test]
    fn test_unify_integer_with_integer_type() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let i = integer(&db, &mut table);
        let u64 = Ty::unsigned(&db, UnsignedIntTy::U64);

        table.unify(i, u64).unwrap();

        assert_eq!(table.shallow_resolve(i), u64);
    }

    #[test]
    fn test_unify_integer_with_bool_is_mismatch() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let i = integer(&db, &mut table);

        assert_eq!(
            table.unify(i, Ty::bool(&db)),
            Err(TypeMismatch {
                expected: i,
                found: Ty::bool(&db)
            })
        );
    }

    #[test]
    fn test_unify_general_with_integer_variable() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let a = general(&db, &mut table);
        let i = integer(&db, &mut table);
        let i32 = Ty::signed(&db, SignedIntTy::I32);

        table.unify(a, i).unwrap();
        table.unify(i, i32).unwrap();

        assert_eq!(table.shallow_resolve(a), i32);
    }

    #[test]
    fn test_unify_functions_recurses() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let a = general(&db, &mut table);
        let i32 = Ty::signed(&db, SignedIntTy::I32);

        let expected = Ty::function(&db, vec![a], i32);
        let found = Ty::function(&db, vec![Ty::bool(&db)], i32);
        table.unify(expected, found).unwrap();

        assert_eq!(table.shallow_resolve(a), Ty::bool(&db));
    }

    #[test]
    fn test_unify_functions_with_different_arity_is_mismatch() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let unit = Ty::unit(&db);

        let expected = Ty::function(&db, vec![Ty::bool(&db)], unit);
        let found = Ty::function(&db, vec![], unit);

        assert!(table.unify(expected, found).is_err());
    }

    #[test]
    fn test_occurs_check() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);
        let a = general(&db, &mut table);
        let function = Ty::function(&db, vec![a], Ty::unit(&db));

        assert!(table.unify(a, function).is_err());
        assert_eq!(table.shallow_resolve(a), a);
    }

    #[test]
    fn test_unify_error_with_anything() {
        let db = BlueberryDatabase::default();
        let mut table = InferenceTable::new(&db);

        assert!(table.unify(Ty::error(&db), Ty::bool(&db)).is_ok());
    }
}
