use super::Handle;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::{fmt, slice};

#[derive(Clone)]
pub(crate) struct SideHandleMap<K, V> {
    data: Vec<Option<V>>,
    _marker: PhantomData<K>,
}

pub(crate) struct Iter<'a, K, V> {
    inner: std::iter::Enumerate<slice::Iter<'a, Option<V>>>,
    _marker: PhantomData<K>,
}

pub(crate) struct IterMut<'a, K, V> {
    inner: std::iter::Enumerate<slice::IterMut<'a, Option<V>>>,
    _marker: PhantomData<K>,
}

pub(crate) struct IntoIter<K, V> {
    inner: std::iter::Enumerate<std::vec::IntoIter<Option<V>>>,
    _marker: PhantomData<K>,
}

impl<K: Handle, V> SideHandleMap<K, V> {
    pub(crate) fn new() -> Self {
        Self {
            data: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            _marker: PhantomData,
        }
    }

    /// Inserts `value` at `key`, growing the map if needed. Slots skipped over while
    /// growing are left absent, not a cloned default: a key that was never explicitly
    /// added is never observably present, whether out of range or sitting in a gap.
    pub(crate) fn add(&mut self, key: K, value: V) -> Option<V> {
        let index = key.index();
        if index >= self.data.len() {
            self.data.resize_with(index + 1, || None);
        }
        self.data[index].replace(value)
    }

    pub(crate) fn resize(&mut self, n: usize) {
        self.data.resize_with(n, || None);
    }

    pub(crate) fn get(&self, key: K) -> Option<&V> {
        self.data.get(key.index())?.as_ref()
    }

    pub(crate) fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.data.get_mut(key.index())?.as_mut()
    }

    pub(crate) fn remove(&mut self, key: K) -> Option<V> {
        self.data.get_mut(key.index())?.take()
    }

    pub(crate) fn count(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = K> + '_ {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(i, v)| v.is_some().then(|| K::new(i)))
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.data.iter().filter_map(Option::as_ref)
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut V> + '_ {
        self.data.iter_mut().filter_map(Option::as_mut)
    }

    pub(crate) fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.data.iter().enumerate(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            inner: self.data.iter_mut().enumerate(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.data.clear()
    }
}

// for `map[k]`
impl<K: Handle, V> Index<K> for SideHandleMap<K, V> {
    type Output = V;
    fn index(&self, k: K) -> &V {
        self.data[k.index()].as_ref().expect("no entry for key")
    }
}

// for `map[k] = v`
impl<K: Handle, V> IndexMut<K> for SideHandleMap<K, V> {
    fn index_mut(&mut self, k: K) -> &mut V {
        self.data[k.index()].as_mut().expect("no entry for key")
    }
}

// for `SideHandleMap::default()`
impl<K: Handle, V> Default for SideHandleMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

// for `(k, v) in map`
impl<K: Handle, V> IntoIterator for SideHandleMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.data.into_iter().enumerate(),
            _marker: PhantomData,
        }
    }
}

// for `(k, v) in &map`
impl<'a, K: Handle, V> IntoIterator for &'a SideHandleMap<K, V> {
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// for `(k, v) in &mut map`
impl<'a, K: Handle, V> IntoIterator for &'a mut SideHandleMap<K, V> {
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// for `println!("{:?}", map)`
impl<K: Handle + fmt::Debug, V: fmt::Debug> fmt::Debug for SideHandleMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// for `iter.next()` on shared references
impl<'a, K: Handle, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        for (i, v) in self.inner.by_ref() {
            if let Some(v) = v {
                return Some((K::new(i), v));
            }
        }
        None
    }
}

// for `iter.next()` on mutable references
impl<'a, K: Handle, V> Iterator for IterMut<'a, K, V> {
    type Item = (K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        for (i, v) in self.inner.by_ref() {
            if let Some(v) = v {
                return Some((K::new(i), v));
            }
        }
        None
    }
}

// for `iter.next()` on owned values
impl<K: Handle, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        for (i, v) in self.inner.by_ref() {
            if let Some(v) = v {
                return Some((K::new(i), v));
            }
        }
        None
    }
}
