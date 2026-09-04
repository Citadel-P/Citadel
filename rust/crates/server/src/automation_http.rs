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
    Router::new()
        .contract_route(routes::LIST_AUTOMATION_ACTIONS, list)
        .contract_route(routes::CREATE_AUTOMATION_ACTION, create)
        .contract_route(routes::GET_AUTOMATION_ACTION, get_one)
        .contract_route(routes::DELETE_AUTOMATION_ACTION, remove)
        .contract_route(routes::RUN_AUTOMATION_ACTION, run_action)
        .contract_route(routes::TEST_AUTOMATION_ACTION, test_action)
        .contract_route(routes::LIST_AUTOMATION_RUNS, list_runs)
        .contract_route(routes::GET_AUTOMATION_RUN, get_run)
        .contract_route(routes::GET_AUTOMATION_RUN_LOGS, run_logs)
        .contract_route(routes::CANCEL_AUTOMATION_RUN, cancel_run)
        .with_state(state)
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
    authorize_global(&state, &principal, PermissionLevel::Read, &headers).await?;
    let actions = identity_result(
        state.automation.store().list().await.map_err(map_error),
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
    let args = input
        .and_then(|Json(value)| value.args_json)
        .unwrap_or_else(|| serde_json::json!({}));
    let run = identity_result(
        state
            .automation
            .store()
            .enqueue(principal.actor_id, id, trigger, &args)
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
            .list_runs(id, 100)
            .await
            .map_err(map_error),
        &headers,
    )?
    .into_iter()
    .find(|run| run.id == run_id)
    .ok_or_else(|| {
        crate::identity_http::IdentityHttpError::from_parts(IdentityError::NotFound, &headers)
    })?;
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
            .list_runs(id, 100)
            .await
            .map_err(map_error),
        &headers,
    )?
    .into_iter()
    .find(|run| run.id == run_id)
    .ok_or_else(|| {
        crate::identity_http::IdentityHttpError::from_parts(IdentityError::NotFound, &headers)
    })?;
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
        state
            .automation
            .store()
            .cancel(id, run_id)
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
    }
}
