use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
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
    projects: Vec<citadel_builds::BuildProjectView>,
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
    logs: Vec<citadel_builds::BuildLog>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Pools {
    build_agent_pools: Vec<citadel_builds::BuildAgentPoolView>,
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
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let projects = identity_result(
        state
            .builds
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(Projects { projects }).into_response()))
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
    let project = identity_result(
        state
            .builds
            .store()
            .create(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(project).into_response()))
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
    Ok(no_store(Json(project).into_response()))
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
        state.builds.store().archive(id).await.map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn queue_run(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    input: Option<Json<QueueInput>>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let trigger = input
        .and_then(|Json(value)| value.trigger)
        .unwrap_or_else(|| "Manual".to_owned());
    if !matches!(trigger.as_str(), "Manual" | "Webhook" | "Dependency") {
        return Err(crate::identity_http::IdentityHttpError::from_parts(
            IdentityError::Validation("Build trigger is invalid.".to_owned()),
            &headers,
        ));
    }
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
    authorize(
        &state,
        &principal,
        run.build_project_id,
        PermissionLevel::Execute,
        &headers,
    )
    .await?;
    identity_result(state.builds.cancel(id).await.map_err(map_error), &headers)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
async fn list_pools(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let build_agent_pools = identity_result(
        state
            .builds
            .store()
            .list_pools(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(Pools { build_agent_pools }).into_response()))
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
    Ok(no_store(Json(pool).into_response()))
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
    Ok(no_store(Json(pool).into_response()))
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
            .archive_pool(id)
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
        PermissionLevel::Write,
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
        BuildError::Validation(message) => IdentityError::Validation(message),
        BuildError::NotFound => IdentityError::NotFound,
        BuildError::Conflict(message) => IdentityError::Conflict(message),
        BuildError::Storage(message) => IdentityError::Storage(message),
    }
}
