use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resource_access::{
            authorize_global, authorize_resource, capabilities, publish_resource_change,
            require_actor,
        },
        resources::tags::{
            requests::{NewTag, ReplaceResourceTagsInput, TagPatch},
            views::{AuthorizedTagView, ResourceTagsResponse, TagsResponse},
        },
    },
    openapi::router::OpenApiRouterExt,
    realtime::RealtimeHub,
    request_validation::invalid_json,
};

use axum::{
    Json, Router,
    extract::{
        Extension, Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::HeaderMap,
    response::IntoResponse,
};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_tags::TaggableResourceType;

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct TagsHttpState {
    pub identity: Arc<IdentityService>,
    pub tags: Arc<dyn citadel_tags::TagRepository>,
    pub realtime: Option<RealtimeHub>,
}

pub(crate) fn metadata_error(error: citadel_tags::TagError) -> ApiError {
    match error {
        citadel_tags::TagError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        citadel_tags::TagError::NotFound => ApiError::NotFound,
        citadel_tags::TagError::Conflict(message) => ApiError::Conflict(message),
        source @ citadel_tags::TagError::Storage(_) => ApiError::internal(source),
    }
}

pub fn router(state: TagsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<TagsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_tags))
        .normalized_routes(utoipa_axum::routes!(create_tag))
        .normalized_routes(utoipa_axum::routes!(patch_tag))
        .normalized_routes(utoipa_axum::routes!(delete_tag))
        .normalized_routes(utoipa_axum::routes!(get_platform_tags))
        .normalized_routes(utoipa_axum::routes!(get_deployment_tags))
        .normalized_routes(utoipa_axum::routes!(replace_deployment_tags))
        .normalized_routes(utoipa_axum::routes!(get_stack_tags))
        .normalized_routes(utoipa_axum::routes!(replace_stack_tags))
        .normalized_routes(utoipa_axum::routes!(get_service_tags))
        .normalized_routes(utoipa_axum::routes!(replace_service_tags))
        .normalized_routes(utoipa_axum::routes!(get_action_tags))
        .normalized_routes(utoipa_axum::routes!(replace_action_tags))
        .normalized_routes(utoipa_axum::routes!(get_build_tags))
        .normalized_routes(utoipa_axum::routes!(replace_build_tags))
        .normalized_routes(utoipa_axum::routes!(get_pool_tags))
        .normalized_routes(utoipa_axum::routes!(replace_pool_tags))
        .normalized_routes(utoipa_axum::routes!(get_policy_tags))
        .normalized_routes(utoipa_axum::routes!(replace_policy_tags))
        .normalized_routes(utoipa_axum::routes!(replace_platform_tags))
        .normalized_routes(utoipa_axum::routes!(get_registry_tags))
        .normalized_routes(utoipa_axum::routes!(replace_registry_tags))
        .normalized_routes(utoipa_axum::routes!(get_git_repository_tags))
        .normalized_routes(utoipa_axum::routes!(replace_git_repository_tags))
}

#[utoipa::path(
    get,
    path = "/api/v1/tags",
    operation_id = "listTags",
    tag = "Tags",
    summary = "List resource tags",
    responses(
        (status = 200, description = "Success", body = TagsResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_tags(
    State(state): State<TagsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let global_capabilities = capabilities(
        &state.identity,
        &principal,
        ResourceType::Tag,
        None,
        &headers,
    )
    .await?;
    let authorized_tags = api_result(
        state
            .tags
            .list_tags(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    // SQL has already selected the authorized IDs. Resolve their capability
    // metadata with one batch of ACL misses, including denied entries.
    let permission_ids = authorized_tags
        .iter()
        .map(|resource| resource.id)
        .collect::<Vec<_>>();
    let row_permissions = api_result(
        state
            .identity
            .permissions_for_resources(&principal, ResourceType::Tag, &permission_ids)
            .await,
        &headers,
    )?;
    let mut tags = Vec::with_capacity(authorized_tags.len());
    for tag in authorized_tags {
        let capabilities = crate::api::resource_access::capabilities_from_permission(
            row_permissions.get(&tag.id).copied().flatten(),
        );
        tags.push(AuthorizedTagView {
            tag: tag.into(),
            capabilities,
        });
    }
    Ok(no_store(
        Json(TagsResponse {
            tags,
            capabilities: global_capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/tags",
    operation_id = "createTag",
    tag = "Tags",
    summary = "Create a resource tag",
    request_body = NewTag,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::tags::views::TagView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_tag(
    State(state): State<TagsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewTag>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let mut input: citadel_tags::NewTag = input.into();
    let principal = api_result(require_actor(principal), &headers)?;
    authorize_global(
        &state.identity,
        &principal,
        ResourceType::Tag,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    api_result(input.validate().map_err(metadata_error), &headers)?;
    let tag = api_result(
        state
            .tags
            .create_tag(principal.actor_id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Tag", "tagChanged");
    Ok(no_store(
        Json(crate::api::resources::tags::views::TagView::from(tag)).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/tags/{id}",
    operation_id = "patchTag",
    tag = "Tags",
    summary = "Update a resource tag",
    request_body = TagPatch,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::tags::views::TagView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn patch_tag(
    State(state): State<TagsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<TagPatch>, JsonRejection>,
) -> HttpResult {
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let input: citadel_tags::TagPatch = input.into();
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::Tag,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let tag = api_result(
        state
            .tags
            .update_tag(id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Tag", "tagChanged");
    Ok(no_store(
        Json(crate::api::resources::tags::views::TagView::from(tag)).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/tags/{id}",
    operation_id = "deleteTag",
    tag = "Tags",
    summary = "Delete a resource tag",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_tag(
    State(state): State<TagsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state.identity,
        &principal,
        ResourceType::Tag,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    api_result(
        state.tags.delete_tag(id).await.map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state.realtime, "Tag", "tagChanged");
    Ok(no_store(axum::http::StatusCode::NO_CONTENT.into_response()))
}

macro_rules! resource_tag_handlers {
    ($(#[$get_attr:meta])* $get:ident, $(#[$replace_attr:meta])* $replace:ident, $tag_type:expr, $resource_type:expr) => {
        $(#[$get_attr])*
        async fn $get(
            State(state): State<TagsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> HttpResult {
            resource_tags(state, principal, path, headers, $tag_type, $resource_type).await
        }

        $(#[$replace_attr])*
        async fn $replace(
            State(state): State<TagsHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            input: Result<Json<ReplaceResourceTagsInput>, JsonRejection>,
        ) -> HttpResult {
            let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
            replace_resource_tags(
                state,
                principal,
                path,
                headers,
                input,
                $tag_type,
                $resource_type,
            )
            .await
        }
    };
}

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/platforms/{id}/tags",
    operation_id = "getPlatformTags",
    tag = "Platforms",
    summary = "Get Platform tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_platform_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/platforms/{id}/tags",
    operation_id = "replacePlatformTags",
    tag = "Platforms",
    summary = "Replace Platform tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_platform_tags,
    TaggableResourceType::Platform,
    ResourceType::Platform
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/registries/{id}/tags",
    operation_id = "getRegistryTags",
    tag = "Registries",
    summary = "Get Registry tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_registry_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/registries/{id}/tags",
    operation_id = "replaceRegistryTags",
    tag = "Registries",
    summary = "Replace Registry tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_registry_tags,
    TaggableResourceType::Registry,
    ResourceType::Registry
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/gitRepositories/{id}/tags",
    operation_id = "getGitRepositoryTags",
    tag = "GitRepositories",
    summary = "Get Git repository tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_git_repository_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/gitRepositories/{id}/tags",
    operation_id = "replaceGitRepositoryTags",
    tag = "GitRepositories",
    summary = "Replace Git repository tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_git_repository_tags,
    TaggableResourceType::GitRepository,
    ResourceType::GitRepository
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/automation/actions/{id}/tags",
    operation_id = "getAutomationActionTags",
    tag = "AutomationActions",
    summary = "Get AutomationAction tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_action_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/automation/actions/{id}/tags",
    operation_id = "replaceAutomationActionTags",
    tag = "AutomationActions",
    summary = "Replace AutomationAction tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_action_tags,
    TaggableResourceType::AutomationAction,
    ResourceType::AutomationAction
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/buildProjects/{id}/tags",
    operation_id = "getBuildTags",
    tag = "BuildProjects",
    summary = "Get BuildProject tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_build_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/buildProjects/{id}/tags",
    operation_id = "replaceBuildTags",
    tag = "BuildProjects",
    summary = "Replace BuildProject tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_build_tags,
    TaggableResourceType::Build,
    ResourceType::Build
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools/{id}/tags",
    operation_id = "getBuildAgentPoolTags",
    tag = "BuildAgentPools",
    summary = "Get BuildAgentPool tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_pool_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/buildAgentPools/{id}/tags",
    operation_id = "replaceBuildAgentPoolTags",
    tag = "BuildAgentPools",
    summary = "Replace BuildAgentPool tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_pool_tags,
    TaggableResourceType::BuildAgentPool,
    ResourceType::BuildAgentPool
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/backupPolicies/{id}/tags",
    operation_id = "getBackupPolicyTags",
    tag = "BackupPolicies",
    summary = "Get BackupPolicy tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_policy_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/backupPolicies/{id}/tags",
    operation_id = "replaceBackupPolicyTags",
    tag = "BackupPolicies",
    summary = "Replace BackupPolicy tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_policy_tags,
    TaggableResourceType::BackupPolicy,
    ResourceType::BackupPolicy
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/deployments/{deploymentId}/tags",
    operation_id = "getDeploymentTags",
    tag = "Deployments",
    summary = "Get Deployment tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("deploymentId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_deployment_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/deployments/{deploymentId}/tags",
    operation_id = "replaceDeploymentTags",
    tag = "Deployments",
    summary = "Replace Deployment tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("deploymentId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_deployment_tags,
    TaggableResourceType::Deployment,
    ResourceType::Deployment
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/tags",
    operation_id = "getStackTags",
    tag = "Stacks",
    summary = "Get Stack tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_stack_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/stacks/{stackId}/tags",
    operation_id = "replaceStackTags",
    tag = "Stacks",
    summary = "Replace Stack tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_stack_tags,
    TaggableResourceType::Stack,
    ResourceType::Stack
);

resource_tag_handlers!(
    #[utoipa::path(
    get,
    path = "/api/v1/swarmServices/{id}/tags",
    operation_id = "getSwarmServiceTags",
    tag = "SwarmServices",
    summary = "Get SwarmService tags",
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    get_service_tags,
    #[utoipa::path(
    put,
    path = "/api/v1/swarmServices/{id}/tags",
    operation_id = "replaceSwarmServiceTags",
    tag = "SwarmServices",
    summary = "Replace SwarmService tags",
    request_body = crate::api::resources::tags::requests::ReplaceResourceTagsInput,
    responses(
        (status = 200, description = "Success", body = ResourceTagsResponse, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    replace_service_tags,
    TaggableResourceType::SwarmService,
    ResourceType::SwarmService
);

async fn resource_tags(
    state: TagsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    tag_type: TaggableResourceType,
    resource_type: ResourceType,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state.identity,
        &principal,
        resource_type,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let tags = api_result(
        state
            .tags
            .get_resource_tags(tag_type, id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ResourceTagsResponse {
            tags: tags.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

async fn replace_resource_tags(
    state: TagsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: ReplaceResourceTagsInput,
    tag_type: TaggableResourceType,
    resource_type: ResourceType,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path(id) = api_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state.identity,
        &principal,
        resource_type,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let tags = api_result(
        state
            .tags
            .replace_resource_tags(principal.actor_id, tag_type, id, &input.tag_ids)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(
        &state.realtime,
        tag_type.as_database_str(),
        "resourceTagsChanged",
    );
    Ok(no_store(
        Json(ResourceTagsResponse {
            tags: tags.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

fn invalid_path(message: String) -> crate::api::error::ApiError {
    crate::api::error::ApiError::Validation(message)
}

pub(crate) fn parse_filters(
    query: Option<&str>,
) -> Result<Vec<String>, crate::api::error::ApiError> {
    let mut tags = Vec::new();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key != "tags" {
            continue;
        }
        if tags.len() >= 100 || value.len() > 128 || value.trim().is_empty() {
            return Err(crate::api::error::ApiError::Validation(
                "Tag filters must contain at most 100 non-empty tag names or IDs.".into(),
            ));
        }
        tags.push(value.into_owned());
    }
    Ok(tags)
}

pub(crate) fn matches_filters(tags: &[citadel_tags::TagSummary], filters: &[String]) -> bool {
    filters.iter().all(|filter| {
        tags.iter().any(|tag| {
            tag.name.eq_ignore_ascii_case(filter)
                || Uuid::parse_str(filter).is_ok_and(|id| id == tag.id)
        })
    })
}
