use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub const ADMIN_ROLE_ID: Uuid = Uuid::from_u128(0x30000000000000000000000000000001);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleType {
    System,
    Custom,
}

impl RoleType {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Custom => "Custom",
        }
    }

    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "System" => Some(Self::System),
            "Custom" => Some(Self::Custom),
            _ => None,
        }
    }
}
