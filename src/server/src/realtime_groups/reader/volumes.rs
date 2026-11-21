use super::*;
use crate::api::resources::platforms::runtime_views::{map_node_volume, map_volume};
use crate::realtime::AuthorizationLease;
use tokio_util::sync::CancellationToken;

pub(super) async fn snapshot(
    state: &ApplicationGroupReader,
    principal: &ActorPrincipal,
    id: Uuid,
    lease: Option<&AuthorizationLease>,
    change: Option<&PublishedRuntimeEvent>,
) -> Result<GroupSnapshot, RealtimeReadError> {
    if let Some(change) = change
        && (change.payload["dockerResourceType"] == "volume"
            || matches!(change.resource_type, "BackupCoverage" | "License"))
        && let Some(partial) = runtime::realtime_resource_snapshot(
            state,
            principal,
            id,
            lease,
            change,
            &Topic::Volumes(id),
        )
        .await?
    {
        return Ok(partial);
    }
    let (platform, volume_cap) = match lease {
        Some(lease) => {
            lease
                .daemon_capabilities(&state.identity, principal, id)
                .await?
        }
        None => runtime::daemon_capabilities(state, principal, id).await?,
    };
    // Node inventory also carries hostnames and stale flags, which can change
    // independently of volume events when the Swarm membership changes.
    let nodes = crate::realtime::shared_reads::node_volumes(&state.platforms, id, change)
        .await?
        .into_iter()
        .map(|v| map_node_volume(v, volume_cap))
        .collect::<Result<Vec<_>, _>>()
        .map_err(failure)?;
    if let Some(change) = change
        && (matches!(
            change.payload["dockerResourceType"].as_str(),
            Some("swarm" | "nodeAgentCoverage")
        ) || change.payload["action"] == "offline"
            || (change.event_kind == "connectionChanged"
                && !state
                    .platforms
                    .get_platform(id)
                    .await
                    .map_err(failure)?
                    .is_some_and(|p| p.status == citadel_primitives::PlatformStatus::Online)))
    {
        let volumes = runtime::enrich_volumes(state, principal, id, nodes).await?;
        return event(
            "VolumesUpdated",
            json!({"platformId":id,"scope":"nodes","volumes":volumes}),
        );
    }
    let read_manager = || async {
        let cancel = CancellationToken::new();
        let _guard = cancel.clone().drop_guard();
        state
            .runtime
            .inventory(id, &cancel)
            .await
            .map_err(failure)?
            .list_volumes(&cancel)
            .await
            .map_err(failure)
    };
    let manager = match change.filter(|e| e.platform_id == Some(id)) {
        Some(change) => change
            .reads
            .volumes
            .get_or_try_init(read_manager)
            .await?
            .clone(),
        None => read_manager().await?,
    };
    let mut volumes = manager
        .into_iter()
        .map(|v| map_volume(v, volume_cap))
        .collect::<Result<Vec<_>, _>>()
        .map_err(failure)?;
    volumes.extend(nodes);
    let volumes = runtime::enrich_volumes(state, principal, id, volumes).await?;
    event(
        "VolumesUpdated",
        json!({"platformId":id,"scope":"all","volumes":volumes,
        "capabilities":{"canRead":platform.can_read,"canWrite":platform.can_write,"canExecute":platform.can_execute}}),
    )
}
