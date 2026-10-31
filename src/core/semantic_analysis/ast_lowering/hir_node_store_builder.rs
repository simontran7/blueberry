use crate::core::common::runs::Run;
use crate::core::common::text_range::{AbsoluteTextRange, TextOffset};
use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, ExpressionHandle, HirNodeRanges, HirNodeStore, LocalBindingHandle, StatementHandle,
    TypeExpressionHandle,
};
use crate::core::syntactic_analysis::cst::RedNode;

pub(crate) struct HirNodeStoreBuilder<'db> {
    anchor: TextOffset,
    store: HirNodeStore<'db>,
    ranges: HirNodeRanges,
}

impl<'db> HirNodeStoreBuilder<'db> {
    pub(crate) fn new(anchor: TextOffset) -> Self {
        Self {
            anchor,
            store: HirNodeStore::default(),
            ranges: HirNodeRanges::default(),
        }
    }

    pub(crate) fn add_expression(
        &mut self,
        expression: nodes::Expression<'db>,
        node: &RedNode,
    ) -> ExpressionHandle {
        let expression_handle = self.store.expressions.add(expression);
        self.ranges
            .expressions
            .add(expression_handle, node.range().relative_to(self.anchor));
        expression_handle
    }

    pub(crate) fn add_statement(
        &mut self,
        statement: nodes::Statement,
        node: &RedNode,
    ) -> StatementHandle {
        let statement_handle = self.store.statements.add(statement);
        self.ranges
            .statements
            .add(statement_handle, node.range().relative_to(self.anchor));
        statement_handle
    }

    pub(crate) fn add_local_binding(
        &mut self,
        binding: nodes::LocalBinding<'db>,
        name_range: AbsoluteTextRange,
    ) -> LocalBindingHandle {
        let binding_handle = self.store.local_bindings.add(binding);
        self.ranges
            .local_bindings
            .add(binding_handle, name_range.relative_to(self.anchor));
        binding_handle
    }

    pub(crate) fn add_type_expression(
        &mut self,
        type_expression: nodes::TypeExpression<'db>,
        node: &RedNode,
    ) -> TypeExpressionHandle {
        let type_expression_handle = self.store.type_expressions.add(type_expression);
        self.ranges.type_expressions.add(
            type_expression_handle,
            node.range().relative_to(self.anchor),
        );
        type_expression_handle
    }

    pub(crate) fn add_expression_children(
        &mut self,
        child_handles: &[ExpressionHandle],
    ) -> Run<ExpressionHandle> {
        self.store.expression_children.add_many(child_handles)
    }

    pub(crate) fn add_statement_children(
        &mut self,
        child_handles: &[StatementHandle],
    ) -> Run<StatementHandle> {
        self.store.statement_children.add_many(child_handles)
    }

    pub(crate) fn add_type_expression_children(
        &mut self,
        child_handles: &[TypeExpressionHandle],
    ) -> Run<TypeExpressionHandle> {
        self.store.type_expression_children.add_many(child_handles)
    }

    pub(crate) fn finish(self) -> (HirNodeStore<'db>, HirNodeRanges) {
        (self.store, self.ranges)
    }
}
