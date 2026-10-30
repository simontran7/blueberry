use crate::core::common::handle_collections::handle_disjoint_set::{HandleDisjointSet, MergeValue};
use crate::core::semantic_analysis::ast_lowering::nodes::{ExpressionHandle, LocalBindingHandle};
use crate::core::semantic_analysis::type_checking::types::{
    GeneralVariableHandle, InferenceVariable, IntegerVariableHandle, SignedIntTy, Ty, TyKind,
};

pub(crate) struct InferenceTable<'db> {
    db: &'db dyn crate::Db,
    general_inference_variables:
        HandleDisjointSet<GeneralVariableHandle, GeneralInferenceVariableSolution<'db>>,
    integer_inference_variables:
        HandleDisjointSet<IntegerVariableHandle, IntegerInferenceVariableSolution<'db>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct TypeMismatch<'db> {
    pub(crate) expected: Ty<'db>,
    pub(crate) found: Ty<'db>,
}

/// What a general inference variable was created for, so that an error about its type can point
/// at its cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) enum VariableOrigin {
    /// the type of an unannotated `let` binding
    LocalBinding(LocalBindingHandle),
    /// the type of an `if`, a block, or a `loop` that wasn't expected to be anything known
    Expression(ExpressionHandle),
}

/// The solution of a disjoint set of general inference variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeneralInferenceVariableSolution<'db> {
    ty: Option<Ty<'db>>,
    /// Whether a value of type [`TyKind::Bottom`] was coerced to the set. If still unsolved at the
    /// end, it falls back to [`TyKind::Bottom`].
    coerced_from_bottom: bool,
    origin: VariableOrigin,
}

/// The solution of a disjoint set of integer inference variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntegerInferenceVariableSolution<'db> {
    ty: Option<Ty<'db>>,
}

impl<'db> InferenceTable<'db> {
    pub(crate) fn new(db: &'db dyn crate::Db) -> Self {
        Self {
            db,
            general_inference_variables: HandleDisjointSet::new(),
            integer_inference_variables: HandleDisjointSet::new(),
        }
    }

    pub(crate) fn shallow_resolve(&mut self, ty: Ty<'db>) -> Ty<'db> {
        let mut current = ty;

        loop {
            match current.kind(self.db) {
                TyKind::InferenceVariable(InferenceVariable::General(variable_handle)) => {
                    if let Some(representative_value) =
                        self.general_inference_variables.value(*variable_handle).ty
                    {
                        current = representative_value;
                        continue;
                    }
                    return current;
                }
                TyKind::InferenceVariable(InferenceVariable::Integer(variable_handle)) => {
                    if let Some(representative_value) =
                        self.integer_inference_variables.value(*variable_handle).ty
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

    /// Zonks `ty`: replaces every solved variable inside it with its solution, at every depth,
    /// leaving unsolved ones as they are.
    pub(crate) fn zonk(&mut self, ty: Ty<'db>) -> Ty<'db> {
        let ty = self.shallow_resolve(ty);
        ty.map_children(self.db, |child| self.zonk(child))
    }

    /// Zonks `ty` like [`Self::zonk`], and poisons each unsolved general variable in it: gives up
    /// on it, by solving it to the error type in the table.
    ///
    /// Returns the resolved type, and the origin of the first unsolved variable, if any, so that the
    /// caller can report "type annotations needed".
    pub(crate) fn zonk_and_poison(&mut self, ty: Ty<'db>) -> (Ty<'db>, Option<VariableOrigin>) {
        fn poison<'db>(
            table: &mut InferenceTable<'db>,
            ty: Ty<'db>,
            unsolved_origin: &mut Option<VariableOrigin>,
        ) -> Ty<'db> {
            let ty = table.shallow_resolve(ty);
            match ty.kind(table.db) {
                TyKind::InferenceVariable(InferenceVariable::General(variable_handle)) => {
                    let error = Ty::error(table.db);
                    table
                        .general_inference_variables
                        .update_value(*variable_handle, |solution| {
                            unsolved_origin.get_or_insert(solution.origin);
                            solution.ty = Some(error);
                        });
                    error
                }
                TyKind::InferenceVariable(InferenceVariable::Integer(_)) => unreachable!(
                    "an integer variable is never unknown, since it defaults to `I32`, so it's \
                     never poisoned"
                ),
                _ => ty.map_children(table.db, |child| poison(table, child, unsolved_origin)),
            }
        }

        let mut unsolved_origin = None;
        let ty = poison(self, ty, &mut unsolved_origin);
        (ty, unsolved_origin)
    }

    pub(crate) fn make_general_inference_variable(&mut self, origin: VariableOrigin) -> Ty<'db> {
        let inference_variable_handle =
            self.general_inference_variables
                .make_set(GeneralInferenceVariableSolution {
                    ty: None,
                    coerced_from_bottom: false,
                    origin,
                });
        Ty::new(
            self.db,
            TyKind::InferenceVariable(InferenceVariable::General(inference_variable_handle)),
        )
    }

    pub(crate) fn make_integer_inference_variable(&mut self) -> Ty<'db> {
        let inference_variable_handle = self
            .integer_inference_variables
            .make_set(IntegerInferenceVariableSolution { ty: None });
        Ty::new(
            self.db,
            TyKind::InferenceVariable(InferenceVariable::Integer(inference_variable_handle)),
        )
    }

    pub(crate) fn unify(
        &mut self,
        expected: Ty<'db>,
        found: Ty<'db>,
    ) -> Result<(), TypeMismatch<'db>> {
        use InferenceVariable::{General, Integer};

        let expected = self.shallow_resolve(expected);
        let found = self.shallow_resolve(found);

        if expected == found {
            return Ok(());
        }

        match (expected.kind(self.db), found.kind(self.db)) {
            // Two unsolved variables: merge their groups so they share one solution. Both sides
            // were shallow resolved, so neither group has a solution yet and `union` can't fail.
            (
                TyKind::InferenceVariable(General(a_handle)),
                TyKind::InferenceVariable(General(b_handle)),
            ) => self.general_inference_variables.union(*a_handle, *b_handle),
            // One unsolved variable: solve it to the other side, unless that side contains the
            // variable, since no finite type could satisfy that.
            (TyKind::InferenceVariable(General(variable_handle)), _) => {
                if self.occurs(*variable_handle, found) {
                    return Err(TypeMismatch { expected, found });
                }
                self.general_inference_variables
                    .update_value(*variable_handle, |value| value.ty = Some(found));
                Ok(())
            }
            (_, TyKind::InferenceVariable(General(variable_handle))) => {
                if self.occurs(*variable_handle, expected) {
                    return Err(TypeMismatch { expected, found });
                }
                self.general_inference_variables
                    .update_value(*variable_handle, |value| value.ty = Some(expected));
                Ok(())
            }

            // An integer variable may only ever become an integer type (or the error type, see below)
            (
                TyKind::InferenceVariable(Integer(a_handle)),
                TyKind::InferenceVariable(Integer(b_handle)),
            ) => self.integer_inference_variables.union(*a_handle, *b_handle),
            (
                TyKind::InferenceVariable(Integer(variable_handle)),
                TyKind::Signed(_) | TyKind::Unsigned(_) | TyKind::Error,
            ) => {
                self.integer_inference_variables
                    .update_value(*variable_handle, |value| value.ty = Some(found));
                Ok(())
            }
            (
                TyKind::Signed(_) | TyKind::Unsigned(_) | TyKind::Error,
                TyKind::InferenceVariable(Integer(variable_handle)),
            ) => {
                self.integer_inference_variables
                    .update_value(*variable_handle, |value| value.ty = Some(expected));
                Ok(())
            }

            // Poisons: an error type unifies with anything, to avoid a pile of follow-up errors.
            // This comes after the variable cases, so that a variable unified with an error is
            // solved to it, instead of being left unsolved and later reported as unknown.
            (TyKind::Error, _) | (_, TyKind::Error) => Ok(()),

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

            (TyKind::Tuple(expected_elements), TyKind::Tuple(found_elements))
                if expected_elements.len() == found_elements.len() =>
            {
                for (expected_part, found_part) in expected_elements
                    .iter()
                    .copied()
                    .zip(found_elements.iter().copied())
                {
                    self.unify(expected_part, found_part)
                        .map_err(|_| TypeMismatch { expected, found })?;
                }
                Ok(())
            }

            _ => Err(TypeMismatch { expected, found }),
        }
    }

    /// Coerces `from` to `to`.
    pub(crate) fn coerce(&mut self, from: Ty<'db>, to: Ty<'db>) -> Result<(), TypeMismatch<'db>> {
        if self.shallow_resolve(from).is_bottom(self.db) {
            if let TyKind::InferenceVariable(InferenceVariable::General(variable_handle)) =
                self.shallow_resolve(to).kind(self.db)
            {
                self.general_inference_variables
                    .update_value(*variable_handle, |value| value.coerced_from_bottom = true);
            }
            return Ok(());
        }
        self.unify(to, from)
    }

    /// Solves every integer variable that's still unsolved to [`I32`](crate::core::semantic_analysis::type_checking::types::SignedIntTy::I32)
    pub(crate) fn fallback_integer_variables(&mut self) {
        let i32 = Ty::signed(self.db, SignedIntTy::I32);
        for value in self.integer_inference_variables.values_mut() {
            value.ty.get_or_insert(i32);
        }
    }

    /// Solves every set that's still unsolved and was coerced from [`Bottom`](crate::core::semantic_analysis::type_checking::types::TyKind::Bottom) to `Bottom`.
    pub(crate) fn fallback_general_variables(&mut self) {
        let bottom = Ty::bottom(self.db);
        for value in self.general_inference_variables.values_mut() {
            if value.coerced_from_bottom {
                value.ty.get_or_insert(bottom);
            }
        }
    }

    /// Returns whether the general inference variable `variable_handle` appears anywhere inside
    /// `ty`.
    fn occurs(&mut self, variable_handle: GeneralVariableHandle, ty: Ty<'db>) -> bool {
        let ty = self.shallow_resolve(ty);
        match ty.kind(self.db) {
            TyKind::InferenceVariable(InferenceVariable::General(other_handle)) => self
                .general_inference_variables
                .is_connected(variable_handle, *other_handle),
            _ => ty
                .children(self.db)
                .any(|child| self.occurs(variable_handle, child)),
        }
    }
}

impl<'db> MergeValue for GeneralInferenceVariableSolution<'db> {
    type Error = TypeMismatch<'db>;

    fn merge(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        Ok(Self {
            ty: Option::merge(&a.ty, &b.ty)?,
            coerced_from_bottom: a.coerced_from_bottom || b.coerced_from_bottom,
            // a `let` binding makes the best error message, since it can be given an annotation
            origin: if matches!(a.origin, VariableOrigin::Expression(_)) {
                b.origin
            } else {
                a.origin
            },
        })
    }
}

impl<'db> MergeValue for IntegerInferenceVariableSolution<'db> {
    type Error = TypeMismatch<'db>;

    fn merge(a: &Self, b: &Self) -> Result<Self, Self::Error> {
        Ok(Self {
            ty: Option::merge(&a.ty, &b.ty)?,
        })
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
