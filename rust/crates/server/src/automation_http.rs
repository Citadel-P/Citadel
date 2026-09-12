use crate::request_validation::ApiPath;
use crate::request_validation::ApiQuery;
use crate::request_validation::ValidatedJson;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_automation::{
    AutomationActionInput, AutomationError, AutomationProgress, AutomationProgressError,
    AutomationService, UpdateAutomationActionInput, UpdateAutomationActionMetadata,
};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::openapi::router::OpenApiRouterExt;

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

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct ActionList {
    actions: Vec<AuthorizedAction>,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct AuthorizedAction {
    #[serde(flatten)]
    action: citadel_automation::AutomationActionView,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}

pub(crate) async fn authorized_actions(
    store: &dyn citadel_automation::AutomationStore,
    principal: &ActorPrincipal,
    actions: Vec<citadel_automation::AutomationActionView>,
) -> Result<Vec<AuthorizedAction>, AutomationError> {
    let ids: Vec<_> = actions.iter().map(|action| action.id).collect();
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        store.permissions(principal.actor_id, &ids).await?
    };
    Ok(actions
        .into_iter()
        .map(|action| {
            let level = if principal.is_administrator() {
                7
            } else {
                permissions.get(&action.id).copied().unwrap_or(0)
            };
            AuthorizedAction {
                action,
                capabilities: capabilities(level),
            }
        })
        .collect())
}

fn capabilities(level: i32) -> citadel_platforms::ResourceCapabilitiesView {
    citadel_platforms::ResourceCapabilitiesView {
        can_read: level >= 1,
        can_write: level >= 2,
        can_execute: level >= 4,
    }
}

async fn action_response(
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    action: citadel_automation::AutomationActionView,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let mut actions = identity_result(
        authorized_actions(state.automation.store().as_ref(), principal, vec![action])
            .await
            .map_err(map_error),
        headers,
    )?;
    Ok(no_store(Json(actions.remove(0)).into_response()))
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct RunList {
    runs: Vec<citadel_automation::AutomationRunView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct AutomationActionRunLogsView {
    run_id: Uuid,
    logs: String,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct RunInput {
    args_json: Option<Value>,
    timeout_seconds: Option<i32>,
    code: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct RenameInput {
    id: Uuid,
    name: String,
}

#[derive(Deserialize, Default)]
struct LimitQuery {
    limit: Option<usize>,
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
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let tags = identity_result(
        crate::resources_http::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut actions = identity_result(
        state
            .automation
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    actions.retain(|action| crate::resources_http::tags::matches_filters(&action.tags, &tags));
    let actions = identity_result(
        authorized_actions(state.automation.store().as_ref(), &principal, actions)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let level = if principal.is_administrator() {
        7
    } else {
        identity_result(
            state
                .identity
                .global_permission(&principal, ResourceType::AutomationAction)
                .await,
            &headers,
        )?
        .map_or(0, |grant| grant.level as i32)
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
        (status = 200, description = "Success", body = citadel_automation::AutomationActionView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(mut input): ValidatedJson<AutomationActionInput>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    identity_result(
        state
            .automation
            .validate_input(&mut input, principal.actor_id)
            .map_err(map_error),
        &headers,
    )?;
    identity_result(
        state
            .identity
            .ensure_run_as_allowed(
                &principal,
                citadel_domain::ActorId::new(
                    input
                        .run_as_actor_id
                        .expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &headers,
    )?;
    if citadel_automation::changes_paid_trigger(None, &input) {
        identity_result(
            state
                .automation
                .ensure_paid_trigger()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let action = identity_result(
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
        (status = 200, description = "Success", body = citadel_automation::AutomationActionView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let action = identity_result(
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
        (status = 200, description = "Success", body = citadel_automation::AutomationActionView, content_type = "application/json"),
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
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(
        &state,
        &principal,
        input.id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let action = identity_result(
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
        (status = 200, description = "Success", body = citadel_automation::AutomationActionView, content_type = "application/json"),
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
) -> IdentityHttpResult {
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
        (status = 200, description = "Success", body = citadel_automation::AutomationActionView, content_type = "application/json"),
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
) -> IdentityHttpResult {
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
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let patch = identity_result(
        patch.map_err(crate::request_validation::invalid_json),
        &headers,
    )?;
    let current = identity_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    let mut input = patch.apply(current.clone());
    identity_result(
        state
            .automation
            .validate_input(&mut input, principal.actor_id)
            .map_err(map_error),
        &headers,
    )?;
    identity_result(
        state
            .identity
            .ensure_run_as_allowed(
                &principal,
                citadel_domain::ActorId::new(
                    input
                        .run_as_actor_id
                        .expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &headers,
    )?;
    if citadel_automation::changes_paid_trigger(Some(&current), &input) {
        identity_result(
            state
                .automation
                .ensure_paid_trigger()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let action = identity_result(
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
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    identity_result(
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
        (status = 200, description = "Success", body = ref("#/components/schemas/runAutomationActionResponse"), content_type = "application/json"),
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
async fn test_action(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
) -> IdentityHttpResult {
    enqueue(state, principal, id, headers, input, "Test").await
}

async fn enqueue(
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
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Automation progress serializes")));
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
    Ok(no_store(Json(RunList { runs }).into_response()))
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
async fn get_run(
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
    Ok(no_store(Json(run).into_response()))
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
async fn cancel_run(
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

async fn authorize_global(
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        state
            .identity
            .authorize(principal, ResourceType::AutomationAction, level, None)
            .await,
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
) -> IdentityHttpResult<()> {
    identity_result(
        state
            .identity
            .authorize_resource(principal, ResourceType::AutomationAction, id, level, None)
            .await,
        headers,
    )?;
    Ok(())
}
fn map_error(error: AutomationError) -> IdentityError {
    match error {
        AutomationError::LicenseRequired => IdentityError::LicenseRequired("automated-operations"),
        AutomationError::Validation(message) => IdentityError::Validation(message),
        AutomationError::NotFound => IdentityError::NotFound,
        AutomationError::Conflict(message) => IdentityError::Conflict(message),
        AutomationError::Storage(message) => IdentityError::Storage(message),
        AutomationError::External(message) => IdentityError::External(message),
    }
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
