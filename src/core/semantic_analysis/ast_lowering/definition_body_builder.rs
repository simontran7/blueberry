use crate::core::common::handle_collections::handle_map::HandleMap;
use crate::core::common::segments::{Segment, SegmentList};
use crate::core::semantic_analysis::hir::nodes::{
    DefinitionBody, DefinitionBodySourceMap, Expression, ExpressionHandle, LocalBinding,
    LocalBindingHandle, Statement, StatementHandle, TypeAnnotation, TypeAnnotationHandle,
};
use crate::core::semantic_analysis::red_node_directory::RedNodeTag;
use crate::core::syntactic_analysis::cst::RedNode;

#[derive(Default)]
pub(crate) struct DefinitionBodyBuilder<'db> {
    expressions: HandleMap<ExpressionHandle, Expression<'db>>,
    statements: HandleMap<StatementHandle, Statement>,
    local_bindings: HandleMap<LocalBindingHandle, LocalBinding<'db>>,
    type_annotations: HandleMap<TypeAnnotationHandle, TypeAnnotation<'db>>,
    binding_children: SegmentList<LocalBindingHandle>,
    expression_children: SegmentList<ExpressionHandle>,
    statement_children: SegmentList<StatementHandle>,
    source_map: DefinitionBodySourceMap,
}

impl<'db> DefinitionBodyBuilder<'db> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_expression(
        &mut self,
        expression: Expression<'db>,
        node: &RedNode,
    ) -> ExpressionHandle {
        let handle = self.expressions.add(expression);
        self.source_map
            .expressions
            .add(handle, RedNodeTag::new(node));
        handle
    }

    pub(crate) fn add_statement(
        &mut self,
        statement: Statement,
        node: &RedNode,
    ) -> StatementHandle {
        let handle = self.statements.add(statement);
        self.source_map
            .statements
            .add(handle, RedNodeTag::new(node));
        handle
    }

    pub(crate) fn add_local_binding(
        &mut self,
        binding: LocalBinding<'db>,
        node: &RedNode,
    ) -> LocalBindingHandle {
        let handle = self.local_bindings.add(binding);
        self.source_map
            .local_bindings
            .add(handle, RedNodeTag::new(node));
        handle
    }

    pub(crate) fn add_type_annotation(
        &mut self,
        annotation: TypeAnnotation<'db>,
        node: &RedNode,
    ) -> TypeAnnotationHandle {
        let handle = self.type_annotations.add(annotation);
        self.source_map
            .type_annotations
            .add(handle, RedNodeTag::new(node));
        handle
    }

    pub(crate) fn add_binding_children(
        &mut self,
        children: &[LocalBindingHandle],
    ) -> Segment<LocalBindingHandle> {
        self.binding_children.add_many(children)
    }

    pub(crate) fn add_expression_children(
        &mut self,
        children: &[ExpressionHandle],
    ) -> Segment<ExpressionHandle> {
        self.expression_children.add_many(children)
    }

    pub(crate) fn add_statement_children(
        &mut self,
        children: &[StatementHandle],
    ) -> Segment<StatementHandle> {
        self.statement_children.add_many(children)
    }

    pub(crate) fn finish(
        self,
        root: ExpressionHandle,
        parameters: Segment<LocalBindingHandle>,
    ) -> (DefinitionBody<'db>, DefinitionBodySourceMap) {
        let body = DefinitionBody {
            root,
            parameters,
            expressions: self.expressions,
            statements: self.statements,
            local_bindings: self.local_bindings,
            type_annotations: self.type_annotations,
            binding_children: self.binding_children,
            expression_children: self.expression_children,
            statement_children: self.statement_children,
        };
        (body, self.source_map)
    }
}
