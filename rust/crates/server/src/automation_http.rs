use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_automation::{AutomationActionInput, AutomationError, AutomationService};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

#[derive(Clone)]
pub struct AutomationHttpState {
    pub identity: Arc<IdentityService>,
    pub automation: Arc<AutomationService>,
}

pub fn router(state: AutomationHttpState) -> Router {
    crate::realtime::notify_mutations(
        Router::new()
            .contract_route(routes::LIST_AUTOMATION_ACTIONS, list)
            .contract_route(routes::CREATE_AUTOMATION_ACTION, create)
            .contract_route(routes::GET_AUTOMATION_ACTION, get_one)
            .contract_route(routes::RENAME_AUTOMATION_ACTION, rename)
            .contract_route(routes::UPDATE_AUTOMATION_ACTION, update)
            .contract_route(routes::DELETE_AUTOMATION_ACTION, remove)
            .contract_route(routes::RUN_AUTOMATION_ACTION, run_action)
            .contract_route(routes::TEST_AUTOMATION_ACTION, test_action)
            .contract_route(routes::LIST_AUTOMATION_RUNS, list_runs)
            .contract_route(routes::GET_AUTOMATION_RUN, get_run)
            .contract_route(routes::GET_AUTOMATION_RUN_LOGS, run_logs)
            .contract_route(routes::CANCEL_AUTOMATION_RUN, cancel_run)
            .with_state(state),
        "AutomationAction",
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionList {
    actions: Vec<citadel_automation::AutomationActionView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunList {
    runs: Vec<citadel_automation::AutomationRunView>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RunInput {
    args_json: Option<Value>,
    timeout_seconds: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameInput {
    id: Uuid,
    name: String,
}

#[derive(Deserialize, Default)]
struct LimitQuery {
    limit: Option<usize>,
}

async fn list(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let actions = identity_result(
        state
            .automation
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(ActionList { actions }).into_response()))
}

async fn create(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(mut input): Json<AutomationActionInput>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    identity_result(
        input.validate(principal.actor_id).map_err(map_error),
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
    let action = identity_result(
        state
            .automation
            .store()
            .create(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(action).into_response()))
}

async fn get_one(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let action = identity_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(action).into_response()))
}

async fn rename(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<RenameInput>,
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
            .rename(input.id, &input.name)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(action).into_response()))
}

async fn update(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let current = identity_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    let mut input = merge_update(current, patch, &headers)?;
    identity_result(
        input.validate(principal.actor_id).map_err(map_error),
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
    let action = identity_result(
        state
            .automation
            .store()
            .update(id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(Json(action).into_response()))
}

async fn remove(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    identity_result(
        state.automation.store().delete(id).await.map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn run_action(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    input: Option<Json<RunInput>>,
) -> IdentityHttpResult {
    enqueue(state, principal, id, headers, input, "Manual").await
}

async fn test_action(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    input: Option<Json<RunInput>>,
) -> IdentityHttpResult {
    enqueue(state, principal, id, headers, input, "Test").await
}

async fn enqueue(
    state: AutomationHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    input: Option<Json<RunInput>>,
    trigger: &str,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let input = input.map(|Json(value)| value).unwrap_or_default();
    let args = match input.args_json {
        Some(Value::String(value)) => identity_result(
            serde_json::from_str::<Value>(&value)
                .map_err(|_| IdentityError::Validation("Args must be valid JSON.".to_owned())),
            &headers,
        )?,
        Some(value) => value,
        None => serde_json::json!({}),
    };
    let run = identity_result(
        state
            .automation
            .store()
            .enqueue(
                principal.actor_id,
                id,
                trigger,
                &args,
                input.timeout_seconds,
            )
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(Json(vec![serde_json::json!({
        "runId": run.id,
        "status": "Queued",
        "progressMessage": "Automation Action run queued."
    })])
    .into_response())
}

async fn list_runs(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    Query(query): Query<LimitQuery>,
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

async fn get_run(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
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

async fn run_logs(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
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
        Json(serde_json::json!({"runId":run.id,"logs":run.logs.unwrap_or_default()}))
            .into_response(),
    ))
}

async fn cancel_run(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
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

fn merge_update(
    current: citadel_automation::AutomationActionView,
    patch: Value,
    headers: &HeaderMap,
) -> IdentityHttpResult<AutomationActionInput> {
    const FIELDS: &[&str] = &[
        "description",
        "code",
        "defaultArgsJson",
        "enabled",
        "scheduleEnabled",
        "scheduleCron",
        "scheduleTimeZone",
        "webhook",
        "timeoutSeconds",
        "alertOnFailure",
        "runAsActorId",
    ];
    let Value::Object(patch) = patch else {
        return identity_result(
            Err(IdentityError::Validation(
                "Automation Action update must be a JSON object.".to_owned(),
            )),
            headers,
        );
    };
    if let Some(field) = patch.keys().find(|field| !FIELDS.contains(&field.as_str())) {
        return identity_result(
            Err(IdentityError::Validation(format!(
                "Automation Action update field '{field}' is not supported."
            ))),
            headers,
        );
    }
    let mut value = serde_json::json!({
        "name": current.name,
        "description": current.description,
        "code": current.code,
        "defaultArgsJson": current.default_args_json,
        "enabled": current.enabled,
        "scheduleEnabled": current.schedule_enabled,
        "scheduleCron": current.schedule_cron,
        "scheduleTimeZone": current.schedule_time_zone,
        "webhook": current.webhook,
        "timeoutSeconds": current.timeout_seconds,
        "alertOnFailure": current.alert_on_failure,
        "runAsActorId": current.run_as_actor_id,
        "tagIds": [],
    });
    let target = value
        .as_object_mut()
        .expect("Automation Action update base is an object");
    for (key, value) in patch {
        target.insert(key, value);
    }
    identity_result(
        serde_json::from_value(value).map_err(|error| {
            IdentityError::Validation(format!("Automation Action update is invalid: {error}"))
        }),
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
        AutomationError::Validation(message) => IdentityError::Validation(message),
        AutomationError::NotFound => IdentityError::NotFound,
        AutomationError::Conflict(message) => IdentityError::Conflict(message),
        AutomationError::Storage(message) => IdentityError::Storage(message),
        AutomationError::External(message) => IdentityError::External(message),
    }
}
