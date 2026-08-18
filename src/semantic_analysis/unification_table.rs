use soup::handle_map::{Handle, HandleMap};

use crate::common::types::TypeId;

soup::handle_impl!(pub(crate) TypeVarId);
soup::handle_impl!(pub(crate) IntVarId);

pub(crate) struct UnificationTable {
    type_var_parent: HandleMap<TypeVarId, TypeVarId>,
    type_var_rank: HandleMap<TypeVarId, u8>,
    type_var_concrete: HandleMap<TypeVarId, Option<TypeId>>,

    int_var_parent: HandleMap<IntVarId, IntVarId>,
    int_var_rank: HandleMap<IntVarId, u8>,
    int_var_concrete: HandleMap<IntVarId, Option<TypeId>>,
}

impl UnificationTable {
    pub(crate) fn new() -> Self {
        Self {
            type_var_parent: HandleMap::new(),
            type_var_rank: HandleMap::new(),
            type_var_concrete: HandleMap::new(),

            int_var_parent: HandleMap::new(),
            int_var_rank: HandleMap::new(),
            int_var_concrete: HandleMap::new(),
        }
    }

    pub(crate) fn make_type_var_set(&mut self) -> TypeVarId {
        let id = TypeVarId::new(self.type_var_parent.count());
        self.type_var_parent.add(id);
        self.type_var_rank.add(0);
        self.type_var_concrete.add(None);
        id
    }

    pub(crate) fn make_int_var_set(&mut self) -> IntVarId {
        let id = IntVarId::new(self.int_var_parent.count());
        self.int_var_parent.add(id);
        self.int_var_rank.add(0);
        self.int_var_concrete.add(None);
        id
    }

    pub(crate) fn find_type_var(&mut self, mut id: TypeVarId) -> TypeVarId {
        while self.type_var_parent[id] != id {
            self.type_var_parent[id] = self.type_var_parent[self.type_var_parent[id]];
            id = self.type_var_parent[id];
        }
        id
    }

    pub(crate) fn find_int_var(&mut self, mut id: IntVarId) -> IntVarId {
        while self.int_var_parent[id] != id {
            self.int_var_parent[id] = self.int_var_parent[self.int_var_parent[id]];
            id = self.int_var_parent[id];
        }
        id
    }

    pub(crate) fn union_type_vars(&mut self, a: TypeVarId, b: TypeVarId) {
        let a_rep = self.find_type_var(a);
        let b_rep = self.find_type_var(b);

        if a_rep == b_rep {
            return;
        }

        if self.type_var_rank[a_rep] < self.type_var_rank[b_rep] {
            self.type_var_parent[a_rep] = b_rep;
        } else if self.type_var_rank[b_rep] < self.type_var_rank[a_rep] {
            self.type_var_parent[b_rep] = a_rep;
        } else {
            self.type_var_parent[b_rep] = a_rep;
            self.type_var_rank[a_rep] += 1;
        }
    }

    pub(crate) fn union_int_vars(&mut self, a: IntVarId, b: IntVarId) {
        let a_rep = self.find_int_var(a);
        let b_rep = self.find_int_var(b);

        if a_rep == b_rep {
            return;
        }

        if self.int_var_rank[a_rep] < self.int_var_rank[b_rep] {
            self.int_var_parent[a_rep] = b_rep;
        } else if self.int_var_rank[b_rep] < self.int_var_rank[a_rep] {
            self.int_var_parent[b_rep] = a_rep;
        } else {
            self.int_var_parent[b_rep] = a_rep;
            self.int_var_rank[a_rep] += 1;
        }
    }

    pub(crate) fn get_concrete_type_var(&self, root: TypeVarId) -> Option<TypeId> {
        self.type_var_concrete[root]
    }

    pub(crate) fn get_concrete_int_var(&self, root: IntVarId) -> Option<TypeId> {
        self.int_var_concrete[root]
    }

    pub(crate) fn set_concrete_type_var(&mut self, root: TypeVarId, ty: TypeId) {
        self.type_var_concrete[root] = Some(ty);
    }

    pub(crate) fn set_concrete_int_var(&mut self, root: IntVarId, ty: TypeId) {
        self.int_var_concrete[root] = Some(ty);
    }
}
