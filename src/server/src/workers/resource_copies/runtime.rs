use citadel_resource_copies::{CopyError, CopyService};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub async fn run(
    cancel: CancellationToken,
    service: Arc<CopyService>,
    wake: watch::Receiver<()>,
) -> Result<(), CopyError> {
    // Independent lanes preserve cancellation/finalization while the single
    // execution lane is occupied. Neither lane cancels an admitted durable copy.
    let execution = async {
        let mut wake = wake.clone();
        while !cancel.is_cancelled() {
            let result = service.process_current_queue(&cancel).await;
            let active = match result {
                Ok(active) => active,
                Err(error) => {
                    tracing::error!(%error, "Resource copy execution failed");
                    true
                }
            };
            wait_for_work(&mut wake, &cancel, active).await;
        }
    };
    let reconciliation = async {
        let mut wake = wake.clone();
        while !cancel.is_cancelled() {
            let result = service.process(&cancel).await;
            let active = match result {
                Ok(active) => active,
                Err(error) => {
                    tracing::error!(%error, "Resource copy reconciliation failed");
                    true
                }
            };
            wait_for_work(&mut wake, &cancel, active).await;
        }
    };
    tokio::join!(execution, reconciliation);
    Ok(())
}

pub(super) async fn wait_for_work(
    wake: &mut watch::Receiver<()>,
    cancel: &CancellationToken,
    active: bool,
) {
    if active {
        // A worker's own writes also notify. Retain the existing active cadence
        // and error backoff so those writes cannot create a tight feedback loop.
        tokio::select! {
            () = cancel.cancelled() => {},
            () = tokio::time::sleep(Duration::from_secs(2)) => {},
        }
    } else {
        super::super::notifications::wait(wake, cancel, Duration::from_secs(30)).await;
    }
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
