//! One generation per service, no per-repository channels or retained results.
use super::*;
use std::future::Future;
use tokio::sync::watch;

const FALLBACK: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(300);

pub(super) async fn bounded<T>(
    cancellation: &CancellationToken,
    work: impl Future<Output = Result<T, GitRepositoryExecutionError>>,
) -> Result<T, GitRepositoryExecutionError> {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => Err(GitError::Process(citadel_execution::ProcessError::Cancelled).into()),
        result = tokio::time::timeout(TIMEOUT, work) =>
            result.unwrap_or_else(|_| Err(GitError::Process(citadel_execution::ProcessError::Timeout(TIMEOUT)).into())),
    }
}

pub(super) async fn wait<F, R>(
    mut completion: watch::Receiver<()>,
    mut read: F,
) -> Result<String, GitRepositoryExecutionError>
where
    F: FnMut() -> R,
    R: Future<Output = Result<Option<GitRepositoryRef>, GitRepositoryExecutionError>>,
{
    loop {
        // Mark before reading, never after: a commit during the read must wake
        // another read if this one returns an older snapshot.
        completion.borrow_and_update();
        let reference = read().await?.ok_or(GitRepositoryExecutionError::NotFound)?;
        match reference.status {
            crate::GitRepositoryRefStatus::Healthy => {
                let commit = reference
                    .resolved_commit_sha
                    .ok_or(GitRepositoryExecutionError::NotSynchronized)?;
                if !is_full_object_id(&commit) {
                    return Err(GitRepositoryExecutionError::Validation(
                        "Repository returned an invalid commit ID.".into(),
                    ));
                }
                return Ok(commit);
            }
            crate::GitRepositoryRefStatus::Degraded => {
                return Err(GitRepositoryExecutionError::Validation(
                    "Repository synchronization failed. See its activity for details.".into(),
                ));
            }
            crate::GitRepositoryRefStatus::Pending | crate::GitRepositoryRefStatus::Syncing => {}
        }
        tokio::select! {
            _ = tokio::time::sleep(FALLBACK) => {},
            result = completion.changed() => if result.is_err() {
                // Closed channels must not become a hot loop.
                tokio::time::sleep(FALLBACK).await;
            },
        }
    }
}

#[cfg(test)]
mod tests;
