use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
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
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

pub(super) fn router(state: ResourcesHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_TAGS, list_tags)
        .contract_route(routes::CREATE_TAG, create_tag)
        .contract_route(routes::PATCH_TAG, patch_tag)
        .contract_route(routes::DELETE_TAG, delete_tag)
        .contract_route(routes::GET_PLATFORM_TAGS, get_platform_tags)
        .contract_route(routes::GET_AUTOMATION_ACTION_TAGS, get_action_tags)
        .contract_route(routes::REPLACE_AUTOMATION_ACTION_TAGS, replace_action_tags)
        .contract_route(routes::GET_BUILD_PROJECT_TAGS, get_build_tags)
        .contract_route(routes::REPLACE_BUILD_PROJECT_TAGS, replace_build_tags)
        .contract_route(routes::GET_BUILD_AGENT_POOL_TAGS, get_pool_tags)
        .contract_route(routes::REPLACE_BUILD_AGENT_POOL_TAGS, replace_pool_tags)
        .contract_route(routes::GET_BACKUP_POLICY_TAGS, get_policy_tags)
        .contract_route(routes::REPLACE_BACKUP_POLICY_TAGS, replace_policy_tags)
        .contract_route(routes::REPLACE_PLATFORM_TAGS, replace_platform_tags)
        .contract_route(routes::GET_REGISTRY_TAGS, get_registry_tags)
        .contract_route(routes::REPLACE_REGISTRY_TAGS, replace_registry_tags)
        .contract_route(routes::GET_GIT_REPOSITORY_TAGS, get_git_repository_tags)
        .contract_route(
            routes::REPLACE_GIT_REPOSITORY_TAGS,
            replace_git_repository_tags,
        )
        .with_state(state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TagsResponse {
    tags: Vec<AuthorizedTagView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
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
    ($get:ident, $replace:ident, $tag_type:expr, $resource_type:expr) => {
        async fn $get(
            State(state): State<ResourcesHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            resource_tags(state, principal, path, headers, $tag_type, $resource_type).await
        }

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
    get_platform_tags,
    replace_platform_tags,
    TaggableResourceType::Platform,
    ResourceType::Platform
);
resource_tag_handlers!(
    get_registry_tags,
    replace_registry_tags,
    TaggableResourceType::Registry,
    ResourceType::Registry
);
resource_tag_handlers!(
    get_git_repository_tags,
    replace_git_repository_tags,
    TaggableResourceType::GitRepository,
    ResourceType::GitRepository
);

resource_tag_handlers!(
    get_action_tags,
    replace_action_tags,
    TaggableResourceType::AutomationAction,
    ResourceType::AutomationAction
);
resource_tag_handlers!(
    get_build_tags,
    replace_build_tags,
    TaggableResourceType::Build,
    ResourceType::Build
);
resource_tag_handlers!(
    get_pool_tags,
    replace_pool_tags,
    TaggableResourceType::BuildAgentPool,
    ResourceType::BuildAgentPool
);
resource_tag_handlers!(
    get_policy_tags,
    replace_policy_tags,
    TaggableResourceType::BackupPolicy,
    ResourceType::BackupPolicy
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
