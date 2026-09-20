use super::{requests::*, spec::*, views::*};

use citadel_stacks::permissions as policy;

use citadel_primitives::PermissionPolicy;

use crate::request_validation::WorkloadQuery;

use crate::request_validation::{invalid_json, invalid_path, invalid_query};

use std::sync::Arc;

use axum::body::Body;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};

use axum::extract::{Extension, Path, Query, State};

use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};

use axum::http::{HeaderMap, HeaderValue, StatusCode};

use axum::response::IntoResponse;

use axum::{Json, Router};

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};

use citadel_stacks::{
    ImportComposeProject, StackAction, StackChangeNotifier, StackError, StackFilter, StackService,
};

use serde::Deserialize;

use serde_json::Value;

use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

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
    documented_routes().split_for_parts().0.with_state(state)
}

async fn authorize_git_source(
    state: &StacksHttpState,
    principal: &ActorPrincipal,
    spec: &StackSpec,
    headers: &HeaderMap,
) -> Result<(), crate::identity_http::IdentityHttpError> {
    let StackSpec::Git { git_repo_id, .. } = spec else {
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
    ($(#[$name_attr:meta])* $name:ident,$action:expr) => {
        $(#[$name_attr])*
        async fn $name(
            State(state): State<StacksHttpState>,
            principal: Option<Extension<ActorPrincipal>>,
            headers: HeaderMap,
            input: Result<Json<Vec<Uuid>>, JsonRejection>,
        ) -> IdentityHttpResult {
            let principal = identity_result(require_actor(principal), &headers)?;
            let Json(ids) = identity_result(input.map_err(invalid_json), &headers)?;
            for id in &ids {
                authorize::<policy::ChangeStackState>(
                    &state,
                    &principal,
                    *id,

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
) -> IdentityHttpResult<(ActorPrincipal, Uuid)> {
    let principal = identity_result(require_actor(principal), headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), headers)?;
    authorize::<P>(state, &principal, id, headers).await?;
    Ok((principal, id))
}

async fn authorize<P: PermissionPolicy>(
    state: &StacksHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if principal.is_administrator() {
        return Ok(());
    }
    state
        .identity
        .require_resource::<P>(principal, id)
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
            can_read: true,
            can_write: true,
            can_execute: true,
        });
    }
    let p = identity
        .global_permission(principal, ResourceType::Stack)
        .await
        .map_err(|error| crate::identity_http::IdentityHttpError::from_parts(error, headers))?;
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

fn stack_error(error: StackError) -> IdentityError {
    match error {
        StackError::Validation(message) => crate::request_validation::validation_error(message),
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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<StacksHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(read::list))
        .normalized_routes(utoipa_axum::routes!(mutations::create))
        .normalized_routes(utoipa_axum::routes!(mutations::delete))
        .normalized_routes(utoipa_axum::routes!(mutations::rename))
        .normalized_routes(utoipa_axum::routes!(operations::apply))
        .normalized_routes(utoipa_axum::routes!(operations::rollback))
        .normalized_routes(utoipa_axum::routes!(adoption::preflight))
        .normalized_routes(utoipa_axum::routes!(start))
        .normalized_routes(utoipa_axum::routes!(stop))
        .normalized_routes(utoipa_axum::routes!(pause))
        .normalized_routes(utoipa_axum::routes!(resume))
        .normalized_routes(utoipa_axum::routes!(restart))
        .normalized_routes(utoipa_axum::routes!(read::get))
        .normalized_routes(utoipa_axum::routes!(operations::check_updates))
        .normalized_routes(utoipa_axum::routes!(read::get_config))
        .normalized_routes(utoipa_axum::routes!(read::duplicate_draft))
        .normalized_routes(utoipa_axum::routes!(read::releases))
        .normalized_routes(utoipa_axum::routes!(mutations::update))
        .normalized_routes(utoipa_axum::routes!(mutations::update_metadata))
        .normalized_routes(utoipa_axum::routes!(operations::drift))
        .normalized_routes(utoipa_axum::routes!(operations::update_drift_policy))
        .normalized_routes(utoipa_axum::routes!(operations::reconcile))
        .normalized_routes(utoipa_axum::routes!(adoption::import_draft))
        .normalized_routes(utoipa_axum::routes!(adoption::validate_import_draft))
        .normalized_routes(utoipa_axum::routes!(adoption::import))
}
mod adoption;

mod operations;

mod read;

mod mutations;
