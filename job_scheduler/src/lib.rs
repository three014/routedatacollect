use futures::{future::BoxFuture, Future};

mod job;
mod runner;
pub mod scheduler;

pub type Result =
    core::result::Result<(), Box<dyn std::error::Error + core::marker::Send + core::marker::Sync>>;
pub type JobId = u32;
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
        peeked: Option<T>,
    }

    impl<T> PeekableReciever<T> {
        pub fn from_receiver(rx: mpsc::Receiver<T>) -> Self {
            Self { rx, peeked: None }
        }
        pub fn peek(&mut self) -> Option<&T> {
            if self.peeked.is_some() {
                self.peeked.as_ref()
            } else {
                match self.rx.try_recv() {
                    Ok(value) => {
                        self.peeked = Some(value);
                        self.peeked.as_ref()
                    }
                    Err(_) => None,
                }
            }
        }

        pub fn try_recv(&mut self) -> Result<T, mpsc::TryRecvError> {
            if let Some(value) = self.peeked.take() {
                Ok(value)
            } else {
                self.rx.try_recv()
            }
        }
    }
}
