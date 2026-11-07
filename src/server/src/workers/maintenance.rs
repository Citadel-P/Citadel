use crate::realtime::RealtimeHub;
use citadel_runtime::maintenance::{MaintenanceError, MaintenanceStore};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
    realtime: Option<RealtimeHub>,
    build_retention_days: Option<i32>,
    retention_interval: Duration,
) -> Result<(), std::convert::Infallible> {
    // Startup recovery already ran under the exclusive Core lease. Historical
    // retention is independent: its transactions cannot delay operation recovery.
    let recovery = async {
        let mut tick = super::schedule::interval("recovery", super::recovery::FALLBACK);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            match store.reconcile().await {
                Ok(changed) => {
                    if let Some(hub) = &realtime {
                        for resource in changed {
                            hub.publish_resource_change(resource, uuid::Uuid::nil(), "reconciled");
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "Resource reconciliation failed"),
            }
        }
    };
    let expiry = async {
        let mut tick = super::schedule::interval("lease-expiry", Duration::from_secs(30));
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            if let Err(error) = store.expire_leases().await {
                tracing::warn!(%error, "Lease expiry failed");
            }
        }
    };
    let retention = async {
        let mut tick = super::schedule::interval("retention", retention_interval);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            let drain = async {
                // Drain backlogs with short commits, at most 128 passes / ten seconds.
                // A pass bounds row deletes and expires complete statistics partitions.
                // Compaction has its own cadence; do not sweep retention per source-hour.
                for _ in 0..128 {
                    if store.cleanup(build_retention_days).await? == 0 {
                        break;
                    }
                }
                Ok::<_, MaintenanceError>(())
            };
            tokio::select! {
                () = token.cancelled() => break,
                result = tokio::time::timeout(Duration::from_secs(10), drain) => match result {
                    Ok(Ok(())) => {},
                    Ok(Err(error)) => tracing::warn!(%error, "Scheduled retention failed"),
                    Err(_) => tracing::info!("Retention budget exhausted; remaining rows wait for the next maintenance pass"),
                }
            }
        }
    };
    let compaction = async {
        let mut tick = super::schedule::interval("statistics-compaction", COMPACTION_INTERVAL);
        loop {
            tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
            tokio::select! {
                ()=token.cancelled()=>break,
                result=tokio::time::timeout(COMPACTION_BUDGET,compact_batch(store.as_ref())) => match result {
                    Ok(Ok(())) => {},
                    Ok(Err(error)) => tracing::warn!(%error,"Statistics compaction failed"),
                    Err(_) => tracing::debug!("Compaction budget exhausted; remaining history waits for the next pass"),
                }
            }
        }
    };
    tokio::join!(recovery, expiry, retention, compaction);
    Ok(())
}

// At most 15,360 container source-hours/hour before the time bound. A 1,024-source
// installation has capacity to catch up as well as process newly eligible hours.
// Native database cost still determines the actual throughput; polling is unchanged.
const COMPACTION_INTERVAL: Duration = Duration::from_secs(30);
const COMPACTION_BUDGET: Duration = Duration::from_secs(5);
const COMPACTION_PASSES: usize = 128;

async fn compact_batch(store: &dyn MaintenanceStore) -> Result<(), MaintenanceError> {
    for _ in 0..COMPACTION_PASSES {
        if store.compact().await? == 0 {
            break;
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

#[cfg(test)]
#[path = "maintenance_tests.rs"]
mod tests;
