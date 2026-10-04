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
    AutomationActionRunFailed,
    BuildRunFailed,
    BuildAgentPoolUnavailable,
    LicenseEnteredGracePeriod,
    LicenseExpired,
    }
}

impl AlertType {
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

#[cfg(test)]
mod tests {
    use super::*;

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
