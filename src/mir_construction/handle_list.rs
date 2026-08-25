use std::fmt;
use std::marker::PhantomData;

use handlemap::handle_map::Handle;

/// A 4-byte handle to a growable, mutable list of `H`s living in a [`HandleListSubAllocator`].
/// Used wherever MIR needs a variable-length run of handles of the same type (e.g., block
/// parameters, call/branch arguments, and instruction results for `ValueId`; predecessor
/// instructions or deferred variables for other handle types).
///
/// `start` is the index in the allocator's backing storage `HandleListSubAllocator::data` where
/// this list's elements begin. The live element count lives in the header, at index
/// `start - 1`. This common trick keeps the handle 4 bytes large and `Copy` (unlike a `Vec`,
/// which would be 24 bytes and owned).
///
/// # Safety
///
/// [`HandleList`] is `Copy`, but must be treated as a unique logical owner of its
/// allocated block. Aliasing copies (e.g. cloning a handle and calling [`HandleList::clear`]
/// through one while the other still exists) will leave the surviving copy dangling, i.e.,
/// the allocator may hand the freed block out to a new list, and the stale handle
/// would then silently read or corrupt someone else's data.
///
/// There is no generation counter or other mechanism to detect this. The caller is
/// responsible for ensuring that at most one live handle refers to any given allocation
/// at any time.
///
/// # Examples
///
/// ```rust,ignore
/// let mut allocator = HandleListSubAllocator::<ValueId>::new();
/// let mut list = HandleList::<ValueId>::from(&mut allocator, &[ValueId::new(1), ValueId::new(2)]);
/// list.add_last(&mut allocator, ValueId::new(3));
/// assert_eq!(list.to_slice(&allocator), &[ValueId::new(1), ValueId::new(2), ValueId::new(3)]);
/// ```
#[derive(Clone, Copy)]
pub(crate) struct HandleList<H> {
    pub(super) start: u32,
    _marker: PhantomData<H>,
}

impl<H> fmt::Debug for HandleList<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandleList")
            .field("start", &self.start)
            .finish()
    }
}

impl<H> Default for HandleList<H> {
    fn default() -> Self {
        Self {
            start: 0,
            _marker: PhantomData,
        }
    }
}

/// A Segregated Free List suballocator storing every [`HandleList`]'s content.
///
/// # Layout
///
/// `data` holds every list contiguously. A list "points" to a contiguous chunk
/// of slots called a **memory block**. This memory block has three components:
///
/// ```text
/// [header][elements][spare]
/// ```
///
/// 1. **Header**: one slot, which holds the number of live elements.
/// 2. **Elements**: the list's actual contents.
/// 3. **Spare**: reserved slots left over from rounding up to a size class.
///
/// Every memory block is sized according to [`SizeClass`].
///
/// A **free list** is an *intrusive* linked list where each node is a *free* block.
/// This allocator creates at most one free list per size class. Concretely, `free`
/// is an array list that maps a [`SizeClass`] as an index, to its free list's head node as element.
/// As such, for some size class `sz` without a free list, its element at `free[sz]` is `0` (see
/// [`HandleList`]'s documentation).
/// Every free block's header is `H::new(0)`, and the free list's node's next pointer is also
/// embedded in `data` as an `H`.
/// The tail node of a free list's next pointer is `0`.
pub(crate) struct HandleListSubAllocator<H> {
    pub(super) data: Vec<H>,
    free: Vec<usize>,
}

/// Size class for the allocator's segregated free lists.
/// A size class `n` (0, 1, 2, ...) spans `4 << n` slots (4, 8, 16, 32, ...).
#[derive(Clone, Copy)]
struct SizeClass(u8);

impl<H: Handle> HandleList<H> {
    /// Marks an empty list.
    ///
    /// 0 may be used as the empty sentinel value because non-empty lists
    /// *always* have a `start` >= 1 (give that 1 is lowest possible `start`
    /// that would be able to accomodate a header `start - 1` within [0, ...]).
    const EMPTY: u32 = 0;

    /// Creates and returns a new, empty list.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Allocates a new list in `allocator` and copies `slice`'s elements into it.
    pub(crate) fn from(allocator: &mut HandleListSubAllocator<H>, slice: &[H]) -> Self {
        // No allocation if `slice` is empty.
        if slice.is_empty() {
            return Self::new();
        }
        let count = slice.len();
        let block = allocator.alloc(count);
        allocator.data[block] = H::new(count);
        allocator.data[block + 1..=block + count].copy_from_slice(slice);
        Self {
            start: (block + 1) as u32,
            _marker: PhantomData,
        }
    }

    /// Returns the number of elements in the list.
    pub(crate) fn count(self, allocator: &HandleListSubAllocator<H>) -> usize {
        // wrapping_sub so that start == 0 (empty) maps to usize::MAX, which is
        // guaranteed out of bounds for any Vec. This makes `.get()` return `None`,
        // collapsing the emptiness check and bounds check into one.
        allocator
            .data
            .get((self.start as usize).wrapping_sub(1))
            .map_or(0, |v| v.index())
    }

    /// Returns whether the list has no elements.
    pub(crate) fn is_empty(self, allocator: &HandleListSubAllocator<H>) -> bool {
        self.count(allocator) == 0
    }

    /// Returns the list's elements as a slice, or an empty slice if empty.
    pub(crate) fn to_slice(self, allocator: &HandleListSubAllocator<H>) -> &[H] {
        if self.is_empty(allocator) {
            &[]
        } else {
            &allocator.data[self.start as usize..self.start as usize + self.count(allocator)]
        }
    }

    /// Returns the list's elements as a mutable slice, or an empty slice if empty.
    pub(crate) fn to_mut_slice(self, allocator: &mut HandleListSubAllocator<H>) -> &mut [H] {
        if self.is_empty(allocator) {
            &mut []
        } else {
            let start = self.start as usize;
            let count = self.count(allocator);
            &mut allocator.data[start..start + count]
        }
    }

    /// Returns the element at `index`, or `None` if out of bounds.
    pub(crate) fn get(&self, allocator: &HandleListSubAllocator<H>, index: usize) -> Option<H> {
        self.to_slice(allocator).get(index).copied()
    }

    /// Adds `value` to the end of the list.
    pub(crate) fn add_last(&mut self, allocator: &mut HandleListSubAllocator<H>, value: H) {
        let start = self.start as usize;
        if self.is_empty(allocator) {
            let block = allocator.alloc(1);
            allocator.data[block] = H::new(1);
            allocator.data[block + 1] = value;
            self.start = (block + 1) as u32;
        } else {
            let new_count = self.count(allocator) + 1;
            let block;

            if SizeClass::exceeds_capacity(new_count) {
                block = allocator.realloc(
                    start - 1,
                    self.count(allocator),
                    new_count,
                    self.count(allocator) + 1,
                );
                self.start = (block + 1) as u32;
            } else {
                block = start - 1;
            }

            allocator.data[block + new_count] = value;
            allocator.data[block] = H::new(new_count);
        }
    }

    /// Removes the element at `index`, shifting later elements down by one to preserve their relative order.
    pub(crate) fn remove(&mut self, index: usize, allocator: &mut HandleListSubAllocator<H>) {
        let count = self.count(allocator);
        assert!(index < count, "index out of bounds");
        // The backing block is not freed or reallocated, just shrunk in place.
        let start = self.start as usize;
        allocator
            .data
            .copy_within(start + index + 1..start + count, start + index);
        allocator.data[start - 1] = H::new(count - 1);
    }

    /// Removes the last element, decrementing the header count in place.
    pub(crate) fn clear_last(&mut self, allocator: &mut HandleListSubAllocator<H>) {
        // The backing block is not freed or reallocated — the slot is simply abandoned.
        let count = self.count(allocator);
        assert!(count > 0, "called `.clear_last()` on an empty list");
        allocator.data[self.start as usize - 1] = H::new(count - 1);
    }

    /// Frees the list's backing storage and resets it to empty.
    pub(crate) fn clear(&mut self, allocator: &mut HandleListSubAllocator<H>) {
        // Any other `Copy` of this handle still has the old `start` and is left dangling,
        // so it may point at a block the allocator later hands out to a different list.
        if !self.is_empty(allocator) {
            allocator.free(self.start as usize - 1, self.count(allocator));
        }
        self.start = Self::EMPTY;
    }
}

impl<H: Handle> HandleListSubAllocator<H> {
    /// Creates and returns an allocator for lists.
    pub(crate) const fn new() -> Self {
        Self {
            data: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Drops every [`HandleList`]'s contents and free-list state, returning the allocator to its initial empty state.
    pub(crate) fn reset(&mut self) {
        // Any `HandleList` into this allocator is invalidated.
        self.data.clear();
        self.free.clear();
    }

    /// Returns the index of the block's header that can fit `count` elements.
    fn alloc(&mut self, count: usize) -> usize {
        let size_class = SizeClass::new(count);
        if let Some(&head) = self.free.get(size_class.0 as usize)
            && head > 0
        {
            let next = self.data[head];
            self.free[size_class.0 as usize] = next.index();
            head - 1
        } else {
            let header_idx = self.data.len();
            self.data
                .resize(header_idx + size_class.capacity(), H::new(0));
            header_idx
        }
    }

    /// Frees `block` which contains `count` slots.
    fn free(&mut self, block: usize, count: usize) {
        let size_class = SizeClass::new(count);
        if self.free.len() <= size_class.0 as usize {
            self.free.resize(size_class.0 as usize + 1, 0);
        }
        self.data[block] = H::new(0);
        self.data[block + 1] = H::new(self.free[size_class.0 as usize]);
        self.free[size_class.0 as usize] = block + 1;
    }

    /// Moves a list's current block to a block sized for `new_count`.
    fn realloc(&mut self, block: usize, old_count: usize, new_count: usize, copy: usize) -> usize {
        let new_block = self.alloc(new_count);
        if copy > 0 {
            let (old, new) = self.mut_slices(block, new_block);
            new[..copy].copy_from_slice(&old[..copy]);
        }
        self.free(block, old_count);
        new_block
    }

    /// Returns two mutable slices into `data` starting at `block0` and `block1` respectively.
    fn mut_slices(&mut self, block0: usize, block1: usize) -> (&mut [H], &mut [H]) {
        // Uses `split_at_mut` to satisfy Rust's aliasing rules, since you cannot take two
        // mutable references (specifically, slices) into the same `Vec` directly.
        if block0 < block1 {
            let (s0, s1) = self.data.split_at_mut(block1);
            let s0 = &mut s0[block0..];
            (s0, s1)
        } else {
            let (s1, s0) = self.data.split_at_mut(block0);
            let s1 = &mut s1[block1..];
            (s0, s1)
        }
    }
}

impl SizeClass {
    /// Constructs the smallest size class that can fit the desired `count` of handles (excluding the header slot).
    fn new(count: usize) -> Self {
        assert!(count > 0);
        Self(((count | 3).ilog2() - 1) as u8)
    }

    /// Returns the number of slots that a size class may accomodate.
    const fn capacity(&self) -> usize {
        4 << self.0
    }

    /// Returns whether a list that just grew to `count` elements has outgrown its current block and must be moved to the next size class.
    const fn exceeds_capacity(count: usize) -> bool {
        // Block capacities (element slots, excluding the header) are 3, 7, 15, 31, ..., so
        // a list overflows its block exactly when `count` is 4, 8, 16, 32, ....
        count > 3 && count.is_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    handlemap::handle_impl!(pub(crate) TestId);

    fn ids(values: &[u32]) -> Vec<TestId> {
        values.iter().map(|&v| TestId::new(v as usize)).collect()
    }

    #[test]
    fn new_list_is_empty() {
        let allocator = HandleListSubAllocator::<TestId>::new();
        let list = HandleList::<TestId>::new();
        assert!(list.is_empty(&allocator));
        assert_eq!(list.count(&allocator), 0);
        assert_eq!(list.to_slice(&allocator), &[]);
    }

    #[test]
    fn from_empty_slice_is_empty_without_allocating() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let list = HandleList::<TestId>::from(&mut allocator, &[]);
        assert!(list.is_empty(&allocator));
        assert!(allocator.data.is_empty());
    }

    #[test]
    fn from_slice_copies_elements() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let values = ids(&[10, 20, 30]);
        let list = HandleList::<TestId>::from(&mut allocator, &values);
        assert_eq!(list.count(&allocator), 3);
        assert_eq!(list.to_slice(&allocator), values.as_slice());
    }

    #[test]
    fn add_last_grows_across_size_class_boundaries() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let mut list = HandleList::<TestId>::new();
        let values = ids(&[0, 1, 2, 3, 4, 5, 6, 7]);
        // Size class 0 holds 3 elements and class 1 holds 7, so this push sequence crosses
        // both boundaries (at the 4th and 8th elements).
        for &v in &values {
            list.add_last(&mut allocator, v);
        }
        assert_eq!(list.count(&allocator), 8);
        assert_eq!(list.to_slice(&allocator), values.as_slice());
    }

    #[test]
    fn get_returns_none_out_of_bounds() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let list = HandleList::<TestId>::from(&mut allocator, &ids(&[10, 20]));
        assert_eq!(list.get(&allocator, 1), Some(TestId::new(20)));
        assert_eq!(list.get(&allocator, 2), None);
    }

    #[test]
    fn to_mut_slice_allows_in_place_mutation() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let list = HandleList::<TestId>::from(&mut allocator, &ids(&[10, 20, 30]));
        list.to_mut_slice(&mut allocator)[1] = TestId::new(99);
        assert_eq!(list.to_slice(&allocator), ids(&[10, 99, 30]).as_slice());
    }

    #[test]
    fn remove_shifts_later_elements_and_preserves_order() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let mut list = HandleList::<TestId>::from(&mut allocator, &ids(&[0, 1, 2, 3]));

        list.remove(1, &mut allocator);
        assert_eq!(list.to_slice(&allocator), ids(&[0, 2, 3]).as_slice());

        list.remove(0, &mut allocator);
        assert_eq!(list.to_slice(&allocator), ids(&[2, 3]).as_slice());

        list.remove(1, &mut allocator);
        assert_eq!(list.to_slice(&allocator), ids(&[2]).as_slice());
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn remove_panics_out_of_bounds() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let mut list = HandleList::<TestId>::from(&mut allocator, &ids(&[0, 1]));
        list.remove(2, &mut allocator);
    }

    #[test]
    fn clear_last_shrinks_from_the_end() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let mut list = HandleList::<TestId>::from(&mut allocator, &ids(&[0, 1, 2]));

        list.clear_last(&mut allocator);
        assert_eq!(list.to_slice(&allocator), ids(&[0, 1]).as_slice());

        list.clear_last(&mut allocator);
        list.clear_last(&mut allocator);
        assert!(list.is_empty(&allocator));
    }

    #[test]
    #[should_panic(expected = "empty list")]
    fn clear_last_panics_on_empty_list() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let mut list = HandleList::<TestId>::new();
        list.clear_last(&mut allocator);
    }

    #[test]
    fn clear_frees_the_block_for_reuse() {
        let mut allocator = HandleListSubAllocator::<TestId>::new();
        let mut a = HandleList::<TestId>::from(&mut allocator, &ids(&[0, 1, 2]));
        let original_start = a.start;

        a.clear(&mut allocator);
        assert!(a.is_empty(&allocator));

        // A second same-size-class list should be handed the block `a` just freed, instead
        // of growing the allocator's backing storage.
        let len_before = allocator.data.len();
        let b = HandleList::<TestId>::from(&mut allocator, &ids(&[9, 8, 7]));
        assert_eq!(b.start, original_start);
        assert_eq!(allocator.data.len(), len_before);
    }

    #[test]
    fn size_class_capacities_match_documented_thresholds() {
        assert_eq!(SizeClass::new(1).capacity(), 4);
        assert_eq!(SizeClass::new(3).capacity(), 4);
        assert_eq!(SizeClass::new(4).capacity(), 8);
        assert_eq!(SizeClass::new(7).capacity(), 8);
        assert_eq!(SizeClass::new(8).capacity(), 16);

        assert!(!SizeClass::exceeds_capacity(3));
        assert!(SizeClass::exceeds_capacity(4));
        assert!(!SizeClass::exceeds_capacity(5));
        assert!(SizeClass::exceeds_capacity(8));
    }
}
