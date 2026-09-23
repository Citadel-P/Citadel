use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionGrant {
    pub resource_type: ResourceType,
    pub level: PermissionLevel,
    pub specifics: citadel_primitives::SpecificPermissions,
}

impl PermissionGrant {
    #[must_use]
    pub const fn has_specific(self, permission: SpecificPermission) -> bool {
        self.specifics.contains(permission)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationSnapshot {
    pub actor_id: ActorId,
    pub enabled: bool,
    pub direct_and_team_permissions: Vec<PermissionGrant>,
}

impl AuthorizationSnapshot {
    #[must_use]
    pub fn permits(
        &self,
        resource_type: ResourceType,
        level: PermissionLevel,
        specific: Option<SpecificPermission>,
    ) -> bool {
        self.enabled
            && self.direct_and_team_permissions.iter().any(|grant| {
                grant.resource_type == resource_type
                    && grant.level.grants(level)
                    && specific.is_none_or(|permission| grant.has_specific(permission))
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionCapability {
    pub maximum_level: PermissionLevel,
    pub specifics: &'static [(SpecificPermission, PermissionLevel)],
}

const NONE: &[(SpecificPermission, PermissionLevel)] = &[];

const SERVICE_ACCOUNT: &[(SpecificPermission, PermissionLevel)] = &[
    (SpecificPermission::Use, PermissionLevel::Read),
    (SpecificPermission::ManageCredentials, PermissionLevel::Read),
];

const PLATFORM: &[(SpecificPermission, PermissionLevel)] = &[
    (SpecificPermission::Logs, PermissionLevel::Read),
    (SpecificPermission::Pull, PermissionLevel::Read),
    (SpecificPermission::Terminal, PermissionLevel::Read),
    (SpecificPermission::Inspect, PermissionLevel::Read),
    (
        SpecificPermission::ManageNodeAgents,
        PermissionLevel::Execute,
    ),
];

const DEPLOYMENT: &[(SpecificPermission, PermissionLevel)] = &[
    (SpecificPermission::Apply, PermissionLevel::Read),
    (SpecificPermission::ResourceBindings, PermissionLevel::Read),
    (SpecificPermission::Logs, PermissionLevel::Read),
    (SpecificPermission::Terminal, PermissionLevel::Read),
    (SpecificPermission::Inspect, PermissionLevel::Read),
];

const SWARM_SERVICE: &[(SpecificPermission, PermissionLevel)] = &[
    (SpecificPermission::Apply, PermissionLevel::Read),
    (SpecificPermission::ResourceBindings, PermissionLevel::Read),
    (SpecificPermission::Logs, PermissionLevel::Read),
    (SpecificPermission::Inspect, PermissionLevel::Read),
];

const STACK: &[(SpecificPermission, PermissionLevel)] = &[
    (SpecificPermission::Apply, PermissionLevel::Read),
    (SpecificPermission::ResourceBindings, PermissionLevel::Read),
    (SpecificPermission::Releases, PermissionLevel::Read),
    (SpecificPermission::Logs, PermissionLevel::Read),
    (SpecificPermission::Inspect, PermissionLevel::Read),
];

const BACKUP_POLICY: &[(SpecificPermission, PermissionLevel)] =
    &[(SpecificPermission::Restore, PermissionLevel::Read)];

const BUILD: &[(SpecificPermission, PermissionLevel)] =
    &[(SpecificPermission::Apply, PermissionLevel::Read)];

const VOLUME: &[(SpecificPermission, PermissionLevel)] = &[
    (SpecificPermission::Browse, PermissionLevel::Read),
    (SpecificPermission::Download, PermissionLevel::Read),
];

#[must_use]
pub fn permission_matrix() -> BTreeMap<ResourceType, PermissionCapability> {
    ResourceType::ALL
        .into_iter()
        .map(|resource| {
            let specifics = match resource {
                ResourceType::ServiceAccount => SERVICE_ACCOUNT,
                ResourceType::Platform => PLATFORM,
                ResourceType::Deployment => DEPLOYMENT,
                ResourceType::SwarmService => SWARM_SERVICE,
                ResourceType::Stack => STACK,
                ResourceType::BackupPolicy => BACKUP_POLICY,
                ResourceType::Build | ResourceType::BuildAgentPool => BUILD,
                ResourceType::Volume => VOLUME,
                _ => NONE,
            };
            (
                resource,
                PermissionCapability {
                    maximum_level: PermissionLevel::Execute,
                    specifics,
                },
            )
        })
        .collect()
}
