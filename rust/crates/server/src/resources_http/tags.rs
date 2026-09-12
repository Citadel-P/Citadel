use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::ActorPrincipal;
use citadel_platforms::ResourceCapabilitiesView;
use citadel_resources::{NewTag, TagPatch, TagSummary, TagView, TaggableResourceType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    ResourcesHttpState, authorize_global, authorize_resource, capabilities, invalid_json,
    metadata_error, publish_resource_change, require_actor,
};
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::openapi::router::OpenApiRouterExt;

pub(super) fn router(state: ResourcesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct TagsResponse {
    tags: Vec<AuthorizedTagView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct AuthorizedTagView {
    #[serde(flatten)]
    tag: TagView,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceTagsResponse {
    tags: Vec<TagSummary>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceResourceTagsInput {
    #[serde(default)]
    tag_ids: Vec<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tags",
    operation_id = "listTags",
    summary = "List resource tags",
    responses(
        (status = 200, description = "Success", body = TagsResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_tags(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let global_capabilities =
        capabilities(&state, &principal, ResourceType::Tag, None, &headers).await?;
    let authorized_tags = identity_result(
        state
            .resources
            .store()
            .list_tags(principal.actor_id, principal.is_administrator())
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    let mut tags = Vec::with_capacity(authorized_tags.len());
    for tag in authorized_tags {
        let capabilities = capabilities(
            &state,
            &principal,
            ResourceType::Tag,
            Some(tag.id),
            &headers,
        )
        .await?;
        tags.push(AuthorizedTagView { tag, capabilities });
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
    summary = "Create a resource tag",
    request_body = NewTag,
    responses(
        (status = 200, description = "Success", body = citadel_resources::TagView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_tag(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<NewTag>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(mut input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize_global(
        &state,
        &principal,
        ResourceType::Tag,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    identity_result(input.validate().map_err(metadata_error), &headers)?;
    let tag = identity_result(
        state
            .resources
            .store()
            .create_tag(principal.actor_id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Tag", "tagChanged");
    Ok(no_store(Json(tag).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/tags/{id}",
    operation_id = "patchTag",
    summary = "Update a resource tag",
    request_body = TagPatch,
    responses(
        (status = 200, description = "Success", body = citadel_resources::TagView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn patch_tag(
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<TagPatch>, JsonRejection>,
) -> IdentityHttpResult {
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state,
        &principal,
        ResourceType::Tag,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let tag = identity_result(
        state
            .resources
            .store()
            .update_tag(id, &input)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Tag", "tagChanged");
    Ok(no_store(Json(tag).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/tags/{id}",
    operation_id = "deleteTag",
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
    State(state): State<ResourcesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state,
        &principal,
        ResourceType::Tag,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    identity_result(
        state
            .resources
            .store()
            .delete_tag(id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, "Tag", "tagChanged");
    Ok(no_store(axum::http::StatusCode::NO_CONTENT.into_response()))
}

macro_rules! resource_tag_handlers {
    ($(#[$get_attr:meta])* $get:ident, $(#[$replace_attr:meta])* $replace:ident, $tag_type:expr, $resource_type:expr) => {
        $(#[$get_attr])*
        async fn $get(
            State(state): State<ResourcesHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            resource_tags(state, principal, path, headers, $tag_type, $resource_type).await
        }

        $(#[$replace_attr])*
        async fn $replace(
            State(state): State<ResourcesHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
            input: Result<Json<ReplaceResourceTagsInput>, JsonRejection>,
        ) -> IdentityHttpResult {
            let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
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
    summary = "Get Platform tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace Platform tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get Registry tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace Registry tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get Git repository tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace Git repository tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get AutomationAction tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace AutomationAction tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get BuildProject tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace BuildProject tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get BuildAgentPool tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace BuildAgentPool tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get BackupPolicy tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace BackupPolicy tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get Deployment tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace Deployment tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get Stack tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace Stack tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Get SwarmService tags",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    summary = "Replace SwarmService tags",
    request_body = ref("#/components/schemas/ReplaceResourceTagsInput"),
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/ResourceTagsView"), content_type = "application/json"),
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
    state: ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    tag_type: TaggableResourceType,
    resource_type: ResourceType,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state,
        &principal,
        resource_type,
        id,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let tags = identity_result(
        state
            .resources
            .store()
            .get_resource_tags(tag_type, id)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ResourceTagsResponse { tags }).into_response(),
    ))
}

async fn replace_resource_tags(
    state: ResourcesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: ReplaceResourceTagsInput,
    tag_type: TaggableResourceType,
    resource_type: ResourceType,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(
        path.map_err(|error| invalid_path(error.to_string())),
        &headers,
    )?;
    authorize_resource(
        &state,
        &principal,
        resource_type,
        id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let tags = identity_result(
        state
            .resources
            .store()
            .replace_resource_tags(principal.actor_id, tag_type, id, &input.tag_ids)
            .await
            .map_err(metadata_error),
        &headers,
    )?;
    publish_resource_change(&state, tag_type.as_database_str(), "resourceTagsChanged");
    Ok(no_store(
        Json(ResourceTagsResponse { tags }).into_response(),
    ))
}

fn invalid_path(message: String) -> citadel_identity::IdentityError {
    citadel_identity::IdentityError::Validation(message)
}

pub(crate) fn parse_filters(
    query: Option<&str>,
) -> Result<Vec<String>, citadel_identity::IdentityError> {
    let mut tags = Vec::new();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key != "tags" {
            continue;
        }
        if tags.len() >= 100 || value.len() > 128 || value.trim().is_empty() {
            return Err(citadel_identity::IdentityError::Validation(
                "Tag filters must contain at most 100 non-empty tag names or IDs.".into(),
            ));
        }
        tags.push(value.into_owned());
    }
    Ok(tags)
}

pub(crate) fn matches_filters(tags: &[TagSummary], filters: &[String]) -> bool {
    filters.iter().all(|filter| {
        tags.iter().any(|tag| {
            tag.name.eq_ignore_ascii_case(filter)
                || Uuid::parse_str(filter).is_ok_and(|id| id == tag.id)
        })
    })
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<ResourcesHttpState> {
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
