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
    let (completed, mut completions) = watch::channel(());
    let execution = async {
        let mut wake = wake.clone();
        let mut cursor = None;
        let mut progressed = false;
        while !cancel.is_cancelled() {
            if cursor.is_none() {
                wake.borrow_and_update();
            }
            let result = citadel_runtime::jobs::observe_work(
                service.process_current_queue(&cancel, &mut cursor),
                |worked| Some((u64::from(*worked), "productive queue passes")),
            )
            .await;
            let failed = match result {
                Ok(worked) => {
                    if worked {
                        progressed = true;
                        completed.send_modify(|_| {});
                    }
                    false
                }
                Err(error) => {
                    tracing::error!(%error, "Resource copy execution failed");
                    true
                }
            };
            if !failed && (cursor.is_some() || std::mem::take(&mut progressed)) {
                continue;
            }
            wait_for_work(&mut wake, &cancel, failed).await;
        }
    };
    let reconciliation = async {
        let mut wake = wake.clone();
        let mut cursor = None;
        while !cancel.is_cancelled() {
            // Keep signals arriving during any page pending until the sweep
            // completes, including changes to IDs already visited in this sweep.
            if cursor.is_none() {
                wake.borrow_and_update();
                completions.borrow_and_update();
            }
            let result =
                citadel_runtime::jobs::observe(service.process(&cancel, &mut cursor)).await;
            let failed = match result {
                Ok(_) => false,
                Err(error) => {
                    tracing::error!(%error, "Resource copy reconciliation failed");
                    true
                }
            };
            if !failed && cursor.is_some() {
                continue;
            }
            if failed {
                wait_for_work(&mut wake, &cancel, true).await;
            } else {
                tokio::select! {
                    _ = completions.changed() => {},
                    () = wait_for_work(&mut wake, &cancel, false) => {},
                }
            }
        }
    };
    tokio::join!(execution, reconciliation);
    Ok(())
}

pub(super) async fn wait_for_work(
    wake: &mut watch::Receiver<()>,
    cancel: &CancellationToken,
    failed: bool,
) {
    if failed {
        // Failed database operations retain a cancellable backoff even when
        // notifications arrive. Successful passes wait for changes or recovery.
        tokio::select! {
            () = cancel.cancelled() => {},
            () = tokio::time::sleep(Duration::from_secs(5)) => {},
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
    let job = citadel_runtime::jobs::JobHandle::current();
    if let Some(job) = &job {
        job.policy(
            "Resource copy change notifications; reconnect after 2 seconds",
            None,
        );
    }
    loop {
        if let Some(job) = &job {
            job.waiting("Awaiting resource copy changes", None);
        }
        let next = tokio::select! { ()=token.cancelled()=>return Ok(()), next=source.next()=>next };
        let observation = citadel_runtime::jobs::JobHandle::iteration();
        let id = match next {
            Ok(id) => id,
            Err(error) => {
                if let Some(observation) = observation {
                    observation.finish(citadel_runtime::jobs::JobOutcome::Failed, None);
                }
                if let Some(job) = &job {
                    job.waiting(
                        "Reconnecting resource copy notifications",
                        Some(tokio::time::Instant::now() + std::time::Duration::from_secs(2)),
                    );
                }
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
        if let Some(observation) = observation {
            observation.finish(citadel_runtime::jobs::JobOutcome::Completed, None);
        }
    }
}
