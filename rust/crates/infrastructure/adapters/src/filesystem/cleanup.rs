//! Cleanup is admitted before a workspace exists and remains tracked through Drop.
use citadel_runtime::{DynamicTaskReservation, DynamicTasks};
use std::{
    io,
    path::{Path, PathBuf},
};

pub(super) struct DirectoryCleanup {
    path: PathBuf,
    reservation: Option<DynamicTaskReservation>,
}
impl DirectoryCleanup {
    pub fn reserve(tasks: &DynamicTasks) -> io::Result<DynamicTaskReservation> {
        tasks.reserve().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Interrupted,
                "Workspace cleanup is shutting down",
            )
        })
    }
    pub fn new(path: PathBuf, reservation: DynamicTaskReservation) -> Self {
        Self {
            path,
            reservation: Some(reservation),
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn disarm(&mut self) {
        self.reservation.take();
    }
    pub async fn cleanup(&mut self) -> io::Result<()> {
        match tokio::fs::remove_dir_all(&self.path).await {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        self.disarm();
        Ok(())
    }
}
impl Drop for DirectoryCleanup {
    fn drop(&mut self) {
        if let Some(reservation) = self.reservation.take() {
            let path = self.path.clone();
            reservation.spawn(async move {
                let result =
                    tokio::task::spawn_blocking(move || std::fs::remove_dir_all(path)).await;
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => {}
                    result => tracing::error!(
                        ?result,
                        "Workspace cleanup failed; directory retained for recovery"
                    ),
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn dropped_workspace_keeps_cleanup_tracked_without_blocking_async_executor() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        runtime.block_on(async {
            let path =
                std::env::temp_dir().join(format!("citadel-cleanup-{}", uuid::Uuid::now_v7()));
            std::fs::create_dir(&path).unwrap();
            std::fs::write(path.join("secret"), "sensitive script").unwrap();
            let shutdown = CancellationToken::new();
            let tasks = DynamicTasks::new(shutdown.clone());
            let cleanup =
                DirectoryCleanup::new(path.clone(), DirectoryCleanup::reserve(&tasks).unwrap());
            let (release, held) = std::sync::mpsc::channel();
            let (started, running) = tokio::sync::oneshot::channel();
            let blocking = tokio::task::spawn_blocking(move || {
                started.send(()).unwrap();
                held.recv_timeout(Duration::from_secs(5)).unwrap();
            });
            running.await.unwrap();
            shutdown.cancel();
            drop(cleanup);
            // The only blocking worker is occupied. Async timers and shutdown
            // still advance; drain must account for the queued removal.
            assert!(tasks.drain(Duration::from_millis(20)).await.is_err());
            assert!(path.exists());
            assert_eq!(tasks.active(), 1);
            release.send(()).unwrap();
            blocking.await.unwrap();
            tasks.drain(Duration::from_secs(5)).await.unwrap();
            assert!(!path.exists());
        });
    }
}
