use crate::common::context::CompilerContext;
use crate::common::span::Span;
use crate::common::types::{InferTy, Ty, TypeId};
use crate::diagnostics::semantic_diagnostics::SemanticDiagnostic;
use crate::front_end::semantic_analysis::constraints::{Constraint, Provenance};
use crate::front_end::semantic_analysis::hir::{
    BindingId, BindingKind, DefinitionId, DefinitionKind, ExpressionId, ExpressionIdSpan,
    ExpressionKind, Hir, LocalBindingId, LoopSource, StatementId, StatementKind,
};
use crate::front_end::semantic_analysis::symbol_table::{
    DefineError, LookupError, ScopeKind, SymbolTable,
};
use crate::front_end::semantic_analysis::unification_table::UnificationTable;
use crate::front_end::syntactic_analysis::cst::ast::{
    Assignment, AstNode, BinaryOperation, Block, Break, Call, ConstantDefinition, Continue,
    Definition, Expression, FunctionDefinition, IfExpression, InfiniteLoop, IntegerLiteral,
    Parameter, Return, SourceFile, Statement, Type, UnaryOperation, Variable, WhileLoop,
};
use crate::front_end::syntactic_analysis::cst::green::GreenNode;
use crate::front_end::syntactic_analysis::cst::operators::{BinOp, UnOp};
use crate::front_end::syntactic_analysis::cst::typed::RedNode;

pub(crate) struct SemanticAnalyzer<'ast> {
    tree: SourceFile<'ast>,
    ctx: &'ast mut CompilerContext,
    symbol_table: SymbolTable,
    hir: Hir,
    constraints: Vec<Constraint>,
    substitutions: UnificationTable,
    current_return_ty: Option<TypeId>,
    loop_frames: Vec<LoopFrame>,
    diagnostics: Vec<SemanticDiagnostic>,
}

struct LoopFrame {
    source: LoopSource,
    result_ty: TypeId,
    has_break: bool,
}

enum UnificationError {
    TypeMismatch {
        expected_id: TypeId,
        actual_id: TypeId,
    },
}

impl<'ast> SemanticAnalyzer<'ast> {
    pub(crate) fn new(cst: &'ast GreenNode, ctx: &'ast mut CompilerContext) -> Self {
        Self {
            hir: Hir::new(u32::from(cst.width) as usize),
            tree: SourceFile::cast(RedNode::new(cst, 0)).unwrap(),
            ctx,
            symbol_table: SymbolTable::new(),
            substitutions: UnificationTable::new(),
            constraints: Vec::new(),
            current_return_ty: None,
            loop_frames: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn analyze(mut self) -> (Hir, Vec<SemanticDiagnostic>) {
        self.symbol_table.enter_scope(ScopeKind::Normal);
        self.collect_top_level_definitions();
        self.typecheck_source_file();
        self.symbol_table.exit_scope();

        self.solve_constraints();

        self.substitute();

        (self.hir, self.diagnostics)
    }

    fn collect_top_level_definitions(&mut self) {
        let file = self.tree;
        for def in file.definitions() {
            self.collect_definition(def);
        }
    }

    fn typecheck_source_file(&mut self) {
        let file = self.tree;
        let mut root_definition_ids: Vec<DefinitionId> = Vec::new();

        for def in file.definitions() {
            let definition_id = match def {
                Definition::FunctionDefinition(def) => self.typecheck_function_definition(def),
                Definition::ConstantDefinition(def) => self.typecheck_constant_definition(def),
            };
            root_definition_ids.push(definition_id);
        }

        self.hir.source_file.definition_id_span = self.hir.add_definition_ids(&root_definition_ids);
    }

    fn solve_constraints(&mut self) {
        for constraint in std::mem::take(&mut self.constraints) {
            let Constraint::Equality {
                expected_id,
                actual_id,
                provenance,
            } = constraint;
            if let Err(UnificationError::TypeMismatch {
                expected_id: e,
                actual_id: a,
            }) = self.unify(expected_id, actual_id)
            {
                let e_resolved = self.shallow_resolve(e);
                let a_resolved = self.shallow_resolve(a);
                let e_str = self.ctx.type_interner.to_string(e_resolved);
                let a_str = self.ctx.type_interner.to_string(a_resolved);
                let diagnostic = match provenance {
                    Provenance::TypeMismatch { span } => SemanticDiagnostic::TypeMismatch {
                        expected: e_str,
                        found: a_str,
                        span,
                    },
                    Provenance::IfBranchMismatch {
                        then_span,
                        else_span,
                    } => SemanticDiagnostic::IfBranchMismatch {
                        then_ty: e_str,
                        else_ty: a_str,
                        then_span,
                        else_span,
                    },
                    Provenance::IfWithoutElse { then_span } => SemanticDiagnostic::IfWithoutElse {
                        found: e_str,
                        then_span,
                    },
                    Provenance::BinaryOperandMismatch { lhs_span, rhs_span } => {
                        SemanticDiagnostic::BinaryOperandMismatch {
                            lhs_ty: e_str,
                            rhs_ty: a_str,
                            lhs_span,
                            rhs_span,
                        }
                    }
                    Provenance::BinaryOperandNotNumeric { operand_span } => {
                        SemanticDiagnostic::BinaryOperandNotNumeric {
                            found: a_str,
                            operand_span,
                        }
                    }
                    Provenance::BinaryOperandNotBool { operand_span } => {
                        SemanticDiagnostic::BinaryOperandNotBool {
                            expected: e_str,
                            found: a_str,
                            operand_span,
                        }
                    }
                    Provenance::UnaryOperandMismatch {
                        operator,
                        operand_span,
                    } => SemanticDiagnostic::UnaryOperandMismatch {
                        operator,
                        expected: e_str,
                        found: a_str,
                        operand_span,
                    },
                    Provenance::BlockMissingTail { block_span } => {
                        SemanticDiagnostic::BlockMissingTail {
                            expected: e_str,
                            block_span,
                        }
                    }
                    Provenance::ReturnMissingValue { return_span } => {
                        SemanticDiagnostic::ReturnMissingValue {
                            expected: e_str,
                            return_span,
                        }
                    }
                    Provenance::LoopBodyNotUnit { source, body_span } => {
                        SemanticDiagnostic::LoopBodyNotUnit {
                            source,
                            found: e_str,
                            body_span,
                        }
                    }
                };
                self.diagnostics.push(diagnostic);
            }
        }
    }

    fn substitute(&mut self) {
        // unresolved IntVar defaults to i32, unresolved TyVar becomes error
        let tys: Vec<_> = self
            .hir
            .expressions
            .values()
            .map(|expression| expression.ty)
            .collect();
        let tys: Vec<_> = tys.iter().map(|&ty| self.shallow_resolve(ty)).collect();
        for (expression, ty) in self.hir.expressions.values_mut().zip(tys) {
            expression.ty = match self.ctx.type_interner.resolve(ty).unwrap() {
                Ty::Infer(InferTy::IntVar(_)) => self.ctx.type_interner.i32_id,
                Ty::Infer(InferTy::TyVar(_)) => self.ctx.type_interner.error_id,
                _ => ty,
            };
        }

        // same for local bindings
        let tys: Vec<_> = self.hir.local_bindings.values().map(|b| b.ty).collect();
        let tys: Vec<_> = tys.iter().map(|&ty| self.shallow_resolve(ty)).collect();
        for (binding, ty) in self.hir.local_bindings.values_mut().zip(tys) {
            binding.ty = match self.ctx.type_interner.resolve(ty).unwrap() {
                Ty::Infer(InferTy::IntVar(_)) => self.ctx.type_interner.i32_id,
                Ty::Infer(InferTy::TyVar(_)) => self.ctx.type_interner.error_id,
                _ => ty,
            };
        }
    }

    fn block_parts(block: Block<'ast>) -> (Vec<Statement<'ast>>, Option<Expression<'ast>>) {
        let mut statements: Vec<Statement<'ast>> = block.statements().collect();
        let tail = match statements.last() {
            Some(Statement::Expression(last)) if !last.has_semicolon() => {
                let Some(Statement::Expression(last)) = statements.pop() else {
                    unreachable!()
                };
                last.expr()
            }
            _ => None,
        };
        (statements, tail)
    }

    fn collect_definition(&mut self, def: Definition<'ast>) {
        match def {
            Definition::FunctionDefinition(def) => {
                let params: Vec<Parameter<'ast>> = def
                    .parameter_list()
                    .into_iter()
                    .flat_map(|list| list.parameters().collect::<Vec<_>>())
                    .collect();
                let parameters_ty: Vec<TypeId> = params
                    .into_iter()
                    .map(|param| {
                        let annotation = param
                            .ty()
                            .expect("parser guarantees a Type on every well-formed Parameter");
                        self.resolve_type_annotation(annotation)
                    })
                    .collect();

                let return_ty = def
                    .return_type()
                    .map_or(self.ctx.type_interner.unit_id, |annotation| {
                        self.resolve_type_annotation(annotation)
                    });

                let name = self.ctx.string_interner.intern(
                    def.name()
                        .expect("parser guarantees a name on every well-formed FunctionDefinition")
                        .text(),
                );
                let span = def.span();

                let definition_binding_id = self.hir.add_definition_binding(
                    name,
                    self.ctx.type_interner.intern(Ty::Function {
                        parameter_type_ids: parameters_ty,
                        return_type_id: return_ty,
                    }),
                    span,
                );
                if let Err(DefineError::AlreadyDefined { previous_binding }) = self
                    .symbol_table
                    .add_binding(name, definition_binding_id.into())
                {
                    self.diagnostics
                        .push(SemanticDiagnostic::DuplicateDefinition {
                            name: self.ctx.string_interner.resolve(name).unwrap(),
                            span,
                            previous_span: self
                                .hir
                                .get_definition_binding(previous_binding.as_definition().unwrap())
                                .span(),
                        });
                }
            }
            Definition::ConstantDefinition(def) => {
                let annotation = def
                    .ty()
                    .expect("parser guarantees a Type on every well-formed ConstantDefinition");
                let ty = self.resolve_type_annotation(annotation);

                let name = self.ctx.string_interner.intern(
                    def.name()
                        .expect("parser guarantees a name on every well-formed ConstantDefinition")
                        .text(),
                );
                let span = def.span();
                let definition_binding_id = self.hir.add_definition_binding(name, ty, span);
                if let Err(DefineError::AlreadyDefined { previous_binding }) = self
                    .symbol_table
                    .add_binding(name, definition_binding_id.into())
                {
                    self.diagnostics
                        .push(SemanticDiagnostic::DuplicateDefinition {
                            name: self.ctx.string_interner.resolve(name).unwrap(),
                            span,
                            previous_span: self
                                .hir
                                .get_definition_binding(previous_binding.as_definition().unwrap())
                                .span(),
                        });
                }
            }
        }
    }

    fn typecheck_function_definition(&mut self, def: FunctionDefinition<'ast>) -> DefinitionId {
        let name = self.ctx.string_interner.intern(
            def.name()
                .expect("parser guarantees a name on every well-formed FunctionDefinition")
                .text(),
        );
        let binding_id = self.symbol_table.find_binding(name).unwrap();
        let func_ty = self
            .hir
            .get_definition_binding(binding_id.as_definition().unwrap())
            .ty();
        let (parameter_tys, return_ty) = self
            .ctx
            .type_interner
            .as_func(func_ty)
            .map(|(params, ret)| (params.to_vec(), ret))
            .unwrap();

        self.symbol_table.enter_scope(ScopeKind::FunctionBoundary);

        let mut param_local_binding_ids: Vec<LocalBindingId> = Vec::new();
        let params: Vec<Parameter<'ast>> = def
            .parameter_list()
            .into_iter()
            .flat_map(|list| list.parameters().collect::<Vec<_>>())
            .collect();
        for (i, param) in params.into_iter().enumerate() {
            let name = self.ctx.string_interner.intern(
                param
                    .name()
                    .expect("parser guarantees a name on every well-formed Parameter")
                    .text(),
            );
            let mutable = param.is_mut();
            let span = param.span();
            let local_binding_id = self.hir.add_local_binding(
                name,
                mutable,
                Some(parameter_tys[i]),
                parameter_tys[i],
                span,
            );
            if let Err(DefineError::AlreadyDefined { previous_binding }) =
                self.symbol_table.add_binding(name, local_binding_id.into())
            {
                self.diagnostics
                    .push(SemanticDiagnostic::DuplicateDefinition {
                        name: self.ctx.string_interner.resolve(name).unwrap(),
                        span,
                        previous_span: self
                            .hir
                            .get_local_binding(previous_binding.as_local().unwrap())
                            .span(),
                    });
            }

            param_local_binding_ids.push(local_binding_id);
        }
        let parameter_id_span = self.hir.add_parameter_ids(&param_local_binding_ids);

        let temp_return_ty = self.current_return_ty;
        let temp_loop_frames = std::mem::take(&mut self.loop_frames);
        self.current_return_ty = Some(return_ty);

        // A missing `{` leaves no `Block` child at all (see `parse_function_definition`).
        let body_id = match def.body() {
            Some(body) => self.analyze_block(body, Some(return_ty)),
            None => self.hir.add_expression(
                ExpressionKind::Missing,
                self.ctx.type_interner.error_id,
                def.span(),
            ),
        };

        self.current_return_ty = temp_return_ty;
        self.loop_frames = temp_loop_frames;

        self.symbol_table.exit_scope();

        self.hir.add_definition(
            DefinitionKind::Function {
                definition_binding_id: binding_id.as_definition().unwrap(),
                parameter_id_span,
                body_id,
            },
            def.span(),
        )
    }

    fn typecheck_constant_definition(&mut self, def: ConstantDefinition<'ast>) -> DefinitionId {
        let name = self.ctx.string_interner.intern(
            def.name()
                .expect("parser guarantees a name on every well-formed ConstantDefinition")
                .text(),
        );
        let binding_id = self.symbol_table.find_binding(name).unwrap();

        self.symbol_table.enter_scope(ScopeKind::ConstantBoundary);
        let initializer_id = self.expect_expr_checked(
            def.value(),
            self.hir
                .get_definition_binding(binding_id.as_definition().unwrap())
                .ty(),
            def.span(),
        );
        self.symbol_table.exit_scope();

        self.hir.add_definition(
            DefinitionKind::Constant {
                definition_binding_id: binding_id.as_definition().unwrap(),
                initializer_id,
            },
            def.span(),
        )
    }

    fn analyze_block(&mut self, block: Block<'ast>, expected_id: Option<TypeId>) -> ExpressionId {
        self.symbol_table.enter_scope(ScopeKind::Normal);
        self.collect_block_statements(block);
        let expression_id = self.typecheck_block(block, expected_id);
        self.symbol_table.exit_scope();
        expression_id
    }

    fn collect_block_statements(&mut self, block: Block<'ast>) {
        let stmts: Vec<Statement<'ast>> = block.statements().collect();
        for stmt in stmts {
            if let Statement::Definition(stmt) = stmt {
                let def = stmt.definition().expect(
                    "parser guarantees a Definition on every well-formed DefinitionStatement",
                );
                self.collect_definition(def);
            }
        }
    }

    fn typecheck_block(&mut self, block: Block<'ast>, expected_id: Option<TypeId>) -> ExpressionId {
        let (stmt_nodes, tail_node) = Self::block_parts(block);

        let mut statement_ids: Vec<StatementId> = Vec::new();
        for stmt in stmt_nodes {
            statement_ids.push(self.typecheck_statement(stmt));
        }
        let statement_id_span = self.hir.add_statement_ids(&statement_ids);

        let (tail_id, ty) = match (tail_node, expected_id) {
            (Some(tail), Some(expected)) => {
                let expression_id = self.check(tail, expected);
                (Some(expression_id), expected)
            }
            (Some(tail), None) => {
                let expression_id = self.infer(tail);
                (
                    Some(expression_id),
                    self.hir.get_expression(expression_id).ty(),
                )
            }
            (None, _) if self.last_statement_diverges(&statement_ids) => {
                (None, self.ctx.type_interner.bottom_id)
            }
            (None, Some(expected)) => {
                self.constrain(Constraint::Equality {
                    expected_id: expected,
                    actual_id: self.ctx.type_interner.unit_id,
                    provenance: Provenance::BlockMissingTail {
                        block_span: block.span(),
                    },
                });
                (None, self.ctx.type_interner.unit_id)
            }
            (None, None) => (None, self.ctx.type_interner.unit_id),
        };

        self.hir.add_expression(
            ExpressionKind::Block {
                statement_id_span,
                tail_id,
            },
            ty,
            block.span(),
        )
    }

    fn last_statement_diverges(&self, statement_ids: &[StatementId]) -> bool {
        let Some(&last_statement_id) = statement_ids.last() else {
            return false;
        };
        let expression_id = match *self.hir.get_statement(last_statement_id).kind() {
            StatementKind::Expression { expression_id, .. } => expression_id,
            StatementKind::Let {
                value_id: Some(value_id),
                ..
            } => value_id,
            StatementKind::Let { value_id: None, .. } => return false,
            StatementKind::Definition { .. } => return false,
        };
        self.hir.get_expression(expression_id).ty() == self.ctx.type_interner.bottom_id
    }

    fn typecheck_statement(&mut self, stmt: Statement<'ast>) -> StatementId {
        match stmt {
            Statement::Expression(stmt) => {
                let expr = stmt.expr().expect(
                    "parser guarantees an expression on every well-formed ExpressionStatement",
                );
                let expression_id = self.infer(expr);
                let has_semicolon = stmt.has_semicolon();
                self.hir.add_statement(
                    StatementKind::Expression {
                        expression_id,
                        has_semicolon,
                    },
                    stmt.span(),
                )
            }
            Statement::Definition(stmt) => {
                let def = stmt.definition().expect(
                    "parser guarantees a Definition on every well-formed DefinitionStatement",
                );
                let definition_id = match def {
                    Definition::FunctionDefinition(def) => self.typecheck_function_definition(def),
                    Definition::ConstantDefinition(def) => self.typecheck_constant_definition(def),
                };
                self.hir
                    .add_statement(StatementKind::Definition { definition_id }, stmt.span())
            }
            Statement::Let(stmt) => {
                let annotated_ty = stmt
                    .ty()
                    .map(|annotation| self.resolve_type_annotation(annotation));

                let value = stmt.value();
                let (value_id, ty) = match (value, annotated_ty) {
                    (Some(value), Some(expected)) => (Some(self.check(value, expected)), expected),
                    (Some(value), None) => {
                        let value_id = self.infer(value);
                        let ty = self.hir.get_expression(value_id).ty();
                        (Some(value_id), ty)
                    }
                    (None, Some(expected)) => (None, expected),
                    (None, None) => {
                        self.diagnostics
                            .push(SemanticDiagnostic::LetMissingTypeOrValue { span: stmt.span() });
                        (None, self.ctx.type_interner.error_id)
                    }
                };

                let name = self.ctx.string_interner.intern(
                    stmt.name()
                        .expect("parser guarantees a name on every well-formed LetStatement")
                        .text(),
                );
                let mutable = stmt.is_mut();
                let local_binding_id =
                    self.hir
                        .add_local_binding(name, mutable, annotated_ty, ty, stmt.span());
                if let Err(DefineError::AlreadyDefined { previous_binding }) =
                    self.symbol_table.add_binding(name, local_binding_id.into())
                {
                    self.diagnostics
                        .push(SemanticDiagnostic::DuplicateDefinition {
                            name: self.ctx.string_interner.resolve(name).unwrap(),
                            span: stmt.span(),
                            previous_span: self
                                .hir
                                .get_local_binding(previous_binding.as_local().unwrap())
                                .span(),
                        });
                }

                self.hir.add_statement(
                    StatementKind::Let {
                        pattern_id: local_binding_id,
                        value_id,
                    },
                    stmt.span(),
                )
            }
        }
    }

    fn integer_literal_value(&mut self, expr: IntegerLiteral<'ast>) -> Option<u128> {
        let token = expr.token();
        let raw = token.text();
        let cleaned: String = raw.chars().filter(|&c| c != '_').collect();
        #[allow(clippy::option_if_let_else)]
        let (digits, base) = if let Some(rest) = cleaned.strip_prefix("0x") {
            (rest, 16)
        } else if let Some(rest) = cleaned.strip_prefix("0b") {
            (rest, 2)
        } else if let Some(rest) = cleaned.strip_prefix("0o") {
            (rest, 8)
        } else {
            (cleaned.as_str(), 10)
        };
        match u128::from_str_radix(digits, base) {
            Ok(value) => Some(value),
            Err(_) => {
                self.diagnostics
                    .push(SemanticDiagnostic::InvalidIntegerLiteral {
                        found: raw.to_string(),
                        span: expr.span(),
                    });
                None
            }
        }
    }

    fn check(&mut self, expr: Expression<'ast>, ty: TypeId) -> ExpressionId {
        match (expr, self.ctx.type_interner.resolve(ty).unwrap()) {
            (Expression::IntegerLiteral(int), Ty::Signed(_) | Ty::Unsigned(_)) => {
                match self.integer_literal_value(int) {
                    Some(value) => {
                        self.hir
                            .add_expression(ExpressionKind::Integer(value), ty, int.span())
                    }
                    None => self.hir.add_expression(
                        ExpressionKind::Integer(0),
                        self.ctx.type_interner.error_id,
                        int.span(),
                    ),
                }
            }
            (Expression::UnitLiteral(unit), Ty::Unit) => {
                self.hir
                    .add_expression(ExpressionKind::Unit, ty, unit.span())
            }
            // grouping parens are purely syntactic -- check straight through
            (Expression::ParenthesizedExpression(paren), _) => {
                self.expect_expr_checked(paren.inner(), ty, paren.span())
            }
            (Expression::BinaryOperation(bin), _) => {
                let operator = BinOp::from_token_kind(bin.operator_token().kind);
                match operator {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        let lhs_id = self.expect_expr_checked(bin.lhs(), ty, bin.span());
                        let rhs_id = self.expect_expr_checked(bin.rhs(), ty, bin.span());

                        let int_ty = self.fresh_int_var();
                        self.constrain(Constraint::Equality {
                            expected_id: int_ty,
                            actual_id: ty,
                            provenance: Provenance::BinaryOperandNotNumeric {
                                operand_span: self.hir.get_expression(lhs_id).span(),
                            },
                        });

                        self.hir.add_expression(
                            ExpressionKind::Binary {
                                operator,
                                lhs_id,
                                rhs_id,
                            },
                            ty,
                            bin.span(),
                        )
                    }
                    _ => {
                        let expression_id = self.infer(Expression::BinaryOperation(bin));
                        let expression_view = self.hir.get_expression(expression_id);
                        self.constrain(Constraint::Equality {
                            expected_id: ty,
                            actual_id: expression_view.ty(),
                            provenance: Provenance::TypeMismatch {
                                span: expression_view.span(),
                            },
                        });
                        expression_id
                    }
                }
            }
            _ => {
                let expression_id = self.infer(expr);
                let expression_view = self.hir.get_expression(expression_id);
                self.constrain(Constraint::Equality {
                    expected_id: ty,
                    actual_id: expression_view.ty(),
                    provenance: Provenance::TypeMismatch {
                        span: expression_view.span(),
                    },
                });
                expression_id
            }
        }
    }

    /// Infers an expression position that the grammar can legitimately leave
    /// empty under recovery -- `nud()`'s fallback records a diagnostic and
    /// produces no node at all, rather than a `NodeKind::Error` placeholder,
    /// so the various `Option<Expression>` accessors can genuinely come back empty
    /// here. Falls back to a `Missing`, error-typed expression instead of
    /// panicking.
    fn expect_expr(&mut self, expr: Option<Expression<'ast>>, fallback_span: Span) -> ExpressionId {
        match expr {
            Some(expr) => self.infer(expr),
            None => self.hir.add_expression(
                ExpressionKind::Missing,
                self.ctx.type_interner.error_id,
                fallback_span,
            ),
        }
    }

    /// `expect_expr`'s counterpart for a position checked against an
    /// expected type rather than inferred.
    fn expect_expr_checked(
        &mut self,
        expr: Option<Expression<'ast>>,
        ty: TypeId,
        fallback_span: Span,
    ) -> ExpressionId {
        match expr {
            Some(expr) => self.check(expr, ty),
            None => self.hir.add_expression(
                ExpressionKind::Missing,
                self.ctx.type_interner.error_id,
                fallback_span,
            ),
        }
    }

    fn infer(&mut self, expr: Expression<'ast>) -> ExpressionId {
        match expr {
            Expression::Error(e) => self.hir.add_expression(
                ExpressionKind::Missing,
                self.ctx.type_interner.error_id,
                e.span(),
            ),
            Expression::UnitLiteral(e) => self.hir.add_expression(
                ExpressionKind::Unit,
                self.ctx.type_interner.unit_id,
                e.span(),
            ),
            Expression::BooleanLiteral(e) => {
                let value = e.value();
                self.hir.add_expression(
                    ExpressionKind::Boolean(value),
                    self.ctx.type_interner.bool_id,
                    e.span(),
                )
            }
            Expression::IntegerLiteral(e) => match self.integer_literal_value(e) {
                Some(value) => {
                    let ty = self.fresh_int_var();
                    self.hir
                        .add_expression(ExpressionKind::Integer(value), ty, e.span())
                }
                None => self.hir.add_expression(
                    ExpressionKind::Integer(0),
                    self.ctx.type_interner.error_id,
                    e.span(),
                ),
            },
            // grouping parens are purely syntactic -- infer straight through,
            // producing no HIR node of their own (same as today's AST, which
            // never represented them at all)
            Expression::ParenthesizedExpression(e) => self.expect_expr(e.inner(), e.span()),
            Expression::Variable(e) => self.typecheck_variable(e),
            Expression::UnaryOperation(e) => self.typecheck_unary_operation(e),
            Expression::BinaryOperation(e) => self.typecheck_binary_operation(e),
            Expression::IfExpression(e) => self.typecheck_if_expression(e),
            Expression::Return(e) => self.typecheck_return(e),
            Expression::WhileLoop(e) => self.typecheck_while_expression(e),
            Expression::InfiniteLoop(e) => self.typecheck_loop_expression(e),
            Expression::Break(e) => self.typecheck_break(e),
            Expression::Continue(e) => self.typecheck_continue(e),
            Expression::Assignment(e) => self.typecheck_assign(e),
            Expression::Call(e) => self.typecheck_function_call(e),
            Expression::Block(e) => self.analyze_block(e, None),
            Expression::Index(e) => {
                unreachable!(
                    "infer called on a non-expression node: {:?}",
                    e.syntax().kind()
                )
            }
        }
    }

    fn typecheck_variable(&mut self, var: Variable<'ast>) -> ExpressionId {
        let name = var
            .name_token()
            .expect("Variable always wraps an Identifier");
        let symbol = self.ctx.string_interner.intern(name.text());
        let span = var.span();

        let binding_id = match self.symbol_table.find_binding(symbol) {
            Ok(binding_id) => binding_id,
            Err(LookupError::BlockedByBoundary(ScopeKind::ConstantBoundary)) => {
                self.diagnostics
                    .push(SemanticDiagnostic::NonConstantValue { span });
                return self.hir.add_expression(
                    ExpressionKind::Variable(BindingId::ERROR),
                    self.ctx.type_interner.error_id,
                    span,
                );
            }
            Err(LookupError::BlockedByBoundary(ScopeKind::FunctionBoundary)) => {
                self.diagnostics
                    .push(SemanticDiagnostic::CaptureInFunction { span });
                return self.hir.add_expression(
                    ExpressionKind::Variable(BindingId::ERROR),
                    self.ctx.type_interner.error_id,
                    span,
                );
            }
            Err(LookupError::BlockedByBoundary(ScopeKind::Normal)) => unreachable!(),
            Err(LookupError::NotFound) => {
                self.diagnostics.push(SemanticDiagnostic::UnresolvedName {
                    name: self.ctx.string_interner.resolve(symbol).unwrap(),
                    span,
                });
                return self.hir.add_expression(
                    ExpressionKind::Variable(BindingId::ERROR),
                    self.ctx.type_interner.error_id,
                    span,
                );
            }
        };

        let ty = match binding_id.kind() {
            BindingKind::Local => self
                .hir
                .get_local_binding(binding_id.as_local().unwrap())
                .ty(),
            BindingKind::Definition => self
                .hir
                .get_definition_binding(binding_id.as_definition().unwrap())
                .ty(),
        };

        self.hir
            .add_expression(ExpressionKind::Variable(binding_id), ty, span)
    }

    fn typecheck_unary_operation(&mut self, unary: UnaryOperation<'ast>) -> ExpressionId {
        let operator = UnOp::from_token_kind(unary.operator_token().kind);
        let rhs_id = self.expect_expr(unary.operand(), unary.span());

        let ty = match operator {
            UnOp::Not => {
                self.constrain(Constraint::Equality {
                    expected_id: self.ctx.type_interner.bool_id,
                    actual_id: self.hir.get_expression(rhs_id).ty(),
                    provenance: Provenance::UnaryOperandMismatch {
                        operator: operator.to_string(),
                        operand_span: self.hir.get_expression(rhs_id).span(),
                    },
                });
                self.ctx.type_interner.bool_id
            }
            UnOp::Neg => {
                let int_ty = self.fresh_int_var();
                self.constrain(Constraint::Equality {
                    expected_id: int_ty,
                    actual_id: self.hir.get_expression(rhs_id).ty(),
                    provenance: Provenance::UnaryOperandMismatch {
                        operator: operator.to_string(),
                        operand_span: self.hir.get_expression(rhs_id).span(),
                    },
                });
                self.hir.get_expression(rhs_id).ty()
            }
        };

        self.hir.add_expression(
            ExpressionKind::Unary {
                operator,
                operand_id: rhs_id,
            },
            ty,
            unary.span(),
        )
    }

    fn typecheck_binary_operation(&mut self, bin: BinaryOperation<'ast>) -> ExpressionId {
        let operator = BinOp::from_token_kind(bin.operator_token().kind);
        let lhs_id = self.expect_expr(bin.lhs(), bin.span());
        let rhs_id = self.expect_expr(bin.rhs(), bin.span());

        let ty = match operator {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                self.constrain(Constraint::Equality {
                    expected_id: self.hir.get_expression(lhs_id).ty(),
                    actual_id: self.hir.get_expression(rhs_id).ty(),
                    provenance: Provenance::BinaryOperandMismatch {
                        lhs_span: self.hir.get_expression(lhs_id).span(),
                        rhs_span: self.hir.get_expression(rhs_id).span(),
                    },
                });
                let int_ty = self.fresh_int_var();
                self.constrain(Constraint::Equality {
                    expected_id: int_ty,
                    actual_id: self.hir.get_expression(lhs_id).ty(),
                    provenance: Provenance::BinaryOperandNotNumeric {
                        operand_span: self.hir.get_expression(lhs_id).span(),
                    },
                });
                self.hir.get_expression(lhs_id).ty()
            }
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                self.constrain(Constraint::Equality {
                    expected_id: self.hir.get_expression(lhs_id).ty(),
                    actual_id: self.hir.get_expression(rhs_id).ty(),
                    provenance: Provenance::BinaryOperandMismatch {
                        lhs_span: self.hir.get_expression(lhs_id).span(),
                        rhs_span: self.hir.get_expression(rhs_id).span(),
                    },
                });
                let int_ty = self.fresh_int_var();
                self.constrain(Constraint::Equality {
                    expected_id: int_ty,
                    actual_id: self.hir.get_expression(lhs_id).ty(),
                    provenance: Provenance::BinaryOperandNotNumeric {
                        operand_span: self.hir.get_expression(lhs_id).span(),
                    },
                });
                self.ctx.type_interner.bool_id
            }
            BinOp::And | BinOp::Or => {
                self.constrain(Constraint::Equality {
                    expected_id: self.ctx.type_interner.bool_id,
                    actual_id: self.hir.get_expression(lhs_id).ty(),
                    provenance: Provenance::BinaryOperandNotBool {
                        operand_span: self.hir.get_expression(lhs_id).span(),
                    },
                });
                self.constrain(Constraint::Equality {
                    expected_id: self.ctx.type_interner.bool_id,
                    actual_id: self.hir.get_expression(rhs_id).ty(),
                    provenance: Provenance::BinaryOperandNotBool {
                        operand_span: self.hir.get_expression(rhs_id).span(),
                    },
                });
                self.ctx.type_interner.bool_id
            }
            BinOp::Eq | BinOp::Ne => {
                self.constrain(Constraint::Equality {
                    expected_id: self.hir.get_expression(lhs_id).ty(),
                    actual_id: self.hir.get_expression(rhs_id).ty(),
                    provenance: Provenance::BinaryOperandMismatch {
                        lhs_span: self.hir.get_expression(lhs_id).span(),
                        rhs_span: self.hir.get_expression(rhs_id).span(),
                    },
                });
                self.ctx.type_interner.bool_id
            }
        };

        self.hir.add_expression(
            ExpressionKind::Binary {
                operator,
                lhs_id,
                rhs_id,
            },
            ty,
            bin.span(),
        )
    }

    fn typecheck_assign(&mut self, assign: Assignment<'ast>) -> ExpressionId {
        let target_id = self.expect_expr(assign.target(), assign.span());
        let value = assign.value();
        let target_view = self.hir.get_expression(target_id);

        let target_binding_id = match *target_view.kind() {
            ExpressionKind::Variable(binding_id) => Some(binding_id),
            _ => None,
        };

        let target_is_error = match target_binding_id {
            Some(binding_id) if binding_id.as_local().is_some() => false,
            Some(binding_id) if binding_id.as_definition().is_some() => {
                self.diagnostics
                    .push(SemanticDiagnostic::InvalidAssignTarget {
                        span: assign.span(),
                    });
                true
            }
            Some(_) => true, // binding error (`UnresolvedName` diagnostic already reported)
            None => {
                self.diagnostics
                    .push(SemanticDiagnostic::InvalidAssignTarget {
                        span: target_view.span(),
                    });
                true
            }
        };

        let value_id = if target_is_error || target_view.ty() == self.ctx.type_interner.error_id {
            self.expect_expr(value, assign.span())
        } else {
            self.expect_expr_checked(value, target_view.ty(), assign.span())
        };

        self.hir.add_expression(
            ExpressionKind::Assign {
                target_id,
                value_id,
            },
            self.ctx.type_interner.unit_id,
            assign.span(),
        )
    }

    fn typecheck_function_call(&mut self, call: Call<'ast>) -> ExpressionId {
        let callee = call.callee().expect("Call always has a callee");
        let arg_list = call
            .argument_list()
            .expect("Call always has an ArgumentList");
        // Each argument is its own `Argument` wrapper (so the parser has
        // somewhere to attach the trailing comma), not a bare expression
        // directly under `ArgumentList` -- unwrap one level.
        let args: Vec<Expression<'ast>> = arg_list
            .arguments()
            .map(|arg| {
                arg.expr()
                    .expect("parser guarantees an expression on every well-formed Argument")
            })
            .collect();

        let callee_id = self.infer(callee);
        let callee_view = self.hir.get_expression(callee_id);

        if callee_view.ty() == self.ctx.type_interner.error_id {
            for &arg in &args {
                self.infer(arg); // surface errors inside args
            }
            return self.hir.add_expression(
                ExpressionKind::Call {
                    callee_id,
                    argument_id_span: ExpressionIdSpan { start: 0, len: 0 },
                },
                self.ctx.type_interner.error_id,
                call.span(),
            );
        }

        let Ty::Function {
            parameter_type_ids: parameters,
            return_type_id: ret,
        } = self.ctx.type_interner.resolve(callee_view.ty()).unwrap()
        else {
            self.diagnostics.push(SemanticDiagnostic::NotCallable {
                found: self.ctx.type_interner.to_string(callee_view.ty()),
                callee_span: callee_view.span(),
                call_span: call.span(),
            });
            for &arg in &args {
                self.infer(arg); // surface errors inside args
            }
            return self.hir.add_expression(
                ExpressionKind::Call {
                    callee_id,
                    argument_id_span: ExpressionIdSpan { start: 0, len: 0 },
                },
                self.ctx.type_interner.error_id,
                call.span(),
            );
        };
        let (parameter_tys, return_ty) = (parameters.to_vec(), *ret);

        if args.len() != parameter_tys.len() {
            self.diagnostics.push(SemanticDiagnostic::ArityMismatch {
                expected: parameter_tys.len(),
                found: args.len(),
                callee_span: callee_view.span(),
                call_span: call.span(),
                // when there are too few args, extra_arg_spans is empty (no extra args to point to),
                // and when there are too many, it correctly collects the spans of the surplus arguments.
                extra_arg_spans: args[parameter_tys.len().min(args.len())..]
                    .iter()
                    .map(Expression::span)
                    .collect(),
            });
        }

        let mut argument_ids: Vec<ExpressionId> = Vec::new();
        for (i, &arg) in args.iter().enumerate() {
            if i < parameter_tys.len() {
                argument_ids.push(self.check(arg, parameter_tys[i]));
            } else {
                argument_ids.push(self.infer(arg)); // arity mismatch: surface errors
            }
        }
        let argument_id_span = self.hir.add_expression_ids(&argument_ids);

        let ty = if args.len() != parameter_tys.len() {
            self.ctx.type_interner.error_id
        } else {
            return_ty
        };

        self.hir.add_expression(
            ExpressionKind::Call {
                callee_id,
                argument_id_span,
            },
            ty,
            call.span(),
        )
    }

    fn typecheck_return(&mut self, ret: Return<'ast>) -> ExpressionId {
        let value = ret.value();
        let span = ret.span();

        if self.current_return_ty.is_none() {
            let value_id = value.map(|v| self.infer(v));
            self.diagnostics
                .push(SemanticDiagnostic::ReturnOutsideFunction { span });
            return self.hir.add_expression(
                ExpressionKind::Return { value_id },
                self.ctx.type_interner.error_id,
                span,
            );
        }

        let return_ty = self.current_return_ty.unwrap();

        let value_id = match value {
            Some(v) => Some(self.check(v, return_ty)),
            None => {
                self.constrain(Constraint::Equality {
                    expected_id: return_ty,
                    actual_id: self.ctx.type_interner.unit_id,
                    provenance: Provenance::ReturnMissingValue { return_span: span },
                });
                None
            }
        };

        self.hir.add_expression(
            ExpressionKind::Return { value_id },
            self.ctx.type_interner.bottom_id,
            span,
        )
    }

    fn typecheck_if_expression(&mut self, if_expression: IfExpression<'ast>) -> ExpressionId {
        let condition_id = self.expect_expr_checked(
            if_expression.condition(),
            self.ctx.type_interner.bool_id,
            if_expression.span(),
        );
        let then_branch = if_expression.then_branch();
        let else_branch = if_expression.else_branch();

        let then_branch_id = match then_branch {
            Some(then_branch) => self.analyze_block(then_branch, None),
            None => self.hir.add_expression(
                ExpressionKind::Missing,
                self.ctx.type_interner.error_id,
                if_expression.span(),
            ),
        };

        let (else_branch_id, ty) = match else_branch {
            Some(else_node) => {
                let else_expression_id = self.infer(else_node);
                self.constrain(Constraint::Equality {
                    expected_id: self.hir.get_expression(then_branch_id).ty(),
                    actual_id: self.hir.get_expression(else_expression_id).ty(),
                    provenance: Provenance::IfBranchMismatch {
                        then_span: self.hir.get_expression(then_branch_id).span(),
                        else_span: self.hir.get_expression(else_expression_id).span(),
                    },
                });
                (
                    Some(else_expression_id),
                    self.hir.get_expression(then_branch_id).ty(),
                )
            }
            None => {
                self.constrain(Constraint::Equality {
                    expected_id: self.hir.get_expression(then_branch_id).ty(),
                    actual_id: self.ctx.type_interner.unit_id,
                    provenance: Provenance::IfWithoutElse {
                        then_span: self.hir.get_expression(then_branch_id).span(),
                    },
                });
                (None, self.ctx.type_interner.unit_id)
            }
        };

        self.hir.add_expression(
            ExpressionKind::If {
                condition_id,
                then_branch_id,
                else_branch_id,
            },
            ty,
            if_expression.span(),
        )
    }

    fn typecheck_while_expression(&mut self, while_expr: WhileLoop<'ast>) -> ExpressionId {
        let condition = while_expr.condition();
        let body = while_expr.body();

        // `while` never itself produces a value (see `typecheck_break`, which
        // rejects `break value` targeting a `LoopSource::While`), so
        // `result_ty` is never actually consulted; `unit_id` is just a cheap
        // placeholder rather than allocating a fresh, unused type variable.
        self.loop_frames.push(LoopFrame {
            source: LoopSource::While,
            result_ty: self.ctx.type_interner.unit_id,
            has_break: false,
        });

        let condition_id =
            self.expect_expr_checked(condition, self.ctx.type_interner.bool_id, while_expr.span());
        let condition_span = self.hir.get_expression(condition_id).span();

        // `if not condition { break; }`
        let negated_condition_id = self.hir.add_expression(
            ExpressionKind::Unary {
                operator: UnOp::Not,
                operand_id: condition_id,
            },
            self.ctx.type_interner.bool_id,
            condition_span,
        );
        let break_id = self.hir.add_expression(
            ExpressionKind::Break { value_id: None },
            self.ctx.type_interner.bottom_id,
            condition_span,
        );
        let guard_id = self.hir.add_expression(
            ExpressionKind::If {
                condition_id: negated_condition_id,
                then_branch_id: break_id,
                else_branch_id: None,
            },
            self.ctx.type_interner.unit_id,
            condition_span,
        );
        let guard_statement_id = self.hir.add_statement(
            StatementKind::Expression {
                expression_id: guard_id,
                has_semicolon: true,
            },
            condition_span,
        );
        let guard_statement_id_span = self.hir.add_statement_ids(&[guard_statement_id]);

        let original_body_id = match body {
            Some(body) => self.analyze_block(body, None),
            None => self.hir.add_expression(
                ExpressionKind::Missing,
                self.ctx.type_interner.error_id,
                while_expr.span(),
            ),
        };
        let body_span = self.hir.get_expression(original_body_id).span();

        // `{ if not condition { break; } <original body> }`
        let body_id = self.hir.add_expression(
            ExpressionKind::Block {
                statement_id_span: guard_statement_id_span,
                tail_id: Some(original_body_id),
            },
            self.hir.get_expression(original_body_id).ty(),
            body_span,
        );

        self.loop_frames.pop();

        self.constrain(Constraint::Equality {
            expected_id: self.hir.get_expression(body_id).ty(),
            actual_id: self.ctx.type_interner.unit_id,
            provenance: Provenance::LoopBodyNotUnit {
                source: LoopSource::While,
                body_span,
            },
        });

        self.hir.add_expression(
            ExpressionKind::Loop {
                body_id,
                source: LoopSource::While,
            },
            self.ctx.type_interner.unit_id,
            while_expr.span(),
        )
    }

    fn typecheck_loop_expression(&mut self, loop_expr: InfiniteLoop<'ast>) -> ExpressionId {
        let body = loop_expr.body();

        let result_ty = self.fresh_ty_var();
        self.loop_frames.push(LoopFrame {
            source: LoopSource::Loop,
            result_ty,
            has_break: false,
        });

        let body_id = match body {
            Some(body) => self.analyze_block(body, None),
            None => self.hir.add_expression(
                ExpressionKind::Missing,
                self.ctx.type_interner.error_id,
                loop_expr.span(),
            ),
        };

        let frame = self
            .loop_frames
            .pop()
            .expect("just pushed this loop's own frame");

        self.constrain(Constraint::Equality {
            expected_id: self.hir.get_expression(body_id).ty(),
            actual_id: self.ctx.type_interner.unit_id,
            provenance: Provenance::LoopBodyNotUnit {
                source: LoopSource::Loop,
                body_span: self.hir.get_expression(body_id).span(),
            },
        });

        let ty = if frame.has_break {
            result_ty
        } else {
            self.ctx.type_interner.bottom_id
        };

        self.hir.add_expression(
            ExpressionKind::Loop {
                body_id,
                source: LoopSource::Loop,
            },
            ty,
            loop_expr.span(),
        )
    }

    fn typecheck_break(&mut self, brk: Break<'ast>) -> ExpressionId {
        let value = brk.value();
        let span = brk.span();

        let Some(&LoopFrame {
            source, result_ty, ..
        }) = self.loop_frames.last()
        else {
            let value_id = value.map(|v| self.infer(v));
            self.diagnostics
                .push(SemanticDiagnostic::BreakOutsideLoop { span });
            return self.hir.add_expression(
                ExpressionKind::Break { value_id },
                self.ctx.type_interner.error_id,
                span,
            );
        };

        if let Some(v) = value
            && source == LoopSource::While
        {
            // still lower the value for recovery; its type is irrelevant
            let _ = self.infer(v);
            self.diagnostics
                .push(SemanticDiagnostic::BreakWithValueFromWhile { span });
            return self.hir.add_expression(
                ExpressionKind::Break { value_id: None },
                self.ctx.type_interner.bottom_id,
                span,
            );
        }

        self.loop_frames
            .last_mut()
            .expect("just matched a non-empty loop_frames above")
            .has_break = true;

        let value_id = match value {
            Some(v) => Some(self.check(v, result_ty)),
            None => {
                self.constrain(Constraint::Equality {
                    expected_id: result_ty,
                    actual_id: self.ctx.type_interner.unit_id,
                    provenance: Provenance::TypeMismatch { span },
                });
                None
            }
        };

        self.hir.add_expression(
            ExpressionKind::Break { value_id },
            self.ctx.type_interner.bottom_id,
            span,
        )
    }

    fn typecheck_continue(&mut self, cont: Continue<'ast>) -> ExpressionId {
        let span = cont.span();
        let ty = if self.loop_frames.is_empty() {
            self.diagnostics
                .push(SemanticDiagnostic::ContinueOutsideLoop { span });
            self.ctx.type_interner.error_id
        } else {
            self.ctx.type_interner.bottom_id
        };

        self.hir.add_expression(ExpressionKind::Continue, ty, span)
    }

    fn resolve_type_annotation(&mut self, type_expr: Type<'ast>) -> TypeId {
        let name = type_expr
            .name_token()
            .expect("parser guarantees an Identifier on every well-formed Type");
        let symbol = self.ctx.string_interner.intern(name.text());
        if let Some(ty) = self
            .ctx
            .type_interner
            .builtin_type_id(symbol, &self.ctx.string_interner)
        {
            return ty;
        }
        // TODO: try resolve user-defined types here
        self.diagnostics.push(SemanticDiagnostic::UnknownType {
            name: self.ctx.string_interner.resolve(symbol).unwrap(),
            span: name.span,
        });
        self.ctx.type_interner.error_id
    }

    fn constrain(&mut self, constraint: Constraint) {
        let Constraint::Equality {
            expected_id,
            actual_id,
            ..
        } = &constraint;
        // don't constrain error types, but poison silently
        if *expected_id == self.ctx.type_interner.error_id
            || *actual_id == self.ctx.type_interner.error_id
        {
            return;
        }
        self.constraints.push(constraint);
    }

    fn shallow_resolve(&mut self, ty: TypeId) -> TypeId {
        match self.ctx.type_interner.resolve(ty).unwrap() {
            Ty::Infer(InferTy::TyVar(vid)) => {
                let root = self.substitutions.find_type_var(*vid);
                match self.substitutions.get_concrete_type_var(root) {
                    Some(concrete) => self.shallow_resolve(concrete),
                    None => self
                        .ctx
                        .type_interner
                        .intern(Ty::Infer(InferTy::TyVar(root))),
                }
            }
            Ty::Infer(InferTy::IntVar(vid)) => {
                let root = self.substitutions.find_int_var(*vid);
                match self.substitutions.get_concrete_int_var(root) {
                    Some(concrete) => self.shallow_resolve(concrete),
                    None => self
                        .ctx
                        .type_interner
                        .intern(Ty::Infer(InferTy::IntVar(root))),
                }
            }
            _ => ty,
        }
    }

    fn unify(&mut self, expected: TypeId, actual: TypeId) -> Result<(), UnificationError> {
        let expected = self.shallow_resolve(expected);
        let actual = self.shallow_resolve(actual);

        if expected == actual {
            return Ok(());
        }

        // A diverging expression (`return`, `break`, `continue`, ...) never
        // actually produces a value, so its `Bottom` type is compatible with
        // whatever was expected — the same "never type coerces to anything"
        // rule as Rust's `!`. Checked symmetrically, not just for `actual`:
        // callers don't consistently put "the value's real type" in one
        // particular slot (e.g. `IfWithoutElse` constrains the then-branch's
        // own type as `expected` against a literal `unit_id` `actual`), so
        // `Bottom` needs to short-circuit compatibility from either side,
        // mirroring how the `TyVar` arms below are already handled in both
        // orders rather than just one.
        if expected == self.ctx.type_interner.bottom_id
            || actual == self.ctx.type_interner.bottom_id
        {
            return Ok(());
        }

        match (
            self.ctx.type_interner.resolve(expected).unwrap(),
            self.ctx.type_interner.resolve(actual).unwrap(),
        ) {
            (Ty::Infer(InferTy::TyVar(vid1)), Ty::Infer(InferTy::TyVar(vid2))) => {
                self.substitutions.union_type_vars(*vid1, *vid2);
                Ok(())
            }
            (Ty::Infer(InferTy::IntVar(vid1)), Ty::Infer(InferTy::IntVar(vid2))) => {
                self.substitutions.union_int_vars(*vid1, *vid2);
                Ok(())
            }
            (Ty::Infer(InferTy::TyVar(vid)), _) => {
                self.substitutions.set_concrete_type_var(*vid, actual);
                Ok(())
            }
            (_, Ty::Infer(InferTy::TyVar(vid))) => {
                self.substitutions.set_concrete_type_var(*vid, expected);
                Ok(())
            }
            (Ty::Infer(InferTy::IntVar(vid)), Ty::Signed(_) | Ty::Unsigned(_)) => {
                self.substitutions.set_concrete_int_var(*vid, actual);
                Ok(())
            }
            (Ty::Signed(_) | Ty::Unsigned(_), Ty::Infer(InferTy::IntVar(vid))) => {
                self.substitutions.set_concrete_int_var(*vid, expected);
                Ok(())
            }
            _ => Err(UnificationError::TypeMismatch {
                expected_id: expected,
                actual_id: actual,
            }),
        }
    }

    fn fresh_ty_var(&mut self) -> TypeId {
        let vid = self.substitutions.make_type_var_set();
        self.ctx
            .type_interner
            .intern(Ty::Infer(InferTy::TyVar(vid)))
    }

    fn fresh_int_var(&mut self) -> TypeId {
        let vid = self.substitutions.make_int_var_set();
        self.ctx
            .type_interner
            .intern(Ty::Infer(InferTy::IntVar(vid)))
    }
}
