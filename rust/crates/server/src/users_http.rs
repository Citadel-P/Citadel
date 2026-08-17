use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, PermissionGrant};
use citadel_identity::{
    AddUserRoleRequest, CreateUserRequest, DeleteUsersRequest, IdentityError, IdentityService,
    PagedResult, PatchUserRequest, RenameUserRequest, UserMutationService, UserReadService,
    UserResourceAccessRequest, UserView,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::capabilities::ResourceCapabilities;
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{
    IdentityHttpResult, identity_result, no_store, require_human_administrator,
};

#[derive(Clone)]
pub struct UsersHttpState {
    pub identity: Arc<IdentityService>,
    pub users: Arc<UserReadService>,
    pub mutations: Arc<UserMutationService>,
}

pub fn router(state: UsersHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_USERS, list)
        .contract_route(routes::SEARCH_USERS, search)
        .contract_route(routes::GET_USER, get)
        .contract_route(routes::CREATE_USER, create)
        .contract_route(routes::UPDATE_USER, patch)
        .contract_route(routes::RENAME_USER, rename)
        .contract_route(routes::ADD_USER_ROLE, add_role)
        .contract_route(routes::REMOVE_USER_ROLE, remove_role)
        .contract_route(routes::ADD_USER_RESOURCE_ACCESS, add_resource_access)
        .contract_route(routes::REMOVE_USER_RESOURCE_ACCESS, remove_resource_access)
        .contract_route(routes::DELETE_USERS, delete)
        .with_state(state)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsersFilter {
    #[serde(alias = "Page")]
    #[serde(default = "first_page")]
    page: i64,
    #[serde(alias = "PageSize")]
    #[serde(default = "default_page_size")]
    page_size: i64,
    #[serde(alias = "Name")]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserSearchFilter {
    #[serde(alias = "Query")]
    query: String,
    #[serde(alias = "Limit")]
    #[serde(default = "default_search_limit")]
    limit: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UsersResponse {
    paged_result: PagedResult<UserView>,
    capabilities: ResourceCapabilities,
}

async fn list(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<UsersFilter>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    let permission = identity_result(
        user_read_permission(&state.identity, &principal).await,
        &headers,
    )?;
    let filter = identity_result(user_query(query, "User filters are invalid."), &headers)?;
    let paged_result = identity_result(
        state
            .users
            .list(filter.page, filter.page_size, filter.name.as_deref())
            .await,
        &headers,
    )?;
    let paged_result = if paged_result.items.is_empty() {
        PagedResult {
            items: Vec::new(),
            total_count: 0,
            page: 0,
            page_size: 0,
        }
    } else {
        paged_result
    };
    Ok(no_store(
        Json(UsersResponse {
            paged_result,
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}

async fn search(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<UserSearchFilter>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        user_read_permission(&state.identity, &principal).await,
        &headers,
    )?;
    let filter = identity_result(
        user_query(query, "User search filters are invalid."),
        &headers,
    )?;
    let users = identity_result(
        state.users.search(&filter.query, filter.limit).await,
        &headers,
    )?;
    Ok(no_store(Json(users).into_response()))
}

async fn get(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        user_read_permission(&state.identity, &principal).await,
        &headers,
    )?;
    let id = identity_result(user_path(path), &headers)?;
    let user = identity_result(state.users.get(id).await, &headers)?;
    Ok(no_store(Json(user).into_response()))
}

async fn create(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    payload: Result<Json<CreateUserRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, None).await,
        &headers,
    )?;
    let request = identity_result(user_json(payload), &headers)?;
    let user = identity_result(
        state.mutations.create(request, principal.actor_id).await,
        &headers,
    )?;
    Ok(no_store(Json(user).into_response()))
}

async fn patch(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    payload: Result<Json<PatchUserRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, None).await,
        &headers,
    )?;
    let id = identity_result(user_path(path), &headers)?;
    let request = identity_result(user_json(payload), &headers)?;
    let user = identity_result(
        state.mutations.patch(id, request, principal.actor_id).await,
        &headers,
    )?;
    Ok(no_store(Json(user).into_response()))
}

async fn rename(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    payload: Result<Json<RenameUserRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, None).await,
        &headers,
    )?;
    let request = identity_result(user_json(payload), &headers)?;
    let user = identity_result(
        state
            .mutations
            .rename(request.id, &request.name, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(user).into_response()))
}

async fn add_role(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    payload: Result<Json<AddUserRoleRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let id = identity_result(user_path(path), &headers)?;
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let request = identity_result(user_json(payload), &headers)?;
    let user = identity_result(
        state
            .mutations
            .add_role(id, request.role_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(user).into_response()))
}

async fn remove_role(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, Uuid)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (id, role_id) = identity_result(user_path(path), &headers)?;
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let user = identity_result(
        state
            .mutations
            .remove_role(id, role_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(user).into_response()))
}

async fn add_resource_access(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    payload: Result<Json<UserResourceAccessRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let id = identity_result(user_path(path), &headers)?;
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let request = identity_result(user_json(payload), &headers)?;
    let user = identity_result(
        state
            .mutations
            .add_resource_access(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(user).into_response()))
}

async fn remove_resource_access(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    payload: Result<Json<UserResourceAccessRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let id = identity_result(user_path(path), &headers)?;
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let request = identity_result(user_json(payload), &headers)?;
    let user = identity_result(
        state
            .mutations
            .remove_resource_access(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(user).into_response()))
}

async fn delete(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    payload: Result<Json<DeleteUsersRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Execute, None).await,
        &headers,
    )?;
    let request = identity_result(user_json(payload), &headers)?;
    identity_result(
        state
            .mutations
            .delete(request.ids, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(axum::http::StatusCode::NO_CONTENT.into_response())
}

fn user_path<T>(path: Result<Path<T>, PathRejection>) -> Result<T, IdentityError> {
    path.map(|Path(value)| value)
        .map_err(|_| IdentityError::Validation("User path is invalid.".to_owned()))
}

fn user_json<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, IdentityError> {
    payload
        .map(|Json(value)| value)
        .map_err(|_| IdentityError::Validation("User request body is invalid.".to_owned()))
}

fn user_query<T>(
    query: Result<Query<T>, QueryRejection>,
    message: &str,
) -> Result<T, IdentityError> {
    query
        .map(|Query(value)| value)
        .map_err(|_| IdentityError::Validation(message.to_owned()))
}

async fn authorize_administrator(
    state: &UsersHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    level: PermissionLevel,
    resource_id: Option<Uuid>,
) -> Result<ActorPrincipal, IdentityError> {
    let principal = require_human_administrator(principal)?;
    if let Some(resource_id) = resource_id {
        state
            .identity
            .authorize_resource(&principal, ResourceType::User, resource_id, level, None)
            .await?;
    } else {
        state
            .identity
            .authorize(&principal, ResourceType::User, level, None)
            .await?;
    }
    Ok(principal)
}

async fn user_read_permission(
    identity: &IdentityService,
    principal: &ActorPrincipal,
) -> Result<PermissionGrant, IdentityError> {
    match identity
        .global_permission(principal, ResourceType::User)
        .await?
    {
        Some(permission) if permission.level.grants(PermissionLevel::Read) => Ok(permission),
        _ => Err(IdentityError::Forbidden),
    }
}

const fn first_page() -> i64 {
    1
}

const fn default_page_size() -> i64 {
    50
}

const fn default_search_limit() -> i64 {
    20
}
