use crate::request_validation::ApiPath;
use crate::request_validation::ApiQuery;
use crate::request_validation::ValidatedJson;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
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
use crate::identity_http::{
    IdentityHttpResult, identity_result, require_human, require_human_administrator,
};
use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct ServiceAccountHttpState {
    pub identity: Arc<IdentityService>,
    pub service_accounts: Arc<ServiceAccountService>,
}

pub fn router(state: ServiceAccountHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
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

#[utoipa::path(
    get,
    path = "/api/v1/serviceAccounts",
    operation_id = "listServiceAccounts",
    tag = "ServiceAccounts",
    summary = "List Service Accounts",
    responses(
        (status = 200, description = "Success", body = ServiceAccountsResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("Page" = Option<i64>, Query, minimum = 1, extensions(("x-citadel-default" = json!(1)))), ("PageSize" = Option<i64>, Query, minimum = 1, extensions(("x-citadel-default" = json!(50)))), ("Name" = Option<String>, Query), ("IncludeArchived" = Option<bool>, Query, extensions(("x-citadel-default" = json!(false))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiQuery(filter): ApiQuery<ListFilter>,
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

#[utoipa::path(
    get,
    path = "/api/v1/serviceAccounts/{id}",
    operation_id = "getServiceAccount",
    tag = "ServiceAccounts",
    summary = "Get a Service Account",
    responses(
        (status = 200, description = "Success", body = ServiceAccountDetailResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_one(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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

#[utoipa::path(
    post,
    path = "/api/v1/serviceAccounts",
    operation_id = "createServiceAccount",
    tag = "ServiceAccounts",
    summary = "Create a Service Account",
    request_body = CreateServiceAccountRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<CreateServiceAccountRequest>,
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

#[utoipa::path(
    patch,
    path = "/api/v1/serviceAccounts/{id}",
    operation_id = "updateServiceAccount",
    tag = "ServiceAccounts",
    summary = "Update a Service Account",
    request_body(content(
        (UpdateServiceAccountRequest = "application/merge-patch+json"),
        (UpdateServiceAccountRequest = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<UpdateServiceAccountRequest>,
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

#[utoipa::path(
    get,
    path = "/api/v1/serviceAccounts/{id}/usages",
    operation_id = "listServiceAccountUsages",
    tag = "ServiceAccounts",
    summary = "List Service Account execution usages",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/RunAsActorUsageList"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn usages(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
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
                PermissionLevel::Read,
                None,
            )
            .await,
        &headers,
    )?;
    let usages = identity_result(state.service_accounts.usages(id).await, &headers)?;
    Ok(crate::identity_http::no_store(Json(usages).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/serviceAccounts/rename",
    operation_id = "renameServiceAccount",
    tag = "ServiceAccounts",
    summary = "Rename a Service Account",
    request_body = RenameServiceAccountRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<RenameServiceAccountRequest>,
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

#[utoipa::path(
    delete,
    path = "/api/v1/serviceAccounts",
    operation_id = "archiveServiceAccounts",
    tag = "ServiceAccounts",
    summary = "Archive Service Accounts",
    request_body = ArchiveServiceAccountsRequest,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn archive(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<ArchiveServiceAccountsRequest>,
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

#[utoipa::path(
    post,
    path = "/api/v1/serviceAccounts/{id}/roles",
    operation_id = "addServiceAccountRole",
    tag = "ServiceAccounts",
    summary = "Assign a Role to a Service Account",
    request_body = AddServiceAccountRoleRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn add_role(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<AddServiceAccountRoleRequest>,
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

#[utoipa::path(
    delete,
    path = "/api/v1/serviceAccounts/{id}/roles/{roleId}",
    operation_id = "removeServiceAccountRole",
    tag = "ServiceAccounts",
    summary = "Remove a Role from a Service Account",
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path), ("roleId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove_role(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, role_id)): ApiPath<(Uuid, Uuid)>,
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

#[utoipa::path(
    post,
    path = "/api/v1/serviceAccounts/{id}/resource-accesses",
    operation_id = "addServiceAccountResourceAccess",
    tag = "ServiceAccounts",
    summary = "Add a resource override to a Service Account",
    request_body = AddServiceAccountResourceAccessRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn add_resource_access(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<AddServiceAccountResourceAccessRequest>,
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

#[utoipa::path(
    delete,
    path = "/api/v1/serviceAccounts/{id}/resource-accesses/{resourceAccessId}",
    operation_id = "removeServiceAccountResourceAccess",
    tag = "ServiceAccounts",
    summary = "Remove a resource override from a Service Account",
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path), ("resourceAccessId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove_resource_access(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, resource_access_id)): ApiPath<(Uuid, Uuid)>,
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

#[utoipa::path(
    get,
    path = "/api/v1/serviceAccounts/limits",
    operation_id = "getServiceAccountLimits",
    tag = "ServiceAccounts",
    summary = "Get Service Account limits",
    responses(
        (status = 200, description = "Success", body = citadel_identity::ServiceAccountLimitsView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/serviceAccounts/{id}/tokens",
    operation_id = "listServiceAccountTokens",
    tag = "ServiceAccounts",
    summary = "List Service Account tokens",
    responses(
        (status = 200, description = "Success", body = ServiceAccountTokensResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path), ("Page" = Option<i64>, Query, minimum = 1, extensions(("x-citadel-default" = json!(1)))), ("PageSize" = Option<i64>, Query, minimum = 1, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list_tokens(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ApiQuery(filter): ApiQuery<TokenFilter>,
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

#[utoipa::path(
    post,
    path = "/api/v1/serviceAccounts/{id}/tokens",
    operation_id = "createServiceAccountToken",
    tag = "ServiceAccounts",
    summary = "Create a Service Account token",
    request_body = CreateServiceAccountTokenRequest,
    responses(
        (status = 201, description = "Success", body = citadel_identity::CreatedServiceAccountTokenView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_token(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<CreateServiceAccountTokenRequest>,
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

#[utoipa::path(
    delete,
    path = "/api/v1/serviceAccounts/{id}/tokens/{tokenId}",
    operation_id = "revokeServiceAccountToken",
    tag = "ServiceAccounts",
    summary = "Revoke a Service Account token",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path), ("tokenId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn revoke_token(
    State(state): State<ServiceAccountHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, token_id)): ApiPath<(Uuid, Uuid)>,
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

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountsResponse {
    paged_result: PagedResult<ServiceAccountView>,
    capabilities: ResourceCapabilities,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountTokensResponse {
    paged_result: PagedResult<ServiceAccountTokenView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct ServiceAccountDetailResponse {
    #[serde(flatten)]
    account: ServiceAccountView,
    capabilities: ServiceAccountCapabilities,
}

#[derive(Serialize, utoipa::ToSchema)]
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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<ServiceAccountHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(archive))
        .normalized_routes(utoipa_axum::routes!(limits))
        .normalized_routes(utoipa_axum::routes!(get_one))
        .normalized_routes(utoipa_axum::routes!(usages))
        .normalized_routes(utoipa_axum::routes!(update))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(add_role))
        .normalized_routes(utoipa_axum::routes!(remove_role))
        .normalized_routes(utoipa_axum::routes!(add_resource_access))
        .normalized_routes(utoipa_axum::routes!(remove_resource_access))
        .normalized_routes(utoipa_axum::routes!(list_tokens))
        .normalized_routes(utoipa_axum::routes!(create_token))
        .normalized_routes(utoipa_axum::routes!(revoke_token))
}
