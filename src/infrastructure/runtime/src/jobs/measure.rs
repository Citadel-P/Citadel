//! Observe an existing operation without changing its cancellation/retry policy.
use super::{JobHandle, JobOutcome};
use std::future::Future;

pub async fn observe<T, E>(future: impl Future<Output = Result<T, E>>) -> Result<T, E> {
    observe_work(future, |_| None).await
}
pub async fn observe_work<T, E>(
    future: impl Future<Output = Result<T, E>>,
    work: impl FnOnce(&T) -> Option<(u64, &'static str)>,
) -> Result<T, E> {
    let iteration = JobHandle::iteration();
    let result = future.await;
    if let Some(iteration) = iteration {
        let units = result.as_ref().ok().and_then(work);
        let outcome = match (&result, units) {
            (Err(_), _) => JobOutcome::Failed,
            (Ok(_), Some((0, _))) => JobOutcome::NoWork,
            (Ok(_), Some(_)) => JobOutcome::Productive,
            (Ok(_), None) => JobOutcome::Completed,
        };
        iteration.finish(outcome, units);
    }
    result
}
