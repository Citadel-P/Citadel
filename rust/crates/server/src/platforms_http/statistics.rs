use super::*;
use citadel_adapters::statistics_read_store::PostgresStatisticsReadStore;
use citadel_platforms::{StatisticsReadStore, StatisticsWorkload, StatsWindow};

#[derive(Serialize)]
struct History<T> {
    stats: Vec<T>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct TaskHistory {
    container_projection_id: Uuid,
    docker_container_id: String,
    stats: Vec<citadel_platforms::ContainerStatView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContainerHistory {
    container_id: String,
    container_name: String,
    stats: Vec<citadel_platforms::ContainerStatView>,
}
#[derive(Serialize)]
struct StackHistory {
    containers: Vec<ContainerHistory>,
}

#[derive(Deserialize)]
pub(super) struct Hours {
    #[serde(default = "default_hours", alias = "Hours")]
    hours: u16,
}
const fn default_hours() -> u16 {
    24
}
fn window(
    query: Result<Query<Hours>, QueryRejection>,
    headers: &HeaderMap,
) -> IdentityHttpResult<StatsWindow> {
    let Query(query) = identity_result(query.map_err(invalid_query), headers)?;
    identity_result(
        StatsWindow::new(query.hours)
            .ok_or_else(|| IdentityError::Validation("Hours must be 24, 48, or 72.".into())),
        headers,
    )
}
fn storage(error: RuntimeCapabilityError) -> IdentityError {
    IdentityError::Storage(error.to_string())
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
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("hours" = Option<crate::openapi::compatibility::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn task(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    use citadel_platforms::SwarmTaskRuntimePort;
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    identity_result(validate_docker_resource_id(&id), &headers)?;
    authorize_platform(&state, &principal, platform_id, &headers).await?;
    let platform = required(
        state
            .platforms
            .get_platform(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return identity_result(
            Err(IdentityError::Validation(
                "Platform must be Docker Swarm.".into(),
            )),
            &headers,
        );
    }
    if platform.status != "Online" {
        return identity_result(
            Err(IdentityError::Conflict(
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
    let live = match runtime_for(&state, platform_id).await {
        Ok(RuntimeRef::Local(runtime)) => runtime.inspect_task(&id, &cancellation).await,
        Ok(RuntimeRef::Agent(runtime)) => runtime.inspect_task(&id, &cancellation).await,
        Ok(RuntimeRef::Edge(runtime)) => runtime.inspect_task(&id, &cancellation).await,
        Err(error) => Err(error),
    };
    let live = match live {
        Ok(live) => live,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let docker_id = match citadel_platforms::validate_running_task(&projection, &live) {
        Ok(id) => id,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let store = PostgresStatisticsReadStore::new(state.pool.clone());
    let target = match store
        .task_container(platform_id, &live.node_id, docker_id)
        .await
    {
        Ok(target) => target,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let Some(target) = target else {
        return identity_result(
            Err(IdentityError::Conflict(
                "Task Container projection has not synchronized.".into(),
            )),
            &headers,
        );
    };
    let stats = identity_result(
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
            stats,
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
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceStatsView"), content_type = "application/json"),
        crate::openapi::errors::ExternalRuntimeErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("hours" = Option<crate::openapi::compatibility::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn service(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    identity_result(validate_docker_resource_id(&id), &headers)?;
    authorize_platform(&state, &principal, platform_id, &headers).await?;
    let platform = required(
        state
            .platforms
            .get_platform(platform_id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return identity_result(
            Err(IdentityError::Validation(
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
    let store = PostgresStatisticsReadStore::new(state.pool.clone());
    let now = chrono::Utc::now().timestamp();
    let tasks = match store.service_current_tasks(platform_id, &id, now).await {
        Ok(tasks) => tasks,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    let stats = identity_result(
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
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerStatsView"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = String, Path), ("hours" = Option<crate::openapi::compatibility::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn container(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(reference) = identity_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    identity_result(
        if Uuid::parse_str(&reference).is_ok()
            || (reference.len() >= 12
                && reference.len() <= 64
                && reference.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            Ok(())
        } else {
            Err(IdentityError::Validation(
                "Must be a valid container id".into(),
            ))
        },
        &headers,
    )?;
    let store = PostgresStatisticsReadStore::new(state.pool.clone());
    let target = match store.find_container(&reference).await {
        Ok(target) => required(Ok(target), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    authorize_platform(&state, &principal, target.platform_id, &headers).await?;
    let stats = identity_result(
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
        (status = 200, description = "Success", body = ref("#/components/schemas/PlatformStatsView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("hours" = Option<crate::openapi::compatibility::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn platform(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    authorize_platform(&state, &principal, id, &headers).await?;
    required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let store = PostgresStatisticsReadStore::new(state.pool.clone());
    let stats = identity_result(
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
        (status = 200, description = "Success", body = ref("#/components/schemas/ContainerStatsView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("hours" = Option<crate::openapi::compatibility::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn deployment(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
        (status = 200, description = "Success", body = ref("#/components/schemas/StackStatsView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path), ("hours" = Option<crate::openapi::compatibility::StatsHours>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn stack(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let window = window(query, &headers)?;
    match workload {
        StatisticsWorkload::Deployment => identity_result(
            state
                .identity
                .require_resource::<citadel_deployments::permissions::ReadDeployment>(
                    &principal, id,
                )
                .await,
            &headers,
        )?,
        StatisticsWorkload::Stack if !principal.is_administrator() => identity_result(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::ReadStack>(&principal, id)
                .await,
            &headers,
        )?,
        StatisticsWorkload::Stack => {}
    }
    let store = PostgresStatisticsReadStore::new(state.pool.clone());
    let containers = match store.workload_containers(workload, id).await {
        Ok(containers) => required(Ok(containers), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    if matches!(workload, StatisticsWorkload::Deployment) && containers.is_empty() {
        return identity_result(Err(IdentityError::NotFound), &headers);
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
            stats: grouped.remove(&c.id).unwrap_or_default(),
        })
        .collect();
    Ok(no_store(Json(StackHistory { containers }).into_response()))
}
