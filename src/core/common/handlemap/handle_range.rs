use std::fmt;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::Range;

use super::Handle;

pub(crate) struct HandleRange<K> {
    start: u32,
    count: u32,
    _marker: PhantomData<K>,
}

impl<K: Handle> HandleRange<K> {
    pub(crate) fn new(range: Range<K>) -> Self {
        let start = u32::try_from(range.start.index()).expect("index too large");
        let count = u32::try_from(range.end.index() - range.start.index() as usize)
            .expect("range too large");
        Self {
            start,
            count,
            _marker: PhantomData,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub(crate) fn start(&self) -> K {
        K::new(self.start as usize)
    }

    pub(crate) fn end(&self) -> K {
        K::new((self.start + self.count) as usize)
    }
}

// for `range.clone()`
impl<K> Clone for HandleRange<K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<K> Copy for HandleRange<K> {}

// for `range_a == range_b`
// NOTE: manually implemented, since `#[derive(PartialEq)]` would add
// an unwanted `K: PartialEq` bound via the `PhantomData<K>` field.
impl<K> PartialEq for HandleRange<K> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.count == other.count
    }
}
impl<K> Eq for HandleRange<K> {}

// for `println!("{:?}", range)`
impl<K> fmt::Debug for HandleRange<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandleRange")
            .field("start", &self.start)
            .field("count", &self.count)
            .finish()
    }
}

// for `for k in range { .. }`
impl<K: Handle> Iterator for HandleRange<K> {
    type Item = K;

    fn next(&mut self) -> Option<K> {
        if self.count > 0 {
            let k = K::new(self.start as usize);
            self.start += 1;
            self.count -= 1;
            Some(k)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.count as usize;
        (len, Some(len))
    }
}

// for iterating from the back, e.g. `range.next_back()`
impl<K: Handle> DoubleEndedIterator for HandleRange<K> {
    fn next_back(&mut self) -> Option<K> {
        if self.count > 0 {
            self.count -= 1;
            Some(K::new((self.start + self.count) as usize))
        } else {
            None
        }
    }
}

// for `range.len()`
impl<K: Handle> ExactSizeIterator for HandleRange<K> {}

impl<K: Handle> FusedIterator for HandleRange<K> {}
