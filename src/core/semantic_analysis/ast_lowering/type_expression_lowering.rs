use crate::core::semantic_analysis::ast_lowering::hir_node_store_builder::HirNodeStoreBuilder;
use crate::core::semantic_analysis::ast_lowering::nodes::{self, TypeExpressionHandle};
use crate::core::semantic_analysis::ast_lowering::path_lowering::lower_path;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};

pub(crate) fn lower_type_expression<'db>(
    db: &'db dyn crate::Db,
    builder: &mut HirNodeStoreBuilder<'db>,
    type_expression: &ast::TypeExpression,
) -> TypeExpressionHandle {
    let lowered = match type_expression {
        ast::TypeExpression::PathType(path_type) => path_type
            .path()
            .and_then(|path| lower_path(db, &path))
            .map_or(nodes::TypeExpression::Error, nodes::TypeExpression::Path),
        ast::TypeExpression::TupleType(tuple) => {
            let element_handles: Vec<TypeExpressionHandle> = tuple
                .elements()
                .map(|element| lower_type_expression(db, builder, &element))
                .collect();
            nodes::TypeExpression::Tuple(builder.add_type_expression_children(&element_handles))
        }
        ast::TypeExpression::ParenthesizedType(parenthesized) => match parenthesized.inner() {
            Some(inner) => return lower_type_expression(db, builder, &inner),
            None => nodes::TypeExpression::Error,
        },
    };
    builder.add_type_expression(lowered, type_expression.red())
}
