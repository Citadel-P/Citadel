//! Known API alert kinds. Internal observations may carry other names.
citadel_primitives::status_enum! {
    pub enum AlertType {
PlatformCpuHigh,
    PlatformRamHigh,
    PlatformDiskHigh,
    PlatformUnreachable,
    PlatformVersionMismatch,
    UnmanagedContainerCreated,
    DeploymentImageUpdateAvailable,
    DeploymentAutoDeployFailed,
    DeploymentAutoUpdated,
    SwarmServiceOperationFailed,
    StackImageUpdateAvailable,
    StackAutoDeployFailed,
    StackAutoUpdated,
    StackServiceAutoDeployFailed,
    StackServiceAutoUpdated,
    StackDriftDetected,
    StackDriftAutoReconciled,
    StackGitUpdateAvailable,
    StackGitAutoUpdated,
    StackGitAutoDeployFailed,
    StackConfigurationResolutionFailed,
    DeploymentConfigurationResolutionFailed,
    WebhookAuthenticationFailed,
    WebhookDispatchFailed,
    WebhookGitRepoSyncFailed,
    WebhookStackGitDeployFailed,
    ScriptRunFailed,
    BuildRunFailed,
    BackupRunFailed,
    BuildAgentPoolUnavailable,
    LicenseEnteredGracePeriod,
    LicenseExpired,
    CoreUpdateAvailable,
    AgentUpdatesAvailable,
    }
}

impl AlertType {
    /// The resource family this rule observes, independent of its name or scope.
    pub const fn resource_type(self) -> AlertResourceType {
        match self {
            Self::PlatformCpuHigh
            | Self::PlatformRamHigh
            | Self::PlatformDiskHigh
            | Self::PlatformUnreachable
            | Self::PlatformVersionMismatch
            | Self::UnmanagedContainerCreated => AlertResourceType::Platform,
            Self::DeploymentImageUpdateAvailable
            | Self::DeploymentAutoDeployFailed
            | Self::DeploymentAutoUpdated
            | Self::DeploymentConfigurationResolutionFailed => AlertResourceType::Deployment,
            Self::SwarmServiceOperationFailed => AlertResourceType::SwarmService,
            Self::StackImageUpdateAvailable
            | Self::StackAutoDeployFailed
            | Self::StackAutoUpdated
            | Self::StackServiceAutoDeployFailed
            | Self::StackServiceAutoUpdated
            | Self::StackDriftDetected
            | Self::StackDriftAutoReconciled
            | Self::StackGitUpdateAvailable
            | Self::StackGitAutoUpdated
            | Self::StackGitAutoDeployFailed
            | Self::StackConfigurationResolutionFailed => AlertResourceType::Stack,
            Self::WebhookAuthenticationFailed
            | Self::WebhookDispatchFailed
            | Self::WebhookGitRepoSyncFailed
            | Self::WebhookStackGitDeployFailed => AlertResourceType::Webhook,
            Self::ScriptRunFailed => AlertResourceType::Script,
            Self::BuildRunFailed => AlertResourceType::Build,
            Self::BuildAgentPoolUnavailable => AlertResourceType::BuildAgentPool,
            Self::BackupRunFailed => AlertResourceType::BackupPolicy,
            Self::LicenseEnteredGracePeriod | Self::LicenseExpired => AlertResourceType::License,
            Self::CoreUpdateAvailable | Self::AgentUpdatesAvailable => AlertResourceType::Updates,
        }
    }

    pub const fn is_threshold(self) -> bool {
        matches!(
            self,
            Self::PlatformCpuHigh | Self::PlatformRamHigh | Self::PlatformDiskHigh
        )
    }
}

citadel_primitives::status_enum! {
    pub enum AlertSeverity { Info, Warning, Critical }
}

citadel_primitives::status_enum! {
    pub enum AlertResourceType {
        Platform, Deployment, Stack, GitRepository, Webhook, Script, Build,
        BuildAgentPool, BackupPolicy, License, Updates, SwarmService
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_resource_alerts_belong_to_their_observed_resource() {
        for (kind, resource) in [
            (
                AlertType::BuildAgentPoolUnavailable,
                AlertResourceType::BuildAgentPool,
            ),
            (AlertType::BuildRunFailed, AlertResourceType::Build),
            (AlertType::BackupRunFailed, AlertResourceType::BackupPolicy),
            (
                AlertType::StackServiceAutoDeployFailed,
                AlertResourceType::Stack,
            ),
            (
                AlertType::StackGitAutoDeployFailed,
                AlertResourceType::Stack,
            ),
            (
                AlertType::WebhookStackGitDeployFailed,
                AlertResourceType::Webhook,
            ),
            (
                AlertType::UnmanagedContainerCreated,
                AlertResourceType::Platform,
            ),
            (AlertType::AgentUpdatesAvailable, AlertResourceType::Updates),
            (
                AlertType::SwarmServiceOperationFailed,
                AlertResourceType::SwarmService,
            ),
        ] {
            assert_eq!(kind.resource_type(), resource);
        }
    }

    #[test]
    fn known_alert_kinds_and_severities_roundtrip_exactly() {
        for kind in AlertType::ALL {
            assert_eq!(kind.as_str().parse::<AlertType>().unwrap(), *kind);
            assert_eq!(serde_json::to_value(kind).unwrap(), kind.as_str());
        }
        for severity in AlertSeverity::ALL {
            assert_eq!(
                severity.as_str().parse::<AlertSeverity>().unwrap(),
                *severity
            );
            assert_eq!(serde_json::to_value(severity).unwrap(), severity.as_str());
        }
        for invalid in ["Information", "warning", "Critical ", ""] {
            assert!(invalid.parse::<AlertSeverity>().is_err());
        }
        assert!("Verification".parse::<AlertType>().is_err());
        let thresholds: Vec<_> = AlertType::ALL
            .iter()
            .copied()
            .filter(|kind| kind.is_threshold())
            .collect();
        assert_eq!(
            thresholds,
            [
                AlertType::PlatformCpuHigh,
                AlertType::PlatformRamHigh,
                AlertType::PlatformDiskHigh
            ]
        );
    }
}
