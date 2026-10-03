use crate::AutomationError;
use std::time::Duration;

/// Execution limits shared by HTTP, webhooks, the scheduler and workers.
#[derive(Debug, Clone, Copy)]
pub struct AutomationOptions {
    pub enabled: bool,
    pub max_parallel_runs: usize,
    pub default_timeout_seconds: i32,
    pub max_timeout_seconds: i32,
    pub poll_interval: Duration,
    pub schedule_poll_interval: Duration,
}

impl Default for AutomationOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            max_parallel_runs: 4,
            default_timeout_seconds: 300,
            max_timeout_seconds: 1800,
            poll_interval: Duration::from_secs(2),
            schedule_poll_interval: Duration::from_secs(30),
        }
    }
}

impl AutomationOptions {
    pub fn validate(&self) -> Result<(), AutomationError> {
        if !(1..=64).contains(&self.max_parallel_runs)
            || !(1..=86_400).contains(&self.max_timeout_seconds)
            || !(1..=self.max_timeout_seconds).contains(&self.default_timeout_seconds)
            || self.poll_interval < Duration::from_secs(1)
            || self.schedule_poll_interval < Duration::from_secs(5)
        {
            return Err(AutomationError::Validation("Invalid Automation execution limits: parallel runs must be 1–64, timeout defaults must fit the 1–86400 second maximum, polling at least 1 second and schedule polling at least 5 seconds.".into()));
        }
        Ok(())
    }

    pub fn validate_execution(&self, timeout: i32) -> Result<(), AutomationError> {
        if !self.enabled {
            return Err(AutomationError::Conflict(
                "Automation execution is disabled in server configuration.".into(),
            ));
        }
        self.validate_timeout(timeout)
    }

    pub fn validate_timeout(&self, timeout: i32) -> Result<(), AutomationError> {
        if !(1..=self.max_timeout_seconds).contains(&timeout) {
            return Err(AutomationError::Validation(format!(
                "Timeout must be between 1 and {} seconds.",
                self.max_timeout_seconds
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_bounds_match_automation_options_and_queue_validation() {
        let defaults = AutomationOptions::default();
        assert_eq!(defaults.default_timeout_seconds, 300);
        assert_eq!(defaults.max_timeout_seconds, 1800);
        assert!(defaults.validate_execution(1800).is_ok());
        for seconds in [0, -1, 1801, i32::MAX] {
            assert!(defaults.validate_execution(seconds).is_err());
        }
        let disabled = AutomationOptions {
            enabled: false,
            ..defaults
        };
        assert!(disabled.validate_execution(1).is_err());
        assert!(
            disabled.validate_timeout(1).is_ok(),
            "disabled execution still permits editing configuration"
        );
        for options in [
            AutomationOptions {
                max_parallel_runs: 0,
                ..defaults
            },
            AutomationOptions {
                max_parallel_runs: 65,
                ..defaults
            },
            AutomationOptions {
                default_timeout_seconds: 1801,
                ..defaults
            },
            AutomationOptions {
                poll_interval: Duration::ZERO,
                ..defaults
            },
        ] {
            assert!(options.validate().is_err());
        }
    }
}
