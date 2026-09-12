use crate::request_validation::ApiPath;
use crate::request_validation::ValidatedJson;
use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Extension, Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, PermissionGrant};
use citadel_identity::{
    AddTeamMemberRequest, AddTeamRoleRequest, CreateTeamRequest, DeleteTeamsRequest, IdentityError,
    IdentityService, PagedResult, PatchTeamRequest, RenameTeamRequest, TeamMutationService,
    TeamReadService, TeamResourceAccessInput, TeamView,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::capabilities::ResourceCapabilities;
use crate::identity_http::{
    IdentityHttpResult, identity_result, no_store, require_human_administrator,
};
use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct TeamsHttpState {
    pub identity: Arc<IdentityService>,
    pub teams: Arc<TeamReadService>,
    pub mutations: Arc<TeamMutationService>,
}

pub fn router(state: TeamsHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "Team",
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamsFilter {
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
struct TeamSearchFilter {
    #[serde(alias = "Query")]
    query: String,
    #[serde(alias = "Limit")]
    #[serde(default = "default_search_limit")]
    limit: i64,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct TeamsResponse {
    paged_result: PagedResult<TeamView>,
    capabilities: ResourceCapabilities,
}

#[utoipa::path(
    get,
    path = "/api/v1/teams",
    operation_id = "listTeams",
    summary = "Get all Teams",
    responses(
        (status = 200, description = "Success", body = TeamsResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("Name" = Option<String>, Query), ("Page" = Option<i32>, Query, minimum = 1, extensions(("x-citadel-default" = json!(1)))), ("PageSize" = Option<i32>, Query, minimum = 1, maximum = 500, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<TeamsFilter>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    let permission = identity_result(
        team_read_permission(&state.identity, &principal).await,
        &headers,
    )?;
    let filter = identity_result(team_query(query), &headers)?;
    let paged_result = identity_result(
        state
            .teams
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
        Json(TeamsResponse {
            paged_result,
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/teams/search",
    operation_id = "searchTeams",
    summary = "Search Teams for assignment",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/TeamSearchItems"), content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("Query" = Option<String>, Query), ("Limit" = Option<i32>, Query, minimum = 1, maximum = 50, extensions(("x-citadel-default" = json!(20))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn search(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<TeamSearchFilter>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        team_read_permission(&state.identity, &principal).await,
        &headers,
    )?;
    let filter = identity_result(team_query(query), &headers)?;
    let teams = identity_result(
        state.teams.search(&filter.query, filter.limit).await,
        &headers,
    )?;
    Ok(no_store(Json(teams).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/teams/{id}",
    operation_id = "getTeam",
    summary = "Get a Team by ID",
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_human_administrator(principal), &headers)?;
    identity_result(
        team_read_permission(&state.identity, &principal).await,
        &headers,
    )?;
    let team = identity_result(state.teams.get(id).await, &headers)?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/teams",
    operation_id = "createTeam",
    summary = "Create a Team",
    request_body = CreateTeamRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<CreateTeamRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, None).await,
        &headers,
    )?;
    let team = identity_result(
        state.mutations.create(request, principal.actor_id).await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/teams/{id}",
    operation_id = "updateTeam",
    summary = "Update a Team",
    request_body = PatchTeamRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn patch(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<PatchTeamRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let team = identity_result(
        state.mutations.patch(id, request, principal.actor_id).await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/teams/rename",
    operation_id = "renameTeam",
    summary = "Rename a Team",
    request_body = RenameTeamRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<RenameTeamRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, None).await,
        &headers,
    )?;
    let team = identity_result(
        state
            .mutations
            .rename(request.id, &request.name, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/teams/{id}/roles",
    operation_id = "addTeamRole",
    summary = "Assign a Role to a Team",
    request_body = AddTeamRoleRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn add_role(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<AddTeamRoleRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let team = identity_result(
        state
            .mutations
            .add_role(id, request.role_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/teams/{id}/roles/{roleId}",
    operation_id = "removeTeamRole",
    summary = "Remove a Role from a Team",
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("roleId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove_role(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, role_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let team = identity_result(
        state
            .mutations
            .remove_role(id, role_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/teams/{id}/members",
    operation_id = "addTeamMember",
    summary = "Add an Actor to a Team",
    request_body = AddTeamMemberRequest,
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn add_member(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<AddTeamMemberRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let team = identity_result(
        state
            .mutations
            .add_member(id, request.member_actor_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/teams/{id}/members/{memberActorId}",
    operation_id = "removeTeamMember",
    summary = "Remove an Actor from a Team",
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path), ("memberActorId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove_member(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath((id, member_actor_id)): ApiPath<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let team = identity_result(
        state
            .mutations
            .remove_member(id, member_actor_id, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/teams/{id}/resource-accesses",
    operation_id = "addTeamResourceAccess",
    summary = "Add a resource override to a Team",
    request_body = TeamResourceAccessInput,
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn add_resource_access(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<TeamResourceAccessInput>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let team = identity_result(
        state
            .mutations
            .add_resource_access(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/teams/{id}/resource-accesses",
    operation_id = "removeTeamResourceAccess",
    summary = "Remove a resource override from a Team",
    request_body = TeamResourceAccessInput,
    responses(
        (status = 200, description = "Success", body = citadel_identity::TeamView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn remove_resource_access(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<TeamResourceAccessInput>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Write, Some(id)).await,
        &headers,
    )?;
    let team = identity_result(
        state
            .mutations
            .remove_resource_access(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(Json(team).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/teams",
    operation_id = "deleteTeams",
    summary = "Delete Teams",
    request_body = DeleteTeamsRequest,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(request): ValidatedJson<DeleteTeamsRequest>,
) -> IdentityHttpResult {
    let principal = identity_result(
        authorize_administrator(&state, principal, PermissionLevel::Execute, None).await,
        &headers,
    )?;
    identity_result(
        state
            .mutations
            .delete(request.ids, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(axum::http::StatusCode::NO_CONTENT.into_response())
}

async fn authorize_administrator(
    state: &TeamsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    level: PermissionLevel,
    resource_id: Option<Uuid>,
) -> Result<ActorPrincipal, IdentityError> {
    let principal = require_human_administrator(principal)?;
    if let Some(resource_id) = resource_id {
        state
            .identity
            .authorize_resource(&principal, ResourceType::Team, resource_id, level, None)
            .await?;
    } else {
        state
            .identity
            .authorize(&principal, ResourceType::Team, level, None)
            .await?;
    }
    Ok(principal)
}

async fn team_read_permission(
    identity: &IdentityService,
    principal: &ActorPrincipal,
) -> Result<PermissionGrant, IdentityError> {
    match identity
        .global_permission(principal, ResourceType::Team)
        .await?
    {
        Some(permission) if permission.level.grants(PermissionLevel::Read) => Ok(permission),
        _ => Err(IdentityError::Forbidden),
    }
}

fn team_query<T>(query: Result<Query<T>, QueryRejection>) -> Result<T, IdentityError> {
    query
        .map(|Query(value)| value)
        .map_err(crate::request_validation::invalid_query)
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

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<TeamsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(search))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(patch))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(add_role))
        .normalized_routes(utoipa_axum::routes!(remove_role))
        .normalized_routes(utoipa_axum::routes!(add_member))
        .normalized_routes(utoipa_axum::routes!(remove_member))
        .normalized_routes(utoipa_axum::routes!(add_resource_access))
        .normalized_routes(utoipa_axum::routes!(remove_resource_access))
        .normalized_routes(utoipa_axum::routes!(delete))
}
