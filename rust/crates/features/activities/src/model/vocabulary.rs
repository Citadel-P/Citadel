use serde::{Deserialize, Serialize};
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
