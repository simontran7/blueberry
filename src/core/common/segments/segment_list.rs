use std::ops::Index;

use super::segment::Segment;

#[derive(Clone, PartialEq, Eq, Debug, salsa::SalsaValue)]
pub(crate) struct SegmentList<T> {
    items: Vec<T>,
}

impl<T> SegmentList<T> {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T: Copy> SegmentList<T> {
    pub(crate) fn add_many(&mut self, items: &[T]) -> Segment<T> {
        let start = u32::try_from(self.items.len()).expect("list too large");
        let len = u32::try_from(items.len()).expect("range too large");
        self.items.extend_from_slice(items);
        Segment::new(start, len)
    }
}

// for `arena[range]`
impl<T> Index<Segment<T>> for SegmentList<T> {
    type Output = [T];

    fn index(&self, range: Segment<T>) -> &[T] {
        &self.items[range.bounds()]
    }
}

// NOTE: manually implemented, since `#[derive(Default)]` would add
// an unwanted `T: Default` bound.
impl<T> Default for SegmentList<T> {
    fn default() -> Self {
        Self::new()
    }
}
