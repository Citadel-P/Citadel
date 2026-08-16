use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use citadel_application::{
    AddServiceAccountResourceAccessRequest, AddServiceAccountRoleRequest,
    ArchiveServiceAccountsRequest, CreateServiceAccountRequest, CreateServiceAccountTokenRequest,
    DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS, IdentityError, IdentityService,
    MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS, MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
    PagedResult, RenameServiceAccountRequest, ServiceAccountLimitsView, ServiceAccountService,
    ServiceAccountTokenView, ServiceAccountView, UpdateServiceAccountRequest,
};
use citadel_domain::{
    ActorPrincipal, PermissionGrant, PermissionLevel, ResourceType, SpecificPermission,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::identity_http::identity_error_response;

#[derive(Clone)]
pub struct ServiceAccountHttpState {
    pub identity: Arc<IdentityService>,
    pub service_accounts: Arc<ServiceAccountService>,
}

pub fn router(state: ServiceAccountHttpState) -> Router {
    Router::new()
        .route(
            "/api/v1/serviceAccounts",
            get(list).post(create).delete(archive),
        )
        .route("/api/v1/serviceAccounts/limits", get(limits))
        .route("/api/v1/serviceAccounts/{id}", get(get_one).patch(update))
        .route("/api/v1/serviceAccounts/rename", post(rename))
        .route("/api/v1/serviceAccounts/{id}/roles", post(add_role))
        .route(
            "/api/v1/serviceAccounts/{id}/roles/{roleId}",
            delete(remove_role),
        )
        .route(
            "/api/v1/serviceAccounts/{id}/resource-accesses",
            post(add_resource_access),
        )
        .route(
            "/api/v1/serviceAccounts/{id}/resource-accesses/{resourceAccessId}",
            delete(remove_resource_access),
        )
        .route(
            "/api/v1/serviceAccounts/{id}/tokens",
            get(list_tokens).post(create_token),
        )
        .route(
            "/api/v1/serviceAccounts/{id}/tokens/{tokenId}",
            delete(revoke_token),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListFilter {
    #[serde(default = "first_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
    name: Option<String>,
    #[serde(default)]
    include_archived: bool,
}

async fn list(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Query(filter): Query<ListFilter>,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    let permission = match state
        .identity
        .global_permission(&principal, ResourceType::ServiceAccount)
        .await
    {
        Ok(Some(permission)) if permission.level.grants(PermissionLevel::Read) => permission,
        Ok(_) => return identity_error_response(IdentityError::Forbidden, &headers),
        Err(error) => return identity_error_response(error, &headers),
    };
    match state
        .service_accounts
        .list(
            principal.actor_id,
            principal.is_administrator(),
            filter.include_archived,
            filter.name.as_deref(),
            filter.page,
            filter.page_size,
        )
        .await
    {
        Ok(paged_result) => Json(ServiceAccountsResponse {
            paged_result,
            capabilities: ResourceCapabilities::from(permission),
        })
        .into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get_one(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    let permission = match state
        .identity
        .permission_for_resource(&principal, ResourceType::ServiceAccount, id)
        .await
    {
        Ok(Some(permission)) if permission.level.grants(PermissionLevel::Read) => permission,
        Ok(_) => return identity_error_response(IdentityError::NotFound, &headers),
        Err(error) => return identity_error_response(error, &headers),
    };
    match state.service_accounts.get(id).await {
        Ok(account) => Json(ServiceAccountDetailResponse {
            account,
            capabilities: ServiceAccountCapabilities::from(permission),
        })
        .into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn create(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<CreateServiceAccountRequest>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize(
            &principal,
            ResourceType::ServiceAccount,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .create(request, principal.actor_id)
        .await
    {
        Ok(account) => (StatusCode::OK, Json(account)).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn update(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateServiceAccountRequest>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state.service_accounts.update(id, request).await {
        Ok(account) => Json(account).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn rename(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<RenameServiceAccountRequest>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::ServiceAccount,
            request.id,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .rename(request.id, &request.name)
        .await
    {
        Ok(account) => Json(account).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn archive(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<ArchiveServiceAccountsRequest>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize(
            &principal,
            ResourceType::ServiceAccount,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .archive(request.ids, principal.actor_id)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn add_role(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AddServiceAccountRoleRequest>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state.service_accounts.add_role(id, request.role_id).await {
        Ok(account) => Json(account).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn remove_role(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, role_id)): Path<(Uuid, Uuid)>,
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
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state.service_accounts.remove_role(id, role_id).await {
        Ok(account) => Json(account).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn add_resource_access(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AddServiceAccountResourceAccessRequest>,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .add_resource_access(id, request)
        .await
    {
        Ok(account) => Json(account).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn remove_resource_access(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, resource_access_id)): Path<(Uuid, Uuid)>,
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
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Write,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .remove_resource_access(id, resource_access_id)
        .await
    {
        Ok(account) => Json(account).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn limits(principal: Option<Extension<ActorPrincipal>>, headers: HeaderMap) -> Response {
    if principal.is_none() {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    }
    Json(ServiceAccountLimitsView {
        default_token_lifetime_days: DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
        maximum_token_lifetime_days: MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
        maximum_active_tokens_per_account: MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS,
    })
    .into_response()
}

async fn list_tokens(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Query(filter): Query<TokenFilter>,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Read,
            None,
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .list_tokens(id, filter.page, filter.page_size)
        .await
    {
        Ok(paged_result) => Json(ServiceAccountTokensResponse { paged_result }).into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn create_token(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateServiceAccountTokenRequest>,
) -> Response {
    let principal = match require_human(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Read,
            Some(SpecificPermission::ManageCredentials),
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .create_token(id, request, principal.actor_id)
        .await
    {
        Ok(created) => {
            let mut response = (StatusCode::CREATED, Json(created)).into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
                .headers_mut()
                .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
            response
        }
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn revoke_token(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, token_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = state
        .identity
        .authorize_resource(
            &principal,
            ResourceType::ServiceAccount,
            id,
            PermissionLevel::Read,
            Some(SpecificPermission::ManageCredentials),
        )
        .await
    {
        return identity_error_response(error, &headers);
    }
    match state
        .service_accounts
        .revoke_token(id, token_id, principal.actor_id)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => identity_error_response(error, &headers),
    }
}

fn require_human(
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

fn require_human_administrator(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    let principal = require_human(principal)?;
    if !principal.is_administrator() {
        return Err(IdentityError::Forbidden);
    }
    Ok(principal)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenFilter {
    #[serde(default = "first_page")]
    page: i64,
    #[serde(default = "default_page_size")]
    page_size: i64,
}

const fn first_page() -> i64 {
    1
}

const fn default_page_size() -> i64 {
    50
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountsResponse {
    paged_result: PagedResult<ServiceAccountView>,
    capabilities: ResourceCapabilities,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountTokensResponse {
    paged_result: PagedResult<ServiceAccountTokenView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountDetailResponse {
    #[serde(flatten)]
    account: ServiceAccountView,
    capabilities: ServiceAccountCapabilities,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceCapabilities {
    can_read: bool,
    can_write: bool,
    can_execute: bool,
}

impl From<PermissionGrant> for ResourceCapabilities {
    fn from(permission: PermissionGrant) -> Self {
        Self {
            can_read: permission.level.grants(PermissionLevel::Read),
            can_write: permission.level.grants(PermissionLevel::Write),
            can_execute: permission.level.grants(PermissionLevel::Execute),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountCapabilities {
    can_use: bool,
    can_manage_credentials: bool,
    can_read: bool,
    can_write: bool,
    can_execute: bool,
}

impl From<PermissionGrant> for ServiceAccountCapabilities {
    fn from(permission: PermissionGrant) -> Self {
        Self {
            can_use: permission.has_specific(SpecificPermission::Use),
            can_manage_credentials: permission.has_specific(SpecificPermission::ManageCredentials),
            can_read: permission.level.grants(PermissionLevel::Read),
            can_write: permission.level.grants(PermissionLevel::Write),
            can_execute: permission.level.grants(PermissionLevel::Execute),
        }
    }
}
