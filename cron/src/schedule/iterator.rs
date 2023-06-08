use super::Schedule;
use chrono::{DateTime, TimeZone};

pub struct ScheduleIter<'a, Tz: TimeZone> {
    schedule: &'a mut Schedule,
    tz: Tz,
}

pub struct OwnedScheduleIter<Tz: TimeZone> {
    schedule: Schedule,
    tz: Tz,
}

impl<'a, Tz: TimeZone> ScheduleIter<'a, Tz> {
    pub fn new(schedule: &'a mut Schedule, timezone: Tz) -> Self {
        Self {
            schedule,
            tz: timezone,
        }
    }
}

impl<Tz: TimeZone> OwnedScheduleIter<Tz> {
    pub fn new(schedule: Schedule, timezone: Tz) -> Self {
        Self {
            schedule,
            tz: timezone,
        }
    }
}

impl<'a, Tz: TimeZone + 'static> Iterator for ScheduleIter<'a, Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.schedule.next(&self.tz))
    }
}

impl<Tz: TimeZone + 'static> Iterator for OwnedScheduleIter<Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.schedule.next(&self.tz))
    }
}

/// An infinite immutable ring-buffer.
///
/// The contents must implement
/// the `Copy` trait, so this is mostly
/// meant to be used with number and
/// enum types.
///
/// Implements the `Iterator` trait,
/// but will never end unless it's
/// cut short with `CopyRing::take`
/// or `CopyRing::one_cycle`.
#[derive(Clone, Debug)]
pub struct CopyRing<T>
where
    T: Copy,
{
    index: usize,
    collection: Vec<T>,
}

impl<T> CopyRing<T>
where
    T: Copy,
{
    /// Sets the inner pointer to the 
    /// first item in the ring, so
    /// that calling `next` yields
    /// that first item.
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Rotates the ring to the left, yielding `Copyring::period`
    /// number of values in total. Dropping this iterator
    /// before it finishes will leave the `CopyRing` at
    /// wherever it was before the next iteration.
    pub fn one_cycle(&mut self) -> impl ExactSizeIterator<Item = T> + '_ {
        let number_iters_left = self.collection.len();
        self.take_mut(number_iters_left)
    }

    /// Returns the size of one loop around the ring,
    /// aka the period of the stream.
    pub fn period(&self) -> usize {
        self.collection.len()
    }

    /// Returns the next item in the ring, 
    /// advancing the ring by one, but 
    /// also returns whether the ring
    /// wrapped back around to the 
    /// first item.
    /// 
    /// True if wrapping occurred, false otherwise.
    pub fn checked_next(&mut self) -> Option<(T, bool)> {
        let prev_idx = self.index;
        let next = self.next()?;
        if prev_idx > self.index {
            Some((next, true))
        } else {
            Some((next, false))
        }
    }

    /// Rotates the ring to the left, yielding each value until the
    /// ring reaches the start again.
    pub fn until_start(&mut self) -> impl ExactSizeIterator<Item = T> + '_ {
        let number_iters_left = self.period() - self.index;
        let corrected_num = number_iters_left % self.period();
        self.take_mut(corrected_num)
    }

    /// Returns the previous item in the ring.
    ///
    /// Since `next` advances the ring by one,
    /// calling `prev` will yield the same value
    /// as `next` if they are called one-after-another.
    pub fn prev(&mut self) -> Option<T> {
        if self.collection.len() == 0 {
            return None;
        }
        self.rotate_right(1);
        Some(self.collection[self.index])
    }

    pub fn rotate_left(&mut self, n: usize) {
        self.index = (self.index + n) % self.period()
    }

    pub fn rotate_right(&mut self, n: usize) {
        let len = self.period() as isize;
        let idx = self.index as isize;
        let x = n as isize;
        self.index = ((-((x - idx) % len) + len) % len) as usize
    }

    /// Returns the same value as `next` without
    /// advancing the ring's index.
    pub fn peek(&self) -> Option<T> {
        if self.period() == 0 {
            None
        } else {
            Some(self.collection[self.index])
        }
    }

    pub fn next(&mut self) -> Option<T> {
        if self.collection.len() == 0 {
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
    /// without mutating the ring, use `take` instead.
    pub fn take_mut(&mut self, n: usize) -> impl ExactSizeIterator<Item = T> + '_ {
        CycleIterMut { ring: self, n }
    }

    /// Takes the first n elements of the ring
    /// and iterates through it just like `take_mut`
    /// without mutating the ring itself.
    pub fn take(&self, n: usize) -> impl ExactSizeIterator<Item = T> + '_ {
        CycleIter { ring_buf: &self.collection, index: self.index, n }
    }
}

struct CycleIterMut<'a, T: Copy> {
    ring: &'a mut CopyRing<T>,
    n: usize,
}

struct CycleIter<'a, T: Copy> {
    ring_buf: &'a Vec<T>,
    index: usize,
    n: usize,
}

impl<'a, T: Copy> Iterator for CycleIter<'a, T> {
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

impl<'a, T: Copy> ExactSizeIterator for CycleIter<'a, T> {}

impl<'a, T: Copy> Iterator for CycleIterMut<'a, T> {
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

impl<'a, T: Copy> ExactSizeIterator for CycleIterMut<'a, T> {}

impl<T> From<T> for CopyRing<T>
where
    T: Copy,
{
    fn from(value: T) -> Self {
        Self {
            index: 0,
            collection: vec![value],
        }
    }
}

impl<T> From<Vec<T>> for CopyRing<T>
where
    T: Copy,
{
    fn from(value: Vec<T>) -> Self {
        Self {
            index: 0,
            collection: value,
        }
    }
}

impl<T> FromIterator<T> for CopyRing<T>
where
    T: Copy,
{
    fn from_iter<A: IntoIterator<Item = T>>(iter: A) -> Self {
        Self {
            index: 0,
            collection: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::VecDeque;

    use super::CopyRing;

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
    fn first_next_equals_first_value() {
        let mut ring = CopyRing::from(vec![2, 5, 7, 8]);
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
        let mut right = CopyRing::from_iter(left.clone());

        assert_eq!(*left.last().unwrap(), right.prev().unwrap());
    }

    #[test]
    fn first_cycle_equals_origin_iter() {
        let left = 0..100;
        let mut right = CopyRing::from_iter(left.clone());

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
        let mut ring = CopyRing::from(vec![3]);

        for _ in 0..1000 {
            assert_eq!(3, ring.next().unwrap());
        }
    }

    #[test]
    fn zero_items_returns_none() {
        let mut ring: CopyRing<i32> = CopyRing::from(vec![]);

        assert!(ring.next().is_none())
    }
}