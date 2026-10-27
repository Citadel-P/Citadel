use std::sync::Arc;
use std::time::Duration;

use citadel_backups::{BackupError, BackupService};
use tokio_util::sync::CancellationToken;

pub async fn backup_runs(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    options: crate::config::BackupWorkerConfig,
    wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BackupError> {
    dispatch_runs(cancellation, service, false, options, wake).await
}

pub async fn restore_runs(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    options: crate::config::BackupWorkerConfig,
    wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BackupError> {
    dispatch_runs(cancellation, service, true, options, wake).await
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

async fn dispatch_runs(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
    restores: bool,
    options: crate::config::BackupWorkerConfig,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BackupError> {
    if !options.enabled {
        cancellation.cancelled().await;
        return Ok(());
    }
    let cancellation = cancellation.child_token();
    let mut controllers = tokio::task::JoinSet::new();
    let family = if restores {
        citadel_runtime::runtime_metrics::RuntimeWork::RestoreClaim
    } else {
        citadel_runtime::runtime_metrics::RuntimeWork::BackupClaim
    };
    let minimum = options.poll_interval;
    let mut delay = minimum;
    while !cancellation.is_cancelled() {
        while let Some(result) = controllers.try_join_next() {
            report_controller(result, restores);
        }
        // Only this dispatcher polls its queue. The claim carries a shared
        // tracking permit, including while completion persistence is retried.
        if controllers.len() < options.tracking_slots {
            let claim = {
                let _iteration = family.start();
                if restores {
                    service.claim_restore(&cancellation).await
                } else {
                    service.claim_backup(&cancellation).await
                }
            };
            match claim {
                Ok(Some(claim)) => {
                    family.units(1);
                    let service = service.clone();
                    let token = cancellation.clone();
                    // Complete claims even if shutdown arrived during SQL; every
                    // claimed run gets an owned controller and cleanup path.
                    controllers.spawn(async move { service.execute_claim(claim, &token).await });
                    delay = minimum;
                    continue;
                }
                Ok(None) => {}
                Err(error) => tracing::error!(%error, restores, "Backup dispatch failed"),
            }
            delay = citadel_runtime::worker_poll_delay(delay, minimum, false);
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            result = controllers.join_next(), if !controllers.is_empty() => {
                if let Some(result) = result { report_controller(result, restores); }
            }
            () = super::notifications::wait(&mut wake, &cancellation, delay.max(Duration::from_secs(30))) => {},
        }
    }
    // Shutdown cancels controllers cooperatively and waits for their cleanup.
    while let Some(result) = controllers.join_next().await {
        report_controller(result, restores);
    }
    Ok(())
}

fn report_controller(
    result: Result<Result<bool, BackupError>, tokio::task::JoinError>,
    restores: bool,
) {
    match result {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => tracing::error!(%error, restores, "Backup controller failed"),
        Err(error) => tracing::error!(%error, restores, "Backup controller task failed"),
    }
}

pub async fn cleanup_reservations(
    cancellation: CancellationToken,
    service: Arc<BackupService>,
) -> Result<(), BackupError> {
    let mut cursor = None;
    while !cancellation.is_cancelled() {
        match service.reconcile_cleanup(cursor, &cancellation).await {
            Ok(next) => cursor = next,
            Err(error) => tracing::warn!(%error, "Backup cleanup reconciliation failed"),
        }
        tokio::select! { ()=cancellation.cancelled()=>break, ()=tokio::time::sleep(Duration::from_secs(30))=>{} }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
