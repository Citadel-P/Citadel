use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Extension, Path, Query, RawQuery, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_stacks::{
    ApplyStackInput, CreateStackInput, ImportComposeProjectInput, PatchStackInput,
    RenameStackInput, ResourceCapabilities, RollbackStackInput, StackAction, StackChangeNotifier,
    StackError, StackFilter, StackService,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::realtime::RealtimeHub;

pub struct StacksRealtimeNotifier {
    realtime: Option<RealtimeHub>,
}

impl StacksRealtimeNotifier {
    #[must_use]
    pub const fn new(realtime: Option<RealtimeHub>) -> Self {
        Self { realtime }
    }
}

impl StackChangeNotifier for StacksRealtimeNotifier {
    fn changed(&self, id: Uuid, event: &'static str) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change("Stack", id, event);
        }
    }
}

#[derive(Clone)]
pub struct StacksHttpState {
    pub identity: Arc<IdentityService>,
    pub stacks: Arc<StackService>,
}

pub fn router(state: StacksHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_STACKS, list)
        .contract_route(routes::CREATE_STACK, create)
        .contract_route(routes::DELETE_STACKS, delete)
        .contract_route(routes::RENAME_STACK, rename)
        .contract_route(routes::APPLY_STACK, apply)
        .contract_route(routes::ROLLBACK_STACK, rollback)
        .contract_route(routes::PREFLIGHT_SWARM_STACK, preflight)
        .contract_route(routes::START_STACKS, start)
        .contract_route(routes::STOP_STACKS, stop)
        .contract_route(routes::PAUSE_STACKS, pause)
        .contract_route(routes::RESUME_STACKS, resume)
        .contract_route(routes::RESTART_STACKS, restart)
        .contract_route(routes::GET_STACK, get)
        .contract_route(routes::CHECK_STACK_UPDATES, check_updates)
        .contract_route(routes::GET_STACK_CONFIG, get_config)
        .contract_route(routes::GET_STACK_DUPLICATE_DRAFT, duplicate_draft)
        .contract_route(routes::LIST_STACK_RELEASES, releases)
        .contract_route(routes::UPDATE_STACK, update)
        .contract_route(routes::UPDATE_STACK_METADATA, update_metadata)
        .contract_route(routes::GET_STACK_DRIFT, drift)
        .contract_route(routes::UPDATE_STACK_DRIFT_POLICY, update_drift_policy)
        .contract_route(routes::RECONCILE_STACK, reconcile)
        .contract_route(routes::GET_COMPOSE_IMPORT_DRAFT, import_draft)
        .contract_route(routes::VALIDATE_COMPOSE_IMPORT_DRAFT, validate_import_draft)
        .contract_route(routes::IMPORT_COMPOSE_PROJECT, import)
        .with_state(state)
}

async fn check_updates(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let value = identity_result(
        state
            .stacks
            .check_updates(
                principal.actor_id,
                principal.is_administrator(),
                id,
                &cancel,
            )
            .await
            .map_err(|error| match error {
                StackError::Runtime(message) => IdentityError::External(message),
                other => stack_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn list(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let filter = identity_result(parse_filter(query.as_deref()), &headers)?;
    let capabilities = collection_capabilities(&state.identity, &principal, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .list(
                principal.actor_id,
                principal.is_administrator(),
                &filter,
                capabilities,
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn get(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .stacks
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn get_config(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .stacks
            .get_config(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn duplicate_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Read,
        Some(SpecificPermission::ResourceBindings),
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .stacks
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn releases(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .stacks
            .releases(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(citadel_stacks::StackReleasesView { releases: value }).into_response(),
    ))
}

async fn create(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Stack,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .create(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn update(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_patch_source(&state, &principal, input.spec.as_ref(), &headers).await?;
    let value = identity_result(
        state
            .stacks
            .update(principal.actor_id, principal.is_administrator(), id, input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn update_metadata(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let description = match input.as_object().and_then(|value| value.get("description")) {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) => None,
        Some(_) => {
            return Err(crate::identity_http::IdentityHttpError::from_parts(
                IdentityError::Validation("Description must be a string or null.".to_owned()),
                &headers,
            ));
        }
        None => {
            return get(
                State(state),
                Some(Extension(principal)),
                Ok(Path(id)),
                headers,
            )
            .await;
        }
    };
    let value = identity_result(
        state
            .stacks
            .update_metadata(
                principal.actor_id,
                principal.is_administrator(),
                id,
                description,
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn rename(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize(
        &state,
        &principal,
        input.id,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .stacks
            .rename(
                principal.actor_id,
                principal.is_administrator(),
                input.id,
                input.name,
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn delete(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<Vec<Uuid>>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(ids) = identity_result(input.map_err(invalid_json), &headers)?;
    for id in &ids {
        authorize(
            &state,
            &principal,
            *id,
            PermissionLevel::Execute,
            None,
            &headers,
        )
        .await?;
    }
    identity_result(
        state
            .stacks
            .delete(principal.actor_id, principal.is_administrator(), &ids)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn apply(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ApplyStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize(
        &state,
        &principal,
        input.id,
        PermissionLevel::Execute,
        Some(SpecificPermission::Apply),
        &headers,
    )
    .await?;
    let receiver = identity_result(
        state
            .stacks
            .apply(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(progress_response(receiver))
}

async fn rollback(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RollbackStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize(
        &state,
        &principal,
        input.stack_id,
        PermissionLevel::Execute,
        Some(SpecificPermission::Apply),
        &headers,
    )
    .await?;
    let receiver = identity_result(
        state
            .stacks
            .rollback(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(progress_response(receiver))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwarmPreflightInput {
    compose_files: Vec<String>,
    #[serde(default)]
    build_image_bindings: Vec<citadel_stacks::StackBuildImageBinding>,
}
async fn preflight(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<SwarmPreflightInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Stack,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .stacks
            .preflight_swarm(&input.compose_files, &input.build_image_bindings)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn drift(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Read,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .stacks
            .drift(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StackDriftPolicyInput {
    mode: Option<citadel_stacks::StackDriftMode>,
    alert_on_drift: Option<bool>,
    mark_degraded: Option<bool>,
    auto_start_stopped_containers: Option<bool>,
    auto_resume_paused_containers: Option<bool>,
    remove_extra_containers: Option<bool>,
}

impl From<StackDriftPolicyInput> for citadel_stacks::StackDriftPolicy {
    fn from(value: StackDriftPolicyInput) -> Self {
        let defaults = Self::default();
        Self {
            mode: value.mode.unwrap_or(defaults.mode),
            alert_on_drift: value.alert_on_drift.unwrap_or(defaults.alert_on_drift),
            mark_degraded: value.mark_degraded.unwrap_or(defaults.mark_degraded),
            auto_start_stopped_containers: value
                .auto_start_stopped_containers
                .unwrap_or(defaults.auto_start_stopped_containers),
            auto_resume_paused_containers: value
                .auto_resume_paused_containers
                .unwrap_or(defaults.auto_resume_paused_containers),
            remove_extra_containers: value
                .remove_extra_containers
                .unwrap_or(defaults.remove_extra_containers),
        }
        .normalized()
    }
}

async fn update_drift_policy(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<StackDriftPolicyInput>, JsonRejection>,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Write,
        None,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .stacks
            .update_drift_policy(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn reconcile(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = actor_and_id(
        &state,
        principal,
        path,
        PermissionLevel::Execute,
        None,
        &headers,
    )
    .await?;
    let value = identity_result(
        state
            .stacks
            .reconcile_drift(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn import_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ImportDraftQuery>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(query) = identity_result(query.map_err(invalid_query), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Platform,
                platform_id,
                PermissionLevel::Read,
                None,
            )
            .await,
        &headers,
    )?;
    let value = identity_result(
        state
            .stacks
            .import_draft(platform_id, &project_name, query.import_kind)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportDraftQuery {
    import_kind: Option<citadel_stacks::StackImportKind>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ValidateImportRequest {
    name: String,
    stack_source: citadel_stacks::StackSource,
    spec: citadel_stacks::StackSpec,
    import_kind: Option<citadel_stacks::StackImportKind>,
}

async fn validate_import_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ValidateImportRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = identity_result(path.map_err(invalid_path), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Stack,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .validate_import(
                platform_id,
                &project_name,
                &input.name,
                input.stack_source,
                &input.spec,
                input.import_kind,
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportRequest {
    name: String,
    description: Option<String>,
    stack_source: citadel_stacks::StackSource,
    spec: citadel_stacks::StackSpec,
    preview_fingerprint: String,
    #[serde(default)]
    tag_ids: Vec<Uuid>,
    import_kind: Option<citadel_stacks::StackImportKind>,
    #[serde(default)]
    import_sensitive_environment_as_secrets: bool,
}

async fn import(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ImportRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = identity_result(path.map_err(invalid_path), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Stack,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    if input.stack_source != input.spec.source() {
        return Err(crate::identity_http::IdentityHttpError::from_parts(
            IdentityError::Validation("Stack source and specification type must match.".to_owned()),
            &headers,
        ));
    }
    if input.import_sensitive_environment_as_secrets {
        return Err(crate::identity_http::IdentityHttpError::from_parts(
            IdentityError::Validation(
                "Importing newly detected sensitive values requires the Phase 7 Secret execution slice."
                    .to_owned(),
            ),
            &headers,
        ));
    }
    let import_kind = match input.import_kind {
        Some(value) => value,
        None => {
            identity_result(
                state
                    .stacks
                    .import_draft(platform_id, &project_name, None)
                    .await
                    .map_err(stack_error),
                &headers,
            )?
            .import_kind
        }
    };
    let input = ImportComposeProjectInput {
        name: input.name,
        platform_id,
        project_name,
        description: input.description,
        spec: input.spec,
        tag_ids: input.tag_ids,
        import_kind,
        preview_fingerprint: input.preview_fingerprint,
        detected_secret_values: std::collections::BTreeMap::new(),
    };
    let value = identity_result(
        state
            .stacks
            .import(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(value).into_response()))
}

async fn authorize_git_source(
    state: &StacksHttpState,
    principal: &ActorPrincipal,
    spec: &citadel_stacks::StackSpec,
    headers: &HeaderMap,
) -> Result<(), crate::identity_http::IdentityHttpError> {
    let citadel_stacks::StackSpec::Git { git_repo_id, .. } = spec else {
        return Ok(());
    };
    identity_result(
        state
            .identity
            .authorize_resource(
                principal,
                ResourceType::GitRepository,
                *git_repo_id,
                PermissionLevel::Read,
                None,
            )
            .await,
        headers,
    )
}

async fn authorize_git_patch_source(
    state: &StacksHttpState,
    principal: &ActorPrincipal,
    spec: Option<&Value>,
    headers: &HeaderMap,
) -> Result<(), crate::identity_http::IdentityHttpError> {
    let Some(git_repository_id) = spec
        .and_then(Value::as_object)
        .and_then(|spec| spec.get("gitRepoId").or_else(|| spec.get("GitRepoId")))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        return Ok(());
    };
    identity_result(
        state
            .identity
            .authorize_resource(
                principal,
                ResourceType::GitRepository,
                git_repository_id,
                PermissionLevel::Read,
                None,
            )
            .await,
        headers,
    )
}

macro_rules! state_action {
    ($name:ident,$action:expr) => {
        async fn $name(
            State(state): State<StacksHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            input: Result<Json<Vec<Uuid>>, JsonRejection>,
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Json(ids) = identity_result(input.map_err(invalid_json), &headers)?;
            for id in &ids {
                authorize(
                    &state,
                    &principal,
                    *id,
                    PermissionLevel::Write,
                    None,
                    &headers,
                )
                .await?;
            }
            identity_result(
                state
                    .stacks
                    .change_state(
                        principal.actor_id,
                        principal.is_administrator(),
                        &ids,
                        $action,
                    )
                    .await
                    .map_err(stack_error),
                &headers,
            )?;
            Ok(StatusCode::NO_CONTENT.into_response())
        }
    };
}
state_action!(start, StackAction::Start);
state_action!(stop, StackAction::Stop);
state_action!(pause, StackAction::Pause);
state_action!(resume, StackAction::Resume);
state_action!(restart, StackAction::Restart);

fn progress_response(
    mut receiver: tokio::sync::mpsc::Receiver<citadel_stacks::StackStreamItem>,
) -> axum::response::Response {
    let stream = async_stream::stream! {yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));let mut first=true;while let Some(item)=receiver.recv().await{if !first{yield Ok(bytes::Bytes::from_static(b","));}first=false;if let Ok(value)=serde_json::to_vec(&item){yield Ok(bytes::Bytes::from(value));}}yield Ok(bytes::Bytes::from_static(b"]"));};
    let mut response = Body::from_stream(stream).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn actor_and_id(
    state: &StacksHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    level: PermissionLevel,
    specific: Option<SpecificPermission>,
    headers: &HeaderMap,
) -> IdentityHttpResult<(ActorPrincipal, Uuid)> {
    let principal = identity_result(require_actor(principal), headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), headers)?;
    authorize(state, &principal, id, level, specific, headers).await?;
    Ok((principal, id))
}
async fn authorize(
    state: &StacksHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    level: PermissionLevel,
    specific: Option<SpecificPermission>,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    state
        .identity
        .authorize_resource(principal, ResourceType::Stack, id, level, specific)
        .await
        .map_err(|error| crate::identity_http::IdentityHttpError::from_parts(error, headers))
}
async fn collection_capabilities(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceCapabilities> {
    if principal.is_administrator() {
        return Ok(ResourceCapabilities {
            can_create: true,
            can_delete: true,
        });
    }
    let p = identity
        .global_permission(principal, ResourceType::Stack)
        .await
        .map_err(|error| crate::identity_http::IdentityHttpError::from_parts(error, headers))?;
    Ok(ResourceCapabilities {
        can_create: p
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Write)),
        can_delete: p
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Execute)),
    })
}
fn parse_filter(query: Option<&str>) -> Result<StackFilter, IdentityError> {
    let mut filter = StackFilter::default();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key.eq_ignore_ascii_case("tags") {
            if filter.tags.len() >= 100 {
                return Err(IdentityError::Validation(
                    "At most 100 Stack Tags may be filtered at once.".to_owned(),
                ));
            }
            if !value.trim().is_empty() && !filter.tags.iter().any(|tag| tag == &value) {
                filter.tags.push(value.into_owned());
            }
        } else if key.eq_ignore_ascii_case("platformId") {
            if filter.platform_id.is_some() {
                return Err(IdentityError::Validation(
                    "The Platform ID filter may be supplied only once.".to_owned(),
                ));
            }
            filter.platform_id = Some(Uuid::parse_str(&value).map_err(|_| {
                IdentityError::Validation("The Platform ID filter is invalid.".to_owned())
            })?);
        }
    }
    Ok(filter)
}
fn stack_error(error: StackError) -> IdentityError {
    match error {
        StackError::Validation(message) => IdentityError::Validation(message),
        StackError::NotFound => IdentityError::NotFound,
        StackError::Forbidden => IdentityError::Forbidden,
        StackError::LicenseRequired(capability) => IdentityError::LicenseRequired(capability),
        StackError::Conflict(message)
        | StackError::RuntimeRejected(message)
        | StackError::Runtime(message) => IdentityError::Conflict(message),
        StackError::Storage(message) => IdentityError::Storage(message),
        StackError::Cancelled => {
            IdentityError::Conflict("The Stack operation was cancelled.".to_owned())
        }
    }
}
fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(value)| value)
        .ok_or(IdentityError::Unauthenticated)
}
fn invalid_path(_: PathRejection) -> IdentityError {
    IdentityError::Validation("The Stack path is invalid.".to_owned())
}
fn invalid_query(_: QueryRejection) -> IdentityError {
    IdentityError::Validation("The Stack query is invalid.".to_owned())
}
fn invalid_json(_: JsonRejection) -> IdentityError {
    IdentityError::Validation("The request body is invalid.".to_owned())
}
