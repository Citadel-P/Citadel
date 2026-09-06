use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Path, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_automation::{
    AutomationActionInput, AutomationError, AutomationProgress, AutomationProgressError,
    AutomationService,
};
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
            .contract_route(routes::UPDATE_AUTOMATION_ACTION_METADATA, update_metadata)
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
    actions: Vec<AuthorizedAction>,
    capabilities: citadel_platforms::ResourceCapabilitiesView,
}

#[derive(Serialize)]
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

async fn create(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(mut input): Json<AutomationActionInput>,
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
    action_response(&state, &principal, action, &headers).await
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
            .rename(input.id, &input.name, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

async fn update(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> IdentityHttpResult {
    update_action(state, principal, id, headers, patch, false).await
}

async fn update_metadata(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(patch): Json<Value>,
) -> IdentityHttpResult {
    update_action(state, principal, id, headers, patch, true).await
}

async fn update_action(
    state: AutomationHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    patch: Value,
    metadata_only: bool,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    if metadata_only
        && !patch
            .as_object()
            .is_some_and(|fields| fields.keys().all(|key| key == "description"))
    {
        return identity_result(
            Err(IdentityError::Validation(
                "Only description can be changed through Action metadata.".into(),
            )),
            &headers,
        );
    }
    let current = identity_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    let mut input = merge_update(current.clone(), patch, &headers)?;
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

async fn remove(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
        AutomationError::LicenseRequired => IdentityError::LicenseRequired("automated-operations"),
        AutomationError::Validation(message) => IdentityError::Validation(message),
        AutomationError::NotFound => IdentityError::NotFound,
        AutomationError::Conflict(message) => IdentityError::Conflict(message),
        AutomationError::Storage(message) => IdentityError::Storage(message),
        AutomationError::External(message) => IdentityError::External(message),
    }
}
