pub(crate) mod body_lowerer;
pub(crate) mod dumper;
pub(crate) mod hir_node_store_builder;
pub(crate) mod nodes;

use crate::core::common::span::TextSize;
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::ast_lowering::hir_node_store_builder::HirNodeStoreBuilder;
use crate::core::semantic_analysis::ast_lowering::nodes::{HirNodeSourceMap, TypeExpressionHandle};
use crate::core::syntactic_analysis::cst::RedToken;
use crate::core::syntactic_analysis::cst::ast::{self, AstNode};

pub(crate) fn lower_function_signature<'db>(
    db: &'db dyn crate::Db,
    function: &ast::FunctionDefinition,
    anchor: TextSize,
) -> (nodes::FunctionSignature<'db>, HirNodeSourceMap) {
    let name = function.name().map(|token| lower_name(db, &token)).unwrap();

    let mut builder = HirNodeStoreBuilder::new(anchor);

    let parameter_annotation_handles = function
        .parameter_list()
        .into_iter()
        .flat_map(|list| list.parameters().collect::<Vec<_>>())
        .map(|parameter| match parameter.type_annotation() {
            Some(annotation) => lower_type_expression(db, &mut builder, &annotation),
            None => builder.add_type_expression(nodes::TypeExpression::Error, parameter.red()),
        })
        .collect();

    let return_type_annotation = function
        .return_type_annotation()
        .map(|annotation| lower_type_expression(db, &mut builder, &annotation));

    let (store, source_map) = builder.finish();
    let signature = nodes::FunctionSignature {
        name,
        store,
        parameter_annotation_handles,
        return_type_annotation,
    };
    (signature, source_map)
}

pub(crate) fn lower_constant_signature<'db>(
    db: &'db dyn crate::Db,
    constant: &ast::ConstantDefinition,
    anchor: TextSize,
) -> (nodes::ConstantSignature<'db>, HirNodeSourceMap) {
    let name = constant.name().map(|token| lower_name(db, &token)).unwrap();

    let mut builder = HirNodeStoreBuilder::new(anchor);

    let annotation = constant
        .type_annotation()
        .map(|annotation| lower_type_expression(db, &mut builder, &annotation));

    let (store, source_map) = builder.finish();
    let signature = nodes::ConstantSignature {
        name,
        store,
        annotation,
    };
    (signature, source_map)
}

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

pub(crate) fn lower_name<'db>(db: &'db dyn crate::Db, token: &RedToken) -> Symbol<'db> {
    Symbol::new(db, token.lexeme().to_string())
}

pub(crate) fn lower_path<'db>(
    db: &'db dyn crate::Db,
    path: &ast::Path,
) -> Option<nodes::Path<'db>> {
    let mut segments = Vec::new();
    let mut current = Some(path.clone());

    while let Some(path) = current {
        let name = path.segment()?.name()?;
        segments.push(lower_name(db, &name));
        current = path.qualifier();
    }

    segments.reverse();

    Some(nodes::Path::new(db, segments.into_boxed_slice()))
}
