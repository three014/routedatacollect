use std::{cmp::Ordering, ops::Deref, sync::Arc};

#[derive(Clone, Debug)]
enum Container<'a, T, const N: usize>
where
    T: Copy + Sized + 'a,
{
    Arc(Arc<[T]>),
    Owned([T; N]),
    Ref(&'a [T]),
}

impl<'a, T, const N: usize> Deref for Container<'a, T, N>
where
    T: Copy + Sized + 'a,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            Container::Arc(a) => a,
            Container::Owned(o) => o,
            Container::Ref(r) => r,
        }
    }
}

/// An infinite immutable ring-buffer.
///
/// The contents must implement
/// the `Copy` trait, so this is mostly
/// meant to be used with number and
/// enum types.
#[derive(Clone, Debug)]
pub struct CopyRing<'a, T, const N: usize>
where
    T: Copy + 'a,
    [T]: 'a,
{
    index: usize,
    collection: Container<'a, T, N>,
    init: bool,
}

impl<T: Copy, const N: usize> CopyRing<'static, T, N> {
    pub const fn owned(collection: [T; N]) -> Self {
        Self {
            index: 0,
            init: false,
            collection: Container::Owned(collection),
        }
    }
}

impl<T: Copy> CopyRing<'static, T, 0> {
    pub fn arc(collection: Arc<[T]>) -> Self {
        Self {
            index: 0,
            init: false,
            collection: Container::Arc(collection),
        }
    }
}

impl<'a, T> CopyRing<'a, T, 0>
where
    T: Copy + Sized + 'a,
{
    pub const fn borrowed(collection: &'a [T]) -> Self {
        Self {
            index: 0,
            init: false,
            collection: Container::Ref(collection),
        }
    }
}

impl<'a, T, const N: usize> CopyRing<'a, T, N>
where
    T: Copy + 'a,
{
    pub fn arc_with_size(collection: Arc<[T]>) -> Self {
        Self {
            index: 0,
            init: false,
            collection: Container::Arc(collection),
        }
    }

    pub const fn borrowed_with_size(collection: &'a [T]) -> Self {
        Self {
            index: 0,
            init: false,
            collection: Container::Ref(collection),
        }
    }

    /// Sets the inner pointer to the
    /// first item in the ring, so
    /// that calling `next` yields
    /// that first item.
    pub fn reset(&mut self) {
        self.index = 0;
        self.set_init(false);
    }

    /// Rotates the ring to the left, yielding `Copyring::period`
    /// number of values in total. Dropping this iterator
    /// before it finishes will leave the `CopyRing` at
    /// wherever it was before the next iteration.
    pub fn one_cycle(&mut self) -> CycleIterMut<'a, '_, T, N> {
        let number_iters_left = self.collection.len();
        self.take_mut(number_iters_left)
    }

    /// Returns the size of one loop around the ring,
    /// aka the period of the stream.
    pub fn period(&self) -> usize {
        self.collection.len()
    }

    pub fn is_empty(&self) -> bool {
        self.collection.is_empty()
    }

    pub fn first(&self) -> Option<T> {
        self.collection.first().copied()
    }

    pub fn last(&self) -> Option<T> {
        self.collection.last().copied()
    }

    /// Returns the next item in the ring,
    /// advancing the ring by one, but
    /// also returns whether the ring
    /// wrapped back around to the
    /// first item.
    ///
    /// `true` if wrapping occurred, `false` otherwise.
    pub fn checked_next(&mut self) -> Option<(T, bool)> {
        let was_init = self.is_init();
        let prev_index = self.index;
        let next = self.next()?;
        Some((next, prev_index == 0 && was_init))
    }

    /// Rotates the ring to the left, yielding each value until the
    /// ring reaches the start again.
    pub fn until_start(&mut self) -> CycleIterMut<'a, '_, T, N> {
        let number_iters_left = self.period() - self.index;
        self.take_mut(number_iters_left)
    }

    /// Returns the previous item in the ring.
    ///
    /// Since `next` advances the ring by one,
    /// calling `prev` will yield the same value
    /// as `next` if they are called one-after-another.
    pub fn prev(&mut self) -> Option<T> {
        if self.collection.is_empty() {
            return None;
        }
        Some(self.collection[self.index])
    }

    /// Returns the same value as `prev` without
    /// reverting the ring's index.
    pub fn peek_prev(&self) -> Option<T> {
        if self.collection.is_empty() {
            None
        } else {
            let len = self.period() as isize;
            let idx = self.index as isize;
            let new_idx = ((-((1 - idx) % len) + len) % len) as usize;
            Some(self.collection[new_idx])
        }
    }

    pub fn rotate_left(&mut self, n: usize) {
        self.index = (self.index + n) % self.period();
        self.set_init(n != 0 || self.init);
        //println!("Rotated left - new index: {}", self.index);
    }

    pub fn rotate_right(&mut self, n: usize) {
        let len = self.period() as isize;
        let idx = self.index as isize;
        let x = n as isize;
        self.index = ((-((x - idx) % len) + len) % len) as usize;
        self.set_init(n != 0 || self.init);
        //println!("Rotated right - new index: {}", self.index);
    }

    /// Returns the same value as `next` without
    /// advancing the ring's index.
    pub fn peek_next(&self) -> Option<T> {
        if self.collection.is_empty() {
            None
        } else {
            Some(self.collection[self.index])
        }
    }

    pub fn next(&mut self) -> Option<T> {
        if self.collection.is_empty() {
            return None;
        }
        let index = self.index;
        self.rotate_left(1);
        Some(self.collection[index])
    }

    /// Takes the first n elements of the ring and
    /// iterates through those elements, rotating
    /// the `CopyRing` to the left for each
    /// iteration.
    ///
    /// If you'd like to iterate through the ring
    /// without mutating the ring, use `take_ref` instead.
    pub fn take_mut(&mut self, n: usize) -> CycleIterMut<'a, '_, T, N> {
        CycleIterMut { ring: self, n }
    }

    /// Takes the first n elements of the ring
    /// and iterates through it just like `take_mut`
    /// without mutating the ring itself.
    pub fn take_ref(&self, n: usize) -> impl ExactSizeIterator<Item = T> + '_ {
        CycleIter {
            ring_buf: &self.collection,
            index: self.index,
            n,
        }
    }

    fn set_init(&mut self, init: bool) {
        self.init = init;
    }

    pub const fn is_init(&self) -> bool {
        self.init
    }
}

impl<'a, T: Copy + Ord + 'a, const N: usize> CopyRing<'a, T, N> {
    /// Binary searches the internal buffer for a given element.
    /// If the slice is not sorted, the returned result
    /// is unspecified and meaningless.
    ///
    /// If the element was not found, then the function returns the first
    /// item to the right of where the element should have been.
    /// But if there is no element to the right, then the function will
    /// wrap around and select the first element, returning the
    /// element with a value of `true`. Otherwise, the bool will be `false`.
    pub fn binary_search_or_greater(&mut self, x: &T) -> Option<(T, bool)> {
        self.reset();
        let index = match self.collection.binary_search(x) {
            Ok(i) => i,
            Err(i) => i,
        };
        self.rotate_left(index);
        self.checked_next()
    }

    pub fn binary_search_or_greater_by<F>(&mut self, f: F) -> Option<(T, bool)>
    where
        F: FnMut(&T) -> Ordering,
    {
        self.reset();
        let index = match self.collection.binary_search_by(f) {
            Ok(i) => i,
            Err(i) => i,
        };
        self.rotate_left(index);
        self.checked_next()
    }
}

pub struct CycleIterMut<'a: 'b, 'b, T: Copy, const N: usize> {
    ring: &'b mut CopyRing<'a, T, N>,
    n: usize,
}

struct CycleIter<'a, T: Copy> {
    ring_buf: &'a [T],
    index: usize,
    n: usize,
}

impl<'a: 'b, 'b, T: Copy, const N: usize> CycleIterMut<'a, 'b, T, N> {
    pub fn checked_next(&mut self) -> Option<(T, bool)> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            self.ring.checked_next()
        }
    }

    pub fn checked(self) -> Checked<'a, 'b, T, N> {
        Checked(self)
    }
}

impl<T: Copy> Iterator for CycleIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            let index = self.index;
            self.index = (self.index + 1) % self.ring_buf.len();
            Some(self.ring_buf[index])
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }
}
impl<T: Copy> ExactSizeIterator for CycleIter<'_, T> {}

pub struct Checked<'a: 'b, 'b, T: Copy, const N: usize>(CycleIterMut<'a, 'b, T, N>);
impl<T: Copy, const N: usize> Iterator for Checked<'_, '_, T, N> {
    type Item = (T, bool);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.checked_next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl<T: Copy, const N: usize> ExactSizeIterator for Checked<'_, '_, T, N> {}

impl<T: Copy, const N: usize> Iterator for CycleIterMut<'_, '_, T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            self.ring.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }
}
impl<T: Copy, const N: usize> ExactSizeIterator for CycleIterMut<'_, '_, T, N> {}

impl<T: Copy> From<Vec<T>> for CopyRing<'static, T, 0> {
    fn from(value: Vec<T>) -> Self {
        Self::arc(Arc::from(value))
    }
}

impl<T: Copy, const N: usize> From<[T; N]> for CopyRing<'static, T, N> {
    fn from(value: [T; N]) -> Self {
        Self::owned(value)
    }
}

impl<T: Copy> From<T> for CopyRing<'static, T, 1> {
    fn from(value: T) -> Self {
        Self::owned([value])
    }
}

impl<T: Copy> From<Box<[T]>> for CopyRing<'static, T, 0> {
    fn from(value: Box<[T]>) -> Self {
        Self::arc(Arc::from(value))
    }
}

impl<T: Copy> FromIterator<T> for CopyRing<'static, T, 0> {
    fn from_iter<A: IntoIterator<Item = T>>(iter: A) -> Self {
        Self::arc(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod test {
    use super::CopyRing;
    use std::collections::VecDeque;

    #[test]
    fn first_cycle_equals_origin_vec() {
        let left = vec![1, 2, 3];
        let right: Vec<i32> = CopyRing::from(left.clone()).take_mut(3).collect();

        assert_eq!(left, right)
    }

    #[test]
    fn one_cycle_fn_equals_origin_vec() {
        let left = vec![1, 2, 3, 4];
        let mut right = CopyRing::from(left.clone());
        let right_cycle = right.one_cycle();

        for (l, r) in left.iter().zip(right_cycle) {
            assert_eq!(*l, r)
        }
    }

    #[test]
    fn checked_next_only_true_when_wrap_occurs() {
        let mut ring = CopyRing::from_iter(0..3);
        assert!(!ring.is_init());

        let next = ring.checked_next().unwrap();
        assert!(ring.is_init());

        assert_eq!(0, next.0);
        assert_eq!(false, next.1);

        ring.rotate_left(2);
        let next = ring.checked_next().unwrap();

        assert_eq!(0, next.0);
        assert_eq!(true, next.1);
    }

    #[test]
    fn first_next_equals_first_value() {
        let mut ring = CopyRing::owned([2, 5, 7, 8]);
        assert_eq!(2, ring.next().unwrap())
    }

    #[test]
    fn shifted_ring_equals_shifted_vec() {
        let mut left = VecDeque::from([3, 2, 63, 7, 4]);
        let mut right = CopyRing::from_iter(left.clone());

        left.rotate_left(3);
        right.next();
        right.next();
        right.next();
        for (l, r) in left.iter().zip(right.take_mut(5)) {
            assert_eq!(*l, r)
        }
    }

    #[test]
    fn next_equals_prev() {
        let mut ring = CopyRing::from(vec![1, 5, 8, 12]);

        let next = ring.next().unwrap();
        let prev = ring.prev().unwrap();

        assert_eq!(next, prev)
    }

    #[test]
    fn prev_equals_last_item_in_vec() {
        let left = [2, 4, 6, 8];
        let mut right = CopyRing::borrowed(&left);

        assert_eq!(*left.last().unwrap(), right.prev().unwrap());
    }

    #[test]
    fn first_cycle_equals_origin_iter() {
        let left = 0..100;
        let mut right = CopyRing::arc(left.clone().collect());

        for (l, r) in left.zip(right.take_mut(1000)) {
            assert_eq!(l, r)
        }
    }

    #[test]
    fn rotate_right_works() {
        let mut ring = CopyRing::from_iter(0..8);

        ring.rotate_right(3);
        assert_eq!(5, ring.index);

        ring.rotate_right(0);
        assert_eq!(5, ring.index);

        ring.rotate_right(ring.period());
        assert_eq!(5, ring.index);

        ring.rotate_right(2);
        assert_eq!(3, ring.index);
    }

    #[test]
    fn single_item_loops_forever() {
        let mut ring = CopyRing::owned([3]);

        for _ in 0..1000 {
            assert_eq!(3, ring.next().unwrap());
        }
    }

    #[test]
    fn zero_items_returns_none() {
        let mut ring: CopyRing<i32, 0> = CopyRing::owned([]);

        assert!(ring.next().is_none())
    }

    #[allow(unused)]
    fn size() {
        let _ring = CopyRing::owned([0u8; 23]);
    }
}
