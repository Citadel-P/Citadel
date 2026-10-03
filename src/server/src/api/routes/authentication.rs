use crate::{
    Readiness,
    api::{
        error::{error_response, no_store},
        resources::authentication::{
            requests::{InitializeCitadelRequest, LoginRequest},
            views::AccessTokenResponse,
        },
    },
    openapi::router::OpenApiRouterExt,
    request_validation::ValidatedJson,
};

use axum::{
    Json, Router,
    extract::{ConnectInfo, Extension, State},
    http::{
        HeaderMap, HeaderValue, Request, StatusCode,
        header::{AUTHORIZATION, COOKIE, SET_COOKIE},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};

use chrono::{DateTime, Utc};

use citadel_identity::{
    ActorPrincipal, BrowserAuthenticationAction, IdentityError, IdentityService, MfaService,
    SessionMetadata,
};

use std::{net::SocketAddr, sync::Arc};

pub(crate) const REFRESH_COOKIE: &str = "refresh_token";

pub(crate) const MFA_CHALLENGE_COOKIE: &str = "citadel_mfa_challenge";

pub(crate) const MFA_SETUP_COOKIE: &str = "citadel_mfa_setup";

const API_COOKIE_PATH: &str = "/api/v1";

#[derive(Clone)]
pub struct IdentityHttpState {
    pub identity: Arc<IdentityService>,
    pub mfa: Arc<MfaService>,
    pub readiness: Arc<Readiness>,
    pub secure_cookies: bool,
}

pub fn router(state: IdentityHttpState) -> Router {
    let mfa = crate::api::routes::mfa::router(state.clone());
    documented_routes()
        .split_for_parts()
        .0
        .with_state(state)
        .merge(mfa)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<IdentityHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(setup_status))
        .normalized_routes(utoipa_axum::routes!(initialize))
        .normalized_routes(utoipa_axum::routes!(login))
        .normalized_routes(utoipa_axum::routes!(refresh))
        .normalized_routes(utoipa_axum::routes!(logout))
}

pub async fn authentication_middleware(
    State(identity): State<Arc<IdentityService>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if let Some(token) = bearer(request.headers())
        && let Ok(authenticated) = identity.authenticate_bearer_context(token).await
    {
        if let Err(error) =
            crate::token_safety::authorize_http(&authenticated, request.uri().path())
        {
            return error_response(error, request.headers());
        }
        request.extensions_mut().insert(authenticated.principal);
    }
    next.run(request).await
}

#[utoipa::path(
    get,
    path = "/api/v1/setup/status",
    operation_id = "getSetupStatus",
    tag = "Setup",
    summary = "Get setup status",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::authentication::views::SetupStatusView, content_type = "application/json"),
        crate::openapi::errors::ReadinessErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(true)))
)]
async fn setup_status(State(state): State<IdentityHttpState>, headers: HeaderMap) -> Response {
    match state.identity.setup_status().await {
        Ok(status) => no_store(
            Json(crate::api::resources::authentication::views::SetupStatusView::from(status))
                .into_response(),
        ),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/setup/initialize",
    operation_id = "initializeCitadel",
    tag = "Setup",
    summary = "Initialize Citadel",
    request_body = InitializeCitadelRequest,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::authentication::views::LoginResponse, content_type = "application/json"),
        crate::openapi::errors::InitializationErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(true)))
)]
async fn initialize(
    State(state): State<IdentityHttpState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<InitializeCitadelRequest>,
) -> Response {
    let request: citadel_identity::InitializeCitadel = request.into();
    let metadata = session_metadata(&headers, connect);
    match state.mfa.initialize(request, metadata).await {
        Ok(result) => {
            state.readiness.set_setup(true);
            with_authentication_action(
                no_store(
                    Json(
                        crate::api::resources::authentication::views::LoginResponse::from(
                            result.response,
                        ),
                    )
                    .into_response(),
                ),
                result.action,
                state.secure_cookies,
            )
        }
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/authentication/login",
    operation_id = "login",
    tag = "Authentication",
    summary = "Sign in",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::authentication::views::LoginResponse, content_type = "application/json", headers(("Set-Cookie" = String, description = "Sets refresh_token on success, or citadel_mfa_challenge / citadel_mfa_setup when MFA is required. Multiple Set-Cookie headers may be returned."))),
        crate::openapi::errors::LoginErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn login(
    State(state): State<IdentityHttpState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<LoginRequest>,
) -> Response {
    let request: citadel_identity::Login = request.into();
    let metadata = session_metadata(&headers, connect);
    match state.mfa.login(request, metadata).await {
        Ok(result) => with_authentication_action(
            no_store(
                Json(
                    crate::api::resources::authentication::views::LoginResponse::from(
                        result.response,
                    ),
                )
                .into_response(),
            ),
            result.action,
            state.secure_cookies,
        ),
        Err(error) => error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/authentication/refresh",
    operation_id = "refreshToken",
    tag = "Authentication",
    summary = "Refresh browser access",
    params(("refresh_token" = String, Cookie, description = "Refresh Token")),
    responses(
        (status = 200, description = "Success", body = AccessTokenResponse, content_type = "application/json", headers(("Set-Cookie" = String, description = "Rotates refresh_token."))),
        crate::openapi::errors::LoginErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn refresh(
    State(state): State<IdentityHttpState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let Some(refresh_token) = current_refresh_token(&headers) else {
        return error_response(IdentityError::InvalidCredentials, &headers);
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
        Err(error) => {
            with_deleted_refresh_cookie(error_response(error, &headers), state.secure_cookies)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/authentication/logout",
    operation_id = "logout",
    tag = "Authentication",
    summary = "End browser session",
    params(("refresh_token" = String, Cookie, description = "Refresh Token")),
    responses(
        (status = 204, description = "Success", headers(("Set-Cookie" = String, description = "Expires refresh_token, citadel_mfa_challenge and citadel_mfa_setup using separate Set-Cookie headers."))),
        crate::openapi::errors::ResourceErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn logout(State(state): State<IdentityHttpState>, headers: HeaderMap) -> Response {
    let refresh_token = current_refresh_token(&headers);
    match state.identity.logout(refresh_token).await {
        Ok(()) => with_deleted_authentication_cookies(
            StatusCode::NO_CONTENT.into_response(),
            state.secure_cookies,
        ),
        Err(error) => error_response(error, &headers),
    }
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

pub(crate) fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
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

pub(crate) fn session_metadata(
    headers: &HeaderMap,
    connect: ConnectInfo<SocketAddr>,
) -> SessionMetadata {
    SessionMetadata {
        user_agent: headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.chars().take(1_024).collect()),
        ip_address: Some(connect.0.ip().to_string()),
    }
}

pub(crate) fn with_refresh_cookie(
    mut response: Response,
    token: &str,
    expires_at: DateTime<Utc>,
    secure: bool,
) -> Response {
    let expires = expires_at.format("%a, %d %b %Y %H:%M:%S GMT");
    let policy = cookie_policy(secure);
    let value = format!(
        "{REFRESH_COOKIE}={token}; Path={API_COOKIE_PATH}; Expires={expires}; HttpOnly{policy}"
    );
    if let Ok(value) = HeaderValue::from_str(&value) {
        response.headers_mut().append(SET_COOKIE, value);
    }
    response
}

fn with_deleted_refresh_cookie(response: Response, secure: bool) -> Response {
    with_deleted_cookie(response, REFRESH_COOKIE, secure)
}

pub(crate) fn with_cookie(
    mut response: Response,
    name: &str,
    value: &str,
    expires_at: DateTime<Utc>,
    secure: bool,
) -> Response {
    let expires = expires_at.format("%a, %d %b %Y %H:%M:%S GMT");
    let policy = cookie_policy(secure);
    let value =
        format!("{name}={value}; Path={API_COOKIE_PATH}; Expires={expires}; HttpOnly{policy}");
    if let Ok(value) = HeaderValue::from_str(&value) {
        response.headers_mut().append(SET_COOKIE, value);
    }
    response
}

pub(crate) fn with_deleted_cookie(mut response: Response, name: &str, secure: bool) -> Response {
    let policy = cookie_policy(secure);
    for path in [API_COOKIE_PATH, "/api/v1/authentication", "/"] {
        let value = format!("{name}=; Path={path}; Max-Age=0; HttpOnly{policy}");
        if let Ok(value) = HeaderValue::from_str(&value) {
            response.headers_mut().append(SET_COOKIE, value);
        }
    }
    response
}

fn cookie_policy(secure: bool) -> &'static str {
    if secure {
        "; Secure; SameSite=None"
    } else {
        "; SameSite=Lax"
    }
}

pub(crate) fn with_deleted_authentication_cookies(response: Response, secure: bool) -> Response {
    let response = with_deleted_refresh_cookie(response, secure);
    let response = with_deleted_cookie(response, MFA_CHALLENGE_COOKIE, secure);
    with_deleted_cookie(response, MFA_SETUP_COOKIE, secure)
}

fn with_authentication_action(
    response: Response,
    action: BrowserAuthenticationAction,
    secure: bool,
) -> Response {
    match action {
        BrowserAuthenticationAction::Completed(session) => with_refresh_cookie(
            response,
            &session.refresh_token,
            session.refresh_expires_at,
            secure,
        ),
        BrowserAuthenticationAction::VerifyMfa {
            challenge_id,
            expires_at,
        } => with_cookie(
            response,
            MFA_CHALLENGE_COOKIE,
            &challenge_id.to_string(),
            expires_at,
            secure,
        ),
        BrowserAuthenticationAction::EnrollMfa {
            setup_session_id,
            expires_at,
        } => with_cookie(
            response,
            MFA_SETUP_COOKIE,
            &setup_session_id.to_string(),
            expires_at,
            secure,
        ),
    }
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

#[cfg(test)]
mod tests {
    const REQUEST_ID: &str = "x-request-id";

    use crate::api::{error::api_result, routes::authentication::*};
    use axum::http::header::CACHE_CONTROL;

    #[test]
    fn cookie_parser_requires_an_exact_cookie_name() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("other=x; refresh_token=token"),
        );
        assert_eq!(cookie(&headers, REFRESH_COOKIE), Some("token"));
        assert_eq!(cookie(&headers, "citadel"), None);
    }

    #[tokio::test]
    async fn structured_resource_errors_preserve_dotnet_problem_contracts() {
        use serde_json::json;
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID, HeaderValue::from_static("request-42"));
        for (error, status, expected) in [
            (
                IdentityError::FieldValidation(
                    [(
                        "Field".into(),
                        vec!["First error.".into(), "Second error.".into()],
                    )]
                    .into(),
                ),
                StatusCode::BAD_REQUEST,
                json!({"type":"https://tools.ietf.org/html/rfc9110#section-15.5.1","title":"One or more validation errors occurred.","status":400,"errors":{"Field":["First error.","Second error."]},"traceId":"request-42"}),
            ),
            (
                IdentityError::ResourceNotFound("The provided alert rule does not exist"),
                StatusCode::NOT_FOUND,
                json!({"type":"https://tools.ietf.org/html/rfc9110#section-15.5.5","title":"Not Found","status":404,"detail":"The provided alert rule does not exist","traceId":"request-42"}),
            ),
            // Existing non-structured errors keep their established shape.
            (
                IdentityError::Validation("Invalid input.".into()),
                StatusCode::BAD_REQUEST,
                json!({"type":"validation_error","title":"Validation failed","status":400,"detail":"Invalid input.","requestId":"request-42"}),
            ),
            (
                IdentityError::Storage("postgres://private-credential/database".into()),
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"type":"internal_error","title":"Internal server error","status":500,"detail":"An unexpected error occurred.","requestId":"request-42"}),
            ),
        ] {
            let response = api_result::<()>(Err(error), &headers)
                .unwrap_err()
                .into_response();
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
            assert_eq!(
                response.headers()[axum::http::header::CONTENT_TYPE],
                "application/problem+json"
            );
            let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024)
                .await
                .unwrap();
            let actual: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn authentication_cookies_match_the_browser_contract() {
        let response = with_refresh_cookie(
            StatusCode::OK.into_response(),
            "token",
            Utc::now() + chrono::Duration::minutes(5),
            false,
        );
        let value = response.headers()[SET_COOKIE].to_str().unwrap();
        assert!(value.starts_with("refresh_token=token;"));
        assert!(value.contains("Path=/api/v1"));
        assert!(value.contains("HttpOnly"));
        assert!(value.contains("SameSite=Lax"));
        assert!(!value.contains("Secure"));

        let response = with_cookie(
            StatusCode::OK.into_response(),
            MFA_CHALLENGE_COOKIE,
            "challenge",
            Utc::now() + chrono::Duration::minutes(5),
            true,
        );
        let value = response.headers()[SET_COOKIE].to_str().unwrap();
        assert!(value.contains("Secure"));
        assert!(value.contains("SameSite=None"));
    }

    #[tokio::test]
    async fn license_errors_match_the_existing_problem_details_contract() {
        let response = error_response(
            IdentityError::LicenseRequired("custom-access-control"),
            &HeaderMap::new(),
        );
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap();
        let problem: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            problem["type"],
            "https://citadel.local/problems/license-capability-required"
        );
        assert_eq!(
            problem["detail"],
            "Custom access control requires a Team license."
        );
        assert_eq!(problem["capability"], "CustomAccessControl");
        assert_eq!(problem["licenseStatus"], "Community");
        assert_eq!(problem["effectiveEdition"], "Community");
    }

    #[tokio::test]
    async fn identity_result_maps_failures_without_changing_problem_details() {
        let mut headers = HeaderMap::new();
        headers.insert(REQUEST_ID, HeaderValue::from_static("request-42"));

        let response = api_result::<()>(Err(IdentityError::Forbidden), &headers)
            .unwrap_err()
            .into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap();
        let problem: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(problem["requestId"], "request-42");
        assert_eq!(problem["type"], "forbidden");
    }

    #[test]
    fn logout_expires_current_and_legacy_cookie_paths() {
        let response = with_deleted_cookie(
            StatusCode::NO_CONTENT.into_response(),
            REFRESH_COOKIE,
            false,
        );
        let values = response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 3);
        assert!(
            values
                .iter()
                .any(|value| value.contains("Path=/api/v1;") && value.contains("Max-Age=0"))
        );
        assert!(values.iter().any(|value| {
            value.contains("Path=/api/v1/authentication") && value.contains("Max-Age=0")
        }));
        assert!(
            values
                .iter()
                .any(|value| value.contains("Path=/;") && value.contains("Max-Age=0"))
        );
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
            citadel_identity::AuthenticatedPrincipalType::User,
            citadel_identity::AuthenticatedPrincipalType::ServiceAccount,
        ] {
            let principal = ActorPrincipal {
                subject_id: uuid::Uuid::now_v7(),
                actor_id: citadel_primitives::ActorId::new(uuid::Uuid::now_v7()),
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
            actor_id: citadel_primitives::ActorId::new(uuid::Uuid::now_v7()),
            name: "owner".to_owned(),
            principal_type: citadel_identity::AuthenticatedPrincipalType::User,
            credential_id: None,
            roles: vec!["Admin".to_owned()],
        };
        assert!(require_human_administrator(Some(Extension(principal))).is_ok());
    }
}
