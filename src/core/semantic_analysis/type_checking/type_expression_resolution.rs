use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, HirNodeStore, TypeExpressionHandle,
};
use crate::core::semantic_analysis::type_checking::types::Ty;

/// What a type expression resolves to.
pub(crate) struct ResolvedTypeExpression<'db> {
    /// the type it names
    pub(crate) ty: Ty<'db>,
    /// the paths that name no type
    pub(crate) unknown_paths: Vec<UnknownPath<'db>>,
}

/// A path in a type expression that names no type.
pub(crate) struct UnknownPath<'db> {
    pub(crate) handle: TypeExpressionHandle,
    pub(crate) path: nodes::Path<'db>,
}

/// Resolves `type_expression_handle` to the type it names.
pub(crate) fn resolve_type_expression<'db>(
    db: &'db dyn crate::Db,
    store: &HirNodeStore<'db>,
    type_expression_handle: TypeExpressionHandle,
) -> ResolvedTypeExpression<'db> {
    match store.type_expression(type_expression_handle) {
        nodes::TypeExpression::Path(path) => match path.segments(db) {
            // the only types so far are primitives, which are never qualified
            [name] if let Some(ty) = Ty::to_primitive(db, name.text(db)) => {
                ResolvedTypeExpression {
                    ty,
                    unknown_paths: Vec::new(),
                }
            }
            _ => ResolvedTypeExpression {
                ty: Ty::error(db),
                unknown_paths: vec![UnknownPath {
                    handle: type_expression_handle,
                    path: *path,
                }],
            },
        },
        nodes::TypeExpression::Tuple(element_run) => {
            let element_handles = store.type_expression_handles(*element_run);
            let mut elements = Vec::with_capacity(element_handles.len());
            let mut unknown_paths = Vec::new();

            for &element_handle in element_handles {
                let element = resolve_type_expression(db, store, element_handle);
                elements.push(element.ty);
                unknown_paths.extend(element.unknown_paths);
            }

            ResolvedTypeExpression {
                ty: Ty::tuple(db, elements.into_boxed_slice()),
                unknown_paths,
            }
        }
        // no need to emit a diagnostic since it must be a syntax error that's already reported by
        // the parser
        nodes::TypeExpression::Error => ResolvedTypeExpression {
            ty: Ty::error(db),
            unknown_paths: Vec::new(),
        },
    }
}
