//! Administrator-only instance update status and explicit release checks.
use crate::{
    api::{
        error::{ApiError, error_response, no_store},
        resource_access::require_human_administrator,
        resources::updates::{
            ApproveUpdateInput, ReviewUpdateInput, UpdateOperationView, UpdateReviewView,
            UpdateStatusView,
        },
    },
    openapi::router::OpenApiRouterExt,
};
use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use citadel_identity::ActorPrincipal;
use citadel_licensing::LicenseService;
use citadel_updates::{CheckState, ReleaseCheckPolicy, ReleaseCheckService};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct UpdateHttpState {
    pub instance: Arc<citadel_instance::InstanceService>,
    pub checker: ReleaseCheckService,
    pub installer: Option<citadel_updates::UpdateInstallService>,
    pub agents: Option<citadel_updates::AgentUpdateService>,
    pub licenses: Arc<LicenseService>,
}

pub fn router(state: UpdateHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}
pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<UpdateHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .merge(super::agents::routes())
        .normalized_routes(utoipa_axum::routes!(status))
        .normalized_routes(utoipa_axum::routes!(check))
        .normalized_routes(utoipa_axum::routes!(review))
        .normalized_routes(utoipa_axum::routes!(approve))
        .normalized_routes(utoipa_axum::routes!(operation))
}

#[utoipa::path(get, path="/api/v1/updates", operation_id="getInstanceUpdates", tag="Updates", summary="Get instance update status",
    responses((status=200, description="Success", body=UpdateStatusView), crate::openapi::errors::AccessErrors), security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn status(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(value) => value,
        Err(error) => return error_response(error, &headers),
    };
    let checks = match state.checker.status().await {
        Ok(checks) => checks,
        Err(error) => return error_response(error, &headers),
    };
    respond(&state, &principal, &headers, checks).await
}

#[utoipa::path(post, path="/api/v1/updates/check", operation_id="checkInstanceUpdates", tag="Updates", summary="Check official releases once without installing",
    responses((status=200, description="Success", body=UpdateStatusView), crate::openapi::errors::AccessErrors), security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn check(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(value) => value,
        Err(error) => return error_response(error, &headers),
    };
    let settings = match state.instance.get_installation_settings(&principal).await {
        Ok(settings) => settings,
        Err(error) => return error_response(error, &headers),
    };
    let checks = match state
        .checker
        .check_now(
            ReleaseCheckPolicy {
                initialized: settings.initialized,
                checks_allowed: state.instance.installation_policy().update_checks_allowed,
            },
            &CancellationToken::new(),
        )
        .await
    {
        Ok(checks) => checks,
        Err(error) => return error_response(error, &headers),
    };
    respond(&state, &principal, &headers, checks).await
}

async fn respond(
    state: &UpdateHttpState,
    principal: &ActorPrincipal,
    headers: &HeaderMap,
    checks: CheckState,
) -> Response {
    let result = async {
        let settings = state.instance.get_installation_settings(principal).await?;
        let request = state.licenses.request().await?;
        let mut view = UpdateStatusView::new(
            state.checker.describe(checks),
            request.instance_id,
            state.instance.installation_policy().update_checks_allowed,
            settings.update_checks_enabled,
        );
        view.database_version = state.checker.database_version().await;
        let cancellation = CancellationToken::new();
        let registered = state.installer.as_ref().is_some_and(|installer| installer.registered());
        let database = if registered {
            citadel_updates::DatabaseInstallation::Bundled
        } else {
            state.checker.database_installation(&cancellation).await
        };
        view.database_installation = database.into();
        view.database_update_reason = Some(match database {
            citadel_updates::DatabaseInstallation::Bundled => "In-app updates are not configured for this Compose installation. Complete the one-time update setup on the Docker host. This message does not mean a PostgreSQL upgrade is available.",
            citadel_updates::DatabaseInstallation::External => "PostgreSQL is managed outside this Citadel Compose installation. Update it using its own deployment or provider.",
            citadel_updates::DatabaseInstallation::Unknown => "Could not verify how PostgreSQL is deployed. Check Core’s local Docker access and Compose configuration.",
        }.into());
        if let Some(installer) = &state.installer {
            view.operation = installer.active().await?.map(Into::into);
            // Registration is verified at startup or by the supervised setup task.
            // An active Core operation must not make its bundled database look external.
            if registered || database == citadel_updates::DatabaseInstallation::Bundled {
                view.set_database_update(installer.database_update(&cancellation).await);
            }
            if !installer.registered() {
                view.installation_unavailable_reason = Some("Automatic update setup has not completed. Check Core logs for setup diagnostics.".into());
            } else if state.instance.installation_policy().update_checks_allowed {
                view.installation_unavailable_reason = None;
            }
        }
        Ok::<_, ApiError>(view)
    }
    .await;
    match result {
        Ok(value) => no_store(Json(value).into_response()),
        Err(error) => error_response(error, headers),
    }
}

impl From<citadel_updates::UpdateError> for ApiError {
    fn from(error: citadel_updates::UpdateError) -> Self {
        use citadel_updates::UpdateError::*;
        match error {
            Forbidden => Self::Forbidden,
            NotFound => Self::NotFound,
            Validation(message) => Self::Validation(message),
            Conflict(message) | Unavailable(message) => Self::Conflict(message),
            Cancelled => Self::Conflict("Update preparation was cancelled.".into()),
            other @ Storage(_) => Self::internal(other),
        }
    }
}

fn installation(
    state: &UpdateHttpState,
) -> Result<&citadel_updates::UpdateInstallService, ApiError> {
    if !state.instance.installation_policy().update_checks_allowed {
        return Err(ApiError::Forbidden);
    }
    state.installer.as_ref().ok_or_else(|| {
        ApiError::Conflict("This installation is not registered for updates.".into())
    })
}
pub(super) fn actor(principal: ActorPrincipal) -> citadel_updates::UpdateActor {
    citadel_updates::UpdateActor {
        subject_id: principal.subject_id,
        actor_id: principal.actor_id,
    }
}

#[utoipa::path(post, path="/api/v1/updates/review", operation_id="reviewInstanceUpdate", tag="Updates", summary="Review a verified release without installing", request_body=ReviewUpdateInput,
    responses((status=200, description="Success", body=UpdateReviewView), crate::openapi::errors::AccessErrors), security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn review(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<ReviewUpdateInput>,
) -> Response {
    let result = async {
        let principal = require_human_administrator(principal)?;
        let installer = installation(&state)?;
        if input.database {
            return installer
                .review_database(actor(principal))
                .await
                .map(UpdateReviewView::from)
                .map_err(ApiError::from);
        }
        let notice = state.checker.status().await?.available.ok_or_else(|| {
            ApiError::Conflict("Check for updates before reviewing a release.".into())
        })?;
        if notice.version != input.version {
            return Err(ApiError::Conflict(
                "The available release changed. Check for updates again.".into(),
            ));
        }
        installer
            .review(actor(principal), &input.version)
            .await
            .map(UpdateReviewView::from)
            .map_err(ApiError::from)
    }
    .await;
    match result {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(post, path="/api/v1/updates/approve", operation_id="approveInstanceUpdate", tag="Updates", summary="Approve a reviewed instance update", request_body=ApproveUpdateInput,
    responses((status=200, description="Success", body=UpdateOperationView), crate::openapi::errors::AccessErrors), security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn approve(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<ApproveUpdateInput>,
) -> Response {
    let result = async {
        let principal = require_human_administrator(principal)?;
        installation(&state)?
            .approve(actor(principal), input.review_id, input.request_id)
            .await
            .map(UpdateOperationView::from)
            .map_err(ApiError::from)
    }
    .await;
    match result {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(get, path="/api/v1/updates/operations/{id}", operation_id="getInstanceUpdateOperation", tag="Updates", summary="Read an instance update operation",
    params(("id" = uuid::Uuid, Path, description="Operation ID")),
    responses((status=200, description="Success", body=UpdateOperationView), crate::openapi::errors::AccessErrors), security(("Bearer"=[])),
    extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn operation(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Path(id): Path<uuid::Uuid>,
) -> Response {
    let result = async {
        require_human_administrator(principal)?;
        let installer = state.installer.as_ref().ok_or(ApiError::NotFound)?;
        installer
            .operation(id)
            .await
            .map(UpdateOperationView::from)
            .map_err(ApiError::from)
    }
    .await;
    match result {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => error_response(error, &headers),
    }
}
