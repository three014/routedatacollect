pub mod copy_ring;
pub mod iter;

pub(self) fn new_idx_left(cur: usize, n: usize, period: usize) -> usize {
    (cur + n) % period
}

pub(self) fn new_idx_right(cur: usize, n: usize, period: usize) -> usize {
    let len = period as isize;
    let idx = cur as isize;
    let x = n as isize;
    ((-((x - idx) % len) + len) % len) as usize
}
