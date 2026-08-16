use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    ActorId, AuthenticatedPrincipalType, PermissionLevel, ResourceType, SpecificPermission,
};

pub const SYSTEM_ACTOR_ID: Uuid = Uuid::from_u128(1);
pub const ADMIN_ROLE_ID: Uuid = Uuid::from_u128(0x30000000000000000000000000000001);
pub const MAX_NAME_CHARS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorPrincipal {
    pub subject_id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub principal_type: AuthenticatedPrincipalType,
    pub credential_id: Option<Uuid>,
    pub roles: Vec<String>,
}

impl ActorPrincipal {
    #[must_use]
    pub fn is_human(&self) -> bool {
        self.principal_type == AuthenticatedPrincipalType::User
    }

    #[must_use]
    pub fn is_administrator(&self) -> bool {
        self.is_human() && self.roles.iter().any(|role| role == "Admin")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionGrant {
    pub resource_type: ResourceType,
    pub level: PermissionLevel,
    pub specific_mask: i32,
}

impl PermissionGrant {
    #[must_use]
    pub const fn has_specific(self, permission: SpecificPermission) -> bool {
        self.specific_mask & permission as i32 != 0
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAuthentication {
    pub user_id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub enabled: bool,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceAccountCredential {
    pub credential_id: Uuid,
    pub service_account_id: Uuid,
    pub actor_id: ActorId,
    pub name: String,
    pub secret_hash: [u8; 32],
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub roles: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_account_sensitive_permissions_are_explicit() {
        let matrix = permission_matrix();
        let capabilities = &matrix[&ResourceType::ServiceAccount];
        assert_eq!(capabilities.specifics, SERVICE_ACCOUNT);
        assert!(
            !matrix[&ResourceType::User]
                .specifics
                .iter()
                .any(|(permission, _)| *permission == SpecificPermission::ManageCredentials)
        );
    }

    #[test]
    fn authorization_requires_level_and_specific_bit() {
        let actor = ActorId::new(Uuid::now_v7());
        let snapshot = AuthorizationSnapshot {
            actor_id: actor,
            enabled: true,
            direct_and_team_permissions: vec![PermissionGrant {
                resource_type: ResourceType::ServiceAccount,
                level: PermissionLevel::Read,
                specific_mask: SpecificPermission::Use as i32,
            }],
        };
        assert!(snapshot.permits(
            ResourceType::ServiceAccount,
            PermissionLevel::Read,
            Some(SpecificPermission::Use)
        ));
        assert!(!snapshot.permits(
            ResourceType::ServiceAccount,
            PermissionLevel::Read,
            Some(SpecificPermission::ManageCredentials)
        ));
        assert!(!snapshot.permits(ResourceType::ServiceAccount, PermissionLevel::Write, None));
    }
}
