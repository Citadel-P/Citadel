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
