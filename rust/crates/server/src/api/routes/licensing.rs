//! Licensing HTTP routes, authorization and local request handling.
use crate::{
    api::{
        error::{ApiError, error_response, no_store},
        resources::licensing::{requests::InstallLicenseRequest, views},
        routes::authentication::require_human_administrator,
    },
    openapi::router::OpenApiRouterExt,
};

use axum::{
    Json, Router,
    extract::{Extension, State, rejection::JsonRejection},
    http::HeaderMap,
    response::{IntoResponse, Response},
};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_licensing::{LicenseError, LicenseService};

use citadel_primitives::{PermissionLevel, ResourceType};

use std::sync::Arc;

#[derive(Clone)]
pub struct LicenseHttpState {
    pub identity: Arc<IdentityService>,
    pub licenses: Arc<LicenseService>,
}

pub fn router(state: LicenseHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<LicenseHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(entitlements))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(install))
        .normalized_routes(utoipa_axum::routes!(remove))
        .normalized_routes(utoipa_axum::routes!(request))
}

#[utoipa::path(
    get,
    path = "/api/v1/license/entitlements",
    operation_id = "getLicenseEntitlements",
    tag = "License",
    summary = "Get effective license entitlements",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::licensing::views::LicenseEntitlementsView, content_type = "application/json"),
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
        return error_response(ApiError::Unauthenticated, &headers);
    }
    match state.licenses.entitlements().await {
        Ok(state) => no_store(Json(views::entitlements_view(&state)).into_response()),
        Err(error) => license_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/license",
    operation_id = "getLicense",
    tag = "License",
    summary = "Get installed license state",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::licensing::views::LicenseView, content_type = "application/json"),
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
        return error_response(error, &headers);
    }
    match state.licenses.get().await {
        Ok(state) => no_store(Json(views::to_view(&state)).into_response()),
        Err(error) => license_error_response(error, &headers),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/license",
    operation_id = "installLicense",
    tag = "License",
    summary = "Install or replace a license",
    request_body = InstallLicenseRequest,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::licensing::views::LicenseView, content_type = "application/json"),
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
        Err(error) => return error_response(error, &headers),
    };
    let request = match payload {
        Ok(Json(request)) => request,
        Err(error) => {
            return error_response(ApiError::Validation(error.body_text()), &headers);
        }
    };
    match state
        .licenses
        .install(&request.license, principal.actor_id)
        .await
    {
        Ok(state) => no_store(Json(views::to_view(&state)).into_response()),
        Err(error) => license_error_response(error, &headers),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/license",
    operation_id = "removeLicense",
    tag = "License",
    summary = "Remove the installed license",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::licensing::views::LicenseView, content_type = "application/json"),
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
        Err(error) => return error_response(error, &headers),
    };
    match state.licenses.remove(principal.actor_id).await {
        Ok(state) => no_store(Json(views::to_view(&state)).into_response()),
        Err(error) => license_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/license/request",
    operation_id = "getLicenseRequest",
    tag = "License",
    summary = "Get license request metadata",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::licensing::views::LicenseRequestView, content_type = "application/json"),
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
        return error_response(error, &headers);
    }
    match state.licenses.request().await {
        Ok(state) => no_store(Json(views::request_view(state)).into_response()),
        Err(error) => license_error_response(error, &headers),
    }
}

async fn authorize(
    state: &LicenseHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    level: PermissionLevel,
) -> Result<ActorPrincipal, ApiError> {
    let principal = require_human_administrator(principal)?;
    state
        .identity
        .authorize(&principal, ResourceType::License, level, None)
        .await?;
    Ok(principal)
}

impl From<LicenseError> for ApiError {
    fn from(error: LicenseError) -> Self {
        match error {
            LicenseError::Validation(message) => Self::Validation(message),
            LicenseError::Conflict(message) => Self::Conflict(message),
            LicenseError::ReplacementMismatch => Self::TypedConflict {
                problem_type: "https://citadel.local/problems/license-replacement-mismatch",
                message: error.to_string(),
            },
            LicenseError::ReplacementNotYetEffective => Self::TypedConflict {
                problem_type: "https://citadel.local/problems/license-replacement-not-yet-effective",
                message: error.to_string(),
            },
            source @ (LicenseError::Storage(_) | LicenseError::InvalidKey) => {
                Self::internal(source)
            }
        }
    }
}

fn license_error_response(error: LicenseError, headers: &HeaderMap) -> Response {
    error_response(error, headers)
}
