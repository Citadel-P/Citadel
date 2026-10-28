//! Script HTTP routes, authorization and local request handling.
use crate::api::resources::scripts::authorized::{authorized_scripts, map_error};
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        request_validation::{ApiPath, ApiQuery, ValidatedJson},
        resources::scripts::{
            capabilities::{capabilities, granted},
            patch::{UpdateScriptInput, UpdateScriptMetadata},
            requests::*,
            views::*,
        },
    },
    openapi::router::OpenApiRouterExt,
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Extension, RawQuery, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_scripts::{ScriptError, ScriptService};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, ResourceType};

use serde_json::Value;

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct ScriptHttpState {
    pub identity: Arc<IdentityService>,
    pub scripts: Arc<ScriptService>,
}

pub fn router(state: ScriptHttpState) -> Router {
    crate::api::mutation_notifications::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "Script",
    )
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<ScriptHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(get_one))
        .normalized_routes(utoipa_axum::routes!(duplicate_draft))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(update))
        .normalized_routes(utoipa_axum::routes!(update_metadata))
        .normalized_routes(utoipa_axum::routes!(remove))
        .normalized_routes(utoipa_axum::routes!(run_script))
        .normalized_routes(utoipa_axum::routes!(test_script))
        .normalized_routes(utoipa_axum::routes!(list_runs))
        .normalized_routes(utoipa_axum::routes!(get_run))
        .normalized_routes(utoipa_axum::routes!(run_logs))
        .normalized_routes(utoipa_axum::routes!(cancel_run))
        .normalized_routes(utoipa_axum::routes!(resolve_cleanup))
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
    state: &ScriptHttpState,
    principal: &ActorPrincipal,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> HttpResult<()> {
    api_result(
        match level {
            PermissionLevel::Read => {
                state
                    .identity
                    .require_scope::<citadel_scripts::permissions::ReadScript>(principal)
                    .await
            }
            PermissionLevel::Write => {
                state
                    .identity
                    .require_scope::<citadel_scripts::permissions::WriteScript>(principal)
                    .await
            }
            PermissionLevel::Execute => {
                state
                    .identity
                    .require_scope::<citadel_scripts::permissions::ExecuteScript>(principal)
                    .await
            }
            _ => Err(citadel_identity::IdentityError::Forbidden),
        },
        headers,
    )?;
    Ok(())
}

async fn authorize(
    state: &ScriptHttpState,
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
                    .require_resource::<citadel_scripts::permissions::ReadScript>(principal, id)
                    .await
            }
            PermissionLevel::Write => {
                state
                    .identity
                    .require_resource::<citadel_scripts::permissions::WriteScript>(principal, id)
                    .await
            }
            PermissionLevel::Execute => {
                state
                    .identity
                    .require_resource::<citadel_scripts::permissions::ExecuteScript>(principal, id)
                    .await
            }
            _ => Err(citadel_identity::IdentityError::Forbidden),
        },
        headers,
    )?;
    Ok(())
}

async fn script_response(
    state: &ScriptHttpState,
    principal: &ActorPrincipal,
    script: citadel_scripts::Script,
    headers: &HeaderMap,
) -> HttpResult {
    let mut scripts = api_result(
        authorized_scripts(state.scripts.store().as_ref(), principal, vec![script]).await,
        headers,
    )?;
    Ok(no_store(Json(scripts.remove(0)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/scripts",
    operation_id = "listScripts",
    tag = "Scripts",
    summary = "List Scripts",
    responses(
        (status = 200, description = "Success", body = ScriptList, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    let tags = api_result(
        crate::api::routes::tags::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut scripts = api_result(
        state
            .scripts
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    scripts.retain(|script| crate::api::routes::tags::matches_filters(&script.tags, &tags));
    let scripts = api_result(
        authorized_scripts(state.scripts.store().as_ref(), &principal, scripts).await,
        &headers,
    )?;
    let level = if principal.is_administrator() {
        citadel_primitives::EffectivePermission::Administrator
    } else {
        granted(
            api_result(
                state
                    .identity
                    .global_permission(&principal, ResourceType::Script)
                    .await,
                &headers,
            )?
            .map_or(PermissionLevel::None, |grant| grant.level),
        )
    };
    Ok(no_store(
        Json(ScriptList {
            scripts,
            capabilities: capabilities(level),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/scripts",
    operation_id = "createScript",
    tag = "Scripts",
    summary = "Create a Script",
    request_body = ScriptInput,
    responses(
        (status = 200, description = "Success", body = AuthorizedScript, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<ScriptInput>,
) -> HttpResult {
    let mut input: citadel_scripts::ScriptConfiguration = input.into();
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    if let Some(source_id) = input.duplicate_source_id {
        authorize(
            &state,
            &principal,
            source_id,
            PermissionLevel::Read,
            &headers,
        )
        .await?;
    }
    api_result(
        state
            .scripts
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
    if citadel_scripts::changes_paid_configuration(None, &input) {
        api_result(
            state
                .scripts
                .ensure_automated_operations()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let script = api_result(
        state
            .scripts
            .store()
            .create(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    script_response(&state, &principal, script, &headers).await
}

#[utoipa::path(
    get,
    path = "/api/v1/scripts/{id}",
    operation_id = "getScript",
    tag = "Scripts",
    summary = "Get a Script",
    responses(
        (status = 200, description = "Success", body = AuthorizedScript, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_one(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let script = api_result(
        state.scripts.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    script_response(&state, &principal, script, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/scripts/rename",
    operation_id = "renameScript",
    tag = "Scripts",
    summary = "Rename a Script",
    request_body = RenameInput,
    responses(
        (status = 200, description = "Success", body = AuthorizedScript, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<ScriptHttpState>,
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
    let script = api_result(
        state
            .scripts
            .store()
            .rename(input.id, &input.name, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    script_response(&state, &principal, script, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/scripts/{id}",
    operation_id = "updateScript",
    tag = "Scripts",
    summary = "Update a Script",
    request_body(content(
        (UpdateScriptInput = "application/merge-patch+json"),
        (UpdateScriptInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AuthorizedScript, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    patch: Result<Json<UpdateScriptInput>, JsonRejection>,
) -> HttpResult {
    update_script(
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
    path = "/api/v1/scripts/{id}/_metadata",
    operation_id = "updateScriptMetadata",
    tag = "Scripts",
    summary = "Update Script metadata",
    request_body(content(
        (UpdateScriptMetadata = "application/merge-patch+json"),
        (UpdateScriptMetadata = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AuthorizedScript, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_metadata(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    patch: Result<Json<UpdateScriptMetadata>, JsonRejection>,
) -> HttpResult {
    update_script(
        state,
        principal,
        id,
        headers,
        patch.map(|Json(value)| value.into()),
        true,
    )
    .await
}

async fn update_script(
    state: ScriptHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    patch: Result<UpdateScriptInput, JsonRejection>,
    metadata_only: bool,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let patch = api_result(
        patch.map_err(crate::api::request_validation::invalid_json),
        &headers,
    )?;
    let current = api_result(
        state.scripts.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    let mut input = patch.apply(current.clone());
    api_result(
        state
            .scripts
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
    if citadel_scripts::changes_paid_configuration(Some(&current), &input) {
        api_result(
            state
                .scripts
                .ensure_automated_operations()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let script = api_result(
        state
            .scripts
            .store()
            .update(&current, &input, principal.actor_id, metadata_only)
            .await
            .map_err(map_error),
        &headers,
    )?;
    script_response(&state, &principal, script, &headers).await
}

#[utoipa::path(
    delete,
    path = "/api/v1/scripts/{id}",
    operation_id = "deleteScript",
    tag = "Scripts",
    summary = "Delete a Script",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    api_result(
        state
            .scripts
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
    path = "/api/v1/scripts/{id}/run",
    operation_id = "runScript",
    tag = "Scripts",
    summary = "Queue a Script run",
    request_body = Option<RunInput>,
    responses(
        (status = 200, description = "Success", body = Vec<ScriptProgress>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn run_script(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
) -> HttpResult {
    enqueue(state, principal, id, headers, input, "Manual").await
}

#[utoipa::path(
    post,
    path = "/api/v1/scripts/{id}/test",
    operation_id = "testScript",
    tag = "Scripts",
    summary = "Queue a test Script run",
    request_body = Option<RunInput>,
    responses(
        (status = 200, description = "Success", body = Vec<ScriptProgress>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn test_script(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
) -> HttpResult {
    enqueue(state, principal, id, headers, input, "Test").await
}

async fn enqueue(
    state: ScriptHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    input: Option<ValidatedJson<RunInput>>,
    trigger: &str,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let input = input.map(|ValidatedJson(value)| value).unwrap_or_default();
    let draft = if let Some(code) = &input.code {
        let current = api_result(
            state.scripts.store().get(id).await.map_err(map_error),
            &headers,
        )?;
        api_result(
            state
                .identity
                .ensure_run_as_allowed(
                    &principal,
                    citadel_primitives::ActorId::new(current.run_as_actor_id),
                )
                .await,
            &headers,
        )?;
        Some(citadel_scripts::ScriptDraft {
            code: code.clone(),
            expected_row_version: current.row_version,
        })
    } else {
        None
    };
    let args = match input.args_json {
        Some(Value::String(value)) => api_result(
            serde_json::from_str::<Value>(&value)
                .map_err(|_| ApiError::Validation("Args must be valid JSON.".to_owned())),
            &headers,
        )?,
        Some(value) => value,
        None => {
            let script = api_result(
                state.scripts.store().get(id).await.map_err(map_error),
                &headers,
            )?;
            api_result(
                serde_json::from_str(&script.default_args_json).map_err(ApiError::internal),
                &headers,
            )?
        }
    };
    let mut receiver = match state
        .scripts
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
            draft.as_ref(),
        )
        .await
    {
        Ok(receiver) => receiver,
        Err(error) => {
            let code = match error {
                ScriptError::LicenseRequired => 403,
                ScriptError::Validation(_) => 400,
                ScriptError::NotFound => 404,
                ScriptError::Conflict(_) => 409,
                _ => 500,
            };
            let message = if code == 500 {
                "Script could not be started.".to_owned()
            } else {
                error.to_string()
            };
            return Ok(no_store(
                Json(vec![ScriptProgress {
                    error_message: Some(message.clone()),
                    error: Some(ScriptProgressError { code, message }),
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
            yield ScriptProgress::try_from(item)
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
    path = "/api/v1/scripts/{id}/runs",
    operation_id = "listScriptRuns",
    tag = "Scripts",
    summary = "List Script runs",
    responses(
        (status = 200, description = "Success", body = RunList, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("limit" = Option<i32>, Query, minimum = 1, maximum = 100, extensions(("x-citadel-default" = json!(20))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_runs(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<LimitQuery>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let runs = api_result(
        state
            .scripts
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
                    .map(ScriptRunView::try_from)
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
    path = "/api/v1/scripts/{id}/runs/{runId}",
    operation_id = "getScriptRun",
    tag = "Scripts",
    summary = "Get a Script run",
    responses(
        (status = 200, description = "Success", body = ScriptRunView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_run(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let run = api_result(
        state
            .scripts
            .store()
            .get_run(id, run_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(
        Json(api_result(
            ScriptRunView::try_from(run).map_err(ApiError::internal),
            &headers,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/scripts/{id}/runs/{runId}/logs",
    operation_id = "getScriptRunLogs",
    tag = "Scripts",
    summary = "Get Script run logs",
    responses(
        (status = 200, description = "Success", body = ScriptRunLogsView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn run_logs(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let run = api_result(
        state
            .scripts
            .store()
            .get_run(id, run_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ScriptRunLogsView {
            run_id: run.id,
            logs: run.logs.unwrap_or_default(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/scripts/{id}/runs/{runId}/cancel",
    operation_id = "cancelScriptRun",
    tag = "Scripts",
    summary = "Cancel a Script run",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn cancel_run(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    api_result(
        state.scripts.cancel(id, run_id).await.map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    get, path = "/api/v1/scripts/{id}/duplicate-draft", operation_id = "getScriptDuplicateDraft",
    tag = "Scripts", summary = "Prepare a disabled configuration copy",
    responses((status = 200, description = "Editable configuration without activation or runtime state", body = ScriptDuplicateDraftView), crate::openapi::errors::ResourceErrors),
    params(("id" = uuid::Uuid, Path)), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn duplicate_draft(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let source = api_result(
        state.scripts.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    let draft = ScriptInput::from(source.duplicate_configuration());
    Ok(no_store(
        Json(ScriptDuplicateDraftView {
            source_name: source.name,
            draft,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/scripts/{id}/runs/{runId}/recovery", operation_id = "resolveScriptRunCleanup",
    tag = "Scripts", summary = "Resolve retained scripts capacity after manual cleanup and Agent restart",
    request_body = ResolveScriptCleanupInput,
    responses((status = 204, description = "Capacity released and acknowledgment audited; result unchanged"), crate::openapi::errors::ResourceMutationErrors),
    params(("id" = uuid::Uuid, Path), ("runId" = uuid::Uuid, Path)), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn resolve_cleanup(
    State(state): State<ScriptHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, run_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
    input: Result<Json<ResolveScriptCleanupInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(
        super::authentication::require_human_administrator(principal),
        &headers,
    )?;
    let Json(input) = api_result(
        input.map_err(|_| ApiError::Validation("Invalid cleanup request.".into())),
        &headers,
    )?;
    api_result(
        state
            .scripts
            .resolve_cleanup(
                id,
                run_id,
                input.attempt_id,
                principal.actor_id,
                &input.reason,
                input.acknowledged,
            )
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}
