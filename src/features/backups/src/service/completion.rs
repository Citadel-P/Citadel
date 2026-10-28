use super::*;
use std::{future::Future, time::Duration};

/// Retry persistence of an already computed result, never external execution.
pub(super) async fn persist<T, F, Fut>(
    run_id: Uuid,
    shutdown: &CancellationToken,
    mut completion: F,
) -> Result<Option<T>, BackupError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<T>, BackupError>>,
{
    let mut delay = Duration::from_millis(250);
    loop {
        // Give an in-flight commit a chance to finish even during shutdown.
        match tokio::time::timeout(Duration::from_secs(10), completion()).await {
            Ok(Ok(confirmed)) => return Ok(confirmed),
            Ok(Err(BackupError::Storage(error))) => {
                tracing::warn!(%error, %run_id, "Backup operation completion pending");
            }
            Ok(Err(error)) => return Err(error),
            Err(_) => tracing::warn!(%run_id, "Backup operation completion timed out"),
        }
        tokio::select! {
            biased;
            () = shutdown.cancelled() => return Ok(None),
            () = tokio::time::sleep(delay) => {},
        }
        delay = (delay * 2).min(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, future::pending};
    use tokio::time::Instant;

    #[tokio::test(start_paused = true)]
    async fn timeout_and_storage_failures_share_capped_backoff() {
        let attempts = RefCell::new(Vec::new());
        let start = Instant::now();
        let result = persist(Uuid::now_v7(), &CancellationToken::new(), || {
            let mut attempts = attempts.borrow_mut();
            attempts.push(start.elapsed());
            let index = attempts.len();
            async move {
                match index {
                    1 | 2 => pending().await,
                    3..=7 => Err(BackupError::Storage("unavailable".into())),
                    _ => Ok(Some(42)),
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(result, Some(42));
        assert_eq!(
            *attempts.borrow(),
            [0, 10_250, 20_750, 21_750, 23_750, 27_750, 32_750, 37_750].map(Duration::from_millis)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_allows_one_commit_attempt_and_stops_retries() {
        let shutdown = CancellationToken::new();
        shutdown.cancel();
        let calls = std::cell::Cell::new(0);
        let result: Option<()> = persist(Uuid::now_v7(), &shutdown, || {
            calls.set(calls.get() + 1);
            async { Err(BackupError::Storage("unavailable".into())) }
        })
        .await
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(calls.get(), 1);
        assert_eq!(
            persist(Uuid::now_v7(), &shutdown, || async { Ok(Some(42)) })
                .await
                .unwrap(),
            Some(42)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn definitive_failure_is_not_retried() {
        let calls = std::cell::Cell::new(0);
        let result: Result<Option<()>, _> =
            persist(Uuid::now_v7(), &CancellationToken::new(), || {
                calls.set(calls.get() + 1);
                async { Err(BackupError::Conflict("superseded".into())) }
            })
            .await;
        assert!(matches!(result, Err(BackupError::Conflict(_))));
        assert_eq!(calls.get(), 1);
    }
}
