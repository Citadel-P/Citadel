use super::EdgeSession;
use crate::connectors::agent::client as agent;
use crate::connectors::agent::execution::unary;
use citadel_contracts::citadel::{
    containers::v1::*,
    edge::v1::EdgeCommandKind,
    images::v1::*,
    networks::v1::*,
    platforms::v1::{PlatformStatsRequest, PlatformStatsResponse},
    shared_models::v1::{PlatformInfoResponse, VolumeResponse},
    swarm::v1::*,
    volumes::v1::*,
};
use citadel_platforms::*;
use futures_util::future::BoxFuture;
use prost::Message;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct EdgeRuntime {
    pub session: Arc<EdgeSession>,
}

impl EdgeRuntime {
    pub fn stream_container_stats(
        &self,
        interval: Duration,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeContainerStatsStream, RuntimeCapabilityError> {
        let interval_ms = i32::try_from(interval.as_millis())
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::InvalidRequest,
                    "Invalid statistics interval.",
                    false,
                )
            })?;
        if cancellation.is_cancelled() {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Unavailable,
                "Statistics stream canceled.",
                false,
            ));
        }
        let mut pending = self
            .session
            .command(
                EdgeCommandKind::ContainersStatsStream,
                StreamContainersStatsRequest {
                    fetch_interval_ms: interval_ms,
                }
                .encode_to_vec(),
                Duration::from_secs(3600),
                true,
            )
            .map_err(super::EdgeError::runtime)?;
        let cancellation = cancellation.clone();
        Ok(Box::pin(async_stream::stream! {
            loop {
                match pending.next(&cancellation).await {
                    Ok(Some(payload)) => {
                        match ContainersStatsResponse::decode(payload.as_slice()) {
                            Ok(value) => yield Ok(agent::map_container_stats(value)),
                            Err(_) => { yield Err(RuntimeCapabilityError::new(RuntimeErrorKind::Remote, "Invalid Edge statistics response.", false)); break; }
                        }
                    }
                    Ok(None) => break,
                    Err(error) => { yield Err(error.runtime()); break; }
                }
            }
        }))
    }
}
impl citadel_platforms::PlatformInfoPort for EdgeRuntime {
    fn get_info<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response: PlatformInfoResponse = unary(
                &self.session,
                EdgeCommandKind::PlatformGetInfo,
                (),
                cancellation,
            )
            .await?;
            Ok(agent::map_platform_info(response))
        })
    }
}

impl citadel_platforms::ContainerInventoryPort for EdgeRuntime {
    fn list_containers<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeContainerSummary>, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response: ListContainersResponse = unary(
                &self.session,
                EdgeCommandKind::ContainerList,
                ListContainersRequest {
                    all: Some(true),
                    size: Some(false),
                    ..Default::default()
                },
                cancellation,
            )
            .await?;
            Ok(response
                .containers
                .into_values()
                .map(agent::map_container)
                .collect())
        })
    }
}

impl citadel_platforms::PlatformStatsPort for EdgeRuntime {
    fn stream_stats<'a>(
        &'a self,
        interval: Duration,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeStatsStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            let request = PlatformStatsRequest {
                fetch_interval_ms: interval.as_millis().clamp(1, i32::MAX as u128) as i32,
            };
            let mut pending = self
                .session
                .command(
                    EdgeCommandKind::PlatformStatsStream,
                    request.encode_to_vec(),
                    Duration::from_secs(3600),
                    true,
                )
                .map_err(super::EdgeError::runtime)?;
            let cancellation = cancellation.clone();
            Ok(Box::pin(async_stream::stream! {
                loop { match pending.next(&cancellation).await {
                    Ok(Some(payload)) => yield PlatformStatsResponse::decode(payload.as_slice()).map(agent::map_platform_stats).map_err(|_| RuntimeCapabilityError::new(RuntimeErrorKind::Remote,"Invalid Edge statistics response.",false)),
                    Ok(None) => break,
                    Err(error) => { yield Err(error.runtime()); break; }
                } }
            }) as RuntimeStatsStream)
        })
    }
}

// Compile-time dispatch with the same protobuf models and projection mappers
impl SwarmTaskRuntimePort for EdgeRuntime {
    fn inspect_task<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeSwarmTask, RuntimeCapabilityError>> {
        Box::pin(async move {
            let task: SwarmTaskMessage = unary(
                &self.session,
                EdgeCommandKind::SwarmTaskInspect,
                InspectSwarmTaskRequest { task_id: id.into() },
                cancellation,
            )
            .await?;
            Ok(agent::map_swarm_task(task))
        })
    }
}

impl citadel_platforms::NetworkMutationPort for EdgeRuntime {
    fn create_network<'a>(
        &'a self,
        input: &'a CreateRuntimeNetwork,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<CreatedRuntimeNetwork, RuntimeCapabilityError>> {
        Box::pin(async move {
            let result: CreateNetworkResponse = unary(
                &self.session,
                EdgeCommandKind::NetworkCreate,
                agent::network_request(input),
                cancellation,
            )
            .await?;
            Ok(CreatedRuntimeNetwork { id: result.id })
        })
    }

    fn delete_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            unary(
                &self.session,
                EdgeCommandKind::NetworkDelete,
                DeleteNetworkRequest {
                    ids: vec![id.into()],
                },
                cancellation,
            )
            .await
        })
    }
}

impl citadel_platforms::VolumeMutationPort for EdgeRuntime {
    fn create_volume<'a>(
        &'a self,
        input: &'a CreateRuntimeVolume,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        Box::pin(async move {
            let result: VolumeResponse = unary(
                &self.session,
                EdgeCommandKind::VolumeCreate,
                CreateVolumeRequest {
                    name: input.name.clone(),
                    driver: input.driver.clone(),
                    labels: input.labels.clone().into_iter().collect(),
                    options: input.options.clone().into_iter().collect(),
                },
                cancellation,
            )
            .await?;
            Ok(agent::map_volume(result))
        })
    }

    fn delete_volume<'a>(
        &'a self,
        name: &'a str,
        force: bool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            unary(
                &self.session,
                EdgeCommandKind::VolumeDelete,
                RemoveVolumeRequest {
                    names: vec![name.into()],
                    force,
                },
                cancellation,
            )
            .await
        })
    }
}

// as direct Agents. No reflection or second Docker model.
macro_rules! list {
    ($method:ident,$result:ty,$kind:ident,$request:expr,$response:ty,$field:ident,$mapper:ident) => {
        fn $method<'a>(
            &'a self,
            cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<Vec<$result>, RuntimeCapabilityError>> {
            Box::pin(async move {
                let response: $response = unary(
                    &self.session,
                    EdgeCommandKind::$kind,
                    $request,
                    cancellation,
                )
                .await?;
                Ok(response.$field.into_iter().map(agent::$mapper).collect())
            })
        }
    };
}
impl citadel_platforms::ImageInventoryPort for EdgeRuntime {
    list!(
        list_images,
        RuntimeImageSummary,
        ImageList,
        ListImagesRequest {},
        citadel_contracts::citadel::shared_models::v1::ListImageResponse,
        images,
        map_image
    );
}

impl citadel_platforms::NetworkInventoryPort for EdgeRuntime {
    list!(
        list_networks,
        RuntimeNetworkSummary,
        NetworkList,
        ListNetworksRequest::default(),
        ListNetworksResponse,
        networks,
        map_network
    );
}

impl citadel_platforms::VolumeInventoryPort for EdgeRuntime {
    list!(
        list_volumes,
        RuntimeVolumeSummary,
        VolumeList,
        ListVolumesRequest::default(),
        ListVolumesResponse,
        volumes,
        map_volume
    );
}

impl citadel_platforms::SwarmInventoryPort for EdgeRuntime {
    list!(
        list_swarm_nodes,
        RuntimeSwarmNode,
        SwarmNodeList,
        ListSwarmNodesRequest {
            max_items: 10000,
            include_task_counts: true
        },
        ListSwarmNodesResponse,
        nodes,
        map_swarm_node
    );
    list!(
        list_swarm_services,
        RuntimeSwarmService,
        SwarmServiceList,
        ListSwarmServicesRequest { max_items: 10000 },
        ListSwarmServicesResponse,
        services,
        map_swarm_service
    );
    list!(
        list_swarm_tasks,
        RuntimeSwarmTask,
        SwarmTaskList,
        ListSwarmTasksRequest {
            max_items: i32::MAX
        },
        ListSwarmTasksResponse,
        tasks,
        map_swarm_task
    );
    list!(
        list_swarm_configs,
        RuntimeSwarmConfig,
        SwarmConfigList,
        ListSwarmConfigsRequest { max_items: 10000 },
        ListSwarmConfigsResponse,
        configs,
        map_swarm_config
    );
    list!(
        list_swarm_secrets,
        RuntimeSwarmSecret,
        SwarmSecretList,
        ListSwarmSecretsRequest { max_items: 10000 },
        ListSwarmSecretsResponse,
        secrets,
        map_swarm_secret
    );
}

impl citadel_platforms::NetworkObservationPort for EdgeRuntime {
    fn inspect_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response = unary(
                &self.session,
                EdgeCommandKind::NetworkInspect,
                InspectNetworkRequest { id: id.into() },
                cancellation,
            )
            .await?;
            Ok(agent::map_network_inspect(response))
        })
    }
}

impl citadel_platforms::VolumeObservationPort for EdgeRuntime {
    fn inspect_volume<'a>(
        &'a self,
        name: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        Box::pin(async move {
            let response: VolumeResponse = unary(
                &self.session,
                EdgeCommandKind::VolumeInspect,
                InspectVolumeRequest { name: name.into() },
                cancellation,
            )
            .await?;
            Ok(agent::map_volume(response))
        })
    }
}
