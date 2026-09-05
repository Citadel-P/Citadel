use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::{ActorPrincipal, PermissionGrant};
use citadel_identity::{
    AddServiceAccountResourceAccessRequest, AddServiceAccountRoleRequest,
    ArchiveServiceAccountsRequest, CreateServiceAccountRequest, CreateServiceAccountTokenRequest,
    DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS, IdentityError, IdentityService,
    MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS, MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
    PagedResult, RenameServiceAccountRequest, ServiceAccountLimitsView, ServiceAccountService,
    ServiceAccountTokenView, ServiceAccountView, UpdateServiceAccountRequest,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::capabilities::ResourceCapabilities;
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{
    IdentityHttpResult, identity_result, require_human, require_human_administrator,
};

#[derive(Clone)]
pub struct ServiceAccountHttpState {
    pub identity: Arc<IdentityService>,
    pub service_accounts: Arc<ServiceAccountService>,
}

pub fn router(state: ServiceAccountHttpState) -> Router {
    crate::realtime::notify_mutations(
        Router::new()
            .contract_route(routes::LIST_SERVICE_ACCOUNTS, list)
            .contract_route(routes::CREATE_SERVICE_ACCOUNT, create)
            .contract_route(routes::ARCHIVE_SERVICE_ACCOUNTS, archive)
            .contract_route(routes::GET_SERVICE_ACCOUNT_LIMITS, limits)
            .contract_route(routes::GET_SERVICE_ACCOUNT, get_one)
            .contract_route(routes::UPDATE_SERVICE_ACCOUNT, update)
            .contract_route(routes::RENAME_SERVICE_ACCOUNT, rename)
            .contract_route(routes::ADD_SERVICE_ACCOUNT_ROLE, add_role)
            .contract_route(routes::REMOVE_SERVICE_ACCOUNT_ROLE, remove_role)
            .contract_route(
                routes::ADD_SERVICE_ACCOUNT_RESOURCE_ACCESS,
                add_resource_access,
            )
            .contract_route(
                routes::REMOVE_SERVICE_ACCOUNT_RESOURCE_ACCESS,
                remove_resource_access,
            )
            .contract_route(routes::LIST_SERVICE_ACCOUNT_TOKENS, list_tokens)
            .contract_route(routes::CREATE_SERVICE_ACCOUNT_TOKEN, create_token)
            .contract_route(routes::REVOKE_SERVICE_ACCOUNT_TOKEN, revoke_token)
            .with_state(state),
        "ServiceAccount",
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListFilter {
    #[serde(alias = "Page")]
    #[serde(default = "first_page")]
    page: i64,
    #[serde(alias = "PageSize")]
    #[serde(default = "default_page_size")]
    page_size: i64,
    #[serde(alias = "Name")]
    name: Option<String>,
    #[serde(alias = "IncludeArchived")]
    #[serde(default)]
    include_archived: bool,
}

async fn list(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Query(filter): Query<ListFilter>,
) -> IdentityHttpResult {
    let principal = identity_result(authenticated_principal(principal), &headers)?;
    let permission = identity_result(
        service_account_list_permission(&state.identity, &principal).await,
        &headers,
    )?;
    let paged_result = identity_result(
        state
            .service_accounts
            .list(
                principal.actor_id,
                principal.is_administrator(),
                filter.include_archived,
                filter.name.as_deref(),
                filter.page,
                filter.page_size,
            )
            .await,
        &headers,
    )?;
    Ok(Json(ServiceAccountsResponse {
        paged_result,
        capabilities: ResourceCapabilities::from(permission),
    })
    .into_response())
}

async fn get_one(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(authenticated_principal(principal), &headers)?;
    let permission = identity_result(
        service_account_read_permission(&state.identity, &principal, id).await,
        &headers,
    )?;
    let account = identity_result(state.service_accounts.get(id).await, &headers)?;
    Ok(Json(ServiceAccountDetailResponse {
        account,
        capabilities: ServiceAccountCapabilities::from(permission),
    })
    .into_response())
}

async fn create(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<CreateServiceAccountRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::ServiceAccount,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let account = identity_result(
        state
            .service_accounts
            .create(request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok((StatusCode::OK, Json(account)).into_response())
}

async fn update(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateServiceAccountRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let account = identity_result(
        state
            .service_accounts
            .update(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(Json(account).into_response())
}

async fn rename(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<RenameServiceAccountRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                request.id,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let account = identity_result(
        state
            .service_accounts
            .rename(request.id, &request.name, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(Json(account).into_response())
}

async fn archive(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<ArchiveServiceAccountsRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::ServiceAccount,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    identity_result(
        state
            .service_accounts
            .archive(request.ids, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn add_role(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AddServiceAccountRoleRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let account = identity_result(
        state
            .service_accounts
            .add_role(id, request.role_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(Json(account).into_response())
}

async fn remove_role(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, role_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let account = identity_result(
        state
            .service_accounts
            .remove_role(id, role_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(Json(account).into_response())
}

async fn add_resource_access(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AddServiceAccountResourceAccessRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let account = identity_result(
        state
            .service_accounts
            .add_resource_access(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(Json(account).into_response())
}

async fn remove_resource_access(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, resource_access_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let account = identity_result(
        state
            .service_accounts
            .remove_resource_access(id, resource_access_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(Json(account).into_response())
}

async fn limits(
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    identity_result(authenticated_principal(principal), &headers)?;
    Ok(Json(ServiceAccountLimitsView {
        default_token_lifetime_days: DEFAULT_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
        maximum_token_lifetime_days: MAXIMUM_SERVICE_ACCOUNT_TOKEN_LIFETIME_DAYS,
        maximum_active_tokens_per_account: MAXIMUM_ACTIVE_SERVICE_ACCOUNT_TOKENS,
    })
    .into_response())
}

async fn list_tokens(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Query(filter): Query<TokenFilter>,
) -> IdentityHttpResult {
    let principal = identity_result(authenticated_principal(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Read,
                None,
            )
            .await,
        &headers,
    )?;
    let paged_result = identity_result(
        state
            .service_accounts
            .list_tokens(id, filter.page, filter.page_size)
            .await,
        &headers,
    )?;
    Ok(Json(ServiceAccountTokensResponse { paged_result }).into_response())
}

async fn create_token(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateServiceAccountTokenRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(require_human(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Read,
                Some(SpecificPermission::ManageCredentials),
            )
            .await,
        &headers,
    )?;
    let created = identity_result(
        state
            .service_accounts
            .create_token(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    let mut response = (StatusCode::CREATED, Json(created)).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    Ok(response)
}

async fn revoke_token(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, token_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::ServiceAccount,
                id,
                PermissionLevel::Read,
                Some(SpecificPermission::ManageCredentials),
            )
            .await,
        &headers,
    )?;
    identity_result(
        state
            .service_accounts
            .revoke_token(id, token_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

fn authenticated_principal(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(principal)| principal)
        .ok_or(IdentityError::Unauthenticated)
}

async fn service_account_list_permission(
    identity: &IdentityService,
    principal: &ActorPrincipal,
) -> Result<PermissionGrant, IdentityError> {
    match identity
        .global_permission(principal, ResourceType::ServiceAccount)
        .await?
    {
        Some(permission) if permission.level.grants(PermissionLevel::Read) => Ok(permission),
        _ => Err(IdentityError::Forbidden),
    }
}

async fn service_account_read_permission(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    id: Uuid,
) -> Result<PermissionGrant, IdentityError> {
    match identity
        .permission_for_resource(principal, ResourceType::ServiceAccount, id)
        .await?
    {
        Some(permission) if permission.level.grants(PermissionLevel::Read) => Ok(permission),
        _ => Err(IdentityError::NotFound),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenFilter {
    #[serde(alias = "Page")]
    #[serde(default = "first_page")]
    page: i64,
    #[serde(alias = "PageSize")]
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
