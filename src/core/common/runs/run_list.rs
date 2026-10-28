use std::ops::Index;

use crate::core::common::runs::run::Run;

#[derive(Clone, PartialEq, Eq, Debug, salsa::SalsaValue)]
pub(crate) struct RunList<T> {
    items: Vec<T>,
}

impl<T> RunList<T> {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T: Copy> RunList<T> {
    pub(crate) fn add_many(&mut self, items: &[T]) -> Run<T> {
        let start = u32::try_from(self.items.len()).unwrap();
        let len = u32::try_from(items.len()).unwrap();
        self.items.extend_from_slice(items);
        Run::new(start, len)
    }
}

// for `arena[range]`
impl<T> Index<Run<T>> for RunList<T> {
    type Output = [T];

    fn index(&self, run: Run<T>) -> &[T] {
        &self.items[run.bounds()]
    }
}

// NOTE: manually implemented, since `#[derive(Default)]` would add
// an unwanted `T: Default` bound.
impl<T> Default for RunList<T> {
    fn default() -> Self {
        Self::new()
    }
}
