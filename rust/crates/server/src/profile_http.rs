use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use citadel_application::{
    ChangeCurrentPasswordRequest, IdentityError, PatchUserPreferencesRequest, ProfileService,
    UpdateCurrentProfileRequest,
};
use citadel_contracts::http::routes;
use citadel_domain::ActorPrincipal;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{current_refresh_token, identity_error_response, no_store};

#[derive(Clone)]
pub struct ProfileHttpState {
    pub profiles: Arc<ProfileService>,
}

pub fn router(state: ProfileHttpState) -> Router {
    Router::new()
        .contract_route(routes::GET_CURRENT_PROFILE, get_current)
        .contract_route(routes::UPDATE_CURRENT_PROFILE, update_current)
        .contract_route(routes::GET_PROFILE_PREFERENCES, get_preferences)
        .contract_route(routes::PATCH_PROFILE_PREFERENCES, patch_preferences)
        .contract_route(routes::CHANGE_CURRENT_PASSWORD, change_password)
        .contract_route(routes::LIST_PROFILE_SESSIONS, list_sessions)
        .contract_route(routes::REVOKE_OTHER_PROFILE_SESSIONS, revoke_other_sessions)
        .contract_route(routes::REVOKE_PROFILE_SESSION, revoke_session)
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
