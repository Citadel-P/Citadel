use super::*;
use crate::api::resources::platforms::runtime_views::*;
use citadel_platforms::{AuthorizedReadError, RuntimeCapabilityError};
use tokio_util::sync::CancellationToken;

/// Project only the changed resource. Authorization and capabilities remain per actor;
/// event payloads are immutable observations accepted by the projection writer.
pub(crate) async fn realtime_resource_snapshot(
    state: &ApplicationGroupReader,
    principal: &ActorPrincipal,
    id: Uuid,
    lease: Option<&crate::realtime::AuthorizationLease>,
    event: &crate::realtime::PublishedRuntimeEvent,
) -> Result<Option<crate::realtime_groups::GroupSnapshot>, crate::realtime::RealtimeReadError> {
    use crate::{
        realtime::RealtimeReadError,
        realtime_groups::{ClientEvent, GroupRows, GroupSnapshot, RowStyle},
    };
    use citadel_platforms::jobs::ResourceDelta;
    if event.platform_id != Some(id) {
        return Ok(Some(GroupSnapshot::default()));
    }
    let kind = event.payload["dockerResourceType"]
        .as_str()
        .unwrap_or_default();
    if !matches!(kind, "image" | "network" | "volume") {
        return Ok(None);
    }
    let failure = |e: serde_json::Error| RealtimeReadError::Storage(e.to_string());
    let (platform, volume_cap) = match lease {
        Some(lease) => {
            lease
                .daemon_capabilities(&state.identity, principal, id)
                .await?
        }
        None => daemon_capabilities(state, principal, id).await?,
    };
    if kind == "image" {
        let images: Vec<_> =
            crate::realtime::shared_reads::images(&state.platforms, id, Some(event))
                .await?
                .into_iter()
                .map(|image| {
                    let mut view = crate::api::resources::platforms::views::ImageView::from(image);
                    view.capabilities = Some(image_capabilities(platform));
                    view
                })
                .map(serde_json::to_value)
                .collect::<Result<_, _>>()
                .map_err(failure)?;
        let events = if crate::realtime::shared_reads::platform(&state.platforms, id, Some(event))
            .await?
            .is_some_and(|p| p.platform_type == citadel_platforms::PlatformKind::DockerSwarm)
        {
            vec![ClientEvent::new(
                "SwarmNodeLocalResourcesUpdated",
                vec![serde_json::json!({"platformId":id,"images":images})],
            )]
        } else {
            vec![]
        };
        return Ok(Some(GroupSnapshot {
            events,
            rows: vec![GroupRows {
                target: "ImageEventReceived",
                style: RowStyle::DockerResource,
                rows: images,
            }],
        }));
    }
    if event.payload["dockerNodeId"].as_str().is_some() {
        // Node sets are durable and scoped. Never fetch manager resources to
        // publish a worker's change, and never replace a sibling resource list.
        let mut payload = serde_json::json!({"platformId":id,"nodeOnly":true});
        if kind == "volume" {
            payload["volumes"] = serde_json::to_value(
                crate::realtime::shared_reads::node_volumes(&state.platforms, id, Some(event))
                    .await?
                    .into_iter()
                    .map(|v| map_node_volume(v, volume_cap))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?,
            )
            .map_err(failure)?;
            let manager_count =
                crate::realtime::shared_reads::platform(&state.platforms, id, Some(event))
                    .await?
                    .map_or(0, |p| p.volume_count.max(0) as usize);
            payload["volumeCount"] = serde_json::json!(
                manager_count + payload["volumes"].as_array().map_or(0, Vec::len)
            );
        } else {
            payload["networks"] = serde_json::to_value(
                crate::realtime::shared_reads::node_networks(&state.platforms, id, Some(event))
                    .await?
                    .into_iter()
                    .map(|v| map_node_network(v, network_capabilities(platform)))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failure)?,
            )
            .map_err(failure)?;
        }
        return Ok(Some(GroupSnapshot {
            rows: vec![],
            events: vec![ClientEvent::new(
                "SwarmNodeLocalResourcesUpdated",
                vec![payload],
            )],
        }));
    }
    let delta = event
        .payload
        .get("resourceDelta")
        .map(|v| serde_json::from_value::<ResourceDelta>(v.clone()))
        .transpose()
        .map_err(failure)?;
    let (style, networks, volumes) =
        match delta {
            Some(ResourceDelta::Network { id, value }) => (
                RowStyle::DaemonPatch(id),
                value.into_iter().collect(),
                vec![],
            ),
            Some(ResourceDelta::Volume { id, value }) => (
                RowStyle::DaemonPatch(id),
                vec![],
                value.into_iter().collect(),
            ),
            _ if event.payload.get("resourceSnapshot").is_some() => (
                RowStyle::Daemon,
                serde_json::from_value(
                    event.payload["resourceSnapshot"]
                        .get("networks")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([])),
                )
                .map_err(failure)?,
                serde_json::from_value(
                    event.payload["resourceSnapshot"]
                        .get("volumes")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([])),
                )
                .map_err(failure)?,
            ),
            _ => {
                // Coarse API mutation notifications have no observation. Share only
                // the relevant live read across authorized subscribers of this event.
                let networks =
                    if kind == "network" {
                        event
                            .reads
                            .networks
                            .get_or_try_init(|| async {
                                let cancel = CancellationToken::new();
                                let _guard = cancel.clone().drop_guard();
                                let runtime = state.runtime.inventory(id, &cancel).await.map_err(
                                    |error| RealtimeReadError::Storage(error.to_string()),
                                )?;
                                runtime
                                    .list_networks(&cancel)
                                    .await
                                    .map_err(|e| RealtimeReadError::Storage(e.to_string()))
                            })
                            .await?
                            .clone()
                    } else {
                        vec![]
                    };
                let volumes =
                    if kind == "volume" {
                        event
                            .reads
                            .volumes
                            .get_or_try_init(|| async {
                                let cancel = CancellationToken::new();
                                let _guard = cancel.clone().drop_guard();
                                let runtime = state.runtime.inventory(id, &cancel).await.map_err(
                                    |error| RealtimeReadError::Storage(error.to_string()),
                                )?;
                                runtime
                                    .list_volumes(&cancel)
                                    .await
                                    .map_err(|e| RealtimeReadError::Storage(e.to_string()))
                            })
                            .await?
                            .clone()
                    } else {
                        vec![]
                    };
                (RowStyle::Daemon, networks, volumes)
            }
        };
    let (target, rows) = if kind == "network" {
        (
            "NetworkEventReceived",
            networks
                .into_iter()
                .map(|v| {
                    map_network(v, network_capabilities(platform)).and_then(serde_json::to_value)
                })
                .collect::<Result<_, _>>()
                .map_err(failure)?,
        )
    } else {
        (
            "VolumeEventReceived",
            volumes
                .into_iter()
                .map(|v| map_volume(v, volume_cap).and_then(serde_json::to_value))
                .collect::<Result<_, _>>()
                .map_err(failure)?,
        )
    };
    Ok(Some(GroupSnapshot {
        events: vec![],
        rows: vec![GroupRows {
            target,
            style,
            rows,
        }],
    }))
}

pub(crate) async fn realtime_daemon_snapshot(
    state: &ApplicationGroupReader,
    principal: &ActorPrincipal,
    id: Uuid,
    lease: Option<&crate::realtime::AuthorizationLease>,
) -> Result<crate::realtime_groups::GroupSnapshot, crate::realtime::RealtimeReadError> {
    use crate::{
        realtime::RealtimeReadError,
        realtime_groups::{GroupRows, GroupSnapshot, RowStyle},
    };
    let failure = |error: RuntimeCapabilityError| RealtimeReadError::Storage(error.to_string());
    let (platform, volume_cap) = match lease {
        Some(lease) => {
            lease
                .daemon_capabilities(&state.identity, principal, id)
                .await?
        }
        None => daemon_capabilities(state, principal, id).await?,
    };
    let cancellation = CancellationToken::new();
    let runtime = state
        .runtime
        .inventory(id, &cancellation)
        .await
        .map_err(failure)?;
    let networks = runtime
        .list_networks(&cancellation)
        .await
        .map_err(failure)?;
    let volumes = runtime.list_volumes(&cancellation).await.map_err(failure)?;
    let serialize = |error: serde_json::Error| RealtimeReadError::Storage(error.to_string());
    let mut events = vec![];
    if state
        .platforms
        .get_platform(id)
        .await
        .map_err(|error| RealtimeReadError::Storage(error.to_string()))?
        .is_some_and(|platform| {
            platform.platform_type == citadel_platforms::PlatformKind::DockerSwarm
        })
    {
        let failure = |error: AuthorizedReadError| RealtimeReadError::Storage(error.to_string());
        let mut images = state
            .platforms
            .list_images(id)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::ImageView::from)
                    .collect::<Vec<_>>()
            })
            .map_err(failure)?;
        for image in &mut images {
            image.capabilities = Some(image_capabilities(platform));
        }
        let mut all_volumes: Vec<_> = volumes
            .iter()
            .cloned()
            .map(|volume| map_volume(volume, volume_cap))
            .collect::<Result<_, _>>()
            .map_err(serialize)?;
        all_volumes.extend(
            state
                .platforms
                .list_node_volumes(id)
                .await
                .map_err(failure)?
                .into_iter()
                .map(|volume| map_node_volume(volume, volume_cap))
                .collect::<Result<Vec<_>, _>>()
                .map_err(serialize)?,
        );
        let node_networks: Vec<_> = state
            .platforms
            .list_node_networks(id)
            .await
            .map_err(failure)?
            .into_iter()
            .map(|network| map_node_network(network, network_capabilities(platform)))
            .collect::<Result<_, _>>()
            .map_err(serialize)?;
        events.push(crate::realtime_groups::ClientEvent::new("SwarmNodeLocalResourcesUpdated", vec![serde_json::json!({
            "platformId": id, "images": images, "volumes": all_volumes, "networks": node_networks,
        })]));
    }
    Ok(GroupSnapshot {
        events,
        rows: vec![
            GroupRows {
                target: "NetworkEventReceived",
                style: RowStyle::Daemon,
                rows: networks
                    .into_iter()
                    .map(|n| {
                        map_network(n, network_capabilities(platform))
                            .and_then(serde_json::to_value)
                    })
                    .collect::<Result<_, _>>()
                    .map_err(serialize)?,
            },
            GroupRows {
                target: "VolumeEventReceived",
                style: RowStyle::Daemon,
                rows: volumes
                    .into_iter()
                    .map(|v| map_volume(v, volume_cap).and_then(serde_json::to_value))
                    .collect::<Result<_, _>>()
                    .map_err(serialize)?,
            },
        ],
    })
}
async fn daemon_capabilities(
    state: &ApplicationGroupReader,
    principal: &ActorPrincipal,
    id: Uuid,
) -> Result<
    (
        crate::api::resources::platforms::views::PlatformCapabilitiesView,
        crate::api::resources::platforms::views::VolumeCapabilitiesView,
    ),
    RealtimeReadError,
> {
    use crate::api::resources::platforms::{
        capabilities::{permission_for, platform_capabilities},
        views::VolumeCapabilitiesView,
    };
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        state
            .platforms
            .permissions_for_platforms(principal.actor_id, &[id])
            .await
            .map_err(failure)?
    };
    let platform = platform_capabilities(permission_for(
        &permissions,
        id,
        principal.is_administrator(),
    ));
    if !platform.can_read {
        return Err(RealtimeReadError::Authorization);
    }
    let content = if principal.is_administrator() {
        None
    } else {
        state
            .identity
            .permission_for_resource(principal, ResourceType::Volume, id)
            .await
            .map_err(failure)?
    };
    let allowed = |specific| {
        principal.is_administrator()
            || content.as_ref().is_some_and(|permission| {
                permission.level.grants(PermissionLevel::Read) && permission.has_specific(specific)
            })
    };
    Ok((
        platform,
        VolumeCapabilitiesView {
            can_read: platform.can_read,
            can_write: platform.can_write,
            can_execute: platform.can_execute,
            can_inspect: platform.can_inspect,
            can_browse: allowed(SpecificPermission::Browse),
            can_download: allowed(SpecificPermission::Download),
        },
    ))
}
