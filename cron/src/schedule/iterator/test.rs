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
