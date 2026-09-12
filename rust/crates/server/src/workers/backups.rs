use std::sync::Arc;
use std::time::Duration;

use citadel_backups::{BackupError, BackupService};
use tokio_util::sync::CancellationToken;

pub async fn backup_runs(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    options: crate::config::BackupWorkerConfig,
) -> Result<(), BackupError> {
    run_parallel(cancellation, service, false, options).await
}

pub async fn restore_runs(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    options: crate::config::BackupWorkerConfig,
) -> Result<(), BackupError> {
    run_parallel(cancellation, service, true, options).await
}

pub async fn policy_scheduler(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    options: crate::config::BackupWorkerConfig,
) -> Result<(), BackupError> {
    while !cancellation.is_cancelled() {
        if options.enabled
            && let Err(error) = service.queue_due_scheduled(chrono::Utc::now()).await
        {
            tracing::error!(%error, "Backup Policy scheduler iteration failed");
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(options.schedule_interval) => {}
        }
    }
    Ok(())
}

async fn run_parallel(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    restores: bool,
    options: crate::config::BackupWorkerConfig,
) -> Result<(), BackupError> {
    if !options.enabled {
        cancellation.cancelled().await;
        return Ok(());
    }
    for result in futures_util::future::join_all((0..options.parallel_runs).map(|_| {
        run_loop(
            cancellation.clone(),
            service.clone(),
            restores,
            options.poll_interval,
        )
    }))
    .await
    {
        result?;
    }
    Ok(())
}

async fn run_loop(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    restores: bool,
    poll_interval: Duration,
) -> Result<(), BackupError> {
    let minimum = poll_interval;
    let mut delay = minimum;
    while !cancellation.is_cancelled() {
        let result = if restores {
            service.process_restore(&cancellation).await
        } else {
            service.process_backup(&cancellation).await
        };
        match result {
            Ok(true) => {
                delay = citadel_application::worker_poll_delay(delay, minimum, true);
                continue;
            }
            Ok(false) => {}
            Err(error) => tracing::error!(%error, restores, "Backup worker iteration failed"),
        }
        delay = citadel_application::worker_poll_delay(delay, minimum, false);
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(delay) => {}
        }
    }
    Ok(())
}
