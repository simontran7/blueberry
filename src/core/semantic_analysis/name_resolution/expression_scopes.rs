use crate::core::common::handle_collections::handle_impl;
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::common::segments::{Segment, SegmentList};
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::{
    BlockKey, DefinitionBody, Expression, ExpressionHandle, LocalBindingHandle, Statement,
};

/// Tree of scopes for one function or constant body.
#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ExpressionScopes<'db> {
    scopes: HandleMap<ScopeHandle, Scope<'db>>,
    scope_entries: SegmentList<ScopeEntry<'db>>,
    scope_by_expression: SideHandleMap<ExpressionHandle, ScopeHandle>,
}

/// Node in the scope tree
#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
struct Scope<'db> {
    parent_handle: Option<ScopeHandle>,
    block_key: Option<BlockKey<'db>>,
    entries: Segment<ScopeEntry<'db>>,
}

/// A single name visible in a scope, and the binding it currently refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ScopeEntry<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) binding_handle: LocalBindingHandle,
}

handle_impl!(pub(crate) ScopeHandle);

#[derive(Clone, Copy)]
pub(crate) struct ScopeView<'a, 'db> {
    store: &'a ExpressionScopes<'db>,
    scope_handle: ScopeHandle,
}

pub(crate) struct ScopeViewMut<'a, 'db> {
    store: &'a mut ExpressionScopes<'db>,
    scope_handle: ScopeHandle,
}

impl<'db> ExpressionScopes<'db> {
    /// Builds the scope tree for a body, starting from its parameters.
    pub(crate) fn new(body: &DefinitionBody<'db>) -> Self {
        let mut expression_scopes = Self {
            scopes: HandleMap::new(),
            scope_entries: SegmentList::new(),
            scope_by_expression: SideHandleMap::new(),
        };

        let root_handle = expression_scopes.scopes.add(Scope {
            parent_handle: None,
            block_key: None,
            entries: Segment::empty(),
        });
        expression_scopes
            .get_scope_mut(root_handle)
            .add_entries(body, &body.binding_children[body.parameters]);

        expression_scopes.visit_expression(body, body.root, root_handle);

        expression_scopes
    }

    /// Returns a view of the scope an expression was computed under, if any.
    pub(crate) fn containing_scope(
        &self,
        expression_handle: ExpressionHandle,
    ) -> Option<ScopeView<'_, 'db>> {
        self.scope_by_expression
            .get(expression_handle)
            .copied()
            .map(|scope_handle| self.get_scope(scope_handle))
    }

    fn get_scope(&self, scope_handle: ScopeHandle) -> ScopeView<'_, 'db> {
        ScopeView {
            store: self,
            scope_handle,
        }
    }

    fn get_scope_mut(&mut self, scope_handle: ScopeHandle) -> ScopeViewMut<'_, 'db> {
        ScopeViewMut {
            store: self,
            scope_handle,
        }
    }

    fn visit_expression(
        &mut self,
        body: &DefinitionBody<'db>,
        expression_handle: ExpressionHandle,
        parent_handle: ScopeHandle,
    ) {
        self.scope_by_expression
            .add(expression_handle, parent_handle);

        match &body.expressions[expression_handle] {
            Expression::Unit
            | Expression::Integer(_)
            | Expression::Boolean(_)
            | Expression::Path(_)
            | Expression::Hole
            | Expression::Continue => {}
            Expression::If {
                condition_handle,
                then_branch_handle,
                else_branch_handle,
            } => {
                self.visit_expression(body, *condition_handle, parent_handle);
                self.visit_expression(body, *then_branch_handle, parent_handle);
                if let Some(else_branch_handle) = else_branch_handle {
                    self.visit_expression(body, *else_branch_handle, parent_handle);
                }
            }
            Expression::Block {
                block_key,
                statements,
                tail_handle,
            } => {
                let mut parent_handle = self.scopes.add(Scope {
                    parent_handle: Some(parent_handle),
                    block_key: *block_key,
                    entries: Segment::empty(),
                });

                for statement in &body.statement_children[*statements] {
                    match &body.statements[*statement] {
                        Statement::Let {
                            name_handle,
                            initializer_handle,
                            ..
                        } => {
                            if let Some(initializer_handle) = initializer_handle {
                                self.visit_expression(body, *initializer_handle, parent_handle);
                            }
                            parent_handle = self.scopes.add(Scope {
                                parent_handle: Some(parent_handle),
                                block_key: None,
                                entries: Segment::empty(),
                            });
                            self.get_scope_mut(parent_handle)
                                .add_entries(body, &[*name_handle]);
                        }
                        Statement::Expression {
                            expression_handle, ..
                        } => {
                            self.visit_expression(body, *expression_handle, parent_handle);
                        }
                        Statement::Definition => {}
                    }
                }

                if let Some(tail_handle) = tail_handle {
                    self.visit_expression(body, *tail_handle, parent_handle);
                }
            }
            Expression::Loop {
                body_handle: loop_body_handle,
                ..
            } => self.visit_expression(body, *loop_body_handle, parent_handle),
            Expression::Call {
                callee_handle,
                arguments,
            } => {
                self.visit_expression(body, *callee_handle, parent_handle);
                for argument_handle in &body.expression_children[*arguments] {
                    self.visit_expression(body, *argument_handle, parent_handle);
                }
            }
            Expression::Break { value_handle } | Expression::Return { value_handle } => {
                if let Some(value_handle) = value_handle {
                    self.visit_expression(body, *value_handle, parent_handle);
                }
            }
            Expression::UnaryOperation { operand_handle, .. } => {
                self.visit_expression(body, *operand_handle, parent_handle)
            }
            Expression::BinaryOperation {
                lhs_handle,
                rhs_handle,
                ..
            } => {
                self.visit_expression(body, *lhs_handle, parent_handle);
                self.visit_expression(body, *rhs_handle, parent_handle);
            }
            Expression::Assignment {
                target_handle,
                value_handle,
            } => {
                self.visit_expression(body, *target_handle, parent_handle);
                self.visit_expression(body, *value_handle, parent_handle);
            }
        }
    }
}

impl<'a, 'db> ScopeView<'a, 'db> {
    /// Returns the block this scope corresponds to, if it's a block's own initial scope.
    pub(crate) fn block(&self) -> Option<BlockKey<'db>> {
        self.store.scopes[self.scope_handle].block_key
    }

    /// Returns the entries declared directly in this scope, not its ancestors.
    pub(crate) fn entries(&self) -> &'a [ScopeEntry<'db>] {
        let segment = self.store.scopes[self.scope_handle].entries;
        &self.store.scope_entries[segment]
    }

    /// Returns a view of this scope's parent, if it has one.
    pub(crate) fn parent(&self) -> Option<Self> {
        self.store.scopes[self.scope_handle]
            .parent_handle
            .map(|parent_handle| self.store.get_scope(parent_handle))
    }

    /// Walks from this scope up through its ancestors to the root.
    pub(crate) fn chain(self) -> impl Iterator<Item = Self> {
        std::iter::successors(Some(self), Self::parent)
    }

    /// Resolves a name to the local binding it refers to, searching outward from this scope.
    pub(crate) fn get_local_binding(self, name: Symbol<'db>) -> Option<LocalBindingHandle> {
        self.chain().find_map(|scope| {
            scope
                .entries()
                .iter()
                .rev()
                .find(|entry| entry.name == name)
                .map(|entry| entry.binding_handle)
        })
    }
}

impl<'db> ScopeViewMut<'_, 'db> {
    /// Sets this scope's entries, given the local bindings it declares.
    pub(crate) fn add_entries(
        &mut self,
        body: &DefinitionBody<'db>,
        binding_handles: &[LocalBindingHandle],
    ) {
        let entries: Vec<ScopeEntry<'db>> = binding_handles
            .iter()
            .map(|binding_handle| ScopeEntry {
                name: body.local_bindings[*binding_handle].name,
                binding_handle: *binding_handle,
            })
            .collect();
        self.store.scopes[self.scope_handle].entries = self.store.scope_entries.add_many(&entries);
    }
}
