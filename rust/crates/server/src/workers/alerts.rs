use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertDeliveryService, AlertError};
use tokio_util::sync::CancellationToken;

pub async fn deliveries(
    cancellation: CancellationToken,
    service: Arc<AlertDeliveryService>,
) -> Result<(), AlertError> {
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        match service.process_one(&cancellation).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "Alert delivery worker iteration failed"),
        }
        tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            () = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
}

/// Producers notify inside their state transaction; PostgreSQL delivers only after commit.
/// Carry the failure snapshot, since another operation may already have replaced the row.
pub(super) async fn observations(
    cancel: CancellationToken,
    alerts: Arc<dyn citadel_alerts::AlertEventSink>,
    mut listener: sqlx::postgres::PgListener,
) -> Result<(), std::convert::Infallible> {
    loop {
        let notification = tokio::select! { biased; ()=cancel.cancelled()=>return Ok(()), result=listener.recv()=>result };
        match notification {
            Ok(notification) => match serde_json::from_str::<citadel_alerts::AlertObservation>(
                notification.payload(),
            ) {
                Ok(observation) => {
                    if let Err(error) = alerts.observe(&observation).await {
                        tracing::warn!(%error,"Committed job alert evaluation failed");
                    }
                }
                Err(error) => tracing::warn!(%error,"Invalid job alert observation"),
            },
            Err(error) => {
                tracing::warn!(%error,"Job alert listener failed");
                tokio::select! { ()=cancel.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
            }
        }
    }
}
