use std::{fmt::Display, future::Future, time::Duration};
use tokio_util::sync::CancellationToken;

/// Retry only the commit of a computed result. Even a draining worker gets one attempt.
pub async fn persist_completion<T, E: Display, F, Fut>(
    run_id: impl Display,
    shutdown: &CancellationToken,
    retryable: impl Fn(&E) -> bool,
    mut commit: F,
) -> Result<Option<T>, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut delay = Duration::from_millis(250);
    loop {
        match tokio::time::timeout(Duration::from_secs(10), commit()).await {
            Ok(Ok(value)) => return Ok(Some(value)),
            Ok(Err(error)) if retryable(&error) => {
                tracing::warn!(%error, %run_id, "Completion persistence pending")
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => tracing::warn!(%run_id, "Completion persistence timed out"),
        }
        tokio::select! {
            biased;
            () = shutdown.cancelled() => return Ok(None),
            () = tokio::time::sleep(delay) => {},
        }
        delay = (delay * 2).min(Duration::from_secs(5));
    }
}
