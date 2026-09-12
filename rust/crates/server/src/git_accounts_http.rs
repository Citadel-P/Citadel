use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_git::{
    GitAccountError, GitAccountInput, GitAccountPatch, GitAccountService, GitAccountView,
};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_platforms::ResourceCapabilitiesView;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::realtime::RealtimeHub;

#[derive(Clone)]
pub struct GitAccountsHttpState {
    pub identity: Arc<IdentityService>,
    pub accounts: Arc<GitAccountService>,
    pub realtime: Option<RealtimeHub>,
}

pub fn router(state: GitAccountsHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_GIT_ACCOUNTS, list)
        .contract_route(routes::CREATE_GIT_ACCOUNT, create)
        .contract_route(routes::DELETE_GIT_ACCOUNTS, delete)
        .contract_route(routes::GET_GIT_ACCOUNT, get)
        .contract_route(routes::GET_GIT_ACCOUNT_CONFIG, get_config)
        .contract_route(routes::UPDATE_GIT_ACCOUNT, update)
        .with_state(state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GitAccountsResponse {
    git_accounts: Vec<AuthorizedGitAccountView>,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizedGitAccountView {
    #[serde(flatten)]
    account: GitAccountView,
    capabilities: ResourceCapabilitiesView,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteGitAccountsInput {
    ids: Vec<Uuid>,
}

async fn list(
    State(state): State<GitAccountsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let global = permission(&state, &principal, None, &headers).await?;
    let accounts = identity_result(
        state
            .accounts
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(account_error),
        &headers,
    )?;
    let mut authorized = Vec::with_capacity(accounts.len());
    for account in accounts {
        let capabilities = permission(&state, &principal, Some(account.id), &headers).await?;
        authorized.push(AuthorizedGitAccountView {
            account,
            capabilities,
        });
    }
    Ok(no_store(
        Json(GitAccountsResponse {
            git_accounts: authorized,
            capabilities: global,
        })
        .into_response(),
    ))
}

async fn get(
    State(state): State<GitAccountsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(
        &state,
        &principal,
        Some(id),
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let capabilities = permission(&state, &principal, Some(id), &headers).await?;
    let account = identity_result(
        state.accounts.get(id).await.map_err(account_error),
        &headers,
    )?;
    Ok(no_store(
        Json(AuthorizedGitAccountView {
            account,
            capabilities,
        })
        .into_response(),
    ))
}

async fn get_config(
    State(state): State<GitAccountsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(
        &state,
        &principal,
        Some(id),
        PermissionLevel::Read,
        &headers,
    )
    .await?;
    let account = identity_result(
        state.accounts.get_config(id).await.map_err(account_error),
        &headers,
    )?;
    Ok(no_store(Json(account).into_response()))
}

async fn create(
    State(state): State<GitAccountsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<GitAccountInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    authorize(&state, &principal, None, PermissionLevel::Write, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let account = identity_result(
        state
            .accounts
            .create(principal.actor_id, input)
            .await
            .map_err(account_error),
        &headers,
    )?;
    publish(&state);
    Ok(no_store(Json(account).into_response()))
}

async fn update(
    State(state): State<GitAccountsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<GitAccountPatch>, JsonRejection>,
) -> IdentityHttpResult {
    let (principal, id) = principal_and_id(principal, path, &headers)?;
    authorize(
        &state,
        &principal,
        Some(id),
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let account = identity_result(
        state
            .accounts
            .update(id, input)
            .await
            .map_err(account_error),
        &headers,
    )?;
    publish(&state);
    Ok(no_store(Json(account).into_response()))
}

async fn delete(
    State(state): State<GitAccountsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<DeleteGitAccountsInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    for id in &input.ids {
        authorize(
            &state,
            &principal,
            Some(*id),
            PermissionLevel::Execute,
            &headers,
        )
        .await?;
    }
    identity_result(
        state
            .accounts
            .delete(&input.ids)
            .await
            .map_err(account_error),
        &headers,
    )?;
    publish(&state);
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(value)| value)
        .ok_or(IdentityError::Unauthenticated)
}

fn principal_and_id(
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: &HeaderMap,
) -> IdentityHttpResult<(ActorPrincipal, Uuid)> {
    let principal = identity_result(require_actor(principal), headers)?;
    let Path(id) = identity_result(
        path.map_err(|error| IdentityError::Validation(error.to_string())),
        headers,
    )?;
    Ok((principal, id))
}

async fn authorize(
    state: &GitAccountsHttpState,
    principal: &ActorPrincipal,
    id: Option<Uuid>,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let result = match id {
        Some(id) => {
            state
                .identity
                .authorize_resource(principal, ResourceType::GitAccount, id, level, None)
                .await
        }
        None => {
            state
                .identity
                .authorize(principal, ResourceType::GitAccount, level, None)
                .await
        }
    };
    identity_result(result, headers)
}

async fn permission(
    state: &GitAccountsHttpState,
    principal: &ActorPrincipal,
    id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceCapabilitiesView> {
    let permission = match id {
        Some(id) => {
            state
                .identity
                .permission_for_resource(principal, ResourceType::GitAccount, id)
                .await
        }
        None => {
            state
                .identity
                .global_permission(principal, ResourceType::GitAccount)
                .await
        }
    };
    let permission = identity_result(permission, headers)?;
    Ok(
        permission.map_or_else(ResourceCapabilitiesView::default, |permission| {
            ResourceCapabilitiesView {
                can_read: permission.level.grants(PermissionLevel::Read),
                can_write: permission.level.grants(PermissionLevel::Write),
                can_execute: permission.level.grants(PermissionLevel::Execute),
            }
        }),
    )
}

fn account_error(error: GitAccountError) -> IdentityError {
    match error {
        GitAccountError::Validation(message) => crate::request_validation::validation_error(message),
        GitAccountError::NotFound => IdentityError::NotFound,
        GitAccountError::Conflict(message) => IdentityError::Conflict(message),
        GitAccountError::Credential => IdentityError::Credential,
        GitAccountError::Storage(message) => IdentityError::Storage(message),
    }
}

fn invalid_json(error: JsonRejection) -> IdentityError {
    crate::request_validation::invalid_json(error)
}

fn publish(state: &GitAccountsHttpState) {
    if let Some(realtime) = &state.realtime {
        realtime.publish_resource_change("GitAccount", Uuid::nil(), "gitAccountChanged");
    }
}
