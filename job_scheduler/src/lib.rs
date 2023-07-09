pub use self::scheduler::Scheduler;
use chrono::NaiveDateTime;
use futures::{future::BoxFuture, Future};

mod job;
mod runner;
mod scheduler;

pub type Result =
    core::result::Result<(), Box<dyn std::error::Error + core::marker::Send + core::marker::Sync>>;
pub type JobId = u32;

pub enum Limit {
    NumTimes(usize),
    EndDate(NaiveDateTime),
}

/// Adapted from Ibraheem Ahmed's solution on https://stackoverflow.com, Feb 5, 2021.
/// An implementation to store async functions as trait objects in structs.
pub trait AsyncFn {
    fn call(&self) -> BoxFuture<'static, Result>;
}

impl<T, F> AsyncFn for T
where
    T: (FnOnce() -> F) + Clone + Send + 'static,
    F: Future<Output = Result> + Send + 'static,
{
    fn call(&self) -> BoxFuture<'static, Result> {
        Box::pin(self.clone()())
    }
}

mod receiver {
    use std::sync::mpsc;

    /// Adapted from kpreid's solution on https://users.rust-lang.org, March, 2022.
    pub struct PeekableReciever<T> {
        rx: mpsc::Receiver<T>,
        peeked: Option<Result<T, mpsc::TryRecvError>>,
    }

    impl<T> PeekableReciever<T> {
        pub fn from_receiver(rx: mpsc::Receiver<T>) -> Self {
            Self { rx, peeked: None }
        }

        pub fn peek(&mut self) -> Result<&T, &mpsc::TryRecvError> {
            if self.peeked.is_some() {
                self.peeked.as_ref().unwrap().as_ref()
            } else {
                self.peeked = Some(self.rx.try_recv());
                self.peeked.as_ref().unwrap().as_ref()
            }
        }

        pub fn try_recv(&mut self) -> Result<T, mpsc::TryRecvError> {
            if let Some(item) = self.peeked.take() {
                item
            } else {
                self.rx.try_recv()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::AsyncFn;

    #[allow(unused)]
    #[test]
    fn foo() {
        #[tokio::main]
        async fn bar() {
            let mut g = "hello".to_owned();

            let a = move || async move {
                g.push_str(" world!");
                println!("{g}");
                Ok(())
            };

            let a2 = move || async move {
                println!("This is the second future type!");

                Ok(())
            };

            struct Hmm {
                f: Box<dyn AsyncFn>,
            }

            impl AsyncFn for Hmm {
                fn call(&self) -> futures::future::BoxFuture<'static, crate::Result> {
                    self.f.as_ref().call()
                }
            }

            let b = Hmm { f: Box::new(a) };
            let c = Hmm { f: Box::new(a2) };

            let v = vec![b, c];

            v[0].call().await;
            v[0].call().await;
            v[1].call().await;
        }

        bar();
    }
}
