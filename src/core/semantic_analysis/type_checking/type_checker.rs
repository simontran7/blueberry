use std::collections::HashSet;

use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::semantic_analysis::hir::nodes::{
    BinaryOperator, DefinitionBody, Expression, ExpressionHandle, LocalBindingHandle, LoopSource,
    Statement, StatementHandle, TypeAnnotation, TypeAnnotationHandle, UnaryOperator,
};
use crate::core::semantic_analysis::ids::keys::BlockKey;
use crate::core::semantic_analysis::name_resolution::resolver::{Resolution, Resolver};
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::type_checking::inference_diagnostic::{
    InferenceDiagnostic, MismatchCause,
};
use crate::core::semantic_analysis::type_checking::inference_table::InferenceTable;
use crate::core::semantic_analysis::type_checking::types::{InferenceVariable, Ty, TyKind};
use crate::core::semantic_analysis::{constant_signature_type_of, function_signature_type_of};
use crate::core::source_file_key::SourceFileKey;

pub(crate) struct TypeChecker<'a, 'db> {
    db: &'db dyn crate::Db,
    body: &'a DefinitionBody<'db>,
    scopes: &'a ScopeTree<'db>,
    file: SourceFileKey,
    enclosing_block: Option<BlockKey<'db>>,
    table: InferenceTable<'db>,

    owner: BodyOwner<'db>,
    /// Whether the code walked so far always diverges (i.e., never finishes normally).
    diverges: Diverges,
    /// The loops enclosing the expression being checked, innermost last.
    breakables: Vec<BreakableContext<'db>>,

    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
    diagnostics: Vec<InferenceDiagnostic<'db>>,
    /// Negations of operands whose integer type wasn't known yet, checked once it is.
    deferred_negations: Vec<(ExpressionHandle, Ty<'db>)>,
    expressions_with_mismatches: HashSet<ExpressionHandle>,
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct TypeCheckSink<'db> {
    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
    diagnostics: Vec<InferenceDiagnostic<'db>>,
}

/// The definition whose body is being checked, which decides what the body's value must be.
enum BodyOwner<'db> {
    /// The body's value, and each `return`'s value, are coerced to the return type.
    Function { return_coercion: CoerceMany<'db> },
    /// The body's value is coerced to the declared type, and `return` isn't allowed.
    Constant { declared_type: Ty<'db> },
}

/// What the surrounding code expects an expression's type to be, used as a hint while checking it.
#[derive(Clone, Copy)]
enum Expectation<'db> {
    None,
    HasType(Ty<'db>),
}

#[derive(Clone, Copy)]
struct CoerceMany<'db> {
    expected_ty: Ty<'db>,
    final_ty: Option<Ty<'db>>,
}

struct BreakableContext<'db> {
    may_break: bool,
    coerce: Option<CoerceMany<'db>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Diverges {
    Maybe,
    Always,
}

impl<'a, 'db> TypeChecker<'a, 'db> {
    /// Gives each parameter its declared type, and coerces the body's value and each
    /// `return`'s value to the return type.
    pub(crate) fn for_function(
        db: &'db dyn crate::Db,
        body: &'a DefinitionBody<'db>,
        scopes: &'a ScopeTree<'db>,
        file: SourceFileKey,
        enclosing_block: Option<BlockKey<'db>>,
        signature_type: Ty<'db>,
    ) -> Self {
        let TyKind::Function {
            parameters,
            r#return,
        } = signature_type.kind(db)
        else {
            unreachable!("a function's signature type is always a function type");
        };

        let owner = BodyOwner::Function {
            return_coercion: CoerceMany::new(*r#return),
        };
        let mut checker = Self::new(db, body, scopes, file, enclosing_block, owner);
        let binding_handles = &body.binding_children[body.parameter_segment];
        for (&binding_handle, &ty) in binding_handles.iter().zip(parameters) {
            checker.local_bindings.add(binding_handle, ty);
        }
        checker
    }

    pub(crate) fn for_constant(
        db: &'db dyn crate::Db,
        body: &'a DefinitionBody<'db>,
        scopes: &'a ScopeTree<'db>,
        file: SourceFileKey,
        enclosing_block: Option<BlockKey<'db>>,
        declared_type: Ty<'db>,
    ) -> Self {
        let owner = BodyOwner::Constant { declared_type };
        Self::new(db, body, scopes, file, enclosing_block, owner)
    }

    /// Checks the body: a function's body like a `return` of its value, and a constant's value by
    /// coercing it to its declared type.
    pub(crate) fn check_body(&mut self) {
        let root_handle = self.body.root_handle;
        match self.owner {
            BodyOwner::Function { .. } => self.check_return_value(root_handle),
            BodyOwner::Constant { declared_type } => {
                let expectation = Expectation::has_type(self.db, declared_type);
                self.check_expression_coerce(root_handle, expectation);
            }
        }
    }

    /// Falls back unsolved integer variables to `I32` and unsolved general variables
    /// coerced from `Bottom` to `Bottom`, then replaces every inference variable with its solution
    /// in the recorded types, so that they're final.
    ///
    /// Diagnostics aren't touched: they keep types as they were known when reported (see
    /// `InferenceDiagnostic`).
    pub(crate) fn finish(mut self) -> TypeCheckSink<'db> {
        self.table.fallback_integer_variables();
        self.table.fallback_general_variables();

        for (expression_handle, operand_ty) in std::mem::take(&mut self.deferred_negations) {
            // Reported now, since the operand's type is only known after the fallback.
            let operand_ty = self
                .table
                .deep_resolve_with_poisoning(operand_ty, &mut |_| {});
            if let TyKind::Unsigned(_) = operand_ty.kind(self.db) {
                let found = self.render(operand_ty);
                self.push_diagnostic(InferenceDiagnostic::UnaryOperatorCannotBeApplied {
                    expression_handle,
                    operator: UnaryOperator::Neg,
                    found,
                    is_unsigned: true,
                });
            }
        }

        let mut reported_variables = HashSet::new();
        for (expression_handle, ty) in self.expressions.iter_mut() {
            let mut unsolved = false;
            *ty = self
                .table
                .deep_resolve_with_poisoning(*ty, &mut |variable_handle| {
                    unsolved |= reported_variables.insert(variable_handle);
                });
            if unsolved {
                self.diagnostics
                    .push(InferenceDiagnostic::TypeMustBeKnown { expression_handle });
            }
        }
        for (_, ty) in self.local_bindings.iter_mut() {
            *ty = self.table.deep_resolve_with_poisoning(*ty, &mut |_| {});
        }

        TypeCheckSink {
            expressions: self.expressions,
            local_bindings: self.local_bindings,
            diagnostics: self.diagnostics,
        }
    }

    fn new(
        db: &'db dyn crate::Db,
        body: &'a DefinitionBody<'db>,
        scopes: &'a ScopeTree<'db>,
        file: SourceFileKey,
        enclosing_block: Option<BlockKey<'db>>,
        owner: BodyOwner<'db>,
    ) -> Self {
        Self {
            db,
            body,
            scopes,
            file,
            enclosing_block,
            table: InferenceTable::new(db),
            owner,
            diverges: Diverges::Maybe,
            breakables: Vec::new(),
            expressions: SideHandleMap::new(),
            local_bindings: SideHandleMap::new(),
            diagnostics: Vec::new(),
            deferred_negations: Vec::new(),
            expressions_with_mismatches: HashSet::new(),
        }
    }

    fn resolver_for(&self, expression_handle: ExpressionHandle) -> Option<Resolver<'a, 'db>> {
        let scope_view = self.scopes.containing_scope(expression_handle)?;
        Some(Resolver {
            file: self.file,
            scope_view,
            enclosing_block: self.enclosing_block,
        })
    }

    fn resolve_path(&self, expression_handle: ExpressionHandle) -> Option<Resolution<'db>> {
        let Expression::Path(path) = &self.body.expressions[expression_handle] else {
            return None;
        };
        // Paths of several segments only appear in imports for now.
        let [name] = path.segments(self.db).as_slice() else {
            return None;
        };
        self.resolver_for(expression_handle)?
            .resolve(self.db, *name)
    }

    /// Checks `expression_handle`, then coerces its type to the expected one, recording a mismatch if
    /// that fails. Returns the type after the coercion, i.e., the expected type on a mismatch,
    /// so that the surrounding code goes on as if the expression were correct.
    fn check_expression_coerce(
        &mut self,
        expression_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let ty = self.check_expression_inner(expression_handle, expectation);
        match expectation.only_has_type(&mut self.table) {
            Some(target) => match self.coerce(ty, target) {
                Ok(ty) => ty,
                Err(()) => {
                    self.emit_type_mismatch(expression_handle, target, ty, MismatchCause::Other);
                    target
                }
            },
            None => ty,
        }
    }

    fn check_expression_no_expectation(&mut self, expression_handle: ExpressionHandle) -> Ty<'db> {
        self.check_expression_inner(expression_handle, Expectation::None)
    }

    /// Computes `expression_handle`'s type, using the expectation only as a hint: a mismatch with it
    /// is left to the caller.
    fn check_expression_inner(
        &mut self,
        expression_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let body = self.body;
        let ty = match &body.expressions[expression_handle] {
            Expression::Unit => Ty::unit(self.db),
            Expression::Boolean(_) => Ty::bool(self.db),
            Expression::Integer(_) => self.check_integer_literal(expectation),
            Expression::Path(_) => self.check_path(expression_handle),
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
                statement_segment,
                tail_handle,
                ..
            } => self.check_block(
                expression_handle,
                &body.statement_children[*statement_segment],
                *tail_handle,
                expectation,
            ),
            Expression::Loop {
                source,
                body_handle,
            } => self.check_loop(*source, *body_handle, expectation),
            Expression::Call {
                callee_handle,
                argument_segment,
            } => self.check_call(
                expression_handle,
                *callee_handle,
                &body.expression_children[*argument_segment],
            ),
            Expression::Continue => {
                if self.breakables.is_empty() {
                    self.push_diagnostic(InferenceDiagnostic::BreakOutsideOfLoop {
                        expression_handle,
                        is_break: false,
                        bad_value_break: false,
                    });
                }
                Ty::bottom(self.db)
            }
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
            // A syntax error, already reported by the parser.
            Expression::Error => Ty::error(self.db),
        };

        self.expressions.add(expression_handle, ty);
        if self.is_bottom(ty) {
            self.diverges = Diverges::Always;
        }
        ty
    }

    /// Returns the expected type for an integer literal when it's already a concrete integer type,
    /// or a fresh integer variable otherwise.
    fn check_integer_literal(&mut self, expectation: Expectation<'db>) -> Ty<'db> {
        if let Some(expected) = expectation.only_has_type(&mut self.table)
            && let TyKind::Signed(_) | TyKind::Unsigned(_) = expected.kind(self.db)
        {
            return expected;
        }
        self.new_integer_variable()
    }

    /// Resolves the name, then takes the type of what it names.
    fn check_path(&mut self, expression_handle: ExpressionHandle) -> Ty<'db> {
        match self.resolve_path(expression_handle) {
            Some(Resolution::Local(binding_handle)) => self.local_bindings[binding_handle],
            Some(Resolution::Function(key)) => *function_signature_type_of(self.db, key),
            Some(Resolution::Constant(key)) => *constant_signature_type_of(self.db, key),
            None => {
                self.push_diagnostic(InferenceDiagnostic::UnresolvedName { expression_handle });
                Ty::error(self.db)
            }
        }
    }

    /// Checks an `if`, coercing both branches to one type. An `if` without `else` must have a `()`
    /// then-branch.
    fn check_if(
        &mut self,
        expression_handle: ExpressionHandle,
        condition_handle: ExpressionHandle,
        then_branch_handle: ExpressionHandle,
        else_branch_handle: Option<ExpressionHandle>,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let expectation = expectation.adjust_for_branches(self.db, &mut self.table);
        self.check_expression_coerce(condition_handle, Expectation::HasType(Ty::bool(self.db)));
        let condition_diverges = std::mem::replace(&mut self.diverges, Diverges::Maybe);

        let then_ty = self.check_expression_inner(then_branch_handle, expectation);
        let then_diverges = std::mem::replace(&mut self.diverges, Diverges::Maybe);

        let mut coercion =
            CoerceMany::new(expectation.coercion_target_type(self.db, &mut self.table));
        self.coerce_into_or_report(
            &mut coercion,
            then_branch_handle,
            then_ty,
            MismatchCause::Other,
        );

        match else_branch_handle {
            Some(else_branch_handle) => {
                let else_ty = self.check_expression_inner(else_branch_handle, expectation);
                let else_diverges = std::mem::replace(&mut self.diverges, Diverges::Maybe);
                if let Err((expected, found)) = self.coerce_into(&mut coercion, else_ty)
                    && self.expressions_with_mismatches.insert(else_branch_handle)
                {
                    let expected = self.render(expected);
                    let found = self.render(found);
                    self.push_diagnostic(InferenceDiagnostic::IfElseMismatch {
                        then_branch_handle,
                        else_branch_handle,
                        expected,
                        found,
                    });
                }
                self.diverges = condition_diverges.max(then_diverges.min(else_diverges));
            }
            None => {
                if let Err(expected) = self.coerce_forced_unit_into(&mut coercion) {
                    let expected = self.render(expected);
                    self.push_diagnostic(InferenceDiagnostic::IfWithoutElse {
                        expression_handle,
                        expected,
                    });
                }
                self.diverges = condition_diverges;
            }
        }

        coercion.complete(self.db)
    }

    /// Checks a block, whose value is its tail, or `()` if there's none, unless the block always
    /// diverges, in which case it's `Bottom`.
    fn check_block(
        &mut self,
        expression_handle: ExpressionHandle,
        statement_handles: &[StatementHandle],
        tail_handle: Option<ExpressionHandle>,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let body = self.body;
        let mut coercion =
            CoerceMany::new(expectation.coercion_target_type(self.db, &mut self.table));

        for statement_handle in statement_handles {
            match &body.statements[*statement_handle] {
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
                        self.check_expression_no_expectation(*expression_handle);
                    } else {
                        // e.g. an `if` without a semicolon, followed by more statements
                        self.check_expression_coerce(
                            *expression_handle,
                            Expectation::HasType(Ty::unit(self.db)),
                        );
                    }
                }
                // Checked as a body of its own.
                Statement::Definition => {}
            }
        }

        // The tail of a function's body is the function's result.
        let is_function_body = matches!(self.owner, BodyOwner::Function { .. })
            && expression_handle == self.body.root_handle;
        match tail_handle {
            Some(tail_handle) => {
                let tail_ty = self.check_expression_inner(tail_handle, expectation);
                let cause = if is_function_body {
                    MismatchCause::ReturnValue
                } else {
                    MismatchCause::Other
                };
                self.coerce_into_or_report(&mut coercion, tail_handle, tail_ty, cause);
            }
            None => {
                if self.diverges != Diverges::Always
                    && let Err(expected) = self.coerce_forced_unit_into(&mut coercion)
                {
                    let cause = if is_function_body {
                        MismatchCause::ImplicitUnitReturn
                    } else {
                        MismatchCause::Other
                    };
                    self.emit_type_mismatch(expression_handle, expected, Ty::unit(self.db), cause);
                }
            }
        }

        coercion.complete(self.db)
    }

    /// Checks a `let`: with an annotation, the initializer is coerced to it; without one, the
    /// binding takes the initializer's type.
    fn check_let(
        &mut self,
        binding_handle: LocalBindingHandle,
        annotation_handle: Option<TypeAnnotationHandle>,
        initializer_handle: Option<ExpressionHandle>,
    ) {
        let declared_ty = annotation_handle
            .map(|annotation_handle| self.lower_type_annotation(annotation_handle));
        let ty = match (declared_ty, initializer_handle) {
            (Some(declared_ty), Some(initializer_handle)) => {
                self.check_expression_coerce(
                    initializer_handle,
                    Expectation::has_type(self.db, declared_ty),
                );
                declared_ty
            }
            (None, Some(initializer_handle)) => {
                self.check_expression_no_expectation(initializer_handle)
            }
            (Some(declared_ty), None) => declared_ty,
            // A syntax error, already reported by the parser.
            (None, None) => Ty::error(self.db),
        };
        self.local_bindings.add(binding_handle, ty);
    }

    /// Checks a loop. A `loop`'s type comes from its `break`s (`Bottom` if there are none); a
    /// `while` loop's is always `()`.
    fn check_loop(
        &mut self,
        source: LoopSource,
        loop_body_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let coerce = match source {
            LoopSource::Loop => Some(CoerceMany::new(
                expectation.coercion_target_type(self.db, &mut self.table),
            )),
            LoopSource::While => None,
        };
        self.breakables.push(BreakableContext {
            may_break: false,
            coerce,
        });
        self.check_expression_coerce(loop_body_handle, Expectation::HasType(Ty::unit(self.db)));
        let context = self.breakables.pop().expect("pushed above");

        self.diverges = if context.may_break {
            Diverges::Maybe
        } else {
            Diverges::Always
        };
        match context.coerce {
            Some(coercion) => coercion.complete(self.db),
            None => Ty::unit(self.db),
        }
    }

    /// Checks a `break`, coercing its value to the innermost loop's type.
    fn check_break(
        &mut self,
        expression_handle: ExpressionHandle,
        value_handle: Option<ExpressionHandle>,
    ) -> Ty<'db> {
        let Some(context) = self.breakables.last() else {
            if let Some(value_handle) = value_handle {
                self.check_expression_no_expectation(value_handle);
            }
            self.push_diagnostic(InferenceDiagnostic::BreakOutsideOfLoop {
                expression_handle,
                is_break: true,
                bad_value_break: false,
            });
            return Ty::bottom(self.db);
        };
        let loop_ty = context.coerce.as_ref().map(|coercion| coercion.expected_ty);

        let value_ty = match value_handle {
            Some(value_handle) => {
                let expectation = match loop_ty {
                    Some(loop_ty) => Expectation::HasType(loop_ty),
                    None => {
                        self.push_diagnostic(InferenceDiagnostic::BreakOutsideOfLoop {
                            expression_handle,
                            is_break: true,
                            bad_value_break: true,
                        });
                        Expectation::None
                    }
                };
                Some((
                    value_handle,
                    self.check_expression_inner(value_handle, expectation),
                ))
            }
            None => None,
        };

        let diverges = self.diverges;
        let context = self.breakables.last_mut().expect("checked above");
        // A `break` whose value diverges itself (e.g. `break return 5`) never reaches the loop's end.
        context.may_break |= diverges != Diverges::Always;
        if let Some(mut coercion) = context.coerce.take() {
            match value_ty {
                Some((value_handle, value_ty)) => self.coerce_into_or_report(
                    &mut coercion,
                    value_handle,
                    value_ty,
                    MismatchCause::Other,
                ),
                None => {
                    if let Err(expected) = self.coerce_forced_unit_into(&mut coercion) {
                        let unit = Ty::unit(self.db);
                        self.emit_type_mismatch(
                            expression_handle,
                            expected,
                            unit,
                            MismatchCause::Other,
                        );
                    }
                }
            }
            self.breakables.last_mut().expect("checked above").coerce = Some(coercion);
        }

        Ty::bottom(self.db)
    }

    fn check_return(
        &mut self,
        expression_handle: ExpressionHandle,
        value_handle: Option<ExpressionHandle>,
    ) -> Ty<'db> {
        match (&self.owner, value_handle) {
            (BodyOwner::Function { .. }, Some(value_handle)) => {
                self.check_return_value(value_handle)
            }
            (BodyOwner::Function { .. }, None) => self.with_return_coercion(|this, coercion| {
                if this.coerce_forced_unit_into(coercion).is_err() {
                    this.push_diagnostic(InferenceDiagnostic::ReturnWithoutValue {
                        expression_handle,
                    });
                }
            }),
            (BodyOwner::Constant { .. }, value_handle) => {
                self.push_diagnostic(InferenceDiagnostic::ReturnOutsideFunction {
                    expression_handle,
                });
                if let Some(value_handle) = value_handle {
                    self.check_expression_no_expectation(value_handle);
                }
            }
        }
        Ty::bottom(self.db)
    }

    /// Checks a value returned from the function, i.e., a `return`'s value or the body itself.
    fn check_return_value(&mut self, value_handle: ExpressionHandle) {
        let BodyOwner::Function { return_coercion } = self.owner else {
            unreachable!("only a function's body returns values")
        };
        let return_type = return_coercion.expected_ty;
        let value_ty = self.check_expression_inner(value_handle, Expectation::HasType(return_type));
        self.with_return_coercion(|this, coercion| {
            this.coerce_into_or_report(
                coercion,
                value_handle,
                value_ty,
                MismatchCause::ReturnValue,
            );
        });
    }

    /// Runs `f` on the function's return coercion. `f` must not check expressions, since a
    /// `return` inside them would update the coercion too.
    fn with_return_coercion(&mut self, f: impl FnOnce(&mut Self, &mut CoerceMany<'db>)) {
        let BodyOwner::Function {
            return_coercion: mut coercion,
        } = self.owner
        else {
            unreachable!("only a function's body returns values")
        };
        f(self, &mut coercion);
        self.owner = BodyOwner::Function {
            return_coercion: coercion,
        };
    }

    /// Checks a call, coercing each argument to its parameter's type.
    fn check_call(
        &mut self,
        call_handle: ExpressionHandle,
        callee_handle: ExpressionHandle,
        argument_handles: &[ExpressionHandle],
    ) -> Ty<'db> {
        let callee_ty = self.check_expression_no_expectation(callee_handle);
        let callee_ty = self.table.shallow_resolve(callee_ty);

        let (parameters, return_ty) = match callee_ty.kind(self.db) {
            TyKind::Function {
                parameters,
                r#return,
            } => (parameters.clone(), *r#return),
            _ => {
                if !self.involves_error(callee_ty) {
                    let found = self.render(callee_ty);
                    self.push_diagnostic(InferenceDiagnostic::ExpectedFunction {
                        call_handle,
                        found,
                    });
                }
                for &argument_handle in argument_handles {
                    self.check_expression_no_expectation(argument_handle);
                }
                return Ty::error(self.db);
            }
        };

        if parameters.len() != argument_handles.len() {
            let function = match self.resolve_path(callee_handle) {
                Some(Resolution::Function(function)) => Some(function),
                _ => None,
            };
            let parameters = parameters.iter().map(|&ty| self.render(ty)).collect();
            self.push_diagnostic(InferenceDiagnostic::MismatchedArgumentCount {
                call_handle,
                function,
                parameters,
            });
        }
        for (index, &argument_handle) in argument_handles.iter().enumerate() {
            match parameters.get(index) {
                Some(&parameter) => {
                    self.check_expression_coerce(
                        argument_handle,
                        Expectation::has_type(self.db, parameter),
                    );
                }
                None => {
                    self.check_expression_no_expectation(argument_handle);
                }
            }
        }

        return_ty
    }

    /// Checks a unary operation: `-` needs an integer, and `not` needs a `Bool`.
    fn check_unary_operation(
        &mut self,
        expression_handle: ExpressionHandle,
        operator: UnaryOperator,
        operand_handle: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        match operator {
            // Like Rust, only signed integers can be negated.
            UnaryOperator::Neg => {
                let operand_ty = self.check_expression_inner(operand_handle, expectation);
                let resolved = self.table.shallow_resolve(operand_ty);
                match resolved.kind(self.db) {
                    TyKind::Signed(_) | TyKind::Bottom | TyKind::Error => operand_ty,
                    TyKind::InferenceVariable(_) if self.require_integer(resolved) => {
                        self.deferred_negations
                            .push((expression_handle, operand_ty));
                        operand_ty
                    }
                    kind => {
                        let is_unsigned = matches!(kind, TyKind::Unsigned(_));
                        let found = self.render(resolved);
                        self.push_diagnostic(InferenceDiagnostic::UnaryOperatorCannotBeApplied {
                            expression_handle,
                            operator,
                            found,
                            is_unsigned,
                        });
                        Ty::error(self.db)
                    }
                }
            }
            UnaryOperator::Not => {
                self.check_expression_coerce(
                    operand_handle,
                    Expectation::HasType(Ty::bool(self.db)),
                );
                Ty::bool(self.db)
            }
        }
    }

    /// Checks a binary operation: `&&` and `||` need two `Bool`s, and every other operator needs
    /// both sides to have the same type, which is an integer for arithmetic.
    fn check_binary_operation(
        &mut self,
        expression_handle: ExpressionHandle,
        operator: Option<BinaryOperator>,
        lhs_handle: ExpressionHandle,
        rhs_handle: ExpressionHandle,
    ) -> Ty<'db> {
        let Some(operator) = operator else {
            // A syntax error, already reported by the parser.
            self.check_expression_no_expectation(lhs_handle);
            self.check_expression_no_expectation(rhs_handle);
            return Ty::error(self.db);
        };

        match operator {
            BinaryOperator::And | BinaryOperator::Or => {
                let bool = Ty::bool(self.db);
                self.check_expression_coerce(lhs_handle, Expectation::HasType(bool));
                let lhs_diverges = self.diverges;
                self.check_expression_coerce(rhs_handle, Expectation::HasType(bool));
                // Depending on the lhs's value, the rhs may never run.
                self.diverges = lhs_diverges;
                bool
            }
            BinaryOperator::Add
            | BinaryOperator::Sub
            | BinaryOperator::Mul
            | BinaryOperator::Div => {
                let lhs_ty = self.check_binary_operands(lhs_handle, rhs_handle);
                if self.require_integer(lhs_ty) {
                    lhs_ty
                } else {
                    let rhs_ty = self.expressions[rhs_handle];
                    if !self.involves_error(lhs_ty) && !self.involves_error(rhs_ty) {
                        let lhs = self.render(lhs_ty);
                        let rhs = self.render(rhs_ty);
                        self.push_diagnostic(InferenceDiagnostic::BinaryOperatorCannotBeApplied {
                            expression_handle,
                            operator,
                            lhs,
                            rhs,
                        });
                    }
                    Ty::error(self.db)
                }
            }
            BinaryOperator::Lt
            | BinaryOperator::Gt
            | BinaryOperator::Le
            | BinaryOperator::Ge
            | BinaryOperator::Eq
            | BinaryOperator::Ne => {
                self.check_binary_operands(lhs_handle, rhs_handle);
                Ty::bool(self.db)
            }
        }
    }

    /// Checks the lhs on its own, then coerces the rhs to the lhs's type. Returns the lhs's type.
    fn check_binary_operands(
        &mut self,
        lhs_handle: ExpressionHandle,
        rhs_handle: ExpressionHandle,
    ) -> Ty<'db> {
        let lhs_ty = self.check_expression_no_expectation(lhs_handle);
        let rhs_expectation = if self.is_bottom(lhs_ty) {
            Expectation::None
        } else {
            Expectation::has_type(self.db, lhs_ty)
        };
        self.check_expression_coerce(rhs_handle, rhs_expectation);
        lhs_ty
    }

    fn check_assignment(
        &mut self,
        assignment_handle: ExpressionHandle,
        target_handle: ExpressionHandle,
        value_handle: ExpressionHandle,
    ) -> Ty<'db> {
        let target_ty = self.check_expression_no_expectation(target_handle);
        self.check_expression_coerce(value_handle, Expectation::has_type(self.db, target_ty));

        let is_place = match &self.body.expressions[target_handle] {
            // An unresolved name was already reported.
            Expression::Path(_) => !matches!(
                self.resolve_path(target_handle),
                Some(Resolution::Function(_) | Resolution::Constant(_))
            ),
            Expression::Error => true,
            _ => false,
        };
        if !is_place {
            self.push_diagnostic(InferenceDiagnostic::InvalidAssignmentTarget {
                assignment_handle,
                target_handle,
            });
        }

        Ty::unit(self.db)
    }

    /// Turns a type annotation written inside the body into a type, reporting an unknown name.
    fn lower_type_annotation(&mut self, annotation_handle: TypeAnnotationHandle) -> Ty<'db> {
        match &self.body.type_annotations[annotation_handle] {
            TypeAnnotation::Path(name) => Ty::to_primitive(self.db, name.text(self.db))
                .unwrap_or_else(|| {
                    self.push_diagnostic(InferenceDiagnostic::UnknownType { annotation_handle });
                    Ty::error(self.db)
                }),
            // A syntax error, already reported by the parser.
            TypeAnnotation::Error => Ty::error(self.db),
        }
    }

    /// Coerces `from` to `to`, where the only coercion is from `Bottom` to any type. Returns the
    /// type after the coercion.
    fn coerce(&mut self, from: Ty<'db>, to: Ty<'db>) -> Result<Ty<'db>, ()> {
        if self.is_bottom(from) {
            self.table.set_coerced_from_bottom(to);
            return Ok(to);
        }
        self.table.unify(to, from).map(|()| to).map_err(drop)
    }

    /// Coerces `ty` to the merged type, returning the expected and found types on a mismatch.
    fn coerce_into(
        &mut self,
        coercion: &mut CoerceMany<'db>,
        ty: Ty<'db>,
    ) -> Result<(), (Ty<'db>, Ty<'db>)> {
        let merged_ty = coercion.merged_ty();
        match self.coerce(ty, merged_ty) {
            Ok(merged_ty) => {
                coercion.final_ty = Some(merged_ty);
                Ok(())
            }
            Err(()) => {
                coercion.final_ty = Some(Ty::error(self.db));
                Err((merged_ty, ty))
            }
        }
    }

    fn coerce_into_or_report(
        &mut self,
        coercion: &mut CoerceMany<'db>,
        expression_handle: ExpressionHandle,
        ty: Ty<'db>,
        cause: MismatchCause,
    ) {
        if let Err((expected, found)) = self.coerce_into(coercion, ty) {
            self.emit_type_mismatch(expression_handle, expected, found, cause);
        }
    }

    /// Coerces an implicit `()`, e.g. of an `if` without `else`, or a `break` without a value,
    /// returning the type that was expected instead on a mismatch.
    fn coerce_forced_unit_into(&mut self, coercion: &mut CoerceMany<'db>) -> Result<(), Ty<'db>> {
        let merged_ty = coercion.merged_ty();
        match self.table.unify(merged_ty, Ty::unit(self.db)) {
            Ok(()) => {
                coercion.final_ty = Some(merged_ty);
                Ok(())
            }
            Err(_) => {
                coercion.final_ty = Some(Ty::error(self.db));
                Err(merged_ty)
            }
        }
    }

    /// Records a mismatch, at most once per expression.
    fn emit_type_mismatch(
        &mut self,
        expression_handle: ExpressionHandle,
        expected: Ty<'db>,
        found: Ty<'db>,
        cause: MismatchCause,
    ) {
        if self.expressions_with_mismatches.insert(expression_handle) {
            let expected = self.render(expected);
            let found = self.render(found);
            self.push_diagnostic(InferenceDiagnostic::TypeMismatch {
                expression_handle,
                expected,
                found,
                cause,
            });
        }
    }

    fn push_diagnostic(&mut self, diagnostic: InferenceDiagnostic<'db>) {
        self.diagnostics.push(diagnostic);
    }

    /// Renders `ty` for a diagnostic, as it's known right now (see `InferenceDiagnostic`).
    fn render(&mut self, ty: Ty<'db>) -> String {
        self.table.deep_resolve(ty).display(self.db)
    }

    /// Returns whether `ty`, as it's known right now, involves the error type. A problem with such
    /// a type isn't reported: the error was already reported where the type came from.
    fn involves_error(&mut self, ty: Ty<'db>) -> bool {
        self.table.deep_resolve(ty).is_poisoned(self.db)
    }

    /// Requires `ty` to be an integer type, returning whether it can be. This isn't only a check:
    /// a type that's still unknown (a general variable) is constrained to be an integer, by
    /// unifying it with a fresh integer variable.
    fn require_integer(&mut self, ty: Ty<'db>) -> bool {
        let ty = self.table.shallow_resolve(ty);
        match ty.kind(self.db) {
            TyKind::Signed(_)
            | TyKind::Unsigned(_)
            | TyKind::InferenceVariable(InferenceVariable::Integer(_))
            | TyKind::Bottom
            | TyKind::Error => true,
            TyKind::InferenceVariable(InferenceVariable::General(_)) => {
                let integer = self.new_integer_variable();
                self.table.unify(ty, integer).is_ok()
            }
            TyKind::Unit | TyKind::Bool | TyKind::Function { .. } => false,
        }
    }

    fn is_bottom(&mut self, ty: Ty<'db>) -> bool {
        matches!(self.table.shallow_resolve(ty).kind(self.db), TyKind::Bottom)
    }

    fn new_integer_variable(&mut self) -> Ty<'db> {
        let variable_handle = self.table.make_integer_inference_variable();
        Ty::new(
            self.db,
            TyKind::InferenceVariable(InferenceVariable::Integer(variable_handle)),
        )
    }
}

impl<'db> TypeCheckSink<'db> {
    pub(crate) fn expression_type(&self, expression_handle: ExpressionHandle) -> Ty<'db> {
        self.expressions[expression_handle]
    }

    pub(crate) fn local_binding_type(&self, binding_handle: LocalBindingHandle) -> Ty<'db> {
        self.local_bindings[binding_handle]
    }

    pub(crate) fn expressions(&self) -> impl Iterator<Item = (ExpressionHandle, Ty<'db>)> + '_ {
        self.expressions
            .iter()
            .map(|(expression_handle, ty)| (expression_handle, *ty))
    }

    pub(crate) fn local_bindings(
        &self,
    ) -> impl Iterator<Item = (LocalBindingHandle, Ty<'db>)> + '_ {
        self.local_bindings
            .iter()
            .map(|(binding_handle, ty)| (binding_handle, *ty))
    }

    pub(crate) fn diagnostics(&self) -> &[InferenceDiagnostic<'db>] {
        &self.diagnostics
    }
}

impl<'db> Expectation<'db> {
    /// Creates an expectation of `ty`, unless it's the error type, which carries no information.
    fn has_type(db: &'db dyn crate::Db, ty: Ty<'db>) -> Self {
        match ty.kind(db) {
            TyKind::Error => Self::None,
            _ => Self::HasType(ty),
        }
    }

    fn only_has_type(self, table: &mut InferenceTable<'db>) -> Option<Ty<'db>> {
        match self {
            Self::HasType(ty) => Some(table.shallow_resolve(ty)),
            Self::None => None,
        }
    }

    /// Returns the type a `CoerceMany` should start from: the expected type, or a fresh variable if
    /// there's none.
    fn coercion_target_type(
        self,
        db: &'db dyn crate::Db,
        table: &mut InferenceTable<'db>,
    ) -> Ty<'db> {
        self.only_has_type(table).unwrap_or_else(|| {
            let variable_handle = table.make_general_inference_variable();
            Ty::new(
                db,
                TyKind::InferenceVariable(InferenceVariable::General(variable_handle)),
            )
        })
    }

    /// Branches are only hinted with an expected type that's known; an unsolved variable says
    /// nothing yet.
    fn adjust_for_branches(self, db: &'db dyn crate::Db, table: &mut InferenceTable<'db>) -> Self {
        match self.only_has_type(table) {
            Some(ty) if !matches!(ty.kind(db), TyKind::InferenceVariable(_)) => Self::HasType(ty),
            _ => Self::None,
        }
    }
}

impl<'db> CoerceMany<'db> {
    fn new(expected_ty: Ty<'db>) -> Self {
        Self {
            expected_ty,
            final_ty: None,
        }
    }

    fn merged_ty(&self) -> Ty<'db> {
        self.final_ty.unwrap_or(self.expected_ty)
    }

    fn complete(self, db: &'db dyn crate::Db) -> Ty<'db> {
        self.final_ty.unwrap_or_else(|| Ty::bottom(db))
    }
}
