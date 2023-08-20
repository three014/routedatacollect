pub(super) use cycle_iter::CycleIter;
pub use cycle_iter_mut::{Checked, CycleIterMut};
mod cycle_iter_mut {
    use crate::collection::copy_ring::CopyRing;

    pub struct CycleIterMut<'a: 'b, 'b, T: Copy, const N: usize> {
        ring: &'b mut CopyRing<'a, T, N>,
        n: usize,
    }

    impl<'a: 'b, 'b, T: Copy, const N: usize> CycleIterMut<'a, 'b, T, N> {
        pub fn new(ring: &'b mut CopyRing<'a, T, N>, n: usize) -> Self {
            Self { ring, n }
        }
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
}

mod cycle_iter {
    pub struct CycleIter<'a, T: Copy> {
        ring_buf: &'a [T],
        index: usize,
        n: usize,
    }

    impl<'a, T: Copy> CycleIter<'a, T> {
        pub fn new(ring_buf: &'a [T], index: usize, n: usize) -> Self {
            Self { ring_buf, index, n }
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
}
