use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SetupInitializationMode {
    Interactive,
    Unattended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LookupResourceType {
    Platform,
    Deployment,
    Stack,
    Image,
    Network,
    Volume,
    Registry,
    GitRepository,
    GitAccount,
    OidcProvider,
    AutomationAction,
    Alert,
    AlertChannel,
    User,
    UserActor,
    Team,
    Role,
    ResourceBinding,
    License,
    BackupRepository,
    BackupPolicy,
    Build,
    BuildAgentPool,
    SwarmService,
    RunAsActor,
    ServiceAccount,
}

impl LookupResourceType {
    pub const ALL: &'static [Self] = &[
        Self::Platform,
        Self::Deployment,
        Self::Stack,
        Self::Image,
        Self::Network,
        Self::Volume,
        Self::Registry,
        Self::GitRepository,
        Self::GitAccount,
        Self::OidcProvider,
        Self::AutomationAction,
        Self::Alert,
        Self::AlertChannel,
        Self::User,
        Self::UserActor,
        Self::Team,
        Self::Role,
        Self::ResourceBinding,
        Self::License,
        Self::BackupRepository,
        Self::BackupPolicy,
        Self::Build,
        Self::BuildAgentPool,
        Self::SwarmService,
        Self::RunAsActor,
        Self::ServiceAccount,
    ];
}

macro_rules! database_string_enum {
    ($(#[$metadata:meta])* pub enum $name:ident { $($(#[$variant_metadata:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$metadata])*
        pub enum $name {
            $($(#[$variant_metadata])* $variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn as_database_str(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant)),+
                }
            }

            #[must_use]
            pub fn from_database_str(value: &str) -> Option<Self> {
                match value {
                    $(stringify!($variant) => Some(Self::$variant)),+,
                    _ => None,
                }
            }
        }
    };
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum ActivityStatus {
        Success,
        Failure,
        Warning,
        Information,
    }
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub enum LicenseCapability {
        CustomAccessControl,
        AutomatedOperations,
        AdvancedAlerting,
        OperationalGuardrails,
        ElasticBuildExecution,
    }
}

impl LicenseCapability {
    #[must_use]
    pub const fn as_license_key(self) -> &'static str {
        match self {
            Self::CustomAccessControl => "custom-access-control",
            Self::AutomatedOperations => "automated-operations",
            Self::AdvancedAlerting => "advanced-alerting",
            Self::OperationalGuardrails => "operational-guardrails",
            Self::ElasticBuildExecution => "elastic-build-execution",
        }
    }

    #[must_use]
    pub fn from_license_key(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|capability| capability.as_license_key() == value)
    }
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum LicenseStatus {
        Community,
        Valid,
        GracePeriod,
        NotYetValid,
        Expired,
        Invalid,
        InstanceMismatch,
        UnsupportedSchema,
        UnknownSigningKey,
    }
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum MfaPolicy {
        #[default]
        Optional,
        RequiredForAdministrators,
        RequiredForAllUsers,
    }
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum ActivityResourceType {
        Platform,
        Registry,
        Deployment,
        Stack,
        AlertRule,
        GitRepository,
        OidcProvider,
        AutomationAction,
        User,
        Team,
        Role,
        License,
        Build,
        BuildAgentPool,
        Volume,
        BackupPolicy,
        SwarmService,
        ServiceAccount,
    }
}

database_string_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum ActivityEventType {
        DeploymentCreated,
        DeploymentDuplicated,
        DeploymentUpdated,
        DeploymentRenamed,
        DeploymentDeleted,
        DeploymentStarted,
        DeploymentStopped,
        DeploymentPaused,
        DeploymentApplied,
        DeploymentDegraded,
        DeploymentAdopted,
        PlatformCreated,
        PlatformDeleted,
        PlatformConnected,
        PlatformDisconnected,
        PlatformRenamed,
        PlatformNodeAgentLifecycle,
        RegistryCreated,
        RegistryRenamed,
        RegistryUpdated,
        RegistryDeleted,
        AlertRuleCreated,
        AlertRuleUpdated,
        AlertRuleDeleted,
        AlertRuleRenamed,
        GitRepoCreated,
        GitRepoUpdated,
        GitRepoDeleted,
        GitRepoRenamed,
        GitRepoPulled,
        GitRepoCloned,
        GitRepoWebhookReceived,
        OidcProviderCreated,
        OidcProviderUpdated,
        OidcProviderRenamed,
        OidcProviderDeleted,
        ActionCreated,
        ActionUpdated,
        ActionRenamed,
        ActionDeleted,
        ActionRunQueued,
        ActionRunStarted,
        ActionRunSucceeded,
        ActionRunFailed,
        ActionRunTimedOut,
        ActionRunCancelled,
        ActionRunRejected,
        StackCreated,
        StackDuplicated,
        StackUpdated,
        StackRenamed,
        StackDeleted,
        StackStarted,
        StackStopped,
        StackPaused,
        StackApplied,
        StackRollback,
        StackDegraded,
        StackDriftDetected,
        StackDriftResolved,
        StackReconciliationAttempted,
        StackGitUpdateAvailable,
        StackGitAutoUpdated,
        StackGitAutoDeployFailed,
        StackImported,
        StackWebhookReceived,
        InitialAdministratorCreated,
        UserProfileUpdated,
        UserPreferencesUpdated,
        UserPasswordChanged,
        UserSessionRevoked,
        UserOtherSessionsRevoked,
        UserMfaEnabled,
        UserMfaDisabled,
        UserMfaVerificationFailed,
        UserMfaRecoveryCodeUsed,
        UserMfaRecoveryCodesRegenerated,
        UserMfaResetByAdministrator,
        UserCreated,
        UserUpdated,
        UserRenamed,
        UserDeleted,
        TeamCreated,
        TeamUpdated,
        TeamRenamed,
        TeamDeleted,
        RoleCreated,
        RoleUpdated,
        RoleRenamed,
        RoleDeleted,
        LicenseInstalled,
        LicenseReplaced,
        LicenseRemoved,
        LicenseEnteredGracePeriod,
        LicenseExpired,
        LicenseValidationFailed,
        VolumeContentDownloaded,
        BuildCreated,
        BuildUpdated,
        BuildRenamed,
        BuildDeleted,
        BuildRunQueued,
        BuildRunStarted,
        BuildRunSucceeded,
        BuildRunFailed,
        BuildRunTimedOut,
        BuildRunCancelled,
        BuildWebhookReceived,
        BuildAgentPoolCreated,
        BuildAgentPoolUpdated,
        BuildAgentPoolRenamed,
        BuildAgentPoolDeleted,
        BuildAgentPoolTested,
        BackupPolicyCreated,
        BackupPolicyUpdated,
        BackupPolicyRenamed,
        BackupPolicyArchived,
        SwarmServiceCreated,
        SwarmServiceAdopted,
        SwarmServiceUpdated,
        SwarmServiceRenamed,
        SwarmServiceDeleted,
        SwarmServiceApplied,
        SwarmServiceScaled,
        SwarmServiceForceUpdated,
        SwarmServiceOperationFailed,
        SwarmServiceDuplicated,
        SwarmServiceWebhookReceived,
        ServiceAccountCreated,
        ServiceAccountUpdated,
        ServiceAccountRenamed,
        ServiceAccountEnabled,
        ServiceAccountDisabled,
        ServiceAccountArchived,
        ServiceAccountTokenCreated,
        ServiceAccountTokenRevoked,
    }
}

impl ActivityEventType {
    #[must_use]
    pub const fn resource_type(self) -> ActivityResourceType {
        match self {
            Self::DeploymentCreated
            | Self::DeploymentDuplicated
            | Self::DeploymentUpdated
            | Self::DeploymentRenamed
            | Self::DeploymentDeleted
            | Self::DeploymentStarted
            | Self::DeploymentStopped
            | Self::DeploymentPaused
            | Self::DeploymentApplied
            | Self::DeploymentDegraded
            | Self::DeploymentAdopted => ActivityResourceType::Deployment,
            Self::PlatformCreated
            | Self::PlatformDeleted
            | Self::PlatformConnected
            | Self::PlatformDisconnected
            | Self::PlatformRenamed
            | Self::PlatformNodeAgentLifecycle => ActivityResourceType::Platform,
            Self::RegistryCreated
            | Self::RegistryRenamed
            | Self::RegistryUpdated
            | Self::RegistryDeleted => ActivityResourceType::Registry,
            Self::AlertRuleCreated
            | Self::AlertRuleUpdated
            | Self::AlertRuleDeleted
            | Self::AlertRuleRenamed => ActivityResourceType::AlertRule,
            Self::GitRepoCreated
            | Self::GitRepoUpdated
            | Self::GitRepoDeleted
            | Self::GitRepoRenamed
            | Self::GitRepoPulled
            | Self::GitRepoCloned
            | Self::GitRepoWebhookReceived => ActivityResourceType::GitRepository,
            Self::OidcProviderCreated
            | Self::OidcProviderUpdated
            | Self::OidcProviderRenamed
            | Self::OidcProviderDeleted => ActivityResourceType::OidcProvider,
            Self::ActionCreated
            | Self::ActionUpdated
            | Self::ActionRenamed
            | Self::ActionDeleted
            | Self::ActionRunQueued
            | Self::ActionRunStarted
            | Self::ActionRunSucceeded
            | Self::ActionRunFailed
            | Self::ActionRunTimedOut
            | Self::ActionRunCancelled
            | Self::ActionRunRejected => ActivityResourceType::AutomationAction,
            Self::StackCreated
            | Self::StackDuplicated
            | Self::StackUpdated
            | Self::StackRenamed
            | Self::StackDeleted
            | Self::StackStarted
            | Self::StackStopped
            | Self::StackPaused
            | Self::StackApplied
            | Self::StackRollback
            | Self::StackDegraded
            | Self::StackDriftDetected
            | Self::StackDriftResolved
            | Self::StackReconciliationAttempted
            | Self::StackGitUpdateAvailable
            | Self::StackGitAutoUpdated
            | Self::StackGitAutoDeployFailed
            | Self::StackImported
            | Self::StackWebhookReceived => ActivityResourceType::Stack,
            Self::InitialAdministratorCreated
            | Self::UserProfileUpdated
            | Self::UserPreferencesUpdated
            | Self::UserPasswordChanged
            | Self::UserSessionRevoked
            | Self::UserOtherSessionsRevoked
            | Self::UserMfaEnabled
            | Self::UserMfaDisabled
            | Self::UserMfaVerificationFailed
            | Self::UserMfaRecoveryCodeUsed
            | Self::UserMfaRecoveryCodesRegenerated
            | Self::UserMfaResetByAdministrator
            | Self::UserCreated
            | Self::UserUpdated
            | Self::UserRenamed
            | Self::UserDeleted => ActivityResourceType::User,
            Self::TeamCreated | Self::TeamUpdated | Self::TeamRenamed | Self::TeamDeleted => {
                ActivityResourceType::Team
            }
            Self::RoleCreated | Self::RoleUpdated | Self::RoleRenamed | Self::RoleDeleted => {
                ActivityResourceType::Role
            }
            Self::LicenseInstalled
            | Self::LicenseReplaced
            | Self::LicenseRemoved
            | Self::LicenseEnteredGracePeriod
            | Self::LicenseExpired
            | Self::LicenseValidationFailed => ActivityResourceType::License,
            Self::VolumeContentDownloaded => ActivityResourceType::Volume,
            Self::BuildCreated
            | Self::BuildUpdated
            | Self::BuildRenamed
            | Self::BuildDeleted
            | Self::BuildRunQueued
            | Self::BuildRunStarted
            | Self::BuildRunSucceeded
            | Self::BuildRunFailed
            | Self::BuildRunTimedOut
            | Self::BuildRunCancelled
            | Self::BuildWebhookReceived => ActivityResourceType::Build,
            Self::BuildAgentPoolCreated
            | Self::BuildAgentPoolUpdated
            | Self::BuildAgentPoolRenamed
            | Self::BuildAgentPoolDeleted
            | Self::BuildAgentPoolTested => ActivityResourceType::BuildAgentPool,
            Self::BackupPolicyCreated
            | Self::BackupPolicyUpdated
            | Self::BackupPolicyRenamed
            | Self::BackupPolicyArchived => ActivityResourceType::BackupPolicy,
            Self::SwarmServiceCreated
            | Self::SwarmServiceAdopted
            | Self::SwarmServiceUpdated
            | Self::SwarmServiceRenamed
            | Self::SwarmServiceDeleted
            | Self::SwarmServiceApplied
            | Self::SwarmServiceScaled
            | Self::SwarmServiceForceUpdated
            | Self::SwarmServiceOperationFailed
            | Self::SwarmServiceDuplicated
            | Self::SwarmServiceWebhookReceived => ActivityResourceType::SwarmService,
            Self::ServiceAccountCreated
            | Self::ServiceAccountUpdated
            | Self::ServiceAccountRenamed
            | Self::ServiceAccountEnabled
            | Self::ServiceAccountDisabled
            | Self::ServiceAccountArchived
            | Self::ServiceAccountTokenCreated
            | Self::ServiceAccountTokenRevoked => ActivityResourceType::ServiceAccount,
        }
    }
}

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

    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "User" => Some(Self::User),
            "System" => Some(Self::System),
            "Agent" => Some(Self::Agent),
            "ServiceAccount" => Some(Self::ServiceAccount),
            "Team" => Some(Self::Team),
            _ => None,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmServiceOwnership {
    #[default]
    Unmanaged,
    DockerStackExternal,
    CitadelService,
    CitadelStack,
    OwnershipConflict,
    System,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserDateTimeFormat {
    System,
    TwentyFourHour,
    TwelveHour,
}

impl UserDateTimeFormat {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::TwentyFourHour => "TwentyFourHour",
            Self::TwelveHour => "TwelveHour",
        }
    }

    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "System" => Some(Self::System),
            "TwentyFourHour" => Some(Self::TwentyFourHour),
            "TwelveHour" => Some(Self::TwelveHour),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserTheme {
    System,
    Light,
    Dark,
}

impl UserTheme {
    #[must_use]
    pub const fn as_database_str(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    #[must_use]
    pub fn from_database_str(value: &str) -> Option<Self> {
        match value {
            "System" => Some(Self::System),
            "Light" => Some(Self::Light),
            "Dark" => Some(Self::Dark),
            _ => None,
        }
    }
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

    #[test]
    fn preference_values_match_the_existing_database_contract() {
        assert_eq!(
            UserDateTimeFormat::from_database_str("TwentyFourHour"),
            Some(UserDateTimeFormat::TwentyFourHour)
        );
        assert_eq!(UserTheme::Dark.as_database_str(), "Dark");
        assert_eq!(UserTheme::from_database_str("dark"), None);
    }
}
