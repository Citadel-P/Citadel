//! Platforms HTTP routes, authorization and local request handling.
use crate::api::error::runtime_error_response;
use crate::api::resources::capabilities::ResourceCapabilitiesView;
use crate::api::resources::platforms::capabilities::{
    permission_for, platform_capabilities, resource_capabilities,
};
use crate::api::resources::platforms::container_views::{
    ContainerRuntimeListView, ContainerRuntimeView, ContainerSummaryView,
};
use crate::api::resources::platforms::edge::EdgeHttpContext;
use crate::api::resources::platforms::inventory_views::CreatedNetworkView;
use crate::api::resources::platforms::runtime_views::*;
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::platforms::{
            requests::{
                ContentQuery, CreateNetworkInput, CreatePlatformInput, CreateRuntimeNetwork,
                CreateRuntimeVolume, CreateVolumeInput, DeleteImagesInput, DeleteInput,
                DeleteNetworksInput, DeletePlatformsInput, DeleteSwarmResourcesInput,
                DeleteVolumesInput, DockerNodeSelector, Hours, NetworkFilters,
                PatchPlatformMetadataInput, PrunePlatformInput, PullImageInput,
                RenamePlatformInput, SwarmTaskFilters, Tail, UpdateSwarmNodeInput,
                UpdateSwarmNodesAvailabilityInput, UpdateSwarmResourceLabelsInput, VolumeFilters,
            },
            swarm_views,
            views::{
                ContainerHistory, ContainerView, ContainersResponse, History, ImagesResponse,
                NetworkView, NetworksResponse, PlatformCapabilitiesView, PlatformView,
                PlatformsResponse, StackHistory, SwarmConfigView, SwarmItemsResponse,
                SwarmNetworkView, SwarmNodeView, SwarmSecretView, SwarmServiceView, SwarmTaskView,
                TaskHistory, TaskTerminalView, VolumeCapabilitiesView, VolumeView, VolumesResponse,
            },
        },
    },
    openapi::router::OpenApiRouterExt,
    realtime::RealtimeHub,
    request_validation::{invalid_json, invalid_path, invalid_query},
};
use citadel_platforms::edge_management::EdgeTarget;
use citadel_primitives::PatchField;
use futures_util::StreamExt;

use axum::{
    Json, Router,
    extract::{
        Extension, Path, Query, RawQuery, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_platforms::{
    AuthorizedReadError, EffectivePlatformPermission, PlatformReadService,
    PlatformRegistrationError, PlatformRegistrationService, RuntimeCapabilityError,
    RuntimeErrorKind, RuntimeNetworkSummary, RuntimeVolumeSummary, StatisticsReader,
    StatisticsWorkload, StatsWindow,
    containers::ContainerAction,
    deletion::PlatformDeletionError,
    image_pull::{ImagePullError, PullImageStreamItem},
    logs::LogResource,
    management::patch_input,
    node_agents::setup::{NodeAgentSetupService, SetupKind, SetupOptions},
    swarm_mutations::{CreateSwarmMaterialInput, SwarmResourceKind, manager_identity, resource_id},
    volume_content::normalize_path,
};

use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};

use citadel_registries::registry_images::{RegistryBrowseKind, validate_browse_name};

use citadel_swarm_services::SwarmServiceRepository;

use serde_json::json;

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

const ALL_LEVELS: i32 =
    PermissionLevel::Read as i32 | PermissionLevel::Write as i32 | PermissionLevel::Execute as i32;

const ALL_PLATFORM_SPECIFIC: i32 = SpecificPermission::Logs as i32
    | SpecificPermission::Inspect as i32
    | SpecificPermission::Pull as i32
    | SpecificPermission::Terminal as i32
    | SpecificPermission::ManageNodeAgents as i32;

const MAX_DOCKER_RESOURCE_ID_BYTES: usize = 256;

#[derive(Clone)]
pub struct PlatformsHttpState {
    pub node_agent_store: Arc<dyn citadel_platforms::node_agents::setup::NodeAgentSetupStore>,
    pub node_agent_runtime: Arc<dyn citadel_platforms::node_agents::setup::NodeAgentSetupRuntime>,
    pub node_agent_coverage: Arc<dyn citadel_platforms::node_agents::NodeAgentCoverageReader>,
    pub deletions: Arc<citadel_platforms::deletion::PlatformDeletionService>,
    pub management: Arc<citadel_platforms::management::PlatformManagementService>,
    pub image_store: Arc<dyn citadel_platforms::image_mutations::ImageMutationStore>,
    pub projections: Arc<dyn citadel_platforms::InventoryProjectionStore>,
    pub statistics: Arc<dyn StatisticsReader>,
    pub services: Arc<dyn SwarmServiceRepository>,
    pub runtime: Arc<dyn citadel_platforms::runtime_provider::PlatformRuntimeProvider>,
    pub tasks: citadel_runtime::DynamicTasks,
    pub volume_content: Arc<dyn citadel_platforms::volume_content::VolumeContentPort>,
    pub volume_activity: Arc<dyn citadel_activities::VolumeDownloadActivitySink>,
    pub volume_coverage: Arc<dyn citadel_backups::runs::read_models::VolumeCoverageReader>,
    pub registry_browser: Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>,
    pub containers: Arc<citadel_platforms::containers::ContainerMutationService>,
    pub identity: Arc<IdentityService>,
    pub platforms: Arc<PlatformReadService>,
    pub registrations: Arc<PlatformRegistrationService>,
    pub registries: Arc<dyn citadel_registries::RegistryRepository>,
    pub platform_metadata: Arc<dyn citadel_platforms::PlatformMetadataRepository>,
    pub realtime: Option<RealtimeHub>,
    pub stats_sample_max_age: std::time::Duration,
}

pub fn router(state: PlatformsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<PlatformsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(refresh_swarm_inventory))
        .normalized_routes(utoipa_axum::routes!(inspect_swarm_task))
        .normalized_routes(utoipa_axum::routes!(terminal))
        .normalized_routes(utoipa_axum::routes!(inspect_container))
        .normalized_routes(utoipa_axum::routes!(info))
        .normalized_routes(utoipa_axum::routes!(data))
        .normalized_routes(utoipa_axum::routes!(deployment_info))
        .normalized_routes(utoipa_axum::routes!(start_deployments))
        .normalized_routes(utoipa_axum::routes!(stop_deployments))
        .normalized_routes(utoipa_axum::routes!(restart_deployments))
        .normalized_routes(utoipa_axum::routes!(pause_deployments))
        .normalized_routes(utoipa_axum::routes!(resume_deployments))
        .normalized_routes(utoipa_axum::routes!(inspect_deployment))
        .normalized_routes(utoipa_axum::routes!(stack_data))
        .normalized_routes(utoipa_axum::routes!(inspect_stack))
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(download))
        .normalized_routes(utoipa_axum::routes!(list_platforms))
        .normalized_routes(utoipa_axum::routes!(create_platform))
        .normalized_routes(utoipa_axum::routes!(delete_platforms))
        .normalized_routes(utoipa_axum::routes!(get_agent_setup))
        .normalized_routes(utoipa_axum::routes!(rotate_key))
        .normalized_routes(utoipa_axum::routes!(patch))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(prune))
        .normalized_routes(utoipa_axum::routes!(get_platform))
        .normalized_routes(utoipa_axum::routes!(enroll))
        .normalized_routes(utoipa_axum::routes!(status))
        .normalized_routes(utoipa_axum::routes!(node_coverage))
        .normalized_routes(utoipa_axum::routes!(remove_node_agents))
        .normalized_routes(utoipa_axum::routes!(install_node_agents))
        .normalized_routes(utoipa_axum::routes!(repair_node_agents))
        .normalized_routes(utoipa_axum::routes!(upgrade_node_agents))
        .normalized_routes(utoipa_axum::routes!(revoke))
        .normalized_routes(utoipa_axum::routes!(update_platform_metadata))
        .normalized_routes(utoipa_axum::routes!(list_containers))
        .normalized_routes(utoipa_axum::routes!(get_container))
        .normalized_routes(utoipa_axum::routes!(start))
        .normalized_routes(utoipa_axum::routes!(stop))
        .normalized_routes(utoipa_axum::routes!(restart))
        .normalized_routes(utoipa_axum::routes!(pause))
        .normalized_routes(utoipa_axum::routes!(unpause))
        .normalized_routes(utoipa_axum::routes!(delete_containers))
        .normalized_routes(utoipa_axum::routes!(container))
        .normalized_routes(utoipa_axum::routes!(platform))
        .normalized_routes(utoipa_axum::routes!(deployment))
        .normalized_routes(utoipa_axum::routes!(stack))
        .normalized_routes(utoipa_axum::routes!(service_statistics))
        .normalized_routes(utoipa_axum::routes!(task_statistics))
        .normalized_routes(utoipa_axum::routes!(service_logs))
        .normalized_routes(utoipa_axum::routes!(inspect_managed_service))
        .normalized_routes(utoipa_axum::routes!(task_logs))
        .normalized_routes(utoipa_axum::routes!(managed_service))
        .normalized_routes(utoipa_axum::routes!(list_images))
        .normalized_routes(utoipa_axum::routes!(delete_images))
        .normalized_routes(utoipa_axum::routes!(pull))
        .normalized_routes(utoipa_axum::routes!(repositories))
        .normalized_routes(utoipa_axum::routes!(docker_repositories))
        .normalized_routes(utoipa_axum::routes!(docker_tags))
        .normalized_routes(utoipa_axum::routes!(github_versions))
        .normalized_routes(utoipa_axum::routes!(inspect_image))
        .normalized_routes(utoipa_axum::routes!(exposed_ports))
        .normalized_routes(utoipa_axum::routes!(list_networks))
        .normalized_routes(utoipa_axum::routes!(get_network))
        .normalized_routes(utoipa_axum::routes!(create_network))
        .normalized_routes(utoipa_axum::routes!(delete_networks))
        .normalized_routes(utoipa_axum::routes!(list_volumes))
        .normalized_routes(utoipa_axum::routes!(get_volume))
        .normalized_routes(utoipa_axum::routes!(create_volume))
        .normalized_routes(utoipa_axum::routes!(delete_volumes))
        .normalized_routes(utoipa_axum::routes!(list_swarm_nodes))
        .normalized_routes(utoipa_axum::routes!(get_swarm_node))
        .normalized_routes(utoipa_axum::routes!(list_swarm_services))
        .normalized_routes(utoipa_axum::routes!(get_swarm_service))
        .normalized_routes(utoipa_axum::routes!(list_swarm_tasks))
        .normalized_routes(utoipa_axum::routes!(get_swarm_task))
        .normalized_routes(utoipa_axum::routes!(list_swarm_networks))
        .normalized_routes(utoipa_axum::routes!(get_swarm_network))
        .normalized_routes(utoipa_axum::routes!(list_swarm_configs))
        .normalized_routes(utoipa_axum::routes!(get_swarm_config))
        .normalized_routes(utoipa_axum::routes!(list_swarm_secrets))
        .normalized_routes(utoipa_axum::routes!(get_swarm_secret))
        .normalized_routes(utoipa_axum::routes!(update_node))
        .normalized_routes(utoipa_axum::routes!(inspect_node))
        .normalized_routes(utoipa_axum::routes!(update_availability))
        .normalized_routes(utoipa_axum::routes!(delete_services))
        .normalized_routes(utoipa_axum::routes!(inspect_service))
        .normalized_routes(utoipa_axum::routes!(restart_service))
        .normalized_routes(utoipa_axum::routes!(create_secret))
        .normalized_routes(utoipa_axum::routes!(delete_secrets))
        .normalized_routes(utoipa_axum::routes!(update_secret_labels))
        .normalized_routes(utoipa_axum::routes!(create_config))
        .normalized_routes(utoipa_axum::routes!(delete_configs))
        .normalized_routes(utoipa_axum::routes!(config_data))
        .normalized_routes(utoipa_axum::routes!(update_config_labels))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/agent/setup",
    operation_id = "getAgentSetup",
    tag = "Platforms",
    summary = "Get regular Agent setup instructions",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::AgentSetupView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_agent_setup(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    setup: Option<Extension<Arc<crate::api::resources::platforms::views::AgentSetupView>>>,
    live: Option<Extension<AgentSetupContext>>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(
        principal
            .map(|Extension(actor)| actor)
            .ok_or(ApiError::Unauthenticated),
        &headers,
    )?;
    api_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Platform,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Extension(setup) = api_result(
        setup.ok_or_else(|| ApiError::Storage("Agent setup is not configured.".into())),
        &headers,
    )?;
    if let Some(Extension(live)) = live {
        return Ok(no_store(Json(live.view()).into_response()));
    }
    Ok(no_store(Json(setup.as_ref()).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms",
    operation_id = "listPlatforms",
    tag = "Platforms",
    summary = "List authorized Platforms",
    responses(
        (status = 200, description = "Success", body = PlatformsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_platforms(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(raw_query): RawQuery,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let tags = api_result(parse_tag_filters(raw_query.as_deref()), &headers)?;
    let mut platforms = api_result(
        state
            .platforms
            .list_authorized(principal.actor_id, principal.is_administrator(), &tags)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .into_iter()
                    .map(PlatformView::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ApiError::internal)
            }),
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
            .map_err(|error| crate::api::error::HttpError::from_parts(error, &headers))?
            .map_or_else(EffectivePlatformPermission::default, |permission| {
                EffectivePlatformPermission {
                    level_mask: permission.level as i32,
                    // Project the capability response using the Platform read model.
                    specific_mask: permission.specifics.bits() as i32,
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

#[utoipa::path(
    post,
    path = "/api/v1/platforms",
    operation_id = "createPlatform",
    tag = "Platforms",
    summary = "Create a Platform",
    request_body = CreatePlatformInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_platform(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreatePlatformInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    authorize_platform_creation(&state, &principal, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::CreatePlatformInput = input.into();
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
            return api_result(Err(platform_registration_error(error)), &headers);
        }
    };
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "create", &id.to_string());
    Ok(no_store(Json(platform).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}",
    operation_id = "getPlatfom",
    tag = "Platforms",
    summary = "Get a Platform",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_platform(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    Ok(no_store(Json(platform).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{id}/_metadata",
    operation_id = "updatePlatformMetadata",
    tag = "Platforms",
    summary = "Update Platform metadata",
    request_body(content(
        (PatchPlatformMetadataInput = "application/merge-patch+json"),
        (PatchPlatformMetadataInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_platform_metadata(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchPlatformMetadataInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;

    let description = match input.description {
        PatchField::Missing => None,
        PatchField::Null => Some(None),
        PatchField::Value(value) => {
            if value.chars().count() > 600 {
                return api_result(
                    Err(ApiError::Validation(
                        "Description cannot exceed 600 characters.".to_owned(),
                    )),
                    &headers,
                );
            }
            Some(Some(value))
        }
    };
    if let Some(description) = description {
        api_result(
            state
                .platform_metadata
                .update_platform_description(id, description.as_deref())
                .await
                .map_err(platform_metadata_error),
            &headers,
        )?;
    }
    let capabilities = authorize_platform(&state, &principal, id, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    platform.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(Json(platform).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/containers",
    operation_id = "listContainers",
    tag = "Platforms",
    summary = "List Platform Containers",
    responses(
        (status = 200, description = "Success", body = ContainersResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_containers(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(platform_id) = api_result(path.map_err(invalid_path), &headers)?;
    let capabilities = authorize_platform(&state, &principal, platform_id, &headers).await?;
    let mut containers = api_result(
        state
            .platforms
            .list_containers(platform_id)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::ContainerView::from)
                    .collect::<Vec<_>>()
            })
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

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}",
    operation_id = "getContainer",
    tag = "Containers",
    summary = "Get a Container",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::ContainerView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_container(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(reference) = api_result(path.map_err(invalid_path), &headers)?;
    if !valid_container_reference(&reference) {
        return api_result(
            Err(ApiError::Validation("Must be a valid container id".into())),
            &headers,
        );
    }
    let id = match Uuid::parse_str(&reference) {
        Ok(id) => id,
        Err(_) => {
            let store = &state.statistics;
            match store.find_container(&reference).await {
                Ok(target) => required(Ok(target), &headers)?.id,
                Err(error) => return Ok(runtime_error_response(error, &headers)),
            }
        }
    };
    let mut container = required(
        state
            .platforms
            .get_container(id)
            .await
            .map(|value| value.map(crate::api::resources::platforms::views::ContainerView::from))
            .map_err(platform_error),
        &headers,
    )?;
    container.capabilities =
        Some(authorize_platform(&state, &principal, container.platform_id, &headers).await?);
    Ok(no_store(Json(container).into_response()))
}

fn valid_container_reference(reference: &str) -> bool {
    match Uuid::parse_str(reference) {
        Ok(id) => !id.is_nil(),
        Err(_) => {
            (12..=64).contains(&reference.len()) && reference.bytes().all(|b| b.is_ascii_hexdigit())
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/images/{platformId}",
    operation_id = "listImages",
    tag = "Images",
    summary = "List Platform Images",
    responses(
        (status = 200, description = "Success", body = ImagesResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_images(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(platform_id) = api_result(path.map_err(invalid_path), &headers)?;
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = image_capabilities(platform_capabilities);
    let mut images = api_result(
        state
            .platforms
            .list_images(platform_id)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::ImageView::from)
                    .collect::<Vec<_>>()
            })
            .map_err(platform_error),
        &headers,
    )?;
    let registries = if images.iter().any(|image| image.registry_id.is_some()) {
        api_result(
            state
                .registries
                .list_registries(principal.actor_id, principal.is_administrator())
                .await
                .map_err(ApiError::internal),
            &headers,
        )?
    } else {
        Vec::new()
    };
    let registries = api_result(
        registries
            .into_iter()
            .map(|registry| {
                Ok((
                    registry.id,
                    crate::api::resources::platforms::views::ImageRegistryView {
                        id: registry.id,
                        name: registry.name,
                        registry_host: registry.registry_host,
                        registry_type: serde_json::from_value(registry.registry_type.into())?,
                    },
                ))
            })
            .collect::<Result<std::collections::HashMap<_, _>, serde_json::Error>>()
            .map_err(ApiError::internal),
        &headers,
    )?;
    for image in &mut images {
        image.registry = image
            .registry_id
            .and_then(|id| registries.get(&id).cloned());
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

#[utoipa::path(
    get,
    path = "/api/v1/networks/{platformId}",
    operation_id = "listNetworks",
    tag = "Networks",
    summary = "List Platform Networks",
    responses(
        (status = 200, description = "Success", body = NetworksResponse, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("Dangling" = Option<bool>, Query), ("Driver" = Option<String>, Query), ("Id" = Option<String>, Query), ("Name" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_networks(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    filters: Result<Query<NetworkFilters>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(platform_id) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(filters) = api_result(filters.map_err(invalid_query), &headers)?;
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
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let values = async {
        state
            .runtime
            .inventory(platform_id, &cancellation)
            .await?
            .list_networks(&cancellation)
            .await
    }
    .await;
    let values = match values {
        Ok(values) => values,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let mut networks: Vec<_> = api_result(
        values
            .into_iter()
            .filter(|network| network_matches(network, &filters))
            .map(|network| map_network(network, capabilities))
            .collect::<Result<_, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?;
    let node_networks = api_result(
        state
            .platforms
            .list_node_networks(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    networks.extend(api_result(
        node_networks
            .into_iter()
            .filter(|network| network_matches(&network.resource, &filters))
            .map(|network| map_node_network(network, capabilities))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?);
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
    kind: citadel_discovery::LookupResourceType,
    headers: &HeaderMap,
) -> HttpResult {
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let names = async {
        let runtime = state.runtime.inventory(platform_id, &cancellation).await?;
        if kind == citadel_discovery::LookupResourceType::Volume {
            runtime
                .list_volumes(&cancellation)
                .await
                .map(|values| values.into_iter().map(|v| v.name).collect::<Vec<_>>())
        } else {
            runtime
                .list_networks(&cancellation)
                .await
                .map(|values| values.into_iter().map(|v| v.name).collect::<Vec<_>>())
        }
    }
    .await;
    let mut names = match names {
        Ok(names) => names,
        Err(error) => return Ok(runtime_error_response(error, headers)),
    };
    names.sort();
    let rows: Vec<_> = names
        .into_iter()
        .map(|name| citadel_discovery::LookupResourceInfo {
            id: Uuid::nil(),
            name,
            group: None,
        })
        .collect();
    Ok(no_store(Json(rows).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/networks/{platformId}/{networkId}",
    operation_id = "inspectNetwork",
    tag = "Networks",
    summary = "Inspect a Platform Network",
    responses(
        (status = 200, description = "Success", body = NetworkView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("networkId" = String, Path), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_network(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, network_id)) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(selector) = api_result(selector.map_err(invalid_query), &headers)?;
    api_result(validate_docker_resource_id(&network_id), &headers)?;
    if let Some(node_id) = selector.docker_node_id.as_deref() {
        api_result(validate_docker_resource_id(node_id), &headers)?;
    }
    let platform_capabilities =
        authorize_platform(&state, &principal, platform_id, &headers).await?;
    let capabilities = network_capabilities(platform_capabilities);
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let value = async {
        state
            .runtime
            .networks(
                platform_id,
                selector.docker_node_id.as_deref(),
                &cancellation,
            )
            .await?
            .inspect_network(&network_id, &cancellation)
            .await
    }
    .await;
    let mut network = match value {
        Ok(network) => api_result(
            map_network(network, capabilities).map_err(ApiError::internal),
            &headers,
        )?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    network.docker_node_id = selector.docker_node_id;
    Ok(no_store(Json(network).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/networks",
    operation_id = "createNetwork",
    tag = "Networks",
    summary = "Create a Network",
    request_body = CreateNetworkInput,
    responses(
        (status = 200, description = "Success", body = CreatedNetworkView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_network(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateNetworkInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(validate_network_input(&input.network), &headers)?;
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    if input.network.scope.eq_ignore_ascii_case("swarm")
        && !api_result(platform_is_swarm(&state, input.platform_id).await, &headers)?
    {
        return api_result(
            Err(ApiError::Validation(
                "Swarm-scoped overlay networks require a Docker Swarm platform.".to_owned(),
            )),
            &headers,
        );
    }
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let result = async {
        state
            .runtime
            .networks(input.platform_id, None, &cancellation)
            .await?
            .create_network(&input.network.into(), &cancellation)
            .await
    }
    .await;
    let created = match result {
        Ok(created) => created,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    publish_runtime_change(&state, input.platform_id, "network", "create", &created.id);
    Ok(no_store(
        Json(CreatedNetworkView { id: created.id }).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/networks",
    operation_id = "deleteNetworks",
    tag = "Networks",
    summary = "Delete Networks",
    request_body = DeleteNetworksInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_networks(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteNetworksInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(mut input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
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
    let is_swarm = api_result(platform_is_swarm(&state, input.platform_id).await, &headers)?;
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let runtime = match state
        .runtime
        .networks(input.platform_id, None, &cancellation)
        .await
    {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    match citadel_platforms::resource_mutations::delete_networks(
        &state.platforms,
        runtime.as_ref(),
        input.platform_id,
        is_swarm,
        &input.ids,
        &cancellation,
        |id| publish_runtime_change(&state, input.platform_id, "network", "remove", id),
    )
    .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(citadel_platforms::resource_mutations::NetworkDeletionError::Read(error)) => {
            api_result(Err(platform_error(error)), &headers)
        }
        Err(citadel_platforms::resource_mutations::NetworkDeletionError::Runtime(error)) => {
            Ok(runtime_error_response(error, &headers))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/volumes/{platformId}",
    operation_id = "listVolumes",
    tag = "Volumes",
    summary = "List Platform Volumes",
    responses(
        (status = 200, description = "Success", body = VolumesResponse, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("Dangling" = Option<bool>, Query), ("Driver" = Option<String>, Query), ("Name" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_volumes(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    filters: Result<Query<VolumeFilters>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(platform_id) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(filters) = api_result(filters.map_err(invalid_query), &headers)?;
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
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let values = async {
        state
            .runtime
            .inventory(platform_id, &cancellation)
            .await?
            .list_volumes(&cancellation)
            .await
    }
    .await;
    let values = match values {
        Ok(values) => values,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let mut volumes: Vec<_> = api_result(
        values
            .into_iter()
            .filter(|volume| volume_matches(volume, &filters))
            .map(|volume| map_volume(volume, capabilities))
            .collect::<Result<_, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?;
    let node_volumes = api_result(
        state
            .platforms
            .list_node_volumes(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    volumes.extend(api_result(
        node_volumes
            .into_iter()
            .filter(|volume| volume_matches(&volume.resource, &filters))
            .map(|volume| map_node_volume(volume, capabilities))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ApiError::internal),
        &headers,
    )?);
    attach_volume_coverage(&state, &principal, platform_id, &mut volumes, &headers).await?;
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

#[utoipa::path(
    get,
    path = "/api/v1/volumes/{platformId}/{name}",
    operation_id = "inspectVolume",
    tag = "Volumes",
    summary = "Inspect a Platform Volume",
    responses(
        (status = 200, description = "Success", body = VolumeView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("name" = String, Path), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_volume(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, name)) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(selector) = api_result(selector.map_err(invalid_query), &headers)?;
    api_result(validate_docker_resource_id(&name), &headers)?;
    if let Some(node_id) = selector.docker_node_id.as_deref() {
        api_result(validate_docker_resource_id(node_id), &headers)?;
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
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let value = async {
        state
            .runtime
            .volumes(
                platform_id,
                selector.docker_node_id.as_deref(),
                &cancellation,
            )
            .await?
            .inspect_volume(&name, &cancellation)
            .await
    }
    .await;
    let mut volume = match value {
        Ok(volume) => api_result(
            map_volume(volume, capabilities).map_err(ApiError::internal),
            &headers,
        )?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    volume.docker_node_id = selector.docker_node_id;
    attach_volume_coverage(
        &state,
        &principal,
        platform_id,
        std::slice::from_mut(&mut volume),
        &headers,
    )
    .await?;
    Ok(no_store(Json(volume).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/volumes",
    operation_id = "createVolume",
    tag = "Volumes",
    summary = "Create a Volume",
    request_body = CreateVolumeInput,
    responses(
        (status = 200, description = "Success", body = VolumeView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_volume(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateVolumeInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(validate_volume_input(&input.volume), &headers)?;
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
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let result = async {
        state
            .runtime
            .volumes(input.platform_id, None, &cancellation)
            .await?
            .create_volume(&input.volume.into(), &cancellation)
            .await
    }
    .await;
    let volume = match result {
        Ok(volume) => volume,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    publish_runtime_change(&state, input.platform_id, "volume", "create", &volume.name);
    Ok(no_store(
        Json(api_result(
            map_volume(volume, capabilities).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/volumes",
    operation_id = "deleteVolumes",
    tag = "Volumes",
    summary = "Delete Volumes",
    request_body = DeleteVolumesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_volumes(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteVolumesInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(mut input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
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
    if api_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
        return Ok(conflict_response(
            "Node-local Volume deletion requires an explicit Node target and is not available."
                .to_owned(),
            &headers,
        ));
    }
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let result = async {
        let runtime = state
            .runtime
            .volumes(input.platform_id, None, &cancellation)
            .await?;
        citadel_platforms::resource_mutations::delete_volumes(
            runtime.as_ref(),
            &input.names,
            input.force.unwrap_or(false),
            &cancellation,
            |name| publish_runtime_change(&state, input.platform_id, "volume", "remove", name),
        )
        .await
    }
    .await;
    match result {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

macro_rules! swarm_list_handler {
    ($(#[$name_attr:meta])* $name:ident, $method:ident, $item:ty) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path(platform_id) = api_result(path.map_err(invalid_path), &headers)?;
            let capabilities =
                authorize_platform(&state, &principal, platform_id, &headers).await?;
            let mut items: Vec<$item> = api_result(
                state
                    .platforms
                    .$method(platform_id)
                    .await.map(|items| items.into_iter().map(<$item>::from).collect::<Vec<_>>())
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
    ($(#[$name_attr:meta])* $name:ident, $method:ident, $item:ident) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path((platform_id, resource_id)) =
                api_result(path.map_err(invalid_path), &headers)?;
            api_result(validate_docker_resource_id(&resource_id), &headers)?;
            let capabilities =
                authorize_platform(&state, &principal, platform_id, &headers).await?;
            let mut item = required(
                state
                    .platforms
                    .$method(platform_id, &resource_id)
                    .await.map(|item| item.map(<$item>::from))
                    .map_err(platform_error),
                &headers,
            )?;
            item.capabilities = Some(capabilities);
            Ok(no_store(Json(item).into_response()))
        }
    };
}

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/nodes",
    operation_id = "listSwarmNodes",
    tag = "Platforms",
    summary = "List Swarm Nodes",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmNodeView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_nodes,
    list_swarm_nodes,
    SwarmNodeView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}",
    operation_id = "getSwarmNode",
    tag = "Platforms",
    summary = "Get a Swarm Node",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmNodeView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("nodeId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_node,
    get_swarm_node,
    SwarmNodeView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services",
    operation_id = "listSwarmServices",
    tag = "Platforms",
    summary = "List Swarm Services",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmServiceView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_services,
    list_swarm_services,
    SwarmServiceView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}",
    operation_id = "getSwarmService",
    tag = "Platforms",
    summary = "Get a Swarm Service",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_service,
    get_swarm_service,
    SwarmServiceView
);

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks",
    operation_id = "listSwarmTasks",
    tag = "Platforms",
    summary = "List Swarm Tasks",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmTaskView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("limit" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(50)))), ("serviceId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_swarm_tasks(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    filters: Result<Query<SwarmTaskFilters>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(platform_id) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(filters) = api_result(filters.map_err(invalid_query), &headers)?;
    api_result(validate_swarm_task_filters(&filters), &headers)?;
    let capabilities = authorize_platform(&state, &principal, platform_id, &headers).await?;
    let mut items = api_result(
        state
            .platforms
            .list_swarm_tasks(platform_id, filters.service_id.as_deref(), filters.limit)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::SwarmTaskView::from)
                    .collect::<Vec<_>>()
            })
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

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}",
    operation_id = "getSwarmTask",
    tag = "Platforms",
    summary = "Get a Swarm Task",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmTaskView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_task,
    get_swarm_task,
    SwarmTaskView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/networks",
    operation_id = "listSwarmNetworks",
    tag = "Platforms",
    summary = "List Swarm Networks",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmNetworkView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_networks,
    list_swarm_networks,
    SwarmNetworkView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/networks/{resourceId}",
    operation_id = "getSwarmNetwork",
    tag = "Platforms",
    summary = "Get a Swarm Network",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmNetworkView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_network,
    get_swarm_network,
    SwarmNetworkView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/configs",
    operation_id = "listSwarmConfigs",
    tag = "Platforms",
    summary = "List Swarm Configs",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmConfigView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_configs,
    list_swarm_configs,
    SwarmConfigView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/configs/{resourceId}",
    operation_id = "getSwarmConfig",
    tag = "Platforms",
    summary = "Get a Swarm Config",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmConfigView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_config,
    get_swarm_config,
    SwarmConfigView
);

swarm_list_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/secrets",
    operation_id = "listSwarmSecrets",
    tag = "Platforms",
    summary = "List Swarm Secrets",
    responses(
        (status = 200, description = "Success", body = SwarmItemsResponse<SwarmSecretView>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    list_swarm_secrets,
    list_swarm_secrets,
    SwarmSecretView
);

swarm_get_handler!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/secrets/{resourceId}",
    operation_id = "getSwarmSecret",
    tag = "Platforms",
    summary = "Get a Swarm Secret",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::SwarmSecretView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_swarm_secret,
    get_swarm_secret,
    SwarmSecretView
);

async fn effective_permissions(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    ids: &[Uuid],
    headers: &HeaderMap,
) -> HttpResult<std::collections::BTreeMap<Uuid, EffectivePlatformPermission>> {
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
    api_result(
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
) -> HttpResult<PlatformCapabilitiesView> {
    let permissions = effective_permissions(state, principal, &[id], headers).await?;
    let permission = permission_for(&permissions, id, principal.is_administrator());
    if permission.level_mask & ALL_LEVELS == 0 {
        return api_result(Err(ApiError::Forbidden), headers);
    }
    Ok(platform_capabilities(permission))
}

async fn authorize_platform_creation(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    headers: &HeaderMap,
) -> HttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    let permission = state
        .identity
        .global_permission(principal, ResourceType::Platform)
        .await
        .map_err(|error| crate::api::error::HttpError::from_parts(error, headers))?;
    if permission.is_some_and(|value| value.level.grants(PermissionLevel::Write)) {
        Ok(())
    } else {
        api_result(Err(ApiError::Forbidden), headers)
    }
}

async fn authorize_platform_level(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    required: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<PlatformCapabilitiesView> {
    let permissions = effective_permissions(state, principal, &[id], headers).await?;
    let permission = permission_for(&permissions, id, principal.is_administrator());
    if !resource_capabilities(permission).grants(required) {
        return api_result(Err(ApiError::Forbidden), headers);
    }
    Ok(platform_capabilities(permission))
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

async fn volume_capabilities(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    platform_id: Uuid,
    platform: PlatformCapabilitiesView,
    headers: &HeaderMap,
) -> HttpResult<VolumeCapabilitiesView> {
    let content = if principal.is_administrator() {
        None
    } else {
        Some(
            state
                .identity
                .permission_for_resource(principal, ResourceType::Volume, platform_id)
                .await
                .map_err(|error| crate::api::error::HttpError::from_parts(error, headers))?,
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
) -> HttpResult<()> {
    api_result(
        if values
            .into_iter()
            .flatten()
            .any(|value| value.len() > MAX_DOCKER_RESOURCE_ID_BYTES)
        {
            Err(ApiError::Validation(
                "A Docker resource filter is too long.".to_owned(),
            ))
        } else {
            Ok(())
        },
        headers,
    )
}

fn validate_swarm_task_filters(filters: &SwarmTaskFilters) -> Result<(), ApiError> {
    if !(1..=200).contains(&filters.limit) {
        return Err(ApiError::Validation(
            "The Swarm Task limit must be between 1 and 200.".to_owned(),
        ));
    }
    if filters
        .service_id
        .as_ref()
        .is_some_and(|service_id| service_id.len() > 64)
    {
        return Err(ApiError::Validation(
            "The Swarm Service ID is too long.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_network_input(input: &CreateRuntimeNetwork) -> Result<(), ApiError> {
    validate_name_identifier(&input.name, "Network")?;
    if !matches!(
        input.driver.as_str(),
        "bridge" | "macvlan" | "ipvlan" | "overlay"
    ) {
        return Err(ApiError::Validation(
            "Network driver must be bridge, macvlan, ipvlan, or overlay.".to_owned(),
        ));
    }
    if !matches!(input.scope.as_str(), "local" | "swarm") {
        return Err(ApiError::Validation(
            "Network scope must be local or swarm.".to_owned(),
        ));
    }
    if (input.scope == "swarm") != (input.driver == "overlay") {
        return Err(ApiError::Validation(
            "Overlay networks must use Swarm scope, and Swarm-scoped networks must use the overlay driver."
                .to_owned(),
        ));
    }
    if input.attachable == Some(true) && input.driver != "overlay" {
        return Err(ApiError::Validation(
            "Attachable is available only for overlay networks.".to_owned(),
        ));
    }
    if input.internal.is_some() && !matches!(input.driver.as_str(), "bridge" | "overlay") {
        return Err(ApiError::Validation(
            "Internal is available only for bridge and overlay networks.".to_owned(),
        ));
    }
    if input.enable_ipv4 == Some(false) && input.enable_ipv6 == Some(false) {
        return Err(ApiError::Validation(
            "At least one of EnableIPv4 or EnableIPv6 must be enabled.".to_owned(),
        ));
    }
    validate_map(&input.labels, "Network label")?;
    validate_map(&input.options, "Network option")?;
    if let Some(ipam) = &input.ipam {
        if ipam.driver.trim().len() < 3 || ipam.config.len() != 2 {
            return Err(ApiError::Validation(
                "IPAM requires a driver and exactly two configuration entries (IPv4 then IPv6)."
                    .to_owned(),
            ));
        }
        validate_map(&ipam.options, "IPAM option")?;
    }
    Ok(())
}

fn validate_volume_input(input: &CreateRuntimeVolume) -> Result<(), ApiError> {
    validate_name_identifier(&input.name, "Volume")?;
    if input.driver.trim().is_empty()
        || input.driver.len() > 255
        || input.driver.chars().any(char::is_whitespace)
    {
        return Err(ApiError::Validation(
            "Volume driver is required, cannot exceed 255 characters, and cannot contain whitespace."
                .to_owned(),
        ));
    }
    validate_map(&input.labels, "Volume label")?;
    validate_map(&input.options, "Volume option")
}

fn validate_name_identifier(value: &str, resource: &str) -> Result<(), ApiError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_DOCKER_RESOURCE_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err(ApiError::Validation(format!(
            "{resource} name contains unsupported characters."
        )));
    }
    Ok(())
}

fn validate_map(
    values: &std::collections::BTreeMap<String, String>,
    resource: &str,
) -> Result<(), ApiError> {
    if values.len() > 256
        || values
            .iter()
            .any(|(key, value)| key.trim().is_empty() || key.len() > 256 || value.len() > 4096)
    {
        return Err(ApiError::Validation(format!(
            "{resource} keys and values exceed the supported limits."
        )));
    }
    Ok(())
}

fn validate_resource_ids(
    values: &mut Vec<String>,
    maximum: usize,
    resource: &str,
) -> Result<(), ApiError> {
    if values.is_empty() || values.len() > maximum {
        return Err(ApiError::Validation(format!(
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
) -> Result<bool, ApiError> {
    state
        .platforms
        .platform_kind(platform_id)
        .await
        .map(|kind| kind == citadel_platforms::PlatformKind::DockerSwarm)
        .map_err(platform_error)
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

fn require_actor(principal: Option<Extension<ActorPrincipal>>) -> Result<ActorPrincipal, ApiError> {
    principal
        .map(|Extension(principal)| principal)
        .ok_or(ApiError::Unauthenticated)
}

fn required<T>(result: Result<Option<T>, ApiError>, headers: &HeaderMap) -> HttpResult<T> {
    api_result(result, headers)?
        .ok_or_else(|| crate::api::error::HttpError::from_parts(ApiError::NotFound, headers))
}

fn parse_tag_filters(query: Option<&str>) -> Result<Vec<String>, ApiError> {
    let mut tags = Vec::new();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if !key.eq_ignore_ascii_case("tags") {
            continue;
        }
        if tags.len() >= 100 {
            return Err(ApiError::Validation(
                "At most 100 Platform tags may be filtered at once.".to_owned(),
            ));
        }
        let tag = value.trim();
        if !tag.is_empty()
            && !tags
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
        {
            tags.push(tag.to_owned());
        }
    }
    Ok(tags)
}

fn validate_docker_resource_id(value: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() || value.len() > MAX_DOCKER_RESOURCE_ID_BYTES {
        return Err(ApiError::Validation(
            "The Docker resource ID is invalid.".to_owned(),
        ));
    }
    Ok(())
}

fn platform_error(error: AuthorizedReadError) -> ApiError {
    match error {
        AuthorizedReadError::NotFound => ApiError::NotFound,
        AuthorizedReadError::Conflict(message) => ApiError::Conflict(message),
        error @ AuthorizedReadError::Storage(_) => ApiError::internal(error),
    }
}

fn platform_registration_error(error: PlatformRegistrationError) -> ApiError {
    match error {
        PlatformRegistrationError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        PlatformRegistrationError::Conflict(message) => ApiError::Conflict(message),
        source @ PlatformRegistrationError::Storage(_) => ApiError::internal(source),
        source @ PlatformRegistrationError::Runtime(_) => ApiError::internal(source),
    }
}

fn platform_metadata_error(error: citadel_platforms::PlatformMetadataError) -> ApiError {
    match error {
        citadel_platforms::PlatformMetadataError::NotFound => ApiError::NotFound,
        source @ citadel_platforms::PlatformMetadataError::Storage(_) => ApiError::internal(source),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/inspect",
    operation_id = "inspectManagedSwarmService",
    tag = "SwarmServices",
    summary = "Inspect the deployed managed Service",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ServiceInspectionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn inspect_managed_service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .require_resource::<citadel_swarm_services::permissions::InspectSwarmService>(
                    &principal, id,
                )
                .await,
            &headers,
        )?;
    }
    let store = &state.services;
    let service = api_result(
        store
            .get_authorized(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(crate::api::routes::swarm_services::service_error),
        &headers,
    )?;
    let docker_id = api_result(
        service
            .docker_service_id
            .as_ref()
            .ok_or_else(|| ApiError::Conflict("The Service has not been applied.".into())),
        &headers,
    )?;
    authorize_platform(&state, &principal, service.platform_id, &headers).await?;
    swarm_platform(&state, service.platform_id, &headers).await?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let runtime = match state.runtime.swarm(service.platform_id, &cancel).await {
        Ok(runtime) => runtime,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let client = &runtime;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.inspect_service(docker_id, &cancel),
    )
    .await;
    let service = match result {
        Ok(Ok(service)) if service.id == *docker_id => service,
        Ok(Err(error)) => return Ok(runtime_error_response(error, &headers)),
        _ => {
            return api_result(
                Err(ApiError::External(
                    "Service inspection failed or timed out.".into(),
                )),
                &headers,
            );
        }
    };
    Ok(no_store(
        Json(inspect_service_view(service)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/data",
    operation_id = "getContainersData",
    tag = "Stacks",
    summary = "Get Stack runtime containers",
    responses(
        (status = 200, description = "Success", body = ContainerRuntimeListView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn stack_data(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(stack_id) = api_result(path.map_err(invalid_path), &headers)?;
    if stack_id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Invalid Stack id".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::ReadStack>(&principal, stack_id)
                .await,
            &headers,
        )?;
    }
    let containers = api_result(
        state
            .platforms
            .list_stack_containers(stack_id)
            .await
            .map(|value| {
                value
                    .into_iter()
                    .map(crate::api::resources::platforms::views::ContainerView::from)
                    .collect::<Vec<_>>()
            })
            .map_err(platform_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ContainerRuntimeListView {
            containers: api_result(
                containers
                    .iter()
                    .map(ContainerView::runtime_data)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(ApiError::internal),
                &headers,
            )?,
        })
        .into_response(),
    ))
}

#[derive(Clone, Copy)]
enum ReadKind {
    Inspect,
    Info,
    Data,
}

macro_rules! container_reader {
    ($(#[$name_attr:meta])* $name:ident, $kind:ident) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<String>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_container(State(state), principal, path, headers, ReadKind::$kind).await
        }
    };
}

container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/inspect",
    operation_id = "inspectContainer",
    tag = "Containers",
    summary = "Inspect a Container with sensitive environment values redacted",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_container,
    Inspect
);

container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/info",
    operation_id = "getContainerInfo",
    tag = "Containers",
    summary = "getContainerInfo",
    responses(
        (status = 200, description = "Success", body = ContainerSummaryView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    info,
    Info
);

container_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/data",
    operation_id = "getContainerData",
    tag = "Containers",
    summary = "getContainerData",
    responses(
        (status = 200, description = "Success", body = ContainerRuntimeView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    data,
    Data
);

async fn read_container(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    kind: ReadKind,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(reference) = api_result(path.map_err(invalid_path), &headers)?;
    if !valid_container_reference(&reference) {
        return api_result(
            Err(ApiError::Validation("Must be a valid container id".into())),
            &headers,
        );
    }
    let store = &state.statistics;
    let target = match store.find_container(&reference).await {
        Ok(target) => required(Ok(target), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let capabilities = authorize_platform(&state, &principal, target.platform_id, &headers).await?;
    if matches!(kind, ReadKind::Inspect) && !capabilities.can_inspect {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    let container = required(
        state
            .platforms
            .get_container(target.id)
            .await
            .map(|value| value.map(crate::api::resources::platforms::views::ContainerView::from))
            .map_err(platform_error),
        &headers,
    )?;
    if container.platform_id != target.platform_id || container.container_id != target.docker_id {
        return Ok(conflict_response(
            "Container identity changed. Refresh inventory before inspecting.".into(),
            &headers,
        ));
    }
    if matches!(kind, ReadKind::Data) {
        let mut data = api_result(
            container.runtime_data().map_err(ApiError::internal),
            &headers,
        )?;
        data.capabilities = Some(capabilities);
        data.container_stat = None;
        return Ok(no_store(Json(data).into_response()));
    }
    inspect_target(&state, container, &headers, kind, Some(capabilities)).await
}

macro_rules! deployment_reader {
    ($(#[$name_attr:meta])* $name:ident, $kind:ident) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_deployment(state, principal, path, headers, ReadKind::$kind).await
        }
    };
}

deployment_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/deployments/{id}/inspect",
    operation_id = "inspectDeployment",
    tag = "Deployments",
    summary = "inspectDeployment",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_deployment,
    Inspect
);

deployment_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/deployments/{id}/info",
    operation_id = "getDeploymentContainerInfo",
    tag = "Deployments",
    summary = "getDeploymentContainerInfo",
    responses(
        (status = 200, description = "Success", body = ContainerSummaryView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    deployment_info,
    Info
);

async fn read_deployment(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    kind: ReadKind,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Invalid Deployment id".into())),
            &headers,
        );
    }
    use citadel_deployments::permissions::{InspectDeployment, ReadDeployment};
    let authorization = if matches!(kind, ReadKind::Inspect) {
        state
            .identity
            .require_resource::<InspectDeployment>(&principal, id)
            .await
    } else {
        state
            .identity
            .require_resource::<ReadDeployment>(&principal, id)
            .await
    };
    api_result(authorization, &headers)?;
    let container = api_result(
        state
            .platforms
            .deployment_container(id)
            .await
            .map(ContainerView::from)
            .map_err(platform_error),
        &headers,
    )?;
    inspect_target(&state, container, &headers, kind, None).await
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/containers/{containerId}/inspect",
    operation_id = "inspectStackContainer",
    tag = "Stacks",
    summary = "Inspect a Stack Container with sensitive environment values redacted",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path), ("containerId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn inspect_stack(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((stack_id, reference)) = api_result(path.map_err(invalid_path), &headers)?;
    if stack_id.is_nil() || !valid_container_reference(&reference) {
        return api_result(
            Err(ApiError::Validation("Invalid Stack or container id".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::InspectStack>(&principal, stack_id)
                .await,
            &headers,
        )?;
    }
    let container = api_result(
        state
            .platforms
            .stack_container(stack_id, &reference)
            .await
            .map(ContainerView::from)
            .map_err(platform_error),
        &headers,
    )?;
    inspect_target(&state, container, &headers, ReadKind::Inspect, None).await
}

async fn inspect_target(
    state: &PlatformsHttpState,
    container: ContainerView,
    headers: &HeaderMap,
    kind: ReadKind,
    capabilities: Option<PlatformCapabilitiesView>,
) -> HttpResult {
    if container.projection_stale_since.is_some() {
        return Ok(conflict_response(
            "Container inventory is stale. Refresh before inspecting.".into(),
            headers,
        ));
    }
    let platform = required(
        state
            .platforms
            .get_platform(container.platform_id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        headers,
    )?;
    if platform.status != citadel_primitives::PlatformStatus::Online {
        return Ok(conflict_response(
            "Platform is disconnected or unavailable.".into(),
            headers,
        ));
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        state
            .runtime
            .container_inspection(
                container.platform_id,
                container.docker_node_id.as_deref(),
                &cancel,
            )
            .await?
            .inspection(&container.container_id, &cancel)
            .await
    })
    .await
    .unwrap_or_else(|_| {
        Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "Container inspection timed out.",
            false,
        ))
    });
    match result {
        Ok(inspection) => {
            if matches!(kind, ReadKind::Info) {
                Ok(no_store(
                    Json(api_result(
                        ContainerSummaryView::from_inspection(
                            &container,
                            &platform.name,
                            &inspection,
                            capabilities,
                        )
                        .map_err(ApiError::internal),
                        headers,
                    )?)
                    .into_response(),
                ))
            } else {
                let inspection = api_result(
                    serde_json::from_value::<
                        crate::api::resources::platforms::descriptor_views::ContainerInspectionView,
                    >(inspection)
                    .map_err(ApiError::internal),
                    headers,
                )?;
                Ok(no_store(Json(inspection).into_response()))
            }
        }
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}

macro_rules! action_handler {
    ($(#[$name_attr:meta])* $name:ident, $action:ident) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            body: Result<Json<Vec<String>>, JsonRejection>,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Json(ids) = api_result(body.map_err(invalid_json), &headers)?;
            execute(state, principal, ids, ContainerAction::$action, &headers).await
        }
    };
}

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/start",
    operation_id = "startContainers",
    tag = "Containers",
    summary = "Start Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    start,
    Start
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/stop",
    operation_id = "stopContainers",
    tag = "Containers",
    summary = "Stop Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    stop,
    Stop
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/restart",
    operation_id = "restartContainers",
    tag = "Containers",
    summary = "Restart Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    restart,
    Restart
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/pause",
    operation_id = "pauseContainers",
    tag = "Containers",
    summary = "Pause Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    pause,
    Pause
);

action_handler!(
    #[utoipa::path(
    patch,
    path = "/api/v1/containers/unpause",
    operation_id = "unpauseContainers",
    tag = "Containers",
    summary = "Unpause Containers",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    unpause,
    Unpause
);

macro_rules! deployment_action_handler {
    ($(#[$name_attr:meta])* $name:ident, $action:ident) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            body: Result<Json<Vec<Uuid>>, JsonRejection>,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Json(ids) = api_result(body.map_err(invalid_json), &headers)?;
            match state
                .containers
                .execute_deployments(
                    principal.actor_id,
                    principal.is_administrator(),
                    ids,
                    ContainerAction::$action,
                )
                .await
            {
                Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
                Err(error) => Ok(runtime_error_response(error, &headers)),
            }
        }
    };
}

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/start",
    operation_id = "startDeployments",
    tag = "Deployments",
    summary = "Start Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    start_deployments,
    Start
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/stop",
    operation_id = "stopDeployments",
    tag = "Deployments",
    summary = "Stop Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    stop_deployments,
    Stop
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/restart",
    operation_id = "restartDeployments",
    tag = "Deployments",
    summary = "Restart Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    restart_deployments,
    Restart
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/pause",
    operation_id = "pauseDeployments",
    tag = "Deployments",
    summary = "Pause Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    pause_deployments,
    Pause
);

deployment_action_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/deployments/resume",
    operation_id = "resumeDeployments",
    tag = "Deployments",
    summary = "Resume Deployments",
    request_body = Vec<uuid::Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    resume_deployments,
    Unpause
);

#[utoipa::path(
    delete,
    path = "/api/v1/containers",
    operation_id = "deleteContainers",
    tag = "Containers",
    summary = "Delete Containers",
    request_body = DeleteInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_containers(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    body: Result<Json<DeleteInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    execute(
        state,
        principal,
        input.container_ids,
        ContainerAction::Delete(input.options.into()),
        &headers,
    )
    .await
}

async fn execute(
    state: PlatformsHttpState,
    principal: ActorPrincipal,
    ids: Vec<String>,
    action: ContainerAction,
    headers: &HeaderMap,
) -> HttpResult {
    match state
        .containers
        .execute(
            principal.actor_id,
            principal.is_administrator(),
            ids,
            action,
        )
        .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/platforms",
    operation_id = "deletePlatforms",
    tag = "Platforms",
    summary = "Delete Platform registrations",
    request_body = DeletePlatformsInput,
    responses(
        (status = 200, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_platforms(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeletePlatformsInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_platforms::deletion::DeletePlatformsInput = input.into();
    api_result(input.validate().map_err(deletion_error), &headers)?;
    let permissions = effective_permissions(&state, &principal, &input.ids, &headers).await?;
    if input.ids.iter().any(|id| {
        !resource_capabilities(permission_for(
            &permissions,
            *id,
            principal.is_administrator(),
        ))
        .can_execute
    }) {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    api_result(
        state
            .deletions
            .delete(principal.actor_id, &input.ids, |id| {
                if let Some(realtime) = &state.realtime {
                    realtime.publish_resource_change("Platform", id, "deleted");
                }
            })
            .await
            .map_err(deletion_error),
        &headers,
    )?;
    Ok(no_store(StatusCode::OK.into_response()))
}

fn deletion_error(error: PlatformDeletionError) -> ApiError {
    match error {
        PlatformDeletionError::Validation => ApiError::Validation(error.to_string()),
        PlatformDeletionError::NotFound => ApiError::NotFound,
        PlatformDeletionError::InUse => ApiError::Conflict(error.to_string()),
        source @ PlatformDeletionError::Storage(_) => ApiError::internal(source),
    }
}

macro_rules! setup_handler {
    ($(#[$name_attr:meta])* $name:ident,$kind:ident) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            Extension(edge): Extension<EdgeHttpContext>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            node_agent_operation(
                state,
                principal,
                path,
                headers,
                Some((
                    SetupKind::$kind,
                    SetupOptions {
                        policy: edge.node_agent_policy,
                        core_url: edge.core_url,
                        image: edge.agent_image,
                        ca_bundle: edge.node_agent_ca_bundle,
                    },
                )),
            )
            .await
        }
    };
}

setup_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/node-agents/install",
    operation_id = "installSwarmNodeAgents",
    tag = "Platforms",
    summary = "Install Docker Swarm node agents",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    install_node_agents,
    Install
);

setup_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/node-agents/repair",
    operation_id = "repairSwarmNodeAgents",
    tag = "Platforms",
    summary = "Repair Docker Swarm node-agent coverage",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    repair_node_agents,
    Repair
);

setup_handler!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/node-agents/upgrade",
    operation_id = "upgradeSwarmNodeAgents",
    tag = "Platforms",
    summary = "Upgrade Docker Swarm node agents",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    upgrade_node_agents,
    Upgrade
);

#[utoipa::path(
    delete,
    path = "/api/v1/platforms/{id}/node-agents",
    operation_id = "removeSwarmNodeAgents",
    tag = "Platforms",
    summary = "Remove Docker Swarm node agents",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::NodeAgentProgressSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove_node_agents(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    node_agent_operation(state, principal, path, headers, None).await
}

async fn node_agent_operation(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    setup: Option<(SetupKind, SetupOptions)>,
) -> HttpResult {
    static OPERATIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Execute, &headers)
            .await?;
    if !capabilities.can_manage_node_agents {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    let platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != citadel_platforms::PlatformKind::DockerSwarm {
        return api_result(
            Err(ApiError::Validation(
                "Node agents require a Docker Swarm platform.".into(),
            )),
            &headers,
        );
    }
    if let Some((_, options)) = &setup {
        api_result(
            options
                .validate()
                .map_err(|e| ApiError::Validation(e.message)),
            &headers,
        )?;
    }
    let permit = match OPERATIONS.try_acquire() {
        Ok(permit) => permit,
        Err(_) => {
            return Ok(runtime_error_response(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::ResourceExhausted,
                    "Too many node-agent operations are running.",
                    true,
                ),
                &headers,
            ));
        }
    };
    let Some(task) = state.tasks.reserve() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Platform operations are shutting down.",
                false,
            ),
            &headers,
        ));
    };
    let cancellation = state.tasks.cancellation();
    let notify_state = state.clone();
    let store = state.node_agent_store.clone();
    let runtime = state.node_agent_runtime.clone();
    let changed: Arc<dyn Fn(Uuid) + Send + Sync> = Arc::new(move |id| {
        publish_runtime_change(
            &notify_state,
            id,
            "nodeAgentCoverage",
            "update",
            &id.to_string(),
        );
    });
    let guard = cancellation.clone().drop_guard();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
    task.spawn(async move {
        let _permit = permit;
        if let Some((kind, options)) = setup {
            NodeAgentSetupService {
                store,
                runtime,
                changed,
            }
            .run(principal.actor_id, id, kind, options, sender, cancellation)
            .await;
        } else {
            citadel_platforms::node_agents::lifecycle::NodeAgentRemovalService {
                store,
                runtime,
                changed,
            }
            .remove(principal.actor_id, id, sender, cancellation)
            .await;
        }
    });
    let stream = async_stream::stream! {
        let _guard = guard;
        yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first=true;
        while let Some(item)=receiver.recv().await {
            if !first {yield Ok(bytes::Bytes::from_static(b","));} first=false;
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Node-agent progress serializes")));
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    let mut response = no_store(axum::body::Body::from_stream(stream).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Ok(response)
}

async fn initialize_swarm(
    state: &PlatformsHttpState,
    platform: &citadel_platforms::PlatformDetails,
) -> Result<bool, RuntimeCapabilityError> {
    citadel_platforms::swarm_mutations::initialize_inventory(
        &state.platforms,
        state.projections.as_ref(),
        state.runtime.as_ref(),
        platform,
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/node-agent-coverage",
    operation_id = "getSwarmNodeAgentCoverage",
    tag = "Platforms",
    summary = "Get Docker Swarm node-agent coverage",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::SwarmNodeAgentCoverageSchema, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn node_coverage(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != citadel_platforms::PlatformKind::DockerSwarm {
        return api_result(
            Err(ApiError::Validation(
                "Node Agent coverage is available only for Docker Swarm platforms.".into(),
            )),
            &headers,
        );
    }
    let mut coverage = api_result(
        state
            .node_agent_coverage
            .read(&platform)
            .await
            .map_err(ApiError::internal),
        &headers,
    )?;
    if coverage.total_nodes == 0 {
        let initialized = initialize_swarm(&state, &platform).await;
        let changed = match initialized {
            Ok(changed) => changed,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
        if changed {
            publish_runtime_change(&state, id, "platform", "update", &id.to_string());
        }
        platform = required(
            state
                .platforms
                .get_platform(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        coverage = api_result(
            state
                .node_agent_coverage
                .read(&platform)
                .await
                .map_err(ApiError::internal),
            &headers,
        )?;
    }
    coverage.can_manage_node_agents = capabilities.can_manage_node_agents;
    Ok(no_store(Json(coverage).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/edge/enrollments",
    operation_id = "createEdgeAgentEnrollment",
    tag = "Platforms",
    summary = "Create an Edge Agent enrollment token",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::EdgeEnrollmentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn enroll(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let enrollment = api_result(
        edge.enrollment(EdgeTarget::platform(id), principal.actor_id.value())
            .await,
        &headers,
    )?;
    Ok(no_store(Json(enrollment).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/edge/status",
    operation_id = "getEdgeAgentStatus",
    tag = "Platforms",
    summary = "Get Edge Agent connection status",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::EdgeStatusView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn status(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let status = api_result(
        edge.store
            .status(&EdgeTarget::platform(id))
            .await
            .map_err(EdgeHttpContext::error),
        &headers,
    )?;
    Ok(no_store(
        Json(crate::api::resources::platforms::operation_views::EdgeStatusView::from(status))
            .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/edge/revoke",
    operation_id = "revokeEdgeAgent",
    tag = "Platforms",
    summary = "Revoke an Edge Agent binding",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn revoke(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let target = EdgeTarget::platform(id);
    api_result(
        citadel_platforms::edge_management::revoke(
            edge.store.as_ref(),
            edge.registry.as_ref(),
            &target,
        )
        .await
        .map_err(EdgeHttpContext::error),
        &headers,
    )?;
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

static PULL_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[utoipa::path(
    post,
    path = "/api/v1/images/pull",
    operation_id = "pullImage",
    tag = "Images",
    summary = "Pull a Docker image with progress",
    request_body = PullImageInput,
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::platforms::PullImageStreamItemSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn pull(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<PullImageInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::image_pull::PullImageInput = input.into();
    if input.platform_id.is_nil()
        || input.registry_id.is_nil()
        || input.image_tag.trim().is_empty()
        || input.image_tag.len() > 2048
        || input.image_tag.chars().any(char::is_control)
    {
        return api_result(
            Err(ApiError::Validation(
                "A Platform, Registry and valid image reference are required.".into(),
            )),
            &headers,
        );
    }
    let capabilities = authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    if !capabilities.can_pull {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    api_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Registry,
                input.registry_id,
                PermissionLevel::Read,
                None,
            )
            .await,
        &headers,
    )?;
    let permit = api_result(
        PULL_SLOTS
            .try_acquire()
            .map_err(|_| ApiError::Conflict("Image pull capacity is busy.".into())),
        &headers,
    )?;
    let (image, auth) = match state
        .image_store
        .prepare_pull(input.registry_id, &input.image_tag)
        .await
    {
        Ok(value) => value,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let Some(task) = state.tasks.reserve() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Platform operations are shutting down.",
                false,
            ),
            &headers,
        ));
    };
    let cancel = state.tasks.cancellation();
    let guard = cancel.clone().drop_guard();
    let (sender, receiver) = tokio::sync::mpsc::channel(16);
    let (completed, completion) = tokio::sync::oneshot::channel();
    task.spawn(async move {
        let _permit = permit;
        let result=tokio::time::timeout(std::time::Duration::from_secs(600),async {
            let runtime = state.runtime.image_mutations(input.platform_id, &cancel).await?;
            citadel_platforms::image_mutations::pull_and_persist(
                state.image_store.as_ref(), runtime.as_ref(), &input, &image, auth.as_deref().map(String::as_str), &sender, &cancel,
                |kind, operation, id| publish_runtime_change(&state, input.platform_id, kind, operation, id),
            ).await?;
            Ok::<_,RuntimeCapabilityError>(())
        }).await;
        let error = match result {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(match e.kind {
                RuntimeErrorKind::Remote | RuntimeErrorKind::Authentication => "Image pull failed. Check the image reference and Registry credentials.".into(),
                RuntimeErrorKind::Unavailable => "Image pull could not complete or be persisted. Refresh inventory before retrying.".into(),
                _ => e.message,
            }),
            Err(_) => Some("Image pull timed out. Refresh inventory before retrying.".into()),
        };
        let terminal = error.map(|message| PullImageStreamItem {
            error_message: Some(message.clone()),
            error: Some(ImagePullError {
                code: Some(500),
                message: Some(message),
            }),
            ..Default::default()
        });
        // Completion must not wait for room in a stalled client's progress queue.
        // The body drains queued progress before delivering this final error.
        let _ = completed.send(terminal);
    });
    let mut response = no_store(progress_body(receiver, completion, guard).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Ok(response)
}

fn progress_body(
    mut receiver: tokio::sync::mpsc::Receiver<PullImageStreamItem>,
    completion: tokio::sync::oneshot::Receiver<Option<PullImageStreamItem>>,
    guard: tokio_util::sync::DropGuard,
) -> axum::body::Body {
    let stream = async_stream::stream! {
        let _guard=guard;
        yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first=true;
        while let Some(item)=receiver.recv().await {if !first{yield Ok(bytes::Bytes::from_static(b","));}first=false;yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Image pull progress serializes")));}
        if let Ok(Some(item)) = completion.await {
            if !first { yield Ok(bytes::Bytes::from_static(b",")); }
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Image pull error serializes")));
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    axum::body::Body::from_stream(stream)
}

static IMAGE_DELETE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[utoipa::path(
    delete,
    path = "/api/v1/images",
    operation_id = "deleteImages",
    tag = "Images",
    summary = "Delete Images",
    request_body = DeleteImagesInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::DeleteImagesView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_images(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteImagesInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(mut input) = api_result(input.map_err(invalid_json), &headers)?;
    api_result(
        validate_resource_ids(&mut input.ids, 100, "Image"),
        &headers,
    )?;
    if input.platform_id.is_nil()
        || input.ids.iter().any(|id| {
            let hash = id.strip_prefix("sha256:").unwrap_or(id);
            hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit())
        })
    {
        return api_result(
            Err(ApiError::Validation(
                "A Platform and full Docker Image IDs are required.".into(),
            )),
            &headers,
        );
    }
    for id in &mut input.ids {
        *id = format!(
            "sha256:{}",
            id.strip_prefix("sha256:")
                .unwrap_or(id)
                .to_ascii_lowercase()
        );
    }
    input.ids.sort();
    input.ids.dedup();
    authorize_platform_level(
        &state,
        &principal,
        input.platform_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    if api_result(platform_is_swarm(&state, input.platform_id).await, &headers)? {
        return Ok(conflict_response(
            "Node-local Image deletion requires an explicit Node target and is not available."
                .into(),
            &headers,
        ));
    }
    // Keep bounded cleanup running if the browser disconnects after Docker accepts deletion.
    let Ok(permit) = IMAGE_DELETE_SLOTS.try_acquire() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Image deletion capacity is busy. Try again later.",
                false,
            ),
            &headers,
        ));
    };
    let Some(task) = state.tasks.reserve() else {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Platform operations are shutting down.",
                false,
            ),
            &headers,
        ));
    };
    let (completed, completion) = tokio::sync::oneshot::channel();
    task.spawn(async move {
        let result = async move {
            let _permit = permit;
            // This token belongs to the tracked operation, not the HTTP body:
            // accepted deletions still need reconciliation after disconnection.
            let cancel = state.tasks.cancellation();
            let _guard = cancel.clone().drop_guard();
            let runtime = tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Cancelled, "Image deletion interrupted by shutdown.", false)),
                runtime = state.runtime.image_mutations(input.platform_id, &cancel) => runtime?,
            };
            let result = citadel_platforms::image_mutations::delete_images(
                state.image_store.as_ref(),
                input.platform_id,
                &input.ids,
                input.force,
                input.no_prune,
                runtime.as_ref(),
                &cancel,
            )
            .await;
            for id in &input.ids {
                publish_runtime_change(&state, input.platform_id, "image", "update", id);
            }
            result
        }
        .await;
        if let Err(Err(error)) = completed.send(result) {
            tracing::error!(%error, "Image deletion failed after the caller disconnected");
        }
    });
    let result = api_result(completion.await.map_err(ApiError::internal), &headers)?;
    match result {
        Ok(items) => Ok(no_store(
            Json(
                crate::api::resources::platforms::operation_views::DeleteImagesView {
                    items: items
                        .into_iter()
                        .map(|result| {
                            crate::api::resources::platforms::operation_views::DeleteImageView {
                                result,
                            }
                        })
                        .collect(),
                },
            )
            .into_response(),
        )),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/images/{platformId}/{imageId}/_ports",
    operation_id = "getExposedPorts",
    tag = "Images",
    summary = "Read exposed ports using the Citadel Image ID",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ExposedPortsView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("imageId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn exposed_ports(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, Uuid)>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, image_id)) = api_result(path.map_err(invalid_path), &headers)?;
    if platform_id.is_nil() || image_id.is_nil() {
        api_result::<()>(
            Err(ApiError::Validation(
                "Platform and Image IDs must not be empty.".into(),
            )),
            &headers,
        )?;
    }
    authorize_platform(&state, &principal, platform_id, &headers).await?;
    // The form passes a Citadel UUID, never a Docker content ID. Resolve it on
    // the requested Platform and retain node identity for projected Swarm images.
    let citadel_platforms::ImageIdentity {
        docker_image_id: docker_id,
        docker_node_id: node_id,
        is_stale: stale,
    } = api_result(
        state
            .platforms
            .image_identity(platform_id, image_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if stale {
        return Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Conflict,
                "The Image projection is stale.",
                false,
            ),
            &headers,
        ));
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let result = async {
        state
            .runtime
            .images(platform_id, node_id.as_deref(), &cancel)
            .await?
            .exposed_ports(&docker_id, &cancel)
            .await
    }
    .await;
    match result {
        Ok(ports) => Ok(no_store(
            Json(crate::api::resources::platforms::operation_views::ExposedPortsView { ports })
                .into_response(),
        )),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/images/{platformId}/{imageId}",
    operation_id = "inspectImage",
    tag = "Images",
    summary = "Inspect an Image on its owning Docker node",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::ImageInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("imageId" = String, Path), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn inspect_image(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    selector: Result<Query<DockerNodeSelector>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, image_id)) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(mut selector) = api_result(selector.map_err(invalid_query), &headers)?;
    api_result(validate_docker_resource_id(&image_id), &headers)?;
    if let Some(node) = &selector.docker_node_id {
        api_result(validate_docker_resource_id(node), &headers)?;
    }
    let capabilities =
        image_capabilities(authorize_platform(&state, &principal, platform_id, &headers).await?);
    if selector.docker_node_id.is_none() {
        let platform = required(
            state
                .platforms
                .get_platform(platform_id)
                .await
                .map_err(platform_error)
                .and_then(|value| {
                    value
                        .map(PlatformView::try_from)
                        .transpose()
                        .map_err(ApiError::internal)
                }),
            &headers,
        )?;
        if platform.platform_type == "DockerSwarm" {
            selector.docker_node_id = platform
                .platform_descriptor
                .as_ref()
                .and_then(
                    crate::api::resources::platforms::descriptor_views::PlatformDescriptor::node_id,
                )
                .filter(|id| !id.is_empty())
                .map(str::to_owned);
            if selector.docker_node_id.is_none() {
                return Ok(runtime_error_response(
                    RuntimeCapabilityError::new(
                        RuntimeErrorKind::Unavailable,
                        "The connected Swarm Node identity is unavailable.",
                        false,
                    ),
                    &headers,
                ));
            }
        }
    }
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let image = async {
        state
            .runtime
            .images(
                platform_id,
                selector.docker_node_id.as_deref(),
                &cancellation,
            )
            .await?
            .inspect_image(&image_id, &cancellation)
            .await
    }
    .await;
    let mut image = match image {
        Ok(image) => image,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    if selector.docker_node_id.is_none() {
        let registry_id: Option<Uuid> = api_result(
            state
                .platforms
                .image_registry_id(platform_id, &image.id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        if let Some(id) = registry_id {
            let registry = api_result(
                state
                    .registries
                    .get_registry(id)
                    .await
                    .map_err(ApiError::internal),
                &headers,
            )?;
            image.registry = Some(api_result(
                serde_json::to_value(registry).map_err(ApiError::internal),
                &headers,
            )?);
        }
    }
    image.docker_node_id = selector.docker_node_id;
    let image = crate::api::resources::platforms::views::ImageInspectionView {
        image,
        capabilities: Some(capabilities),
    };
    Ok(no_store(Json(image).into_response()))
}

fn tail(query: Result<Query<Tail>, QueryRejection>, headers: &HeaderMap) -> HttpResult<u16> {
    let Query(query) = api_result(query.map_err(invalid_query), headers)?;
    api_result(
        if (1..=200).contains(&query.tail) {
            Ok(query.tail)
        } else {
            Err(ApiError::Validation(
                "Tail must be between 1 and 200.".into(),
            ))
        },
        headers,
    )
}

async fn authorize_logs(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    kind: ResourceType,
    id: Uuid,
    headers: &HeaderMap,
) -> HttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    match kind {
        ResourceType::Stack => {
            return api_result(
                state
                    .identity
                    .require_resource::<citadel_stacks::permissions::ViewStackLogs>(principal, id)
                    .await,
                headers,
            );
        }
        ResourceType::SwarmService => {
            return api_result(
                state
                    .identity
                    .require_resource::<citadel_swarm_services::permissions::ViewSwarmServiceLogs>(
                        principal, id,
                    )
                    .await,
                headers,
            );
        }
        _ => {}
    }
    api_result(
        state
            .identity
            .authorize_resource(
                principal,
                kind,
                id,
                PermissionLevel::Read,
                Some(SpecificPermission::Logs),
            )
            .await,
        headers,
    )
}

async fn swarm_platform(
    state: &PlatformsHttpState,
    platform_id: Uuid,
    headers: &HeaderMap,
) -> HttpResult<()> {
    let platform = required(
        state
            .platforms
            .get_platform(platform_id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return api_result(
            Err(ApiError::Validation(
                "Platform must be Docker Swarm.".into(),
            )),
            headers,
        );
    }
    if platform.status != citadel_primitives::PlatformStatus::Online {
        return api_result(
            Err(ApiError::Conflict(
                "Platform is disconnected or unavailable.".into(),
            )),
            headers,
        );
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/logs",
    operation_id = "getSwarmServiceLogs",
    tag = "Platforms",
    summary = "Read bounded Service logs",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::LogSnapshotSchema, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("tail" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(100))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn service_logs(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Tail>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let tail = tail(query, &headers)?;
    api_result(validate_docker_resource_id(&id), &headers)?;
    authorize_logs(
        &state,
        &principal,
        ResourceType::Platform,
        platform,
        &headers,
    )
    .await?;
    read_service_logs(&state, platform, &id, tail, &headers).await
}

async fn read_service_logs(
    state: &PlatformsHttpState,
    platform: Uuid,
    id: &str,
    tail: u16,
    headers: &HeaderMap,
) -> HttpResult {
    swarm_platform(state, platform, headers).await?;
    required(
        state
            .platforms
            .get_swarm_service(platform, id)
            .await
            .map(|value| value.map(crate::api::resources::platforms::views::SwarmServiceView::from))
            .map_err(platform_error),
        headers,
    )?;
    read_logs(
        state,
        platform,
        None,
        LogResource::Service(id),
        tail,
        headers,
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/logs",
    operation_id = "getManagedSwarmServiceLogs",
    tag = "SwarmServices",
    summary = "Read managed Service logs",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::LogSnapshotSchema, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("id" = uuid::Uuid, Path), ("tail" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(100))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn managed_service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Tail>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let tail = tail(query, &headers)?;
    authorize_logs(&state, &principal, ResourceType::SwarmService, id, &headers).await?;
    let store = &state.services;
    let service = api_result(
        store
            .get_authorized(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(crate::api::routes::swarm_services::service_error),
        &headers,
    )?;
    let Some(docker_id) = service.docker_service_id.as_ref() else {
        return api_result(
            Err(ApiError::Conflict(
                "The Service has not been applied.".into(),
            )),
            &headers,
        );
    };
    let permissions =
        effective_permissions(&state, &principal, &[service.platform_id], &headers).await?;
    if permission_for(
        &permissions,
        service.platform_id,
        principal.is_administrator(),
    )
    .level_mask
        & ALL_LEVELS
        == 0
    {
        return api_result(Err(ApiError::NotFound), &headers);
    }
    read_service_logs(&state, service.platform_id, docker_id, tail, &headers).await
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/logs",
    operation_id = "getSwarmTaskLogs",
    tag = "Platforms",
    summary = "Read current Task logs on its owning node",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::LogSnapshotSchema, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("tail" = Option<i32>, Query, minimum = 1, maximum = 200, extensions(("x-citadel-default" = json!(100))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn task_logs(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Tail>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let tail = tail(query, &headers)?;
    api_result(validate_docker_resource_id(&id), &headers)?;
    authorize_logs(
        &state,
        &principal,
        ResourceType::Platform,
        platform,
        &headers,
    )
    .await?;
    swarm_platform(&state, platform, &headers).await?;
    let projection = required(
        state
            .platforms
            .get_swarm_task(platform, &id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _cancel_on_drop = cancel.clone().drop_guard();
    let live = async {
        state
            .runtime
            .tasks(platform, &cancel)
            .await?
            .inspect_task(&id, &cancel)
            .await
    }
    .await;
    let live = match live {
        Ok(live) => live,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let docker_id = match citadel_platforms::validate_running_task(&projection, &live) {
        Ok(id) => id,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    read_logs(
        &state,
        platform,
        Some(&live.node_id),
        LogResource::Container(docker_id),
        tail,
        &headers,
    )
    .await
}

async fn read_logs(
    state: &PlatformsHttpState,
    platform: Uuid,
    node: Option<&str>,
    resource: LogResource<'_>,
    tail: u16,
    headers: &HeaderMap,
) -> HttpResult {
    let cancel = CancellationToken::new();
    let _cancel_on_drop = cancel.clone().drop_guard();
    let result = async {
        state
            .runtime
            .logs(platform, node, &cancel)
            .await?
            .read_logs(resource, tail, &cancel)
            .await
    }
    .await;
    match result {
        Ok(value) => Ok(no_store(Json(value).into_response())),
        Err(error) => Ok(runtime_error_response(error, headers)),
    }
}

static PRUNE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{id}/prune",
    operation_id = "prunePlatform",
    tag = "Platforms",
    summary = "Prune unused Docker resources",
    request_body = PrunePlatformInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PrunePlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn prune(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PrunePlatformInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::prune::PrunePlatformInput = input.into();
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("A Platform ID is required.".into())),
            &headers,
        );
    }
    authorize_platform_level(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let _permit = api_result(
        PRUNE_SLOTS
            .try_acquire()
            .map_err(|_| ApiError::Conflict("Prune capacity is busy. Retry later.".into())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let operation = async {
        state
            .runtime
            .pruning(id, &cancel)
            .await?
            .prune(input.resource, &cancel)
            .await
    };
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(300), operation).await;
    // Partial failures can still delete resources. Invalidate inventory on every outcome.
    for resource in ["volume", "network", "image", "platform"] {
        publish_runtime_change(&state, id, resource, "update", &id.to_string());
    }
    match outcome {
        Ok(Ok(value)) => Ok(no_store(Json(value).into_response())),
        Ok(Err(error)) => Ok(runtime_error_response(error, &headers)),
        Err(_) => Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Timeout,
                "Prune timed out; some deletions may have completed.",
                false,
            ),
            &headers,
        )),
    }
}

#[derive(Clone)]
pub struct AgentSetupContext {
    pub signer: Arc<dyn citadel_platforms::management::AgentSigningPort>,
    pub image: String,
    pub requires_tls: bool,
}

impl AgentSetupContext {
    pub fn view(&self) -> crate::api::resources::platforms::views::AgentSetupView {
        crate::api::resources::platforms::views::AgentSetupView::new(
            self.signer.public_key_base64(),
            self.image.clone(),
            self.requires_tls,
        )
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/agent/setup/rotate-key",
    operation_id = "rotateAgentHubKey",
    tag = "Platforms",
    summary = "Rotate the Agent signing key",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::AgentSetupView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rotate_key(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    setup: Option<Extension<AgentSetupContext>>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    api_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Platform,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Extension(setup) = api_result(
        setup.ok_or_else(|| ApiError::Storage("Agent key storage is not configured.".into())),
        &headers,
    )?;
    api_result(
        setup
            .signer
            .rotate()
            .await
            .map_err(|e| ApiError::Storage(e.message)),
        &headers,
    )?;
    Ok(no_store(Json(setup.view()).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/rename",
    operation_id = "renamePlatform",
    tag = "Platforms",
    summary = "Rename a Platform",
    request_body = RenamePlatformInput,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenamePlatformInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::management::RenamePlatformInput = input.into();
    update(
        state,
        principal,
        input.id,
        serde_json::json!({"name":input.name}),
        true,
        headers,
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{id}",
    operation_id = "updatePlatform",
    tag = "Platforms",
    summary = "Update a Platform",
    request_body(content(
        (crate::api::resources::platforms::patch::PlatformPatch = "application/merge-patch+json"),
        (crate::api::resources::platforms::patch::PlatformPatch = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::views::PlatformView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn patch(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<serde_json::Value>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    update(state, principal, id, input, false, headers).await
}

async fn update(
    state: PlatformsHttpState,
    principal: ActorPrincipal,
    id: Uuid,
    input: serde_json::Value,
    rename: bool,
    headers: HeaderMap,
) -> HttpResult {
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("A Platform ID is required.".into())),
            &headers,
        );
    }
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let current = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    // Decode partial fields only after scoped authorization, preserving PATCH omission/null semantics.
    let patch: crate::api::resources::platforms::patch::PlatformPatch = api_result(
        serde_path_to_error::deserialize(input).map_err(|error| {
            crate::request_validation::validation_error(format!("Invalid Platform patch: {error}"))
        }),
        &headers,
    )?;
    let input = api_result(
        serde_json::to_value(patch).map_err(ApiError::internal),
        &headers,
    )?;
    let input = api_result(
        patch_input(&current, &input).map_err(platform_registration_error),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    match state
        .management
        .update(&current, &input, principal.actor_id, rename, &cancel)
        .await
    {
        Ok(()) => {}
        Err(PlatformRegistrationError::Runtime(error)) => {
            return Ok(runtime_error_response(error, &headers));
        }
        Err(error) => return api_result(Err(platform_registration_error(error)), &headers),
    }
    let updated = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let mut updated = api_result(
        PlatformView::try_from(updated).map_err(ApiError::internal),
        &headers,
    )?;
    updated.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(Json(updated).into_response()))
}

static BROWSE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(8);

macro_rules! browse_one {
    ($(#[$handler_attr:meta])* $handler:ident,$kind:ident) => {
        $(#[$handler_attr])*
        async fn $handler(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<String>, PathRejection>,
            browser: Option<Extension<Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>>>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path(registry) = api_result(path.map_err(invalid_path), &headers)?;
            browse(
                &state,
                &principal,
                &registry,
                None,
                RegistryBrowseKind::$kind,
                browser,
                &headers,
            )
            .await
        }
    };
}

macro_rules! browse_two {
    ($(#[$handler_attr:meta])* $handler:ident,$kind:ident) => {
        $(#[$handler_attr])*
        async fn $handler(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(String, String)>, PathRejection>,
            browser: Option<Extension<Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>>>,
            headers: HeaderMap,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Path((registry, name)) = api_result(path.map_err(invalid_path), &headers)?;
            browse(
                &state,
                &principal,
                &registry,
                Some(&name),
                RegistryBrowseKind::$kind,
                browser,
                &headers,
            )
            .await
        }
    };
}

browse_one!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/{registryName}/repositories",
    operation_id = "getExternalRepositories",
    tag = "Images",
    summary = "List Registry repositories",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::ExternalRepositorySchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    repositories,
    Repositories
);

browse_one!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/dockerhub/{registryName}/repositories",
    operation_id = "getDockerHubRepositories",
    tag = "Images",
    summary = "List Docker Hub repositories",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::DockerHubRepositoryInfoSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    docker_repositories,
    DockerHubRepositories
);

browse_two!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/dockerhub/{registryName}/{repositoryName}/tags",
    operation_id = "getDockerHubRepositoryTags",
    tag = "Images",
    summary = "List Docker Hub repository tags",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::DockerHubTagSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path), ("repositoryName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    docker_tags,
    DockerHubTags
);

browse_two!(
    #[utoipa::path(
    get,
    path = "/api/v1/images/ghcr/{registryName}/{packageName}/versions",
    operation_id = "getGhcrPackageVersions",
    tag = "Images",
    summary = "List GitHub package versions",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::registries::GithubPackageVersionSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("registryName" = String, Path), ("packageName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    github_versions,
    GithubVersions
);

async fn browse(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    registry: &str,
    name: Option<&str>,
    kind: RegistryBrowseKind,
    browser: Option<Extension<Arc<dyn citadel_registries::registry_images::RegistryBrowsePort>>>,
    headers: &HeaderMap,
) -> HttpResult {
    api_result(
        validate_browse_name(registry, true)
            .and_then(|()| name.map_or(Ok(()), |n| validate_browse_name(n, false)))
            .map_err(|m| ApiError::Validation(m.into())),
        headers,
    )?;
    let id = api_result(
        state
            .registries
            .find_id_by_name(registry)
            .await
            .map_err(crate::api::routes::registries::metadata_error)
            .and_then(|id| id.ok_or(ApiError::NotFound)),
        headers,
    )?;
    api_result(
        state
            .identity
            .authorize_resource(
                principal,
                ResourceType::Registry,
                id,
                PermissionLevel::Read,
                None,
            )
            .await,
        headers,
    )?;
    // Resolve credentials only after resource-specific authorization succeeds.
    let registry = api_result(
        state
            .registries
            .get_registry(id)
            .await
            .map_err(crate::api::routes::registries::metadata_error),
        headers,
    )?;
    if registry.status == citadel_registries::RegistryStatus::Disabled {
        return api_result(
            Err(ApiError::Validation("The Registry is disabled.".into())),
            headers,
        );
    }
    let configuration = registry.configuration;
    let browser = browser
        .map(|Extension(browser)| browser)
        .unwrap_or_else(|| state.registry_browser.clone());
    let _permit = api_result(
        BROWSE_SLOTS
            .try_acquire()
            .map_err(|_| ApiError::Conflict("Registry browsing capacity is busy.".into())),
        headers,
    )?;
    let result = api_result(
        tokio::time::timeout(
            std::time::Duration::from_secs(35),
            browser.browse(&configuration, kind, name),
        )
        .await
        .map_err(|_| ApiError::Validation("Registry request timed out.".into()))
        .and_then(|v| v.map_err(crate::api::routes::registries::metadata_error)),
        headers,
    )?;
    Ok(no_store(Json(result).into_response()))
}

fn window(
    query: Result<Query<Hours>, QueryRejection>,
    headers: &HeaderMap,
) -> HttpResult<StatsWindow> {
    let Query(query) = api_result(query.map_err(invalid_query), headers)?;
    api_result(
        StatsWindow::new(query.hours)
            .ok_or_else(|| ApiError::Validation("Hours must be 24, 48, or 72.".into())),
        headers,
    )
}

fn storage(error: RuntimeCapabilityError) -> ApiError {
    ApiError::internal(error)
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/stats",
    operation_id = "getSwarmTaskStats",
    tag = "Platforms",
    summary = "Get current Task statistics",
    responses(
        (status = 200, description = "Success", body = TaskHistory, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn task_statistics(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    api_result(validate_docker_resource_id(&id), &headers)?;
    authorize_platform(&state, &principal, platform_id, &headers).await?;
    let platform = required(
        state
            .platforms
            .get_platform(platform_id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return api_result(
            Err(ApiError::Validation(
                "Platform must be Docker Swarm.".into(),
            )),
            &headers,
        );
    }
    if platform.status != citadel_primitives::PlatformStatus::Online {
        return api_result(
            Err(ApiError::Conflict(
                "Platform is disconnected or unavailable.".into(),
            )),
            &headers,
        );
    }
    let projection = required(
        state
            .platforms
            .get_swarm_task(platform_id, &id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let live = async {
        state
            .runtime
            .tasks(platform_id, &cancellation)
            .await?
            .inspect_task(&id, &cancellation)
            .await
    }
    .await;
    let live = match live {
        Ok(live) => live,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let docker_id = match citadel_platforms::validate_running_task(&projection, &live) {
        Ok(id) => id,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let store = &state.statistics;
    let target = match store
        .task_container(platform_id, &live.node_id, docker_id)
        .await
    {
        Ok(target) => target,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let Some(target) = target else {
        return api_result(
            Err(ApiError::Conflict(
                "Task Container projection has not synchronized.".into(),
            )),
            &headers,
        );
    };
    let stats = api_result(
        store
            .containers(&[target.id], window, chrono::Utc::now().timestamp())
            .await
            .map_err(storage),
        &headers,
    )?;
    Ok(no_store(
        Json(TaskHistory {
            container_projection_id: target.id,
            docker_container_id: docker_id.to_owned(),
            stats: stats.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/stats",
    operation_id = "getSwarmServiceStats",
    tag = "Platforms",
    summary = "Get Service statistics and node coverage",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::ServiceStatisticsSchema, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn service_statistics(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    api_result(validate_docker_resource_id(&id), &headers)?;
    authorize_platform(&state, &principal, platform_id, &headers).await?;
    let platform = required(
        state
            .platforms
            .get_platform(platform_id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return api_result(
            Err(ApiError::Validation(
                "Platform must be Docker Swarm.".into(),
            )),
            &headers,
        );
    }
    let service = required(
        state
            .platforms
            .get_swarm_service(platform_id, &id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let store = &state.statistics;
    let now = chrono::Utc::now().timestamp();
    let tasks = match store.service_current_tasks(platform_id, &id, now).await {
        Ok(tasks) => tasks,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let stats = api_result(
        store
            .service(
                citadel_platforms::ServiceStatIdentity {
                    platform_id,
                    docker_service_id: &id,
                    managed_service_id: service.swarm_service_id,
                    stack_id: service.stack_id,
                    service_name: &service.name,
                },
                window,
                now,
            )
            .await
            .map_err(storage),
        &headers,
    )?;
    let response = citadel_platforms::ServiceStatistics::new(
        &service,
        tasks,
        stats,
        now.saturating_sub(i64::try_from(state.stats_sample_max_age.as_secs()).unwrap_or(i64::MAX)),
    );
    Ok(no_store(Json(response).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/stats",
    operation_id = "getContainerStats",
    tag = "Containers",
    summary = "Get Container statistics",
    responses(
        (status = 200, description = "Success", body = History<crate::api::resources::schema_models::platforms::ContainerStatSnapshotSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = String, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn container(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(reference) = api_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    api_result(
        if Uuid::parse_str(&reference).is_ok()
            || (reference.len() >= 12
                && reference.len() <= 64
                && reference.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            Ok(())
        } else {
            Err(ApiError::Validation("Must be a valid container id".into()))
        },
        &headers,
    )?;
    let store = &state.statistics;
    let target = match store.find_container(&reference).await {
        Ok(target) => required(Ok(target), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    authorize_platform(&state, &principal, target.platform_id, &headers).await?;
    let stats = api_result(
        store
            .containers(&[target.id], window, chrono::Utc::now().timestamp())
            .await
            .map_err(storage),
        &headers,
    )?;
    Ok(no_store(Json(History { stats }).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/stats",
    operation_id = "getPlatformStats",
    tag = "Platforms",
    summary = "Get Platform statistics",
    responses(
        (status = 200, description = "Success", body = History<crate::api::resources::schema_models::platforms::PlatformStatSnapshotSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn platform(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    authorize_platform(&state, &principal, id, &headers).await?;
    required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error)
            .and_then(|value| {
                value
                    .map(PlatformView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)
            }),
        &headers,
    )?;
    let store = &state.statistics;
    let stats = api_result(
        store
            .platform(id, window, chrono::Utc::now().timestamp())
            .await
            .map_err(storage),
        &headers,
    )?;
    Ok(no_store(Json(History { stats }).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/deployments/{id}/stats",
    operation_id = "getDeploymentStats",
    tag = "Deployments",
    summary = "Get Deployment statistics",
    responses(
        (status = 200, description = "Success", body = History<crate::api::resources::schema_models::platforms::ContainerStatSnapshotSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn deployment(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    workload(
        state,
        principal,
        path,
        query,
        headers,
        StatisticsWorkload::Deployment,
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/stats",
    operation_id = "getStackStats",
    tag = "Stacks",
    summary = "Get Stack Container statistics",
    responses(
        (status = 200, description = "Success", body = StackHistory, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn stack(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    workload(
        state,
        principal,
        path,
        query,
        headers,
        StatisticsWorkload::Stack,
    )
    .await
}

async fn workload(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
    workload: StatisticsWorkload,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    match workload {
        StatisticsWorkload::Deployment => api_result(
            state
                .identity
                .require_resource::<citadel_deployments::permissions::ReadDeployment>(
                    &principal, id,
                )
                .await,
            &headers,
        )?,
        StatisticsWorkload::Stack if !principal.is_administrator() => api_result(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::ReadStack>(&principal, id)
                .await,
            &headers,
        )?,
        StatisticsWorkload::Stack => {}
    }
    let store = &state.statistics;
    let containers = match store.workload_containers(workload, id).await {
        Ok(containers) => required(Ok(containers), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    if matches!(workload, StatisticsWorkload::Deployment) && containers.is_empty() {
        return api_result(Err(ApiError::NotFound), &headers);
    }
    let ids: Vec<_> = containers.iter().map(|c| c.id).collect();
    let stats = match store
        .containers(&ids, window, chrono::Utc::now().timestamp())
        .await
    {
        Ok(stats) => stats,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    if matches!(workload, StatisticsWorkload::Deployment) {
        return Ok(no_store(Json(History { stats }).into_response()));
    }
    let mut grouped = std::collections::HashMap::<_, Vec<_>>::new();
    for sample in stats {
        grouped.entry(sample.container_id).or_default().push(sample);
    }
    let containers = containers
        .into_iter()
        .filter(|c| !c.docker_id.is_empty())
        .map(|c| ContainerHistory {
            container_id: c.docker_id,
            container_name: c.name,
            stats: grouped
                .remove(&c.id)
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
        })
        .collect();
    Ok(no_store(Json(StackHistory { containers }).into_response()))
}

fn validation(value: Result<(), &'static str>) -> Result<(), ApiError> {
    value.map_err(|message| ApiError::Validation(message.into()))
}

async fn context(
    state: &PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    platform: Uuid,
    level: PermissionLevel,
    inspect: bool,
    headers: &HeaderMap,
) -> HttpResult<citadel_platforms::PlatformDetails> {
    let principal = api_result(require_actor(principal), headers)?;
    if platform.is_nil() {
        return api_result(
            Err(ApiError::Validation("Platform id is required.".into())),
            headers,
        );
    }
    authorize_platform_level(state, &principal, platform, level, headers).await?;
    if inspect && !principal.is_administrator() {
        api_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::Platform,
                    platform,
                    PermissionLevel::Read,
                    Some(SpecificPermission::Inspect),
                )
                .await,
            headers,
        )?;
    }
    swarm_platform(state, platform, headers).await?;
    required(
        state
            .platforms
            .get_platform(platform)
            .await
            .map_err(platform_error),
        headers,
    )
}

async fn execute_swarm(
    state: &PlatformsHttpState,
    platform: &citadel_platforms::PlatformDetails,
    operation: citadel_platforms::swarm_mutations::SwarmOperation,
    headers: &HeaderMap,
) -> HttpResult {
    let changed = |kind: &str, action: &str, id: &str| {
        publish_runtime_change(state, platform.id, kind, action, id)
    };
    let service = citadel_platforms::swarm_mutations::SwarmOperations {
        reads: &state.platforms,
        services: state.services.as_ref(),
        runtime: state.runtime.as_ref(),
        projections: state.projections.as_ref(),
        changed: &changed,
    };
    match service
        .execute(platform, operation, &state.tasks.cancellation())
        .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(citadel_platforms::swarm_mutations::SwarmCompletionError::Runtime(error)) => {
            Ok(runtime_error_response(error, headers))
        }
        Err(citadel_platforms::swarm_mutations::SwarmCompletionError::Projection(error)) => {
            api_result(Err(ApiError::internal(error)), headers)
        }
    }
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}",
    operation_id = "updateSwarmNode",
    tag = "Platforms",
    summary = "updateSwarmNode",
    request_body = UpdateSwarmNodeInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("nodeId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_node(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmNodeInput>, JsonRejection>,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::UpdateSwarmNodeInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::UpdateNode { id, input },
        &headers,
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/availability",
    operation_id = "updateSwarmNodesAvailability",
    tag = "Platforms",
    summary = "updateSwarmNodesAvailability",
    request_body = UpdateSwarmNodesAvailabilityInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_availability(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmNodesAvailabilityInput>, JsonRejection>,
) -> HttpResult {
    let Path(pid) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::UpdateSwarmNodesAvailabilityInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::UpdateAvailability(input),
        &headers,
    )
    .await
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/restart",
    operation_id = "restartSwarmService",
    tag = "Platforms",
    summary = "restartSwarmService",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn restart_service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Execute,
        false,
        &headers,
    )
    .await?;
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::RestartService(id),
        &headers,
    )
    .await
}

macro_rules! material_create {
    ($(#[$name_attr:meta])* $name:ident,$secret:expr) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<CreateSwarmMaterialInput>, JsonRejection>,
        ) -> HttpResult {
            create_material(state, principal, path, headers, body, $secret).await
        }
    };
}

material_create!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/secrets",
    operation_id = "createSwarmSecret",
    tag = "Platforms",
    summary = "createSwarmSecret",
    request_body = crate::api::resources::schema_models::platforms::CreateSwarmMaterialInputSchema,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    create_secret,
    true
);

material_create!(
    #[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/configs",
    operation_id = "createSwarmConfig",
    tag = "Platforms",
    summary = "createSwarmConfig",
    request_body = crate::api::resources::schema_models::platforms::CreateSwarmMaterialInputSchema,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    create_config,
    false
);

async fn create_material(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<CreateSwarmMaterialInput>, JsonRejection>,
    secret: bool,
) -> HttpResult {
    let Path(pid) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::CreateMaterial { secret, input },
        &headers,
    )
    .await
}

macro_rules! material_labels {
    ($(#[$name_attr:meta])* $name:ident,$secret:expr) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<UpdateSwarmResourceLabelsInput>, JsonRejection>,
        ) -> HttpResult {
            update_labels(state, principal, path, headers, body, $secret).await
        }
    };
}

material_labels!(
    #[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/secrets/{resourceId}/labels",
    operation_id = "updateSwarmSecretLabels",
    tag = "Platforms",
    summary = "updateSwarmSecretLabels",
    request_body = crate::api::resources::platforms::requests::UpdateSwarmResourceLabelsInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    update_secret_labels,
    true
);

material_labels!(
    #[utoipa::path(
    patch,
    path = "/api/v1/platforms/{platformId}/swarm/configs/{resourceId}/labels",
    operation_id = "updateSwarmConfigLabels",
    tag = "Platforms",
    summary = "updateSwarmConfigLabels",
    request_body = crate::api::resources::platforms::requests::UpdateSwarmResourceLabelsInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    update_config_labels,
    false
);

async fn update_labels(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<UpdateSwarmResourceLabelsInput>, JsonRejection>,
    secret: bool,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Write,
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::UpdateSwarmResourceLabelsInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::UpdateLabels { secret, id, input },
        &headers,
    )
    .await
}

macro_rules! delete_resources {
    ($(#[$name_attr:meta])* $name:ident,$kind:expr) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            body: Result<Json<DeleteSwarmResourcesInput>, JsonRejection>,
        ) -> HttpResult {
            delete_swarm_resources(state, principal, path, headers, body, $kind).await
        }
    };
}

delete_resources!(
    #[utoipa::path(
    delete,
    path = "/api/v1/platforms/{platformId}/swarm/services",
    operation_id = "deleteSwarmInventoryServices",
    tag = "Platforms",
    summary = "deleteSwarmInventoryServices",
    request_body = crate::api::resources::platforms::requests::DeleteSwarmResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    delete_services,
    SwarmResourceKind::Service
);

delete_resources!(
    #[utoipa::path(
    delete,
    path = "/api/v1/platforms/{platformId}/swarm/secrets",
    operation_id = "deleteSwarmSecrets",
    tag = "Platforms",
    summary = "deleteSwarmSecrets",
    request_body = crate::api::resources::platforms::requests::DeleteSwarmResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    delete_secrets,
    SwarmResourceKind::Secret
);

delete_resources!(
    #[utoipa::path(
    delete,
    path = "/api/v1/platforms/{platformId}/swarm/configs",
    operation_id = "deleteSwarmConfigs",
    tag = "Platforms",
    summary = "deleteSwarmConfigs",
    request_body = crate::api::resources::platforms::requests::DeleteSwarmResourcesInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    delete_configs,
    SwarmResourceKind::Config
);

async fn delete_swarm_resources(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<DeleteSwarmResourcesInput>, JsonRejection>,
    kind: SwarmResourceKind,
) -> HttpResult {
    let Path(pid) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        if kind == SwarmResourceKind::Service {
            PermissionLevel::Execute
        } else {
            PermissionLevel::Write
        },
        false,
        &headers,
    )
    .await?;
    let Json(input) = api_result(body.map_err(invalid_json), &headers)?;
    let input: citadel_platforms::swarm_mutations::DeleteSwarmResourcesInput = input.into();
    execute_swarm(
        &state,
        &platform,
        citadel_platforms::swarm_mutations::SwarmOperation::Delete { kind, input },
        &headers,
    )
    .await
}

pub(crate) fn inspect_service_view(
    value: citadel_platforms::swarm_mutations::SwarmServiceInspection,
) -> crate::api::resources::platforms::operation_views::ServiceInspectionView {
    crate::api::resources::platforms::operation_views::ServiceInspectionView {
        id: value.id,
        version_index: value.version_index,
        name: value.name,
        mode: value.mode,
        image: value.image,
        running_task_count: value.running_task_count,
        desired_task_count: value.desired_task_count,
        update_state: value.update_state,
        update_message: value.update_message,
        ports: value.ports,
        network_ids: value.network_ids,
        secret_ids: value.secret_ids,
        config_ids: value.config_ids,
        labels: value.labels,
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

macro_rules! reader {
    ($(#[$name_attr:meta])* $name:ident,$kind:expr) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_swarm_resource(state, principal, path, headers, $kind).await
        }
    };
}

reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}/inspect",
    operation_id = "inspectSwarmNode",
    tag = "Platforms",
    summary = "inspectSwarmNode",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::NodeInspectionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("nodeId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_node,
    "node"
);

reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/inspect",
    operation_id = "inspectSwarmService",
    tag = "Platforms",
    summary = "inspectSwarmService",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ServiceInspectionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_service,
    "service"
);

reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/configs/{resourceId}/content",
    operation_id = "getSwarmConfigData",
    tag = "Platforms",
    summary = "getSwarmConfigData",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::operation_views::ConfigContentView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    config_data,
    "config"
);

async fn read_swarm_resource(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    kind: &str,
) -> HttpResult {
    let Path((pid, id)) = api_result(path.map_err(invalid_path), &headers)?;
    let platform = context(
        &state,
        principal,
        pid,
        PermissionLevel::Read,
        true,
        &headers,
    )
    .await?;
    api_result(validation(resource_id(&id)), &headers)?;
    match kind {
        "node" => {
            required(
                state
                    .platforms
                    .get_swarm_node(pid, &id)
                    .await
                    .map(|value| {
                        value.map(crate::api::resources::platforms::views::SwarmNodeView::from)
                    })
                    .map_err(platform_error),
                &headers,
            )?;
        }
        "service" => {
            required(
                state
                    .platforms
                    .get_swarm_service(pid, &id)
                    .await
                    .map(|value| {
                        value.map(crate::api::resources::platforms::views::SwarmServiceView::from)
                    })
                    .map_err(platform_error),
                &headers,
            )?;
        }
        _ => {
            required(
                state
                    .platforms
                    .get_swarm_config(pid, &id)
                    .await
                    .map(|value| {
                        value.map(crate::api::resources::platforms::views::SwarmConfigView::from)
                    })
                    .map_err(platform_error),
                &headers,
            )?;
        }
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let runtime = api_result(
        state
            .runtime
            .swarm(pid, &cancel)
            .await
            .map_err(ApiError::from),
        &headers,
    )?;
    let result = citadel_platforms::swarm_mutations::bounded(async {
        let client = &runtime;
        if kind == "config" {
            manager_identity(runtime.as_ref(), &platform, &cancel).await?;
        }
        match kind {
            "service" => {
                let service = client.inspect_service(&id, &cancel).await?;
                if service.id != id {
                    return Err(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Remote,
                        "Docker returned a different Service.",
                        false,
                    ));
                }
                Ok(json!(inspect_service_view(service)))
            }
            "node" => {
                let (node, running, desired) = client.inspect_node(&id, &cancel).await?;
                if node.id != id {
                    return Err(RuntimeCapabilityError::new(
                        RuntimeErrorKind::Remote,
                        "Docker returned a different Node.",
                        false,
                    ));
                }
                Ok(json!(
                    crate::api::resources::platforms::operation_views::NodeInspectionView {
                        id: node.id,
                        version_index: node.version_index,
                        hostname: node.hostname,
                        role: node.role,
                        is_leader: node.is_leader,
                        reachability: node.reachability,
                        status: node.status,
                        status_message: node.status_message,
                        availability: node.availability,
                        engine_version: node.engine_version,
                        operating_system: node.operating_system,
                        architecture: node.architecture,
                        address: node.address,
                        labels: node.labels,
                        running_task_count: running,
                        desired_task_count: desired,
                        created_at: node.created_at,
                        updated_at: node.updated_at
                    }
                ))
            }
            _ => Ok(json!(
                crate::api::resources::platforms::operation_views::ConfigContentView {
                    content: client.config_data(&id, &cancel).await?
                }
            )),
        }
    })
    .await;
    match result {
        Ok(value) => Ok(no_store(Json(value).into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm",
    operation_id = "getSwarmOverview",
    tag = "Platforms",
    summary = "Get Swarm inventory health and quorum",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::swarm_views::SwarmOverviewView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    if id.is_nil() {
        return api_result(
            Err(ApiError::Validation("Invalid Platform id.".into())),
            &headers,
        );
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
    if platform.platform_type != citadel_platforms::PlatformKind::DockerSwarm {
        return api_result(
            Err(ApiError::Validation(
                "Platform must be Docker Swarm.".into(),
            )),
            &headers,
        );
    }
    let mut summary = api_result(
        state
            .platforms
            .swarm_summary(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if summary.node_count == 0 && platform.status == citadel_primitives::PlatformStatus::Online {
        match initialize_swarm(&state, &platform).await {
            Ok(true) => publish_runtime_change(&state, id, "platform", "update", &id.to_string()),
            Ok(false) => {}
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        }
        platform = required(
            state
                .platforms
                .get_platform(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        summary = api_result(
            state
                .platforms
                .swarm_summary(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
    }
    let descriptor = &platform.platform_descriptor.routing;
    let control = descriptor.control_available;
    let error = descriptor.error.as_deref();
    Ok(no_store(
        Json(crate::api::routes::platforms::swarm_views::overview(
            summary,
            id,
            platform.status == citadel_primitives::PlatformStatus::Online,
            control,
            error,
            capabilities,
        ))
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/refresh",
    operation_id = "refreshSwarmInventory",
    tag = "Platforms",
    summary = "Refresh Swarm inventory from the manager",
    responses(
        (status = 204, description = "Inventory refreshed"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn refresh_swarm_inventory(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let Path(id) = api_result(path.map_err(invalid_path), &headers)?;
    // Refresh only observes Docker state; it does not modify Swarm resources.
    let platform = context(
        &state,
        principal,
        id,
        PermissionLevel::Read,
        false,
        &headers,
    )
    .await?;
    match citadel_platforms::swarm_mutations::refresh_inventory(
        state.projections.as_ref(),
        state.runtime.as_ref(),
        &platform,
        |kind| publish_runtime_change(&state, id, kind, "update", ""),
    )
    .await
    {
        Ok(()) => Ok(no_store(StatusCode::NO_CONTENT.into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

macro_rules! task_reader {
    ($(#[$name_attr:meta])* $name:ident, $permission:ident) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<PlatformsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<(Uuid, String)>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            read_task_runtime(
                state,
                principal,
                path,
                headers,
                SpecificPermission::$permission,
            )
            .await
        }
    };
}

task_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/inspect",
    operation_id = "inspectSwarmTask",
    tag = "Platforms",
    summary = "Inspect the current Task container",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::platforms::descriptor_views::ContainerInspectionView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    inspect_swarm_task,
    Inspect
);

task_reader!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/terminal",
    operation_id = "getSwarmTaskTerminalTarget",
    tag = "Platforms",
    summary = "Resolve the current Task terminal target",
    responses(
        (status = 200, description = "Success", body = TaskTerminalView, content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    terminal,
    Terminal
);

async fn read_task_runtime(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    specific: SpecificPermission,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(validate_docker_resource_id(&id), &headers)?;
    if platform.is_nil() || id.len() > 255 {
        return api_result(
            Err(ApiError::Validation("Invalid Task identity.".into())),
            &headers,
        );
    }
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .authorize_resource(
                    &principal,
                    ResourceType::Platform,
                    platform,
                    PermissionLevel::Read,
                    Some(specific),
                )
                .await,
            &headers,
        )?;
    }
    swarm_platform(&state, platform, &headers).await?;
    let projection = required(
        state
            .platforms
            .get_swarm_task(platform, &id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    // Bound both manager inspection and the node-local read with one deadline.
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let live = state
            .runtime
            .tasks(platform, &cancel)
            .await?
            .inspect_task(&id, &cancel)
            .await?;
        let docker_id = citadel_platforms::validate_running_task(&projection, &live)?;
        if specific == SpecificPermission::Terminal {
            return Ok(Json(TaskTerminalView {
                docker_container_id: docker_id.into(),
            })
            .into_response());
        }
        // Use the existing redacted Container inspection contract, not raw Task JSON.
        let inspection = state
            .runtime
            .container_inspection(platform, Some(&live.node_id), &cancel)
            .await?
            .inspection(docker_id, &cancel)
            .await?;
        let inspection = serde_json::from_value::<
            crate::api::resources::platforms::descriptor_views::ContainerInspectionView,
        >(inspection)
        .map_err(|_| {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Conflict,
                "Invalid Container inspection response.",
                false,
            )
        })?;
        Ok(Json(inspection).into_response())
    })
    .await
    .unwrap_or_else(|_| {
        Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "Task runtime read timed out.",
            false,
        ))
    });
    match result {
        Ok(response) => Ok(no_store(response)),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

async fn authorize(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    platform: Uuid,
    specific: SpecificPermission,
    headers: &HeaderMap,
) -> HttpResult<()> {
    authorize_platform_level(state, principal, platform, PermissionLevel::Read, headers).await?;
    if !principal.is_administrator() {
        api_result(
            state
                .identity
                .authorize_resource(
                    principal,
                    ResourceType::Volume,
                    platform,
                    PermissionLevel::Read,
                    Some(specific),
                )
                .await,
            headers,
        )?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/volumes/{name}/files",
    operation_id = "listVolumeDirectory",
    tag = "Platforms",
    summary = "Browse a Volume directory",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::schema_models::platforms::VolumeDirectorySchema, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("name" = String, Path), ("path" = Option<String>, Query), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ContentQuery>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, name)) = api_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        platform,
        SpecificPermission::Browse,
        &headers,
    )
    .await?;
    let Query(query) = api_result(query.map_err(invalid_query), &headers)?;
    let path = match normalize_path(query.path.as_deref()) {
        Ok(path) => path,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancellation = CancellationToken::new();
    let _guard = cancellation.clone().drop_guard();
    match state
        .volume_content
        .list(
            platform,
            &name,
            path,
            query.docker_node_id.as_deref(),
            &cancellation,
        )
        .await
    {
        Ok(listing) => Ok(no_store(Json(listing).into_response())),
        Err(error) => Ok(runtime_error_response(error, &headers)),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/volumes/{name}/files/download",
    operation_id = "downloadVolumePath",
    tag = "Platforms",
    summary = "Download a Volume file or directory",
    responses(
        (status = 200, description = "Success"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("name" = String, Path), ("path" = Option<String>, Query), ("dockerNodeId" = Option<String>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn download(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ContentQuery>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform, name)) = api_result(path.map_err(invalid_path), &headers)?;
    authorize(
        &state,
        &principal,
        platform,
        SpecificPermission::Download,
        &headers,
    )
    .await?;
    let Query(query) = api_result(query.map_err(invalid_query), &headers)?;
    let path = match normalize_path(query.path.as_deref()) {
        Ok(path) => path,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let cancellation = CancellationToken::new();
    let guard = cancellation.clone().drop_guard();
    let download = match state
        .volume_content
        .download(
            platform,
            &name,
            path,
            query.docker_node_id.as_deref(),
            &cancellation,
        )
        .await
    {
        Ok(download) => download,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let disposition = format!(
        "attachment; filename=\"download{}\"; filename*=UTF-8''{}",
        if download.directory { ".tar" } else { "" },
        urlencoding::encode(&download.filename)
    );
    let directory = download.directory;
    let filename = download.filename;
    let path = path.to_owned();
    let mut input = download.stream;
    let stream: futures_util::stream::BoxStream<'static, Result<Vec<u8>, std::io::Error>> =
        Box::pin(async_stream::try_stream! {
            let _guard = guard;
            while let Some(chunk) = input.next().await { yield chunk?; }
            let activity = state.volume_activity.clone();
            let details = citadel_activities::VolumeContentDownloaded { volume_name: name, path, is_directory: directory, file_name: filename };
            if !matches!(tokio::time::timeout(std::time::Duration::from_secs(5), activity.record_volume_download(principal.actor_id, platform, details)).await, Ok(Ok(()))) {
                tracing::warn!("Could not record completed Volume download activity.");
            }
        });
    let mut response = axum::response::Response::new(axum::body::Body::from_stream(stream));
    response.headers_mut().insert(
        "Content-Type",
        HeaderValue::from_static(if directory {
            "application/x-tar"
        } else {
            "application/octet-stream"
        }),
    );
    response.headers_mut().insert(
        "Content-Disposition",
        HeaderValue::from_str(&disposition).expect("percent-encoded download filename"),
    );
    response.headers_mut().insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    // No guessed Content-Length: the underlying Volume is live and may change.
    Ok(no_store(response))
}

#[cfg(test)]
mod tests {
    #[test]
    fn platform_tags_accept_names_repeated_keys_and_ids() {
        let id = uuid::Uuid::now_v7();
        assert_eq!(
            super::parse_tag_filters(Some(&format!(
                "TaGs=Prod%20%26%20Europe&tags=prod+%26+europe&tags=%20&tags={id}"
            )))
            .unwrap(),
            vec!["Prod & Europe".to_owned(), id.to_string()]
        );
        assert!(super::parse_tag_filters(None).unwrap().is_empty());
        let query = (0..100)
            .map(|n| format!("tags=tag-{n}"))
            .collect::<Vec<_>>()
            .join("&");
        assert_eq!(super::parse_tag_filters(Some(&query)).unwrap().len(), 100);
        assert!(super::parse_tag_filters(Some(&format!("{query}&tags=extra"))).is_err());
    }

    use crate::api::routes::platforms::*;

    #[test]
    fn volume_usage_is_serialized_with_the_existing_ui_field_names() {
        for usage in [
            serde_json::json!({"Size": 1024, "RefCount": 2}),
            serde_json::json!({"size": 1024, "refCount": 2}),
        ] {
            let view = map_volume(
                RuntimeVolumeSummary {
                    usage_data: Some(usage),
                    ..Default::default()
                },
                VolumeCapabilitiesView::default(),
            )
            .unwrap();
            let json = serde_json::to_value(view).unwrap();
            assert_eq!(
                json["usageData"],
                serde_json::json!({"size":1024, "refCount":2})
            );
        }
        let view = map_volume(
            RuntimeVolumeSummary::default(),
            VolumeCapabilitiesView::default(),
        )
        .unwrap();
        assert!(view.usage_data.is_none());
    }

    #[test]
    fn null_volume_status_matches_the_empty_dotnet_string() {
        let volume = RuntimeVolumeSummary {
            status: std::collections::BTreeMap::from([("missing".into(), serde_json::Value::Null)]),
            ..Default::default()
        };
        assert_eq!(
            map_volume(volume, VolumeCapabilitiesView::default())
                .unwrap()
                .status["missing"],
            ""
        );
    }

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
    fn network_create_accepts_the_existing_ui_ipam_and_ip_version_fields() {
        let input: CreateNetworkInput = serde_json::from_value(serde_json::json!({
            "platformId": Uuid::new_v4(), "name":"fscsd", "driver":"bridge",
            "scope":"local", "enableIPv4":true, "enableIPv6":false,
            "internal":false, "attachable":false, "ingress":false,
            "labels":{}, "options":{}, "ipam":{"driver":"default", "config":[{},{}]},
            "configOnly":false
        }))
        .unwrap();
        assert!(validate_network_input(&input.network).is_ok());
        assert_eq!(input.network.enable_ipv4, Some(true));
        assert_eq!(input.network.enable_ipv6, Some(false));
        assert!(input.network.ipam.unwrap().config[0].subnet.is_none());
        let invalid: CreateRuntimeNetwork = serde_json::from_value(serde_json::json!({
            "name":"invalid", "driver":"bridge", "scope":"local",
            "enableIPv4":false, "enableIPv6":false
        }))
        .unwrap();
        assert!(validate_network_input(&invalid).is_err());
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

#[cfg(test)]
mod image_pull_tests {
    use crate::api::routes::platforms::*;

    #[tokio::test]
    async fn terminal_error_does_not_wait_for_space_in_a_stalled_progress_queue() {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (completed, completion) = tokio::sync::oneshot::channel();
        assert!(
            sender
                .try_send(PullImageStreamItem {
                    status: Some("Pulling".into()),
                    ..Default::default()
                })
                .is_ok()
        );
        assert_eq!(sender.capacity(), 0);
        assert!(
            completed
                .send(Some(PullImageStreamItem {
                    error_message: Some("Image pull timed out.".into()),
                    ..Default::default()
                }))
                .is_ok()
        );
        drop(sender);
        let cancel = CancellationToken::new();
        let bytes = axum::body::to_bytes(
            progress_body(receiver, completion, cancel.drop_guard()),
            4096,
        )
        .await
        .unwrap();
        let items: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 2);
        assert_eq!(items[0]["status"], "Pulling");
        assert_eq!(items[1]["errorMessage"], "Image pull timed out.");
    }
}

async fn attach_volume_coverage(
    state: &PlatformsHttpState,
    principal: &ActorPrincipal,
    platform_id: Uuid,
    volumes: &mut [VolumeView],
    headers: &HeaderMap,
) -> HttpResult<()> {
    let keys = volumes
        .iter()
        .map(|v| (v.name.clone(), v.docker_node_id.clone()))
        .collect::<Vec<_>>();
    let coverage = api_result(
        state
            .volume_coverage
            .volume_coverage(
                principal.actor_id,
                principal.is_administrator(),
                platform_id,
                &keys,
            )
            .await
            .map_err(ApiError::internal),
        headers,
    )?;
    let mut coverage = coverage
        .into_iter()
        .map(|c| ((c.volume_name.clone(), c.docker_node_id.clone()), c))
        .collect::<std::collections::HashMap<_, _>>();
    for volume in volumes {
        if let Some(c) = coverage.remove(&(volume.name.clone(), volume.docker_node_id.clone())) {
            volume.backup_coverage = Some(
                crate::api::resources::platforms::views::BackupCoverageView {
                    status: c.status,
                    policy_count: c.policy_count,
                    last_run_id: c.last_run_id,
                    last_run_status: c.last_run_status,
                    last_run_at: c.last_run_at,
                    last_successful_run_at: c.last_successful_run_at,
                    next_run_at: None,
                },
            );
        }
    }
    Ok(())
}
