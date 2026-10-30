use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::hir_node_store_builder::HirNodeStoreBuilder;
use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, HirNodeSourceMap, TypeExpressionHandle,
};
use crate::core::semantic_analysis::ast_lowering::{path_lowering, type_expression_lowering};
use crate::core::semantic_analysis::name_resolution::definition_list::Definition;
use crate::core::syntactic_analysis::cst::RedToken;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};

pub(crate) struct SignatureLowerer<'db> {
    db: &'db dyn crate::Db,
    builder: HirNodeStoreBuilder<'db>,
}

impl<'db> SignatureLowerer<'db> {
    pub(crate) fn new(db: &'db dyn crate::Db, owner: Definition<'db>) -> Self {
        Self {
            db,
            builder: HirNodeStoreBuilder::new(owner.anchor(db)),
        }
    }

    pub(crate) fn lower_function(
        mut self,
        function: &ast::FunctionDefinition,
    ) -> (nodes::Signature<'db>, HirNodeSourceMap) {
        let name = self.lower_name(function.name());

        let parameter_annotation_handles = function
            .parameter_list()
            .into_iter()
            .flat_map(|list| list.parameters().collect::<Vec<_>>())
            .map(|parameter| match parameter.type_annotation() {
                Some(annotation) => self.lower_type_expression(&annotation),
                None => self
                    .builder
                    .add_type_expression(nodes::TypeExpression::Error, parameter.red()),
            })
            .collect();

        let return_type_annotation = function
            .return_type_annotation()
            .map(|annotation| self.lower_type_expression(&annotation));

        let (store, source_map) = self.builder.finish();
        let signature = nodes::Signature {
            name,
            store,
            kind: nodes::SignatureKind::Function(nodes::FunctionSignature {
                parameter_annotation_handles,
                return_type_annotation,
            }),
        };
        (signature, source_map)
    }

    pub(crate) fn lower_constant(
        mut self,
        constant: &ast::ConstantDefinition,
    ) -> (nodes::Signature<'db>, HirNodeSourceMap) {
        let name = self.lower_name(constant.name());

        let annotation = constant
            .type_annotation()
            .map(|annotation| self.lower_type_expression(&annotation));

        let (store, source_map) = self.builder.finish();
        let signature = nodes::Signature {
            name,
            store,
            kind: nodes::SignatureKind::Constant(nodes::ConstantSignature { annotation }),
        };
        (signature, source_map)
    }

    fn lower_name(&self, name: Option<RedToken>) -> Symbol<'db> {
        let Some(name) = name else {
            unreachable!("a definition without a name is never collected, so it's never lowered");
        };
        path_lowering::lower_name(self.db, &name)
    }

    fn lower_type_expression(
        &mut self,
        type_expression: &ast::TypeExpression,
    ) -> TypeExpressionHandle {
        type_expression_lowering::lower_type_expression(self.db, &mut self.builder, type_expression)
    }
}
