use crate::core::common::handle_collections::handle_impl;
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};
use crate::core::common::segments::{Segment, SegmentList};
use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::nodes::{
    DefinitionBody, Expression, ExpressionHandle, LocalBindingHandle, Statement,
};
use crate::core::semantic_analysis::ids::keys::BlockKey;
use crate::core::semantic_analysis::name_resolution::resolver::Resolver;
use crate::core::source_file_key::SourceFileKey;

#[derive(Debug, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ScopeTree<'db> {
    file: SourceFileKey,
    enclosing_block: Option<BlockKey<'db>>,
    scopes: HandleMap<ScopeHandle, Scope<'db>>,
    scope_entries: SegmentList<ScopeEntry<'db>>,
    containing_scopes: SideHandleMap<ExpressionHandle, ScopeHandle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, salsa::SalsaValue)]
pub(crate) struct ScopeEntry<'db> {
    pub(crate) name: Symbol<'db>,
    pub(crate) binding_handle: LocalBindingHandle,
}

handle_impl!(pub(crate) ScopeHandle);

#[derive(Clone, Copy)]
pub(crate) struct ScopeView<'a, 'db> {
    store: &'a ScopeTree<'db>,
    scope_handle: ScopeHandle,
}

pub(crate) struct ScopeViewMut<'a, 'db> {
    store: &'a mut ScopeTree<'db>,
    scope_handle: ScopeHandle,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::SalsaValue)]
struct Scope<'db> {
    parent_handle: Option<ScopeHandle>,
    block_key: Option<BlockKey<'db>>,
    entries: Segment<ScopeEntry<'db>>,
}

impl<'db> ScopeTree<'db> {
    pub(crate) fn new(
        body: &DefinitionBody<'db>,
        file: SourceFileKey,
        enclosing_block: Option<BlockKey<'db>>,
    ) -> Self {
        let mut scope_tree = Self {
            file,
            enclosing_block,
            scopes: HandleMap::new(),
            scope_entries: SegmentList::new(),
            containing_scopes: SideHandleMap::new(),
        };

        let root_handle = scope_tree.scopes.add(Scope {
            parent_handle: None,
            block_key: None,
            entries: Segment::empty(),
        });
        scope_tree
            .get_scope_mut(root_handle)
            .add_entries(body, &body.binding_children[body.parameter_segment]);

        scope_tree.visit_expression(body, body.root_handle, root_handle);

        scope_tree
    }

    pub(crate) fn containing_scope(
        &self,
        expression_handle: ExpressionHandle,
    ) -> ScopeView<'_, 'db> {
        self.get_scope(self.containing_scopes[expression_handle])
    }

    pub(crate) fn resolver_for(&self, expression_handle: ExpressionHandle) -> Resolver<'_, 'db> {
        Resolver {
            file: self.file,
            scope_view: self.containing_scope(expression_handle),
            enclosing_block: self.enclosing_block,
        }
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
        self.containing_scopes.add(expression_handle, parent_handle);

        match &body.expressions[expression_handle] {
            Expression::Unit
            | Expression::Integer(_)
            | Expression::Boolean(_)
            | Expression::Path(_)
            | Expression::Error
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
                statement_segment,
                tail_handle,
            } => {
                let mut parent_handle = self.scopes.add(Scope {
                    parent_handle: Some(parent_handle),
                    block_key: *block_key,
                    entries: Segment::empty(),
                });

                for statement_handle in &body.statement_children[*statement_segment] {
                    match &body.statements[*statement_handle] {
                        Statement::Let {
                            name_handle,
                            initializer_handle,
                            ..
                        } => {
                            self.visit_expression(body, *initializer_handle, parent_handle);
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
                argument_segment,
            } => {
                self.visit_expression(body, *callee_handle, parent_handle);
                for argument_handle in &body.expression_children[*argument_segment] {
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
    pub(crate) fn block(&self) -> Option<BlockKey<'db>> {
        self.store.scopes[self.scope_handle].block_key
    }

    pub(crate) fn entries(&self) -> &'a [ScopeEntry<'db>] {
        let segment = self.store.scopes[self.scope_handle].entries;
        &self.store.scope_entries[segment]
    }

    pub(crate) fn parent(&self) -> Option<Self> {
        self.store.scopes[self.scope_handle]
            .parent_handle
            .map(|parent_handle| self.store.get_scope(parent_handle))
    }

    pub(crate) fn chain(self) -> impl Iterator<Item = Self> {
        std::iter::successors(Some(self), Self::parent)
    }

    pub(crate) fn get_local_binding(self, name: Symbol<'db>) -> Option<LocalBindingHandle> {
        self.chain().find_map(|scope_view| {
            scope_view
                .entries()
                .iter()
                .rev()
                .find(|entry| entry.name == name)
                .map(|entry| entry.binding_handle)
        })
    }
}

impl<'db> ScopeViewMut<'_, 'db> {
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
