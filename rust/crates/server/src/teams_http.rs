use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
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
use crate::contract_router::ContractRouterExt;
use crate::identity_http::{
    IdentityHttpResult, identity_result, no_store, require_human_administrator,
};

#[derive(Clone)]
pub struct TeamsHttpState {
    pub identity: Arc<IdentityService>,
    pub teams: Arc<TeamReadService>,
    pub mutations: Arc<TeamMutationService>,
}

pub fn router(state: TeamsHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_TEAMS, list)
        .contract_route(routes::SEARCH_TEAMS, search)
        .contract_route(routes::GET_TEAM, get)
        .contract_route(routes::CREATE_TEAM, create)
        .contract_route(routes::PATCH_TEAM, patch)
        .contract_route(routes::RENAME_TEAM, rename)
        .contract_route(routes::ADD_TEAM_ROLE, add_role)
        .contract_route(routes::REMOVE_TEAM_ROLE, remove_role)
        .contract_route(routes::ADD_TEAM_MEMBER, add_member)
        .contract_route(routes::REMOVE_TEAM_MEMBER, remove_member)
        .contract_route(routes::ADD_TEAM_RESOURCE_ACCESS, add_resource_access)
        .contract_route(routes::REMOVE_TEAM_RESOURCE_ACCESS, remove_resource_access)
        .contract_route(routes::DELETE_TEAMS, delete)
        .with_state(state)
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamsResponse {
    paged_result: PagedResult<TeamView>,
    capabilities: ResourceCapabilities,
}

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
    let filter = identity_result(team_query(query, "Team filters are invalid."), &headers)?;
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
    let filter = identity_result(
        team_query(query, "Team search filters are invalid."),
        &headers,
    )?;
    let teams = identity_result(
        state.teams.search(&filter.query, filter.limit).await,
        &headers,
    )?;
    Ok(no_store(Json(teams).into_response()))
}

async fn get(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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

async fn create(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<CreateTeamRequest>,
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

async fn patch(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<PatchTeamRequest>,
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

async fn rename(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<RenameTeamRequest>,
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

async fn add_role(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AddTeamRoleRequest>,
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

async fn remove_role(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, role_id)): Path<(Uuid, Uuid)>,
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

async fn add_member(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<AddTeamMemberRequest>,
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

async fn remove_member(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path((id, member_actor_id)): Path<(Uuid, Uuid)>,
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

async fn add_resource_access(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TeamResourceAccessInput>,
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

async fn remove_resource_access(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TeamResourceAccessInput>,
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

async fn delete(
    State(state): State<TeamsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(request): Json<DeleteTeamsRequest>,
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

fn team_query<T>(
    query: Result<Query<T>, QueryRejection>,
    message: &str,
) -> Result<T, IdentityError> {
    query
        .map(|Query(value)| value)
        .map_err(|_| IdentityError::Validation(message.to_owned()))
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
