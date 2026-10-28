citadel_primitives::status_enum! {
    pub enum ScriptRunStatus {
        Queued,
        Running,
        Succeeded,
        Failed,
        TimedOut,
        Cancelled,
        Rejected,
    }
}

impl ScriptRunStatus {
    pub const fn is_terminal(self) -> bool {
        match self {
            Self::Queued | Self::Running => false,
            Self::Succeeded | Self::Failed | Self::TimedOut | Self::Cancelled | Self::Rejected => {
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_include_failure_and_recovery_but_not_work_in_progress() {
        for status in [
            ScriptRunStatus::Succeeded,
            ScriptRunStatus::Failed,
            ScriptRunStatus::TimedOut,
            ScriptRunStatus::Cancelled,
            ScriptRunStatus::Rejected,
        ] {
            assert!(status.is_terminal(), "{status}");
        }
        for status in [ScriptRunStatus::Queued, ScriptRunStatus::Running] {
            assert!(!status.is_terminal(), "{status}");
        }
    }
}
