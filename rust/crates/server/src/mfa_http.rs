use std::net::SocketAddr;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::ActorPrincipal;
use citadel_identity::{
    ConfirmMandatoryMfaSetupInput, ConfirmProfileMfaSetupInput, DisableProfileMfaInput,
    IdentityError, MfaVerificationInput, RegenerateProfileMfaRecoveryCodesInput,
    StartProfileMfaSetupInput,
};
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{
    IdentityHttpState, MFA_CHALLENGE_COOKIE, MFA_SETUP_COOKIE, cookie, current_refresh_token,
    identity_error_response, no_store, require_human, require_human_administrator,
    session_metadata, with_deleted_cookie, with_refresh_cookie,
};

pub fn router(state: IdentityHttpState) -> Router {
    Router::new()
        .contract_route(routes::VERIFY_AUTHENTICATION_MFA, verify_authentication)
        .contract_route(routes::GET_AUTHENTICATION_MFA_SETUP, get_mandatory_setup)
        .contract_route(
            routes::CONFIRM_AUTHENTICATION_MFA_SETUP,
            confirm_mandatory_setup,
        )
        .contract_route(routes::GET_PROFILE_MFA_STATUS, get_profile_status)
        .contract_route(routes::START_PROFILE_MFA_SETUP, start_profile_setup)
        .contract_route(routes::CONFIRM_PROFILE_MFA_SETUP, confirm_profile_setup)
        .contract_route(routes::DISABLE_PROFILE_MFA, disable_profile_mfa)
        .contract_route(
            routes::REGENERATE_PROFILE_MFA_RECOVERY_CODES,
            regenerate_recovery_codes,
        )
        .contract_route(routes::RESET_USER_MFA, reset_user_mfa)
        .with_state(state)
}

async fn verify_authentication(
    State(state): State<IdentityHttpState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    input: Result<Json<MfaVerificationInput>, JsonRejection>,
) -> Response {
    let challenge_id = match authentication_cookie_id(&headers, MFA_CHALLENGE_COOKIE) {
        Ok(id) => id,
        Err(error) => return identity_error_response(error, &headers),
    };
    let Ok(Json(input)) = input else {
        return invalid_json(&headers);
    };
    match state
        .mfa
        .verify_challenge(
            challenge_id,
            input,
            session_metadata(&headers, ConnectInfo(address)),
        )
        .await
    {
        Ok((view, session)) => {
            let response = with_refresh_cookie(
                no_store(Json(view).into_response()),
                &session.refresh_token,
                session.refresh_expires_at,
                state.secure_cookies,
            );
            with_deleted_cookie(response, MFA_CHALLENGE_COOKIE, state.secure_cookies)
        }
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get_mandatory_setup(
    State(state): State<IdentityHttpState>,
    headers: HeaderMap,
) -> Response {
    let setup_id = match authentication_cookie_id(&headers, MFA_SETUP_COOKIE) {
        Ok(id) => id,
        Err(error) => return identity_error_response(error, &headers),
    };
    match state.mfa.mandatory_setup(setup_id).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn confirm_mandatory_setup(
    State(state): State<IdentityHttpState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    input: Result<Json<ConfirmMandatoryMfaSetupInput>, JsonRejection>,
) -> Response {
    let setup_id = match authentication_cookie_id(&headers, MFA_SETUP_COOKIE) {
        Ok(id) => id,
        Err(error) => return identity_error_response(error, &headers),
    };
    let Ok(Json(input)) = input else {
        return invalid_json(&headers);
    };
    match state
        .mfa
        .confirm_mandatory_setup(
            setup_id,
            input,
            session_metadata(&headers, ConnectInfo(address)),
        )
        .await
    {
        Ok((view, session)) => {
            let response = with_refresh_cookie(
                no_store(Json(view).into_response()),
                &session.refresh_token,
                session.refresh_expires_at,
                state.secure_cookies,
            );
            with_deleted_cookie(response, MFA_SETUP_COOKIE, state.secure_cookies)
        }
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get_profile_status(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    match state.mfa.profile_status(&principal).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn start_profile_setup(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<StartProfileMfaSetupInput>, JsonRejection>,
) -> Response {
    let principal = match require_human(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let Ok(Json(input)) = input else {
        return invalid_json(&headers);
    };
    match state.mfa.start_profile_setup(&principal, input).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn confirm_profile_setup(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ConfirmProfileMfaSetupInput>, JsonRejection>,
) -> Response {
    let principal = match require_human(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let Ok(Json(input)) = input else {
        return invalid_json(&headers);
    };
    match state
        .mfa
        .confirm_profile_setup(&principal, input, current_refresh_token(&headers))
        .await
    {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn disable_profile_mfa(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DisableProfileMfaInput>, JsonRejection>,
) -> Response {
    let principal = match require_human(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let Ok(Json(input)) = input else {
        return invalid_json(&headers);
    };
    match state
        .mfa
        .disable_profile_mfa(&principal, input, current_refresh_token(&headers))
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn regenerate_recovery_codes(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RegenerateProfileMfaRecoveryCodesInput>, JsonRejection>,
) -> Response {
    let principal = match require_human(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let Ok(Json(input)) = input else {
        return invalid_json(&headers);
    };
    match state.mfa.regenerate_recovery_codes(&principal, input).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn reset_user_mfa(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(user_id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::User,
            user_id,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state.mfa.reset_user_mfa(user_id, principal.actor_id).await {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

fn authentication_cookie_id(headers: &HeaderMap, name: &str) -> Result<Uuid, IdentityError> {
    cookie(headers, name)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or(IdentityError::Unauthenticated)
}

fn invalid_json(headers: &HeaderMap) -> Response {
    identity_error_response(
        IdentityError::Validation("The request body is invalid.".to_owned()),
        headers,
    )
}

#[cfg(test)]
mod tests {
    use axum::http::header::COOKIE;
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn authentication_cookie_requires_an_exact_uuid() {
        let id = Uuid::now_v7();
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("{MFA_CHALLENGE_COOKIE}={id}")).unwrap(),
        );
        assert_eq!(
            authentication_cookie_id(&headers, MFA_CHALLENGE_COOKIE).unwrap(),
            id
        );
        headers.insert(
            COOKIE,
            HeaderValue::from_static("citadel_mfa_challenge=not-a-uuid"),
        );
        assert!(matches!(
            authentication_cookie_id(&headers, MFA_CHALLENGE_COOKIE),
            Err(IdentityError::Unauthenticated)
        ));
    }
}
