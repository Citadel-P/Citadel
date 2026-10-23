//! Platforms HTTP routes, authorization and local request handling.
use crate::api::error::runtime_error_response;
use crate::api::resources::capabilities::ResourceCapabilitiesView;
use crate::api::resources::platforms::capabilities::{
    container_capabilities, permission_for, platform_capabilities, resource_capabilities,
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
        request_validation::{
            ApiPath, ApiQuery, AuthenticatedActor, ValidatedJson, invalid_json, invalid_path,
            invalid_query,
        },
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
            views::{
                ContainerView, ContainersResponse, History, ImagesResponse, NetworkView,
                NetworksResponse, PlatformCapabilitiesView, PlatformView, PlatformsResponse,
                StackHistory, StackServiceHistory, SwarmConfigView, SwarmItemsResponse,
                SwarmNetworkView, SwarmNodeView, SwarmSecretView, SwarmServiceView, SwarmTaskView,
                TaskHistory, TaskTerminalView, VolumeCapabilitiesView, VolumeView, VolumesResponse,
            },
        },
    },
    openapi::router::OpenApiRouterExt,
    realtime::RealtimeHub,
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
    pub image_updates: Arc<citadel_platforms::image_updates::ImageUpdateService>,
    pub backup_coverage:
        Option<Arc<crate::api::resources::backups::coverage::BackupCoverageService>>,
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
        .merge(crate::api::routes::image_management::documented_routes())
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(refresh_swarm_inventory))
        .normalized_routes(utoipa_axum::routes!(inspect_swarm_task))
        .normalized_routes(utoipa_axum::routes!(terminal))
        .normalized_routes(utoipa_axum::routes!(inspect_container))
        .normalized_routes(utoipa_axum::routes!(container_networking))
        .normalized_routes(utoipa_axum::routes!(network_containers))
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
        .normalized_routes(utoipa_axum::routes!(container_health_history))
        .normalized_routes(utoipa_axum::routes!(container_processes))
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

pub(super) async fn authorize_platform_level(
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

pub(super) async fn platform_is_swarm(
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

pub(super) fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, ApiError> {
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
            crate::api::request_validation::validation_error(message)
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

mod registration;
use registration::*;
mod containers;
use containers::*;
mod images;
use images::*;
mod networks;
pub(crate) use networks::lookup_platform_resources;
use networks::*;
mod volumes;
use volumes::*;
mod swarm;
use swarm::*;
mod logs;
use logs::*;
mod statistics;
use statistics::*;

pub use registration::AgentSetupContext;
#[cfg(test)]
mod tests;
