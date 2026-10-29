use std::collections::HashMap;

use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, ExpressionHandle, LocalBindingHandle, StatementHandle, TypeExpressionHandle,
};
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::semantic_analysis::name_resolution::resolver::Resolution;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::semantic_diagnostic::{
    TypeMismatchCause, UnresolvedSemanticDiagnostic,
};
use crate::core::semantic_analysis::signature;
use crate::core::semantic_analysis::type_checking::inference_table::{
    InferenceTable, TypeMismatch, VariableOrigin,
};
use crate::core::semantic_analysis::type_checking::type_expression_resolution::resolve_type_expression;
use crate::core::semantic_analysis::type_checking::types::{InferenceVariable, Ty, TyKind};

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct BodySemantics<'db> {
    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
    path_resolutions: SideHandleMap<ExpressionHandle, Resolution<'db>>,
    diagnostics: Vec<UnresolvedSemanticDiagnostic<'db>>,
}

struct TypeChecker<'a, 'db> {
    db: &'db dyn crate::Db,
    body: &'a nodes::Body<'db>,
    scopes: &'a ScopeTree<'db>,
    inference: InferenceTable<'db>,
    owner: BodyOwner<'db>,
    /// whether the point the walk has reached can run, which it can't once something before it has
    /// definitely left (a `return`, say)
    is_reachable: bool,
    loop_frames: Vec<LoopFrame<'db>>,
    pending_negations: Vec<(ExpressionHandle, Ty<'db>)>,
    result: BodySemantics<'db>,
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

/// What a body belongs to.
enum BodyOwner<'db> {
    /// a function, whose `return`s, and its body's tail, pour their values into its return
    /// funnel, which has the function's return type
    Function(CoercionFunnel<'db>),
    /// a constant, which isn't called, so there's nothing to return from, and it can't have a
    /// `return`
    Constant,
}

struct LoopFrame<'db> {
    /// whether a `break` that can be reached exits the loop, so that the code after it can run
    can_exit: bool,
    kind: LoopKind<'db>,
}

/// Which kind of loop a loop is.
enum LoopKind<'db> {
    /// a `loop`, whose `break`s pour their values into its funnel, which gives the loop its type
    Loop(CoercionFunnel<'db>),
    /// a `while`, which can also end because its condition is false, without any `break`, so it
    /// has no value to give, and its `break`s can't have one
    While,
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

pub(crate) fn check_function<'db>(
    db: &'db dyn crate::Db,
    body: &nodes::Body<'db>,
    scopes: &ScopeTree<'db>,
    signature_type: Ty<'db>,
) -> BodySemantics<'db> {
    let TyKind::Function {
        parameters,
        r#return,
    } = signature_type.kind(db)
    else {
        unreachable!("a function's signature type is always a function type");
    };

    let owner = BodyOwner::Function(CoercionFunnel::new(*r#return));
    let mut checker = TypeChecker::new(db, body, scopes, owner);

    let binding_handles = body.parameters();
    for (&binding_handle, &ty) in binding_handles.iter().zip(parameters) {
        checker.result.local_bindings.add(binding_handle, ty);
    }

    checker.check_return_value(body.root());

    checker.finish()
}

pub(crate) fn check_constant<'db>(
    db: &'db dyn crate::Db,
    body: &nodes::Body<'db>,
    scopes: &ScopeTree<'db>,
    declared_type: Ty<'db>,
) -> BodySemantics<'db> {
    let mut checker = TypeChecker::new(db, body, scopes, BodyOwner::Constant);

    let expectation = Expectation::of_type(db, declared_type);
    let ty = checker.check_expression(body.root(), expectation);
    checker.coerce_or_report(ty, expectation, body.root());

    checker.finish()
}

impl<'db> BodySemantics<'db> {
    pub(crate) fn expression_type(&self, expression_handle: ExpressionHandle) -> Ty<'db> {
        self.expressions[expression_handle]
    }

    pub(crate) fn local_binding_type(&self, binding_handle: LocalBindingHandle) -> Ty<'db> {
        self.local_bindings[binding_handle]
    }

    pub(crate) fn path_resolution(
        &self,
        expression_handle: ExpressionHandle,
    ) -> Option<Resolution<'db>> {
        self.path_resolutions.get(expression_handle).copied()
    }

    #[expect(
        dead_code,
        reason = "type-check result accessors, for the language server"
    )]
    pub(crate) fn expressions(&self) -> impl Iterator<Item = (ExpressionHandle, Ty<'db>)> + '_ {
        self.expressions
            .iter()
            .map(|(expression_handle, ty)| (expression_handle, *ty))
    }

    #[expect(
        dead_code,
        reason = "type-check result accessors, for the language server"
    )]
    pub(crate) fn local_bindings(
        &self,
    ) -> impl Iterator<Item = (LocalBindingHandle, Ty<'db>)> + '_ {
        self.local_bindings
            .iter()
            .map(|(binding_handle, ty)| (binding_handle, *ty))
    }

    pub(crate) fn diagnostics(&self) -> &[UnresolvedSemanticDiagnostic<'db>] {
        &self.diagnostics
    }

    pub(crate) fn has_errors(&self, db: &'db dyn crate::Db) -> bool {
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

impl<'a, 'db> TypeChecker<'a, 'db> {
    fn new(
        db: &'db dyn crate::Db,
        body: &'a nodes::Body<'db>,
        scopes: &'a ScopeTree<'db>,
        owner: BodyOwner<'db>,
    ) -> Self {
        Self {
            db,
            body,
            scopes,
            inference: InferenceTable::new(db),
            owner,
            is_reachable: true,
            loop_frames: Vec::new(),
            pending_negations: Vec::new(),
            result: BodySemantics::new(),
        }
    }

    fn check_expression(
        &mut self,
        expression_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let ty = match self.body.store().expression(expression_handle) {
            nodes::Expression::If {
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
            nodes::Expression::Block {
                statement_run,
                tail_handle,
                ..
            } => self.check_block(
                expression_handle,
                self.body.store().statement_handles(*statement_run),
                *tail_handle,
                expectation,
            ),
            nodes::Expression::Tuple { element_run } => self.check_tuple(
                self.body.store().expression_handles(*element_run),
                expectation,
            ),
            nodes::Expression::Field {
                receiver_handle,
                name,
            } => self.check_field(expression_handle, *receiver_handle, *name),
            nodes::Expression::Loop {
                source,
                body_handle,
            } => self.check_loop(expression_handle, *source, *body_handle, expectation),
            nodes::Expression::Call {
                callee_handle,
                argument_run,
            } => self.check_call(
                expression_handle,
                *callee_handle,
                self.body.store().expression_handles(*argument_run),
            ),
            nodes::Expression::Continue => self.check_continue(expression_handle),
            nodes::Expression::Break { value_handle } => {
                self.check_break(expression_handle, *value_handle)
            }
            nodes::Expression::Return { value_handle } => {
                self.check_return(expression_handle, *value_handle)
            }
            nodes::Expression::UnaryOperation {
                operator,
                operand_handle,
            } => self.check_unary_operation(
                expression_handle,
                *operator,
                *operand_handle,
                expectation,
            ),
            nodes::Expression::BinaryOperation {
                lhs_handle,
                operator,
                rhs_handle,
            } => {
                self.check_binary_operation(expression_handle, *operator, *lhs_handle, *rhs_handle)
            }
            nodes::Expression::Assignment {
                target_handle,
                value_handle,
            } => self.check_assignment(expression_handle, *target_handle, *value_handle),
            nodes::Expression::Boolean(_) => Ty::bool(self.db),
            nodes::Expression::Integer(_) => self.check_integer_literal(expectation),
            nodes::Expression::Path(path) => self.check_path(expression_handle, *path),
            // A syntax error, already reported by the parser.
            nodes::Expression::Error => Ty::error(self.db),
        };

        self.result.expressions.add(expression_handle, ty);

        if self.is_bottom(ty) {
            self.is_reachable = false;
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

    fn check_path(
        &mut self,
        expression_handle: ExpressionHandle,
        path: nodes::Path<'db>,
    ) -> Ty<'db> {
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
            Ok(Resolution::Function(key)) => {
                *signature::type_of(self.db, Definition::Function(key))
            }
            Ok(Resolution::Constant(key)) => {
                *signature::type_of(self.db, Definition::Constant(key))
            }
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
        let element_expectations: Vec<Expectation<'db>> = if let Some(expected) =
            expectation.expected_type(&mut self.inference)
            && let TyKind::Tuple(expected_elements) = expected.kind(self.db)
            && expected_elements.len() == element_handles.len()
        {
            expected_elements
                .iter()
                .map(|&ty| Expectation::of_type(self.db, ty))
                .collect()
        } else {
            vec![Expectation::None; element_handles.len()]
        };

        let mut elements = Vec::with_capacity(element_handles.len());
        for (&element_handle, element_expectation) in
            element_handles.iter().zip(element_expectations)
        {
            let element_ty = self.check_expression(element_handle, element_expectation);
            self.coerce_or_report(element_ty, element_expectation, element_handle);
            // we don't put an element's own type in the tuple's type when the element has an
            // expected type, because the parent then checks the whole tuple against the same
            // expected type, and would report the element's mistake a second time. For example:
            //
            //     let p: (I32, Bool) = (true, true);
            //
            // the first `true` is reported here, as not being an `I32`. The tuple is then
            // `(I32, Bool)` rather than `(Bool, Bool)`, so when the `let` compares the tuple with
            // its annotation, they match, and the mistake is only reported once.
            elements.push(match element_expectation {
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
        field_name: Option<Symbol<'db>>,
    ) -> Ty<'db> {
        let receiver_ty = self.check_expression(receiver_handle, Expectation::None);

        let Some(field_name) = field_name else {
            return Ty::error(self.db);
        };

        let receiver_ty = self.inference.shallow_resolve(receiver_ty);

        // the receivers whose fields can't be looked up
        match receiver_ty.kind(self.db) {
            // already reported
            TyKind::Error => return Ty::error(self.db),
            // the receiver never produces a value, so the field is never taken
            TyKind::Bottom => return Ty::bottom(self.db),
            TyKind::InferenceVariable(InferenceVariable::General(_)) => {
                // poisons the unknown, so that it's only reported once
                let (_, Some(origin)) = self.inference.deep_resolve_with_poisoning(receiver_ty)
                else {
                    unreachable!("the receiver is an unsolved variable, so it has an origin");
                };
                self.result
                    .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMustBeKnown {
                        expression_handle: receiver_handle,
                        origin,
                    });
                return Ty::error(self.db);
            }
            TyKind::Tuple(_)
            | TyKind::Bool
            | TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::InferenceVariable(InferenceVariable::Integer(_))
            | TyKind::Function { .. } => {}
        }

        if let TyKind::Tuple(elements) = receiver_ty.kind(self.db)
            && let Some(index) = field_name.as_tuple_index(self.db)
            && let Some(&element) = elements.get(index as usize)
        {
            return element;
        }
        let is_primitive = matches!(
            receiver_ty.kind(self.db),
            TyKind::Bool
                | TyKind::Signed(_)
                | TyKind::Unsigned(_)
                | TyKind::InferenceVariable(InferenceVariable::Integer(_))
        );
        let receiver_ty = self.inference.deep_resolve(receiver_ty);
        self.result
            .add_diagnostic(UnresolvedSemanticDiagnostic::UnknownField {
                expression_handle,
                ty: receiver_ty,
                is_primitive,
            });
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
        let mut funnel =
            CoercionFunnel::from_expectation(expectation, &mut self.inference, expression_handle);

        // typecheck condition
        let condition_expectation = Expectation::of_type(self.db, Ty::bool(self.db));
        let condition_ty = self.check_expression(condition_handle, condition_expectation);
        self.coerce_or_report(condition_ty, condition_expectation, condition_handle);
        let condition_reachable = std::mem::replace(&mut self.is_reachable, true); // saves the condition's reachability, and resets it for the branches

        // typecheck the then branch, and pour it into the funnel
        let then_ty = self.check_expression(then_branch_handle, expectation);
        let then_reachable = std::mem::replace(&mut self.is_reachable, true); // saves the then branch's reachability, and resets it for the else branch
        self.pour_or_report(
            &mut funnel,
            then_branch_handle,
            then_ty,
            TypeMismatchCause::General,
        );

        if let Some(else_branch_handle) = else_branch_handle {
            // typecheck the else branch, and pour it into the funnel
            let else_ty = self.check_expression(else_branch_handle, expectation);
            let else_reachable = self.is_reachable; // saves the else branch's reachability
            self.pour_or_report(
                &mut funnel,
                else_branch_handle,
                else_ty,
                TypeMismatchCause::IfElse { then_branch_handle },
            );

            // with an `else`, one of the branches always runs, so the code after the `if` can run if
            // the condition finishes, and either branch does
            self.is_reachable = condition_reachable && (then_reachable || else_reachable);
        } else {
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
            self.is_reachable = condition_reachable;
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
        let mut funnel =
            CoercionFunnel::from_expectation(expectation, &mut self.inference, expression_handle);

        // check each statement in order.
        // A diverging one (e.g., `return 1;`) makes the rest of the block unreachable
        for statement_handle in statement_handles {
            match self.body.store().statement(*statement_handle) {
                nodes::Statement::Let {
                    name_handle,
                    annotation_handle,
                    initializer_handle,
                } => self.check_let(*name_handle, *annotation_handle, *initializer_handle),
                nodes::Statement::Expression {
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
                nodes::Statement::Definition => {}
            }
        }

        // The tail of a function's body is the function's result.
        let is_function_body =
            matches!(self.owner, BodyOwner::Function(_)) && expression_handle == self.body.root();
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
                if self.is_reachable {
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
            Some(annotation_handle) => {
                let resolved =
                    resolve_type_expression(self.db, self.body.store(), annotation_handle);
                for type_expression_handle in resolved.unknown_paths {
                    self.result
                        .add_diagnostic(UnresolvedSemanticDiagnostic::UnknownType {
                            type_expression_handle,
                        });
                }
                resolved.ty
            }
            None => self
                .inference
                .make_general_inference_variable(VariableOrigin::LocalBinding(binding_handle)),
        };
        let expectation = Expectation::of_type(self.db, annotated_ty);
        let initializer_ty = self.check_expression(initializer_handle, expectation);
        self.coerce_or_report(initializer_ty, expectation, initializer_handle);
        self.result.local_bindings.add(binding_handle, annotated_ty);
    }

    fn check_loop(
        &mut self,
        expression_handle: ExpressionHandle,
        source: nodes::LoopSource,
        loop_body_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let kind = match source {
            nodes::LoopSource::Loop => LoopKind::Loop(CoercionFunnel::from_expectation(
                expectation,
                &mut self.inference,
                expression_handle,
            )),
            nodes::LoopSource::While => LoopKind::While,
        };
        self.loop_frames.push(LoopFrame {
            can_exit: false,
            kind,
        });

        // the body's value is discarded each time around, so it must be `()`
        let body_expectation = Expectation::of_type(self.db, Ty::unit(self.db));
        let body_ty = self.check_expression(loop_body_handle, body_expectation);
        self.coerce_or_report(body_ty, body_expectation, loop_body_handle);

        let Some(frame) = self.loop_frames.pop() else {
            unreachable!("the loop's frame was pushed above");
        };

        // a loop only finishes through a `break`, so the code after it can run if a `break` exits it
        self.is_reachable = frame.can_exit;

        match frame.kind {
            LoopKind::Loop(funnel) => funnel.drain(self.db),
            LoopKind::While => Ty::unit(self.db),
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

        let value_expectation = match &frame.kind {
            LoopKind::Loop(funnel) => Expectation::of_type(self.db, funnel.expected),
            LoopKind::While => Expectation::None,
        };
        let value = value_handle.map(|value_handle| {
            let value_ty = self.check_expression(value_handle, value_expectation);
            (value_handle, value_ty)
        });

        let Some(frame) = self.loop_frames.last_mut() else {
            unreachable!("the `break` is inside a loop, as checked above");
        };
        // we check `self.is_reachable` here because type checking the value might have modified it
        if self.is_reachable {
            frame.can_exit = true; // a `break` only exits its loop if
            // control still reaches it, which neither, say, `loop { return; break; }` nor
            // `loop { break return 5; }` does
        }

        match &mut frame.kind {
            LoopKind::Loop(funnel) => {
                // a bare `break` gives the loop `()`
                let (loop_value_handle, loop_value_ty) =
                    value.unwrap_or((expression_handle, Ty::unit(self.db))); // if there's no value, we want the diagnostic to point to the entire `break` i.e. `expression_handle`
                if let Err(mismatch) = funnel.pour(self.db, &mut self.inference, loop_value_ty) {
                    self.result
                        .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMismatch {
                            expression_handle: loop_value_handle,
                            expected: self.inference.deep_resolve(mismatch.expected),
                            found: self.inference.deep_resolve(mismatch.found),
                            cause: TypeMismatchCause::General,
                        });
                }
            }
            LoopKind::While => {
                if value.is_some() {
                    self.result.add_diagnostic(
                        UnresolvedSemanticDiagnostic::BreakWithValueFromWhile { expression_handle },
                    );
                }
            }
        }

        Ty::bottom(self.db) // the break expression's type (not to be confused with the break's value)
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
        let BodyOwner::Function(return_funnel) = &mut self.owner else {
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
            Some(value_handle) => self.check_return_value(value_handle), // `return e`: `e` is checked with the return type, and poured into the return funnel
            None => {
                // bare `return`: returns `()`, so `()` is poured into the return funnel
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

        Ty::bottom(self.db) // return expression's type is always bottom
    }

    fn check_return_value(&mut self, value_handle: ExpressionHandle) {
        let BodyOwner::Function(return_funnel) = &self.owner else {
            unreachable!("only a function's body returns a value");
        };

        let expectation = Expectation::of_type(self.db, return_funnel.expected);
        let value_ty = self.check_expression(value_handle, expectation);

        // the funnel is looked up again after the value is checked, since a `return` inside the
        // value pours into it too
        let BodyOwner::Function(return_funnel) = &mut self.owner else {
            unreachable!("only a function's body returns a value");
        };
        if let Err(mismatch) = return_funnel.pour(self.db, &mut self.inference, value_ty) {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMismatch {
                    expression_handle: value_handle,
                    expected: self.inference.deep_resolve(mismatch.expected),
                    found: self.inference.deep_resolve(mismatch.found),
                    cause: TypeMismatchCause::ReturnValue,
                });
        }
    }

    fn check_call(
        &mut self,
        call_handle: ExpressionHandle,
        callee_handle: ExpressionHandle,
        argument_handles: &[ExpressionHandle],
    ) -> Ty<'db> {
        let callee_ty = self.check_expression(callee_handle, Expectation::None);
        let callee_ty = self.inference.shallow_resolve(callee_ty);

        let (parameters, call_ty) = match callee_ty.kind(self.db) {
            TyKind::Function {
                parameters,
                r#return,
            } => (Some(parameters.clone()), *r#return),
            // the callee never produces a value, so the call never happens
            TyKind::Bottom => (None, Ty::bottom(self.db)),
            // already reported
            TyKind::Error => (None, Ty::error(self.db)),
            TyKind::InferenceVariable(InferenceVariable::General(_)) => {
                let (_, Some(origin)) = self.inference.deep_resolve_with_poisoning(callee_ty)
                else {
                    unreachable!("the callee is an unsolved variable, so it has an origin");
                };
                self.result
                    .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMustBeKnown {
                        expression_handle: callee_handle,
                        origin,
                    });
                (None, Ty::error(self.db))
            }
            TyKind::InferenceVariable(InferenceVariable::Integer(_))
            | TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::Bool
            | TyKind::Tuple(_) => {
                let found = self.inference.deep_resolve(callee_ty);
                self.result
                    .add_diagnostic(UnresolvedSemanticDiagnostic::ExpectedFunction {
                        call_handle,
                        found,
                    });
                (None, Ty::error(self.db))
            }
        };

        let Some(parameters) = parameters else {
            // there are no parameters to check the arguments against, but they're still checked to surface errors
            for &argument_handle in argument_handles {
                self.check_expression(argument_handle, Expectation::None);
            }
            return call_ty;
        };

        if argument_handles.len() != parameters.len() {
            let function = match self.result.path_resolutions.get(callee_handle) {
                Some(Resolution::Function(function)) => Some(*function),
                _ => None,
            };
            let parameters = parameters
                .iter()
                .map(|&ty| self.inference.deep_resolve(ty))
                .collect(); // deep resolve for diagnostics
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
                    // surface errors in the arguments
                    self.check_expression(argument_handle, Expectation::None);
                }
            }
        }

        call_ty
    }

    fn check_unary_operation(
        &mut self,
        expression_handle: ExpressionHandle,
        operator: nodes::UnaryOperator,
        operand_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        match operator {
            nodes::UnaryOperator::Negation => {
                let operand_ty = self.check_expression(operand_handle, expectation);

                let is_integer = self.unify_with_integer(operand_ty).is_ok();
                let is_unsigned = matches!(
                    self.inference.shallow_resolve(operand_ty).kind(self.db),
                    TyKind::Unsigned(_)
                );
                if !is_integer || is_unsigned {
                    self.result.add_diagnostic(
                        UnresolvedSemanticDiagnostic::UnaryOperatorCannotBeApplied {
                            expression_handle,
                            operator,
                            found: self.inference.deep_resolve(operand_ty),
                            is_unsigned,
                        },
                    );
                    return Ty::error(self.db);
                }

                // the operand might be an integer whose type isn't known yet, (e.g., `-x` in `let x = 5; let y = -x; let z: U32 = x;`), so it's checked
                // again in `finish`
                self.pending_negations.push((expression_handle, operand_ty));

                operand_ty
            }
            nodes::UnaryOperator::Not => {
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
        operator: Option<nodes::BinaryOperator>,
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
        let lhs_reachable = self.is_reachable; // saves the lhs's reachability, since the lhs always runs

        // `+`, `-`, `*`, `div` and `mod` can only be applied to integers. This is checked on the lhs before the rhs is
        // checked, since the rhs is checked expecting the lhs's type, which is only right if the lhs
        // is an integer.
        let operator_cannot_be_applied = matches!(rule.operands, OperandTypes::SameInteger)
            && self.unify_with_integer(lhs_ty).is_err();

        let rhs_expectation = if let OperandTypes::Bool = rule.operands {
            // the rhs must be a `Bool` even when the lhs isn't. Without this, in `5 and x`,
            // `x` would be checked as an integer, or in `(return 1) and 5`, the `5`
            // wouldn't be checked at all.
            Expectation::of_type(self.db, bool)
        } else if self.is_bottom(lhs_ty) || operator_cannot_be_applied {
            // a diverging lhs, or a lhs that the operator can't be applied to says nothing about the rhs's type
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
            // lhs's reachability matters
            self.is_reachable = lhs_reachable;
        }

        if operator_cannot_be_applied {
            let lhs = self.inference.deep_resolve(lhs_ty);
            let rhs = self.inference.deep_resolve(rhs_ty);
            self.result.add_diagnostic(
                UnresolvedSemanticDiagnostic::BinaryOperatorCannotBeApplied {
                    expression_handle,
                    operator,
                    lhs,
                    rhs,
                },
            );
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
        match self.body.store().expression(expression_handle) {
            // An unresolved name was already reported.
            nodes::Expression::Path(_) => !matches!(
                self.result.path_resolutions.get(expression_handle),
                Some(Resolution::Function(_) | Resolution::Constant(_))
            ),
            nodes::Expression::Field {
                receiver_handle, ..
            } => self.is_place(*receiver_handle),
            nodes::Expression::Error => true,
            _ => false,
        }
    }

    fn finish(mut self) -> BodySemantics<'db> {
        self.inference.fallback_integer_variables(); // fall back to `I32`
        self.inference.fallback_general_variables(); // fall back `Bottom``, but only if it was coerced from `Bottom`

        self.check_pending_negations();

        // each unknown is poisoned the first time it's found, and so reported only once
        let mut unknowns = Vec::new();
        for (expression_handle, ty) in &mut self.result.expressions {
            let (resolved, origin) = self.inference.deep_resolve_with_poisoning(*ty);
            *ty = resolved;
            if let Some(origin) = origin {
                unknowns.push((expression_handle, origin));
            }
        }

        for (expression_handle, origin) in unknowns {
            self.result
                .add_diagnostic(UnresolvedSemanticDiagnostic::TypeMustBeKnown {
                    expression_handle,
                    origin,
                });
        }

        for (_, ty) in &mut self.result.local_bindings {
            let (resolved, _) = self.inference.deep_resolve_with_poisoning(*ty);
            *ty = resolved;
            // we don't report the bindings' unknowns, because a binding has the same type as its
            // initializer, which is an expression, so its unknowns were already reported just above.
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
                        operator: nodes::UnaryOperator::Negation,
                        found,
                        is_unsigned: true,
                    },
                );
            }
        }
    }

    fn check_literal_ranges(&mut self) {
        let negations: HashMap<ExpressionHandle, ExpressionHandle> = self
            .body
            .store()
            .expressions()
            .filter_map(|(negation_handle, expression)| match expression {
                nodes::Expression::UnaryOperation {
                    operator: nodes::UnaryOperator::Negation,
                    operand_handle,
                } => Some((*operand_handle, negation_handle)),
                _ => None,
            })
            .collect();

        let literals: Vec<_> = self
            .result
            .expressions
            .iter()
            .filter_map(|(expression_handle, ty)| {
                match self.body.store().expression(expression_handle) {
                    nodes::Expression::Integer(value) => Some((expression_handle, value, *ty)),
                    _ => None,
                }
            })
            .collect();

        for (literal_handle, value, ty) in literals {
            let Some(range) = ty.integer_range(self.db) else {
                continue;
            };
            // only a type that has negative values can hold `-5`, so for any other, the negation
            // is reported by itself
            let negation_handle = negations
                .get(&literal_handle)
                .copied()
                .filter(|_| *range.start() < 0);
            // `None` when the literal is too big to even be read
            let signed_value = value.map(|value| {
                let magnitude = i128::try_from(value).unwrap_or(i128::MAX);
                if negation_handle.is_some() {
                    -magnitude
                } else {
                    magnitude
                }
            });

            if !signed_value.is_some_and(|value| range.contains(&value)) {
                let suggested_ty =
                    signed_value.and_then(|value| ty.wider_integer_type(self.db, value));
                self.result
                    .add_diagnostic(UnresolvedSemanticDiagnostic::LiteralOutOfRange {
                        expression_handle: negation_handle.unwrap_or(literal_handle),
                        ty,
                        range: format!("{}..={}", range.start(), range.end()),
                        suggested_ty,
                    });
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

    /// Unifies `ty` with some integer type, without choosing which.
    ///
    /// `Bottom` is accepted too, since a diverging expression never produces a value.
    fn unify_with_integer(&mut self, ty: Ty<'db>) -> Result<(), ()> {
        let ty = self.inference.shallow_resolve(ty);
        match ty.kind(self.db) {
            TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::InferenceVariable(InferenceVariable::Integer(_))
            | TyKind::Bottom
            | TyKind::Error => Ok(()),
            TyKind::InferenceVariable(InferenceVariable::General(_)) => {
                let integer = self.inference.make_integer_inference_variable();
                self.inference.unify(ty, integer).map_err(|_| ())
            }
            TyKind::Tuple(_) | TyKind::Bool | TyKind::Function { .. } => Err(()),
        }
    }
}

impl nodes::BinaryOperator {
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

    /// Creates the funnel of the expression `owner_handle`, expecting the type its expectation says,
    /// or an unknown one.
    fn from_expectation(
        expectation: Expectation<'db>,
        table: &mut InferenceTable<'db>,
        owner_handle: ExpressionHandle,
    ) -> Self {
        let expected = expectation.expected_type(table).unwrap_or_else(|| {
            table.make_general_inference_variable(VariableOrigin::Expression(owner_handle))
        });
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
        });
    }

    #[test]
    fn test_module_type_dump() {
        insta::glob!("snapshot_inputs", "modules/*/main.bb", |path| {
            let db = BlueberryDatabase::default();
            let modules = ModuleMap::load(&db, path.parent().unwrap()).unwrap();
            let main = ModulePath::from_file_path(FilePath::new("main.bb"));
            let file = modules.files(&db)[&main];
            insta::assert_snapshot!(HirBodyDumper::dump_file_with_types(&db, file));
        });
    }
}
