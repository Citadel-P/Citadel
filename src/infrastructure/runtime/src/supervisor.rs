use std::error::Error;
use std::future::Future;
type TaskFailure = Box<dyn Error + Send + Sync>;
use std::time::Duration;

use tokio::task::{JoinError, JoinSet};
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum SupervisedTaskError {
    #[error("task '{name}' failed: {source}")]
    Failed {
        name: &'static str,
        #[source]
        source: TaskFailure,
    },
    #[error("task '{name}' panicked or was cancelled: {source}")]
    Join {
        name: &'static str,
        #[source]
        source: JoinError,
    },
    #[error("task '{name}' exited before supervisor cancellation")]
    Exited { name: &'static str },
    #[error("the supervisor has no owned tasks")]
    Empty,
    #[error("supervisor shutdown exceeded {0:?}")]
    ShutdownTimeout(Duration),
}

pub struct TaskSupervisor {
    cancellation: CancellationToken,
    tasks: JoinSet<(&'static str, Result<(), TaskFailure>)>,
}

impl TaskSupervisor {
    #[must_use]
    pub fn new(cancellation: CancellationToken) -> Self {
        Self {
            cancellation,
            tasks: JoinSet::new(),
        }
    }

    pub fn spawn<F, E>(&mut self, name: &'static str, task: F)
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: Into<TaskFailure> + Send + 'static,
    {
        self.tasks
            .spawn(async move { (name, task.await.map_err(Into::into)) });
    }

    pub async fn wait_for_exit(&mut self) -> Result<(), SupervisedTaskError> {
        match self.tasks.join_next().await {
            Some(Ok((_name, Ok(())))) if self.cancellation.is_cancelled() => Ok(()),
            Some(Ok((name, Ok(())))) => Err(SupervisedTaskError::Exited { name }),
            Some(Ok((name, Err(source)))) => Err(SupervisedTaskError::Failed { name, source }),
            Some(Err(source)) => Err(SupervisedTaskError::Join {
                name: "unknown",
                source,
            }),
            None => Err(SupervisedTaskError::Empty),
        }
    }

    pub async fn shutdown(mut self, timeout: Duration) -> Result<(), SupervisedTaskError> {
        self.cancellation.cancel();

        let drain = async {
            let mut failure = None;
            while let Some(result) = self.tasks.join_next().await {
                match result {
                    Ok((_name, Ok(()))) => {}
                    Ok((name, Err(source))) => {
                        failure.get_or_insert(SupervisedTaskError::Failed { name, source });
                    }
                    Err(source) => {
                        failure.get_or_insert(SupervisedTaskError::Join {
                            name: "unknown",
                            source,
                        });
                    }
                }
            }
            failure.map_or(Ok(()), Err)
        };

        match tokio::time::timeout(timeout, drain).await {
            Ok(result) => result,
            Err(_) => {
                self.tasks.abort_all();
                while self.tasks.join_next().await.is_some() {}
                Err(SupervisedTaskError::ShutdownTimeout(timeout))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn task_failures_preserve_the_original_error_source() {
        let mut supervisor = TaskSupervisor::new(CancellationToken::new());
        supervisor.spawn("io", async {
            Err::<(), _>(std::io::Error::other("disk unavailable"))
        });
        let error = supervisor.wait_for_exit().await.unwrap_err();
        assert!(matches!(
            &error,
            SupervisedTaskError::Failed { name: "io", .. }
        ));
        let source = std::error::Error::source(&error).unwrap();
        assert!(source.downcast_ref::<std::io::Error>().is_some());
        assert_eq!(source.to_string(), "disk unavailable");
    }

    #[tokio::test]
    async fn shutdown_cancels_and_joins_owned_tasks() {
        let cancellation = CancellationToken::new();
        let observed = Arc::new(AtomicBool::new(false));
        let mut supervisor = TaskSupervisor::new(cancellation.clone());
        let task_observed = Arc::clone(&observed);
        supervisor.spawn("fixture", async move {
            cancellation.cancelled().await;
            task_observed.store(true, Ordering::Release);
            Ok::<_, std::convert::Infallible>(())
        });

        supervisor
            .shutdown(Duration::from_secs(1))
            .await
            .expect("owned task should stop");

        assert!(observed.load(Ordering::Acquire));
    }

    // TrackedBackgroundTasks.DrainAsync waits for all executions, even when one fails.
    #[tokio::test]
    async fn shutdown_drains_cleanup_after_another_worker_fails() {
        let cancellation = CancellationToken::new();
        let mut supervisor = TaskSupervisor::new(cancellation.clone());
        let cleaned = Arc::new(AtomicBool::new(false));
        let observed = cleaned.clone();
        supervisor.spawn("failed", async { Err::<(), _>("failure") });
        supervisor.spawn("cleanup", async move {
            cancellation.cancelled().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            observed.store(true, Ordering::Release);
            Ok::<_, std::convert::Infallible>(())
        });
        assert!(matches!(
            supervisor.shutdown(Duration::from_secs(1)).await,
            Err(SupervisedTaskError::Failed { name: "failed", .. })
        ));
        assert!(cleaned.load(Ordering::Acquire));
    }

    // Port: TrackedBackgroundTasksTests.DrainAsync_ShouldRespectShutdownDeadline.
    #[tokio::test]
    async fn shutdown_respects_deadline_for_an_unresponsive_worker() {
        let mut supervisor = TaskSupervisor::new(CancellationToken::new());
        supervisor.spawn(
            "never",
            std::future::pending::<Result<(), std::convert::Infallible>>(),
        );
        let started = tokio::time::Instant::now();
        assert!(matches!(
            supervisor.shutdown(Duration::from_millis(20)).await,
            Err(SupervisedTaskError::ShutdownTimeout(_))
        ));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn supervisor_observes_an_unexpected_task_exit() {
        let cancellation = CancellationToken::new();
        let mut supervisor = TaskSupervisor::new(cancellation);
        supervisor.spawn("fixture", async { Ok::<_, std::convert::Infallible>(()) });

        assert!(matches!(
            supervisor.wait_for_exit().await,
            Err(SupervisedTaskError::Exited { name: "fixture" })
        ));
    }
}
