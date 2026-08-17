use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{ConnectInfo, Extension, Path, Query, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_identity::ActorPrincipal;
use citadel_identity::{
    CreateOidcProviderRequest, IdentityError, OidcService, PatchOidcProviderMetadataRequest,
    PatchOidcProviderRequest, RenameOidcProviderRequest, TestOidcDiscoveryRequest,
};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{
    identity_error_response, no_store, require_human_administrator, session_metadata,
    with_refresh_cookie,
};

#[derive(Clone)]
pub struct OidcHttpState {
    pub oidc: Arc<OidcService>,
    pub public_url: Url,
    pub allowed_return_origins: Vec<String>,
    pub secure_cookies: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BeginLoginQuery {
    return_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub fn router(state: OidcHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_OIDC_LOGIN_PROVIDERS, list_login_providers)
        .contract_route(routes::BEGIN_OIDC_LOGIN, begin_login)
        .contract_route(routes::COMPLETE_OIDC_LOGIN, complete_login)
        .contract_route(routes::LIST_OIDC_PROVIDERS, list_providers)
        .contract_route(routes::GET_OIDC_PROVIDER, get_provider)
        .contract_route(routes::CREATE_OIDC_PROVIDER, create_provider)
        .contract_route(routes::RENAME_OIDC_PROVIDER, rename_provider)
        .contract_route(routes::UPDATE_OIDC_PROVIDER, update_provider)
        .contract_route(
            routes::UPDATE_OIDC_PROVIDER_METADATA,
            update_provider_metadata,
        )
        .contract_route(routes::DELETE_OIDC_PROVIDER, delete_provider)
        .contract_route(
            routes::TEST_OIDC_PROVIDER_DISCOVERY,
            test_provider_discovery,
        )
        .contract_route(routes::TEST_OIDC_DISCOVERY, test_discovery)
        .with_state(state)
}

async fn list_login_providers(State(state): State<OidcHttpState>, headers: HeaderMap) -> Response {
    match state.oidc.list_login_providers().await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn begin_login(
    State(state): State<OidcHttpState>,
    Path(provider_id): Path<Uuid>,
    headers: HeaderMap,
    query: Result<Query<BeginLoginQuery>, QueryRejection>,
) -> Response {
    let query = match query {
        Ok(Query(query)) => query,
        Err(error) => return invalid_query(error, &headers),
    };
    let redirect_uri = callback_url(&state.public_url, provider_id);
    let return_url = query
        .return_url
        .as_deref()
        .unwrap_or(state.public_url.as_str());
    match state
        .oidc
        .begin_login(
            provider_id,
            redirect_uri.as_str(),
            Some(return_url),
            &state.allowed_return_origins,
        )
        .await
    {
        Ok(login) => redirect(login.authorization_url, &headers),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn complete_login(
    State(state): State<OidcHttpState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    Path(provider_id): Path<Uuid>,
    headers: HeaderMap,
    query: Result<Query<CallbackQuery>, QueryRejection>,
) -> Response {
    let query = match query {
        Ok(Query(query)) => query,
        Err(error) => return invalid_query(error, &headers),
    };
    if let Some(error) = query.error.filter(|value| !value.trim().is_empty()) {
        return identity_error_response(
            IdentityError::Validation(query.error_description.unwrap_or(error)),
            &headers,
        );
    }
    let (Some(code), Some(login_state)) = (query.code.as_deref(), query.state.as_deref()) else {
        return identity_error_response(
            IdentityError::Validation("Missing OIDC callback parameters.".to_owned()),
            &headers,
        );
    };
    let redirect_uri = callback_url(&state.public_url, provider_id);
    match state
        .oidc
        .complete_login(
            provider_id,
            code,
            login_state,
            redirect_uri.as_str(),
            session_metadata(&headers, ConnectInfo(address)),
        )
        .await
    {
        Ok(login) => with_refresh_cookie(
            redirect(login.return_url, &headers),
            &login.session.refresh_token,
            login.session.refresh_expires_at,
            state.secure_cookies,
        ),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn list_providers(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = require_human_administrator(principal) {
        return identity_error_response(error, &headers);
    }
    match state.oidc.list().await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = require_human_administrator(principal) {
        return identity_error_response(error, &headers);
    }
    match state.oidc.get(id).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn create_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateOidcProviderRequest>, JsonRejection>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => return invalid_json(error, &headers),
    };
    match state.oidc.create(input, principal.actor_id).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn rename_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameOidcProviderRequest>, JsonRejection>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => return invalid_json(error, &headers),
    };
    match state
        .oidc
        .rename(input.id, &input.name, principal.actor_id)
        .await
    {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn update_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    input: Result<Json<PatchOidcProviderRequest>, JsonRejection>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => return invalid_json(error, &headers),
    };
    match state.oidc.patch(id, input, principal.actor_id).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn update_provider_metadata(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    input: Result<Json<PatchOidcProviderMetadataRequest>, JsonRejection>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => return invalid_json(error, &headers),
    };
    match state
        .oidc
        .update_metadata(id, input, principal.actor_id)
        .await
    {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn delete_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    match state.oidc.delete(id, principal.actor_id).await {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn test_provider_discovery(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    if let Err(error) = require_human_administrator(principal) {
        return identity_error_response(error, &headers);
    }
    match state
        .oidc
        .test_discovery(TestOidcDiscoveryRequest {
            provider_id: Some(id),
            issuer: None,
        })
        .await
    {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn test_discovery(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<TestOidcDiscoveryRequest>, JsonRejection>,
) -> Response {
    if let Err(error) = require_human_administrator(principal) {
        return identity_error_response(error, &headers);
    }
    let input = match input {
        Ok(Json(input)) => input,
        Err(error) => return invalid_json(error, &headers),
    };
    match state.oidc.test_discovery(input).await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

fn callback_url(public_url: &Url, provider_id: Uuid) -> Url {
    let mut callback = public_url.clone();
    callback.set_path(&format!(
        "/api/v1/authentication/oidc/{provider_id}/callback"
    ));
    callback.set_query(None);
    callback.set_fragment(None);
    callback
}

fn redirect(location: String, headers: &HeaderMap) -> Response {
    let Ok(location) = HeaderValue::from_str(&location) else {
        return identity_error_response(
            IdentityError::Validation("Redirect URL is invalid.".to_owned()),
            headers,
        );
    };
    let mut response = StatusCode::FOUND.into_response();
    response.headers_mut().insert(LOCATION, location);
    no_store(response)
}

fn invalid_json(error: JsonRejection, headers: &HeaderMap) -> Response {
    identity_error_response(IdentityError::Validation(error.body_text()), headers)
}

fn invalid_query(error: QueryRejection, headers: &HeaderMap) -> Response {
    identity_error_response(IdentityError::Validation(error.body_text()), headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_replaces_public_url_path_and_query() {
        let public_url = Url::parse("https://citadel.test/base?ignored=true").unwrap();
        let provider_id = Uuid::now_v7();
        assert_eq!(
            callback_url(&public_url, provider_id).as_str(),
            format!("https://citadel.test/api/v1/authentication/oidc/{provider_id}/callback")
        );
    }
}
