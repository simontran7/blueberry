use crate::core::common::runs::Run;
use crate::core::common::span::TextSize;
use crate::core::semantic_analysis::hir::nodes::{
    Expression, ExpressionHandle, HirNodeSourceMap, HirNodeStore, LocalBinding, LocalBindingHandle,
    Statement, StatementHandle, TypeExpression, TypeExpressionHandle,
};
use crate::core::syntactic_analysis::cst::{RedNode, RedNodeTag};

pub(in crate::core) struct HirNodeStoreBuilder<'db> {
    anchor: TextSize,
    store: HirNodeStore<'db>,
    source_map: HirNodeSourceMap,
}

impl<'db> HirNodeStoreBuilder<'db> {
    pub(in crate::core) fn new(anchor: TextSize) -> Self {
        Self {
            anchor,
            store: HirNodeStore::default(),
            source_map: HirNodeSourceMap::default(),
        }
    }

    pub(in crate::core) fn add_expression(
        &mut self,
        expression: Expression<'db>,
        node: &RedNode,
    ) -> ExpressionHandle {
        let expression_handle = self.store.expressions.add(expression);
        self.source_map.expressions.add(
            expression_handle,
            RedNodeTag::new(node).relative_to(self.anchor),
        );
        expression_handle
    }

    pub(in crate::core) fn add_statement(
        &mut self,
        statement: Statement,
        node: &RedNode,
    ) -> StatementHandle {
        let statement_handle = self.store.statements.add(statement);
        self.source_map.statements.add(
            statement_handle,
            RedNodeTag::new(node).relative_to(self.anchor),
        );
        statement_handle
    }

    pub(in crate::core) fn add_local_binding(
        &mut self,
        binding: LocalBinding<'db>,
        node: &RedNode,
    ) -> LocalBindingHandle {
        let binding_handle = self.store.local_bindings.add(binding);
        self.source_map.local_bindings.add(
            binding_handle,
            RedNodeTag::new(node).relative_to(self.anchor),
        );
        binding_handle
    }

    pub(in crate::core) fn add_type_expression(
        &mut self,
        type_expression: TypeExpression<'db>,
        node: &RedNode,
    ) -> TypeExpressionHandle {
        let type_expression_handle = self.store.type_expressions.add(type_expression);
        self.source_map.type_expressions.add(
            type_expression_handle,
            RedNodeTag::new(node).relative_to(self.anchor),
        );
        type_expression_handle
    }

    pub(in crate::core) fn add_expression_children(
        &mut self,
        child_handles: &[ExpressionHandle],
    ) -> Run<ExpressionHandle> {
        self.store.expression_children.add_many(child_handles)
    }

    pub(in crate::core) fn add_statement_children(
        &mut self,
        child_handles: &[StatementHandle],
    ) -> Run<StatementHandle> {
        self.store.statement_children.add_many(child_handles)
    }

    pub(in crate::core) fn add_type_expression_children(
        &mut self,
        child_handles: &[TypeExpressionHandle],
    ) -> Run<TypeExpressionHandle> {
        self.store.type_expression_children.add_many(child_handles)
    }

    pub(in crate::core) fn finish(self) -> (HirNodeStore<'db>, HirNodeSourceMap) {
        (self.store, self.source_map)
    }
}
