//! Process-owned durable work. The database remains the queue and recovery authority.
use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::{
    sync::CancellationToken,
    task::{TaskTracker, task_tracker::TaskTrackerToken},
};

/// Tracks a resource from admission through deferred cleanup, including cleanup
/// started by Drop after shutdown has stopped accepting new operations.
pub struct DynamicTaskReservation(TaskTrackerToken);

impl DynamicTaskReservation {
    pub fn spawn(self, cleanup: impl Future<Output = ()> + Send + 'static) {
        tokio::spawn(async move {
            let _registration = self.0;
            cleanup.await;
        });
    }
}

#[derive(Clone)]
pub struct DynamicTasks {
    tracker: TaskTracker,
    admission: Arc<Mutex<bool>>,
    cancellation: CancellationToken,
}

impl DynamicTasks {
    pub fn new(cancellation: CancellationToken) -> Self {
        Self {
            tracker: TaskTracker::new(),
            admission: Arc::new(Mutex::new(true)),
            cancellation,
        }
    }

    /// The gate and registration are atomic with respect to shutdown. TaskTracker::close
    /// alone is not an admission gate. Callers retain durable recovery on rejection.
    pub fn spawn<F, E>(&self, name: &'static str, operation: F) -> bool
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        let admission = self
            .admission
            .lock()
            .expect("dynamic task admission poisoned");
        if !*admission || self.cancellation.is_cancelled() {
            return false;
        }
        self.tracker.spawn(async move {
            if let Err(error) = operation.await {
                tracing::error!(task = name, %error, "Dynamic operation failed; durable recovery remains authoritative");
            }
        });
        true
    }

    /// A child token lets a task observe shutdown without cancelling the process.
    pub fn cancellation(&self) -> CancellationToken {
        self.cancellation.child_token()
    }

    /// Reserve before acquiring an external resource. Dropping the reservation
    /// releases it; transferring it to cleanup keeps drain waiting without a gap.
    pub fn reserve(&self) -> Option<DynamicTaskReservation> {
        let admission = self
            .admission
            .lock()
            .expect("dynamic task admission poisoned");
        if !*admission || self.cancellation.is_cancelled() {
            return None;
        }
        Some(DynamicTaskReservation(self.tracker.token()))
    }

    pub fn active(&self) -> usize {
        self.tracker.len()
    }

    /// Stop listener/worker producers first. Shared pools must stay open while this drains.
    pub async fn drain(&self, budget: Duration) -> Result<(), tokio::time::error::Elapsed> {
        *self
            .admission
            .lock()
            .expect("dynamic task admission poisoned") = false;
        self.tracker.close();
        tokio::time::timeout(budget, self.tracker.wait()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    #[tokio::test]
    async fn admitted_cleanup_remains_tracked_after_shutdown_closes_admission() {
        let shutdown = CancellationToken::new();
        let tasks = DynamicTasks::new(shutdown.clone());
        let reservation = tasks.reserve().unwrap();
        shutdown.cancel();
        assert!(tasks.reserve().is_none());
        assert!(tasks.drain(Duration::from_millis(1)).await.is_err());
        let (release, released) = tokio::sync::oneshot::channel();
        reservation.spawn(async move {
            let _ = released.await;
        });
        assert_eq!(tasks.active(), 1);
        assert!(tasks.drain(Duration::from_millis(1)).await.is_err());
        release.send(()).unwrap();
        tasks.drain(Duration::from_secs(1)).await.unwrap();
        assert_eq!(tasks.active(), 0);
    }

    #[tokio::test]
    async fn drains_cleanup_and_rejects_late_producers() {
        let token = CancellationToken::new();
        let tasks = DynamicTasks::new(token.clone());
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = done.clone();
        assert!(tasks.spawn("cleanup", async move {
            token.cancelled().await;
            observed.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok::<_, Infallible>(())
        }));
        tasks.cancellation.cancel();
        tasks.drain(Duration::from_secs(1)).await.unwrap();
        assert!(done.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(tasks.active(), 0);
        assert!(!tasks.spawn("late", async { Ok::<_, Infallible>(()) }));
    }

    #[tokio::test]
    async fn drain_has_a_deadline() {
        let tasks = DynamicTasks::new(CancellationToken::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let waiting = release.clone();
        tasks.spawn("slow", async move {
            waiting.notified().await;
            Ok::<_, Infallible>(())
        });
        assert!(tasks.drain(Duration::from_millis(10)).await.is_err());
        assert!(!tasks.spawn("closed", async { Ok::<_, Infallible>(()) }));
        release.notify_one();
        tasks.drain(Duration::from_secs(1)).await.unwrap();
    }
}
