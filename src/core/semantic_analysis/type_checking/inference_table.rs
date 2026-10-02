use crate::core::common::handle_collections::handle_disjoint_set::{HandleDisjointSet, MergeValue};
use crate::core::semantic_analysis::type_checking::types::{
    GeneralVariableHandle, InferenceVariable, IntegerVariableHandle, Ty, TyKind,
};

pub(crate) struct InferenceTable<'db> {
    db: &'db dyn crate::Db,
    general_inference_variables: HandleDisjointSet<GeneralVariableHandle, Option<Ty<'db>>>,
    integer_inference_variables: HandleDisjointSet<IntegerVariableHandle, Option<Ty<'db>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
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

    pub(crate) fn make_general_inference_variable(&mut self) -> GeneralVariableHandle {
        self.general_inference_variables.make_set(None)
    }

    pub(crate) fn make_integer_inference_variable(&mut self) -> IntegerVariableHandle {
        self.integer_inference_variables.make_set(None)
    }

    pub(crate) fn shallow_resolve(&mut self, ty: Ty<'db>) -> Ty<'db> {
        let mut current = ty;

        loop {
            match current.kind(self.db) {
                TyKind::InferenceVariable(InferenceVariable::General(handle)) => {
                    if let Some(representative_value) =
                        self.general_inference_variables.get_value(*handle)
                    {
                        current = representative_value;
                        continue;
                    }
                    return current;
                }
                TyKind::InferenceVariable(InferenceVariable::Integer(handle)) => {
                    if let Some(representative_value) =
                        self.integer_inference_variables.get_value(*handle)
                    {
                        current = representative_value;
                        continue;
                    }
                    return current;
                }
                _ => return current,
            }
        }
    }

    pub(crate) fn unify(
        &mut self,
        expected: Ty<'db>,
        found: Ty<'db>,
    ) -> Result<(), TypeMismatch<'db>> {
        let expected = self.shallow_resolve(expected);
        let found = self.shallow_resolve(found);

        if expected == found {
            return Ok(());
        }

        use InferenceVariable::{General, Integer};
        match (expected.kind(self.db), found.kind(self.db)) {
            // Poisons: an error type unifies with anything, to avoid a pile of follow-up errors.
            (TyKind::Error, _) | (_, TyKind::Error) => Ok(()),

            // Two unsolved variables: merge their groups so they share one solution. Both sides
            // were shallow resolved, so neither group has a solution yet and `union` can't fail.
            (TyKind::InferenceVariable(General(a)), TyKind::InferenceVariable(General(b))) => {
                self.general_inference_variables.union(*a, *b)
            }
            // One unsolved variable: solve it to the other side, unless that side contains the
            // variable, since no finite type could satisfy that.
            (TyKind::InferenceVariable(General(variable)), _) => {
                if self.occurs(*variable, found) {
                    return Err(TypeMismatch { expected, found });
                }
                self.general_inference_variables
                    .set_value(*variable, Some(found));
                Ok(())
            }
            (_, TyKind::InferenceVariable(General(variable))) => {
                if self.occurs(*variable, expected) {
                    return Err(TypeMismatch { expected, found });
                }
                self.general_inference_variables
                    .set_value(*variable, Some(expected));
                Ok(())
            }

            // An integer variable may only ever become an integer type
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

            // Unifies two function types piece by piece. A mismatch inside is reported as the two whole function types.
            (
                TyKind::Function {
                    parameters: expected_parameters,
                    r#return: expected_return,
                },
                TyKind::Function {
                    parameters: found_parameters,
                    r#return: found_return,
                },
            ) if expected_parameters.len() == found_parameters.len() => {
                let pairs: Vec<_> = expected_parameters
                    .iter()
                    .copied()
                    .zip(found_parameters.iter().copied())
                    .chain([(*expected_return, *found_return)])
                    .collect();
                for (expected_part, found_part) in pairs {
                    self.unify(expected_part, found_part)
                        .map_err(|_| TypeMismatch { expected, found })?;
                }
                Ok(())
            }

            _ => Err(TypeMismatch { expected, found }),
        }
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
                r#return,
            } => {
                parameters
                    .clone()
                    .into_iter()
                    .any(|parameter| self.occurs(variable, parameter))
                    || self.occurs(variable, *r#return)
            }
            _ => false,
        }
    }
}

impl<'db> MergeValue for Option<Ty<'db>> {
    type Error = TypeMismatch<'db>;

    fn merge(a: &Self, b: &Self) -> Result<Self, Self::Error> {
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
