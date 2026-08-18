use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::slice;

use super::Handle;

#[derive(Clone)]
pub(crate) struct HandleMap<K, V> {
    data: Vec<V>,
    _marker: PhantomData<K>,
}

pub(crate) struct Iter<'a, K, V> {
    inner: std::iter::Enumerate<slice::Iter<'a, V>>,
    _marker: PhantomData<K>,
}

pub(crate) struct IterMut<'a, K, V> {
    inner: std::iter::Enumerate<slice::IterMut<'a, V>>,
    _marker: PhantomData<K>,
}

pub(crate) struct IntoIter<K, V> {
    inner: std::iter::Enumerate<std::vec::IntoIter<V>>,
    _marker: PhantomData<K>,
}

impl<K: Handle, V> HandleMap<K, V> {
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

    pub(crate) fn next_key(&self) -> K {
        K::new(self.data.len())
    }

    pub(crate) fn last(&self) -> Option<(K, &V)> {
        let len = self.data.len();
        let last = self.data.last()?;
        Some((K::new(len - 1), last))
    }

    pub(crate) fn last_mut(&mut self) -> Option<(K, &mut V)> {
        let len = self.data.len();
        let last = self.data.last_mut()?;
        Some((K::new(len - 1), last))
    }

    pub(crate) fn add(&mut self, value: V) -> K {
        let index = self.data.len();
        self.data.push(value);
        K::new(index)
    }

    pub(crate) fn get(&self, key: K) -> Option<&V> {
        self.data.get(key.index())
    }

    pub(crate) fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.data.get_mut(key.index())
    }

    pub(crate) fn contains_key(&self, k: K) -> bool {
        k.index() < self.data.len()
    }

    pub(crate) fn count(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = K> + '_ {
        (0..self.data.len()).map(K::new)
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.data.iter()
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut V> + '_ {
        self.data.iter_mut()
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
impl<K: Handle, V> Index<K> for HandleMap<K, V> {
    type Output = V;
    fn index(&self, index: K) -> &V {
        &self.data[index.index()]
    }
}

// for `map[k] = v`
impl<K: Handle, V> IndexMut<K> for HandleMap<K, V> {
    fn index_mut(&mut self, index: K) -> &mut V {
        &mut self.data[index.index()]
    }
}

// for `HandleMap::default()`
impl<K: Handle, V> Default for HandleMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

// for `(k, v) in map`
impl<K: Handle, V> IntoIterator for HandleMap<K, V> {
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
impl<'a, K: Handle, V> IntoIterator for &'a HandleMap<K, V> {
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// for `(k, v) in &mut map`
impl<'a, K: Handle, V> IntoIterator for &'a mut HandleMap<K, V> {
    type Item = (K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// for `let map: HandleMap<K, V> = values.collect()`
impl<K: Handle, V> FromIterator<V> for HandleMap<K, V> {
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        Self {
            data: Vec::from_iter(iter),
            _marker: PhantomData,
        }
    }
}

// for `map.extend(<more values>)`
impl<K: Handle, V> Extend<V> for HandleMap<K, V> {
    fn extend<T: IntoIterator<Item = V>>(&mut self, iter: T) {
        self.data.extend(iter);
    }
}

// for `println!("{:?}", map)`
impl<K: Handle + fmt::Debug, V: fmt::Debug> fmt::Debug for HandleMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

// for `iter.next()` on shared references
impl<'a, K: Handle, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (K::new(k), v))
    }
}

// for `iter.next()` on mutable references
impl<'a, K: Handle, V> Iterator for IterMut<'a, K, V> {
    type Item = (K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (K::new(k), v))
    }
}

// for `iter.next()` on owned values
impl<K: Handle, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (K::new(k), v))
    }
}
