use crate::core::common::handle_collections::Handle;
use crate::core::common::handle_collections::handle_disjoint_set::UnifyValue;
use crate::core::common::handle_collections::handle_map::{HandleMap, SideHandleMap};

/// A generic augmented handle disjoint set
pub(crate) struct HandleDisjointSet<H: Handle, V: UnifyValue> {
    parent: HandleMap<H, H>,
    rank: SideHandleMap<H, u8>,
    value: SideHandleMap<H, V>,
}

impl<H: Handle, V: UnifyValue> HandleDisjointSet<H, V> {
    pub(crate) fn new() -> Self {
        Self {
            parent: HandleMap::new(),
            rank: SideHandleMap::new(),
            value: SideHandleMap::new(),
        }
    }

    pub(crate) fn make_set(&mut self, value: V) -> H {
        let handle = self.parent.next_handle();
        self.parent.add(handle);
        self.rank.add(handle, 0);
        self.value.add(handle, value);
        handle
    }

    pub(crate) fn find(&mut self, x: H) -> H {
        let mut cursor = x;
        while self.parent[cursor] != cursor {
            self.parent[cursor] = self.parent[self.parent[cursor]];
            cursor = self.parent[cursor];
        }
        cursor
    }

    pub(crate) fn union(&mut self, a: H, b: H) -> Result<(), V::Error> {
        let a_rep = self.find(a);
        let b_rep = self.find(b);

        if a_rep == b_rep {
            return Ok(());
        }

        let rep_value = V::unify(&self.value[a_rep], &self.value[b_rep])?;

        let rep = if self.rank[a_rep] < self.rank[b_rep] {
            self.parent[a_rep] = b_rep;
            b_rep
        } else if self.rank[b_rep] < self.rank[a_rep] {
            self.parent[b_rep] = a_rep;
            a_rep
        } else {
            self.parent[a_rep] = b_rep;
            self.rank[b_rep] += 1;
            b_rep
        };

        self.value[rep] = rep_value;

        Ok(())
    }

    pub(crate) fn is_connected(&mut self, a: H, b: H) -> bool {
        self.find(a) == self.find(b)
    }

    /// Returns the value of the set containing `a`.
    pub(crate) fn get_value(&mut self, a: H) -> V {
        let rep = self.find(a);
        self.value[rep].clone()
    }

    /// Replaces the value of the set containing `a`.
    pub(crate) fn set_value(&mut self, a: H, value: V) {
        let rep = self.find(a);
        self.value[rep] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::HandleDisjointSet;
    use crate::core::common::handle_collections::handle_disjoint_set::UnifyValue;
    use crate::core::common::handle_collections::handle_impl;

    handle_impl!(TestHandle);

    impl UnifyValue for () {
        type Error = std::convert::Infallible;

        fn unify(_a: &Self, _b: &Self) -> Result<Self, Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_make_set() {
        let mut ds: HandleDisjointSet<TestHandle, ()> = HandleDisjointSet::new();
        let a = ds.make_set(());
        let b = ds.make_set(());
        let c = ds.make_set(());
        assert!(a != b);
        assert!(b != c);
        assert!(a != c);
    }

    #[test]
    fn test_find() {
        let mut ds: HandleDisjointSet<TestHandle, ()> = HandleDisjointSet::new();
        let a = ds.make_set(());
        let b = ds.make_set(());
        assert_eq!(ds.find(a), a);
        assert_eq!(ds.find(b), b);
    }

    #[test]
    fn test_union_merges_singleton_sets() {
        let mut ds: HandleDisjointSet<TestHandle, ()> = HandleDisjointSet::new();
        let a = ds.make_set(());
        let b = ds.make_set(());
        let c = ds.make_set(());
        let d = ds.make_set(());
        ds.union(a, b);
        ds.union(c, d);
        assert!(ds.is_connected(a, b) && ds.is_connected(b, a));
        assert!(ds.is_connected(c, d) && ds.is_connected(d, c));
        assert!(!ds.is_connected(a, c) && !ds.is_connected(c, a));
        assert!(!ds.is_connected(b, d) && !ds.is_connected(d, b));
    }

    #[test]
    fn test_union_merges_non_singleton_sets() {
        let mut ds: HandleDisjointSet<TestHandle, ()> = HandleDisjointSet::new();
        let a = ds.make_set(());
        let b = ds.make_set(());
        let c = ds.make_set(());
        let d = ds.make_set(());

        ds.union(a, b);
        ds.union(c, d);
        ds.union(a, c);

        assert!(ds.is_connected(b, d));
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestValue(Option<u32>);

    #[derive(Debug, PartialEq)]
    struct TestMismatch;

    impl UnifyValue for TestValue {
        type Error = TestMismatch;

        fn unify(a: &Self, b: &Self) -> Result<Self, Self::Error> {
            match (a.0, b.0) {
                (None, None) => Ok(TestValue(None)),
                (Some(x), None) | (None, Some(x)) => Ok(TestValue(Some(x))),
                (Some(x), Some(y)) if x == y => Ok(TestValue(Some(x))),
                (Some(_), Some(_)) => Err(TestMismatch),
            }
        }
    }

    #[test]
    fn test_union_stores_merged_value_on_rep() {
        let mut ds: HandleDisjointSet<TestHandle, TestValue> = HandleDisjointSet::new();

        let a = ds.make_set(TestValue(None));
        let b = ds.make_set(TestValue(Some(7)));
        let c = ds.make_set(TestValue(None));

        ds.union(a, b).unwrap();
        ds.union(a, c).unwrap();

        assert_eq!(ds.get_value(c), TestValue(Some(7)));
    }

    #[test]
    fn test_union_mismatch_leaves_sets_unchanged() {
        let mut ds: HandleDisjointSet<TestHandle, TestValue> = HandleDisjointSet::new();

        let a = ds.make_set(TestValue(Some(1)));
        let b = ds.make_set(TestValue(Some(2)));

        assert_eq!(ds.union(a, b), Err(TestMismatch));

        assert!(!ds.is_connected(a, b));
        assert_eq!(ds.get_value(a), TestValue(Some(1)));
        assert_eq!(ds.get_value(b), TestValue(Some(2)));
    }

    #[test]
    fn test_set_value() {
        let mut ds: HandleDisjointSet<TestHandle, TestValue> = HandleDisjointSet::new();

        let a = ds.make_set(TestValue(None));
        let b = ds.make_set(TestValue(None));

        ds.union(a, b).unwrap();
        ds.set_value(a, TestValue(Some(5)));

        assert_eq!(ds.get_value(b), TestValue(Some(5)));
    }
}
