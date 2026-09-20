use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;

/// A `{ start, len }` range of handles inside a `ChildList<T>`.
#[derive(salsa::SalsaValue)]
pub(crate) struct HandleRange<T> {
    start: u32,
    len: u32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> HandleRange<T> {
    pub(super) fn new(start: u32, len: u32) -> Self {
        Self {
            start,
            len,
            _marker: PhantomData,
        }
    }

    pub(super) fn bounds(&self) -> Range<usize> {
        self.start as usize..(self.start + self.len) as usize
    }

    pub(crate) fn empty() -> Self {
        Self::new(0, 0)
    }

    pub(crate) fn len(&self) -> usize {
        self.len as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T> Clone for HandleRange<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for HandleRange<T> {}

// NOTE: manually implemented, since `#[derive(PartialEq)]` would add
// an unwanted `T: PartialEq` bound via the `PhantomData<T>` field.
impl<T> PartialEq for HandleRange<T> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.len == other.len
    }
}
impl<T> Eq for HandleRange<T> {}

impl<T> fmt::Debug for HandleRange<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandleRange")
            .field("start", &self.start)
            .field("len", &self.len)
            .finish()
    }
}
