use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use citadel_application::{InstallLicenseRequest, LicenseService};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};

use crate::identity_http::{identity_error_response, no_store, require_human_administrator};
use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct LicenseHttpState {
    pub identity: Arc<IdentityService>,
    pub licenses: Arc<LicenseService>,
}

pub fn router(state: LicenseHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[utoipa::path(
    get,
    path = "/api/v1/license/entitlements",
    operation_id = "getLicenseEntitlements",
    summary = "Get effective license entitlements",
    responses(
        (status = 200, description = "Success", body = citadel_application::LicenseEntitlementsView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn entitlements(
    State(state): State<LicenseHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    if principal.is_none() {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    }
    match state.licenses.entitlements().await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/license",
    operation_id = "getLicense",
    summary = "Get installed license state",
    responses(
        (status = 200, description = "Success", body = citadel_application::LicenseView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(state): State<LicenseHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = authorize(&state, principal, PermissionLevel::Read).await {
        return identity_error_response(error, &headers);
    }
    match state.licenses.get().await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/license",
    operation_id = "installLicense",
    summary = "Install or replace a license",
    request_body = InstallLicenseRequest,
    responses(
        (status = 200, description = "Success", body = citadel_application::LicenseView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn install(
    State(state): State<LicenseHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    payload: Result<Json<InstallLicenseRequest>, JsonRejection>,
) -> Response {
    let principal = match authorize(&state, principal, PermissionLevel::Write).await {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let request = match payload {
        Ok(Json(request)) => request,
        Err(error) => {
            return identity_error_response(IdentityError::Validation(error.body_text()), &headers);
        }
    };
    match state
        .licenses
        .install(&request.license, principal.actor_id)
        .await
    {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/license",
    operation_id = "removeLicense",
    summary = "Remove the installed license",
    responses(
        (status = 200, description = "Success", body = citadel_application::LicenseView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove(
    State(state): State<LicenseHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let principal = match authorize(&state, principal, PermissionLevel::Execute).await {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    match state.licenses.remove(principal.actor_id).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/license/request",
    operation_id = "getLicenseRequest",
    summary = "Get license request metadata",
    responses(
        (status = 200, description = "Success", body = citadel_application::LicenseRequestView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn request(
    State(state): State<LicenseHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = authorize(&state, principal, PermissionLevel::Read).await {
        return identity_error_response(error, &headers);
    }
    match state.licenses.request().await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn authorize(
    state: &LicenseHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    level: PermissionLevel,
) -> Result<ActorPrincipal, IdentityError> {
    let principal = require_human_administrator(principal)?;
    state
        .identity
        .authorize(&principal, ResourceType::License, level, None)
        .await?;
    Ok(principal)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<LicenseHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(entitlements))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(install))
        .normalized_routes(utoipa_axum::routes!(remove))
        .normalized_routes(utoipa_axum::routes!(request))
}
