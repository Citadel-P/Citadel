//! Administrator-only instance update status and explicit release checks.
use crate::{
    api::{
        error::{ApiError, error_response, no_store},
        resource_access::require_human_administrator,
        resources::updates::UpdateStatusView,
    },
    openapi::router::OpenApiRouterExt,
};
use axum::{
    Json, Router,
    extract::{Extension, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use citadel_identity::{ActorPrincipal, IdentityService};
use citadel_licensing::LicenseService;
use citadel_updates::{CheckState, ReleaseCheckPolicy, ReleaseCheckService};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct UpdateHttpState {
    pub identity: Arc<IdentityService>,
    pub checker: ReleaseCheckService,
    pub licenses: Arc<LicenseService>,
}

pub fn router(state: UpdateHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}
pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<UpdateHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(status))
        .normalized_routes(utoipa_axum::routes!(check))
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
    let settings = match state.identity.get_installation_settings(&principal).await {
        Ok(settings) => settings,
        Err(error) => return error_response(error, &headers),
    };
    let checks = match state
        .checker
        .check_now(
            ReleaseCheckPolicy {
                initialized: settings.initialized,
                checks_allowed: state.identity.installation_policy().update_checks_allowed,
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
        let settings = state.identity.get_installation_settings(principal).await?;
        let request = state.licenses.request().await?;
        Ok::<_, ApiError>(UpdateStatusView::new(
            state.checker.describe(checks),
            request.instance_id,
            state.identity.installation_policy().update_checks_allowed,
            settings.update_checks_enabled,
        ))
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
