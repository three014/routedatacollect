use super::{Days, Minutes, Seconds};
use crate::schedule::iterator::CopyRing;
use chrono::{Datelike, Timelike, Utc};
use rand::Rng;

const THRESHOLD: i32 = 50;
const UPPER: i32 = 100;

fn gen_range_days_of_month() -> impl Iterator<Item = u8> {
    let mut v = vec![];
    let mut rng = rand::thread_rng();
    for i in 1u8..=31 {
        if rng.gen::<i32>() % UPPER > THRESHOLD {
            v.push(i);
        }
    }
    if v.is_empty() {
        v.push(rng.gen::<u8>() % 31)
    }
    v.into_iter()
}

fn gen_range_days_of_week() -> impl Iterator<Item = u8> {
    let mut v = vec![];
    let mut rng = rand::thread_rng();
    for i in 0u8..7 {
        if rng.gen::<i32>() % UPPER > THRESHOLD {
            v.push(i);
        }
    }
    if v.is_empty() {
        v.push(rng.gen::<u8>() % 7)
    }
    v.into_iter()
}

fn gen_range_hours_or_mins_or_secs() -> impl Iterator<Item = u8> {
    let mut v = vec![];
    let mut rng = rand::thread_rng();
    for i in 0u8..60 {
        if rng.gen::<i32>() % UPPER > THRESHOLD {
            v.push(i)
        }
    }
    if v.is_empty() {
        v.push(rng.gen::<u8>() % 60)
    }
    v.into_iter()
}

#[test]
fn num_weekdays_since_returns_correct_day() {
    let sun_to_fri = Days::num_weekdays_since(0, 5);
    assert_eq!(5, sun_to_fri);

    let fri_to_sun = Days::num_weekdays_since(5, 0);
    assert_eq!(2, fri_to_sun);

    let wed_to_tues = Days::num_weekdays_since(3, 2);
    assert_eq!(6, wed_to_tues);

    let thurs_to_thurs = Days::num_weekdays_since(4, 4);
    assert_eq!(0, thurs_to_thurs);
}

#[test]
fn first_after_works_for_secs() {
    let mut seconds = Seconds::new(CopyRing::from_iter(gen_range_hours_or_mins_or_secs()));
    let now = Utc::now();

    let next = seconds.first_after(now.second() as u8);
    match next.1 {
        true => assert!((next.0 as u32) < now.second()),
        false => assert!((next.0 as u32) >= now.second()),
    }
}

#[test]
fn first_after_works_for_mins_no_overflow() {
    let mut minutes = Minutes::new(CopyRing::from_iter(gen_range_hours_or_mins_or_secs()));
    let now = Utc::now();

    let next = minutes.first_after(now.minute() as u8, false);
    match next.1 {
        true => assert!((next.0 as u32) < now.minute()),
        false => assert!((next.0 as u32) >= now.minute()),
    }
}

#[test]
fn first_after_works_for_mins_overflow() {
    let mut minutes = Minutes::new(CopyRing::from_iter(gen_range_hours_or_mins_or_secs()));
    let now = Utc::now();

    let next = minutes.first_after(now.minute() as u8, true);
    dbg!(next);
    dbg!(minutes);
    match next.1 {
        true => assert!((next.0 as u32) < now.minute()),
        false => assert!((next.0 as u32) >= now.minute()),
    }
}

#[test]
fn first_after_days_both_spec() {
    let mut days = Days::Both {
        week: CopyRing::from_iter(gen_range_days_of_week()),
        month: CopyRing::from_iter(gen_range_days_of_month()),
    };

    let now = Utc::now();
    let next = days.first_after(
        now.day() as u8,
        now.weekday().num_days_from_sunday() as u8,
        false,
        now.month() as u8,
        now.year() as u32,
    );

    //dbg!(days);
    //println!("{}, {:?}", now, next);
}
