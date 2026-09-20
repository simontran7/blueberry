use std::ops::Index;

use super::handle_range::HandleRange;

#[derive(Clone, PartialEq, Eq, Debug, salsa::SalsaValue)]
pub(crate) struct AppendOnlyHandleList<H> {
    handles: Vec<H>,
}

impl<H> AppendOnlyHandleList<H> {
    pub(crate) fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }
}

impl<H: Copy> AppendOnlyHandleList<H> {
    pub(crate) fn add_many(&mut self, handles: &[H]) -> HandleRange<H> {
        let start = u32::try_from(self.handles.len()).expect("list too large");
        let len = u32::try_from(handles.len()).expect("range too large");
        self.handles.extend_from_slice(handles);
        HandleRange::new(start, len)
    }
}

// for `list[range]`
impl<T> Index<HandleRange<T>> for AppendOnlyHandleList<T> {
    type Output = [T];

    fn index(&self, range: HandleRange<T>) -> &[T] {
        &self.handles[range.bounds()]
    }
}

// NOTE: manually implemented, since `#[derive(Default)]` would add
// an unwanted `H: Default` bound.
impl<H> Default for AppendOnlyHandleList<H> {
    fn default() -> Self {
        Self::new()
    }
}
