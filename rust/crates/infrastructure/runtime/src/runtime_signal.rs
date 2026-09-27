//! Fixed, coalescing wakeup topics. Payloads and durable work remain in PostgreSQL.
#[derive(Clone, Copy)]
pub enum RuntimeSignal {
    Targets,
    Automation,
    Builds,
    Backups,
    Restores,
    BuildCompletion,
    ContainerRecovery,
    DeploymentRecovery,
    StackRecovery,
    SwarmServiceRecovery,
    SwarmServiceOperations,
    StackWebhooks,
    PlatformStats,
    Git,
    AlertRules,
}
impl RuntimeSignal {
    pub const ALL: [Self; 15] = [
        Self::Targets,
        Self::Automation,
        Self::Builds,
        Self::Backups,
        Self::Restores,
        Self::BuildCompletion,
        Self::ContainerRecovery,
        Self::DeploymentRecovery,
        Self::StackRecovery,
        Self::SwarmServiceRecovery,
        Self::SwarmServiceOperations,
        Self::StackWebhooks,
        Self::PlatformStats,
        Self::Git,
        Self::AlertRules,
    ];
    pub const fn channel(self) -> &'static str {
        match self {
            Self::Targets => "citadel_platform_targets",
            Self::Automation => "citadel_automation_work",
            Self::Builds => "citadel_build_work",
            Self::Backups => "citadel_backup_work",
            Self::Restores => "citadel_restore_work",
            Self::BuildCompletion => "citadel_build_completion",
            Self::ContainerRecovery => "citadel_container_recovery",
            Self::DeploymentRecovery => "citadel_deployment_recovery",
            Self::StackRecovery => "citadel_stack_recovery",
            Self::SwarmServiceRecovery => "citadel_service_recovery",
            Self::SwarmServiceOperations => "citadel_service_operations",
            Self::StackWebhooks => "citadel_stack_webhooks",
            Self::Git => "citadel_git_work",
            Self::AlertRules => "citadel_alert_rules",
            Self::PlatformStats => "citadel_platform_stats",
        }
    }
}
