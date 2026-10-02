use crate::core::common::handle_collections::handle_map::SideHandleMap;
use crate::core::semantic_analysis::hir::nodes::DefinitionBody;
use crate::core::semantic_analysis::hir::nodes::{
    Expression, ExpressionHandle, LocalBindingHandle,
};
use crate::core::semantic_analysis::name_resolution::resolver::Resolver;
use crate::core::semantic_analysis::name_resolution::scope_tree::ScopeTree;
use crate::core::semantic_analysis::type_checking::inference_table::{
    InferenceTable, TypeMismatch,
};
use crate::core::semantic_analysis::type_checking::types::{InferenceVariable, Ty, TyKind};
use crate::core::source_file_key::SourceFileKey;

/// Type checks one body (a function's block or a constant's initializer).
pub(crate) struct BodyTypeChecker<'a, 'db> {
    db: &'db dyn crate::Db,
    body: &'a DefinitionBody<'db>,
    scopes: &'a ScopeTree<'db>,
    file: SourceFileKey,
    table: InferenceTable<'db>,
    return_type: Option<Ty<'db>>,
    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
    mismatches: SideHandleMap<ExpressionHandle, TypeMismatch<'db>>,
}

#[derive(Clone, Copy)]
enum Expectation<'db> {
    None,
    HasType(Ty<'db>),
}

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct BodyTypeCheckResult<'db> {
    expressions: SideHandleMap<ExpressionHandle, Ty<'db>>,
    local_bindings: SideHandleMap<LocalBindingHandle, Ty<'db>>,
    mismatches: SideHandleMap<ExpressionHandle, TypeMismatch<'db>>,
}

impl<'a, 'db> BodyTypeChecker<'a, 'db> {
    pub(crate) fn new(
        db: &'db dyn crate::Db,
        body: &'a DefinitionBody<'db>,
        scopes: &'a ScopeTree<'db>,
        file: SourceFileKey,
    ) -> Self {
        Self {
            db,
            body,
            scopes,
            file,
            table: InferenceTable::new(db),
            return_type: None,
            expressions: SideHandleMap::new(),
            local_bindings: SideHandleMap::new(),
            mismatches: SideHandleMap::new(),
        }
    }

    /// Records what a function's signature promises before its body is walked.
    /// Each parameter gets its declared type, and the return type is
    /// remembered for `return` expressions.
    pub(crate) fn seed_function_signature(&mut self, signature_type: Ty<'db>) {
        let TyKind::Function {
            parameters,
            r#return,
        } = signature_type.kind(self.db)
        else {
            unreachable!("a function's signature type is always a function type");
        };

        let bindings = &self.body.binding_children[self.body.parameters];
        for (&binding, &ty) in bindings.iter().zip(parameters) {
            self.local_bindings.add(binding, ty);
        }

        self.return_type = Some(*r#return);
    }

    pub(crate) fn seed_constant_signature(&mut self, declared_type: Ty<'db>) {
        self.return_type = Some(declared_type);
    }

    pub(crate) fn check_body(&mut self) {
        let expectation = match self.return_type {
            Some(ty) => Expectation::HasType(ty),
            None => Expectation::None,
        };
        self.infer_expression(self.body.root, expectation);
    }

    fn infer_expression(
        &mut self,
        expression: ExpressionHandle,
        expectation: Expectation<'db>,
    ) -> Ty<'db> {
        let ty = match &self.body.expressions[expression] {
            Expression::Unit => Ty::unit(self.db),
            Expression::Boolean(_) => Ty::bool(self.db),
            Expression::Integer(_) => self.infer_integer_literal(expectation),
            _ => Ty::error(self.db),
        };
        if let Expectation::HasType(expected) = expectation
            && let Err(mismatch) = self.table.unify(expected, ty)
        {
            self.mismatches.add(expression, mismatch);
        }
        self.expressions.add(expression, ty);
        ty
    }

    /// An integer literal takes the expected type directly when it's already a concrete
    /// integer type; otherwise it gets a fresh integer variable.
    fn infer_integer_literal(&mut self, expectation: Expectation<'db>) -> Ty<'db> {
        if let Expectation::HasType(expected) = expectation {
            let expected = self.table.shallow_resolve(expected);
            if let TyKind::Signed(_) | TyKind::Unsigned(_) = expected.kind(self.db) {
                return expected;
            }
        }
        let variable = self.table.make_integer_inference_variable();
        Ty::new(
            self.db,
            TyKind::InferenceVariable(InferenceVariable::Integer(variable)),
        )
    }

    fn resolver_for(&self, expression: ExpressionHandle) -> Option<Resolver<'a, 'db>> {
        let scope = self.scopes.containing_scope(expression)?;
        Some(Resolver {
            file: self.file,
            scope,
        })
    }

    pub(crate) fn finish(self) -> BodyTypeCheckResult<'db> {
        BodyTypeCheckResult {
            expressions: self.expressions,
            local_bindings: self.local_bindings,
            mismatches: self.mismatches,
        }
    }
}

impl<'db> BodyTypeCheckResult<'db> {
    pub(crate) fn expression_type(&self, expression: ExpressionHandle) -> Ty<'db> {
        self.expressions[expression]
    }

    pub(crate) fn local_binding_type(&self, binding: LocalBindingHandle) -> Ty<'db> {
        self.local_bindings[binding]
    }

    pub(crate) fn mismatch(&self, expression: ExpressionHandle) -> Option<&TypeMismatch<'db>> {
        self.mismatches.get(expression)
    }
}
