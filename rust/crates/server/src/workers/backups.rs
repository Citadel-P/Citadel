use std::sync::Arc;
use std::time::Duration;

use citadel_backups::{BackupError, BackupService};
use tokio_util::sync::CancellationToken;

pub async fn backup_runs(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
) -> Result<(), BackupError> {
    run_loop(cancellation, service, false).await
}

pub async fn restore_runs(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
) -> Result<(), BackupError> {
    run_loop(cancellation, service, true).await
}

pub async fn policy_scheduler(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
) -> Result<(), BackupError> {
    while !cancellation.is_cancelled() {
        if let Err(error) = service.queue_due_scheduled(chrono::Utc::now()).await {
            tracing::error!(%error, "Backup Policy scheduler iteration failed");
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(Duration::from_secs(30)) => {}
        }
    }
    Ok(())
}

async fn run_loop(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    restores: bool,
) -> Result<(), BackupError> {
    while !cancellation.is_cancelled() {
        let result = if restores {
            service.process_restore(&cancellation).await
        } else {
            service.process_backup(&cancellation).await
        };
        match result {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::error!(%error, restores, "Backup worker iteration failed"),
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(Duration::from_secs(2)) => {}
        }
    }
    Ok(())
}
