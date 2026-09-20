//! Builds HTTP routes, authorization and local request handling.
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::{
            builds::{
                capabilities::{granted, pool_capabilities, project_capabilities},
                requests::{QueueInput, RenamePool, RunFilter, *},
                views::{AuthorizedPool, AuthorizedProject, Logs, Pools, Projects, Runs, *},
            },
            capabilities::ResourceCapabilitiesView,
        },
    },
    openapi::router::OpenApiRouterExt,
    request_validation::{ApiPath, ApiQuery, ValidatedJson},
};

use axum::{
    Json, Router,
    extract::{Extension, RawQuery, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_builds::{BuildError, BuildProjectConfiguration, BuildService};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{EffectivePermission, PermissionLevel, ResourceType};

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct BuildsHttpState {
    pub identity: Arc<IdentityService>,
    pub builds: Arc<BuildService>,
}

pub fn router(state: BuildsHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes()
            .split_for_parts()
            .0
            .with_state(state.clone()),
        "Build",
    )
    .merge(crate::realtime::notify_mutations(
        documented_pool_routes()
            .split_for_parts()
            .0
            .with_state(state),
        "BuildAgentPool",
    ))
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<BuildsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_projects))
        .normalized_routes(utoipa_axum::routes!(create_project))
        .normalized_routes(utoipa_axum::routes!(get_project))
        .normalized_routes(utoipa_axum::routes!(update_project))
        .normalized_routes(utoipa_axum::routes!(rename_project))
        .normalized_routes(utoipa_axum::routes!(update_project_metadata))
        .normalized_routes(utoipa_axum::routes!(archive_project))
        .normalized_routes(utoipa_axum::routes!(queue_run))
        .normalized_routes(utoipa_axum::routes!(list_runs))
        .normalized_routes(utoipa_axum::routes!(get_run))
        .normalized_routes(utoipa_axum::routes!(get_logs))
        .normalized_routes(utoipa_axum::routes!(cancel_run))
}

pub(crate) fn documented_pool_routes() -> utoipa_axum::router::OpenApiRouter<BuildsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_pools))
        .normalized_routes(utoipa_axum::routes!(create_pool))
        .normalized_routes(utoipa_axum::routes!(get_pool))
        .normalized_routes(utoipa_axum::routes!(update_pool))
        .normalized_routes(utoipa_axum::routes!(rename_pool))
        .normalized_routes(utoipa_axum::routes!(update_pool_metadata))
        .normalized_routes(utoipa_axum::routes!(test_pool))
        .normalized_routes(utoipa_axum::routes!(archive_pool))
        .normalized_routes(utoipa_axum::routes!(enroll_pool))
        .normalized_routes(utoipa_axum::routes!(pool_edge_status))
        .normalized_routes(utoipa_axum::routes!(revoke_pool_edge))
}

fn actor(
    value: Option<Extension<ActorPrincipal>>,
    headers: &HeaderMap,
) -> HttpResult<ActorPrincipal> {
    api_result(
        value
            .map(|Extension(value)| value)
            .ok_or(ApiError::Unauthenticated),
        headers,
    )
}

async fn authorize_global(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    authorize_global_for(state, principal, ResourceType::Build, level, headers).await
}

async fn authorize_global_for(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        match (resource_type, level) {
            (ResourceType::Build, PermissionLevel::Read) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ReadBuild>(principal)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Write) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::WriteBuild>(principal)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ExecuteBuild>(principal)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Read) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ReadBuildAgentPool>(principal)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Write) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::WriteBuildAgentPool>(principal)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ExecuteBuildAgentPool>(principal)
                    .await
            }
            _ => {
                state
                    .identity
                    .authorize(principal, resource_type, level, None)
                    .await
            }
        },
        headers,
    )?;
    Ok(())
}

async fn authorize(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    authorize_for(state, principal, ResourceType::Build, id, level, headers).await
}

async fn authorize_for(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        match (resource_type, level) {
            (ResourceType::Build, PermissionLevel::Read) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ReadBuild>(principal, id)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Write) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::WriteBuild>(principal, id)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ExecuteBuild>(principal, id)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Read) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ReadBuildAgentPool>(
                        principal, id,
                    )
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Write) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::WriteBuildAgentPool>(
                        principal, id,
                    )
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ExecuteBuildAgentPool>(
                        principal, id,
                    )
                    .await
            }
            _ => {
                state
                    .identity
                    .authorize_resource(principal, resource_type, id, level, None)
                    .await
            }
        },
        headers,
    )?;
    Ok(())
}

fn map_error(error: BuildError) -> ApiError {
    match error {
        BuildError::LicenseRequired(capability) => {
            ApiError::LicenseRequired(capability.as_license_key())
        }
        BuildError::Validation(message) => ApiError::Validation(message),
        BuildError::NotFound => ApiError::NotFound,
        BuildError::Conflict(message) => ApiError::Conflict(message),
        source @ BuildError::Storage(_) => ApiError::internal(source),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools",
    operation_id = "listBuildAgentPools",
    tag = "BuildAgentPools",
    summary = "List Build Agent Pools",
    responses(
        (status = 200, description = "Success", body = Pools, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_pools(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    let tags = api_result(
        crate::api::routes::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut build_agent_pools = api_result(
        state
            .builds
            .store()
            .list_pools(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    build_agent_pools.retain(|pool| crate::api::routes::tags::matches_filters(&pool.tags, &tags));
    let pools = api_result(
        authorized_pools(state.builds.store().as_ref(), &principal, build_agent_pools)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let capabilities = pool_permissions(&state, &principal, None, &headers).await?;
    Ok(no_store(
        Json(Pools {
            pools,
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools",
    operation_id = "createBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Create a Build Agent Pool",
    request_body = BuildAgentPoolInput,
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<BuildAgentPoolInput>,
) -> HttpResult {
    let mut input: citadel_builds::BuildAgentPoolConfiguration = input.into();
    let principal = actor(principal, &headers)?;
    authorize_global_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    api_result(input.validate().map_err(map_error), &headers)?;
    let pool = api_result(
        state
            .builds
            .store()
            .create_pool(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "getBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Get a Build Agent Pool",
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let pool = api_result(
        state.builds.store().get_pool(id).await.map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/test",
    operation_id = "testBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Test a Build Agent Pool",
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn test_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let pool = api_result(
        state
            .builds
            .test_pool(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "updateBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Update a Build Agent Pool",
    request_body(content(
        (ref("#/components/schemas/UpdateBuildAgentPoolInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBuildAgentPoolInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> HttpResult {
    save_pool(&state, principal, id, patch, None, false, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildAgentPools/{id}/_metadata",
    operation_id = "updateBuildAgentPoolMetadata",
    tag = "BuildAgentPools",
    summary = "Update Build Agent Pool metadata",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_pool_metadata(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> HttpResult {
    save_pool(&state, principal, id, patch, None, true, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/rename",
    operation_id = "renameBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Rename a Build Agent Pool",
    request_body = RenamePool,
    responses(
        (status = 200, description = "Success", body = BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenamePool>,
) -> HttpResult {
    save_pool(
        &state,
        principal,
        input.id,
        serde_json::json!({}),
        Some(input.name),
        true,
        &headers,
    )
    .await
}

async fn save_pool(
    state: &BuildsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    patch: serde_json::Value,
    name: Option<String>,
    metadata_only: bool,
    headers: &HeaderMap,
) -> HttpResult {
    let principal = actor(principal, headers)?;
    authorize_for(
        state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        headers,
    )
    .await?;
    let current = api_result(
        state.builds.store().get_pool(id).await.map_err(map_error),
        headers,
    )?;
    if current.archived_at.is_some() {
        return api_result(Err(ApiError::NotFound), headers);
    }
    let mut input = api_result(
        current.apply_patch(patch, metadata_only).map_err(map_error),
        headers,
    )?;
    if let Some(name) = name {
        input.name = name;
    }
    api_result(input.validate().map_err(map_error), headers)?;
    let pool = api_result(
        state
            .builds
            .store()
            .update_pool(&current, &input, principal.actor_id)
            .await
            .map_err(map_error),
        headers,
    )?;
    pool_response(state, &principal, pool, headers).await
}

pub(crate) async fn authorized_pools(
    store: &dyn citadel_builds::BuildRepository,
    principal: &ActorPrincipal,
    values: Vec<citadel_builds::BuildAgentPool>,
) -> Result<Vec<AuthorizedPool>, BuildError> {
    let permissions = if principal.is_administrator() || values.is_empty() {
        Default::default()
    } else {
        let ids = values.iter().map(|pool| pool.id).collect::<Vec<_>>();
        store.pool_permissions(principal.actor_id, &ids).await?
    };
    Ok(values
        .into_iter()
        .map(|pool| AuthorizedPool {
            capabilities: pool_capabilities(if principal.is_administrator() {
                EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&pool.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            }),
            pool: pool.into(),
        })
        .collect())
}

async fn pool_permissions(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    id: Option<Uuid>,
    headers: &HeaderMap,
) -> HttpResult<ResourceCapabilitiesView> {
    if principal.is_administrator() {
        return Ok(pool_capabilities(EffectivePermission::Administrator));
    }
    let permission = match id {
        Some(id) => {
            state
                .identity
                .permission_for_resource(principal, ResourceType::BuildAgentPool, id)
                .await
        }
        None => {
            state
                .identity
                .global_permission(principal, ResourceType::BuildAgentPool)
                .await
        }
    };
    Ok(pool_capabilities(granted(
        api_result(permission, headers)?.map_or(PermissionLevel::None, |grant| grant.level),
    )))
}

async fn pool_response(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    pool: citadel_builds::BuildAgentPool,
    headers: &HeaderMap,
) -> HttpResult {
    let capabilities = pool_permissions(state, principal, Some(pool.id), headers).await?;
    Ok(no_store(
        Json(AuthorizedPool {
            pool: pool.into(),
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "archiveBuildAgentPool",
    tag = "BuildAgentPools",
    summary = "Archive a Build Agent Pool",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn archive_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    api_result(
        state
            .builds
            .store()
            .archive_pool(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/edge/enrollments",
    operation_id = "createBuildAgentPoolEdgeEnrollment",
    tag = "BuildAgentPools",
    summary = "Create build pool Edge Agent enrollment",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/EdgeAgentEnrollmentView"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn enroll_pool(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::api::routes::platforms::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    edge.enrollment(
        citadel_adapters::connectors::edge::EdgeTarget::build_pool(id),
        principal.actor_id.value(),
        &headers,
    )
    .await
}

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools/{id}/edge/status",
    operation_id = "getBuildAgentPoolEdgeStatus",
    tag = "BuildAgentPools",
    summary = "Get build pool Edge Agent status",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/EdgeAgentStatusView"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn pool_edge_status(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::api::routes::platforms::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let status = api_result(
        edge.store
            .status(&citadel_adapters::connectors::edge::EdgeTarget::build_pool(
                id,
            ))
            .await
            .map_err(crate::api::routes::platforms::EdgeHttpContext::error),
        &headers,
    )?;
    Ok(no_store(Json(status).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/edge/revoke",
    operation_id = "revokeBuildAgentPoolEdgeAgent",
    tag = "BuildAgentPools",
    summary = "Revoke build pool Edge Agent",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn revoke_pool_edge(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::api::routes::platforms::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    let target = citadel_adapters::connectors::edge::EdgeTarget::build_pool(id);
    api_result(
        edge.store
            .status(&target)
            .await
            .map_err(crate::api::routes::platforms::EdgeHttpContext::error),
        &headers,
    )?;
    api_result(
        edge.store
            .revoke(&target)
            .await
            .map_err(crate::api::routes::platforms::EdgeHttpContext::error),
        &headers,
    )?;
    edge.registry.disconnect(&target);
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

pub(crate) async fn authorized_projects(
    store: &dyn citadel_builds::BuildRepository,
    principal: &ActorPrincipal,
    projects: Vec<citadel_builds::BuildProject>,
) -> Result<Vec<AuthorizedProject>, BuildError> {
    let ids: Vec<_> = projects.iter().map(|project| project.id).collect();
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        store.project_permissions(principal.actor_id, &ids).await?
    };
    Ok(projects
        .into_iter()
        .map(|project| {
            let level = if principal.is_administrator() {
                EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&project.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            };
            AuthorizedProject {
                project: project.into(),
                capabilities: project_capabilities(level),
            }
        })
        .collect())
}

async fn project_response(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    project: citadel_builds::BuildProject,
    headers: &HeaderMap,
) -> HttpResult {
    let mut projects = api_result(
        authorized_projects(state.builds.store().as_ref(), principal, vec![project])
            .await
            .map_err(map_error),
        headers,
    )?;
    Ok(no_store(Json(projects.remove(0)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/buildProjects",
    operation_id = "listBuildProjects",
    tag = "BuildProjects",
    summary = "List Build Projects",
    responses(
        (status = 200, description = "Success", body = Projects, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_projects(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    let tags = api_result(
        crate::api::routes::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut projects = api_result(
        state
            .builds
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    projects.retain(|project| crate::api::routes::tags::matches_filters(&project.tags, &tags));
    let projects = api_result(
        authorized_projects(state.builds.store().as_ref(), &principal, projects)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let permission = api_result(
        state
            .identity
            .global_permission(&principal, ResourceType::Build)
            .await,
        &headers,
    )?;
    let capabilities = project_capabilities(if principal.is_administrator() {
        EffectivePermission::Administrator
    } else {
        granted(permission.map_or(PermissionLevel::None, |grant| grant.level))
    });
    Ok(no_store(
        Json(Projects {
            projects,
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/buildProjects",
    operation_id = "createBuildProject",
    tag = "BuildProjects",
    summary = "Create a Build Project",
    request_body = BuildProjectInput,
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<BuildProjectInput>,
) -> HttpResult {
    let mut input: citadel_builds::BuildProjectConfiguration = input.into();
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    api_result(input.validate().map_err(map_error), &headers)?;
    authorize_build_dependencies(&state, &principal, &input, &headers).await?;
    validate_configuration_entitlements(&state, None, &input, true, &headers).await?;
    let project = api_result(
        state
            .builds
            .store()
            .create(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    project_response(&state, &principal, project, &headers).await
}

async fn validate_configuration_entitlements(
    state: &BuildsHttpState,
    current: Option<&citadel_builds::BuildProject>,
    input: &BuildProjectConfiguration,
    updates_webhook: bool,
    headers: &HeaderMap,
) -> HttpResult<()> {
    if input.builder_kind == "BuildAgentPool"
        && current.is_none_or(|value| {
            value.builder_kind != input.builder_kind
                || value.build_agent_pool_id != input.build_agent_pool_id
        })
    {
        api_result(
            state
                .builds
                .ensure_entitled(citadel_licensing::LicenseCapability::ElasticBuildExecution)
                .await
                .map_err(map_error),
            headers,
        )?;
    }
    if updates_webhook
        && input
            .webhook
            .as_ref()
            .and_then(|value| value.get("enabled"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    {
        api_result(
            state
                .builds
                .ensure_entitled(citadel_licensing::LicenseCapability::AutomatedOperations)
                .await
                .map_err(map_error),
            headers,
        )?;
    }
    Ok(())
}

async fn authorize_build_dependencies(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    input: &BuildProjectConfiguration,
    headers: &HeaderMap,
) -> HttpResult<()> {
    authorize_for(
        state,
        principal,
        ResourceType::GitRepository,
        input.git_repository_id,
        PermissionLevel::Read,
        headers,
    )
    .await?;
    authorize_for(
        state,
        principal,
        ResourceType::Registry,
        input.registry_id,
        PermissionLevel::Read,
        headers,
    )
    .await?;
    if let Some(platform_id) = input.platform_id {
        authorize_for(
            state,
            principal,
            ResourceType::Platform,
            platform_id,
            PermissionLevel::Read,
            headers,
        )
        .await?;
    }
    if let Some(pool_id) = input.build_agent_pool_id {
        authorize_for(
            state,
            principal,
            ResourceType::BuildAgentPool,
            pool_id,
            PermissionLevel::Read,
            headers,
        )
        .await?;
    }
    if input
        .build_secrets
        .as_ref()
        .is_some_and(|secrets| !secrets.is_empty())
    {
        authorize_global_for(
            state,
            principal,
            ResourceType::Binding,
            PermissionLevel::Read,
            headers,
        )
        .await?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "getBuildProject",
    tag = "BuildProjects",
    summary = "Get a Build Project",
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let project = api_result(
        state.builds.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    project_response(&state, &principal, project, &headers).await
}

#[utoipa::path(
    delete,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "archiveBuildProject",
    tag = "BuildProjects",
    summary = "Archive a Build Project",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn archive_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    api_result(
        state
            .builds
            .store()
            .archive(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "updateBuildProject",
    tag = "BuildProjects",
    summary = "Update Build Project",
    request_body(content(
        (ref("#/components/schemas/UpdateBuildProjectInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/UpdateBuildProjectInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> HttpResult {
    save_project(&state, principal, id, patch, None, false, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildProjects/{id}/_metadata",
    operation_id = "updateBuildMetadata",
    tag = "BuildProjects",
    summary = "Update Build Project",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_project_metadata(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> HttpResult {
    save_project(&state, principal, id, patch, None, true, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/buildProjects/rename",
    operation_id = "renameBuild",
    tag = "BuildProjects",
    summary = "Update Build Project",
    request_body = RenamePool,
    responses(
        (status = 200, description = "Success", body = BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenamePool>,
) -> HttpResult {
    save_project(
        &state,
        principal,
        input.id,
        serde_json::json!({}),
        Some(input.name),
        false,
        &headers,
    )
    .await
}

async fn save_project(
    state: &BuildsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    patch: serde_json::Value,
    name: Option<String>,
    metadata_only: bool,
    headers: &HeaderMap,
) -> HttpResult {
    let updates_webhook = patch.get("webhook").is_some();
    let principal = actor(principal, headers)?;
    authorize(state, &principal, id, PermissionLevel::Write, headers).await?;
    let current = api_result(
        state.builds.store().get(id).await.map_err(map_error),
        headers,
    )?;
    let mut input = api_result(
        current.apply_patch(patch, metadata_only).map_err(map_error),
        headers,
    )?;
    if let Some(name) = name {
        input.name = name;
    }
    api_result(input.validate().map_err(map_error), headers)?;
    if !metadata_only {
        authorize_build_dependencies(state, &principal, &input, headers).await?;
        validate_configuration_entitlements(
            state,
            Some(&current),
            &input,
            updates_webhook,
            headers,
        )
        .await?;
    }
    let project = api_result(
        state
            .builds
            .store()
            .update(&current, &input, principal.actor_id, metadata_only)
            .await
            .map_err(map_error),
        headers,
    )?;
    project_response(state, &principal, project, headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/buildProjects/{id}/runs",
    operation_id = "queueBuildRun",
    tag = "BuildProjects",
    summary = "Queue a Build Run",
    request_body = Option<QueueInput>,
    responses(
        (status = 200, description = "Success", body = BuildRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn queue_run(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<QueueInput>>,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    api_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Build,
                id,
                PermissionLevel::Read,
                Some(citadel_primitives::SpecificPermission::Apply),
            )
            .await,
        &headers,
    )?;
    let trigger = input
        .and_then(|ValidatedJson(value)| value.trigger)
        .unwrap_or_else(|| "Manual".to_owned());
    if !matches!(trigger.as_str(), "Manual" | "Webhook" | "Dependency") {
        return Err(crate::api::error::HttpError::from_parts(
            ApiError::Validation("Build trigger is invalid.".to_owned()),
            &headers,
        ));
    }
    let project = api_result(
        state.builds.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    api_result(
        state
            .builds
            .ensure_execution_entitlements(&project, &trigger)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let run = api_result(
        state
            .builds
            .store()
            .enqueue(principal.actor_id, id, &trigger)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(BuildRunView::from(run)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/buildRuns",
    operation_id = "listBuildRuns",
    tag = "BuildRuns",
    summary = "List Build Runs",
    responses(
        (status = 200, description = "Success", body = Runs, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("projectId" = Option<uuid::Uuid>, Query), ("limit" = Option<i32>, Query, minimum = 1, maximum = 100, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_runs(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiQuery(filter): ApiQuery<RunFilter>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    if let Some(id) = filter.project_id {
        authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    }
    let runs = api_result(
        state
            .builds
            .store()
            .list_runs(
                principal.actor_id,
                principal.is_administrator(),
                filter.project_id,
                filter.limit.unwrap_or(50),
            )
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(
        Json(Runs {
            runs: runs.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/buildRuns/{id}",
    operation_id = "getBuildRun",
    tag = "BuildRuns",
    summary = "Get a Build Run",
    responses(
        (status = 200, description = "Success", body = BuildRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_run(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    let run = api_result(
        state.builds.store().get_run(id).await.map_err(map_error),
        &headers,
    )?;
    authorize(
        &state,
        &principal,
        run.build_project_id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    Ok(no_store(Json(BuildRunView::from(run)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/buildRuns/{id}/logs",
    operation_id = "getBuildRunLogs",
    tag = "BuildRuns",
    summary = "Get Build Run logs",
    responses(
        (status = 200, description = "Success", body = Logs, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_logs(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    let run = api_result(
        state.builds.store().get_run(id).await.map_err(map_error),
        &headers,
    )?;
    authorize(
        &state,
        &principal,
        run.build_project_id,
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let logs = api_result(
        state.builds.store().logs(id).await.map_err(map_error),
        &headers,
    )?;
    Ok(no_store(
        Json(Logs {
            run_id: id,
            logs: logs.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/buildRuns/{id}/cancel",
    operation_id = "cancelBuildRun",
    tag = "BuildRuns",
    summary = "Cancel a Build Run",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn cancel_run(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    let run = api_result(
        state.builds.store().get_run(id).await.map_err(map_error),
        &headers,
    )?;
    api_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Build,
                run.build_project_id,
                PermissionLevel::Read,
                Some(citadel_primitives::SpecificPermission::Apply),
            )
            .await,
        &headers,
    )?;
    api_result(state.builds.cancel(id).await.map_err(map_error), &headers)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
