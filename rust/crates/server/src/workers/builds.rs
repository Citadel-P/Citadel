use std::sync::Arc;
use std::time::Duration;

use citadel_builds::{BuildError, BuildService};
use tokio_util::sync::CancellationToken;

pub async fn build_runs(
    cancellation: CancellationToken,
    service: Arc<BuildService>,
) -> Result<(), BuildError> {
    while !cancellation.is_cancelled() {
        match service.process_one(&cancellation).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::error!(%error,"Build worker iteration failed"),
        }
        tokio::select! {()=cancellation.cancelled()=>break,()=tokio::time::sleep(Duration::from_secs(2))=>{}}
    }
    Ok(())
}
