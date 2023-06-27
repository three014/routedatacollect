use crate::table::CronRing;

#[derive(Clone, Debug)]
pub struct Seconds(CronRing);

#[derive(Clone, Debug)]
pub struct Minutes(CronRing);

#[derive(Clone, Debug)]
pub struct Hours(CronRing);

impl Seconds {
    pub const fn new(copy_ring: CronRing) -> Self {
        Self(copy_ring)
    }

    /// Returns the first second that occurs after the given
    /// number of seconds. Rotates the inner buffer so that
    /// calling `next` yields the following value.
    ///
    /// If the inner buffer wrapped back to the earliest second,
    /// then overflow has occurred and the bool is `true`.
    ///
    /// Otherwise, the bool is `false` and no overflow
    /// has occurred.
    pub fn first_after(&mut self, sec: u8) -> (u8, bool) {
        self.0.reset();
        super::first_after(&mut self.0, false, sec)
    }

    /// Returns the next second in the inner
    /// buffer, along with whether overflow
    /// occurred. For seconds, overflow
    /// occurs when the seconds passes 59
    /// and wraps back to 0.
    pub fn next(&mut self) -> (u8, bool) {
        self.0.checked_next().unwrap()
    }
}

impl Minutes {
    pub const fn new(copy_ring: CronRing) -> Self {
        Self(copy_ring)
    }

    /// Returns the first minute that occurs after the given
    /// number of minutes. Rotates the inner buffer so that
    /// calling `next` yields the following value.
    ///
    /// If the inner buffer wrapped back to the earliest minute,
    /// then overflow has occurred and the bool is `true`.
    ///
    /// Otherwise, the bool is `false` and no overflow
    /// has occurred.
    pub fn first_after(&mut self, min: u8, sec_overflow: bool) -> (u8, bool) {
        self.0.reset();
        super::first_after(&mut self.0, sec_overflow, min)
    }

    /// Returns the next minute in the inner
    /// buffer, along with whether overflow
    /// occurred. For minutes, overflow
    /// occurs when the minutes passes 59
    /// and wraps back to 0.
    pub fn next(&mut self, sec_overflow: bool) -> (u8, bool) {
        super::next(&mut self.0, sec_overflow)
    }
}

impl Hours {
    pub const fn new(copy_ring: CronRing) -> Self {
        Self(copy_ring)
    }

    /// Returns the first hour that occurs after the given
    /// number of hours. Rotates the inner buffer so that
    /// calling `next` yields the following value.
    ///
    /// If the inner buffer wrapped back to the earliest hour,
    /// then overflow has occurred and the bool is `true`.
    ///
    /// Otherwise, the bool is `false` and no overflow
    /// has occurred.
    pub fn first_after(&mut self, hr: u8, min_overflow: bool) -> (u8, bool) {
        self.0.reset();
        super::first_after(&mut self.0, min_overflow, hr)
    }

    /// Returns the next hour in the inner
    /// buffer, along with whether overflow
    /// occurred. For hours, overflow
    /// occurs when the hours passes 23
    /// and wraps back to 0.
    pub fn next(&mut self, min_overflow: bool) -> (u8, bool) {
        super::next(&mut self.0, min_overflow)
    }
}

#[cfg(test)]
mod test {
    use super::{Hours, Minutes, Seconds};
    use crate::CopyRing;
    use chrono::{Timelike, Utc};
    use rand::Rng;

    const THRESHOLD: i32 = 50;
    const UPPER: i32 = 100;

    fn gen_range_mins_or_secs() -> Vec<u8> {
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
        v
    }

    fn gen_range_hours() -> Vec<u8> {
        let mut v = vec![];
        let mut rng = rand::thread_rng();
        for i in 0u8..24 {
            if rng.gen::<i32>() % UPPER > THRESHOLD {
                v.push(i)
            }
        }
        if v.is_empty() {
            v.push(rng.gen::<u8>() % 24)
        }
        v
    }

    #[test]
    fn first_after_works_for_secs() {
        let mut seconds =
            Seconds::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
        let now = Utc::now();

        let next = seconds.first_after(now.second() as u8);
        match next.1 {
            true => assert!((next.0 as u32) < now.second()),
            false => assert!((next.0 as u32) >= now.second()),
        }
    }

    #[test]
    fn next_for_seconds() {
        let mut secs = Seconds::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
        let mut rng = rand::thread_rng();
        let s = rng.gen::<u8>() % 60;
        let first = secs.first_after(s);
        eprintln!("First after {} seconds: {:?}", s, first);
        dbg!(secs.next());
    }

    #[test]
    fn first_after_works_for_mins_no_overflow() {
        let mut minutes =
            Minutes::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
        let now = Utc::now();

        let next = minutes.first_after(now.minute() as u8, false);
        match next.1 {
            true => assert!((next.0 as u32) < now.minute()),
            false => assert!((next.0 as u32) >= now.minute()),
        }
    }

    #[test]
    fn first_after_works_for_mins_overflow() {
        let mut minutes =
            Minutes::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
        for i in 0..60 {
            let now2 = i;

            let next = minutes.first_after(now2, true);
            //eprintln!("now: {} minutes", now2);
            //dbg!(next);
            //dbg!(&minutes);
            match next.1 {
                true => assert!((next.0) < now2),
                false => assert!((next.0) >= now2),
            }
        }
    }

    #[test]
    fn first_after_works_for_hours_overflow() {
        let mut hours = Hours::new(CopyRing::arc_with_size(gen_range_hours().into()));
        for i in 0..24 {
            let now2 = i;

            let next = hours.first_after(now2, true);
            //eprintln!("now: {} hours", now2);
            //dbg!(next);
            //dbg!(&hours);
            match next.1 {
                true => assert!((next.0) < now2),
                false => assert!((next.0) >= now2),
            }
        }
    }
}
