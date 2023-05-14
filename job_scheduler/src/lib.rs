use futures::{future::BoxFuture, Future};
use job::JobResult;

mod job;
mod runner;
pub mod scheduler;

/// Adapted from Ibraheem Ahmed's solution on StackOverflow, Feb 5, 2021.
/// An implementation to store async functions as trait objects in structs.
pub trait AsyncFn {
    fn call(&self) -> BoxFuture<'static, JobResult>;
}

impl<T, F> AsyncFn for T
where 
    T: (FnOnce() -> F) + Clone + Send + 'static,
    F: Future<Output = JobResult> + Send + 'static,
{
    fn call(&self) -> BoxFuture<'static, JobResult> {
        Box::pin(self.clone()())
    }
}