use super::*;

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::builds_http::Runs)]
#[serde(rename_all = "camelCase")]
pub(super) struct Runs {
    pub(super) runs: Vec<BuildRunView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::builds_http::Logs)]
#[serde(rename_all = "camelCase")]
pub(super) struct Logs {
    pub(super) run_id: Uuid,
    pub(super) logs: Vec<BuildLogEntry>,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[schema(as = server::builds_http::QueueInput)]
#[serde(rename_all = "camelCase")]
pub(super) struct QueueInput {
    pub(super) trigger: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunFilter {
    pub(super) project_id: Option<Uuid>,
    pub(super) limit: Option<usize>,
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
pub(super) async fn queue_run(
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
                Some(citadel_primitives::SpecificPermission::Apply),
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
pub(super) async fn list_runs(
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
pub(super) async fn get_run(
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
pub(super) async fn get_logs(
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
pub(super) async fn cancel_run(
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
                Some(citadel_primitives::SpecificPermission::Apply),
            )
            .await,
        &headers,
    )?;
    identity_result(state.builds.cancel(id).await.map_err(map_error), &headers)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
