citadel_primitives::status_enum! {
    pub enum BackupRepositoryStatus {
        Unavailable,
        Unknown,
        Uninitialized,
        Ready,
    }
}

citadel_primitives::status_enum! {
    pub enum BackupRepositoryValidationStatus {
        Unknown,
        Ready,
        Uninitialized,
        Unavailable,
        InvalidPassword,
        InvalidConfiguration,
    }
}

citadel_primitives::status_enum! {
    pub enum BackupRestoreStatus {
        Queued,
        Preparing,
        Running,
        Succeeded,
        SucceededWithWarnings,
        Failed,
        TimedOut,
        Cancelled,
        Rejected,
        Interrupted,
        Processing,
    }
}

citadel_primitives::status_enum! {
    pub enum BackupRunStatus {
        Queued,
        Preparing,
        Running,
        ApplyingRetention,
        Succeeded,
        SucceededWithWarnings,
        Failed,
        TimedOut,
        Cancelled,
        Rejected,
        Interrupted,
        Processing,
    }
}

citadel_primitives::status_enum! {
    pub enum BackupRunItemStatus {
        Queued,
        Pending,
        Running,
        Succeeded,
        Failed,
        Cancelled,
    }
}

citadel_primitives::status_enum! {
    pub enum BackupSnapshotAvailability {
        Pending,
        Available,
        Expired,
        Missing,
        NotCreated,
    }
}

citadel_primitives::status_enum! {
    pub enum BackupCoverageStatus {
        NotApplicable,
        Unprotected,
        Protected,
        Warning,
        Failed,
    }
}

impl BackupRestoreStatus {
    pub const fn is_terminal(self) -> bool {
        match self {
            Self::Queued | Self::Preparing | Self::Running | Self::Processing => false,
            Self::Succeeded
            | Self::SucceededWithWarnings
            | Self::Failed
            | Self::TimedOut
            | Self::Cancelled
            | Self::Rejected
            | Self::Interrupted => true,
        }
    }
}

impl BackupRunStatus {
    pub const fn is_terminal(self) -> bool {
        match self {
            Self::Queued
            | Self::Preparing
            | Self::Running
            | Self::Processing
            | Self::ApplyingRetention => false,
            Self::Succeeded
            | Self::SucceededWithWarnings
            | Self::Failed
            | Self::TimedOut
            | Self::Cancelled
            | Self::Rejected
            | Self::Interrupted => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_include_failure_and_recovery_but_not_work_in_progress() {
        for status in [
            BackupRunStatus::Succeeded,
            BackupRunStatus::SucceededWithWarnings,
            BackupRunStatus::Failed,
            BackupRunStatus::TimedOut,
            BackupRunStatus::Cancelled,
            BackupRunStatus::Rejected,
            BackupRunStatus::Interrupted,
        ] {
            assert!(status.is_terminal(), "{status}");
        }
        for status in [
            BackupRunStatus::Queued,
            BackupRunStatus::Preparing,
            BackupRunStatus::Running,
            BackupRunStatus::ApplyingRetention,
            BackupRunStatus::Processing,
        ] {
            assert!(!status.is_terminal(), "{status}");
        }
    }

    #[test]
    fn restore_progress_uses_the_same_terminal_classification() {
        for status in BackupRestoreStatus::ALL {
            assert_eq!(
                crate::runs::progress::is_terminal(status.as_str()),
                status.is_terminal()
            );
        }
        assert!(!crate::runs::progress::is_terminal("unexpected"));
    }
}
