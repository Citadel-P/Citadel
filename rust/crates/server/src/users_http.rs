use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};

use axum::extract::{Extension, Path, Query, State};

use axum::http::HeaderMap;

use axum::response::IntoResponse;

use axum::{Json, Router};

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_identity::{ActorPrincipal, PermissionGrant};

use citadel_identity::{IdentityError, IdentityService, UserMutationService, UserReadService};

use crate::identity_http::dto::{
    AddUserRoleRequest, CreateUserRequest, DeleteUsersRequest, PagedResult, PatchUserRequest,
    RenameUserRequest, UserResourceAccessRequest, UserView,
};

use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::capabilities::ResourceCapabilities;

use crate::identity_http::{
    IdentityHttpResult, identity_result, no_store, require_human_administrator,
};

use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct UsersHttpState {
    pub identity: Arc<IdentityService>,
    pub users: Arc<UserReadService>,
    pub mutations: Arc<UserMutationService>,
}

pub fn router(state: UsersHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "User",
    )
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

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct UsersResponse {
    paged_result: PagedResult<UserView>,
    capabilities: ResourceCapabilities,
}

#[utoipa::path(
    get,
    path = "/api/v1/users",
    operation_id = "listUsers",
    tag = "Users",
    summary = "Get all Users",
    responses(
        (status = 200, description = "Success", body = UsersResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("Name" = Option<String>, Query), ("Page" = Option<i32>, Query, minimum = 1, extensions(("x-citadel-default" = json!(1)))), ("PageSize" = Option<i32>, Query, minimum = 1, maximum = 500, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let filter = identity_result(user_query(query), &headers)?;
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
        paged_result.into()
    };
    Ok(no_store(
        Json(UsersResponse {
            paged_result,
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/search",
    operation_id = "searchUsers",
    tag = "Users",
    summary = "Search Users for assignment",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/UserSearchItems"), content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("Query" = Option<String>, Query), ("Limit" = Option<i32>, Query, minimum = 1, maximum = 50, extensions(("x-citadel-default" = json!(20))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let filter = identity_result(user_query(query), &headers)?;
    let users = identity_result(
        state.users.search(&filter.query, filter.limit).await,
        &headers,
    )?;
    Ok(no_store(
        Json(
            users
                .into_iter()
                .map(crate::identity_http::dto::UserSearchItemView::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    operation_id = "getUser",
    tag = "Users",
    summary = "Get a User by ID",
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    operation_id = "createUser",
    tag = "Users",
    summary = "Create a User",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let request: CreateUserRequest = identity_result(user_json(payload), &headers)?;
    let request: citadel_identity::CreateUser = request.into();
    let user = identity_result(
        state.mutations.create(request, principal.actor_id).await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}",
    operation_id = "updateUser",
    tag = "Users",
    summary = "Update a User",
    request_body(content(
        (PatchUserRequest = "application/merge-patch+json"),
        (PatchUserRequest = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let request: PatchUserRequest = identity_result(user_json(payload), &headers)?;
    let request: citadel_identity::PatchUser = request.into();
    let user = identity_result(
        state.mutations.patch(id, request, principal.actor_id).await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/rename",
    operation_id = "renameUser",
    tag = "Users",
    summary = "Rename a User",
    request_body = RenameUserRequest,
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let request: RenameUserRequest = identity_result(user_json(payload), &headers)?;
    let request: citadel_identity::RenameUser = request.into();
    let user = identity_result(
        state
            .mutations
            .rename(request.id, &request.name, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/roles",
    operation_id = "addUserRole",
    tag = "Users",
    summary = "Assign a Role to a User",
    request_body = AddUserRoleRequest,
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let request: AddUserRoleRequest = identity_result(user_json(payload), &headers)?;
    let request: citadel_identity::AddUserRole = request.into();
    let user = identity_result(
        state
            .mutations
            .add_role(id, request.role_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/roles/{roleId}",
    operation_id = "removeUserRole",
    tag = "Users",
    summary = "Remove a Role from a User",
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("roleId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/resource-accesses",
    operation_id = "addUserResourceAccess",
    tag = "Users",
    summary = "Add a resource override to a User",
    request_body = UserResourceAccessRequest,
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let request: UserResourceAccessRequest = identity_result(user_json(payload), &headers)?;
    let request: citadel_identity::UserResourceAccess = request.into();
    let user = identity_result(
        state
            .mutations
            .add_resource_access(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/resource-accesses",
    operation_id = "removeUserResourceAccess",
    tag = "Users",
    summary = "Remove a resource override from a User",
    request_body = UserResourceAccessRequest,
    responses(
        (status = 200, description = "Success", body = crate::identity_http::dto::UserView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let request: UserResourceAccessRequest = identity_result(user_json(payload), &headers)?;
    let request: citadel_identity::UserResourceAccess = request.into();
    let user = identity_result(
        state
            .mutations
            .remove_resource_access(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::identity_http::dto::UserView::from(user)).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users",
    operation_id = "deleteUsers",
    tag = "Users",
    summary = "Delete Users",
    request_body = DeleteUsersRequest,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
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
    let request: DeleteUsersRequest = identity_result(user_json(payload), &headers)?;
    let request: citadel_identity::DeleteUsers = request.into();
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
        .map_err(crate::request_validation::invalid_path)
}

fn user_json<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, IdentityError> {
    payload
        .map(|Json(value)| value)
        .map_err(crate::request_validation::invalid_json)
}

fn user_query<T>(query: Result<Query<T>, QueryRejection>) -> Result<T, IdentityError> {
    query
        .map(|Query(value)| value)
        .map_err(crate::request_validation::invalid_query)
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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<UsersHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(search))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(patch))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(add_role))
        .normalized_routes(utoipa_axum::routes!(remove_role))
        .normalized_routes(utoipa_axum::routes!(add_resource_access))
        .normalized_routes(utoipa_axum::routes!(remove_resource_access))
        .normalized_routes(utoipa_axum::routes!(delete))
}
