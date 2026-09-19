//! Fixed, coalescing wakeup topics. Payloads and durable work remain in PostgreSQL.
#[derive(Clone, Copy)]
pub enum RuntimeSignal {
    Targets,
    Automation,
    Builds,
    Backups,
    Restores,
    BuildCompletion,
}
impl RuntimeSignal {
    pub const ALL: [Self; 6] = [
        Self::Targets,
        Self::Automation,
        Self::Builds,
        Self::Backups,
        Self::Restores,
        Self::BuildCompletion,
    ];
    pub const fn channel(self) -> &'static str {
        match self {
            Self::Targets => "citadel_platform_targets",
            Self::Automation => "citadel_automation_work",
            Self::Builds => "citadel_build_work",
            Self::Backups => "citadel_backup_work",
            Self::Restores => "citadel_restore_work",
            Self::BuildCompletion => "citadel_build_completion",
        }
    }
}
