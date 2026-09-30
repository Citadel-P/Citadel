use super::*;

/// Project only the changed resource. Authorization and capabilities remain per actor;
/// event payloads are immutable observations accepted by the projection writer.
pub(crate) async fn realtime_resource_snapshot(
    state: &PlatformsHttpState,
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
    let headers = HeaderMap::new();
    let (platform, volume_cap) = if let Some(lease) = lease {
        lease
            .daemon_capabilities(&state.identity, principal, id)
            .await?
    } else {
        let platform = authorize_platform(state, principal, id, &headers)
            .await
            .map_err(|_| RealtimeReadError::Authorization)?;
        let cap = volume_capabilities(state, principal, id, platform, &headers)
            .await
            .map_err(|_| RealtimeReadError::Authorization)?;
        (platform, cap)
    };
    if kind == "image" {
        let images: Vec<_> =
            crate::realtime::shared_reads::images(&state.platforms, id, Some(event))
                .await?
                .into_iter()
                .map(crate::api::resources::platforms::views::ImageView::from)
                .map(serde_json::to_value)
                .collect::<Result<_, _>>()
                .map_err(failure)?;
        let events = if crate::realtime::shared_reads::platform(&state.platforms, id, Some(event))
            .await?
            .is_some_and(|p| matches!(p.platform_type.as_str(), "DockerSwarm" | "Swarm"))
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
    let (style, networks, volumes) = match delta {
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
            let runtime = runtime_for(state, id)
                .await
                .map_err(|e| RealtimeReadError::Storage(e.to_string()))?;
            let cancel = CancellationToken::new();
            let networks = if kind == "network" {
                event
                    .reads
                    .networks
                    .get_or_try_init(|| async {
                        match &runtime {
                            RuntimeRef::Local(r) => {
                                citadel_platforms::NetworkInventoryPort::list_networks(*r, &cancel)
                                    .await
                            }
                            RuntimeRef::Agent(r) => {
                                citadel_platforms::NetworkInventoryPort::list_networks(r, &cancel)
                                    .await
                            }
                            RuntimeRef::Edge(r) => {
                                citadel_platforms::NetworkInventoryPort::list_networks(r, &cancel)
                                    .await
                            }
                        }
                        .map_err(|e| RealtimeReadError::Storage(e.to_string()))
                    })
                    .await?
                    .clone()
            } else {
                vec![]
            };
            let volumes = if kind == "volume" {
                event
                    .reads
                    .volumes
                    .get_or_try_init(|| async {
                        match &runtime {
                            RuntimeRef::Local(r) => {
                                citadel_platforms::VolumeInventoryPort::list_volumes(*r, &cancel)
                                    .await
                            }
                            RuntimeRef::Agent(r) => {
                                citadel_platforms::VolumeInventoryPort::list_volumes(r, &cancel)
                                    .await
                            }
                            RuntimeRef::Edge(r) => {
                                citadel_platforms::VolumeInventoryPort::list_volumes(r, &cancel)
                                    .await
                            }
                        }
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
