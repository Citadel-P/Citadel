use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Extension, Path, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_adapters::agent::AgentClient;
use citadel_adapters::docker::DockerClient;
use citadel_adapters::edge::{EdgeRegistry, EdgeRuntime, EdgeTarget};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_platforms::{
    AuthorizedReadError, ContainerView, CreatePlatformInput, CreateRuntimeNetwork,
    CreateRuntimeVolume, EffectivePlatformPermission, ImageCapabilitiesView, ImageView,
    NetworkCapabilitiesView, NetworkView, PlatformCapabilitiesView, PlatformInventoryPort,
    PlatformReadService, PlatformRegistrationError, PlatformRegistrationService,
    PlatformResourceMutationPort, PlatformView, ResourceCapabilitiesView, RuntimeCapabilityError,
    RuntimeErrorKind, RuntimeNetworkSummary, RuntimeVolumeSummary, SwarmConfigView,
    SwarmNetworkView, SwarmNodeView, SwarmSecretView, SwarmServiceView, VolumeCapabilitiesView,
    VolumeView,
};
use citadel_resources::{MetadataPatch, ResourceMetadataStore};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::realtime::RealtimeHub;

const ALL_LEVELS: i32 =
    PermissionLevel::Read as i32 | PermissionLevel::Write as i32 | PermissionLevel::Execute as i32;
const ALL_PLATFORM_SPECIFIC: i32 = SpecificPermission::Logs as i32
    | SpecificPermission::Inspect as i32
    | SpecificPermission::Pull as i32
    | SpecificPermission::Terminal as i32
    | SpecificPermission::ManageNodeAgents as i32;
const MAX_DOCKER_RESOURCE_ID_BYTES: usize = 256;

mod container_mutations;
mod edge;
mod images;
mod logs;
mod statistics;
mod volume_content;
pub use edge::EdgeHttpContext;

#[derive(Clone)]
pub struct PlatformsHttpState {
    pub volume_content: Arc<citadel_adapters::volume_content::VolumeContentAdapter>,
    pub containers: Arc<citadel_platforms::containers::ContainerMutationService>,
    pub identity: Arc<IdentityService>,
    pub platforms: Arc<PlatformReadService>,
    pub registrations: Arc<PlatformRegistrationService>,
    pub pool: PgPool,
    pub resource_metadata: Arc<dyn ResourceMetadataStore>,
    pub docker: DockerClient,
    pub agent: Option<AgentClient>,
    pub edge: EdgeRegistry,
    pub realtime: Option<RealtimeHub>,
    pub stats_sample_max_age: std::time::Duration,
}

pub fn router(state: PlatformsHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_VOLUME_DIRECTORY, volume_content::list)
        .contract_route(routes::DOWNLOAD_VOLUME_PATH, volume_content::download)
        .contract_route(routes::LIST_PLATFORMS, list_platforms)
        .contract_route(routes::CREATE_PLATFORM, create_platform)
        .contract_route(routes::GET_PLATFORM, get_platform)
        .contract_route(routes::CREATE_EDGE_ENROLLMENT, edge::enroll)
        .contract_route(routes::GET_EDGE_STATUS, edge::status)
        .contract_route(routes::GET_NODE_AGENT_COVERAGE, edge::node_coverage)
        .contract_route(routes::REMOVE_NODE_AGENTS, edge::remove_node_agents)
        .contract_route(routes::INSTALL_NODE_AGENTS, edge::install_node_agents)
        .contract_route(routes::REPAIR_NODE_AGENTS, edge::repair_node_agents)
        .contract_route(routes::UPGRADE_NODE_AGENTS, edge::upgrade_node_agents)
        .contract_route(routes::REVOKE_EDGE, edge::revoke)
        .contract_route(routes::UPDATE_PLATFORM_METADATA, update_platform_metadata)
        .contract_route(routes::LIST_PLATFORM_CONTAINERS, list_containers)
        .contract_route(routes::GET_CONTAINER, get_container)
        .contract_route(routes::START_CONTAINERS, container_mutations::start)
        .contract_route(routes::STOP_CONTAINERS, container_mutations::stop)
        .contract_route(routes::RESTART_CONTAINERS, container_mutations::restart)
        .contract_route(routes::PAUSE_CONTAINERS, container_mutations::pause)
        .contract_route(routes::UNPAUSE_CONTAINERS, container_mutations::unpause)
        .contract_route(routes::DELETE_CONTAINERS, container_mutations::delete)
        .contract_route(routes::GET_CONTAINER_STATS, statistics::container)
        .contract_route(routes::GET_PLATFORM_STATS, statistics::platform)
        .contract_route(routes::GET_DEPLOYMENT_STATS, statistics::deployment)
        .contract_route(routes::GET_STACK_STATS, statistics::stack)
        .contract_route(routes::GET_SWARM_SERVICE_STATS, statistics::service)
        .contract_route(routes::GET_SWARM_TASK_STATS, statistics::task)
        .contract_route(routes::GET_SWARM_SERVICE_LOGS, logs::service)
        .contract_route(routes::GET_SWARM_TASK_LOGS, logs::task)
        .contract_route(
            routes::GET_MANAGED_SWARM_SERVICE_LOGS,
            logs::managed_service,
        )
        .contract_route(routes::LIST_PLATFORM_IMAGES, list_images)
        .contract_route(routes::GET_PLATFORM_IMAGE, images::inspect)
        .contract_route(routes::GET_IMAGE_EXPOSED_PORTS, images::exposed_ports)
        .contract_route(routes::LIST_PLATFORM_NETWORKS, list_networks)
        .contract_route(routes::GET_PLATFORM_NETWORK, get_network)
        .contract_route(routes::CREATE_NETWORK, create_network)
        .contract_route(routes::DELETE_NETWORKS, delete_networks)
        .contract_route(routes::LIST_PLATFORM_VOLUMES, list_volumes)
        .contract_route(routes::GET_PLATFORM_VOLUME, get_volume)
        .contract_route(routes::CREATE_VOLUME, create_volume)
        .contract_route(routes::DELETE_VOLUMES, delete_volumes)
        .contract_route(routes::LIST_SWARM_NODES, list_swarm_nodes)
        .contract_route(routes::GET_SWARM_NODE, get_swarm_node)
        .contract_route(routes::LIST_SWARM_SERVICES, list_swarm_services)
        .contract_route(routes::GET_SWARM_SERVICE, get_swarm_service)
        .contract_route(routes::LIST_SWARM_TASKS, list_swarm_tasks)
        .contract_route(routes::GET_SWARM_TASK, get_swarm_task)
        .contract_route(routes::LIST_SWARM_NETWORKS, list_swarm_networks)
        .contract_route(routes::GET_SWARM_NETWORK, get_swarm_network)
        .contract_route(routes::LIST_SWARM_CONFIGS, list_swarm_configs)
        .contract_route(routes::GET_SWARM_CONFIG, get_swarm_config)
        .contract_route(routes::LIST_SWARM_SECRETS, list_swarm_secrets)
        .contract_route(routes::GET_SWARM_SECRET, get_swarm_secret)
        .with_state(state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformsResponse {
    platforms: Vec<PlatformView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchPlatformMetadataInput {
    #[serde(default)]
    description: MetadataPatch<String>,
    #[serde(default, rename = "tags")]
    _tags: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContainersResponse {
    containers: Vec<ContainerView>,
    capabilities: PlatformCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImagesResponse {
    images: Vec<ImageView>,
    capabilities: ImageCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworksResponse {
    networks: Vec<NetworkView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VolumesResponse {
    volumes: Vec<VolumeView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Debug, Default, Deserialize)]
struct NetworkFilters {
    #[serde(rename = "Dangling", alias = "dangling")]
    dangling: Option<bool>,
    #[serde(rename = "Driver", alias = "driver")]
    driver: Option<String>,
    #[serde(rename = "Id", alias = "id")]
    id: Option<String>,
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct VolumeFilters {
    #[serde(rename = "Dangling", alias = "dangling")]
    dangling: Option<bool>,
    #[serde(rename = "Driver", alias = "driver")]
    driver: Option<String>,
    #[serde(rename = "Name", alias = "name")]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNetworkInput {
    platform_id: Uuid,
    #[serde(flatten)]
    network: CreateRuntimeNetwork,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteNetworksInput {
    platform_id: Uuid,
    ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateVolumeInput {
    platform_id: Uuid,
    #[serde(flatten)]
    volume: CreateRuntimeVolume,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteVolumesInput {
    platform_id: Uuid,
    names: Vec<String>,
    force: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwarmTaskFilters {
    #[serde(default = "default_swarm_task_limit")]
    limit: usize,
    service_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DockerNodeSelector {
    docker_node_id: Option<String>,
}

const fn default_swarm_task_limit() -> usize {
    50
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SwarmItemsResponse<T> {
    items: Vec<T>,
    capabilities: PlatformCapabilitiesView,
}

async fn list_platforms(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let tags = identity_result(parse_tag_filters(raw_query.as_deref()), &headers)?;
    let mut platforms = identity_result(
        state
            .platforms
            .list_authorized(principal.actor_id, principal.is_administrator(), &tags)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let ids = platforms
        .iter()
        .map(|platform| platform.id)
        .collect::<Vec<_>>();
    let permissions = effective_permissions(&state, &principal, &ids, &headers).await?;
    for platform in &mut platforms {
        platform.capabilities = Some(platform_capabilities(permission_for(
            &permissions,
            platform.id,
            principal.is_administrator(),
        )));
    }
    let global = if principal.is_administrator() {
        EffectivePlatformPermission {
            level_mask: ALL_LEVELS,
            specific_mask: ALL_PLATFORM_SPECIFIC,
        }
    } else {
        state
            .identity
            .global_permission(&principal, ResourceType::Platform)
            .await
            .map_err(|error| crate::identity_http::IdentityHttpError::from_parts(error, &headers))?
            .map_or_else(EffectivePlatformPermission::default, |permission| {
                EffectivePlatformPermission {
                    level_mask: permission.level as i32,
                    specific_mask: permission.specific_mask,
                }
            })
    };
    Ok(no_store(
        Json(PlatformsResponse {
            platforms,
            capabilities: resource_capabilities(global),
        })
        .into_response(),
    ))
}

async fn create_platform(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreatePlatformInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize_platform_creation(&state, &principal, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let id = match state
        .registrations
        .create(principal.actor_id, input, &CancellationToken::new())
        .await
    {
        Ok(id) => id,
        Err(PlatformRegistrationError::Runtime(error)) => {
            return Ok(runtime_error_response(error, &headers));
        }
        Err(error) => {
            return identity_result(Err(platform_registration_error(error)), &headers);
        }
    };
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "create", &id.to_string());
    Ok(no_store(Json(platform).into_response()))
}

async fn get_platform(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    Ok(no_store(Json(platform).into_response()))
}

async fn update_platform_metadata(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchPlatformMetadataInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;

    let description = match input.description {
        MetadataPatch::Missing => None,
        MetadataPatch::Null => Some(None),
        MetadataPatch::Value(value) => {
            if value.chars().count() > 600 {
                return identity_result(
                    Err(IdentityError::Validation(
                        "Description cannot exceed 600 characters.".to_owned(),
                    )),
                    &headers,
                );
            }
            Some(Some(value))
        }
    };
    if let Some(description) = description {
        identity_result(
            state
                .resource_metadata
                .update_platform_description(id, description.as_deref())
                .await
                .map_err(resource_metadata_error),
            &headers,
        )?;
    }
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(Json(platform).into_response()))
}

async fn list_containers(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(platform_id) = identity_result(path.map_err(invalid_path), &headers)?;
    let capabilities = authorize_platform(&state, &principal, platform_id, &headers).await?;
    let mut containers = identity_result(
        state
            .platforms
            .list_containers(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    for container in &mut containers {
        container.capabilities = Some(capabilities);
    }
    Ok(no_store(
        Json(ContainersResponse {
            containers,
            capabilities,
        })
        .into_response(),
    ))
}

async fn get_container(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let mut container = required(
        state
            .platforms
            .get_container(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    container.capabilities =
        Some(authorize_platform(&state, &principal, container.platform_id, &headers).await?);
    Ok(no_store(Json(container).into_response()))
}

async fn list_images(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(platform_id) = identity_result(path.map_err(invalid_path), &headers)?;
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = image_capabilities(platform_capabilities);
    let mut images = identity_result(
        state
            .platforms
            .list_images(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    for image in &mut images {
        image.capabilities = Some(capabilities);
    }
    Ok(no_store(
        Json(ImagesResponse {
            images,
            capabilities,
        })
        .into_response(),
    ))
}

async fn list_networks(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    filters: Result<Query<NetworkFilters>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(platform_id) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(filters) = identity_result(filters.map_err(invalid_query), &headers)?;
    validate_filter_lengths(
        [
            filters.driver.as_deref(),
            filters.id.as_deref(),
            filters.name.as_deref(),
        ],
        &headers,
    )?;
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = network_capabilities(platform_capabilities);
    let cancellation = CancellationToken::new();
    let runtime = match runtime_for(&state, platform_id).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let values = match runtime {
        RuntimeRef::Local(runtime) => {
            PlatformInventoryPort::list_networks(runtime, &cancellation).await
        }
        RuntimeRef::Agent(runtime) => {
            PlatformInventoryPort::list_networks(runtime, &cancellation).await
        }
        RuntimeRef::Edge(ref runtime) => {
            PlatformInventoryPort::list_networks(runtime, &cancellation).await
        }
    };
    let values = match values {
        Ok(values) => values,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let mut networks: Vec<_> = values
        .into_iter()
        .filter(|network| network_matches(network, &filters))
        .map(|network| map_network(network, capabilities))
        .collect();
    let node_networks = identity_result(
        state
            .platforms
            .list_node_networks(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    networks.extend(
        node_networks
            .into_iter()
            .filter(|network| network_matches(&network.resource, &filters))
            .map(|network| map_node_network(network, capabilities)),
    );
    Ok(no_store(
        Json(NetworksResponse {
            networks,
            capabilities: ResourceCapabilitiesView {
                can_read: platform_capabilities.can_read,
                can_write: platform_capabilities.can_write,
                can_execute: platform_capabilities.can_execute,
            },
        })
        .into_response(),
    ))
}

/// Called only after LookupStore has checked source/context Platform access.
pub(crate) async fn lookup_platform_resources(
    state: &PlatformsHttpState,
    platform_id: Uuid,
    kind: citadel_domain::LookupResourceType,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let runtime = match runtime_for(state, platform_id).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, headers)),
    };
    let cancellation = CancellationToken::new();
    let names = if kind == citadel_domain::LookupResourceType::Volume {
        let result = match runtime {
            RuntimeRef::Local(runtime) => {
                PlatformInventoryPort::list_volumes(runtime, &cancellation).await
            }
            RuntimeRef::Agent(runtime) => {
                PlatformInventoryPort::list_volumes(runtime, &cancellation).await
            }
            RuntimeRef::Edge(ref runtime) => {
                PlatformInventoryPort::list_volumes(runtime, &cancellation).await
            }
        };
        result.map(|values| {
            values
                .into_iter()
                .map(|value| value.name)
                .collect::<Vec<_>>()
        })
    } else {
        let result = match runtime {
            RuntimeRef::Local(runtime) => {
                PlatformInventoryPort::list_networks(runtime, &cancellation).await
            }
            RuntimeRef::Agent(runtime) => {
                PlatformInventoryPort::list_networks(runtime, &cancellation).await
            }
            RuntimeRef::Edge(ref runtime) => {
                PlatformInventoryPort::list_networks(runtime, &cancellation).await
            }
        };
        result.map(|values| {
            values
                .into_iter()
                .map(|value| value.name)
                .collect::<Vec<_>>()
        })
    };
    let mut names = match names {
        Ok(names) => names,
        Err(error) => return Ok(runtime_error_response(error, headers)),
    };
    names.sort();
    let rows: Vec<_> = names
        .into_iter()
        .map(|name| citadel_resources::LookupResourceInfo {
            id: Uuid::nil(),
            name,
            group: None,
        })
        .collect();
    Ok(no_store(Json(rows).into_response()))
}

async fn get_network(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, network_id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(selector) = identity_result(selector.map_err(invalid_query), &headers)?;
    identity_result(validate_docker_resource_id(&network_id), &headers)?;
    if let Some(node_id) = selector.docker_node_id.as_deref() {
        identity_result(validate_docker_resource_id(node_id), &headers)?;
    }
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = network_capabilities(platform_capabilities);
    let cancellation = CancellationToken::new();
    let runtime =
        match runtime_for_node(&state, platform_id, selector.docker_node_id.as_deref()).await {
            Ok(runtime) => runtime,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
    let value = match runtime {
        RuntimeRef::Local(runtime) => {
            PlatformInventoryPort::inspect_network(runtime, &network_id, &cancellation).await
        }
        RuntimeRef::Agent(runtime) => {
            PlatformInventoryPort::inspect_network(runtime, &network_id, &cancellation).await
        }
        RuntimeRef::Edge(ref runtime) => {
            PlatformInventoryPort::inspect_network(runtime, &network_id, &cancellation).await
        }
    };
    let mut network = match value {
        Ok(network) => map_network(network, capabilities),
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    network.docker_node_id = selector.docker_node_id;
    Ok(no_store(Json(network).into_response()))
}

async fn create_network(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateNetworkInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(validate_network_input(&input.network), &headers)?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    if input.network.scope.eq_ignore_ascii_case("swarm")
        && !identity_result(platform_is_swarm(&state, input.platform_id).await, &headers)?
    {
        return identity_result(
            Err(IdentityError::Validation(
                "Swarm-scoped overlay networks require a Docker Swarm platform.".to_owned(),
            )),
            &headers,
        );
    }
    let cancellation = CancellationToken::new();
    let result = match runtime_for(&state, input.platform_id).await {
        Ok(RuntimeRef::Local(runtime)) => {
            PlatformResourceMutationPort::create_network(runtime, &input.network, &cancellation)
                .await
        }
        Ok(RuntimeRef::Agent(runtime)) => {
            PlatformResourceMutationPort::create_network(runtime, &input.network, &cancellation)
                .await
        }
        Ok(RuntimeRef::Edge(ref runtime)) => {
            PlatformResourceMutationPort::create_network(runtime, &input.network, &cancellation)
                .await
        }
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let created = match result {
        Ok(created) => created,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    publish_runtime_change(&state, input.platform_id, "network", "create", &created.id);
    Ok(no_store(Json(created).into_response()))
}

async fn delete_networks(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteNetworksInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(
        validate_resource_ids(&mut input.ids, 100, "Network"),
        &headers,
    )?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    let is_swarm = identity_result(platform_is_swarm(&state, input.platform_id).await, &headers)?;
    let cancellation = CancellationToken::new();
    let runtime = match runtime_for(&state, input.platform_id).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };

    // Inspect every target before the first irreversible operation. This avoids
    // predictable partial batches while retaining an explicit partial result if
    // Docker changes between preflight and deletion.
    for id in &input.ids {
        let inspected = match runtime {
            RuntimeRef::Local(runtime) => {
                PlatformInventoryPort::inspect_network(runtime, id, &cancellation).await
            }
            RuntimeRef::Agent(runtime) => {
                PlatformInventoryPort::inspect_network(runtime, id, &cancellation).await
            }
            RuntimeRef::Edge(ref runtime) => {
                PlatformInventoryPort::inspect_network(runtime, id, &cancellation).await
            }
        };
        let network = match inspected {
            Ok(network) => network,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
        if network
            .labels
            .get("com.citadel.system")
            .is_some_and(|value| value == "true")
        {
            return Ok(conflict_response(
                format!("System network '{}' cannot be deleted.", network.name),
                &headers,
            ));
        }
        if network.container_count != 0 {
            return Ok(conflict_response(
                format!(
                    "Network '{}' is in use and cannot be deleted.",
                    network.name
                ),
                &headers,
            ));
        }
        if network.labels.contains_key("com.docker.stack.namespace")
            || network.labels.contains_key("com.citadel.stack-id")
        {
            return Ok(conflict_response(
                format!(
                    "Stack-owned network '{}' cannot be deleted independently.",
                    network.name
                ),
                &headers,
            ));
        }
        if is_swarm && !network.scope.eq_ignore_ascii_case("swarm") {
            return Ok(conflict_response(
                "Node-local Network deletion requires an explicit Node target and is not available."
                    .to_owned(),
                &headers,
            ));
        }
        if is_swarm && network.scope.eq_ignore_ascii_case("swarm") {
            identity_result(
                validate_swarm_network_projection(&state, input.platform_id, id, &network.name)
                    .await,
                &headers,
            )?;
        }
    }

    let mut deleted = 0_usize;
    for id in &input.ids {
        let result = match runtime {
            RuntimeRef::Local(runtime) => {
                PlatformResourceMutationPort::delete_network(runtime, id, &cancellation).await
            }
            RuntimeRef::Agent(runtime) => {
                PlatformResourceMutationPort::delete_network(runtime, id, &cancellation).await
            }
            RuntimeRef::Edge(ref runtime) => {
                PlatformResourceMutationPort::delete_network(runtime, id, &cancellation).await
            }
        };
        match result {
            Ok(()) => deleted += 1,
            Err(error) if error.kind == RuntimeErrorKind::NotFound => deleted += 1,
            Err(error) if deleted == 0 => return Ok(runtime_error_response(error, &headers)),
            Err(error) => {
                return Ok(conflict_response(
                    format!(
                        "Deleted {deleted} of {} Networks before Docker rejected the operation: {}",
                        input.ids.len(),
                        error.message
                    ),
                    &headers,
                ));
            }
        }
        publish_runtime_change(&state, input.platform_id, "network", "remove", id);
    }
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn list_volumes(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    filters: Result<Query<VolumeFilters>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(platform_id) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(filters) = identity_result(filters.map_err(invalid_query), &headers)?;
    validate_filter_lengths(
        [filters.driver.as_deref(), filters.name.as_deref()],
        &headers,
    )?;
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = volume_capabilities(
        &state,
        &principal,
        platform_id,
        platform_capabilities,
        &headers,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let runtime = match runtime_for(&state, platform_id).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let values = match runtime {
        RuntimeRef::Local(runtime) => {
            PlatformInventoryPort::list_volumes(runtime, &cancellation).await
        }
        RuntimeRef::Agent(runtime) => {
            PlatformInventoryPort::list_volumes(runtime, &cancellation).await
        }
        RuntimeRef::Edge(ref runtime) => {
            PlatformInventoryPort::list_volumes(runtime, &cancellation).await
        }
    };
    let values = match values {
        Ok(values) => values,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let mut volumes: Vec<_> = values
        .into_iter()
        .filter(|volume| volume_matches(volume, &filters))
        .map(|volume| map_volume(volume, capabilities))
        .collect();
    let node_volumes = identity_result(
        state
            .platforms
            .list_node_volumes(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    volumes.extend(
        node_volumes
            .into_iter()
            .filter(|volume| volume_matches(&volume.resource, &filters))
            .map(|volume| map_node_volume(volume, capabilities)),
    );
    Ok(no_store(
        Json(VolumesResponse {
            volumes,
            capabilities: ResourceCapabilitiesView {
                can_read: platform_capabilities.can_read,
                can_write: platform_capabilities.can_write,
                can_execute: platform_capabilities.can_execute,
            },
        })
        .into_response(),
    ))
}

async fn get_volume(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, name)) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(selector) = identity_result(selector.map_err(invalid_query), &headers)?;
    identity_result(validate_docker_resource_id(&name), &headers)?;
    if let Some(node_id) = selector.docker_node_id.as_deref() {
        identity_result(validate_docker_resource_id(node_id), &headers)?;
    }
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = volume_capabilities(
        &state,
        &principal,
        platform_id,
        platform_capabilities,
        &headers,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let runtime =
        match runtime_for_node(&state, platform_id, selector.docker_node_id.as_deref()).await {
            Ok(runtime) => runtime,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
    let value = match runtime {
        RuntimeRef::Local(runtime) => {
            PlatformInventoryPort::inspect_volume(runtime, &name, &cancellation).await
        }
        RuntimeRef::Agent(runtime) => {
            PlatformInventoryPort::inspect_volume(runtime, &name, &cancellation).await
        }
        RuntimeRef::Edge(ref runtime) => {
            PlatformInventoryPort::inspect_volume(runtime, &name, &cancellation).await
        }
    };
    let mut volume = match value {
        Ok(volume) => map_volume(volume, capabilities),
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    volume.docker_node_id = selector.docker_node_id;
    Ok(no_store(Json(volume).into_response()))
}

async fn create_volume(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateVolumeInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(validate_volume_input(&input.volume), &headers)?;
    let platform_capabilities = authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let capabilities = volume_capabilities(
        &state,
        &principal,
        input.platform_id,
        platform_capabilities,
        &headers,
    )
    .await?;
    let cancellation = CancellationToken::new();
    let result = match runtime_for(&state, input.platform_id).await {
        Ok(RuntimeRef::Local(runtime)) => {
            PlatformResourceMutationPort::create_volume(runtime, &input.volume, &cancellation).await
        }
        Ok(RuntimeRef::Agent(runtime)) => {
            PlatformResourceMutationPort::create_volume(runtime, &input.volume, &cancellation).await
        }
        Ok(RuntimeRef::Edge(ref runtime)) => {
            PlatformResourceMutationPort::create_volume(runtime, &input.volume, &cancellation).await
        }
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let volume = match result {
        Ok(volume) => volume,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    publish_runtime_change(&state, input.platform_id, "volume", "create", &volume.name);
    Ok(no_store(
        Json(map_volume(volume, capabilities)).into_response(),
    ))
}

async fn delete_volumes(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteVolumesInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    identity_result(
        validate_resource_ids(&mut input.names, 100, "Volume"),
        &headers,
    )?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    if identity_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
        return Ok(conflict_response(
            "Node-local Volume deletion requires an explicit Node target and is not available."
                .to_owned(),
            &headers,
        ));
    }
    let cancellation = CancellationToken::new();
    let runtime = match runtime_for(&state, input.platform_id).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let mut deleted = 0_usize;
    for name in &input.names {
        let result = match runtime {
            RuntimeRef::Local(runtime) => {
                PlatformResourceMutationPort::delete_volume(
                    runtime,
                    name,
                    input.force.unwrap_or(false),
                    &cancellation,
                )
                .await
            }
            RuntimeRef::Agent(runtime) => {
                PlatformResourceMutationPort::delete_volume(
                    runtime,
                    name,
                    input.force.unwrap_or(false),
                    &cancellation,
                )
                .await
            }
            RuntimeRef::Edge(ref runtime) => {
                PlatformResourceMutationPort::delete_volume(
                    runtime,
                    name,
                    input.force.unwrap_or(false),
                    &cancellation,
                )
                .await
            }
        };
        match result {
            Ok(()) => deleted += 1,
            Err(error) if error.kind == RuntimeErrorKind::NotFound => deleted += 1,
            Err(error) if deleted == 0 => return Ok(runtime_error_response(error, &headers)),
            Err(error) => {
                return Ok(conflict_response(
                    format!(
                        "Deleted {deleted} of {} Volumes before Docker rejected the operation: {}",
                        input.names.len(),
                        error.message
                    ),
                    &headers,
                ));
            }
        }
        publish_runtime_change(&state, input.platform_id, "volume", "remove", name);
    }
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

macro_rules! swarm_list_handler {
    ($name:ident, $method:ident, $item:ty) => {
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Path(platform_id) = identity_result(path.map_err(invalid_path), &headers)?;
            let capabilities =
                authorize_platform(&state, &principal, platform_id, &headers).await?;
            let mut items: Vec<$item> = identity_result(
                state
                    .platforms
                    .$method(platform_id)
                    .await
                    .map_err(platform_error),
                &headers,
            )?;
            for item in &mut items {
                item.capabilities = Some(capabilities);
            }
            Ok(no_store(
                Json(SwarmItemsResponse {
                    items,
                    capabilities,
                })
                .into_response(),
            ))
        }
    };
}

macro_rules! swarm_get_handler {
    ($name:ident, $method:ident) => {
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Path((platform_id, resource_id)) =
                identity_result(path.map_err(invalid_path), &headers)?;
            identity_result(validate_docker_resource_id(&resource_id), &headers)?;
            let capabilities =
                authorize_platform(&state, &principal, platform_id, &headers).await?;
            let mut item = required(
                state
                    .platforms
                    .$method(platform_id, &resource_id)
                    .await
                    .map_err(platform_error),
                &headers,
            )?;
            item.capabilities = Some(capabilities);
            Ok(no_store(Json(item).into_response()))
        }
    };
}

swarm_list_handler!(list_swarm_nodes, list_swarm_nodes, SwarmNodeView);
swarm_get_handler!(get_swarm_node, get_swarm_node);
swarm_list_handler!(list_swarm_services, list_swarm_services, SwarmServiceView);
swarm_get_handler!(get_swarm_service, get_swarm_service);

async fn list_swarm_tasks(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    filters: Result<Query<SwarmTaskFilters>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(platform_id) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(filters) = identity_result(filters.map_err(invalid_query), &headers)?;
    identity_result(validate_swarm_task_filters(&filters), &headers)?;
    let capabilities = authorize_platform(&state, &principal, platform_id, &headers).await?;
    let mut items = identity_result(
        state
            .platforms
            .list_swarm_tasks(platform_id, filters.service_id.as_deref(), filters.limit)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    for item in &mut items {
        item.capabilities = Some(capabilities);
    }
    Ok(no_store(
        Json(SwarmItemsResponse {
            items,
            capabilities,
        })
        .into_response(),
    ))
}

swarm_get_handler!(get_swarm_task, get_swarm_task);
swarm_list_handler!(list_swarm_networks, list_swarm_networks, SwarmNetworkView);
swarm_get_handler!(get_swarm_network, get_swarm_network);
swarm_list_handler!(list_swarm_configs, list_swarm_configs, SwarmConfigView);
swarm_get_handler!(get_swarm_config, get_swarm_config);
swarm_list_handler!(list_swarm_secrets, list_swarm_secrets, SwarmSecretView);
swarm_get_handler!(get_swarm_secret, get_swarm_secret);

async fn effective_permissions(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    ids: &[Uuid],
    headers: &HeaderMap,
) -> IdentityHttpResult<std::collections::BTreeMap<Uuid, EffectivePlatformPermission>> {
    if principal.is_administrator() {
        return Ok(ids
            .iter()
            .copied()
            .map(|id| {
                (
                    id,
                    EffectivePlatformPermission {
                        level_mask: ALL_LEVELS,
                        specific_mask: ALL_PLATFORM_SPECIFIC,
                    },
                )
            })
            .collect());
    }
    identity_result(
        state
            .platforms
            .permissions_for_platforms(principal.actor_id, ids)
            .await
            .map_err(platform_error),
        headers,
    )
}

async fn authorize_platform(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<PlatformCapabilitiesView> {
    let permissions = effective_permissions(state, principal, &[id], headers).await?;
    let permission = permission_for(&permissions, id, principal.is_administrator());
    if permission.level_mask & ALL_LEVELS == 0 {
        return identity_result(Err(IdentityError::Forbidden), headers);
    }
    Ok(platform_capabilities(permission))
}

async fn authorize_platform_creation(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    let permission = state
        .identity
        .global_permission(principal, ResourceType::Platform)
        .await
        .map_err(|error| crate::identity_http::IdentityHttpError::from_parts(error, headers))?;
    if permission.is_some_and(|value| value.level.grants(PermissionLevel::Write)) {
        Ok(())
    } else {
        identity_result(Err(IdentityError::Forbidden), headers)
    }
}

async fn authorize_platform_level(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    required: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<PlatformCapabilitiesView> {
    let permissions = effective_permissions(state, principal, &[id], headers).await?;
    let permission = permission_for(&permissions, id, principal.is_administrator());
    if !resource_capabilities(permission).grants(required) {
        return identity_result(Err(IdentityError::Forbidden), headers);
    }
    Ok(platform_capabilities(permission))
}

fn permission_for(
    permissions: &std::collections::BTreeMap<Uuid, EffectivePlatformPermission>,
    id: Uuid,
    is_administrator: bool,
) -> EffectivePlatformPermission {
    permissions.get(&id).copied().unwrap_or_else(|| {
        if is_administrator {
            EffectivePlatformPermission {
                level_mask: ALL_LEVELS,
                specific_mask: ALL_PLATFORM_SPECIFIC,
            }
        } else {
            EffectivePlatformPermission::default()
        }
    })
}

fn resource_capabilities(permission: EffectivePlatformPermission) -> ResourceCapabilitiesView {
    let can_execute = permission.level_mask & PermissionLevel::Execute as i32 != 0;
    let can_write = can_execute || permission.level_mask & PermissionLevel::Write as i32 != 0;
    ResourceCapabilitiesView {
        can_read: can_write || permission.level_mask & PermissionLevel::Read as i32 != 0,
        can_write,
        can_execute,
    }
}

trait ResourceCapabilitiesExt {
    fn grants(self, required: PermissionLevel) -> bool;
}

impl ResourceCapabilitiesExt for ResourceCapabilitiesView {
    fn grants(self, required: PermissionLevel) -> bool {
        match required {
            PermissionLevel::Read => self.can_read,
            PermissionLevel::Write => self.can_write,
            PermissionLevel::Execute => self.can_execute,
            _ => false,
        }
    }
}

fn platform_capabilities(permission: EffectivePlatformPermission) -> PlatformCapabilitiesView {
    let common = resource_capabilities(permission);
    PlatformCapabilitiesView {
        can_read: common.can_read,
        can_write: common.can_write,
        can_execute: common.can_execute,
        can_view_logs: common.can_read
            && permission.specific_mask & SpecificPermission::Logs as i32 != 0,
        can_inspect: common.can_read
            && permission.specific_mask & SpecificPermission::Inspect as i32 != 0,
        can_open_terminal: common.can_read
            && permission.specific_mask & SpecificPermission::Terminal as i32 != 0,
        can_pull: common.can_read
            && permission.specific_mask & SpecificPermission::Pull as i32 != 0,
        can_manage_node_agents: common.can_execute
            && permission.specific_mask & SpecificPermission::ManageNodeAgents as i32 != 0,
    }
}

fn image_capabilities(platform: PlatformCapabilitiesView) -> ImageCapabilitiesView {
    ImageCapabilitiesView {
        can_read: platform.can_read,
        can_write: platform.can_write,
        can_execute: platform.can_execute,
        can_inspect: platform.can_inspect,
        can_pull: platform.can_pull,
    }
}

fn network_capabilities(platform: PlatformCapabilitiesView) -> NetworkCapabilitiesView {
    NetworkCapabilitiesView {
        can_read: platform.can_read,
        can_write: platform.can_write,
        can_execute: platform.can_execute,
        can_inspect: platform.can_inspect,
    }
}

async fn volume_capabilities(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    platform_id: Uuid,
    platform: PlatformCapabilitiesView,
    headers: &HeaderMap,
) -> IdentityHttpResult<VolumeCapabilitiesView> {
    let content = if principal.is_administrator() {
        None
    } else {
        Some(
            state
                .identity
                .permission_for_resource(principal, ResourceType::Volume, platform_id)
                .await
                .map_err(|error| {
                    crate::identity_http::IdentityHttpError::from_parts(error, headers)
                })?,
        )
    };
    let has_content_permission = |specific| {
        principal.is_administrator()
            || content
                .as_ref()
                .and_then(|permission| permission.as_ref())
                .is_some_and(|permission| {
                    permission.level.grants(PermissionLevel::Read)
                        && permission.has_specific(specific)
                })
    };
    Ok(VolumeCapabilitiesView {
        can_read: platform.can_read,
        can_write: platform.can_write,
        can_execute: platform.can_execute,
        can_inspect: platform.can_inspect,
        can_browse: has_content_permission(SpecificPermission::Browse),
        can_download: has_content_permission(SpecificPermission::Download),
    })
}

pub(crate) enum RuntimeRef<'a> {
    Local(&'a DockerClient),
    Agent(&'a AgentClient),
    Edge(EdgeRuntime),
}

pub(crate) async fn realtime_daemon_snapshot(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
) -> Result<crate::realtime_groups::GroupSnapshot, crate::realtime::RealtimeReadError> {
    use crate::realtime::RealtimeReadError;
    use crate::realtime_groups::{GroupRows, GroupSnapshot, RowStyle};
    let failure = |error: RuntimeCapabilityError| RealtimeReadError::Storage(error.to_string());
    let headers = HeaderMap::new();
    let platform = authorize_platform(state, principal, id, &headers)
        .await
        .map_err(|_| RealtimeReadError::Authorization)?;
    let volume_cap = volume_capabilities(state, principal, id, platform, &headers)
        .await
        .map_err(|_| RealtimeReadError::Authorization)?;
    let runtime = runtime_for(state, id).await.map_err(failure)?;
    let cancellation = CancellationToken::new();
    let networks = match runtime {
        RuntimeRef::Local(runtime) => {
            PlatformInventoryPort::list_networks(runtime, &cancellation).await
        }
        RuntimeRef::Agent(runtime) => {
            PlatformInventoryPort::list_networks(runtime, &cancellation).await
        }
        RuntimeRef::Edge(ref runtime) => {
            PlatformInventoryPort::list_networks(runtime, &cancellation).await
        }
    }
    .map_err(failure)?;
    let volumes = match runtime {
        RuntimeRef::Local(runtime) => {
            PlatformInventoryPort::list_volumes(runtime, &cancellation).await
        }
        RuntimeRef::Agent(runtime) => {
            PlatformInventoryPort::list_volumes(runtime, &cancellation).await
        }
        RuntimeRef::Edge(ref runtime) => {
            PlatformInventoryPort::list_volumes(runtime, &cancellation).await
        }
    }
    .map_err(failure)?;
    let serialize = |error: serde_json::Error| RealtimeReadError::Storage(error.to_string());
    let mut events = vec![];
    if platform_is_swarm(state, id)
        .await
        .map_err(|error| RealtimeReadError::Storage(error.to_string()))?
    {
        let failure = |error: AuthorizedReadError| RealtimeReadError::Storage(error.to_string());
        let mut images = state.platforms.list_images(id).await.map_err(failure)?;
        for image in &mut images {
            image.capabilities = Some(image_capabilities(platform));
        }
        let mut all_volumes: Vec<_> = volumes
            .iter()
            .cloned()
            .map(|volume| map_volume(volume, volume_cap))
            .collect();
        all_volumes.extend(
            state
                .platforms
                .list_node_volumes(id)
                .await
                .map_err(failure)?
                .into_iter()
                .map(|volume| map_node_volume(volume, volume_cap)),
        );
        let node_networks: Vec<_> = state
            .platforms
            .list_node_networks(id)
            .await
            .map_err(failure)?
            .into_iter()
            .map(|network| map_node_network(network, network_capabilities(platform)))
            .collect();
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
                    .map(|n| serde_json::to_value(map_network(n, network_capabilities(platform))))
                    .collect::<Result<_, _>>()
                    .map_err(serialize)?,
            },
            GroupRows {
                target: "VolumeEventReceived",
                style: RowStyle::Daemon,
                rows: volumes
                    .into_iter()
                    .map(|v| serde_json::to_value(map_volume(v, volume_cap)))
                    .collect::<Result<_, _>>()
                    .map_err(serialize)?,
            },
        ],
    })
}

pub(crate) async fn runtime_for_node<'a>(
    state: &'a PlatformsHttpState,
    platform_id: Uuid,
    node_id: Option<&str>,
) -> Result<RuntimeRef<'a>, RuntimeCapabilityError> {
    let Some(node_id) = node_id else {
        return runtime_for(state, platform_id).await;
    };
    let node = state
        .platforms
        .get_swarm_node(platform_id, node_id)
        .await
        .map_err(|error| {
            RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
        })?
        .ok_or_else(|| {
            RuntimeCapabilityError::new(RuntimeErrorKind::NotFound, "Swarm Node not found.", false)
        })?;
    if !node.is_stale {
        if let Ok(session) = state
            .edge
            .get(&EdgeTarget::node(platform_id, node_id.into()))
        {
            return Ok(RuntimeRef::Edge(EdgeRuntime { session }));
        }
        let manager: Option<String> =
            sqlx::query_scalar("SELECT platformdescriptor->>'nodeID' FROM platforms WHERE id=$1")
                .bind(platform_id)
                .fetch_one(&state.pool)
                .await
                .map_err(|error| {
                    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
                })?;
        if manager.as_deref() != Some(node_id) {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Unavailable,
                "The owning Node Agent is disconnected or unavailable.",
                false,
            ));
        }
        // The connected manager is eligible only when its live identity matches.
        // Never treat a missing worker session as permission to use that daemon.
        let runtime = runtime_for(state, platform_id).await?;
        let cancellation = CancellationToken::new();
        let info = match &runtime {
            RuntimeRef::Local(runtime) => {
                citadel_platforms::PlatformRuntimePort::get_info(*runtime, &cancellation).await
            }
            RuntimeRef::Agent(runtime) => {
                citadel_platforms::PlatformRuntimePort::get_info(*runtime, &cancellation).await
            }
            RuntimeRef::Edge(runtime) => {
                citadel_platforms::PlatformRuntimePort::get_info(runtime, &cancellation).await
            }
        }?;
        if info.swarm.is_some_and(|swarm| {
            swarm.node_id == node_id && swarm.local_node_state.eq_ignore_ascii_case("active")
        }) {
            return Ok(runtime);
        }
    }
    Err(RuntimeCapabilityError::new(
        RuntimeErrorKind::Unavailable,
        "The owning Node Agent is disconnected or unavailable.",
        false,
    ))
}

async fn runtime_for(
    state: &PlatformsHttpState,
    platform_id: Uuid,
) -> Result<RuntimeRef<'_>, RuntimeCapabilityError> {
    let platform = sqlx::query_as::<_, (String, String)>(
        "SELECT connectortype, address FROM platforms WHERE id = $1 LIMIT 1",
    )
    .bind(platform_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| {
        RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
    })?
    .ok_or_else(|| {
        RuntimeCapabilityError::new(RuntimeErrorKind::NotFound, "Platform not found", false)
    })?;
    let (connector, address) = platform;
    if connector.eq_ignore_ascii_case("EdgeAgent") {
        return state
            .edge
            .get(&EdgeTarget::platform(platform_id))
            .map(|session| RuntimeRef::Edge(EdgeRuntime { session }))
            .map_err(|error| {
                RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable, error.to_string(), false)
            });
    }
    if connector.eq_ignore_ascii_case("Local") {
        return Ok(RuntimeRef::Local(&state.docker));
    }
    if connector.eq_ignore_ascii_case("Agent") {
        return state
            .agent
            .as_ref()
            .filter(|agent| agent.address().trim_end_matches('/') == address.trim_end_matches('/'))
            .map(RuntimeRef::Agent)
            .ok_or_else(|| {
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::Unavailable,
                    "The configured Agent transport is unavailable.",
                    true,
                )
            });
    }
    Err(RuntimeCapabilityError::new(
        RuntimeErrorKind::Unavailable,
        "The Edge Agent is disconnected or unavailable.",
        true,
    ))
}

fn map_network(
    network: RuntimeNetworkSummary,
    capabilities: NetworkCapabilitiesView,
) -> NetworkView {
    let is_system = network
        .labels
        .get("com.citadel.system")
        .is_some_and(|value| value == "true");
    NetworkView {
        name: network.name,
        id: network.id,
        created: network.created,
        driver: network.driver,
        scope: network.scope,
        enable_ipv4: network.enable_ipv4,
        enable_ipv6: network.enable_ipv6,
        internal: network.internal,
        attachable: network.attachable,
        ingress: network.ingress,
        config_only: network.config_only,
        in_use: network.container_count > 0,
        config_from: network.config_from,
        ipam: network.ipam,
        options: network.options,
        labels: network.labels,
        containers: network.containers,
        peers: network.peers,
        is_system,
        docker_node_id: None,
        node_hostname: None,
        is_stale: false,
        stale_reason: None,
        capabilities: Some(capabilities),
    }
}

fn map_node_network(
    node: citadel_platforms::NodeResourceProjection<RuntimeNetworkSummary>,
    capabilities: NetworkCapabilitiesView,
) -> NetworkView {
    let mut view = map_network(node.resource, capabilities);
    view.docker_node_id = Some(node.docker_node_id);
    view.node_hostname = node.node_hostname;
    view.is_stale = node.is_stale;
    view.stale_reason = node
        .is_stale
        .then(|| "Node Agent is disconnected or unavailable.".into());
    view
}

fn map_node_volume(
    node: citadel_platforms::NodeResourceProjection<RuntimeVolumeSummary>,
    capabilities: VolumeCapabilitiesView,
) -> VolumeView {
    let mut view = map_volume(node.resource, capabilities);
    view.docker_node_id = Some(node.docker_node_id);
    view.node_hostname = node.node_hostname;
    view.is_stale = node.is_stale;
    view.stale_reason = node
        .is_stale
        .then(|| "Node Agent is disconnected or unavailable.".into());
    view
}

fn map_volume(volume: RuntimeVolumeSummary, capabilities: VolumeCapabilitiesView) -> VolumeView {
    VolumeView {
        id: volume.name.clone(),
        name: volume.name,
        in_use: volume.in_use,
        scope: volume.scope,
        driver: volume.driver,
        mountpoint: volume.mountpoint,
        created_at: volume.created_at,
        cluster_volume: volume.cluster_volume,
        usage_data: volume.usage_data,
        containers: volume.containers,
        status: volume
            .status
            .into_iter()
            .map(|(key, value)| {
                let value = value
                    .as_str()
                    .map_or_else(|| value.to_string(), str::to_owned);
                (key, value)
            })
            .collect(),
        labels: volume.labels,
        options: volume.options,
        docker_node_id: None,
        node_hostname: None,
        is_stale: false,
        stale_reason: None,
        capabilities: Some(capabilities),
    }
}

fn network_matches(network: &RuntimeNetworkSummary, filters: &NetworkFilters) -> bool {
    filters
        .dangling
        .is_none_or(|dangling| dangling == (network.container_count == 0))
        && filters
            .driver
            .as_ref()
            .is_none_or(|driver| network.driver.eq_ignore_ascii_case(driver))
        && filters
            .id
            .as_ref()
            .is_none_or(|id| starts_with_ascii_case_insensitive(&network.id, id))
        && filters
            .name
            .as_ref()
            .is_none_or(|name| contains_ascii_case_insensitive(&network.name, name))
}

fn volume_matches(volume: &RuntimeVolumeSummary, filters: &VolumeFilters) -> bool {
    filters
        .dangling
        .is_none_or(|dangling| dangling == !volume.in_use)
        && filters
            .driver
            .as_ref()
            .is_none_or(|driver| volume.driver.eq_ignore_ascii_case(driver))
        && filters
            .name
            .as_ref()
            .is_none_or(|name| contains_ascii_case_insensitive(&volume.name, name))
}

fn starts_with_ascii_case_insensitive(value: &str, prefix: &str) -> bool {
    value
        .as_bytes()
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix.as_bytes()))
}

fn contains_ascii_case_insensitive(value: &str, needle: &str) -> bool {
    needle.is_empty()
        || value
            .as_bytes()
            .windows(needle.len())
            .any(|candidate| candidate.eq_ignore_ascii_case(needle.as_bytes()))
}

fn validate_filter_lengths<const N: usize>(
    values: [Option<&str>; N],
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        if values
            .into_iter()
            .flatten()
            .any(|value| value.len() > MAX_DOCKER_RESOURCE_ID_BYTES)
        {
            Err(IdentityError::Validation(
                "A Docker resource filter is too long.".to_owned(),
            ))
        } else {
            Ok(())
        },
        headers,
    )
}

fn validate_swarm_task_filters(filters: &SwarmTaskFilters) -> Result<(), IdentityError> {
    if !(1..=200).contains(&filters.limit) {
        return Err(IdentityError::Validation(
            "The Swarm Task limit must be between 1 and 200.".to_owned(),
        ));
    }
    if filters
        .service_id
        .as_ref()
        .is_some_and(|service_id| service_id.len() > 64)
    {
        return Err(IdentityError::Validation(
            "The Swarm Service ID is too long.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_network_input(input: &CreateRuntimeNetwork) -> Result<(), IdentityError> {
    validate_name_identifier(&input.name, "Network")?;
    if !matches!(
        input.driver.as_str(),
        "bridge" | "macvlan" | "ipvlan" | "overlay"
    ) {
        return Err(IdentityError::Validation(
            "Network driver must be bridge, macvlan, ipvlan, or overlay.".to_owned(),
        ));
    }
    if !matches!(input.scope.as_str(), "local" | "swarm") {
        return Err(IdentityError::Validation(
            "Network scope must be local or swarm.".to_owned(),
        ));
    }
    if (input.scope == "swarm") != (input.driver == "overlay") {
        return Err(IdentityError::Validation(
            "Overlay networks must use Swarm scope, and Swarm-scoped networks must use the overlay driver."
                .to_owned(),
        ));
    }
    if input.attachable == Some(true) && input.driver != "overlay" {
        return Err(IdentityError::Validation(
            "Attachable is available only for overlay networks.".to_owned(),
        ));
    }
    if input.internal.is_some() && !matches!(input.driver.as_str(), "bridge" | "overlay") {
        return Err(IdentityError::Validation(
            "Internal is available only for bridge and overlay networks.".to_owned(),
        ));
    }
    if input.enable_ipv4 == Some(false) && input.enable_ipv6 == Some(false) {
        return Err(IdentityError::Validation(
            "At least one of EnableIPv4 or EnableIPv6 must be enabled.".to_owned(),
        ));
    }
    validate_map(&input.labels, "Network label")?;
    validate_map(&input.options, "Network option")?;
    if let Some(ipam) = &input.ipam {
        if ipam.driver.trim().len() < 3 || ipam.config.len() != 2 {
            return Err(IdentityError::Validation(
                "IPAM requires a driver and exactly two configuration entries (IPv4 then IPv6)."
                    .to_owned(),
            ));
        }
        validate_map(&ipam.options, "IPAM option")?;
    }
    Ok(())
}

fn validate_volume_input(input: &CreateRuntimeVolume) -> Result<(), IdentityError> {
    validate_name_identifier(&input.name, "Volume")?;
    if input.driver.trim().is_empty()
        || input.driver.len() > 255
        || input.driver.chars().any(char::is_whitespace)
    {
        return Err(IdentityError::Validation(
            "Volume driver is required, cannot exceed 255 characters, and cannot contain whitespace."
                .to_owned(),
        ));
    }
    validate_map(&input.labels, "Volume label")?;
    validate_map(&input.options, "Volume option")
}

fn validate_name_identifier(value: &str, resource: &str) -> Result<(), IdentityError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_DOCKER_RESOURCE_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err(IdentityError::Validation(format!(
            "{resource} name contains unsupported characters."
        )));
    }
    Ok(())
}

fn validate_map(
    values: &std::collections::BTreeMap<String, String>,
    resource: &str,
) -> Result<(), IdentityError> {
    if values.len() > 256
        || values
            .iter()
            .any(|(key, value)| key.trim().is_empty() || key.len() > 256 || value.len() > 4096)
    {
        return Err(IdentityError::Validation(format!(
            "{resource} keys and values exceed the supported limits."
        )));
    }
    Ok(())
}

fn validate_resource_ids(
    values: &mut Vec<String>,
    maximum: usize,
    resource: &str,
) -> Result<(), IdentityError> {
    if values.is_empty() || values.len() > maximum {
        return Err(IdentityError::Validation(format!(
            "Between 1 and {maximum} {resource} identifiers are required."
        )));
    }
    for value in values.iter_mut() {
        *value = value.trim().to_owned();
        validate_docker_resource_id(value)?;
    }
    values.sort_unstable();
    values.dedup();
    Ok(())
}

async fn platform_is_swarm(
    state: &PlatformsHttpState,
    platform_id: Uuid,
) -> Result<bool, IdentityError> {
    let descriptor = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT platformdescriptor FROM platforms WHERE id=$1",
    )
    .bind(platform_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| IdentityError::Storage(error.to_string()))?
    .ok_or(IdentityError::NotFound)?;
    Ok(descriptor
        .get("$type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| kind.eq_ignore_ascii_case("DockerSwarm")))
}

async fn validate_swarm_network_projection(
    state: &PlatformsHttpState,
    platform_id: Uuid,
    network_id: &str,
    network_name: &str,
) -> Result<(), IdentityError> {
    let projection = sqlx::query_as::<_, (bool, serde_json::Value)>(
        "SELECT isstale, servicenames FROM swarmnetworkprojections WHERE platformid=$1 AND dockernetworkid=$2",
    )
    .bind(platform_id)
    .bind(network_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| IdentityError::Storage(error.to_string()))?;

    let Some((is_stale, service_names)) = projection else {
        return Err(IdentityError::Conflict(format!(
            "Swarm network '{network_name}' has no current inventory observation and cannot be deleted."
        )));
    };
    if is_stale {
        return Err(IdentityError::Conflict(format!(
            "Swarm network '{network_name}' has no current inventory observation and cannot be deleted."
        )));
    }
    if service_names
        .as_array()
        .is_some_and(|services| !services.is_empty())
    {
        return Err(IdentityError::Conflict(format!(
            "Network '{network_name}' is used by one or more Services and cannot be deleted."
        )));
    }
    Ok(())
}

fn publish_runtime_change(
    state: &PlatformsHttpState,
    platform_id: Uuid,
    resource_type: &str,
    action: &str,
    resource_id: &str,
) {
    if let Some(realtime) = &state.realtime {
        realtime.publish_runtime_change(platform_id, resource_type, action, resource_id);
    }
}

fn conflict_response(detail: String, headers: &HeaderMap) -> axum::response::Response {
    runtime_error_response(
        RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, detail, false),
        headers,
    )
}

fn runtime_error_response(
    error: RuntimeCapabilityError,
    headers: &HeaderMap,
) -> axum::response::Response {
    let (status, problem_type, title) = match error.kind {
        RuntimeErrorKind::Cancelled | RuntimeErrorKind::Timeout | RuntimeErrorKind::Unavailable => {
            (
                StatusCode::CONFLICT,
                "platform_unavailable",
                "Platform unavailable",
            )
        }
        RuntimeErrorKind::Authentication => (
            StatusCode::UNAUTHORIZED,
            "authentication_required",
            "Authentication required",
        ),
        RuntimeErrorKind::PermissionDenied => (StatusCode::FORBIDDEN, "forbidden", "Forbidden"),
        RuntimeErrorKind::InvalidRequest => (
            StatusCode::BAD_REQUEST,
            "validation_error",
            "Validation failed",
        ),
        RuntimeErrorKind::NotFound => (StatusCode::NOT_FOUND, "not_found", "Not found"),
        RuntimeErrorKind::Conflict => (StatusCode::CONFLICT, "conflict", "Conflict"),
        RuntimeErrorKind::ResourceExhausted => (
            StatusCode::TOO_MANY_REQUESTS,
            "capacity_exhausted",
            "Capacity exhausted",
        ),
        RuntimeErrorKind::Remote => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
        ),
    };
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let detail = if status.is_server_error() && error.kind == RuntimeErrorKind::Remote {
        "An unexpected error occurred.".to_owned()
    } else {
        error.message
    };
    let mut response = (
        status,
        Json(serde_json::json!({
            "type": format!("https://citadel.dev/problems/{problem_type}"),
            "title": title,
            "status": status.as_u16(),
            "detail": detail,
            "requestId": request_id,
        })),
    )
        .into_response();
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/problem+json"),
    );
    no_store(response)
}

fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(principal)| principal)
        .ok_or(IdentityError::Unauthenticated)
}

fn required<T>(
    result: Result<Option<T>, IdentityError>,
    headers: &HeaderMap,
) -> IdentityHttpResult<T> {
    identity_result(result, headers)?.ok_or_else(|| {
        crate::identity_http::IdentityHttpError::from_parts(IdentityError::NotFound, headers)
    })
}

fn invalid_path(_: PathRejection) -> IdentityError {
    IdentityError::Validation("The resource path is invalid.".to_owned())
}

fn invalid_query(_: QueryRejection) -> IdentityError {
    IdentityError::Validation("The resource query is invalid.".to_owned())
}

fn invalid_json(_: JsonRejection) -> IdentityError {
    IdentityError::Validation("The request body is invalid.".to_owned())
}

fn parse_tag_filters(query: Option<&str>) -> Result<Vec<Uuid>, IdentityError> {
    let mut tags = Vec::new();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if !key.eq_ignore_ascii_case("tags") {
            continue;
        }
        if tags.len() >= 100 {
            return Err(IdentityError::Validation(
                "At most 100 Platform tags may be filtered at once.".to_owned(),
            ));
        }
        let tag = Uuid::parse_str(&value)
            .map_err(|_| IdentityError::Validation("A Platform tag ID is invalid.".to_owned()))?;
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    Ok(tags)
}

fn validate_docker_resource_id(value: &str) -> Result<(), IdentityError> {
    if value.trim().is_empty() || value.len() > MAX_DOCKER_RESOURCE_ID_BYTES {
        return Err(IdentityError::Validation(
            "The Docker resource ID is invalid.".to_owned(),
        ));
    }
    Ok(())
}

fn platform_error(error: AuthorizedReadError) -> IdentityError {
    IdentityError::Storage(error.to_string())
}

fn platform_registration_error(error: PlatformRegistrationError) -> IdentityError {
    match error {
        PlatformRegistrationError::Validation(message) => IdentityError::Validation(message),
        PlatformRegistrationError::Conflict(message) => IdentityError::Conflict(message),
        PlatformRegistrationError::Storage(message) => IdentityError::Storage(message),
        PlatformRegistrationError::Runtime(_) => {
            IdentityError::Storage("Platform runtime failure was not handled.".to_owned())
        }
    }
}

fn resource_metadata_error(error: citadel_resources::ResourceMetadataError) -> IdentityError {
    match error {
        citadel_resources::ResourceMetadataError::Validation(message) => {
            IdentityError::Validation(message)
        }
        citadel_resources::ResourceMetadataError::NotFound => IdentityError::NotFound,
        citadel_resources::ResourceMetadataError::Conflict(message) => {
            IdentityError::Conflict(message)
        }
        citadel_resources::ResourceMetadataError::Credential => IdentityError::Credential,
        citadel_resources::ResourceMetadataError::Storage(message) => {
            IdentityError::Storage(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_implies_write_and_read_without_granting_specific_operations() {
        let capabilities = platform_capabilities(EffectivePlatformPermission {
            level_mask: PermissionLevel::Execute as i32,
            specific_mask: SpecificPermission::ManageNodeAgents as i32,
        });

        assert!(capabilities.can_read);
        assert!(capabilities.can_write);
        assert!(capabilities.can_execute);
        assert!(capabilities.can_manage_node_agents);
        assert!(!capabilities.can_inspect);
    }

    #[test]
    fn docker_ids_are_bounded_before_storage_or_transport() {
        assert!(validate_docker_resource_id("node-1").is_ok());
        assert!(validate_docker_resource_id(" ").is_err());
        assert!(validate_docker_resource_id(&"a".repeat(257)).is_err());
    }

    #[test]
    fn task_filters_preserve_dotnet_bounds() {
        assert!(
            validate_swarm_task_filters(&SwarmTaskFilters {
                limit: 1,
                service_id: Some("service-1".into()),
            })
            .is_ok()
        );
        assert!(
            validate_swarm_task_filters(&SwarmTaskFilters {
                limit: 0,
                service_id: None,
            })
            .is_err()
        );
        assert!(
            validate_swarm_task_filters(&SwarmTaskFilters {
                limit: 201,
                service_id: None,
            })
            .is_err()
        );
        assert!(
            validate_swarm_task_filters(&SwarmTaskFilters {
                limit: 50,
                service_id: Some("x".repeat(65)),
            })
            .is_err()
        );
    }

    #[test]
    fn docker_inventory_filters_use_runtime_identity_and_usage() {
        let network = RuntimeNetworkSummary {
            id: "abcdef".into(),
            name: "frontend".into(),
            driver: "bridge".into(),
            container_count: 0,
            ..RuntimeNetworkSummary::default()
        };
        assert!(network_matches(
            &network,
            &NetworkFilters {
                dangling: Some(true),
                driver: Some("BRIDGE".into()),
                id: Some("ABC".into()),
                name: Some("FRONT".into()),
            }
        ));
        let volume = RuntimeVolumeSummary {
            name: "database".into(),
            in_use: true,
            driver: "local".into(),
            ..RuntimeVolumeSummary::default()
        };
        assert!(volume_matches(
            &volume,
            &VolumeFilters {
                dangling: Some(false),
                driver: Some("LOCAL".into()),
                name: Some("DATA".into()),
            }
        ));
    }

    #[test]
    fn disconnected_platform_uses_the_existing_conflict_contract() {
        let response = runtime_error_response(
            RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable, "offline", true),
            &HeaderMap::new(),
        );
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn network_mutation_validation_preserves_swarm_scope_rules() {
        let valid = CreateRuntimeNetwork {
            name: "frontend".into(),
            driver: "overlay".into(),
            scope: "swarm".into(),
            internal: None,
            attachable: Some(true),
            ingress: None,
            enable_ipv6: Some(false),
            enable_ipv4: Some(true),
            config_only: None,
            ipam: None,
            config_from: None,
            labels: Default::default(),
            options: Default::default(),
        };
        assert!(validate_network_input(&valid).is_ok());
        assert!(
            validate_network_input(&CreateRuntimeNetwork {
                driver: "bridge".into(),
                ..valid.clone()
            })
            .is_err()
        );
        assert!(
            validate_network_input(&CreateRuntimeNetwork {
                enable_ipv4: Some(false),
                ..valid
            })
            .is_err()
        );
    }

    #[test]
    fn batch_mutation_ids_are_bounded_trimmed_and_deduplicated() {
        let mut ids = vec![" network-1 ".into(), "network-1".into(), "network-2".into()];
        validate_resource_ids(&mut ids, 100, "Network").unwrap();
        assert_eq!(ids, ["network-1", "network-2"]);
        assert!(validate_resource_ids(&mut Vec::new(), 100, "Network").is_err());
    }
}
