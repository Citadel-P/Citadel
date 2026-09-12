use crate::request_validation::ApiPath;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{ConnectInfo, Extension, Query, State};
use axum::http::header::LOCATION;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use citadel_identity::ActorPrincipal;
use citadel_identity::{
    CreateOidcProviderRequest, IdentityError, OidcService, PatchOidcProviderMetadataRequest,
    PatchOidcProviderRequest, RenameOidcProviderRequest, TestOidcDiscoveryRequest,
};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::identity_http::{
    identity_error_response, no_store, require_human_administrator, session_metadata,
    with_refresh_cookie,
};
use crate::openapi::router::OpenApiRouterExt;

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
    documented_routes().split_for_parts().0.with_state(state)
}

#[utoipa::path(
    get,
    path = "/api/v1/authentication/oidc/providers",
    operation_id = "listOidcLoginProviders",
    summary = "List enabled OIDC login providers",
    responses(
        (status = 200, description = "Success", body = citadel_identity::OidcLoginProvidersView, content_type = "application/json"),
        crate::openapi::errors::RequestErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_login_providers(State(state): State<OidcHttpState>, headers: HeaderMap) -> Response {
    match state.oidc.list_login_providers().await {
        Ok(view) => no_store(Json(view).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/authentication/oidc/{id}/login",
    operation_id = "beginOidcLogin",
    summary = "Begin OIDC login",
    responses(
        (status = 302, description = "Success"),
        crate::openapi::errors::RedirectErrors
    ),
    params(("id" = uuid::Uuid, Path), ("returnUrl" = Option<String>, Query)),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn begin_login(
    State(state): State<OidcHttpState>,
    ApiPath(provider_id): ApiPath<Uuid>,
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

#[utoipa::path(
    get,
    path = "/api/v1/authentication/oidc/{id}/callback",
    operation_id = "completeOidcLogin",
    summary = "Complete OIDC login",
    responses(
        (status = 302, description = "Success"),
        crate::openapi::errors::RedirectErrors
    ),
    params(("id" = uuid::Uuid, Path), ("code" = Option<String>, Query), ("state" = Option<String>, Query), ("error" = Option<String>, Query), ("error_description" = Option<String>, Query)),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn complete_login(
    State(state): State<OidcHttpState>,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    ApiPath(provider_id): ApiPath<Uuid>,
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

#[utoipa::path(
    get,
    path = "/api/v1/oidcProviders",
    operation_id = "listOidcProviders",
    summary = "List OIDC providers",
    responses(
        (status = 200, description = "Success", body = citadel_identity::OidcProvidersView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/oidcProviders/{id}",
    operation_id = "getOidcProvider",
    summary = "Get OIDC provider",
    responses(
        (status = 200, description = "Success", body = citadel_identity::OidcProviderView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    post,
    path = "/api/v1/oidcProviders",
    operation_id = "createOidcProvider",
    summary = "Create OIDC provider",
    request_body = CreateOidcProviderRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::OidcProviderView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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

#[utoipa::path(
    post,
    path = "/api/v1/oidcProviders/rename",
    operation_id = "renameOidcProvider",
    summary = "Rename OIDC provider",
    request_body = RenameOidcProviderRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::OidcProviderView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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

#[utoipa::path(
    patch,
    path = "/api/v1/oidcProviders/{id}",
    operation_id = "updateOidcProvider",
    summary = "Update OIDC provider",
    request_body = PatchOidcProviderRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::OidcProviderView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    patch,
    path = "/api/v1/oidcProviders/{id}/_metadata",
    operation_id = "updateOidcProviderMetadata",
    summary = "Update OIDC provider metadata",
    request_body = PatchOidcProviderMetadataRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::OidcProviderView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_provider_metadata(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    delete,
    path = "/api/v1/oidcProviders/{id}",
    operation_id = "deleteOidcProvider",
    summary = "Delete OIDC provider",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete_provider(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    post,
    path = "/api/v1/oidcProviders/{id}/testDiscovery",
    operation_id = "testOidcProviderDiscovery",
    summary = "Test OIDC provider discovery",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/OidcDiscoveryResultView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn test_provider_discovery(
    State(state): State<OidcHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    post,
    path = "/api/v1/oidcProviders/testDiscovery",
    operation_id = "testOidcDiscovery",
    summary = "Test OIDC discovery",
    request_body = TestOidcDiscoveryRequest,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/OidcDiscoveryResultView"), content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    identity_error_response(crate::request_validation::invalid_json(error), headers)
}

fn invalid_query(error: QueryRejection, headers: &HeaderMap) -> Response {
    identity_error_response(crate::request_validation::invalid_query(error), headers)
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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<OidcHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_login_providers))
        .normalized_routes(utoipa_axum::routes!(begin_login))
        .normalized_routes(utoipa_axum::routes!(complete_login))
        .normalized_routes(utoipa_axum::routes!(list_providers))
        .normalized_routes(utoipa_axum::routes!(get_provider))
        .normalized_routes(utoipa_axum::routes!(create_provider))
        .normalized_routes(utoipa_axum::routes!(rename_provider))
        .normalized_routes(utoipa_axum::routes!(update_provider))
        .normalized_routes(utoipa_axum::routes!(update_provider_metadata))
        .normalized_routes(utoipa_axum::routes!(delete_provider))
        .normalized_routes(utoipa_axum::routes!(test_provider_discovery))
        .normalized_routes(utoipa_axum::routes!(test_discovery))
}
