citadel_primitives::status_enum! {
    pub enum BuildRunStatus {
        Queued,
        Preparing,
        Running,
        Succeeded,
        Failed,
        TimedOut,
        Cancelled,
        Rejected,
        Interrupted,
    }
}

citadel_primitives::status_enum! {
    pub enum BuildAgentPoolValidationStatus {
        NotTested,
        Ready,
        Invalid,
        Degraded,
    }
}

impl BuildRunStatus {
    pub const fn is_terminal(self) -> bool {
        match self {
            Self::Queued | Self::Preparing | Self::Running => false,
            Self::Succeeded
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
            BuildRunStatus::Succeeded,
            BuildRunStatus::Failed,
            BuildRunStatus::TimedOut,
            BuildRunStatus::Cancelled,
            BuildRunStatus::Rejected,
            BuildRunStatus::Interrupted,
        ] {
            assert!(status.is_terminal(), "{status}");
        }
        for status in [
            BuildRunStatus::Queued,
            BuildRunStatus::Preparing,
            BuildRunStatus::Running,
        ] {
            assert!(!status.is_terminal(), "{status}");
        }
    }
}
