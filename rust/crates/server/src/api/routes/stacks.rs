//! Stacks HTTP routes, authorization and local request handling.
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::stacks::{
            requests::{
                ImportDraftQuery, ImportRequest, StackDriftPolicyInput, SwarmPreflightInput,
                ValidateImportRequest, *,
            },
            spec::*,
            views::*,
        },
    },
    openapi::router::OpenApiRouterExt,
    request_validation::{WorkloadQuery, invalid_json, invalid_path, invalid_query},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{
        Extension, Path, Query, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::IntoResponse,
};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, PermissionPolicy, ResourceType};

use citadel_stacks::{
    ImportComposeProject, StackAction, StackError, StackFilter, StackService, permissions as policy,
};

use serde_json::Value;

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct StacksHttpState {
    pub identity: Arc<IdentityService>,
    pub stacks: Arc<StackService>,
}

pub fn router(state: StacksHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<StacksHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(delete))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(apply))
        .normalized_routes(utoipa_axum::routes!(rollback))
        .normalized_routes(utoipa_axum::routes!(preflight))
        .normalized_routes(utoipa_axum::routes!(start))
        .normalized_routes(utoipa_axum::routes!(stop))
        .normalized_routes(utoipa_axum::routes!(pause))
        .normalized_routes(utoipa_axum::routes!(resume))
        .normalized_routes(utoipa_axum::routes!(restart))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(check_updates))
        .normalized_routes(utoipa_axum::routes!(get_config))
        .normalized_routes(utoipa_axum::routes!(duplicate_draft))
        .normalized_routes(utoipa_axum::routes!(releases))
        .normalized_routes(utoipa_axum::routes!(update))
        .normalized_routes(utoipa_axum::routes!(update_metadata))
        .normalized_routes(utoipa_axum::routes!(drift))
        .normalized_routes(utoipa_axum::routes!(update_drift_policy))
        .normalized_routes(utoipa_axum::routes!(reconcile))
        .normalized_routes(utoipa_axum::routes!(import_draft))
        .normalized_routes(utoipa_axum::routes!(validate_import_draft))
        .normalized_routes(utoipa_axum::routes!(import))
}

async fn authorize_git_source(
    state: &StacksHttpState,
    principal: &ActorPrincipal,
    spec: &StackSpec,
    headers: &HeaderMap,
) -> Result<(), crate::api::error::HttpError> {
    let StackSpec::Git { git_repo_id, .. } = spec else {
        return Ok(());
    };
    api_result(
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
) -> Result<(), crate::api::error::HttpError> {
    let Some(git_repository_id) = spec
        .and_then(Value::as_object)
        .and_then(|spec| spec.get("gitRepoId").or_else(|| spec.get("GitRepoId")))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        return Ok(());
    };
    api_result(
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
    ($(#[$name_attr:meta])* $name:ident,$action:expr) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<StacksHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            input: Result<Json<Vec<Uuid>>, JsonRejection>,
        ) -> HttpResult {
            let principal = api_result(require_actor(principal), &headers)?;
            let Json(ids) = api_result(input.map_err(invalid_json), &headers)?;
            for id in &ids {
                authorize::<policy::ChangeStackState>(
                    &state,
                    &principal,
                    *id,

                    &headers,
                )
                .await?;
            }
            api_result(
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

state_action!(
    #[utoipa::path(
    post,
    path = "/api/v1/stacks/start",
    operation_id = "startStacks",
    tag = "Stacks",
    summary = "Start Stacks",
    request_body = ref("#/components/schemas/StackIds"),
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    start,
    StackAction::Start
);

state_action!(
    #[utoipa::path(
    post,
    path = "/api/v1/stacks/stop",
    operation_id = "stopStacks",
    tag = "Stacks",
    summary = "Stop Stacks",
    request_body = ref("#/components/schemas/StackIds"),
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    stop,
    StackAction::Stop
);

state_action!(
    #[utoipa::path(
    post,
    path = "/api/v1/stacks/pause",
    operation_id = "pauseStacks",
    tag = "Stacks",
    summary = "Pause Stacks",
    request_body = ref("#/components/schemas/StackIds"),
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    pause,
    StackAction::Pause
);

state_action!(
    #[utoipa::path(
    post,
    path = "/api/v1/stacks/resume",
    operation_id = "resumeStacks",
    tag = "Stacks",
    summary = "Resume Stacks",
    request_body = ref("#/components/schemas/StackIds"),
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    resume,
    StackAction::Resume
);

state_action!(
    #[utoipa::path(
    post,
    path = "/api/v1/stacks/restart",
    operation_id = "restartStacks",
    tag = "Stacks",
    summary = "Restart Stacks",
    request_body = ref("#/components/schemas/StackIds"),
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
    restart,
    StackAction::Restart
);

fn progress_response(
    mut receiver: tokio::sync::mpsc::Receiver<citadel_stacks::StackProgressItem>,
) -> axum::response::Response {
    let stream = async_stream::stream! {yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));let mut first=true;while let Some(item)=receiver.recv().await{if !first{yield Ok(bytes::Bytes::from_static(b","));}first=false;if let Ok(value)=serde_json::to_vec(&StackStreamItem::from(item)){yield Ok(bytes::Bytes::from(value));}}yield Ok(bytes::Bytes::from_static(b"]"));};
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

async fn actor_and_id<P: PermissionPolicy>(
    state: &StacksHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: &HeaderMap,
) -> HttpResult<(ActorPrincipal, Uuid)> {
    let principal = api_result(require_actor(principal), headers)?;
    let Path(id) = api_result(path.map_err(invalid_path), headers)?;
    authorize::<P>(state, &principal, id, headers).await?;
    Ok((principal, id))
}

async fn authorize<P: PermissionPolicy>(
    state: &StacksHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    headers: &HeaderMap,
) -> HttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    state
        .identity
        .require_resource::<P>(principal, id)
        .await
        .map_err(|error| crate::api::error::HttpError::from_parts(error, headers))
}

async fn collection_capabilities(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    headers: &HeaderMap,
) -> HttpResult<ResourceCapabilities> {
    if principal.is_administrator() {
        return Ok(ResourceCapabilities {
            can_read: true,
            can_write: true,
            can_execute: true,
        });
    }
    let p = identity
        .global_permission(principal, ResourceType::Stack)
        .await
        .map_err(|error| crate::api::error::HttpError::from_parts(error, headers))?;
    Ok(ResourceCapabilities {
        can_read: p
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Read)),
        can_write: p
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Write)),
        can_execute: p
            .as_ref()
            .is_some_and(|value| value.level.grants(PermissionLevel::Execute)),
    })
}

fn stack_error(error: StackError) -> ApiError {
    match error {
        StackError::Validation(message) => crate::request_validation::validation_error(message),
        StackError::NotFound => ApiError::NotFound,
        StackError::Forbidden => ApiError::Forbidden,
        StackError::LicenseRequired(capability) => ApiError::LicenseRequired(capability),
        StackError::Conflict(message)
        | StackError::RuntimeRejected(message)
        | StackError::Runtime(message) => ApiError::Conflict(message),
        source @ StackError::Storage(_) => ApiError::internal(source),
        StackError::Cancelled => {
            ApiError::Conflict("The Stack operation was cancelled.".to_owned())
        }
    }
}

fn require_actor(principal: Option<Extension<ActorPrincipal>>) -> Result<ActorPrincipal, ApiError> {
    principal
        .map(|Extension(value)| value)
        .ok_or(ApiError::Unauthenticated)
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/preflight/swarm",
    operation_id = "preflightSwarmStack",
    tag = "Stacks",
    summary = "Validate Docker Swarm Stack compatibility",
    request_body = SwarmPreflightInput,
    responses(
        (status = 200, description = "Success", body = SwarmStackCompatibilityReport, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn preflight(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<SwarmPreflightInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<policy::CreateStack>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let value = api_result(
        state
            .stacks
            .preflight_swarm(
                &input.compose_files,
                &input
                    .build_image_bindings
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<_>>(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(SwarmStackCompatibilityReport::from(value)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}",
    operation_id = "getComposeProjectImportDraft",
    tag = "Platforms",
    summary = "Get a Compose or Swarm Stack import draft",
    responses(
        (status = 200, description = "Success", body = ComposeProjectImportDraftView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("projectName" = String, Path), ("importKind" = Option<StackImportKind>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn import_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ImportDraftQuery>, QueryRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = api_result(path.map_err(invalid_path), &headers)?;
    let Query(query) = api_result(query.map_err(invalid_query), &headers)?;
    api_result(
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
    let value = api_result(
        state
            .stacks
            .import_draft(
                platform_id,
                &project_name,
                query.import_kind.map(Into::into),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ComposeProjectImportDraftView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}/import-draft",
    operation_id = "validateComposeProjectImportDraft",
    tag = "Platforms",
    summary = "Validate a source for an unmanaged Compose project",
    request_body = ValidateImportRequest,
    responses(
        (status = 200, description = "Success", body = ComposeProjectImportValidation, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("projectName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn validate_import_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ValidateImportRequest>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<policy::CreateStack>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    let value = api_result(
        state
            .stacks
            .validate_import(
                platform_id,
                &project_name,
                &input.name,
                input.stack_source.into(),
                &input.spec.into(),
                input.import_kind.map(Into::into),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ComposeProjectImportValidation::from(value)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}/import",
    operation_id = "importComposeProject",
    tag = "Platforms",
    summary = "Import a Compose project or Swarm Stack",
    request_body = ImportRequest,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("projectName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn import(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ImportRequest>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = api_result(path.map_err(invalid_path), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<policy::CreateStack>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    if citadel_stacks::StackSource::from(input.stack_source)
        != citadel_stacks::StackSpec::from(input.spec.clone()).source()
    {
        return Err(crate::api::error::HttpError::from_parts(
            ApiError::Validation("Stack source and specification type must match.".to_owned()),
            &headers,
        ));
    }
    if input.import_sensitive_environment_as_secrets {
        return Err(crate::api::error::HttpError::from_parts(
            ApiError::Validation(
                "Importing newly detected sensitive values requires the Phase 7 Secret execution slice."
                    .to_owned(),
            ),
            &headers,
        ));
    }
    let import_kind = match input.import_kind {
        Some(value) => value.into(),
        None => {
            api_result(
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
    let input = ImportComposeProject {
        name: input.name,
        platform_id,
        project_name,
        description: input.description,
        spec: input.spec.into(),
        tag_ids: input.tag_ids,
        import_kind,
        preview_fingerprint: input.preview_fingerprint,
        detected_secret_values: std::collections::BTreeMap::new(),
    };
    let value = api_result(
        state
            .stacks
            .import(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks",
    operation_id = "createStack",
    tag = "Stacks",
    summary = "Create a Stack",
    request_body = CreateStackInput,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateStackInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    api_result(
        state
            .identity
            .require_scope::<policy::CreateStack>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    let input = api_result(
        citadel_stacks::CreateStack::try_from(input).map_err(stack_error),
        &headers,
    )?;
    let value = api_result(
        state
            .stacks
            .create(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/stacks/{id}",
    operation_id = "updateStack",
    tag = "Stacks",
    summary = "Update Stack configuration",
    request_body(content(
        (PatchStackInput = "application/merge-patch+json"),
        (PatchStackInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchStackInput>, JsonRejection>,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize_git_patch_source(&state, &principal, input.spec.as_ref(), &headers).await?;
    let value = api_result(
        state
            .stacks
            .update(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/stacks/{id}/_metadata",
    operation_id = "updateStackMetadata",
    tag = "Stacks",
    summary = "Update Stack metadata",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_metadata(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let description = match input.as_object().and_then(|value| value.get("description")) {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) => None,
        Some(_) => {
            return Err(crate::api::error::HttpError::from_parts(
                ApiError::Validation("Description must be a string or null.".to_owned()),
                &headers,
            ));
        }
        None => {
            return crate::api::routes::stacks::get(
                State(state),
                Some(Extension(principal)),
                Ok(Path(id)),
                headers,
            )
            .await;
        }
    };
    let value = api_result(
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
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/rename",
    operation_id = "renameStack",
    tag = "Stacks",
    summary = "Rename a Stack",
    request_body = RenameStackInput,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameStackInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::WriteStack>(&state, &principal, input.id, &headers).await?;
    let value = api_result(
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
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/stacks",
    operation_id = "deleteStacks",
    tag = "Stacks",
    summary = "Delete Stacks",
    request_body = Vec<Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<Vec<Uuid>>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(ids) = api_result(input.map_err(invalid_json), &headers)?;
    for id in &ids {
        authorize::<policy::DeleteStack>(&state, &principal, *id, &headers).await?;
    }
    api_result(
        state
            .stacks
            .delete(principal.actor_id, principal.is_administrator(), &ids)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/{stackId}/check-updates",
    operation_id = "checkStackUpdates",
    tag = "Stacks",
    summary = "Check a Stack source for updates",
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn check_updates(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let value = api_result(
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
                StackError::Runtime(message) => ApiError::External(message),
                other => stack_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/apply",
    operation_id = "applyStack",
    tag = "Stacks",
    summary = "Apply a Stack and stream progress",
    request_body = ApplyStackInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/StackStreamItems"), content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn apply(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ApplyStackInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::ApplyStack>(&state, &principal, input.id, &headers).await?;
    let receiver = api_result(
        state
            .stacks
            .apply(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(progress_response(receiver))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/rollback",
    operation_id = "rollbackStack",
    tag = "Stacks",
    summary = "Roll back a Stack and stream progress",
    request_body = RollbackStackInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/StackStreamItems"), content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rollback(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RollbackStackInput>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::RollbackStack>(&state, &principal, input.stack_id, &headers).await?;
    let receiver = api_result(
        state
            .stacks
            .rollback(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(progress_response(receiver))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/drift",
    operation_id = "getStackDrift",
    tag = "Stacks",
    summary = "Get Stack drift",
    responses(
        (status = 200, description = "Success", body = StackDriftReport, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn drift(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStack>(&state, principal, path, &headers).await?;
    let value = api_result(
        state
            .stacks
            .drift(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackDriftReport::from(value)).into_response(),
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/stacks/{stackId}/drift-policy",
    operation_id = "updateStackDriftPolicy",
    tag = "Stacks",
    summary = "Update Stack drift policy",
    request_body = StackDriftPolicyInput,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_drift_policy(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<StackDriftPolicyInput>, JsonRejection>,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let Json(input) = api_result(input.map_err(invalid_json), &headers)?;
    let value = api_result(
        state
            .stacks
            .update_drift_policy(
                principal.actor_id,
                principal.is_administrator(),
                id,
                citadel_stacks::StackDriftPolicy::from(input),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/{stackId}/reconcile",
    operation_id = "reconcileStack",
    tag = "Stacks",
    summary = "Reconcile safe Stack drift",
    responses(
        (status = 200, description = "Success", body = StackReconciliationResult, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn reconcile(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReconcileStack>(&state, principal, path, &headers).await?;
    let value = api_result(
        state
            .stacks
            .reconcile_drift(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackReconciliationResult::from(value)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks",
    operation_id = "listStacks",
    tag = "Stacks",
    summary = "List authorized Stacks",
    responses(
        (status = 200, description = "Success", body = StacksView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("tags" = Option<Vec<String>>, Query), ("platformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    filter: Result<WorkloadQuery, crate::api::error::HttpError>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_actor(principal), &headers)?;
    let filter = filter?;
    let filter = StackFilter {
        tags: filter.tags,
        platform_id: filter.platform_id,
    };
    let capabilities = collection_capabilities(&state.identity, &principal, &headers).await?;
    let value = api_result(
        state
            .stacks
            .list(principal.actor_id, principal.is_administrator(), &filter)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StacksView {
            stacks: value.into_iter().map(StackView::from).collect(),
            capabilities,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}",
    operation_id = "getStack",
    tag = "Stacks",
    summary = "Get a Stack",
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStack>(&state, principal, path, &headers).await?;
    let value = api_result(
        state
            .stacks
            .get(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/_cfg",
    operation_id = "getStackConfig",
    tag = "Stacks",
    summary = "Get Stack configuration",
    responses(
        (status = 200, description = "Success", body = StackConfigView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_config(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStack>(&state, principal, path, &headers).await?;
    let value = api_result(
        state
            .stacks
            .get_config(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackConfigView::from(value)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/duplicate-draft",
    operation_id = "getStackDuplicateDraft",
    tag = "Stacks",
    summary = "Build a Stack duplicate draft",
    responses(
        (status = 200, description = "Success", body = StackDuplicateDraftView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn duplicate_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStackBindings>(&state, principal, path, &headers).await?;
    let value = api_result(
        state
            .stacks
            .duplicate_draft(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackDuplicateDraftView::from(value)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/releases",
    operation_id = "listStackReleases",
    tag = "Stacks",
    summary = "List previous healthy Stack releases",
    responses(
        (status = 200, description = "Success", body = StackReleasesView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn releases(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> HttpResult {
    let (principal, id) =
        actor_and_id::<policy::ViewStackReleases>(&state, principal, path, &headers).await?;
    let value = api_result(
        state
            .stacks
            .releases(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackReleasesView {
            releases: value.into_iter().map(StackReleaseView::from).collect(),
        })
        .into_response(),
    ))
}
