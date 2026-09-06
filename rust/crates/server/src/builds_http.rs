use std::sync::Arc;

use axum::extract::{Extension, Path, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_builds::{BuildAgentPoolInput, BuildError, BuildProjectInput, BuildService};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

#[derive(Clone)]
pub struct BuildsHttpState {
    pub identity: Arc<IdentityService>,
    pub builds: Arc<BuildService>,
}

pub fn router(state: BuildsHttpState) -> Router {
    crate::realtime::notify_mutations(
        Router::new()
            .contract_route(routes::LIST_BUILD_PROJECTS, list_projects)
            .contract_route(routes::CREATE_BUILD_PROJECT, create_project)
            .contract_route(routes::GET_BUILD_PROJECT, get_project)
            .contract_route(routes::UPDATE_BUILD_PROJECT, update_project)
            .contract_route(routes::RENAME_BUILD_PROJECT, rename_project)
            .contract_route(
                routes::UPDATE_BUILD_PROJECT_METADATA,
                update_project_metadata,
            )
            .contract_route(routes::ARCHIVE_BUILD_PROJECT, archive_project)
            .contract_route(routes::QUEUE_BUILD_RUN, queue_run)
            .contract_route(routes::LIST_BUILD_RUNS, list_runs)
            .contract_route(routes::GET_BUILD_RUN, get_run)
            .contract_route(routes::GET_BUILD_RUN_LOGS, get_logs)
            .contract_route(routes::CANCEL_BUILD_RUN, cancel_run)
            .with_state(state.clone()),
        "Build",
    )
    .merge(crate::realtime::notify_mutations(
        Router::new()
            .contract_route(routes::LIST_BUILD_AGENT_POOLS, list_pools)
            .contract_route(routes::CREATE_BUILD_AGENT_POOL, create_pool)
            .contract_route(routes::GET_BUILD_AGENT_POOL, get_pool)
            .contract_route(routes::UPDATE_BUILD_AGENT_POOL, update_pool)
            .contract_route(routes::RENAME_BUILD_AGENT_POOL, rename_pool)
            .contract_route(
                routes::UPDATE_BUILD_AGENT_POOL_METADATA,
                update_pool_metadata,
            )
            .contract_route(routes::TEST_BUILD_AGENT_POOL, test_pool)
            .contract_route(routes::ARCHIVE_BUILD_AGENT_POOL, archive_pool)
            .contract_route(routes::CREATE_BUILD_POOL_EDGE_ENROLLMENT, enroll_pool)
            .contract_route(routes::GET_BUILD_POOL_EDGE_STATUS, pool_edge_status)
            .contract_route(routes::REVOKE_BUILD_POOL_EDGE, revoke_pool_edge)
            .with_state(state),
        "BuildAgentPool",
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Projects {
    projects: Vec<AuthorizedProject>,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}
#[derive(Serialize)]
pub(crate) struct AuthorizedProject {
    #[serde(flatten)]
    project: citadel_builds::BuildProjectView,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}

pub(crate) async fn authorized_projects(
    store: &dyn citadel_builds::BuildStore,
    principal: &ActorPrincipal,
    projects: Vec<citadel_builds::BuildProjectView>,
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
                7
            } else {
                permissions.get(&project.id).copied().unwrap_or(0)
            };
            AuthorizedProject {
                project,
                capabilities: pool_capabilities(level),
            }
        })
        .collect())
}

async fn project_response(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    project: citadel_builds::BuildProjectView,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let mut projects = identity_result(
        authorized_projects(state.builds.store().as_ref(), principal, vec![project])
            .await
            .map_err(map_error),
        headers,
    )?;
    Ok(no_store(Json(projects.remove(0)).into_response()))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Runs {
    runs: Vec<citadel_builds::BuildRunView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Logs {
    run_id: Uuid,
    logs: Vec<citadel_builds::BuildLogEntry>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Pools {
    pools: Vec<AuthorizedPool>,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}
#[derive(Serialize)]
pub(crate) struct AuthorizedPool {
    #[serde(flatten)]
    pool: citadel_builds::BuildAgentPoolView,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct QueueInput {
    trigger: Option<String>,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RunFilter {
    project_id: Option<Uuid>,
    limit: Option<usize>,
}

async fn list_projects(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let tags = identity_result(
        crate::resources_http::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut projects = identity_result(
        state
            .builds
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    projects.retain(|project| crate::resources_http::tags::matches_filters(&project.tags, &tags));
    let projects = identity_result(
        authorized_projects(state.builds.store().as_ref(), &principal, projects)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let permission = identity_result(
        state
            .identity
            .global_permission(&principal, ResourceType::Build)
            .await,
        &headers,
    )?;
    let capabilities = pool_capabilities(if principal.is_administrator() {
        7
    } else {
        permission.map_or(0, |grant| grant.level as i32)
    });
    Ok(no_store(
        Json(Projects {
            projects,
            capabilities,
        })
        .into_response(),
    ))
}
async fn create_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(mut input): Json<BuildProjectInput>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    identity_result(input.validate().map_err(map_error), &headers)?;
    authorize_build_dependencies(&state, &principal, &input, &headers).await?;
    validate_configuration_entitlements(&state, None, &input, true, &headers).await?;
    let project = identity_result(
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
    current: Option<&citadel_builds::BuildProjectView>,
    input: &BuildProjectInput,
    updates_webhook: bool,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if input.builder_kind == "BuildAgentPool"
        && current.is_none_or(|value| {
            value.builder_kind != input.builder_kind
                || value.build_agent_pool_id != input.build_agent_pool_id
        })
    {
        identity_result(
            state
                .builds
                .ensure_entitled(citadel_domain::LicenseCapability::ElasticBuildExecution)
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
        identity_result(
            state
                .builds
                .ensure_entitled(citadel_domain::LicenseCapability::AutomatedOperations)
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
    input: &BuildProjectInput,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
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
async fn get_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let project = identity_result(
        state.builds.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    project_response(&state, &principal, project, &headers).await
}
async fn archive_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    identity_result(
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

async fn update_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(patch): Json<serde_json::Value>,
) -> IdentityHttpResult {
    save_project(&state, principal, id, patch, None, false, &headers).await
}

async fn update_project_metadata(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(patch): Json<serde_json::Value>,
) -> IdentityHttpResult {
    save_project(&state, principal, id, patch, None, true, &headers).await
}

async fn rename_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<RenamePool>,
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
    let updates_webhook = patch.get("webhook").is_some();
    let principal = actor(principal, headers)?;
    authorize(state, &principal, id, PermissionLevel::Write, headers).await?;
    let current = identity_result(
        state.builds.store().get(id).await.map_err(map_error),
        headers,
    )?;
    let mut input = identity_result(
        current.apply_patch(patch, metadata_only).map_err(map_error),
        headers,
    )?;
    if let Some(name) = name {
        input.name = name;
    }
    identity_result(input.validate().map_err(map_error), headers)?;
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
    let project = identity_result(
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

async fn queue_run(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    input: Option<Json<QueueInput>>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Build,
                id,
                PermissionLevel::Read,
                Some(citadel_domain::SpecificPermission::Apply),
            )
            .await,
        &headers,
    )?;
    let trigger = input
        .and_then(|Json(value)| value.trigger)
        .unwrap_or_else(|| "Manual".to_owned());
    if !matches!(trigger.as_str(), "Manual" | "Webhook" | "Dependency") {
        return Err(crate::identity_http::IdentityHttpError::from_parts(
            IdentityError::Validation("Build trigger is invalid.".to_owned()),
            &headers,
        ));
    }
    let project = identity_result(
        state.builds.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    identity_result(
        state
            .builds
            .ensure_execution_entitlements(&project, &trigger)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let run = identity_result(
        state
            .builds
            .store()
            .enqueue(principal.actor_id, id, &trigger)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(run).into_response()))
}
async fn list_runs(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Query(filter): Query<RunFilter>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    if let Some(id) = filter.project_id {
        authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    }
    let runs = identity_result(
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
    Ok(no_store(Json(Runs { runs }).into_response()))
}
async fn get_run(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let run = identity_result(
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
    Ok(no_store(Json(run).into_response()))
}
async fn get_logs(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let run = identity_result(
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
    let logs = identity_result(
        state.builds.store().logs(id).await.map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(Logs { run_id: id, logs }).into_response()))
}
async fn cancel_run(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let run = identity_result(
        state.builds.store().get_run(id).await.map_err(map_error),
        &headers,
    )?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Build,
                run.build_project_id,
                PermissionLevel::Read,
                Some(citadel_domain::SpecificPermission::Apply),
            )
            .await,
        &headers,
    )?;
    identity_result(state.builds.cancel(id).await.map_err(map_error), &headers)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn list_pools(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let tags = identity_result(
        crate::resources_http::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut build_agent_pools = identity_result(
        state
            .builds
            .store()
            .list_pools(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    build_agent_pools
        .retain(|pool| crate::resources_http::tags::matches_filters(&pool.tags, &tags));
    let pools = identity_result(
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

async fn create_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(mut input): Json<BuildAgentPoolInput>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_global_for(
        &state,
        &principal,
        ResourceType::BuildAgentPool,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    identity_result(input.validate().map_err(map_error), &headers)?;
    let pool = identity_result(
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

async fn get_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
    let pool = identity_result(
        state.builds.store().get_pool(id).await.map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

async fn test_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
    let pool = identity_result(
        state
            .builds
            .test_pool(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    pool_response(&state, &principal, pool, &headers).await
}

async fn update_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(patch): Json<serde_json::Value>,
) -> IdentityHttpResult {
    save_pool(&state, principal, id, patch, None, false, &headers).await
}

async fn update_pool_metadata(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(patch): Json<serde_json::Value>,
) -> IdentityHttpResult {
    save_pool(&state, principal, id, patch, None, true, &headers).await
}

#[derive(Deserialize)]
struct RenamePool {
    id: Uuid,
    name: String,
}

async fn rename_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<RenamePool>,
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
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
    let current = identity_result(
        state.builds.store().get_pool(id).await.map_err(map_error),
        headers,
    )?;
    if current.archived_at.is_some() {
        return identity_result(Err(IdentityError::NotFound), headers);
    }
    let mut input = identity_result(
        current.apply_patch(patch, metadata_only).map_err(map_error),
        headers,
    )?;
    if let Some(name) = name {
        input.name = name;
    }
    identity_result(input.validate().map_err(map_error), headers)?;
    let pool = identity_result(
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
    store: &dyn citadel_builds::BuildStore,
    principal: &ActorPrincipal,
    values: Vec<citadel_builds::BuildAgentPoolView>,
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
                7
            } else {
                permissions.get(&pool.id).copied().unwrap_or(0)
            }),
            pool,
        })
        .collect())
}

fn pool_capabilities(level: i32) -> citadel_platforms::ResourceCapabilitiesView {
    citadel_platforms::ResourceCapabilitiesView {
        can_read: level >= PermissionLevel::Read as i32,
        can_write: level >= PermissionLevel::Write as i32,
        can_execute: level >= PermissionLevel::Execute as i32,
    }
}
async fn pool_permissions(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<citadel_platforms::ResourceCapabilitiesView> {
    if principal.is_administrator() {
        return Ok(pool_capabilities(7));
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
    Ok(pool_capabilities(
        identity_result(permission, headers)?.map_or(0, |grant| grant.level as i32),
    ))
}
async fn pool_response(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    pool: citadel_builds::BuildAgentPoolView,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let capabilities = pool_permissions(state, principal, Some(pool.id), headers).await?;
    Ok(no_store(
        Json(AuthorizedPool { pool, capabilities }).into_response(),
    ))
}

async fn archive_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
    identity_result(
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
fn actor(
    value: Option<Extension<ActorPrincipal>>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    identity_result(
        value
            .map(|Extension(value)| value)
            .ok_or(IdentityError::Unauthenticated),
        headers,
    )
}

async fn enroll_pool(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
        citadel_adapters::edge::EdgeTarget::build_pool(id),
        principal.actor_id.value(),
        &headers,
    )
    .await
}

async fn pool_edge_status(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
    let status = identity_result(
        edge.store
            .status(&citadel_adapters::edge::EdgeTarget::build_pool(id))
            .await
            .map_err(crate::platforms_http::EdgeHttpContext::error),
        &headers,
    )?;
    Ok(no_store(Json(status).into_response()))
}

async fn revoke_pool_edge(
    State(state): State<BuildsHttpState>,
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
    let target = citadel_adapters::edge::EdgeTarget::build_pool(id);
    identity_result(
        edge.store
            .status(&target)
            .await
            .map_err(crate::platforms_http::EdgeHttpContext::error),
        &headers,
    )?;
    identity_result(
        edge.store
            .revoke(&target)
            .await
            .map_err(crate::platforms_http::EdgeHttpContext::error),
        &headers,
    )?;
    edge.registry.disconnect(&target);
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}
async fn authorize_global(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    authorize_global_for(state, principal, ResourceType::Build, level, headers).await
}
async fn authorize_global_for(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        state
            .identity
            .authorize(principal, resource_type, level, None)
            .await,
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
) -> IdentityHttpResult<()> {
    authorize_for(state, principal, ResourceType::Build, id, level, headers).await
}
async fn authorize_for(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        state
            .identity
            .authorize_resource(principal, resource_type, id, level, None)
            .await,
        headers,
    )?;
    Ok(())
}
fn map_error(error: BuildError) -> IdentityError {
    match error {
        BuildError::LicenseRequired(capability) => {
            IdentityError::LicenseRequired(capability.as_license_key())
        }
        BuildError::Validation(message) => IdentityError::Validation(message),
        BuildError::NotFound => IdentityError::NotFound,
        BuildError::Conflict(message) => IdentityError::Conflict(message),
        BuildError::Storage(message) => IdentityError::Storage(message),
    }
}
