use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::semantic_analysis::type_checking::types::Ty;
use crate::core::semantic_analysis::hir::nodes::DefinitionBody;
use crate::core::semantic_analysis::hir::nodes::{
    Expression, ExpressionHandle, LocalBindingHandle,
};

pub(crate) struct TypeChecker<'a, 'db> {
    db: &'db dyn crate::Db,
    body: &'a DefinitionBody<'db>,
    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct TypeCheckResult<'db> {
    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
}

impl<'a, 'db> TypeChecker<'a, 'db> {
    pub(crate) fn new(db: &'db dyn crate::Db, body: &'a DefinitionBody<'db>) -> Self {
        Self {
            db,
            body,
            expressions: SideHandleMap::new(),
            local_bindings: SideHandleMap::new(),
        }
    }

    pub(crate) fn infer(&mut self, expression: ExpressionHandle) -> Ty<'db> {
        let ty = match &self.body.expressions[expression] {
            Expression::Unit => Ty::unit(self.db),
            Expression::Boolean(_) => Ty::bool(self.db),
            _ => Ty::error(self.db),
        };
        self.expressions.add(expression, ty);
        ty
    }

    pub(crate) fn finish(self) -> TypeCheckResult<'db> {
        TypeCheckResult {
            expressions: self.expressions,
            local_bindings: self.local_bindings,
        }
    }
}

impl<'db> TypeCheckResult<'db> {
    pub(crate) fn expression_type(&self, expression: ExpressionHandle) -> Ty<'db> {
        self.expressions[expression]
    }

    pub(crate) fn local_binding_type(&self, binding: LocalBindingHandle) -> Ty<'db> {
        self.local_bindings[binding]
    }
}
