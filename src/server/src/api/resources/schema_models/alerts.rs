enum_schema!(
    AlertEventStatusSchema,
    "AlertEventStatus",
    citadel_alerts::AlertEventStatus,
    [Active, Acknowledged, Resolved]
);

enum_schema!(
    AlertRuleStatusSchema,
    "AlertRuleStatus",
    citadel_alerts::AlertRuleStatus,
    [Enabled, Disabled]
);

enum_schema!(
    AlertSeveritySchema,
    "AlertSeverity",
    citadel_alerts::AlertSeverity,
    [Info, Warning, Critical]
);
enum_schema!(
    AlertTypeSchema,
    "AlertType",
    citadel_alerts::AlertType,
    [
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
        LicenseEnteredGracePeriod,
        LicenseExpired,
    ]
);
