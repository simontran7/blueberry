use crate::core::semantic_analysis::ast_lowering::nodes::{
    self, HirNodeStore, TypeExpressionHandle,
};
use crate::core::semantic_analysis::type_checking::types::Ty;

pub(crate) fn lower_type_expression<'db>(
    db: &'db dyn crate::Db,
    store: &HirNodeStore<'db>,
    type_expression_handle: TypeExpressionHandle,
    on_unknown: &mut impl FnMut(TypeExpressionHandle),
) -> Ty<'db> {
    match store.type_expression(type_expression_handle) {
        nodes::TypeExpression::Path(path) => match path.segments(db) {
            [name] => Ty::to_primitive(db, name.text(db)),
            _ => None,
        }
        .unwrap_or_else(|| {
            on_unknown(type_expression_handle);
            Ty::error(db)
        }),
        nodes::TypeExpression::Tuple(element_run) => {
            let elements = store
                .type_expression_handles(*element_run)
                .iter()
                .map(|&element_handle| lower_type_expression(db, store, element_handle, on_unknown))
                .collect();
            Ty::tuple(db, elements)
        }
        // A syntax error, already reported by the parser.
        nodes::TypeExpression::Error => Ty::error(db),
    }
}
