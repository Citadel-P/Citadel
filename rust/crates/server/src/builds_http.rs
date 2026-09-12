use crate::request_validation::ApiPath;
use crate::request_validation::ApiQuery;
use crate::request_validation::ValidatedJson;
use std::sync::Arc;

use axum::extract::{Extension, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_builds::{BuildAgentPoolInput, BuildError, BuildProjectInput, BuildService};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::openapi::router::OpenApiRouterExt;

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

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Projects {
    projects: Vec<AuthorizedProject>,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}
#[derive(Serialize, utoipa::ToSchema)]
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
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::builds_http::Runs)]
#[serde(rename_all = "camelCase")]
struct Runs {
    runs: Vec<citadel_builds::BuildRunView>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::builds_http::Logs)]
#[serde(rename_all = "camelCase")]
struct Logs {
    run_id: Uuid,
    logs: Vec<citadel_builds::BuildLogEntry>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Pools {
    pools: Vec<AuthorizedPool>,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AuthorizedPool {
    #[serde(flatten)]
    pool: citadel_builds::BuildAgentPoolView,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}
#[derive(Deserialize, Default, utoipa::ToSchema)]
#[schema(as = server::builds_http::QueueInput)]
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

#[utoipa::path(
    get,
    path = "/api/v1/buildProjects",
    operation_id = "listBuildProjects",
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
#[utoipa::path(
    post,
    path = "/api/v1/buildProjects",
    operation_id = "createBuildProject",
    summary = "Create a Build Project",
    request_body = BuildProjectInput,
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildProjectView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_project(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(mut input): ValidatedJson<BuildProjectInput>,
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
#[utoipa::path(
    get,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "getBuildProject",
    summary = "Get a Build Project",
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildProjectView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let project = identity_result(
        state.builds.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    project_response(&state, &principal, project, &headers).await
}
#[utoipa::path(
    delete,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "archiveBuildProject",
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

#[utoipa::path(
    patch,
    path = "/api/v1/buildProjects/{id}",
    operation_id = "updateBuildProject",
    summary = "Update Build Project",
    request_body = ref("#/components/schemas/UpdateBuildProjectInput"),
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildProjectView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    save_project(&state, principal, id, patch, None, false, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildProjects/{id}/_metadata",
    operation_id = "updateBuildMetadata",
    summary = "Update Build Project",
    request_body = ref("#/components/schemas/PatchResourceMetadata"),
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildProjectView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    save_project(&state, principal, id, patch, None, true, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/buildProjects/rename",
    operation_id = "renameBuild",
    summary = "Update Build Project",
    request_body = RenamePool,
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildProjectView, content_type = "application/json"),
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

#[utoipa::path(
    post,
    path = "/api/v1/buildProjects/{id}/runs",
    operation_id = "queueBuildRun",
    summary = "Queue a Build Run",
    request_body = Option<QueueInput>,
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildRunView, content_type = "application/json"),
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
        .and_then(|ValidatedJson(value)| value.trigger)
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
#[utoipa::path(
    get,
    path = "/api/v1/buildRuns",
    operation_id = "listBuildRuns",
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
#[utoipa::path(
    get,
    path = "/api/v1/buildRuns/{id}",
    operation_id = "getBuildRun",
    summary = "Get a Build Run",
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildRunView, content_type = "application/json"),
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
#[utoipa::path(
    get,
    path = "/api/v1/buildRuns/{id}/logs",
    operation_id = "getBuildRunLogs",
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
#[utoipa::path(
    post,
    path = "/api/v1/buildRuns/{id}/cancel",
    operation_id = "cancelBuildRun",
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
#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools",
    operation_id = "listBuildAgentPools",
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

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools",
    operation_id = "createBuildAgentPool",
    summary = "Create a Build Agent Pool",
    request_body = BuildAgentPoolInput,
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildAgentPoolView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_pool(
    State(state): State<BuildsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(mut input): ValidatedJson<BuildAgentPoolInput>,
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

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "getBuildAgentPool",
    summary = "Get a Build Agent Pool",
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildAgentPoolView, content_type = "application/json"),
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

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/test",
    operation_id = "testBuildAgentPool",
    summary = "Test a Build Agent Pool",
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildAgentPoolView, content_type = "application/json"),
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

#[utoipa::path(
    patch,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "updateBuildAgentPool",
    summary = "Update a Build Agent Pool",
    request_body = ref("#/components/schemas/UpdateBuildAgentPoolInput"),
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildAgentPoolView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    save_pool(&state, principal, id, patch, None, false, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/buildAgentPools/{id}/_metadata",
    operation_id = "updateBuildAgentPoolMetadata",
    summary = "Update Build Agent Pool metadata",
    request_body = ref("#/components/schemas/PatchResourceMetadata"),
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildAgentPoolView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    save_pool(&state, principal, id, patch, None, true, &headers).await
}

#[derive(Deserialize, utoipa::ToSchema)]
struct RenamePool {
    id: Uuid,
    name: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/rename",
    operation_id = "renameBuildAgentPool",
    summary = "Rename a Build Agent Pool",
    request_body = RenamePool,
    responses(
        (status = 200, description = "Success", body = citadel_builds::BuildAgentPoolView, content_type = "application/json"),
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

#[utoipa::path(
    delete,
    path = "/api/v1/buildAgentPools/{id}",
    operation_id = "archiveBuildAgentPool",
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

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/edge/enrollments",
    operation_id = "createBuildAgentPoolEdgeEnrollment",
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
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    get,
    path = "/api/v1/buildAgentPools/{id}/edge/status",
    operation_id = "getBuildAgentPoolEdgeStatus",
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
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    post,
    path = "/api/v1/buildAgentPools/{id}/edge/revoke",
    operation_id = "revokeBuildAgentPoolEdgeAgent",
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
    Extension(edge): Extension<crate::platforms_http::EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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
