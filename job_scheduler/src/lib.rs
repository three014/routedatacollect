use chrono::NaiveDateTime;
pub use scheduler::Scheduler;
use std::future::Future;

mod career;
mod runner;
mod scheduler;
pub(crate) mod utils;

pub type Result =
    core::result::Result<(), Box<dyn std::error::Error + core::marker::Send + core::marker::Sync>>;
pub type BoxFuture<'a, T> = std::pin::Pin<std::boxed::Box<dyn Future<Output = T> + Send + 'a>>;

pub enum Limit {
    NumTimes(u64),
    EndDate(NaiveDateTime),
}

/// Adapted from Ibraheem Ahmed's solution on https://stackoverflow.com, Feb 5, 2021.
/// An implementation to store async functions as trait objects in structs.
pub trait AsyncFn {
    /// Calls the async function and stores the future in the heap with
    /// a pinned box.
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

unsafe trait UnsafeAsyncFn {
    unsafe fn call_clone(&self) -> BoxFuture<'static, Result>;
}
