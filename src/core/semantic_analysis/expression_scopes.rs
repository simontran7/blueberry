use crate::core::common::handle_collections::handle_impl;
use crate::core::common::handle_collections::handle_list::{AppendOnlyHandleList, HandleRange};
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::{
    BlockKey, DefinitionBody, Expression, ExpressionHandle, LocalBindingHandle, Statement,
};

handle_impl!(pub(crate) ScopeHandle);

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ExpressionScopes<'db> {
    scopes: HandleMap<ScopeHandle, ScopeData<'db>>,
    entries: AppendOnlyHandleList<ScopeEntry<'db>>,
    scope_by_expression: SideHandleMap<ExpressionHandle, ScopeHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
struct ScopeData<'db> {
    parent: Option<ScopeHandle>,
    block: Option<BlockKey<'db>>,
    entries: HandleRange<ScopeEntry<'db>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ScopeEntry<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) binding: LocalBindingHandle,
}

impl<'db> ExpressionScopes<'db> {
    pub(crate) fn new(body: &DefinitionBody<'db>) -> Self {
        let mut scopes = Self {
            scopes: HandleMap::new(),
            entries: AppendOnlyHandleList::new(),
            scope_by_expression: SideHandleMap::new(),
        };
        let root = scopes.new_scope(None, None);
        scopes.add_entries(root, body, &body.binding_children[body.parameters]);
        scopes.compute(body, body.root, root);
        scopes
    }

    pub(crate) fn scope_for(&self, expression: ExpressionHandle) -> Option<ScopeHandle> {
        self.scope_by_expression.get(expression).copied()
    }

    pub(crate) fn block_of(&self, scope: ScopeHandle) -> Option<BlockKey<'db>> {
        self.scopes[scope].block
    }

    pub(crate) fn scope_chain(&self, from: ScopeHandle) -> impl Iterator<Item = ScopeHandle> + '_ {
        std::iter::successors(Some(from), |scope| self.scopes[*scope].parent)
    }

    pub(crate) fn entries(&self, scope: ScopeHandle) -> &[ScopeEntry<'db>] {
        &self.entries[self.scopes[scope].entries]
    }

    pub(crate) fn resolve_local(
        &self,
        from: ScopeHandle,
        name: Symbol<'db>,
    ) -> Option<LocalBindingHandle> {
        self.scope_chain(from).find_map(|scope| {
            self.entries(scope)
                .iter()
                .rev()
                .find(|entry| entry.name == name)
                .map(|entry| entry.binding)
        })
    }

    fn new_scope(
        &mut self,
        parent: Option<ScopeHandle>,
        block: Option<BlockKey<'db>>,
    ) -> ScopeHandle {
        self.scopes.add(ScopeData {
            parent,
            block,
            entries: HandleRange::empty(),
        })
    }

    fn add_entries(
        &mut self,
        scope: ScopeHandle,
        body: &DefinitionBody<'db>,
        bindings: &[LocalBindingHandle],
    ) {
        let entries: Vec<ScopeEntry<'db>> = bindings
            .iter()
            .map(|binding| ScopeEntry {
                name: body.local_bindings[*binding].name,
                binding: *binding,
            })
            .collect();
        self.scopes[scope].entries = self.entries.add_many(&entries);
    }

    fn compute(&mut self, body: &DefinitionBody<'db>, expression: ExpressionHandle, scope: ScopeHandle) {
        self.scope_by_expression.add(expression, scope);
        match &body.expressions[expression] {
            Expression::Unit
            | Expression::Integer(_)
            | Expression::Boolean(_)
            | Expression::Path(_)
            | Expression::Hole
            | Expression::Continue => {}
            Expression::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compute(body, *condition, scope);
                self.compute(body, *then_branch, scope);
                if let Some(else_branch) = else_branch {
                    self.compute(body, *else_branch, scope);
                }
            }
            Expression::Block {
                key,
                statements,
                tail,
            } => {
                let mut scope = self.new_scope(Some(scope), *key);
                for statement in &body.statement_children[*statements] {
                    match &body.statements[*statement] {
                        Statement::Let {
                            name, initializer, ..
                        } => {
                            if let Some(initializer) = initializer {
                                self.compute(body, *initializer, scope);
                            }
                            scope = self.new_scope(Some(scope), None);
                            self.add_entries(scope, body, &[*name]);
                        }
                        Statement::Expression { expression, .. } => {
                            self.compute(body, *expression, scope);
                        }
                        Statement::Definition => {}
                    }
                }
                if let Some(tail) = tail {
                    self.compute(body, *tail, scope);
                }
            }
            Expression::Loop { body: loop_body, .. } => self.compute(body, *loop_body, scope),
            Expression::Call { callee, arguments } => {
                self.compute(body, *callee, scope);
                for argument in &body.expression_children[*arguments] {
                    self.compute(body, *argument, scope);
                }
            }
            Expression::Break { value } | Expression::Return { value } => {
                if let Some(value) = value {
                    self.compute(body, *value, scope);
                }
            }
            Expression::UnaryOperation { operand, .. } => self.compute(body, *operand, scope),
            Expression::BinaryOperation { lhs, rhs, .. } => {
                self.compute(body, *lhs, scope);
                self.compute(body, *rhs, scope);
            }
            Expression::Assignment { target, value } => {
                self.compute(body, *target, scope);
                self.compute(body, *value, scope);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::common::handle_collections::Handle;
    use crate::core::db::BlueberryDatabase;
    use crate::core::semantic_analysis::definition_tree::Definition;
    use crate::core::semantic_analysis::{function_body_of, function_scopes_of, top_level_definitions_of};
    use crate::core::source_file_key::SourceFileKey;

    /// For every `Path` in the first function, the index of the local binding it resolves to.
    fn resolved_paths(source: &str) -> Vec<Option<usize>> {
        let db = BlueberryDatabase::default();
        let file = SourceFileKey::new(&db, PathBuf::from("test.bb"), source.to_string());
        let definitions = top_level_definitions_of(&db, file);
        let Definition::Function(key) = definitions.definitions()[0] else {
            panic!("expected a function");
        };
        let body = function_body_of(&db, key);
        let scopes = function_scopes_of(&db, key);
        body.expressions
            .iter()
            .filter_map(|(handle, expression)| match expression {
                Expression::Path(path) => {
                    let scope = scopes.scope_for(handle).expect("every expression has a scope");
                    Some(match path.segments.as_slice() {
                        [name] => scopes
                            .resolve_local(scope, *name)
                            .map(|binding| binding.index()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn let_shadows_parameter_but_not_its_own_initializer() {
        assert_eq!(
            resolved_paths("func f(a: I32) -> I32 { let b = a; let a = b; a }"),
            [Some(0), Some(1), Some(2)]
        );
    }

    #[test]
    fn let_bindings_do_not_leak_out_of_their_block() {
        assert_eq!(
            resolved_paths("func g() { { let x = 1; } x }"),
            [None]
        );
    }

    #[test]
    fn inner_blocks_see_outer_bindings() {
        assert_eq!(resolved_paths("func h(p: I32) { { p } }"), [Some(0)]);
    }
}
