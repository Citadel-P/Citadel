use super::*;
use citadel_instance::InstanceService;
use tokio_util::sync::CancellationToken;

pub(crate) fn stream(
    service: &InstanceService,
    principal: &ActorPrincipal,
    cancel: &CancellationToken,
) -> Result<GroupStream, RealtimeReadError> {
    let mut changes = service.schedule_changes(principal).map_err(failure)?;
    let token = cancel.clone();
    Ok(Box::pin(async_stream::try_stream! {
        let mut revision = None;
        loop {
            let snapshot = {
                let state = changes.borrow_and_update();
                let catalog_changed = revision.replace(state.catalog_revision) != Some(state.catalog_revision);
                serde_json::json!({
                    "catalogChanged": catalog_changed,
                    "activeBatches": state.active_batches.iter().map(|p| p.name()).collect::<Vec<_>>(),
                })
            };
            yield ClientEvent::new("InstanceSchedulesChanged", vec![snapshot]);
            tokio::select! { ()=token.cancelled()=>break, ()=tokio::time::sleep(std::time::Duration::from_secs(1))=>{} }
            tokio::select! { ()=token.cancelled()=>break, r=changes.changed()=>if r.is_err(){break} }
        }
    }))
}
