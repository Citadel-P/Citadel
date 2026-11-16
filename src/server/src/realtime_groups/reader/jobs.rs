//! A watched process snapshot on the existing authorized group stream.
use super::*;
use std::sync::Arc;
impl ApplicationGroupReader {
    pub(super) fn jobs_service(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<Arc<citadel_instance::InstanceService>, RealtimeReadError> {
        if !principal.is_administrator() {
            return Err(RealtimeReadError::Authorization);
        }
        self.instance
            .clone()
            .ok_or_else(|| failure("Jobs diagnostics unavailable"))
    }
    pub(super) fn jobs_stream(
        &self,
        principal: &ActorPrincipal,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<GroupStream, RealtimeReadError> {
        let service = self.jobs_service(principal)?;
        stream(service, principal, cancel)
    }
}

pub(super) fn stream(
    service: Arc<citadel_instance::InstanceService>,
    principal: &ActorPrincipal,
    cancel: &tokio_util::sync::CancellationToken,
) -> Result<GroupStream, RealtimeReadError> {
    // Subscribe before taking the initial snapshot. Changes during delivery remain pending.
    let mut changes = service.job_changes(principal).map_err(failure)?;
    let principal = principal.clone();
    let cancel = cancel.clone();
    Ok(Box::pin(async_stream::try_stream! {
        loop {
            changes.borrow_and_update();
            let snapshot = service.jobs(&principal).map_err(failure)?;
            let value = serde_json::to_value(crate::api::resources::instance::jobs::JobsSnapshotView::from(snapshot)).map_err(failure)?;
            yield ClientEvent::new("InstanceJobsUpdated", vec![value]);
            // Coalesce bursts, retaining the trailing change. No timer emits unchanged snapshots.
            tokio::select! { () = cancel.cancelled() => break, () = tokio::time::sleep(std::time::Duration::from_secs(1)) => {} }
            tokio::select! { () = cancel.cancelled() => break, result = changes.changed() => { if result.is_err() { break; } } }
        }
    }))
}
