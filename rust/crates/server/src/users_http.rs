use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use citadel_application::{IdentityError, IdentityService, PagedResult, UserReadService, UserView};
use citadel_contracts::http::routes;
use citadel_domain::{ActorPrincipal, PermissionGrant, PermissionLevel, ResourceType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::capabilities::ResourceCapabilities;
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{identity_error_response, no_store, require_human_administrator};

#[derive(Clone)]
pub struct UsersHttpState {
    pub identity: Arc<IdentityService>,
    pub users: Arc<UserReadService>,
}

pub fn router(state: UsersHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_USERS, list)
        .contract_route(routes::SEARCH_USERS, search)
        .contract_route(routes::GET_USER, get)
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
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    let permission = match user_read_permission(&state.identity, &principal).await {
        Ok(permission) => permission,
        Err(error) => return identity_error_response(error, &headers),
    };
    let Ok(Query(filter)) = query else {
        return identity_error_response(
            IdentityError::Validation("User filters are invalid.".to_owned()),
            &headers,
        );
    };
    match state
        .users
        .list(filter.page, filter.page_size, filter.name.as_deref())
        .await
    {
        Ok(paged_result) => {
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
            no_store(
                Json(UsersResponse {
                    paged_result,
                    capabilities: permission.into(),
                })
                .into_response(),
            )
        }
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn search(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<UserSearchFilter>, QueryRejection>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = user_read_permission(&state.identity, &principal).await {
        return identity_error_response(error, &headers);
    }
    let Ok(Query(filter)) = query else {
        return identity_error_response(
            IdentityError::Validation("User search filters are invalid.".to_owned()),
            &headers,
        );
    };
    match state.users.search(&filter.query, filter.limit).await {
        Ok(users) => no_store(Json(users).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn get(
    State(state): State<UsersHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let principal = match require_human_administrator(principal) {
        Ok(principal) => principal,
        Err(error) => return identity_error_response(error, &headers),
    };
    if let Err(error) = user_read_permission(&state.identity, &principal).await {
        return identity_error_response(error, &headers);
    }
    match state.users.get(id).await {
        Ok(user) => no_store(Json(user).into_response()),
        Err(error) => identity_error_response(error, &headers),
    }
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
