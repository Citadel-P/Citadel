use crate::request_validation::ApiPath;

use crate::request_validation::ValidatedJson;

use std::sync::Arc;

use axum::Json;

use axum::Router;

use axum::extract::{Extension, State};

use axum::http::{HeaderMap, StatusCode};

use axum::response::{IntoResponse, Response};

use citadel_identity::ActorPrincipal;

use citadel_identity::{IdentityError, ProfileService};

use crate::identity_http::dto::{
    ChangeCurrentPasswordRequest, PatchUserPreferencesRequest, UpdateCurrentProfileRequest,
};

use crate::identity_http::{current_refresh_token, identity_error_response, no_store};

use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct ProfileHttpState {
    pub profiles: Arc<ProfileService>,
}

pub fn router(state: ProfileHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/profile/change-password",
    operation_id = "changeCurrentPassword",
    tag = "Profile",
    summary = "Change current profile password",
    request_body = ChangeCurrentPasswordRequest,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn change_password(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<ChangeCurrentPasswordRequest>,
) -> Response {
    let request: citadel_identity::ChangeCurrentPassword = request.into();
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state
        .profiles
        .change_password(&principal, current_refresh_token(&headers), request)
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/profile/sessions",
    operation_id = "listProfileSessions",
    tag = "Profile",
    summary = "List current profile sessions",
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserSessionsView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_sessions(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state
        .profiles
        .list_sessions(&principal, current_refresh_token(&headers))
        .await
    {
        Ok(sessions) => no_store(
            Json(crate::identity_http::dto::UserSessionsView::from(sessions)).into_response(),
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/profile/sessions/{sessionId}",
    operation_id = "revokeProfileSession",
    tag = "Profile",
    summary = "Revoke a profile session",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("sessionId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn revoke_session(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(session_id): ApiPath<uuid::Uuid>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state
        .profiles
        .revoke_session(&principal, current_refresh_token(&headers), session_id)
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/profile/sessions",
    operation_id = "revokeOtherProfileSessions",
    tag = "Profile",
    summary = "Revoke other profile sessions",
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::RevokeOtherProfileSessionsView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn revoke_other_sessions(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state
        .profiles
        .revoke_other_sessions(&principal, current_refresh_token(&headers))
        .await
    {
        Ok(result) => no_store(
            Json(crate::identity_http::dto::RevokeOtherProfileSessionsView::from(result))
                .into_response(),
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/profile/preferences",
    operation_id = "getProfilePreferences",
    tag = "Profile",
    summary = "Get current profile preferences",
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserPreferencesView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_preferences(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.get_preferences(&principal).await {
        Ok(preferences) => no_store(
            Json(crate::identity_http::dto::UserPreferencesView::from(
                preferences,
            ))
            .into_response(),
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    patch,
    path = "/api/v1/profile/preferences",
    operation_id = "patchProfilePreferences",
    tag = "Profile",
    summary = "Update current profile preferences",
    request_body(content(
        (PatchUserPreferencesRequest = "application/merge-patch+json"),
        (PatchUserPreferencesRequest = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserPreferencesView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn patch_preferences(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<PatchUserPreferencesRequest>,
) -> Response {
    let request: citadel_identity::PatchUserPreferences = request.into();
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.patch_preferences(&principal, request).await {
        Ok(preferences) => no_store(
            Json(crate::identity_http::dto::UserPreferencesView::from(
                preferences,
            ))
            .into_response(),
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/profile",
    operation_id = "getCurrentProfile",
    tag = "Profile",
    summary = "Get current profile",
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::CurrentProfileView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_current(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.get(&principal).await {
        Ok(profile) => no_store(
            Json(crate::identity_http::dto::CurrentProfileView::from(profile)).into_response(),
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    patch,
    path = "/api/v1/profile",
    operation_id = "updateCurrentProfile",
    tag = "Profile",
    summary = "Update current profile",
    request_body = UpdateCurrentProfileRequest,
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::CurrentProfileView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_current(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<UpdateCurrentProfileRequest>,
) -> Response {
    let request: citadel_identity::UpdateCurrentProfile = request.into();
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.update(&principal, request).await {
        Ok(profile) => no_store(
            Json(crate::identity_http::dto::CurrentProfileView::from(profile)).into_response(),
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<ProfileHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(get_current))
        .normalized_routes(utoipa_axum::routes!(update_current))
        .normalized_routes(utoipa_axum::routes!(get_preferences))
        .normalized_routes(utoipa_axum::routes!(patch_preferences))
        .normalized_routes(utoipa_axum::routes!(change_password))
        .normalized_routes(utoipa_axum::routes!(list_sessions))
        .normalized_routes(utoipa_axum::routes!(revoke_other_sessions))
        .normalized_routes(utoipa_axum::routes!(revoke_session))
}
