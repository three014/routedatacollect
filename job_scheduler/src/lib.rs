use futures::{future::BoxFuture, Future};

mod job;
mod runner;
pub mod scheduler;


pub type Result = core::result::Result<(), Box<dyn std::error::Error + core::marker::Send>>;

/// Adapted from Ibraheem Ahmed's solution on StackOverflow, Feb 5, 2021.
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