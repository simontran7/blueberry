use std::collections::HashMap;

use crate::core::common::symbol::Symbol;
use crate::core::semantic_analysis::hir::{BindingId, BindingKind};

#[derive(Clone)]
pub(crate) struct SymbolTable<'db> {
    scopes: Vec<Scope<'db>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Scope<'db> {
    pub(crate) kind: ScopeKind,
    pub(crate) bindings: HashMap<Symbol<'db>, BindingId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ScopeKind {
    Normal,
    FunctionBoundary,
    ConstantBoundary,
}

#[derive(Debug)]
pub(crate) enum LookupError {
    NotFound,
    BlockedByBoundary(ScopeKind),
}

#[derive(Debug)]
pub(crate) enum DefineError {
    AlreadyDefined { previous_binding_id: BindingId },
}

impl<'db> SymbolTable<'db> {
    pub(crate) const fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub(crate) fn enter_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope {
            kind,
            bindings: HashMap::new(),
        });
    }

    pub(crate) fn exit_scope(&mut self) {
        self.scopes.pop().unwrap();
    }

    pub(crate) fn add_binding(
        &mut self,
        name: Symbol<'db>,
        binding_id: BindingId,
    ) -> Result<(), DefineError> {
        let scope = self.scopes.last_mut().unwrap();
        if let Some(&previous_binding_id) = scope.bindings.get(&name) {
            return Err(DefineError::AlreadyDefined { previous_binding_id });
        }
        scope.bindings.insert(name, binding_id);
        Ok(())
    }

    pub(crate) fn find_binding(&self, name: Symbol<'db>) -> Result<BindingId, LookupError> {
        let mut boundary = None;
        for scope in self.scopes.iter().rev() {
            if let Some(&binding) = scope.bindings.get(&name) {
                match binding.kind() {
                    BindingKind::Definition => return Ok(binding),
                    BindingKind::Local if boundary.is_none() => return Ok(binding),
                    BindingKind::Local => {
                        return Err(LookupError::BlockedByBoundary(boundary.unwrap()));
                    }
                }
            }
            // set the new boundary when we cross the *first* constant or the *first* function boundary
            if boundary.is_none()
                && matches!(
                    scope.kind,
                    ScopeKind::FunctionBoundary | ScopeKind::ConstantBoundary
                )
            {
                boundary = Some(scope.kind);
            }
        }
        Err(LookupError::NotFound)
    }
}
