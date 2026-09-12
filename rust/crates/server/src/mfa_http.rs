use crate::request_validation::ApiPath;
use std::net::SocketAddr;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, Extension, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::ActorPrincipal;
use citadel_identity::{
    ConfirmMandatoryMfaSetupInput, ConfirmProfileMfaSetupInput, DisableProfileMfaInput,
    IdentityError, MfaVerificationInput, RegenerateProfileMfaRecoveryCodesInput,
    StartProfileMfaSetupInput,
};
use uuid::Uuid;

use crate::identity_http::{
    IdentityHttpState, MFA_CHALLENGE_COOKIE, MFA_SETUP_COOKIE, cookie, current_refresh_token,
    identity_error_response, no_store, require_human, require_human_administrator,
    session_metadata, with_deleted_cookie, with_refresh_cookie,
};
use crate::openapi::router::OpenApiRouterExt;

pub fn router(state: IdentityHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[utoipa::path(
    post,
    path = "/api/v1/authentication/mfa/verify",
    operation_id = "verifyAuthenticationMfa",
    tag = "Authentication",
    summary = "Complete an MFA challenge",
    request_body = MfaVerificationInput,
    params(("citadel_mfa_challenge" = String, Cookie, description = "MFA Challenge")),
    responses(
        (status = 200, description = "Success", body = citadel_identity::MfaVerificationView, content_type = "application/json", headers(("Set-Cookie" = String, description = "Sets refresh_token and expires the MFA challenge cookie."))),
        crate::openapi::errors::AuthenticationErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => {
            return identity_error_response(
                crate::request_validation::invalid_json(error),
                &headers,
            );
        }
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

#[utoipa::path(
    get,
    path = "/api/v1/authentication/mfa/setup",
    operation_id = "getAuthenticationMfaSetup",
    tag = "Authentication",
    summary = "Get mandatory MFA setup",
    params(("citadel_mfa_setup" = String, Cookie, description = "MFA Setup")),
    responses(
        (status = 200, description = "Success", body = citadel_identity::MandatoryMfaSetupView, content_type = "application/json"),
        crate::openapi::errors::AuthenticationErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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

#[utoipa::path(
    post,
    path = "/api/v1/authentication/mfa/setup/confirm",
    operation_id = "confirmAuthenticationMfaSetup",
    tag = "Authentication",
    summary = "Complete mandatory MFA setup",
    request_body = ConfirmMandatoryMfaSetupInput,
    params(("citadel_mfa_setup" = String, Cookie, description = "MFA Setup")),
    responses(
        (status = 200, description = "Success", body = citadel_identity::MandatoryMfaSetupCompleteView, content_type = "application/json", headers(("Set-Cookie" = String, description = "Sets refresh_token and expires the MFA setup cookie."))),
        crate::openapi::errors::AuthenticationErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => {
            return identity_error_response(
                crate::request_validation::invalid_json(error),
                &headers,
            );
        }
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

#[utoipa::path(
    get,
    path = "/api/v1/profile/mfa",
    operation_id = "getProfileMfaStatus",
    tag = "Profile",
    summary = "Get MFA status",
    responses(
        (status = 200, description = "Success", body = citadel_identity::ProfileMfaStatusView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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

#[utoipa::path(
    post,
    path = "/api/v1/profile/mfa/setup",
    operation_id = "startProfileMfaSetup",
    tag = "Profile",
    summary = "Start MFA setup",
    request_body = StartProfileMfaSetupInput,
    responses(
        (status = 200, description = "Success", body = citadel_identity::ProfileMfaSetupView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => {
            return identity_error_response(
                crate::request_validation::invalid_json(error),
                &headers,
            );
        }
    };
    match state.mfa.start_profile_setup(&principal, input).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/profile/mfa/setup/confirm",
    operation_id = "confirmProfileMfaSetup",
    tag = "Profile",
    summary = "Complete MFA setup",
    request_body = ConfirmProfileMfaSetupInput,
    responses(
        (status = 200, description = "Success", body = citadel_identity::ProfileMfaRecoveryCodesView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => {
            return identity_error_response(
                crate::request_validation::invalid_json(error),
                &headers,
            );
        }
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

#[utoipa::path(
    post,
    path = "/api/v1/profile/mfa/disable",
    operation_id = "disableProfileMfa",
    tag = "Profile",
    summary = "Disable MFA",
    request_body = DisableProfileMfaInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => {
            return identity_error_response(
                crate::request_validation::invalid_json(error),
                &headers,
            );
        }
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

#[utoipa::path(
    post,
    path = "/api/v1/profile/mfa/recovery-codes",
    operation_id = "regenerateProfileMfaRecoveryCodes",
    tag = "Profile",
    summary = "Regenerate MFA recovery codes",
    request_body = RegenerateProfileMfaRecoveryCodesInput,
    responses(
        (status = 200, description = "Success", body = citadel_identity::ProfileMfaRecoveryCodesView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => {
            return identity_error_response(
                crate::request_validation::invalid_json(error),
                &headers,
            );
        }
    };
    match state.mfa.regenerate_recovery_codes(&principal, input).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/mfa",
    operation_id = "resetUserMfa",
    tag = "Users",
    summary = "Reset a user's MFA",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn reset_user_mfa(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(user_id): ApiPath<Uuid>,
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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<IdentityHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(verify_authentication))
        .normalized_routes(utoipa_axum::routes!(get_mandatory_setup))
        .normalized_routes(utoipa_axum::routes!(confirm_mandatory_setup))
        .normalized_routes(utoipa_axum::routes!(get_profile_status))
        .normalized_routes(utoipa_axum::routes!(start_profile_setup))
        .normalized_routes(utoipa_axum::routes!(confirm_profile_setup))
        .normalized_routes(utoipa_axum::routes!(disable_profile_mfa))
        .normalized_routes(utoipa_axum::routes!(regenerate_recovery_codes))
        .normalized_routes(utoipa_axum::routes!(reset_user_mfa))
}
