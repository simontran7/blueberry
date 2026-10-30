use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::hir_node_store_builder::HirNodeStoreBuilder;
use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, ExpressionHandle, HirNodeSourceMap, LocalBindingHandle, StatementHandle,
    TypeExpressionHandle,
};
use crate::core::semantic_analysis::ast_lowering::{path_lowering, type_expression_lowering};
use crate::core::semantic_analysis::keys::BlockKey;
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::source_file_key::SourceFileKey;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};
use crate::core::syntactic_analysis::cst::{RedNode, RedNodeTag, RedToken};

pub(crate) struct BodyLowerer<'db> {
    db: &'db dyn crate::Db,
    file: SourceFileKey,
    owner: Definition<'db>,
    current_block: Option<BlockKey<'db>>,
    builder: HirNodeStoreBuilder<'db>,
}

impl<'db> BodyLowerer<'db> {
    pub(crate) fn new(db: &'db dyn crate::Db, file: SourceFileKey, owner: Definition<'db>) -> Self {
        Self {
            db,
            file,
            owner,
            current_block: None,
            builder: HirNodeStoreBuilder::new(owner.anchor(db)),
        }
    }

    pub(crate) fn lower_expression(&mut self, expression: ast::Expression) -> ExpressionHandle {
        match expression {
            ast::Expression::IntegerLiteral(literal) => self.builder.add_expression(
                nodes::Expression::Integer {
                    magnitude: literal.value().ok(),
                },
                literal.red(),
            ),
            ast::Expression::BooleanLiteral(literal) => self.builder.add_expression(
                literal
                    .value()
                    .map_or(nodes::Expression::Error, nodes::Expression::Boolean),
                literal.red(),
            ),
            ast::Expression::TupleExpression(tuple) => {
                let element_handles: Vec<ExpressionHandle> = tuple
                    .elements()
                    .map(|element| self.lower_expression(element))
                    .collect();
                let element_run = self.builder.add_expression_children(&element_handles);
                self.builder
                    .add_expression(nodes::Expression::Tuple { element_run }, tuple.red())
            }
            ast::Expression::FieldExpression(field) => {
                let receiver_handle = self.lower_optional_expression(field.receiver(), field.red());
                let name = field
                    .field()
                    .map(|token| path_lowering::lower_name(self.db, &token));
                self.builder.add_expression(
                    nodes::Expression::Field {
                        receiver_handle,
                        name,
                    },
                    field.red(),
                )
            }
            ast::Expression::PathExpression(path_expression) => {
                let lowered = path_expression
                    .path()
                    .and_then(|path| self.lower_path(&path))
                    .map_or(nodes::Expression::Error, nodes::Expression::Path);
                self.builder.add_expression(lowered, path_expression.red())
            }
            ast::Expression::ParenthesizedExpression(parenthesized) => {
                self.lower_optional_expression(parenthesized.expression(), parenthesized.red())
            }
            ast::Expression::UnaryOperation(unary) => {
                let operator = unary
                    .operator()
                    .and_then(|token| nodes::UnaryOperator::try_from(token.kind()).ok());
                let Some(operator) = operator else {
                    return self
                        .builder
                        .add_expression(nodes::Expression::Error, unary.red());
                };

                let operand_handle = self.lower_optional_expression(unary.operand(), unary.red());

                self.builder.add_expression(
                    nodes::Expression::UnaryOperation {
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
                    .and_then(|token| nodes::BinaryOperator::try_from(token.kind()).ok());

                let rhs_handle = self.lower_optional_expression(binary.rhs(), binary.red());

                self.builder.add_expression(
                    nodes::Expression::BinaryOperation {
                        lhs_handle,
                        operator,
                        rhs_handle,
                    },
                    binary.red(),
                )
            }
            ast::Expression::Call(call) => {
                let callee_handle = self.lower_optional_expression(call.callee(), call.red());

                let argument_handles: Vec<ExpressionHandle> = call
                    .arguments()
                    .into_iter()
                    .flat_map(|list| list.arguments().collect::<Vec<_>>())
                    .map(|argument| {
                        self.lower_optional_expression(argument.value(), argument.red())
                    })
                    .collect();
                let argument_run = self.builder.add_expression_children(&argument_handles);

                self.builder.add_expression(
                    nodes::Expression::Call {
                        callee_handle,
                        argument_run,
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
                    nodes::Expression::Assignment {
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

                self.builder.add_expression(
                    nodes::Expression::Return { value_handle },
                    return_expression.red(),
                )
            }
            ast::Expression::Break(break_expression) => {
                let value_handle = break_expression
                    .value()
                    .map(|value| self.lower_expression(value));

                self.builder.add_expression(
                    nodes::Expression::Break { value_handle },
                    break_expression.red(),
                )
            }
            ast::Expression::Continue(continue_expression) => self
                .builder
                .add_expression(nodes::Expression::Continue, continue_expression.red()),
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
                    nodes::Expression::If {
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
                    nodes::Expression::Loop {
                        source: nodes::LoopSource::Loop,
                        body_handle,
                    },
                    infinite_loop.red(),
                )
            }
            ast::Expression::WhileLoop(while_loop) => self.lower_while_loop(while_loop),
        }
    }

    pub(crate) fn lower_function(
        mut self,
        function: &ast::FunctionDefinition,
    ) -> (nodes::Body<'db>, HirNodeSourceMap) {
        let parameter_handles = self.lower_parameters(function.parameter_list());
        let root_handle = self.lower_optional_block(function.body(), function.red());
        self.finish(root_handle, parameter_handles)
    }

    pub(crate) fn lower_constant(
        mut self,
        constant: &ast::ConstantDefinition,
    ) -> (nodes::Body<'db>, HirNodeSourceMap) {
        let root_handle = self.lower_optional_expression(constant.value(), constant.red());
        self.finish(root_handle, Box::new([]))
    }

    fn finish(
        self,
        root_handle: ExpressionHandle,
        parameter_handles: Box<[LocalBindingHandle]>,
    ) -> (nodes::Body<'db>, HirNodeSourceMap) {
        let (store, source_map) = self.builder.finish();
        let body = nodes::Body {
            store,
            root_handle,
            parameter_handles,
        };
        (body, source_map)
    }

    fn lower_path(&self, path: &ast::Path) -> Option<nodes::Path<'db>> {
        path_lowering::lower_path(self.db, path)
    }

    fn lower_statement(&mut self, statement: ast::Statement) -> StatementHandle {
        match statement {
            ast::Statement::Let(let_statement) => {
                let name_token = let_statement.name();
                let name = name_token
                    .as_ref()
                    .map(|token| token.lexeme().to_string())
                    .unwrap_or_default();
                let name_range = name_token
                    .as_ref()
                    .map_or_else(|| let_statement.red().range(), RedToken::range);
                let name_handle = self.builder.add_local_binding(
                    nodes::LocalBinding {
                        name: Symbol::new(self.db, name),
                        mutable: let_statement.is_mutable(),
                    },
                    name_range,
                );

                let annotation_handle = let_statement
                    .type_annotation()
                    .map(|annotation| self.lower_type_expression(&annotation));

                let initializer_handle =
                    self.lower_optional_expression(let_statement.value(), let_statement.red());

                self.builder.add_statement(
                    nodes::Statement::Let {
                        name_handle,
                        annotation_handle,
                        initializer_handle,
                    },
                    let_statement.red(),
                )
            }
            ast::Statement::Definition(definition) => self
                .builder
                .add_statement(nodes::Statement::Definition, definition.red()),
            ast::Statement::Expression(expression_statement) => {
                let expression_handle = self.lower_optional_expression(
                    expression_statement.expression(),
                    expression_statement.red(),
                );

                let has_semicolon = expression_statement.has_semicolon();

                self.builder.add_statement(
                    nodes::Statement::Expression {
                        expression_handle,
                        has_semicolon,
                    },
                    expression_statement.red(),
                )
            }
        }
    }

    fn lower_parameters(
        &mut self,
        parameter_list: Option<ast::ParameterList>,
    ) -> Box<[LocalBindingHandle]> {
        let parameters: Vec<ast::Parameter> = parameter_list
            .map(|parameter_list| parameter_list.parameters().collect())
            .unwrap_or_default();

        parameters
            .into_iter()
            .map(|parameter| {
                let name_token = parameter.name();
                let name = name_token
                    .as_ref()
                    .map(|token| token.lexeme().to_string())
                    .unwrap_or_default();
                let name_range = name_token
                    .as_ref()
                    .map_or_else(|| parameter.red().range(), RedToken::range);
                self.builder.add_local_binding(
                    nodes::LocalBinding {
                        name: Symbol::new(self.db, name),
                        mutable: false,
                    },
                    name_range,
                )
            })
            .collect()
    }

    fn lower_while_loop(&mut self, while_loop: ast::WhileLoop) -> ExpressionHandle {
        let node = while_loop.red();

        // lowers to `if c { b } else { break }`
        let condition_handle = self.lower_optional_expression(while_loop.condition(), node);

        let then_branch_handle = self.lower_optional_block(while_loop.body(), node);

        let else_branch_handle = self
            .builder
            .add_expression(nodes::Expression::Break { value_handle: None }, node);

        let if_expression_handle = self.builder.add_expression(
            nodes::Expression::If {
                condition_handle,
                then_branch_handle,
                else_branch_handle: Some(else_branch_handle),
            },
            node,
        );

        // lowers `loop {}`
        let statement_run = self.builder.add_statement_children(&[]);

        let body_handle = self.builder.add_expression(
            nodes::Expression::Block {
                block_key: None,
                statement_run,
                tail_handle: Some(if_expression_handle),
            },
            node,
        );

        self.builder.add_expression(
            nodes::Expression::Loop {
                source: nodes::LoopSource::While,
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
            None => self
                .builder
                .add_expression(nodes::Expression::Error, parent),
        }
    }

    fn lower_block(&mut self, block: ast::Block) -> ExpressionHandle {
        let block_key = BlockKey::new(
            self.db,
            self.file,
            self.owner,
            self.current_block,
            RedNodeTag::new(block.red()),
        );
        let parent_block = self.current_block.replace(block_key);

        let mut statements = block.statements().peekable();
        let mut statement_handles: Vec<StatementHandle> = Vec::new();
        let mut tail_handle = None;
        while let Some(statement) = statements.next() {
            match statement {
                ast::Statement::Expression(last)
                    if statements.peek().is_none() && !last.has_semicolon() =>
                {
                    tail_handle = last
                        .expression()
                        .map(|expression| self.lower_expression(expression));
                }
                statement => statement_handles.push(self.lower_statement(statement)),
            }
        }
        let statement_run = self.builder.add_statement_children(&statement_handles);
        self.current_block = parent_block;

        self.builder.add_expression(
            nodes::Expression::Block {
                block_key: Some(block_key),
                statement_run,
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
            None => self
                .builder
                .add_expression(nodes::Expression::Error, parent),
        }
    }

    fn lower_type_expression(
        &mut self,
        type_expression: &ast::TypeExpression,
    ) -> TypeExpressionHandle {
        type_expression_lowering::lower_type_expression(self.db, &mut self.builder, type_expression)
    }
}
