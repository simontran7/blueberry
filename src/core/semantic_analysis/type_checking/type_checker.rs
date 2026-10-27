use std::collections::{HashMap, HashSet};
use std::ops::Index;

use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::nodes::{
    BinaryOperator, Body, Expression, ExpressionHandle, LocalBindingHandle, LoopSource, Path,
    Statement, StatementHandle, TypeExpressionHandle, UnaryOperator,
};
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::name_resolution::resolver::Resolution;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::semantic_diagnostic::{
    TypeMismatchCause, UnresolvedSemanticDiagnostic,
};
use crate::core::semantic_analysis::signature_type_of;
use crate::core::semantic_analysis::type_checking::inference_table::{
    InferenceTable, TypeMismatch,
};
use crate::core::semantic_analysis::type_checking::type_expression_lowering::lower_type_expression;
use crate::core::semantic_analysis::type_checking::types::{
    InferenceVariable, SignedIntTy, Ty, TyKind, UnsignedIntTy,
};

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(in crate::core) struct TypeCheckResult<'db> {
    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
    path_resolutions: SideHandleMap<ExpressionHandle, Resolution<'db>>,
    diagnostics: Vec<UnresolvedSemanticDiagnostic<'db>>,
}

struct TypeChecker<'a, 'db> {
    db: &'db dyn crate::Db,
    body: &'a Body<'db>,
    scopes: &'a ScopeTree<'db>,
    inference: InferenceTable<'db>,
    return_funnel: Option<CoercionFunnel<'db>>,
    diverges: Diverges,
    loop_frames: Vec<LoopFrame<'db>>,
    pending_negations: Vec<(ExpressionHandle, Ty<'db>)>,
    result: TypeCheckResult<'db>,
}

#[derive(Clone, Copy)]
enum Expectation<'db> {
    None,
    Type(Ty<'db>),
}

struct CoercionFunnel<'db> {
    expected: Ty<'db>,
    merged: Option<Ty<'db>>,
}

struct LoopFrame<'db> {
    may_break: bool,
    funnel: Option<CoercionFunnel<'db>>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Diverges {
    #[default]
    Maybe,
    Always,
}

struct BinaryOperatorRule {
    operands: OperandTypes,
    result: ResultType,
    /// whether the rhs only runs depending on the lhs's value
    short_circuits: bool,
}

enum OperandTypes {
    /// both operands are `Bool`
    Bool,
    /// both operands have the same type, whichever it is
    Same,
    /// both operands have the same integer type
    SameInteger,
}

enum ResultType {
    Bool,
    /// the operands' type
    Operand,
}

pub(in crate::core) fn check_function<'db>(
    db: &'db dyn crate::Db,
    body: &Body<'db>,
    scopes: &ScopeTree<'db>,
    signature_type: Ty<'db>,
) -> TypeCheckResult<'db> {
    let TyKind::Function {
        parameters,
        r#return,
    } = signature_type.kind(db)
    else {
        unreachable!("a function's signature type is always a function type");
    };

    let return_funnel = CoercionFunnel::new(*r#return);
    let mut checker = TypeChecker::new(db, body, scopes, Some(return_funnel));

    let binding_handles = &body.parameter_handles;
    for (&binding_handle, &ty) in binding_handles.iter().zip(parameters) {
        checker.result.local_bindings.add(binding_handle, ty);
    }

    checker.check_returned(body.root_handle);

    checker.finish()
}

pub(in crate::core) fn check_constant<'db>(
    db: &'db dyn crate::Db,
    body: &Body<'db>,
    scopes: &ScopeTree<'db>,
    declared_type: Ty<'db>,
) -> TypeCheckResult<'db> {
    let mut checker = TypeChecker::new(db, body, scopes, None);

    let expectation = Expectation::of_type(db, declared_type);
    let ty = checker.check_expression(body.root_handle, expectation);
    checker.coerce_or_report(ty, expectation, body.root_handle);

    checker.finish()
}

impl<'db> TypeCheckResult<'db> {
    pub(in crate::core) fn path_resolution(
        &self,
        expression_handle: ExpressionHandle,
    ) -> Option<Resolution<'db>> {
        self.path_resolutions.get(expression_handle).copied()
    }

    pub(in crate::core) fn expressions(
        &self,
    ) -> impl Iterator<Item = (ExpressionHandle, Ty<'db>)> + '_ {
        self.expressions
            .iter()
            .map(|(expression_handle, ty)| (expression_handle, *ty))
    }

    pub(in crate::core) fn local_bindings(
        &self,
    ) -> impl Iterator<Item = (LocalBindingHandle, Ty<'db>)> + '_ {
        self.local_bindings
            .iter()
            .map(|(binding_handle, ty)| (binding_handle, *ty))
    }

    pub(in crate::core) fn diagnostics(&self) -> &[UnresolvedSemanticDiagnostic<'db>] {
        &self.diagnostics
    }

    pub(in crate::core) fn has_errors(&self, db: &'db dyn crate::Db) -> bool {
        !self.diagnostics.is_empty()
            || self.expressions.values().any(|ty| ty.is_poisoned(db))
            || self.local_bindings.values().any(|ty| ty.is_poisoned(db))
    }

    fn new() -> Self {
        Self {
            expressions: SideHandleMap::new(),
            local_bindings: SideHandleMap::new(),
            path_resolutions: SideHandleMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn add_diagnostic(&mut self, diagnostic: UnresolvedSemanticDiagnostic<'db>) {
        self.diagnostics.push(diagnostic);
    }
}

impl<'db> Index<ExpressionHandle> for TypeCheckResult<'db> {
    type Output = Ty<'db>;

    fn index(&self, expression_handle: ExpressionHandle) -> &Ty<'db> {
        &self.expressions[expression_handle]
    }
}

impl<'db> Index<LocalBindingHandle> for TypeCheckResult<'db> {
    type Output = Ty<'db>;

    fn index(&self, binding_handle: LocalBindingHandle) -> &Ty<'db> {
        &self.local_bindings[binding_handle]
    }
}

impl<'a, 'db> TypeChecker<'a, 'db> {
    fn new(
        db: &'db dyn crate::Db,
        body: &'a Body<'db>,
        scopes: &'a ScopeTree<'db>,
        return_funnel: Option<CoercionFunnel<'db>>,
    ) -> Self {
        Self {
            db,
            body,
            scopes,
            inference: InferenceTable::new(db),
            return_funnel,
            diverges: Diverges::Maybe,
            loop_frames: Vec::new(),
            pending_negations: Vec::new(),
            result: TypeCheckResult::new(),
        }
    }

    fn check_expression(
        &mut self,
        expression_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let ty = match &self.body.store[expression_handle] {
            Expression::If {
                condition_handle,
                then_branch_handle,
                else_branch_handle,
            } => self.check_if(
                expression_handle,
                *condition_handle,
                *then_branch_handle,
                *else_branch_handle,
                expectation,
            ),
            Expression::Block {
                statement_run,
                tail_handle,
                ..
            } => self.check_block(
                expression_handle,
                &self.body.store[*statement_run],
                *tail_handle,
                expectation,
            ),
            Expression::Tuple { element_run } => {
                self.check_tuple(&self.body.store[*element_run], expectation)
            }
            Expression::Field {
                receiver_handle,
                name,
            } => self.check_field(expression_handle, *receiver_handle, *name),
            Expression::Loop {
                source,
                body_handle,
            } => self.check_loop(*source, *body_handle, expectation),
            Expression::Call {
                callee_handle,
                argument_run,
            } => self.check_call(
                expression_handle,
                *callee_handle,
                &self.body.store[*argument_run],
            ),
            Expression::Continue => self.check_continue(expression_handle),
            Expression::Break { value_handle } => {
                self.check_break(expression_handle, *value_handle)
            }
            Expression::Return { value_handle } => {
                self.check_return(expression_handle, *value_handle)
            }
            Expression::UnaryOperation {
                operator,
                operand_handle,
            } => self.check_unary_operation(
                expression_handle,
                *operator,
                *operand_handle,
                expectation,
            ),
            Expression::BinaryOperation {
                lhs_handle,
                operator,
                rhs_handle,
            } => {
                self.check_binary_operation(expression_handle, *operator, *lhs_handle, *rhs_handle)
            }
            Expression::Assignment {
                target_handle,
                value_handle,
            } => self.check_assignment(expression_handle, *target_handle, *value_handle),
            Expression::Boolean(_) => Ty::bool(self.db),
            Expression::Integer(_) => self.check_integer_literal(expectation),
            Expression::Path(path) => self.check_path(expression_handle, *path),
            // A syntax error, already reported by the parser.
            Expression::Error => Ty::error(self.db),
        };

        self.result.expressions.add(expression_handle, ty);

        if self.is_bottom(ty) {
            self.diverges = Diverges::Always;
        }

        ty
    }

    fn is_bottom(&mut self, ty: Ty<'db>) -> bool {
        matches!(
            self.inference.shallow_resolve(ty).kind(self.db),
            TyKind::Bottom
        )
    }

    fn check_integer_literal(&mut self, expectation: Expectation<'db>) -> Ty<'db> {
        if let Some(expected) = expectation.expected_type(&mut self.inference)
            && let TyKind::Signed(_) | TyKind::Unsigned(_) = expected.kind(self.db)
        {
            return expected;
        }
        self.inference.make_integer_inference_variable()
    }

    fn check_path(&mut self, expression_handle: ExpressionHandle, path: Path<'db>) -> Ty<'db> {
        // resolve path
        let resolution = self
            .scopes
            .resolver_for(expression_handle)
            .resolve_path(self.db, path)
            .inspect(|&resolution| {
                self.result
                    .path_resolutions
                    .add(expression_handle, resolution);
            });

        // return the path's type (if any)
        match resolution {
            Ok(Resolution::Local(binding_handle)) => self.result.local_bindings[binding_handle],
            Ok(Resolution::Function(key)) => *signature_type_of(self.db, Definition::Function(key)),
            Ok(Resolution::Constant(key)) => *signature_type_of(self.db, Definition::Constant(key)),
            Ok(Resolution::Error) => Ty::error(self.db),
            Ok(Resolution::Module(_)) => {
                self.result.add_diagnostic(
                    UnresolvedSemanticDiagnostic::ExpectedValueFoundModule { expression_handle },
                );
                Ty::error(self.db)
            }
            Err(error) => {
                self.result
                    .add_diagnostic(UnresolvedSemanticDiagnostic::UnresolvedPath {
                        expression_handle,
                        error,
                    });
                Ty::error(self.db)
            }
        }
    }

    fn check_tuple(
        &mut self,
        element_handles: &[ExpressionHandle],
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let expected_elements = match expectation
            .expected_type(&mut self.inference)
            .map(|ty| ty.kind(self.db))
        {
            Some(TyKind::Tuple(elements)) if elements.len() == element_handles.len() => {
                Some(elements.clone())
            }
            _ => None,
        };

        let mut elements = Vec::with_capacity(element_handles.len());
        for (index, &element_handle) in element_handles.iter().enumerate() {
            let expectation = match &expected_elements {
                Some(expected_elements) => Expectation::of_type(self.db, expected_elements[index]),
                None => Expectation::None,
            };
            let element_ty = self.check_expression(element_handle, expectation);
            self.coerce_or_report(element_ty, expectation, element_handle);
            elements.push(match expectation {
                Expectation::Type(expected) => expected,
                Expectation::None => element_ty,
            });
        }
        Ty::tuple(self.db, elements.into_boxed_slice())
    }

    fn check_field(
        &mut self,
        expression_handle: ExpressionHandle,
        receiver_handle: ExpressionHandle,
        name: Option<Symbol<'db>>,
    ) -> Ty<'db> {
        let receiver_ty = self.check_expression(receiver_handle, Expectation::None);
        let Some(name) = name else {
            return Ty::error(self.db);
        };

        let receiver_ty = self.inference.shallow_resolve(receiver_ty);
        let is_primitive = match receiver_ty.kind(self.db) {
            TyKind::Error => return Ty::error(self.db),
            TyKind::Bottom => return Ty::bottom(self.db),
            TyKind::Tuple(elements) => {
                if let Some(&element) = name
                    .as_tuple_index(self.db)
                    .and_then(|index| elements.get(index as usize))
                {
                    return element;
                }
                false
            }
            TyKind::InferenceVariable(InferenceVariable::General(_)) => {
                self.result
                    .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMustBeKnown {
                        expression_handle: receiver_handle,
                    });
                let _ = self.inference.unify(receiver_ty, Ty::error(self.db));
                return Ty::error(self.db);
            }
            TyKind::Bool
            | TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::InferenceVariable(InferenceVariable::Integer(_)) => true,
            TyKind::Function { .. } => false,
        };

        let receiver_ty = self.inference.deep_resolve(receiver_ty);
        if !receiver_ty.is_poisoned(self.db) {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::UnknownField {
                    expression_handle,
                    ty: receiver_ty,
                    is_primitive,
                });
        }
        Ty::error(self.db)
    }

    fn check_if(
        &mut self,
        expression_handle: ExpressionHandle,
        condition_handle: ExpressionHandle,
        then_branch_handle: ExpressionHandle,
        else_branch_handle: Option<ExpressionHandle>,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        // Only pass a known expected type down to the branches. If it were an unsolved inference
        // variable, the `then` branch would set that variable to its own type, and the `else`
        // branch would then be checked against it. As a result, a mismatch would be reported as a
        // plain type mismatch on the `else` branch, rather than as "`if` and `else` have
        // incompatible types".
        let expectation = match expectation.expected_type(&mut self.inference) {
            Some(ty) => match ty.kind(self.db) {
                TyKind::InferenceVariable(_) => Expectation::None,
                _ => Expectation::of_type(self.db, ty),
            },
            None => Expectation::None,
        };

        // create a coercion funnel for the branch(es)
        let mut funnel = CoercionFunnel::from_expectation(expectation, &mut self.inference);

        // typecheck condition
        let condition_expectation = Expectation::of_type(self.db, Ty::bool(self.db));
        let condition_ty = self.check_expression(condition_handle, condition_expectation);
        self.coerce_or_report(condition_ty, condition_expectation, condition_handle);
        let condition_diverges = std::mem::take(&mut self.diverges); // saves the condition's divergence, and resets it for the branches

        // typecheck the then branch, and pour it into the funnel
        let then_ty = self.check_expression(then_branch_handle, expectation);
        let then_diverges = std::mem::take(&mut self.diverges); // saves the then branch's divergence, and resets it for the else branch
        self.pour_or_report(
            &mut funnel,
            then_branch_handle,
            then_ty,
            TypeMismatchCause::General,
        );

        match else_branch_handle {
            Some(else_branch_handle) => {
                // typecheck the else branch, and pour it into the funnel
                let else_ty = self.check_expression(else_branch_handle, expectation);
                let else_diverges = self.diverges; // saves the else branch's divergence
                self.pour_or_report(
                    &mut funnel,
                    else_branch_handle,
                    else_ty,
                    TypeMismatchCause::IfElse { then_branch_handle },
                );

                // with an `else`, one of the branches always runs, so the `if` diverges if the condition
                // diverges, or if both branches do
                self.diverges = if condition_diverges == Diverges::Always
                    || (then_diverges == Diverges::Always && else_diverges == Diverges::Always)
                {
                    Diverges::Always
                } else {
                    Diverges::Maybe
                };
            }
            None => {
                // without an `else`, the `if` produces `()` when the condition is false, so `()` is
                // poured as the missing branch
                self.pour_or_report(
                    &mut funnel,
                    expression_handle,
                    Ty::unit(self.db),
                    TypeMismatchCause::IfWithoutElse,
                );

                // the then branch alone can't make the `if` diverge, since a false condition skips the
                // then branch, so only the condition's divergence matters
                self.diverges = condition_diverges;
            }
        }

        funnel.drain(self.db)
    }

    fn check_block(
        &mut self,
        expression_handle: ExpressionHandle,
        statement_handles: &[StatementHandle],
        tail_handle: Option<ExpressionHandle>,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        // create a coercion funnel for the block's value (its tail, or `()` if it has none)
        let mut funnel = CoercionFunnel::from_expectation(expectation, &mut self.inference);

        // check each statement in order.
        // A diverging one (e.g., `return 1;`) makes the rest of the block unreachable
        for statement_handle in statement_handles {
            match &self.body.store[*statement_handle] {
                Statement::Let {
                    name_handle,
                    annotation_handle,
                    initializer_handle,
                } => self.check_let(*name_handle, *annotation_handle, *initializer_handle),
                Statement::Expression {
                    expression_handle,
                    has_semicolon,
                } => {
                    if *has_semicolon {
                        self.check_expression(*expression_handle, Expectation::None);
                    } else {
                        // e.g. an `if` without a semicolon, followed by more statements
                        let expectation = Expectation::of_type(self.db, Ty::unit(self.db));
                        let ty = self.check_expression(*expression_handle, expectation);
                        self.coerce_or_report(ty, expectation, *expression_handle);
                    }
                }
                // a nested definition isn't part of this body, but rather, it's listed in its block's
                // `DefinitionList`, and checked as a body of its own,
                // like a top-level definition
                Statement::Definition => {}
            }
        }

        // The tail of a function's body is the function's result.
        let is_function_body =
            self.return_funnel.is_some() && expression_handle == self.body.root_handle;
        match tail_handle {
            Some(tail_handle) => {
                // typecheck tail
                let tail_ty = self.check_expression(tail_handle, expectation);
                let cause = if is_function_body {
                    TypeMismatchCause::ReturnValue
                } else {
                    TypeMismatchCause::General
                };
                self.pour_or_report(&mut funnel, tail_handle, tail_ty, cause);
            }
            None => {
                // a tail-less block produces `()`, but it might differ with
                // the annotated return type
                if self.diverges == Diverges::Maybe {
                    let cause = if is_function_body {
                        TypeMismatchCause::ImplicitUnitReturn
                    } else {
                        TypeMismatchCause::General
                    };
                    self.pour_or_report(&mut funnel, expression_handle, Ty::unit(self.db), cause);
                }
            }
        }

        funnel.drain(self.db)
    }

    fn check_let(
        &mut self,
        binding_handle: LocalBindingHandle,
        annotation_handle: Option<TypeExpressionHandle>,
        initializer_handle: ExpressionHandle,
    ) {
        let annotated_ty = match annotation_handle {
            Some(annotation_handle) => lower_type_expression(
                self.db,
                &self.body.store,
                annotation_handle,
                &mut |type_expression_handle| {
                    self.result
                        .add_diagnostic(UnresolvedSemanticDiagnostic::UnknownType {
                            type_expression_handle,
                        })
                },
            ),
            None => self.inference.new_general_inference_variable(),
        };
        let expectation = Expectation::of_type(self.db, annotated_ty);
        let initializer_ty = self.check_expression(initializer_handle, expectation);
        self.coerce_or_report(initializer_ty, expectation, initializer_handle);
        self.result.local_bindings.add(binding_handle, annotated_ty);
    }

    fn check_loop(
        &mut self,
        source: LoopSource,
        loop_body_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let funnel = match source {
            LoopSource::Loop => Some(CoercionFunnel::from_expectation(
                expectation,
                &mut self.inference,
            )),
            LoopSource::While => None,
        };
        self.loop_frames.push(LoopFrame {
            may_break: false,
            funnel,
        });
        let body_expectation = Expectation::of_type(self.db, Ty::unit(self.db));
        let body_ty = self.check_expression(loop_body_handle, body_expectation);
        self.coerce_or_report(body_ty, body_expectation, loop_body_handle);
        let frame = self.loop_frames.pop().unwrap();

        self.diverges = if frame.may_break {
            Diverges::Maybe
        } else {
            Diverges::Always
        };
        match frame.funnel {
            Some(funnel) => funnel.drain(self.db),
            None => Ty::unit(self.db),
        }
    }

    fn check_break(
        &mut self,
        expression_handle: ExpressionHandle,
        value_handle: Option<ExpressionHandle>,
    ) -> Ty<'db> {
        let Some(frame) = self.loop_frames.last() else {
            if let Some(value_handle) = value_handle {
                self.check_expression(value_handle, Expectation::None);
            }
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::BreakOutsideOfLoop {
                    expression_handle,
                });
            return Ty::bottom(self.db);
        };
        let loop_ty = frame.funnel.as_ref().map(|funnel| funnel.expected);

        let value_ty = match value_handle {
            Some(value_handle) => {
                let expectation = match loop_ty {
                    Some(loop_ty) => Expectation::of_type(self.db, loop_ty),
                    None => {
                        self.result.add_diagnostic(
                            UnresolvedSemanticDiagnostic::BreakWithValueFromWhile {
                                expression_handle,
                            },
                        );
                        Expectation::None
                    }
                };
                Some((
                    value_handle,
                    self.check_expression(value_handle, expectation),
                ))
            }
            None => None,
        };

        let diverges = self.diverges;
        let frame = self.loop_frames.last_mut().unwrap();
        // A `break` whose value diverges itself (e.g. `break return 5`) never reaches the loop's end.
        frame.may_break |= diverges == Diverges::Maybe;
        // the funnel is taken out of the loop frame while pouring, like in `check_returned`
        if let Some(mut funnel) = frame.funnel.take() {
            match value_ty {
                Some((value_handle, value_ty)) => self.pour_or_report(
                    &mut funnel,
                    value_handle,
                    value_ty,
                    TypeMismatchCause::General,
                ),
                None => self.pour_or_report(
                    &mut funnel,
                    expression_handle,
                    Ty::unit(self.db),
                    TypeMismatchCause::General,
                ),
            }
            self.loop_frames.last_mut().unwrap().funnel = Some(funnel);
        }

        Ty::bottom(self.db)
    }

    fn check_continue(&mut self, expression_handle: ExpressionHandle) -> Ty<'db> {
        if self.loop_frames.is_empty() {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::ContinueOutsideOfLoop {
                    expression_handle,
                });
        }
        Ty::bottom(self.db)
    }

    fn check_return(
        &mut self,
        expression_handle: ExpressionHandle,
        value_handle: Option<ExpressionHandle>,
    ) -> Ty<'db> {
        // a constant's body has no return funnel, so `return` can't appear in it
        let Some(return_funnel) = self.return_funnel.as_mut() else {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::ReturnOutsideFunction {
                    expression_handle,
                });
            if let Some(value_handle) = value_handle {
                self.check_expression(value_handle, Expectation::None);
            }
            return Ty::bottom(self.db);
        };
        match value_handle {
            // `return e`: `e` is checked with the return type, and poured into the return funnel
            Some(value_handle) => self.check_returned(value_handle),
            // bare `return`: returns `()`, so `()` is poured into the return funnel
            None => {
                if return_funnel
                    .pour(self.db, &mut self.inference, Ty::unit(self.db))
                    .is_err()
                {
                    self.result
                        .add_diagnostic(UnresolvedSemanticDiagnostic::ReturnWithoutValue {
                            expression_handle,
                        });
                }
            }
        }

        // return expression's type is always bottom
        Ty::bottom(self.db)
    }

    fn check_returned(&mut self, value_handle: ExpressionHandle) {
        // typecheck value
        let value_ty = self.check_expression(
            value_handle,
            Expectation::of_type(self.db, self.return_funnel.as_ref().unwrap().expected),
        );
        // the funnel is taken out of `self` while pouring, since `pour_or_report` borrows both
        // `self` and the funnel mutably; it's only taken after the value is checked, so that a
        // `return` nested in the value still finds it
        let mut return_funnel = self.return_funnel.take().unwrap();
        self.pour_or_report(
            &mut return_funnel,
            value_handle,
            value_ty,
            TypeMismatchCause::ReturnValue,
        );
        self.return_funnel = Some(return_funnel);
    }

    fn check_call(
        &mut self,
        call_handle: ExpressionHandle,
        callee_handle: ExpressionHandle,
        argument_handles: &[ExpressionHandle],
    ) -> Ty<'db> {
        let callee_ty = self.check_expression(callee_handle, Expectation::None);
        let callee_ty = self.inference.shallow_resolve(callee_ty);

        let (parameters, return_ty) =
            match callee_ty.kind(self.db) {
                TyKind::Function {
                    parameters,
                    r#return,
                } => (parameters.clone(), *r#return),
                _ => {
                    if !self.inference.deep_resolve(callee_ty).is_poisoned(self.db) {
                        let found = self.inference.deep_resolve(callee_ty);
                        self.result.add_diagnostic(
                            UnresolvedSemanticDiagnostic::ExpectedFunction { call_handle, found },
                        );
                    }
                    for &argument_handle in argument_handles {
                        self.check_expression(argument_handle, Expectation::None);
                    }
                    return Ty::error(self.db);
                }
            };

        if parameters.len() != argument_handles.len() {
            let function = match self.result.path_resolutions.get(callee_handle) {
                Some(Resolution::Function(function)) => Some(*function),
                _ => None,
            };
            let parameters = parameters
                .iter()
                .map(|&ty| self.inference.deep_resolve(ty))
                .collect();
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::MismatchedArgumentCount {
                    call_handle,
                    function,
                    parameters,
                });
        }
        for (index, &argument_handle) in argument_handles.iter().enumerate() {
            match parameters.get(index) {
                Some(&parameter) => {
                    let expectation = Expectation::of_type(self.db, parameter);
                    let argument_ty = self.check_expression(argument_handle, expectation);
                    self.coerce_or_report(argument_ty, expectation, argument_handle);
                }
                None => {
                    self.check_expression(argument_handle, Expectation::None);
                }
            }
        }

        return_ty
    }

    fn check_unary_operation(
        &mut self,
        expression_handle: ExpressionHandle,
        operator: UnaryOperator,
        operand_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        match operator {
            UnaryOperator::Negation => {
                let operand_ty = self.check_expression(operand_handle, expectation);
                let resolved = self.inference.shallow_resolve(operand_ty);
                match resolved.kind(self.db) {
                    TyKind::Signed(_) | TyKind::Bottom | TyKind::Error => operand_ty,
                    TyKind::InferenceVariable(_) if self.constrain_to_integer(resolved) => {
                        self.pending_negations.push((expression_handle, operand_ty));
                        operand_ty
                    }
                    kind => {
                        let is_unsigned = matches!(kind, TyKind::Unsigned(_));
                        let found = self.inference.deep_resolve(resolved);
                        self.result.add_diagnostic(
                            UnresolvedSemanticDiagnostic::UnaryOperatorCannotBeApplied {
                                expression_handle,
                                operator,
                                found,
                                is_unsigned,
                            },
                        );
                        Ty::error(self.db)
                    }
                }
            }
            UnaryOperator::Not => {
                let expectation = Expectation::of_type(self.db, Ty::bool(self.db));
                let operand_ty = self.check_expression(operand_handle, expectation);
                self.coerce_or_report(operand_ty, expectation, operand_handle);
                Ty::bool(self.db)
            }
        }
    }

    fn check_binary_operation(
        &mut self,
        expression_handle: ExpressionHandle,
        operator: Option<BinaryOperator>,
        lhs_handle: ExpressionHandle,
        rhs_handle: ExpressionHandle,
    ) -> Ty<'db> {
        let Some(operator) = operator else {
            // A syntax error, already reported by the parser: check both sides so they still get types and diagnostics
            self.check_expression(lhs_handle, Expectation::None);
            self.check_expression(rhs_handle, Expectation::None);
            return Ty::error(self.db);
        };

        let rule = operator.rule();
        let bool = Ty::bool(self.db);

        let lhs_expectation = if let OperandTypes::Bool = rule.operands {
            Expectation::of_type(self.db, bool)
        } else {
            Expectation::None
        };
        let lhs_ty = self.check_expression(lhs_handle, lhs_expectation);
        self.coerce_or_report(lhs_ty, lhs_expectation, lhs_handle);
        let lhs_diverges = self.diverges; // saves the lhs's divergence, since the lhs always runs

        let rhs_expectation = if let OperandTypes::Bool = rule.operands {
            // the rhs must be a `Bool` even when the lhs isn't. Without this, in `5 and x`,
            // `x` would be checked as an integer, or in `(return 1) and 5`, the `5`
            // wouldn't be checked at all.
            Expectation::of_type(self.db, bool)
        } else if self.is_bottom(lhs_ty) {
            // a diverging lhs says nothing about the rhs's type
            Expectation::None
        } else {
            // the rhs must have the lhs's type
            Expectation::of_type(self.db, lhs_ty)
        };
        let rhs_ty = self.check_expression(rhs_handle, rhs_expectation);
        self.coerce_or_report(rhs_ty, rhs_expectation, rhs_handle);

        if rule.short_circuits {
            // Depending on the lhs's value, the rhs may never run (`a and b` is
            // `if a { b } else { false }`), so, like an `if` without an `else`, only the
            // lhs's divergence matters
            self.diverges = lhs_diverges;
        }

        if let OperandTypes::SameInteger = rule.operands
            && !self.constrain_to_integer(lhs_ty)
        {
            let lhs = self.inference.deep_resolve(lhs_ty);
            let rhs = self.inference.deep_resolve(rhs_ty);
            if !lhs.is_poisoned(self.db) && !rhs.is_poisoned(self.db) {
                self.result.add_diagnostic(
                    UnresolvedSemanticDiagnostic::BinaryOperatorCannotBeApplied {
                        expression_handle,
                        operator,
                        lhs,
                        rhs,
                    },
                );
            }
            return Ty::error(self.db);
        }

        match rule.result {
            ResultType::Bool => bool,
            ResultType::Operand => lhs_ty,
        }
    }

    fn check_assignment(
        &mut self,
        assignment_handle: ExpressionHandle,
        target_handle: ExpressionHandle,
        value_handle: ExpressionHandle,
    ) -> Ty<'db> {
        let target_ty = self.check_expression(target_handle, Expectation::None);
        let value_expectation = Expectation::of_type(self.db, target_ty);
        let value_ty = self.check_expression(value_handle, value_expectation);
        self.coerce_or_report(value_ty, value_expectation, value_handle);

        if !self.is_place(target_handle) {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::InvalidAssignmentTarget {
                    assignment_handle,
                    target_handle,
                });
        }

        Ty::unit(self.db)
    }

    fn is_place(&self, expression_handle: ExpressionHandle) -> bool {
        match &self.body.store[expression_handle] {
            // An unresolved name was already reported.
            Expression::Path(_) => !matches!(
                self.result.path_resolutions.get(expression_handle),
                Some(Resolution::Function(_) | Resolution::Constant(_))
            ),
            Expression::Field {
                receiver_handle, ..
            } => self.is_place(*receiver_handle),
            Expression::Error => true,
            _ => false,
        }
    }

    fn finish(mut self) -> TypeCheckResult<'db> {
        self.inference.fallback_integer_variables();
        self.inference.fallback_general_variables();
        self.check_pending_negations();

        let mut reported_variables = HashSet::new();
        let mut unknown_expressions = Vec::new();
        for (expression_handle, ty) in self.result.expressions.iter_mut() {
            let mut unsolved = false;
            *ty = self
                .inference
                .deep_resolve_with_poisoning(*ty, &mut |variable_handle| {
                    unsolved |= reported_variables.insert(variable_handle);
                });
            if unsolved {
                unknown_expressions.push(expression_handle);
            }
        }
        for expression_handle in unknown_expressions {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMustBeKnown {
                    expression_handle,
                });
        }
        for (_, ty) in self.result.local_bindings.iter_mut() {
            *ty = self.inference.deep_resolve_with_poisoning(*ty, &mut |_| {});
        }
        self.check_literal_ranges();

        self.result
    }

    fn check_pending_negations(&mut self) {
        for (expression_handle, operand_ty) in std::mem::take(&mut self.pending_negations) {
            let resolved = self.inference.shallow_resolve(operand_ty);
            if let TyKind::Unsigned(_) = resolved.kind(self.db) {
                let found = self.inference.deep_resolve(resolved);
                self.result.add_diagnostic(
                    UnresolvedSemanticDiagnostic::UnaryOperatorCannotBeApplied {
                        expression_handle,
                        operator: UnaryOperator::Negation,
                        found,
                        is_unsigned: true,
                    },
                );
            }
        }
    }

    fn coerce_or_report(
        &mut self,
        ty: Ty<'db>,
        expectation: Expectation<'db>,
        expression_handle: ExpressionHandle,
    ) {
        if let Some(expected) = expectation.expected_type(&mut self.inference)
            && let Err(mismatch) = self.inference.coerce(ty, expected)
        {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMismatch {
                    expression_handle,
                    expected: self.inference.deep_resolve(mismatch.expected),
                    found: self.inference.deep_resolve(mismatch.found),
                    cause: TypeMismatchCause::General,
                });
        }
    }

    fn pour_or_report(
        &mut self,
        funnel: &mut CoercionFunnel<'db>,
        expression_handle: ExpressionHandle,
        ty: Ty<'db>,
        cause: TypeMismatchCause,
    ) {
        if let Err(mismatch) = funnel.pour(self.db, &mut self.inference, ty) {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMismatch {
                    expression_handle,
                    expected: self.inference.deep_resolve(mismatch.expected),
                    found: self.inference.deep_resolve(mismatch.found),
                    cause,
                });
        }
    }

    fn check_literal_ranges(&mut self) {
        let body = self.body;
        let negations: HashMap<ExpressionHandle, ExpressionHandle> = body
            .store
            .expressions()
            .filter_map(|(negation_handle, expression)| match expression {
                Expression::UnaryOperation {
                    operator: UnaryOperator::Negation,
                    operand_handle,
                } => Some((*operand_handle, negation_handle)),
                _ => None,
            })
            .collect();
        let literals: Vec<_> = self
            .result
            .expressions
            .iter()
            .filter_map(
                |(expression_handle, ty)| match body.store[expression_handle] {
                    Expression::Integer(value) => Some((expression_handle, value, *ty)),
                    _ => None,
                },
            )
            .collect();

        for (literal_handle, value, ty) in literals {
            let (min, max): (i128, i128) = match ty.kind(self.db) {
                TyKind::Signed(SignedIntTy::I32) => (i32::MIN.into(), i32::MAX.into()),
                TyKind::Signed(SignedIntTy::I64) => (i64::MIN.into(), i64::MAX.into()),
                TyKind::Unsigned(UnsignedIntTy::U32) => (0, u32::MAX.into()),
                TyKind::Unsigned(UnsignedIntTy::U64) => (0, u64::MAX.into()),
                _ => continue,
            };
            let negation_handle = negations.get(&literal_handle).copied().filter(|_| min < 0);
            let fits = value.is_some_and(|value| {
                let magnitude = i128::try_from(value).unwrap_or(i128::MAX);
                let signed_value = if negation_handle.is_some() {
                    -magnitude
                } else {
                    magnitude
                };
                (min..=max).contains(&signed_value)
            });
            if !fits {
                let suggested_ty = value
                    .and_then(|value| {
                        self.suggested_literal_type(ty, value, negation_handle.is_some())
                    })
                    .map(|suggested_ty| suggested_ty);
                let ty = ty;
                self.result
                    .add_diagnostic(UnresolvedSemanticDiagnostic::LiteralOutOfRange {
                        expression_handle: negation_handle.unwrap_or(literal_handle),
                        ty,
                        range: format!("{min}..={max}"),
                        suggested_ty,
                    });
            }
        }
    }

    fn suggested_literal_type(&self, ty: Ty<'db>, value: u128, negative: bool) -> Option<Ty<'db>> {
        let u32 = Ty::unsigned(self.db, UnsignedIntTy::U32);
        let u64 = Ty::unsigned(self.db, UnsignedIntTy::U64);
        let i64 = Ty::signed(self.db, SignedIntTy::I64);
        match ty.kind(self.db) {
            TyKind::Signed(SignedIntTy::I32) => {
                if !negative && value <= u32::MAX.into() {
                    Some(u32)
                } else if value <= i64::MAX as u128 + u128::from(negative) {
                    Some(i64)
                } else {
                    None
                }
            }
            TyKind::Signed(SignedIntTy::I64) => {
                (!negative && value <= u64::MAX.into()).then_some(u64)
            }
            TyKind::Unsigned(UnsignedIntTy::U32) => (value <= u64::MAX.into()).then_some(u64),
            _ => None,
        }
    }

    /// Constrains `ty` to be some integer type (without choosing which), returning whether it can be.
    fn constrain_to_integer(&mut self, ty: Ty<'db>) -> bool {
        let ty = self.inference.shallow_resolve(ty);
        match ty.kind(self.db) {
            TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::InferenceVariable(InferenceVariable::Integer(_))
            | TyKind::Bottom
            | TyKind::Error => true,
            TyKind::InferenceVariable(InferenceVariable::General(_)) => {
                let integer = self.inference.make_integer_inference_variable();
                self.inference.unify(ty, integer).is_ok()
            }
            TyKind::Tuple(_) | TyKind::Bool | TyKind::Function { .. } => false,
        }
    }
}

impl BinaryOperator {
    /// The types the operator accepts and produces.
    // Once blueberry has traits, these become the built-in `impl`s of each operator's trait.
    fn rule(self) -> BinaryOperatorRule {
        match self {
            Self::And | Self::Or => BinaryOperatorRule {
                operands: OperandTypes::Bool,
                result: ResultType::Bool,
                short_circuits: true,
            },
            Self::Add | Self::Sub | Self::Mul | Self::Div | Self::Mod => BinaryOperatorRule {
                operands: OperandTypes::SameInteger,
                result: ResultType::Operand,
                short_circuits: false,
            },
            Self::Lt | Self::Gt | Self::Le | Self::Ge | Self::Eq | Self::Ne => BinaryOperatorRule {
                operands: OperandTypes::Same,
                result: ResultType::Bool,
                short_circuits: false,
            },
        }
    }
}

impl<'db> Expectation<'db> {
    fn of_type(db: &'db dyn crate::Db, ty: Ty<'db>) -> Self {
        match ty.kind(db) {
            TyKind::Error => Self::None,
            _ => Self::Type(ty),
        }
    }

    fn expected_type(self, table: &mut InferenceTable<'db>) -> Option<Ty<'db>> {
        match self {
            Self::Type(ty) => Some(table.shallow_resolve(ty)),
            Self::None => None,
        }
    }
}

impl<'db> CoercionFunnel<'db> {
    fn new(expected: Ty<'db>) -> Self {
        Self {
            expected,
            merged: None,
        }
    }

    fn from_expectation(expectation: Expectation<'db>, table: &mut InferenceTable<'db>) -> Self {
        let expected = expectation
            .expected_type(table)
            .unwrap_or_else(|| table.new_general_inference_variable());
        Self::new(expected)
    }

    fn current(&self) -> Ty<'db> {
        self.merged.unwrap_or(self.expected)
    }

    fn pour(
        &mut self,
        db: &'db dyn crate::Db,
        table: &mut InferenceTable<'db>,
        ty: Ty<'db>,
    ) -> Result<(), TypeMismatch<'db>> {
        let result = table.coerce(ty, self.current());
        let merged = match result {
            Ok(()) => self.current(),
            Err(_) => Ty::error(db),
        };
        self.merged = Some(merged);
        result
    }

    fn drain(self, db: &'db dyn crate::Db) -> Ty<'db> {
        self.merged.unwrap_or_else(|| Ty::bottom(db))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path as FilePath;

    use crate::core::db::BlueberryDatabase;
    use crate::core::module_map::{ModuleMap, ModulePath};
    use crate::core::semantic_analysis::ast_lowering::dumper::HirBodyDumper;
    use crate::core::source_file_key::SourceFileKey;

    #[test]
    fn test_type_dump() {
        insta::glob!("snapshot_inputs", "*.bb", |path| {
            let input = fs::read_to_string(path).unwrap();
            let db = BlueberryDatabase::default();
            let file = SourceFileKey::new(&db, path.to_path_buf(), input);
            insta::assert_snapshot!(HirBodyDumper::dump_file_with_types(&db, file));
        })
    }

    #[test]
    fn test_module_type_dump() {
        insta::glob!("snapshot_inputs", "modules/*/main.bb", |path| {
            let db = BlueberryDatabase::default();
            let modules = ModuleMap::load(&db, path.parent().unwrap()).unwrap();
            let main = ModulePath::from_file_path(FilePath::new("main.bb"));
            let file = modules.files(&db)[&main];
            insta::assert_snapshot!(HirBodyDumper::dump_file_with_types(&db, file));
        })
    }
}
