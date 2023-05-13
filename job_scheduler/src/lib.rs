use futures::{future::BoxFuture, Future};
use job::JobResult;

mod job;
mod runner;
pub mod scheduler;

/// Credit: Ibraheem Ahmed on StackOverflow, Feb 5, 2021.
/// An implementation to store async functions as trait objects
/// in structs.
pub trait AsyncFn {
    fn call(&self) -> BoxFuture<'static, JobResult>;
}

impl<T, F> AsyncFn for T
where 
    T: (Fn() -> F) + Send + 'static,
    F: Future<Output = JobResult> + Send + 'static,
{
    fn call(&self) -> BoxFuture<'static, JobResult> {
        Box::pin(self())
    }
}