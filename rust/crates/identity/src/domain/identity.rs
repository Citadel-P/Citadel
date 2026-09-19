use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use citadel_domain::{
    ActorId, AuthenticatedPrincipalType, PermissionLevel, ResourceType, RoleType,
    SpecificPermission, UserDateTimeFormat, UserTheme,
};

pub const SYSTEM_ACTOR_ID: Uuid = Uuid::from_u128(1);
pub const ADMIN_ROLE_ID: Uuid = Uuid::from_u128(0x30000000000000000000000000000001);
pub const MAX_NAME_CHARS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolePermission {
    resource_type: ResourceType,
    permission_level: PermissionLevel,
    specific_permissions: Vec<SpecificPermission>,
}

impl RolePermission {
    #[must_use]
    pub fn new(
        resource_type: ResourceType,
        permission_level: PermissionLevel,
        specific_permissions: Vec<SpecificPermission>,
    ) -> Self {
        Self {
            resource_type,
            permission_level,
            specific_permissions,
        }
    }

    #[must_use]
    pub const fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    #[must_use]
    pub const fn permission_level(&self) -> PermissionLevel {
        self.permission_level
    }

    #[must_use]
    pub fn specific_permissions(&self) -> &[SpecificPermission] {
        &self.specific_permissions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    id: Uuid,
    name: String,
    role_type: RoleType,
    permissions: Vec<RolePermission>,
}

impl Role {
    #[must_use]
    pub fn new_custom(name: String, permissions: Vec<RolePermission>) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            role_type: RoleType::Custom,
            permissions,
        }
    }

    #[must_use]
    pub const fn from_persistence(
        id: Uuid,
        name: String,
        role_type: RoleType,
        permissions: Vec<RolePermission>,
    ) -> Self {
        Self {
            id,
            name,
            role_type,
            permissions,
        }
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn role_type(&self) -> RoleType {
        self.role_type
    }

    #[must_use]
    pub fn permissions(&self) -> &[RolePermission] {
        &self.permissions
    }

    pub fn rename(&mut self, name: String) -> Result<(), RoleMutationError> {
        if self.role_type == RoleType::System {
            return Err(RoleMutationError::SystemRole);
        }
        self.name = name;
        Ok(())
    }

    pub fn set_permissions(
        &mut self,
        permissions: Vec<RolePermission>,
    ) -> Result<(), RoleMutationError> {
        if self.role_type == RoleType::System {
            return Err(RoleMutationError::SystemRole);
        }
        self.permissions = permissions;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleMutationError {
    SystemRole,
}

impl std::fmt::Display for RoleMutationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("System roles cannot be updated.")
    }
}

impl std::error::Error for RoleMutationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    id: Uuid,
    name: String,
    actor_id: ActorId,
}

impl Team {
    #[must_use]
    pub fn new(name: String, actor_id: ActorId) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            actor_id,
        }
    }

    #[must_use]
    pub const fn from_persistence(id: Uuid, name: String, actor_id: ActorId) -> Self {
        Self { id, name, actor_id }
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn actor_id(&self) -> ActorId {
        self.actor_id
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: Uuid,
    name: String,
    email: String,
    password_hash: Option<String>,
    actor_id: ActorId,
    created_by_actor_id: ActorId,
    created_at: DateTime<Utc>,
}

impl User {
    #[must_use]
    pub fn new(
        name: String,
        email: String,
        password_hash: Option<String>,
        actor_id: ActorId,
        created_by_actor_id: ActorId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            email,
            password_hash,
            actor_id,
            created_by_actor_id,
            created_at,
        }
    }

    #[must_use]
    pub fn from_persistence(
        id: Uuid,
        name: String,
        email: String,
        password_hash: Option<String>,
        actor_id: ActorId,
        created_by_actor_id: ActorId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            email,
            password_hash,
            actor_id,
            created_by_actor_id,
            created_at,
        }
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    #[must_use]
    pub fn password_hash(&self) -> Option<&str> {
        self.password_hash.as_deref()
    }

    #[must_use]
    pub const fn actor_id(&self) -> ActorId {
        self.actor_id
    }

    #[must_use]
    pub const fn created_by_actor_id(&self) -> ActorId {
        self.created_by_actor_id
    }

    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_password_hash(&mut self, password_hash: String) {
        self.password_hash = Some(password_hash);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPreferences {
    user_id: Uuid,
    time_zone: String,
    date_time_format: UserDateTimeFormat,
    theme: UserTheme,
    updated_at: DateTime<Utc>,
}

impl UserPreferences {
    #[must_use]
    pub fn new(
        user_id: Uuid,
        time_zone: String,
        date_time_format: UserDateTimeFormat,
        theme: UserTheme,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user_id,
            time_zone,
            date_time_format,
            theme,
            updated_at,
        }
    }

    #[must_use]
    pub const fn user_id(&self) -> Uuid {
        self.user_id
    }

    #[must_use]
    pub fn time_zone(&self) -> &str {
        &self.time_zone
    }

    #[must_use]
    pub const fn date_time_format(&self) -> UserDateTimeFormat {
        self.date_time_format
    }

    #[must_use]
    pub const fn theme(&self) -> UserTheme {
        self.theme
    }

    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn update(
        &mut self,
        time_zone: String,
        date_time_format: UserDateTimeFormat,
        theme: UserTheme,
        updated_at: DateTime<Utc>,
    ) {
        self.time_zone = time_zone;
        self.date_time_format = date_time_format;
        self.theme = theme;
        self.updated_at = updated_at;
    }
}

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
    pub specifics: citadel_domain::SpecificPermissions,
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
    fn user_mutations_preserve_identity_and_audit_state() {
        let actor_id = ActorId::new(Uuid::now_v7());
        let created_by = ActorId::new(Uuid::now_v7());
        let created_at = Utc::now();
        let mut user = User::new(
            "owner".to_owned(),
            "owner@example.test".to_owned(),
            Some("hash-v1".to_owned()),
            actor_id,
            created_by,
            created_at,
        );
        let id = user.id();

        user.rename("new-owner".to_owned());
        user.set_password_hash("hash-v2".to_owned());

        assert_eq!(user.id(), id);
        assert_eq!(user.actor_id(), actor_id);
        assert_eq!(user.created_by_actor_id(), created_by);
        assert_eq!(user.created_at(), created_at);
        assert_eq!(user.name(), "new-owner");
        assert_eq!(user.password_hash(), Some("hash-v2"));
    }

    #[test]
    fn team_rename_preserves_resource_and_actor_identity() {
        let actor_id = ActorId::new(Uuid::now_v7());
        let mut team = Team::new("operations".to_owned(), actor_id);
        let id = team.id();

        team.rename("platform operations".to_owned());

        assert_eq!(team.id(), id);
        assert_eq!(team.actor_id(), actor_id);
        assert_eq!(team.name(), "platform operations");
    }

    #[test]
    fn system_roles_reject_mutation_while_custom_roles_preserve_identity() {
        let permission =
            RolePermission::new(ResourceType::Registry, PermissionLevel::Read, Vec::new());
        let mut custom = Role::new_custom("Operator".to_owned(), vec![permission.clone()]);
        let custom_id = custom.id();
        custom.rename("Custom operator".to_owned()).unwrap();
        custom.set_permissions(Vec::new()).unwrap();
        assert_eq!(custom.id(), custom_id);
        assert_eq!(custom.name(), "Custom operator");
        assert!(custom.permissions().is_empty());

        let mut system = Role::from_persistence(
            ADMIN_ROLE_ID,
            "Admin".to_owned(),
            RoleType::System,
            vec![permission],
        );
        assert_eq!(
            system.rename("Other".to_owned()),
            Err(RoleMutationError::SystemRole)
        );
        assert_eq!(
            system.set_permissions(Vec::new()),
            Err(RoleMutationError::SystemRole)
        );
    }

    #[test]
    fn preference_updates_preserve_user_ownership() {
        let user_id = Uuid::now_v7();
        let created_at = Utc::now();
        let updated_at = created_at + chrono::Duration::minutes(1);
        let mut preferences = UserPreferences::new(
            user_id,
            "UTC".to_owned(),
            UserDateTimeFormat::System,
            UserTheme::System,
            created_at,
        );

        preferences.update(
            "Europe/Paris".to_owned(),
            UserDateTimeFormat::TwentyFourHour,
            UserTheme::Dark,
            updated_at,
        );

        assert_eq!(preferences.user_id(), user_id);
        assert_eq!(preferences.time_zone(), "Europe/Paris");
        assert_eq!(
            preferences.date_time_format(),
            UserDateTimeFormat::TwentyFourHour
        );
        assert_eq!(preferences.theme(), UserTheme::Dark);
        assert_eq!(preferences.updated_at(), updated_at);
    }

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
                specifics: SpecificPermission::Use.into(),
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
