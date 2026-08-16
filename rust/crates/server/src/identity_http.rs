use std::net::SocketAddr;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, Extension, State};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, COOKIE, PRAGMA, SET_COOKIE, WWW_AUTHENTICATE,
};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use citadel_application::{
    IdentityError, IdentityService, InitializeCitadelRequest, LoginRequest, SessionMetadata,
};
use citadel_contracts::http::routes;
use citadel_domain::{ActorPrincipal, permission_matrix};
use serde::Serialize;

use crate::Readiness;
use crate::contract_router::ContractRouterExt;

const REFRESH_COOKIE: &str = "citadel_refresh_token";
const REQUEST_ID: &str = "x-request-id";

#[derive(Clone)]
pub struct IdentityHttpState {
    pub identity: Arc<IdentityService>,
    pub readiness: Arc<Readiness>,
    pub secure_cookies: bool,
}

pub fn router(state: IdentityHttpState) -> Router {
    Router::new()
        .contract_route(routes::GET_SETUP_STATUS, setup_status)
        .contract_route(routes::INITIALIZE_CITADEL, initialize)
        .contract_route(routes::LOGIN, login)
        .contract_route(routes::REFRESH_TOKEN, refresh)
        .contract_route(routes::LOGOUT, logout)
        .contract_route(routes::GET_PERMISSION_MATRIX, get_permission_matrix)
        .with_state(state)
}

pub async fn authentication_middleware(
    State(identity): State<Arc<IdentityService>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if let Some(token) = bearer(request.headers())
        && let Ok(principal) = identity.authenticate_bearer(token).await
    {
        request.extensions_mut().insert(principal);
    }
    next.run(request).await
}

async fn setup_status(State(state): State<IdentityHttpState>, headers: HeaderMap) -> Response {
    match state.identity.setup_status().await {
        Ok(status) => no_store(Json(status).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn initialize(
    State(state): State<IdentityHttpState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<InitializeCitadelRequest>,
) -> Response {
    let metadata = session_metadata(&headers, connect);
    match state.identity.initialize(request, metadata).await {
        Ok((response, session)) => {
            state.readiness.set_setup(true);
            with_refresh_cookie(
                no_store(Json(response).into_response()),
                &session.refresh_token,
                session.refresh_expires_at,
                state.secure_cookies,
            )
        }
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn login(
    State(state): State<IdentityHttpState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Response {
    let metadata = session_metadata(&headers, connect);
    match state.identity.login(request, metadata).await {
        Ok((response, session)) => with_refresh_cookie(
            no_store(Json(response).into_response()),
            &session.refresh_token,
            session.refresh_expires_at,
            state.secure_cookies,
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn refresh(
    State(state): State<IdentityHttpState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let Some(refresh_token) = current_refresh_token(&headers) else {
        return identity_error_response(IdentityError::InvalidCredentials, &headers);
    };
    let metadata = session_metadata(&headers, connect);
    match state.identity.refresh(refresh_token, metadata).await {
        Ok(session) => with_refresh_cookie(
            no_store(
                Json(AccessTokenResponse {
                    access_token: session.access_token,
                })
                .into_response(),
            ),
            &session.refresh_token,
            session.refresh_expires_at,
            state.secure_cookies,
        ),
        Err(error) => with_deleted_refresh_cookie(
            identity_error_response(error, &headers),
            state.secure_cookies,
        ),
    }
}

async fn logout(
    State(state): State<IdentityHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    if !principal.is_human() {
        return identity_error_response(IdentityError::Forbidden, &headers);
    }
    let refresh_token = current_refresh_token(&headers);
    match state.identity.logout(refresh_token).await {
        Ok(()) => with_deleted_refresh_cookie(
            StatusCode::NO_CONTENT.into_response(),
            state.secure_cookies,
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get_permission_matrix() -> Response {
    let entries = permission_matrix()
        .into_iter()
        .map(|(resource_type, capability)| PermissionMatrixEntry {
            resource_type,
            maximum_level: capability.maximum_level,
            specific_permissions: capability
                .specifics
                .iter()
                .map(|(permission, minimum_level)| SpecificPermissionEntry {
                    permission: *permission,
                    minimum_level: *minimum_level,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    no_store(Json(entries).into_response())
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer")
        || token.is_empty()
        || token.chars().any(char::is_whitespace)
    {
        return None;
    }
    Some(token)
}

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(candidate, value)| (candidate == name).then_some(value))
        .filter(|value| !value.is_empty())
}

pub(crate) fn current_refresh_token(headers: &HeaderMap) -> Option<&str> {
    cookie(headers, REFRESH_COOKIE)
}

fn session_metadata(headers: &HeaderMap, connect: ConnectInfo<SocketAddr>) -> SessionMetadata {
    SessionMetadata {
        user_agent: headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.chars().take(1_024).collect()),
        ip_address: Some(connect.0.ip().to_string()),
    }
}

fn with_refresh_cookie(
    mut response: Response,
    token: &str,
    expires_at: DateTime<Utc>,
    secure: bool,
) -> Response {
    let expires = expires_at.format("%a, %d %b %Y %H:%M:%S GMT");
    let secure = if secure { "; Secure" } else { "" };
    let value = format!(
        "{REFRESH_COOKIE}={token}; Path=/; Expires={expires}; HttpOnly{secure}; SameSite=Strict"
    );
    if let Ok(value) = HeaderValue::from_str(&value) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    response
}

fn with_deleted_refresh_cookie(mut response: Response, secure: bool) -> Response {
    let secure = if secure { "; Secure" } else { "" };
    let value =
        format!("citadel_refresh_token=; Path=/; Max-Age=0; HttpOnly{secure}; SameSite=Strict");
    if let Ok(value) = HeaderValue::from_str(&value) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    response
}

pub(crate) fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(PRAGMA, HeaderValue::from_static("no-cache"));
    response
}

pub(crate) fn require_human(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    let Some(Extension(principal)) = principal else {
        return Err(IdentityError::Unauthenticated);
    };
    if !principal.is_human() {
        return Err(IdentityError::Forbidden);
    }
    Ok(principal)
}

pub(crate) fn require_human_administrator(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    let principal = require_human(principal)?;
    if !principal.is_administrator() {
        return Err(IdentityError::Forbidden);
    }
    Ok(principal)
}

pub(crate) fn identity_error_response(error: IdentityError, headers: &HeaderMap) -> Response {
    let request_id = headers
        .get(REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let (status, problem_type, title, detail) = match error {
        IdentityError::InvalidCredentials | IdentityError::Unauthenticated => (
            StatusCode::UNAUTHORIZED,
            "authentication_required",
            "Authentication required",
            "Valid authentication is required.".to_owned(),
        ),
        IdentityError::Forbidden => (
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            "The authenticated principal is not allowed to perform this operation.".to_owned(),
        ),
        IdentityError::SetupRequired => (
            StatusCode::CONFLICT,
            "setup_required",
            "Setup required",
            "Citadel must be initialized before this endpoint can be used.".to_owned(),
        ),
        IdentityError::SetupAlreadyComplete => (
            StatusCode::CONFLICT,
            "setup_already_complete",
            "Setup already complete",
            "Citadel setup is already complete.".to_owned(),
        ),
        IdentityError::Validation(message) => (
            StatusCode::BAD_REQUEST,
            "validation_error",
            "Validation failed",
            message,
        ),
        IdentityError::Conflict(message) => (StatusCode::CONFLICT, "conflict", "Conflict", message),
        IdentityError::NotFound => (
            StatusCode::NOT_FOUND,
            "not_found",
            "Not found",
            "The requested resource was not found.".to_owned(),
        ),
        IdentityError::LicenseRequired(capability) => (
            StatusCode::FORBIDDEN,
            "license_capability_required",
            "License capability required",
            format!("The '{capability}' license capability is required."),
        ),
        IdentityError::Storage(_) | IdentityError::Credential => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
            "An unexpected error occurred.".to_owned(),
        ),
    };
    let mut response = (
        status,
        Json(IdentityProblemDetails {
            r#type: problem_type,
            title,
            status: status.as_u16(),
            detail,
            request_id,
        }),
    )
        .into_response();
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    if status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    no_store(response)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityProblemDetails {
    r#type: &'static str,
    title: &'static str,
    status: u16,
    detail: String,
    request_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionMatrixEntry {
    resource_type: citadel_domain::ResourceType,
    maximum_level: citadel_domain::PermissionLevel,
    specific_permissions: Vec<SpecificPermissionEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpecificPermissionEntry {
    permission: citadel_domain::SpecificPermission,
    minimum_level: citadel_domain::PermissionLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_parser_requires_an_exact_cookie_name() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("other=x; citadel_refresh_token=token"),
        );
        assert_eq!(cookie(&headers, REFRESH_COOKIE), Some("token"));
        assert_eq!(cookie(&headers, "citadel"), None);
    }

    #[test]
    fn bearer_parser_does_not_accept_whitespace_or_other_schemes() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(bearer(&headers), None);
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer one two"));
        assert_eq!(bearer(&headers), None);
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token"));
        assert_eq!(bearer(&headers), Some("token"));
    }

    #[test]
    fn administrator_boundary_rejects_non_administrators_and_service_accounts() {
        for principal_type in [
            citadel_domain::AuthenticatedPrincipalType::User,
            citadel_domain::AuthenticatedPrincipalType::ServiceAccount,
        ] {
            let principal = ActorPrincipal {
                subject_id: uuid::Uuid::now_v7(),
                actor_id: citadel_domain::ActorId::new(uuid::Uuid::now_v7()),
                name: "reader".to_owned(),
                principal_type,
                credential_id: None,
                roles: vec!["Viewer".to_owned()],
            };
            assert!(matches!(
                require_human_administrator(Some(Extension(principal))),
                Err(IdentityError::Forbidden)
            ));
        }
    }

    #[test]
    fn administrator_boundary_accepts_a_human_admin_role() {
        let principal = ActorPrincipal {
            subject_id: uuid::Uuid::now_v7(),
            actor_id: citadel_domain::ActorId::new(uuid::Uuid::now_v7()),
            name: "owner".to_owned(),
            principal_type: citadel_domain::AuthenticatedPrincipalType::User,
            credential_id: None,
            roles: vec!["Admin".to_owned()],
        };
        assert!(require_human_administrator(Some(Extension(principal))).is_ok());
    }
}
