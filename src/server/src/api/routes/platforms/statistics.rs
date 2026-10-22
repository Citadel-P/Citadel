use super::*;

pub(super) fn window(
    query: Result<Query<Hours>, QueryRejection>,
    headers: &HeaderMap,
) -> HttpResult<StatsWindow> {
    let Query(query) = api_result(query.map_err(invalid_query), headers)?;
    let now = chrono::Utc::now().timestamp();
    let selected = match (query.hours, query.minutes, query.from, query.to) {
        (None, None, None, None) => StatsWindow::new(24),
        (Some(hours), None, None, None) => StatsWindow::new(hours),
        (None, Some(minutes), None, None) => StatsWindow::minutes(minutes),
        (None, None, Some(from), to) => {
            // Today can begin in the current second; keep that open-ended range valid.
            // The window validator still bounds clock skew and retained history.
            let to = to.unwrap_or_else(|| now.max(from.saturating_add(1)));
            StatsWindow::between(from, to, now)
        }
        _ => None,
    };
    api_result(selected.ok_or_else(|| ApiError::Validation(
        "Choose minutes (15, 60, 360, 1440, 10080), legacy hours (1, 6, 24, 48, 72, 168), or from/to Unix seconds within the last 7 days. Do not mix ranges; from must precede to.".into()
    )), headers)
}

pub(super) fn storage(error: RuntimeCapabilityError) -> ApiError {
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
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query), ("minutes" = Option<u32>, Query, description = "Relative range: 15, 60, 360, 1440 or 10080 minutes"), ("from" = Option<i64>, Query, description = "Inclusive start time in Unix seconds, within retained history"), ("to" = Option<i64>, Query, description = "Inclusive end time in Unix seconds; defaults to now when from is supplied")),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn task_statistics(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform_id, id)): ApiPath<(Uuid, String)>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
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
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query), ("minutes" = Option<u32>, Query, description = "Relative range: 15, 60, 360, 1440 or 10080 minutes"), ("from" = Option<i64>, Query, description = "Inclusive start time in Unix seconds, within retained history"), ("to" = Option<i64>, Query, description = "Inclusive end time in Unix seconds; defaults to now when from is supplied")),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn service_statistics(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath((platform_id, id)): ApiPath<(Uuid, String)>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
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
    params(("id" = String, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query), ("minutes" = Option<u32>, Query, description = "Relative range: 15, 60, 360, 1440 or 10080 minutes"), ("from" = Option<i64>, Query, description = "Inclusive start time in Unix seconds, within retained history"), ("to" = Option<i64>, Query, description = "Inclusive end time in Unix seconds; defaults to now when from is supplied")),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn container(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(reference): ApiPath<String>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
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
    params(("id" = uuid::Uuid, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query), ("minutes" = Option<u32>, Query, description = "Relative range: 15, 60, 360, 1440 or 10080 minutes"), ("from" = Option<i64>, Query, description = "Inclusive start time in Unix seconds, within retained history"), ("to" = Option<i64>, Query, description = "Inclusive end time in Unix seconds; defaults to now when from is supplied")),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn platform(
    State(state): State<PlatformsHttpState>,
    AuthenticatedActor(principal): AuthenticatedActor,
    ApiPath(id): ApiPath<Uuid>,
    query: Result<Query<Hours>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
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
    params(("id" = uuid::Uuid, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query), ("minutes" = Option<u32>, Query, description = "Relative range: 15, 60, 360, 1440 or 10080 minutes"), ("from" = Option<i64>, Query, description = "Inclusive start time in Unix seconds, within retained history"), ("to" = Option<i64>, Query, description = "Inclusive end time in Unix seconds; defaults to now when from is supplied")),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn deployment(
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
    summary = "Get Stack service statistics",
    responses(
        (status = 200, description = "Success", body = StackHistory, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path), ("hours" = Option<crate::api::resources::platforms::operation_views::StatsHours>, Query), ("minutes" = Option<u32>, Query, description = "Relative range: 15, 60, 360, 1440 or 10080 minutes"), ("from" = Option<i64>, Query, description = "Inclusive start time in Unix seconds, within retained history"), ("to" = Option<i64>, Query, description = "Inclusive end time in Unix seconds; defaults to now when from is supplied")),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn stack(
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

pub(super) async fn workload(
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
    let series = match store
        .workload_history(workload, id, window, chrono::Utc::now().timestamp())
        .await
    {
        Ok(series) => required(Ok(series), &headers)?,
        Err(error) => return Ok(runtime_error_response(error, &headers)),
    };
    if matches!(workload, StatisticsWorkload::Deployment) {
        let stats = series
            .into_iter()
            .flat_map(|series| series.stats)
            .collect::<Vec<_>>();
        return Ok(no_store(Json(History { stats }).into_response()));
    }
    let services = series
        .into_iter()
        .map(|series| StackServiceHistory {
            service_name: series.name,
            container_ids: series.container_ids,
            stats: series.stats.into_iter().map(Into::into).collect(),
        })
        .collect();
    Ok(no_store(Json(StackHistory { services }).into_response()))
}
