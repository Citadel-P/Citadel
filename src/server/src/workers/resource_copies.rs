use citadel_resource_copies::{CopyError, CopyService};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub async fn run(cancel: CancellationToken, service: Arc<CopyService>) -> Result<(), CopyError> {
    // Both futures belong to this supervised worker and share its shutdown
    // token. Long storage work cannot block parent reconciliation. One execution
    // lane bounds current-data helpers to one globally (and per platform/node).
    let execution = async {
        while !cancel.is_cancelled() {
            if let Err(error) = service.process_current_queue(&cancel).await {
                tracing::error!(%error, "Resource copy execution failed");
            }
            tokio::select! { ()=cancel.cancelled()=>break, ()=tokio::time::sleep(std::time::Duration::from_secs(2))=>{} }
        }
    };
    let reconciliation = async {
        while !cancel.is_cancelled() {
            if let Err(error) = service.process(&cancel).await {
                tracing::error!(%error, "Resource copy reconciliation failed");
            }
            tokio::select! { ()=cancel.cancelled()=>break, ()=tokio::time::sleep(std::time::Duration::from_secs(2))=>{} }
        }
    };
    tokio::join!(execution, reconciliation);
    Ok(())
}

/// Authorized group readers reload persisted progress after each committed change.
pub async fn realtime_updates(
    mut source: Box<dyn citadel_resource_copies::CopyChangeSource>,
    realtime: Option<crate::realtime::RealtimeHub>,
    token: CancellationToken,
) -> Result<(), std::convert::Infallible> {
    loop {
        let next = tokio::select! { ()=token.cancelled()=>return Ok(()), next=source.next()=>next };
        let id = match next {
            Ok(id) => id,
            Err(error) => {
                tracing::warn!(%error, "Resource copy listener reconnecting");
                if let Some(hub) = &realtime {
                    hub.publish_resource_change(
                        "ResourceCopy",
                        uuid::Uuid::nil(),
                        "progressChanged",
                    );
                }
                tokio::select! { ()=token.cancelled()=>return Ok(()), _=tokio::time::sleep(std::time::Duration::from_secs(2))=>{} }
                continue;
            }
        };
        if let Some(hub) = &realtime {
            hub.publish_resource_change("ResourceCopy", id.unwrap_or_default(), "progressChanged");
        }
    }
}
