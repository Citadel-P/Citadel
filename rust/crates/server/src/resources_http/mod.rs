use crate::request_validation::invalid_json;
pub(crate) mod bindings;
pub(crate) mod catalog;
pub(crate) mod tags;

use std::sync::Arc;

use axum::Router;
use axum::extract::Extension;
use axum::http::HeaderMap;
use citadel_domain::{PermissionLevel, ResourceType, SpecificPermission};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_platforms::ResourceCapabilitiesView;
use citadel_resources::{ResourceMetadataError, ResourceMetadataService};
use uuid::Uuid;

use crate::identity_http::{IdentityHttpError, IdentityHttpResult, identity_result};
use crate::realtime::RealtimeHub;

#[derive(Clone)]
pub struct ResourcesHttpState {
    pub identity: Arc<IdentityService>,
    pub resources: Arc<ResourceMetadataService>,
    pub realtime: Option<RealtimeHub>,
}

pub fn router(state: ResourcesHttpState) -> Router {
    Router::new()
        .merge(tags::router(state.clone()))
        .merge(catalog::router(state.clone()))
        .merge(bindings::router(state))
}

fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(value)| value)
        .ok_or(IdentityError::Unauthenticated)
}

async fn authorize_global(
    state: &ResourcesHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        state
            .identity
            .authorize(principal, resource_type, level, None)
            .await,
        headers,
    )?;
    Ok(())
}

async fn authorize_resource(
    state: &ResourcesHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    resource_id: Uuid,
    level: PermissionLevel,
    specific: Option<SpecificPermission>,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    // Workload metadata uses the owning feature's named policies.
    let result = match (resource_type, level, specific) {
        (ResourceType::Stack, PermissionLevel::Read, None) => Some(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::ReadStack>(principal, resource_id)
                .await,
        ),
        (ResourceType::Stack, PermissionLevel::Write, None) => Some(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::WriteStack>(principal, resource_id)
                .await,
        ),
        (
            ResourceType::Stack,
            PermissionLevel::Read,
            Some(SpecificPermission::ResourceBindings),
        ) => Some(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::ReadStackBindings>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        (
            ResourceType::Stack,
            PermissionLevel::Write,
            Some(SpecificPermission::ResourceBindings),
        ) => Some(
            state
                .identity
                .require_resource::<citadel_stacks::permissions::WriteStackBindings>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        (ResourceType::SwarmService, PermissionLevel::Read, None) => Some(
            state
                .identity
                .require_resource::<citadel_swarm_services::permissions::ReadSwarmService>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        (ResourceType::SwarmService, PermissionLevel::Write, None) => Some(
            state
                .identity
                .require_resource::<citadel_swarm_services::permissions::WriteSwarmService>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        (
            ResourceType::SwarmService,
            PermissionLevel::Read,
            Some(SpecificPermission::ResourceBindings),
        ) => Some(
            state
                .identity
                .require_resource::<citadel_swarm_services::permissions::ReadSwarmServiceBindings>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        (
            ResourceType::SwarmService,
            PermissionLevel::Write,
            Some(SpecificPermission::ResourceBindings),
        ) => Some(
            state
                .identity
                .require_resource::<citadel_swarm_services::permissions::WriteSwarmServiceBindings>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        _ => None,
    };
    if let Some(result) = result {
        return identity_result(result, headers);
    }
    identity_result(
        state
            .identity
            .authorize_resource(principal, resource_type, resource_id, level, specific)
            .await,
        headers,
    )?;
    Ok(())
}

async fn capabilities(
    state: &ResourcesHttpState,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceCapabilitiesView> {
    let permission = match resource_id {
        Some(id) => {
            state
                .identity
                .permission_for_resource(principal, resource_type, id)
                .await
        }
        None => {
            state
                .identity
                .global_permission(principal, resource_type)
                .await
        }
    }
    .map_err(|error| IdentityHttpError::from_parts(error, headers))?;
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

fn metadata_error(error: ResourceMetadataError) -> IdentityError {
    match error {
        ResourceMetadataError::Validation(message) => {
            crate::request_validation::validation_error(message)
        }
        ResourceMetadataError::NotFound => IdentityError::NotFound,
        ResourceMetadataError::Conflict(message) => IdentityError::Conflict(message),
        ResourceMetadataError::Credential => IdentityError::Credential,
        ResourceMetadataError::Storage(message) => IdentityError::Storage(message),
    }
}

fn publish_resource_change(
    state: &ResourcesHttpState,
    resource_type: &'static str,
    event_kind: &'static str,
) {
    if let Some(realtime) = &state.realtime {
        // Do not expose an identifier to Actors who may only have access to a
        // different resource of the same type. The client performs an
        // authoritative, permission-filtered read after this invalidation.
        realtime.publish_resource_change(resource_type, Uuid::nil(), event_kind);
    }
}
