use std::future::Future;
use std::time::Duration;

use tokio::task::{JoinError, JoinSet};
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum SupervisedTaskError {
    #[error("task '{name}' failed: {message}")]
    Failed { name: &'static str, message: String },
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
    tasks: JoinSet<(&'static str, Result<(), String>)>,
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
        E: std::fmt::Display + Send + 'static,
    {
        self.tasks
            .spawn(async move { (name, task.await.map_err(|error| error.to_string())) });
    }

    pub async fn wait_for_exit(&mut self) -> Result<(), SupervisedTaskError> {
        match self.tasks.join_next().await {
            Some(Ok((_name, Ok(())))) if self.cancellation.is_cancelled() => Ok(()),
            Some(Ok((name, Ok(())))) => Err(SupervisedTaskError::Exited { name }),
            Some(Ok((name, Err(message)))) => Err(SupervisedTaskError::Failed { name, message }),
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
            while let Some(result) = self.tasks.join_next().await {
                match result {
                    Ok((_name, Ok(()))) => {}
                    Ok((name, Err(message))) => {
                        return Err(SupervisedTaskError::Failed { name, message });
                    }
                    Err(source) => {
                        return Err(SupervisedTaskError::Join {
                            name: "unknown",
                            source,
                        });
                    }
                }
            }
            Ok(())
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
