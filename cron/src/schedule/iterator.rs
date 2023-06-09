use super::Schedule;
use chrono::{DateTime, TimeZone};
use std::fmt::Debug;

pub struct ScheduleIter<'a, Tz: TimeZone> {
    schedule: &'a mut Schedule,
    next: Option<DateTime<Tz>>,
}

pub struct OwnedScheduleIter<Tz: TimeZone> {
    schedule: Schedule,
    next: Option<DateTime<Tz>>,
}

impl<'a, Tz: TimeZone> ScheduleIter<'a, Tz> {
    pub fn new(schedule: &'a mut Schedule, next: Option<DateTime<Tz>>) -> Self {
        Self { schedule, next }
    }
}

impl<Tz: TimeZone> OwnedScheduleIter<Tz> {
    pub fn new(schedule: Schedule, next: Option<DateTime<Tz>>) -> Self {
        Self { schedule, next }
    }
}

impl<'a, Tz: TimeZone + 'static> Iterator for ScheduleIter<'a, Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        let now = self.next.take()?;
        self.next = self.schedule.next(&now);
        Some(now)
    }
}

impl<Tz: TimeZone + 'static> Iterator for OwnedScheduleIter<Tz> {
    type Item = DateTime<Tz>;

    fn next(&mut self) -> Option<Self::Item> {
        let now = self.next.take()?;
        self.next = self.schedule.next(&now);
        Some(now)
    }
}

/// An infinite immutable ring-buffer.
///
/// The contents must implement
/// the `Copy` trait, so this is mostly
/// meant to be used with number and
/// enum types.
#[derive(Clone, Debug)]
pub struct CopyRing<T>
where
    T: Copy,
{
    index: usize,
    collection: Vec<T>,
    init: bool,
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
        self.set_init(false);
    }

    /// Rotates the ring to the left, yielding `Copyring::period`
    /// number of values in total. Dropping this iterator
    /// before it finishes will leave the `CopyRing` at
    /// wherever it was before the next iteration.
    pub fn one_cycle(&mut self) -> CycleIterMut<'_, T> {
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
    /// True if wrapping occurred, false otherwise.
    pub fn checked_next(&mut self) -> Option<(T, bool)> {
        let was_init = self.is_init();
        let prev_index = self.index;
        let next = self.next()?;
        // Bug: if the ring was created and we're on index 0,
        // then calling this function yields true, even though
        // we didn't wrap around.
        Some((next, prev_index == 0 && was_init))
    }

    /// Rotates the ring to the left, yielding each value until the
    /// ring reaches the start again.
    pub fn until_start(&mut self) -> CycleIterMut<'_, T> {
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
        self.rotate_right(1);
        Some(self.collection[self.index])
    }

    pub fn rotate_left(&mut self, n: usize) {
        self.index = (self.index + n) % self.period();
        self.set_init(true);
        //println!("Rotated left - new index: {}", self.index);
    }

    pub fn rotate_right(&mut self, n: usize) {
        let len = self.period() as isize;
        let idx = self.index as isize;
        let x = n as isize;
        self.index = ((-((x - idx) % len) + len) % len) as usize;
        self.set_init(true);
        //println!("Rotated right - new index: {}", self.index);
    }

    /// Returns the same value as `next` without
    /// advancing the ring's index.
    pub fn peek(&self) -> Option<T> {
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
    /// without mutating the ring, use `take` instead.
    pub fn take_mut(&mut self, n: usize) -> CycleIterMut<'_, T> {
        CycleIterMut { ring: self, n }
    }

    /// Takes the first n elements of the ring
    /// and iterates through it just like `take_mut`
    /// without mutating the ring itself.
    pub fn take(&self, n: usize) -> impl ExactSizeIterator<Item = T> + '_ {
        CycleIter {
            ring_buf: &self.collection,
            index: self.index,
            n,
        }
    }

    pub fn set_init(&mut self, init: bool) {
        self.init = init;
    }

    pub fn is_init(&self) -> bool {
        self.init
    }
}

pub struct CycleIterMut<'a, T: Copy> {
    ring: &'a mut CopyRing<T>,
    n: usize,
}

struct CycleIter<'a, T: Copy> {
    ring_buf: &'a Vec<T>,
    index: usize,
    n: usize,
}

impl<'a, T: Copy> CycleIterMut<'a, T> {
    pub fn checked_next(&mut self) -> Option<(T, bool)> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            self.ring.checked_next()
        }
    }

    pub fn checked(self) -> impl Iterator<Item = (T, bool)> + 'a {
        struct Checked<'a, T: Copy>(CycleIterMut<'a, T>);
        impl<'a, T: Copy> Iterator for Checked<'a, T> {
            type Item = (T, bool);

            fn next(&mut self) -> Option<Self::Item> {
                self.0.checked_next()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.0.size_hint()
            }
        }
        impl<'a, T: Copy> ExactSizeIterator for Checked<'a, T> {}
        Checked(self)
    }
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
            init: false,
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
            init: false,
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
            init: false,
        }
    }
}

#[cfg(test)]
mod test;
