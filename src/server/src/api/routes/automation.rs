//! Automation HTTP routes, authorization and local request handling.
use crate::api::resources::automation::authorized::{authorized_actions, map_error};
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::automation::{
            capabilities::{capabilities, granted},
            patch::{UpdateAutomationActionInput, UpdateAutomationActionMetadata},
            requests::*,
            views::*,
        },
    },
    openapi::router::OpenApiRouterExt,
    request_validation::{ApiPath, ApiQuery, ValidatedJson},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, RawQuery, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_automation::{AutomationError, AutomationService};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, ResourceType};

use serde_json::Value;

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct AutomationHttpState {
    pub identity: Arc<IdentityService>,
    pub automation: Arc<AutomationService>,
}

pub fn router(state: AutomationHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "AutomationAction",
    )
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<AutomationHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(get_one))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(update))
        .normalized_routes(utoipa_axum::routes!(update_metadata))
        .normalized_routes(utoipa_axum::routes!(remove))
        .normalized_routes(utoipa_axum::routes!(run_action))
        .normalized_routes(utoipa_axum::routes!(test_action))
        .normalized_routes(utoipa_axum::routes!(list_runs))
        .normalized_routes(utoipa_axum::routes!(get_run))
        .normalized_routes(utoipa_axum::routes!(run_logs))
        .normalized_routes(utoipa_axum::routes!(cancel_run))
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
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        match level {
            PermissionLevel::Read => {
                state
                    .identity
                    .require_scope::<citadel_automation::permissions::ReadAutomationAction>(
                        principal,
                    )
                    .await
            }
            PermissionLevel::Write => {
                state
                    .identity
                    .require_scope::<citadel_automation::permissions::WriteAutomationAction>(
                        principal,
                    )
                    .await
            }
            PermissionLevel::Execute => {
                state
                    .identity
                    .require_scope::<citadel_automation::permissions::ExecuteAutomationAction>(
                        principal,
                    )
                    .await
            }
            _ => Err(citadel_identity::IdentityError::Forbidden),
        },
        headers,
    )?;
    Ok(())
}

async fn authorize(
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        match level {
            PermissionLevel::Read => {
                state
                    .identity
                    .require_resource::<citadel_automation::permissions::ReadAutomationAction>(
                        principal, id,
                    )
                    .await
            }
            PermissionLevel::Write => {
                state
                    .identity
                    .require_resource::<citadel_automation::permissions::WriteAutomationAction>(
                        principal, id,
                    )
                    .await
            }
            PermissionLevel::Execute => {
                state
                    .identity
                    .require_resource::<citadel_automation::permissions::ExecuteAutomationAction>(
                        principal, id,
                    )
                    .await
            }
            _ => Err(citadel_identity::IdentityError::Forbidden),
        },
        headers,
    )?;
    Ok(())
}

async fn action_response(
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    action: citadel_automation::AutomationAction,
    headers: &HeaderMap,
) -> HttpResult {
    let mut actions = api_result(
        authorized_actions(state.automation.store().as_ref(), principal, vec![action]).await,
        headers,
    )?;
    Ok(no_store(Json(actions.remove(0)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/automation/actions",
    operation_id = "listAutomationActions",
    tag = "AutomationActions",
    summary = "List Automation Actions",
    responses(
        (status = 200, description = "Success", body = ActionList, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    let tags = api_result(
        crate::api::routes::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut actions = api_result(
        state
            .automation
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    actions.retain(|action| crate::api::routes::tags::matches_filters(&action.tags, &tags));
    let actions = api_result(
        authorized_actions(state.automation.store().as_ref(), &principal, actions).await,
        &headers,
    )?;
    let level = if principal.is_administrator() {
        citadel_primitives::EffectivePermission::Administrator
    } else {
        granted(
            api_result(
                state
                    .identity
                    .global_permission(&principal, ResourceType::AutomationAction)
                    .await,
                &headers,
            )?
            .map_or(PermissionLevel::None, |grant| grant.level),
        )
    };
    Ok(no_store(
        Json(ActionList {
            actions,
            capabilities: capabilities(level),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions",
    operation_id = "createAutomationAction",
    tag = "AutomationActions",
    summary = "Create an Automation Action",
    request_body = AutomationActionInput,
    responses(
        (status = 200, description = "Success", body = AuthorizedAction, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<AutomationActionInput>,
) -> HttpResult {
    let mut input: citadel_automation::AutomationActionConfiguration = input.into();
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    api_result(
        state
            .automation
            .validate_input(&mut input, principal.actor_id)
            .map_err(map_error),
        &headers,
    )?;
    api_result(
        state
            .identity
            .ensure_run_as_allowed(
                &principal,
                citadel_primitives::ActorId::new(
                    input
                        .run_as_actor_id
                        .expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &headers,
    )?;
    if citadel_automation::changes_paid_trigger(None, &input) {
        api_result(
            state
                .automation
                .ensure_paid_trigger()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let action = api_result(
        state
            .automation
            .store()
            .create(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    get,
    path = "/api/v1/automation/actions/{id}",
    operation_id = "getAutomationAction",
    tag = "AutomationActions",
    summary = "Get an Automation Action",
    responses(
        (status = 200, description = "Success", body = AuthorizedAction, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_one(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let action = api_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions/rename",
    operation_id = "renameAutomationAction",
    tag = "AutomationActions",
    summary = "Rename an Automation Action",
    request_body = RenameInput,
    responses(
        (status = 200, description = "Success", body = AuthorizedAction, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenameInput>,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(
        &state,
        &principal,
        input.id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let action = api_result(
        state
            .automation
            .store()
            .rename(input.id, &input.name, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/automation/actions/{id}",
    operation_id = "updateAutomationAction",
    tag = "AutomationActions",
    summary = "Update an Automation Action",
    request_body(content(
        (UpdateAutomationActionInput = "application/merge-patch+json"),
        (UpdateAutomationActionInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AuthorizedAction, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    patch: Result<Json<UpdateAutomationActionInput>, JsonRejection>,
) -> HttpResult {
    update_action(
        state,
        principal,
        id,
        headers,
        patch.map(|Json(value)| value),
        false,
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/api/v1/automation/actions/{id}/_metadata",
    operation_id = "updateAutomationActionMetadata",
    tag = "AutomationActions",
    summary = "Update Automation Action metadata",
    request_body(content(
        (UpdateAutomationActionMetadata = "application/merge-patch+json"),
        (UpdateAutomationActionMetadata = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AuthorizedAction, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_metadata(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    patch: Result<Json<UpdateAutomationActionMetadata>, JsonRejection>,
) -> HttpResult {
    update_action(
        state,
        principal,
        id,
        headers,
        patch.map(|Json(value)| value.into()),
        true,
    )
    .await
}

async fn update_action(
    state: AutomationHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    patch: Result<UpdateAutomationActionInput, JsonRejection>,
    metadata_only: bool,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let patch = api_result(
        patch.map_err(crate::request_validation::invalid_json),
        &headers,
    )?;
    let current = api_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    let mut input = patch.apply(current.clone());
    api_result(
        state
            .automation
            .validate_input(&mut input, principal.actor_id)
            .map_err(map_error),
        &headers,
    )?;
    api_result(
        state
            .identity
            .ensure_run_as_allowed(
                &principal,
                citadel_primitives::ActorId::new(
                    input
                        .run_as_actor_id
                        .expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &headers,
    )?;
    if citadel_automation::changes_paid_trigger(Some(&current), &input) {
        api_result(
            state
                .automation
                .ensure_paid_trigger()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let action = api_result(
        state
            .automation
            .store()
            .update(&current, &input, principal.actor_id, metadata_only)
            .await
            .map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    delete,
    path = "/api/v1/automation/actions/{id}",
    operation_id = "deleteAutomationAction",
    tag = "AutomationActions",
    summary = "Delete an Automation Action",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    api_result(
        state
            .automation
            .store()
            .delete(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions/{id}/run",
    operation_id = "runAutomationAction",
    tag = "AutomationActions",
    summary = "Queue an Automation Action run",
    request_body = Option<RunInput>,
    responses(
        (status = 200, description = "Success", body = Vec<AutomationProgress>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn run_action(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
) -> HttpResult {
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
        (status = 200, description = "Success", body = Vec<AutomationProgress>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn test_action(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
) -> HttpResult {
    enqueue(state, principal, id, headers, input, "Test").await
}

async fn enqueue(
    state: AutomationHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
    trigger: &str,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let input = input.map(|ValidatedJson(value)| value).unwrap_or_default();
    let args = match input.args_json {
        Some(Value::String(value)) => api_result(
            serde_json::from_str::<Value>(&value)
                .map_err(|_| ApiError::Validation("Args must be valid JSON.".to_owned())),
            &headers,
        )?,
        Some(value) => value,
        None => {
            let action = api_result(
                state.automation.store().get(id).await.map_err(map_error),
                &headers,
            )?;
            api_result(
                serde_json::from_str(&action.default_args_json).map_err(ApiError::internal),
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
        yield Ok::<_, serde_json::Error>(bytes::Bytes::from_static(b"["));
        let mut first = true;
        while let Some(item) = receiver.recv().await {
            if !first { yield Ok(bytes::Bytes::from_static(b",")); }
            first = false;
            yield AutomationProgress::try_from(item)
                .and_then(|item| serde_json::to_vec(&item))
                .map(bytes::Bytes::from);
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
async fn list_runs(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<LimitQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let runs = api_result(
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
            runs: api_result(
                runs.into_iter()
                    .map(AutomationRunView::try_from)
                    .collect::<Result<_, _>>()
                    .map_err(ApiError::internal),
                &headers,
            )?,
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
        (status = 200, description = "Success", body = AutomationRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_run(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let run = api_result(
        state
            .automation
            .store()
            .get_run(id, run_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            AutomationRunView::try_from(run).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
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
async fn run_logs(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let run = api_result(
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
async fn cancel_run(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    api_result(
        state.automation.cancel(id, run_id).await.map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
