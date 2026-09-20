use crate::capabilities::ResourceCapabilitiesView;
mod projects;
pub(crate) use projects::authorized_projects;
use projects::*;
mod runs;
use runs::*;
mod agent_pools;
use super::capabilities::{granted, pool_capabilities, project_capabilities};
pub(crate) use agent_pools::authorized_pools;
use agent_pools::*;

use citadel_primitives::EffectivePermission;

use crate::api::builds::views::*;

use crate::api::builds::requests::*;

use crate::request_validation::ApiPath;

use crate::request_validation::ApiQuery;

use crate::request_validation::ValidatedJson;

use std::sync::Arc;

use axum::extract::{Extension, RawQuery, State};

use axum::http::{HeaderMap, StatusCode};

use axum::response::IntoResponse;

use axum::{Json, Router};

use citadel_builds::{BuildError, BuildProjectConfiguration, BuildService};

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};

use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct BuildsHttpState {
    pub identity: Arc<IdentityService>,
    pub builds: Arc<BuildService>,
}

pub fn router(state: BuildsHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes()
            .split_for_parts()
            .0
            .with_state(state.clone()),
        "Build",
    )
    .merge(crate::realtime::notify_mutations(
        documented_pool_routes()
            .split_for_parts()
            .0
            .with_state(state),
        "BuildAgentPool",
    ))
}

fn actor(
    value: Option<Extension<ActorPrincipal>>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    identity_result(
        value
            .map(|Extension(value)| value)
            .ok_or(IdentityError::Unauthenticated),
        headers,
    )
}

async fn authorize_global(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    authorize_global_for(state, principal, ResourceType::Build, level, headers).await
}

async fn authorize_global_for(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        match (resource_type, level) {
            (ResourceType::Build, PermissionLevel::Read) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ReadBuild>(principal)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Write) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::WriteBuild>(principal)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ExecuteBuild>(principal)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Read) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ReadBuildAgentPool>(principal)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Write) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::WriteBuildAgentPool>(principal)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_scope::<citadel_builds::permissions::ExecuteBuildAgentPool>(principal)
                    .await
            }
            _ => {
                state
                    .identity
                    .authorize(principal, resource_type, level, None)
                    .await
            }
        },
        headers,
    )?;
    Ok(())
}

async fn authorize(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    authorize_for(state, principal, ResourceType::Build, id, level, headers).await
}

async fn authorize_for(
    state: &BuildsHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        match (resource_type, level) {
            (ResourceType::Build, PermissionLevel::Read) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ReadBuild>(principal, id)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Write) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::WriteBuild>(principal, id)
                    .await
            }
            (ResourceType::Build, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ExecuteBuild>(principal, id)
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Read) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ReadBuildAgentPool>(
                        principal, id,
                    )
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Write) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::WriteBuildAgentPool>(
                        principal, id,
                    )
                    .await
            }
            (ResourceType::BuildAgentPool, PermissionLevel::Execute) => {
                state
                    .identity
                    .require_resource::<citadel_builds::permissions::ExecuteBuildAgentPool>(
                        principal, id,
                    )
                    .await
            }
            _ => {
                state
                    .identity
                    .authorize_resource(principal, resource_type, id, level, None)
                    .await
            }
        },
        headers,
    )?;
    Ok(())
}

fn map_error(error: BuildError) -> IdentityError {
    match error {
        BuildError::LicenseRequired(capability) => {
            IdentityError::LicenseRequired(capability.as_license_key())
        }
        BuildError::Validation(message) => IdentityError::Validation(message),
        BuildError::NotFound => IdentityError::NotFound,
        BuildError::Conflict(message) => IdentityError::Conflict(message),
        BuildError::Storage(message) => IdentityError::Storage(message),
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<BuildsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_projects))
        .normalized_routes(utoipa_axum::routes!(create_project))
        .normalized_routes(utoipa_axum::routes!(get_project))
        .normalized_routes(utoipa_axum::routes!(update_project))
        .normalized_routes(utoipa_axum::routes!(rename_project))
        .normalized_routes(utoipa_axum::routes!(update_project_metadata))
        .normalized_routes(utoipa_axum::routes!(archive_project))
        .normalized_routes(utoipa_axum::routes!(queue_run))
        .normalized_routes(utoipa_axum::routes!(list_runs))
        .normalized_routes(utoipa_axum::routes!(get_run))
        .normalized_routes(utoipa_axum::routes!(get_logs))
        .normalized_routes(utoipa_axum::routes!(cancel_run))
}

pub(crate) fn documented_pool_routes() -> utoipa_axum::router::OpenApiRouter<BuildsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_pools))
        .normalized_routes(utoipa_axum::routes!(create_pool))
        .normalized_routes(utoipa_axum::routes!(get_pool))
        .normalized_routes(utoipa_axum::routes!(update_pool))
        .normalized_routes(utoipa_axum::routes!(rename_pool))
        .normalized_routes(utoipa_axum::routes!(update_pool_metadata))
        .normalized_routes(utoipa_axum::routes!(test_pool))
        .normalized_routes(utoipa_axum::routes!(archive_pool))
        .normalized_routes(utoipa_axum::routes!(enroll_pool))
        .normalized_routes(utoipa_axum::routes!(pool_edge_status))
        .normalized_routes(utoipa_axum::routes!(revoke_pool_edge))
}
