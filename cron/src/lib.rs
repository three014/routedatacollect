use std::{collections::VecDeque, iter::Cycle, slice::Iter, str::FromStr};

use chrono::{DateTime, NaiveDateTime, NaiveTime, Timelike};
use iterator::CopyRing;

const SECOND: usize = 0;
const MINUTE: usize = 1;
const HOUR: usize = 2;
const DAY_OF_THE_MONTH: usize = 3;
const MONTH: usize = 4;
const DAY_OF_THE_WEEK: usize = 5;

#[derive(Clone, Debug)]
pub struct Schedule {
    fields: Vec<Field>,
}

#[derive(Clone, Debug)]
struct Field {
    every: CopyRing<u32>
}

pub enum Error {
    Empty,
    WrongNumberOfFields,
    InvalidMacro,
    Unknown,
}

impl FromStr for Schedule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields: Vec<&str> = s.split_whitespace().collect();
        Ok(match fields.len() {
            0 => Err(Error::Empty),
            1 => {
                let mut maybe_macro = fields[0].chars();
                maybe_macro
                    .next()
                    .ok_or(Error::Unknown)?
                    .eq(&'@')
                    .then(|| match maybe_macro.next().ok_or(Error::InvalidMacro)? {
                        'y' | 'a' => todo!("The @yearly/@annually macro."),
                        'm' => todo!(),
                        'w' => todo!(),
                        'd' => todo!(),
                        'h' => todo!(),
                        _ => Err(Error::InvalidMacro),
                    })
                    .unwrap_or(Err(Error::WrongNumberOfFields))
            }
            5 => unimplemented!("Will eventually be the equivalent of the 6-field version, but with '00' for seconds."),
            6 => todo!(),
            _ => Err(Error::WrongNumberOfFields),
        }?)
    }
}

fn foo() {
    let mut a = CopyRing::new(vec![15, 32, 50]);
    let b = a.next();
}

mod iterator {

    /// An immutable ring-buffer that
    /// cycles through its contents
    /// indefinitely.
    ///
    /// The contents must implement
    /// the `Copy` trait, so this is mostly
    /// meant to be used with number and
    /// enum types.
    ///
    /// Implements the `Iterator` trait,
    /// but will never end unless it's
    /// zipped with a finite iterator or
    /// cut short with `Iterator::take`,
    /// for instance.
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
        pub fn new(collection: impl Into<Vec<T>>) -> Self {
            Self {
                index: 0,
                collection: collection.into(),
            }
        }
    }

    impl<T> Iterator for CopyRing<T>
    where
        T: Copy,
    {
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            if self.index == self.collection.len() {
                self.index = 0;
            }
            let index = self.index;
            self.index += 1;
            Some(self.collection[index])
        }
    }

    #[cfg(test)]
    mod test {
        use super::CopyRing;

        #[test]
        fn first_cycle_equals_origin_vec() {
            let left = vec![1, 2, 3];
            let right: Vec<i32> = CopyRing::new(left.clone()).take(3).collect();

            assert_eq!(left, right)
        }
    }
}

#[cfg(test)]
mod tests {}
