use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorType {
    User,
    System,
    Agent,
    ServiceAccount,
    Team,
}

impl ActorType {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::System => "System",
            Self::Agent => "Agent",
            Self::ServiceAccount => "ServiceAccount",
            Self::Team => "Team",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthenticatedPrincipalType {
    User,
    ServiceAccount,
    System,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum ResourceType {
    Platform = 0,
    Deployment = 1,
    Stack = 2,
    Registry = 3,
    GitRepository = 4,
    GitAccount = 5,
    Alert = 6,
    AlertChannel = 7,
    User = 8,
    Team = 9,
    Role = 10,
    Binding = 11,
    Tag = 12,
    AutomationAction = 13,
    License = 14,
    BackupRepository = 15,
    BackupPolicy = 16,
    Volume = 17,
    Build = 18,
    BuildAgentPool = 19,
    SwarmService = 20,
    ServiceAccount = 21,
}

impl ResourceType {
    pub const ALL: [Self; 22] = [
        Self::Platform,
        Self::Deployment,
        Self::Stack,
        Self::Registry,
        Self::GitRepository,
        Self::GitAccount,
        Self::Alert,
        Self::AlertChannel,
        Self::User,
        Self::Team,
        Self::Role,
        Self::Binding,
        Self::Tag,
        Self::AutomationAction,
        Self::License,
        Self::BackupRepository,
        Self::BackupPolicy,
        Self::Volume,
        Self::Build,
        Self::BuildAgentPool,
        Self::SwarmService,
        Self::ServiceAccount,
    ];

    #[must_use]
    pub const fn from_i32(value: i32) -> Option<Self> {
        if value < 0 || value >= Self::ALL.len() as i32 {
            return None;
        }
        Some(Self::ALL[value as usize])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum PermissionLevel {
    None = 0,
    Read = 1,
    Write = 2,
    Execute = 4,
}

impl PermissionLevel {
    #[must_use]
    pub const fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Read),
            2 => Some(Self::Write),
            4 => Some(Self::Execute),
            _ => None,
        }
    }

    #[must_use]
    pub const fn grants(self, required: Self) -> bool {
        self as i32 >= required as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum SpecificPermission {
    Logs = 1 << 0,
    Inspect = 1 << 1,
    Apply = 1 << 2,
    Pull = 1 << 3,
    Terminal = 1 << 4,
    ResourceBindings = 1 << 5,
    Releases = 1 << 6,
    Restore = 1 << 7,
    Browse = 1 << 8,
    Download = 1 << 9,
    ManageNodeAgents = 1 << 10,
    Use = 1 << 11,
    ManageCredentials = 1 << 12,
}

impl SpecificPermission {
    pub const ALL: [Self; 13] = [
        Self::Logs,
        Self::Inspect,
        Self::Apply,
        Self::Pull,
        Self::Terminal,
        Self::ResourceBindings,
        Self::Releases,
        Self::Restore,
        Self::Browse,
        Self::Download,
        Self::ManageNodeAgents,
        Self::Use,
        Self::ManageCredentials,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleType {
    System,
    Custom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_values_match_the_accepted_database_contract() {
        assert_eq!(ResourceType::Platform as i32, 0);
        assert_eq!(ResourceType::ServiceAccount as i32, 21);
        for (index, resource) in ResourceType::ALL.into_iter().enumerate() {
            assert_eq!(ResourceType::from_i32(index as i32), Some(resource));
        }
        assert_eq!(ResourceType::from_i32(22), None);
    }
}
