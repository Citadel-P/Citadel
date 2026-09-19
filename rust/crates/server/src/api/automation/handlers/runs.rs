use super::*;

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions/{id}/run",
    operation_id = "runAutomationAction",
    tag = "AutomationActions",
    summary = "Queue an Automation Action run",
    request_body = Option<RunInput>,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/runAutomationActionResponse"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn run_action(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
) -> IdentityHttpResult {
    enqueue(state, principal, id, headers, input, "Manual").await
}

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions/{id}/test",
    operation_id = "testAutomationAction",
    tag = "AutomationActions",
    summary = "Queue a test Automation Action run",
    request_body = Option<RunInput>,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/testAutomationActionResponse"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn test_action(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
) -> IdentityHttpResult {
    enqueue(state, principal, id, headers, input, "Test").await
}

pub(super) async fn enqueue(
    state: AutomationHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
    trigger: &str,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let input = input.map(|ValidatedJson(value)| value).unwrap_or_default();
    let args = match input.args_json {
        Some(Value::String(value)) => identity_result(
            serde_json::from_str::<Value>(&value)
                .map_err(|_| IdentityError::Validation("Args must be valid JSON.".to_owned())),
            &headers,
        )?,
        Some(value) => value,
        None => {
            let action = identity_result(
                state.automation.store().get(id).await.map_err(map_error),
                &headers,
            )?;
            identity_result(
                serde_json::from_str(&action.default_args_json).map_err(|_| {
                    IdentityError::Storage("Stored Automation arguments are invalid.".into())
                }),
                &headers,
            )?
        }
    };
    let mut receiver = match state
        .automation
        .run(
            principal.actor_id,
            id,
            trigger,
            &args,
            if trigger == "Test" {
                None
            } else {
                input.timeout_seconds
            },
            input.code.as_deref(),
        )
        .await
    {
        Ok(receiver) => receiver,
        Err(error) => {
            let code = match error {
                AutomationError::Validation(_) => 400,
                AutomationError::NotFound => 404,
                AutomationError::Conflict(_) => 409,
                _ => 500,
            };
            let message = if code == 500 {
                "Automation could not be started.".to_owned()
            } else {
                error.to_string()
            };
            return Ok(no_store(
                Json(vec![AutomationProgress {
                    error_message: Some(message.clone()),
                    error: Some(AutomationProgressError { code, message }),
                    ..Default::default()
                }])
                .into_response(),
            ));
        }
    };
    let stream = async_stream::stream! {
        yield Ok::<_, std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first = true;
        while let Some(item) = receiver.recv().await {
            if !first { yield Ok(bytes::Bytes::from_static(b",")); }
            first = false;
            // These DTOs contain only strings, integers and UUIDs.
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&AutomationProgress::from(item)).expect("Automation progress serializes")));
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    let mut response = no_store(Body::from_stream(stream).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/v1/automation/actions/{id}/runs",
    operation_id = "listAutomationActionRuns",
    tag = "AutomationActions",
    summary = "List Automation Action runs",
    responses(
        (status = 200, description = "Success", body = RunList, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("limit" = Option<i32>, Query, minimum = 1, maximum = 100, extensions(("x-citadel-default" = json!(20))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_runs(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<LimitQuery>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let runs = identity_result(
        state
            .automation
            .store()
            .list_runs(id, query.limit.unwrap_or(20))
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(
        Json(RunList {
            runs: runs.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/automation/actions/{id}/runs/{runId}",
    operation_id = "getAutomationActionRun",
    tag = "AutomationActions",
    summary = "Get an Automation Action run",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/AutomationActionRunView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_run(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let run = identity_result(
        state
            .automation
            .store()
            .get_run(id, run_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(AutomationRunView::from(run)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/automation/actions/{id}/runs/{runId}/logs",
    operation_id = "getAutomationActionRunLogs",
    tag = "AutomationActions",
    summary = "Get Automation Action run logs",
    responses(
        (status = 200, description = "Success", body = AutomationActionRunLogsView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn run_logs(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let run = identity_result(
        state
            .automation
            .store()
            .get_run(id, run_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(
        Json(AutomationActionRunLogsView {
            run_id: run.id,
            logs: run.logs.unwrap_or_default(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions/{id}/runs/{runId}/cancel",
    operation_id = "cancelAutomationActionRun",
    tag = "AutomationActions",
    summary = "Cancel an Automation Action run",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn cancel_run(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    identity_result(
        state.automation.cancel(id, run_id).await.map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
