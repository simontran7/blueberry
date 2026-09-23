use std::sync::Arc;

use crate::core::common::segments::Segment;
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::definition_body_builder::DefinitionBodyBuilder;
use crate::core::semantic_analysis::hir::nodes::{
    BinaryOperator, BlockKey, DefinitionBody, DefinitionBodySourceMap, Expression,
    ExpressionHandle, LocalBinding, LocalBindingHandle, LoopSource, Path, Statement,
    StatementHandle, TypeAnnotation, TypeAnnotationHandle, UnaryOperator,
};
use crate::core::semantic_analysis::red_node_directory::{RedNodeDirectory, RedNodeId};
use crate::core::syntactic_analysis::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::RedNode;

pub(crate) struct DefinitionBodyLowerer<'db> {
    db: &'db dyn crate::Db,
    directory: Arc<RedNodeDirectory<'db>>,
    builder: DefinitionBodyBuilder<'db>,
}

impl<'db> DefinitionBodyLowerer<'db> {
    pub(crate) fn new(db: &'db dyn crate::Db, directory: Arc<RedNodeDirectory<'db>>) -> Self {
        Self {
            db,
            directory,
            builder: DefinitionBodyBuilder::new(),
        }
    }

    pub(crate) fn lower_function(
        mut self,
        function: &ast::FunctionDefinition,
    ) -> (DefinitionBody<'db>, DefinitionBodySourceMap) {
        let parameters = self.lower_parameters(function.parameter_list());
        let root = self.lower_optional_block(function.body(), function.red());
        self.builder.finish(root, parameters)
    }

    pub(crate) fn lower_constant(
        mut self,
        constant: &ast::ConstantDefinition,
    ) -> (DefinitionBody<'db>, DefinitionBodySourceMap) {
        let parameters = self.builder.add_binding_children(&[]);
        let root = self.lower_optional_expression(constant.value(), constant.red());
        self.builder.finish(root, parameters)
    }

    fn lower_statement(&mut self, statement: ast::Statement) -> StatementHandle {
        match statement {
            ast::Statement::LetStatement(let_statement) => {
                let name = let_statement
                    .name()
                    .map_or_else(String::new, |token| token.lexeme().to_string());
                let name_handle = self.builder.add_local_binding(
                    LocalBinding {
                        name: Symbol::new(self.db, name),
                        mutable: let_statement.is_mutable(),
                    },
                    let_statement.red(),
                );

                let annotation = let_statement
                    .type_annotation()
                    .map(|type_expression| self.lower_type_annotation(&type_expression));

                let initializer_handle = let_statement
                    .value()
                    .map(|value| self.lower_expression(value));

                self.builder.add_statement(
                    Statement::Let {
                        name_handle,
                        annotation,
                        initializer_handle,
                    },
                    let_statement.red(),
                )
            }
            ast::Statement::DefinitionStatement(definition) => self
                .builder
                .add_statement(Statement::Definition, definition.red()),
            ast::Statement::ExpressionStatement(expression_statement) => {
                let expression_handle = self.lower_optional_expression(
                    expression_statement.expression(),
                    expression_statement.red(),
                );

                let has_semicolon = expression_statement.has_semicolon();

                self.builder.add_statement(
                    Statement::Expression {
                        expression_handle,
                        has_semicolon,
                    },
                    expression_statement.red(),
                )
            }
        }
    }

    pub(crate) fn lower_expression(&mut self, expression: ast::Expression) -> ExpressionHandle {
        match expression {
            ast::Expression::IntegerLiteral(literal) => self.builder.add_expression(
                Expression::Integer(literal.value().unwrap_or(0)),
                literal.red(),
            ),
            ast::Expression::BooleanLiteral(literal) => self.builder.add_expression(
                literal
                    .value()
                    .map_or(Expression::Hole, Expression::Boolean),
                literal.red(),
            ),
            ast::Expression::UnitLiteral(literal) => {
                self.builder.add_expression(Expression::Unit, literal.red())
            }
            ast::Expression::PathExpression(path_expression) => {
                let lowered = path_expression
                    .path()
                    .and_then(|path| self.lower_path(&path))
                    .map_or(Expression::Hole, Expression::Path);
                self.builder.add_expression(lowered, path_expression.red())
            }
            ast::Expression::ParenthesizedExpression(parenthesized) => {
                self.lower_optional_expression(parenthesized.expression(), parenthesized.red())
            }
            ast::Expression::UnaryOperation(unary) => {
                let operator = unary
                    .operator()
                    .and_then(|token| UnaryOperator::try_from(token.kind()).ok());
                let Some(operator) = operator else {
                    return self.builder.add_expression(Expression::Hole, unary.red());
                };

                let operand_handle = self.lower_optional_expression(unary.operand(), unary.red());

                self.builder.add_expression(
                    Expression::UnaryOperation {
                        operator,
                        operand_handle,
                    },
                    unary.red(),
                )
            }
            ast::Expression::BinaryOperation(binary) => {
                let lhs_handle = self.lower_optional_expression(binary.lhs(), binary.red());

                let operator = binary
                    .operator()
                    .and_then(|token| BinaryOperator::try_from(token.kind()).ok());

                let rhs_handle = self.lower_optional_expression(binary.rhs(), binary.red());

                self.builder.add_expression(
                    Expression::BinaryOperation {
                        lhs_handle,
                        operator,
                        rhs_handle,
                    },
                    binary.red(),
                )
            }
            ast::Expression::Call(call) => {
                let callee_handle = self.lower_optional_expression(call.callee(), call.red());

                let arguments: Vec<ExpressionHandle> = call
                    .arguments()
                    .into_iter()
                    .flat_map(|list| list.arguments().collect::<Vec<_>>())
                    .map(|argument| {
                        self.lower_optional_expression(argument.value(), argument.red())
                    })
                    .collect();
                let arguments = self.builder.add_expression_children(&arguments);

                self.builder.add_expression(
                    Expression::Call {
                        callee_handle,
                        arguments,
                    },
                    call.red(),
                )
            }
            ast::Expression::Assignment(assignment) => {
                let target_handle =
                    self.lower_optional_expression(assignment.target(), assignment.red());

                let value_handle =
                    self.lower_optional_expression(assignment.value(), assignment.red());

                self.builder.add_expression(
                    Expression::Assignment {
                        target_handle,
                        value_handle,
                    },
                    assignment.red(),
                )
            }
            ast::Expression::Return(return_expression) => {
                let value_handle = return_expression
                    .value()
                    .map(|value| self.lower_expression(value));

                self.builder
                    .add_expression(Expression::Return { value_handle }, return_expression.red())
            }
            ast::Expression::Break(break_expression) => {
                let value_handle = break_expression
                    .value()
                    .map(|value| self.lower_expression(value));

                self.builder
                    .add_expression(Expression::Break { value_handle }, break_expression.red())
            }
            ast::Expression::Continue(continue_expression) => self
                .builder
                .add_expression(Expression::Continue, continue_expression.red()),
            ast::Expression::Block(block) => self.lower_block(block),
            ast::Expression::IfExpression(if_expression) => {
                let condition_handle =
                    self.lower_optional_expression(if_expression.condition(), if_expression.red());
                let then_branch_handle =
                    self.lower_optional_block(if_expression.then_branch(), if_expression.red());
                let else_branch_handle =
                    if_expression
                        .else_branch()
                        .map(|else_branch| match else_branch {
                            ast::ElseBranch::Block(block) => self.lower_block(block),
                            ast::ElseBranch::IfExpression(nested) => {
                                self.lower_expression(ast::Expression::IfExpression(nested))
                            }
                        });
                self.builder.add_expression(
                    Expression::If {
                        condition_handle,
                        then_branch_handle,
                        else_branch_handle,
                    },
                    if_expression.red(),
                )
            }
            ast::Expression::InfiniteLoop(infinite_loop) => {
                let body_handle =
                    self.lower_optional_block(infinite_loop.body(), infinite_loop.red());
                self.builder.add_expression(
                    Expression::Loop {
                        source: LoopSource::Loop,
                        body_handle,
                    },
                    infinite_loop.red(),
                )
            }
            ast::Expression::WhileLoop(while_loop) => self.lower_while_loop(while_loop),
        }
    }

    fn lower_parameters(
        &mut self,
        parameter_list: Option<ast::ParameterList>,
    ) -> Segment<LocalBindingHandle> {
        let parameters: Vec<ast::Parameter> = parameter_list
            .map(|parameter_list| parameter_list.parameters().collect())
            .unwrap_or_default();

        let handles: Vec<LocalBindingHandle> = parameters
            .into_iter()
            .map(|parameter| {
                let name = parameter
                    .name()
                    .map_or_else(String::new, |token| token.lexeme().to_string());
                self.builder.add_local_binding(
                    LocalBinding {
                        name: Symbol::new(self.db, name),
                        mutable: false,
                    },
                    parameter.red(),
                )
            })
            .collect();

        self.builder.add_binding_children(&handles)
    }

    /// `while c { b }` is lowered to `loop { if c { b } else { break } }`.
    fn lower_while_loop(&mut self, while_loop: ast::WhileLoop) -> ExpressionHandle {
        let node = while_loop.red();

        // lowers to `if c { b } else { break }`
        let condition_handle = self.lower_optional_expression(while_loop.condition(), node);

        let then_branch_handle = self.lower_optional_block(while_loop.body(), node);

        let else_branch_handle = self
            .builder
            .add_expression(Expression::Break { value_handle: None }, node);

        let if_expression_handle = self.builder.add_expression(
            Expression::If {
                condition_handle,
                then_branch_handle,
                else_branch_handle: Some(else_branch_handle),
            },
            node,
        );

        // lowers `loop {}`
        let statements = self.builder.add_statement_children(&[]);

        let body_handle = self.builder.add_expression(
            Expression::Block {
                block_key: None,
                statements,
                tail_handle: Some(if_expression_handle),
            },
            node,
        );

        self.builder.add_expression(
            Expression::Loop {
                source: LoopSource::While,
                body_handle,
            },
            node,
        )
    }

    fn lower_optional_block(
        &mut self,
        block: Option<ast::Block>,
        parent: &RedNode,
    ) -> ExpressionHandle {
        match block {
            Some(block) => self.lower_block(block),
            None => self.builder.add_expression(Expression::Hole, parent),
        }
    }

    fn lower_block(&mut self, block: ast::Block) -> ExpressionHandle {
        let block_key = self.block_key(&block);

        let mut statements = block.statements().peekable();
        let mut handles: Vec<StatementHandle> = Vec::new();
        let mut tail_handle = None;
        while let Some(statement) = statements.next() {
            match statement {
                ast::Statement::ExpressionStatement(last)
                    if statements.peek().is_none() && !last.has_semicolon() =>
                {
                    tail_handle = last
                        .expression()
                        .map(|expression| self.lower_expression(expression));
                }
                statement => handles.push(self.lower_statement(statement)),
            }
        }
        let statements = self.builder.add_statement_children(&handles);

        self.builder.add_expression(
            Expression::Block {
                block_key,
                statements,
                tail_handle,
            },
            block.red(),
        )
    }

    fn lower_optional_expression(
        &mut self,
        expression: Option<ast::Expression>,
        parent: &RedNode,
    ) -> ExpressionHandle {
        match expression {
            Some(expression) => self.lower_expression(expression),
            None => self.builder.add_expression(Expression::Hole, parent),
        }
    }

    fn lower_path(&self, path: &ast::Path) -> Option<Path<'db>> {
        super::lower_path(self.db, path)
    }

    fn block_key(&self, block: &ast::Block) -> Option<BlockKey<'db>> {
        block.definitions().next()?;
        let id = self.directory.id_of(block.red())?;
        Some(BlockKey::new(self.db, RedNodeId::new(id)))
    }

    fn lower_type_annotation(
        &mut self,
        type_expression: &ast::TypeExpression,
    ) -> TypeAnnotationHandle {
        let annotation = TypeAnnotation::from_type_expression(self.db, type_expression);
        self.builder
            .add_type_annotation(annotation, type_expression.red())
    }
}
