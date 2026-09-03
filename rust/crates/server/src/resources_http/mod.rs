mod bindings;
mod catalog;
mod tags;

use std::sync::Arc;

use axum::Router;
use axum::extract::Extension;
use axum::extract::rejection::JsonRejection;
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
        ResourceMetadataError::Validation(message) => IdentityError::Validation(message),
        ResourceMetadataError::NotFound => IdentityError::NotFound,
        ResourceMetadataError::Conflict(message) => IdentityError::Conflict(message),
        ResourceMetadataError::Credential => IdentityError::Credential,
        ResourceMetadataError::Storage(message) => IdentityError::Storage(message),
    }
}

fn invalid_json(_: JsonRejection) -> IdentityError {
    IdentityError::Validation("The request body is not valid JSON for this operation.".to_owned())
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
