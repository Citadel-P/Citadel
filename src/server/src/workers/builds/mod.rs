use std::sync::Arc;
use std::time::Duration;

use citadel_builds::{BuildError, BuildService};
use tokio_util::sync::CancellationToken;

pub async fn build_consumers(
    cancellation: CancellationToken,
    service: citadel_builds::BuildCompletionService,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BuildError> {
    while !cancellation.is_cancelled() {
        let result = {
            let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::BuildCompletion.start();
            service.process_batch(&cancellation).await
        };
        match result {
            Ok(count) if count > 0 => continue,
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "Build consumer propagation failed"),
        }
        super::notifications::wait(&mut wake, &cancellation, Duration::from_secs(30)).await;
    }
    Ok(())
}

pub async fn cleanup_reservations(
    cancellation: CancellationToken,
    service: Arc<BuildService>,
) -> Result<(), BuildError> {
    let mut cursor = None;
    while !cancellation.is_cancelled() {
        match service.reconcile_cleanup(cursor, &cancellation).await {
            Ok(next) => cursor = next,
            Err(_) => {
                tracing::warn!("Build cleanup reconciliation could not read pending reservations")
            }
        }
        tokio::select! {
            () = cancellation.cancelled() => break,
            () = tokio::time::sleep(Duration::from_secs(30)) => {}
        }
    }
    Ok(())
}

pub async fn pool_health(
    cancellation: CancellationToken,
    service: Arc<BuildService>,
    edge: Arc<dyn citadel_builds::BuildPoolConnections>,
) -> Result<(), BuildError> {
    while !cancellation.is_cancelled() {
        if let Err(error) = edge.expire_connections().await {
            tracing::warn!(%error, "Build Pool connection reconciliation failed");
        }
        if let Err(error) = service.monitor_pool_health(&cancellation).await {
            tracing::error!(%error,"Build Pool health monitoring failed");
        }
        tokio::select! {()=cancellation.cancelled()=>break,()=tokio::time::sleep(Duration::from_secs(30))=>{}}
    }
    Ok(())
}

pub async fn build_runs(
    cancellation: CancellationToken,
    service: Arc<BuildService>,
    tracking_slots: usize,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), BuildError> {
    let cancellation = cancellation.child_token();
    let mut controllers = tokio::task::JoinSet::new();
    let result = async {
        while !cancellation.is_cancelled() {
            while let Some(result) = controllers.try_join_next() {
                controller_result(result)?;
            }
            let mut preparation_busy = false;
            if controllers.len() < tracking_slots {
                if let Some(preparation) = service.try_preparation() {
                    // Complete this short durable claim even if shutdown arrives.
                    // A returned claim is always handed to an owned controller.
                    if let Some(claim) = service.claim_next().await? {
                        citadel_runtime::runtime_metrics::RuntimeWork::BuildPreparation.units(1);
                        let service = service.clone();
                        let cancellation = cancellation.clone();
                        controllers.spawn(async move {
                            let _tracking = citadel_runtime::runtime_metrics::RuntimeWork::BuildTracking.start();
                            service.execute_claim(claim, preparation, &cancellation).await
                        });
                        continue;
                    }
                } else { preparation_busy = true; }
            }
            tokio::select! {
                () = cancellation.cancelled() => break,
                result = controllers.join_next(), if !controllers.is_empty() => {
                    if let Some(result) = result { controller_result(result)?; }
                }
                () = service.preparation_available(), if preparation_busy => {},
                _ = super::notifications::wait(&mut wake, &cancellation, Duration::from_secs(2)) => {},
            }
        }
        Ok(())
    }.await;
    if result.is_err() {
        cancellation.cancel();
    }
    // No detached streams or dropped result persistence on normal shutdown.
    while let Some(completed) = controllers.join_next().await {
        if let Err(error) = controller_result(completed) {
            tracing::error!(%error, "Build controller failed while draining");
        }
    }
    result
}

fn controller_result(
    result: Result<Result<(), BuildError>, tokio::task::JoinError>,
) -> Result<(), BuildError> {
    result.map_err(|error| BuildError::Storage(format!("Build controller failed: {error}")))?
}

#[cfg(test)]
mod tests;
