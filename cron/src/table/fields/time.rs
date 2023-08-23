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
    pub fn after(&mut self, sec: u8) -> (u8, bool) {
        self.0.reset();
        super::after(&mut self.0, false, sec)
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
    pub fn after(&mut self, min: u8, sec_overflow: bool) -> (u8, bool) {
        self.0.reset();
        super::after(&mut self.0, sec_overflow, min)
    }
}

impl Hours {
    pub const fn new(copy_ring: CronRing) -> Self {
        Self(copy_ring)
    }

    /// Returns the first hour that occurs after the given
    /// number of hours. 
    ///
    /// If the inner buffer wrapped back to the earliest hour,
    /// then overflow has occurred and the bool is `true`.
    ///
    /// Otherwise, the bool is `false` and no overflow
    /// has occurred.
    pub fn after(&mut self, hr: u8, min_overflow: bool) -> (u8, bool) {
        self.0.reset();
        super::after(&mut self.0, min_overflow, hr)
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

        let next = seconds.after(now.second() as u8);
        match next.1 {
            true => assert!((next.0 as u32) < now.second()),
            false => assert!((next.0 as u32) >= now.second()),
        }
    }

    #[test]
    fn first_after_works_for_mins_no_overflow() {
        let mut minutes =
            Minutes::new(CopyRing::arc_with_size(gen_range_mins_or_secs().into()));
        let now = Utc::now();

        let next = minutes.after(now.minute() as u8, false);
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

            let next = minutes.after(now2, true);
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

            let next = hours.after(now2, true);
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
