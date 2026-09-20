use crate::identity_http::{IdentityHttpError, IdentityHttpResult, identity_result};
use crate::platforms_http::views::ResourceCapabilitiesView;
use crate::realtime::RealtimeHub;
use axum::extract::Extension;
use axum::http::HeaderMap;
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use uuid::Uuid;
pub(crate) fn require_actor(
    principal: Option<Extension<ActorPrincipal>>,
) -> Result<ActorPrincipal, IdentityError> {
    principal
        .map(|Extension(value)| value)
        .ok_or(IdentityError::Unauthenticated)
}
pub(crate) async fn authorize_global(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        match (resource_type, level) {
            (ResourceType::GitRepository, PermissionLevel::Read) => {
                identity
                    .require_scope::<citadel_git::permissions::ReadGitRepository>(principal)
                    .await
            }
            (ResourceType::GitRepository, PermissionLevel::Write) => {
                identity
                    .require_scope::<citadel_git::permissions::WriteGitRepository>(principal)
                    .await
            }
            (ResourceType::GitRepository, PermissionLevel::Execute) => {
                identity
                    .require_scope::<citadel_git::permissions::ExecuteGitRepository>(principal)
                    .await
            }
            _ => {
                identity
                    .authorize(principal, resource_type, level, None)
                    .await
            }
        },
        headers,
    )?;
    Ok(())
}
pub(crate) async fn authorize_resource(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    resource_id: Uuid,
    level: PermissionLevel,
    specific: Option<SpecificPermission>,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    let id = resource_id;
    if resource_type == ResourceType::GitRepository && specific.is_none() {
        return identity_result(
            match (resource_type, level) {
                (ResourceType::GitRepository, PermissionLevel::Read) => {
                    identity
                        .require_resource::<citadel_git::permissions::ReadGitRepository>(
                            principal, id,
                        )
                        .await
                }
                (ResourceType::GitRepository, PermissionLevel::Write) => {
                    identity
                        .require_resource::<citadel_git::permissions::WriteGitRepository>(
                            principal, id,
                        )
                        .await
                }
                (ResourceType::GitRepository, PermissionLevel::Execute) => {
                    identity
                        .require_resource::<citadel_git::permissions::ExecuteGitRepository>(
                            principal, id,
                        )
                        .await
                }
                _ => {
                    identity
                        .authorize_resource(principal, resource_type, id, level, None)
                        .await
                }
            },
            headers,
        );
    }
    // Workload metadata uses the owning feature's named policies.
    let result = match (resource_type, level, specific) {
        (ResourceType::Stack, PermissionLevel::Read, None) => Some(
            identity
                .require_resource::<citadel_stacks::permissions::ReadStack>(principal, resource_id)
                .await,
        ),
        (ResourceType::Stack, PermissionLevel::Write, None) => Some(
            identity
                .require_resource::<citadel_stacks::permissions::WriteStack>(principal, resource_id)
                .await,
        ),
        (
            ResourceType::Stack,
            PermissionLevel::Read,
            Some(SpecificPermission::ResourceBindings),
        ) => Some(
            identity
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
            identity
                .require_resource::<citadel_stacks::permissions::WriteStackBindings>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        (ResourceType::SwarmService, PermissionLevel::Read, None) => Some(
            identity
                .require_resource::<citadel_swarm_services::permissions::ReadSwarmService>(
                    principal,
                    resource_id,
                )
                .await,
        ),
        (ResourceType::SwarmService, PermissionLevel::Write, None) => Some(
            identity
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
            identity
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
            identity
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
        identity
            .authorize_resource(principal, resource_type, resource_id, level, specific)
            .await,
        headers,
    )?;
    Ok(())
}
pub(crate) async fn capabilities(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    resource_id: Option<Uuid>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ResourceCapabilitiesView> {
    let permission = match resource_id {
        Some(id) => {
            identity
                .permission_for_resource(principal, resource_type, id)
                .await
        }
        None => identity.global_permission(principal, resource_type).await,
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
pub(crate) fn publish_resource_change(
    hub: &Option<RealtimeHub>,
    resource_type: &'static str,
    event_kind: &'static str,
) {
    if let Some(realtime) = hub {
        // Do not expose an identifier to Actors who may only have access to a
        // different resource of the same type. The client performs an
        // authoritative, permission-filtered read after this invalidation.
        realtime.publish_resource_change(resource_type, Uuid::nil(), event_kind);
    }
}
