use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use citadel_application::{
    ChangeCurrentPasswordRequest, IdentityError, PatchUserPreferencesRequest, ProfileService,
    UpdateCurrentProfileRequest,
};
use citadel_domain::ActorPrincipal;

use crate::identity_http::{current_refresh_token, identity_error_response, no_store};

#[derive(Clone)]
pub struct ProfileHttpState {
    pub profiles: Arc<ProfileService>,
}

pub fn router(state: ProfileHttpState) -> Router {
    Router::new()
        .route("/api/v1/profile", get(get_current).patch(update_current))
        .route(
            "/api/v1/profile/preferences",
            get(get_preferences).patch(patch_preferences),
        )
        .route("/api/v1/profile/change-password", post(change_password))
        .route(
            "/api/v1/profile/sessions",
            get(list_sessions).delete(revoke_other_sessions),
        )
        .route(
            "/api/v1/profile/sessions/{sessionId}",
            delete(revoke_session),
        )
        .with_state(state)
}

async fn change_password(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<ChangeCurrentPasswordRequest>,
) -> Response {
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
        Ok(sessions) => no_store(Json(sessions).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn revoke_session(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(session_id): Path<uuid::Uuid>,
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
        Ok(result) => no_store(Json(result).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get_preferences(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.get_preferences(&principal).await {
        Ok(preferences) => no_store(Json(preferences).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn patch_preferences(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<PatchUserPreferencesRequest>,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.patch_preferences(&principal, request).await {
        Ok(preferences) => no_store(Json(preferences).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get_current(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.get(&principal).await {
        Ok(profile) => no_store(Json(profile).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn update_current(
    State(state): State<ProfileHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<UpdateCurrentProfileRequest>,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.profiles.update(&principal, request).await {
        Ok(profile) => no_store(Json(profile).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}
