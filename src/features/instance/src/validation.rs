use crate::environment;
use crate::{InstanceConfiguration, InstanceError, InstanceOverrides, PatchInstanceConfiguration};
use citadel_primitives::PatchField;

impl InstanceConfiguration {
    pub fn effective(&self, overrides: &InstanceOverrides) -> Self {
        Self {
            password_minimum_length: overrides
                .password_minimum_length
                .unwrap_or(self.password_minimum_length),
            browser_session_lifetime_days: overrides
                .browser_session_lifetime_days
                .unwrap_or(self.browser_session_lifetime_days),
            service_account_default_token_lifetime_days: overrides
                .service_account_default_token_lifetime_days
                .unwrap_or(self.service_account_default_token_lifetime_days),
            service_account_maximum_token_lifetime_days: overrides
                .service_account_maximum_token_lifetime_days
                .unwrap_or(self.service_account_maximum_token_lifetime_days),
            service_account_maximum_active_tokens: overrides
                .service_account_maximum_active_tokens
                .unwrap_or(self.service_account_maximum_active_tokens),
            access_token_lifetime_minutes: overrides
                .access_token_lifetime_minutes
                .unwrap_or(self.access_token_lifetime_minutes),
            statistics_collection_interval_seconds: overrides
                .statistics_collection_interval_seconds
                .unwrap_or(self.statistics_collection_interval_seconds),
            statistics_write_interval_seconds: overrides
                .statistics_write_interval_seconds
                .unwrap_or(self.statistics_write_interval_seconds),
            scripts_enabled: overrides.scripts_enabled.unwrap_or(self.scripts_enabled),
            scripts_maximum_parallel_runs: overrides
                .scripts_maximum_parallel_runs
                .unwrap_or(self.scripts_maximum_parallel_runs),
            scripts_maximum_remote_runs: overrides
                .scripts_maximum_remote_runs
                .unwrap_or(self.scripts_maximum_remote_runs),
            scripts_default_timeout_seconds: overrides
                .scripts_default_timeout_seconds
                .unwrap_or(self.scripts_default_timeout_seconds),
            scripts_maximum_timeout_seconds: overrides
                .scripts_maximum_timeout_seconds
                .unwrap_or(self.scripts_maximum_timeout_seconds),
            build_preparation_slots: overrides
                .build_preparation_slots
                .unwrap_or(self.build_preparation_slots),
            build_tracking_slots: overrides
                .build_tracking_slots
                .unwrap_or(self.build_tracking_slots),
            build_cleanup_enabled: overrides
                .build_cleanup_enabled
                .unwrap_or(self.build_cleanup_enabled),
            build_retention_days: overrides
                .build_retention_days
                .unwrap_or(self.build_retention_days),
            backups_enabled: overrides.backups_enabled.unwrap_or(self.backups_enabled),
            backup_tracking_slots: overrides
                .backup_tracking_slots
                .unwrap_or(self.backup_tracking_slots),
            backup_default_timeout_seconds: overrides
                .backup_default_timeout_seconds
                .unwrap_or(self.backup_default_timeout_seconds),
            openapi_enabled: overrides.openapi_enabled.unwrap_or(self.openapi_enabled),
            require_direct_agent_https: overrides
                .require_direct_agent_https
                .unwrap_or(self.require_direct_agent_https),
            agent_image_override: overrides
                .agent_image_override
                .clone()
                .unwrap_or_else(|| self.agent_image_override.clone()),
            log_verbosity: overrides
                .log_verbosity
                .clone()
                .unwrap_or_else(|| self.log_verbosity.clone()),
            log_format: overrides.log_format.unwrap_or(self.log_format),
        }
    }

    /// Check only the deployment Core connector. Database-backed Agent inventory
    /// is deliberately outside this setting validation.
    pub fn validate_deployment(&self, core_agent_requires_http: bool) -> Result<(), InstanceError> {
        if self.require_direct_agent_https && core_agent_requires_http {
            return Err(InstanceError::Validation(
                "Configure CITADEL_CORE_AGENT_ADDRESS to use HTTPS before requiring HTTPS for Direct Agents.".into(),
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), InstanceError> {
        if !(8..=128).contains(&self.password_minimum_length) {
            return Err(InstanceError::Validation(format!(
                "passwordMinimumLength must be between 8 and 128. ({})",
                environment::CITADEL_PASSWORDS_MINIMUM_LENGTH
            )));
        }
        if !(1..=365000).contains(&self.browser_session_lifetime_days) {
            return Err(InstanceError::Validation(format!(
                "browserSessionLifetimeDays must be between 1 and 365000. ({})",
                environment::CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS
            )));
        }
        if !(1..=365000).contains(&self.service_account_default_token_lifetime_days) {
            return Err(InstanceError::Validation(format!(
                "serviceAccountDefaultTokenLifetimeDays must be between 1 and 365000. ({})",
                environment::CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS
            )));
        }
        if !(1..=365000).contains(&self.service_account_maximum_token_lifetime_days) {
            return Err(InstanceError::Validation(format!(
                "serviceAccountMaximumTokenLifetimeDays must be between 1 and 365000. ({})",
                environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS
            )));
        }
        if !(1..=2147483647).contains(&self.service_account_maximum_active_tokens) {
            return Err(InstanceError::Validation(format!(
                "serviceAccountMaximumActiveTokens must be between 1 and 2147483647. ({})",
                environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT
            )));
        }
        if !(1..=2147483647).contains(&self.access_token_lifetime_minutes) {
            return Err(InstanceError::Validation(format!(
                "accessTokenLifetimeMinutes must be between 1 and 2147483647. ({})",
                environment::CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES
            )));
        }
        if !(1..=2147483647).contains(&self.statistics_collection_interval_seconds) {
            return Err(InstanceError::Validation(format!(
                "statisticsCollectionIntervalSeconds must be between 1 and 2147483647. ({})",
                environment::CITADEL_STATS_POLL_INTERVAL_SECONDS
            )));
        }
        if !(1..=2147483647).contains(&self.statistics_write_interval_seconds) {
            return Err(InstanceError::Validation(format!(
                "statisticsWriteIntervalSeconds must be between 1 and 2147483647. ({})",
                environment::CITADEL_STATS_FLUSH_INTERVAL_SECONDS
            )));
        }
        if !(1..=64).contains(&self.scripts_maximum_parallel_runs) {
            return Err(InstanceError::Validation(format!(
                "scriptsMaximumParallelRuns must be between 1 and 64. ({})",
                environment::CITADEL_SCRIPTS_MAX_PARALLEL_RUNS
            )));
        }
        if !(1..=256).contains(&self.scripts_maximum_remote_runs) {
            return Err(InstanceError::Validation(format!(
                "scriptsMaximumRemoteRuns must be between 1 and 256. ({})",
                environment::CITADEL_SCRIPTS_MAX_REMOTE_RUNS
            )));
        }
        if !(1..=86400).contains(&self.scripts_default_timeout_seconds) {
            return Err(InstanceError::Validation(format!(
                "scriptsDefaultTimeoutSeconds must be between 1 and 86400. ({})",
                environment::CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS
            )));
        }
        if !(1..=86400).contains(&self.scripts_maximum_timeout_seconds) {
            return Err(InstanceError::Validation(format!(
                "scriptsMaximumTimeoutSeconds must be between 1 and 86400. ({})",
                environment::CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS
            )));
        }
        if !(1..=16).contains(&self.build_preparation_slots) {
            return Err(InstanceError::Validation(format!(
                "buildPreparationSlots must be between 1 and 16. ({})",
                environment::CITADEL_BUILDS_PREPARATION_SLOTS
            )));
        }
        if !(1..=128).contains(&self.build_tracking_slots) {
            return Err(InstanceError::Validation(format!(
                "buildTrackingSlots must be between 1 and 128. ({})",
                environment::CITADEL_BUILDS_TRACKING_SLOTS
            )));
        }
        if !(1..=2147483647).contains(&self.build_retention_days) {
            return Err(InstanceError::Validation(format!(
                "buildRetentionDays must be between 1 and 2147483647. ({})",
                environment::CITADEL_BUILDS_RUN_RETENTION_DAYS
            )));
        }
        if !(1..=128).contains(&self.backup_tracking_slots) {
            return Err(InstanceError::Validation(format!(
                "backupTrackingSlots must be between 1 and 128. ({})",
                environment::CITADEL_BACKUPS_TRACKING_SLOTS
            )));
        }
        if !(5..=86400).contains(&self.backup_default_timeout_seconds) {
            return Err(InstanceError::Validation(format!(
                "backupDefaultTimeoutSeconds must be between 5 and 86400. ({})",
                environment::CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS
            )));
        }
        if self.service_account_default_token_lifetime_days
            > self.service_account_maximum_token_lifetime_days
        {
            return Err(InstanceError::Validation(
                "Default token lifetime cannot exceed its maximum.".into(),
            ));
        }
        if self.scripts_default_timeout_seconds > self.scripts_maximum_timeout_seconds {
            return Err(InstanceError::Validation(
                "Default script timeout cannot exceed its maximum.".into(),
            ));
        }
        // Deployment RUST_LOG can contain module filters. The HTTP patch only accepts levels.
        if self.log_verbosity.is_empty() || self.log_verbosity.len() > 4096 {
            return Err(InstanceError::Validation("Invalid log filter.".into()));
        }
        if self.agent_image_override.len() > 512
            || self.agent_image_override.chars().any(char::is_whitespace)
        {
            return Err(InstanceError::Validation("Agent image must be empty or an image reference without whitespace (maximum 512 characters).".into()));
        }
        Ok(())
    }
}

impl PatchInstanceConfiguration {
    pub fn is_empty(&self) -> bool {
        matches!(self.password_minimum_length, PatchField::Missing)
            && matches!(self.browser_session_lifetime_days, PatchField::Missing)
            && matches!(
                self.service_account_default_token_lifetime_days,
                PatchField::Missing
            )
            && matches!(
                self.service_account_maximum_token_lifetime_days,
                PatchField::Missing
            )
            && matches!(
                self.service_account_maximum_active_tokens,
                PatchField::Missing
            )
            && matches!(self.access_token_lifetime_minutes, PatchField::Missing)
            && matches!(
                self.statistics_collection_interval_seconds,
                PatchField::Missing
            )
            && matches!(self.statistics_write_interval_seconds, PatchField::Missing)
            && matches!(self.scripts_enabled, PatchField::Missing)
            && matches!(self.scripts_maximum_parallel_runs, PatchField::Missing)
            && matches!(self.scripts_maximum_remote_runs, PatchField::Missing)
            && matches!(self.scripts_default_timeout_seconds, PatchField::Missing)
            && matches!(self.scripts_maximum_timeout_seconds, PatchField::Missing)
            && matches!(self.build_preparation_slots, PatchField::Missing)
            && matches!(self.build_tracking_slots, PatchField::Missing)
            && matches!(self.build_cleanup_enabled, PatchField::Missing)
            && matches!(self.build_retention_days, PatchField::Missing)
            && matches!(self.backups_enabled, PatchField::Missing)
            && matches!(self.backup_tracking_slots, PatchField::Missing)
            && matches!(self.backup_default_timeout_seconds, PatchField::Missing)
            && matches!(self.openapi_enabled, PatchField::Missing)
            && matches!(self.require_direct_agent_https, PatchField::Missing)
            && matches!(self.agent_image_override, PatchField::Missing)
            && matches!(self.log_verbosity, PatchField::Missing)
            && matches!(self.log_format, PatchField::Missing)
    }

    /// Merge against the locked singleton so concurrent partial updates remain atomic.
    pub fn apply(
        &self,
        saved: &InstanceConfiguration,
        overrides: &InstanceOverrides,
    ) -> Result<InstanceConfiguration, InstanceError> {
        let next = InstanceConfiguration {
            password_minimum_length: patch_value(
                &self.password_minimum_length,
                &saved.password_minimum_length,
                &overrides.password_minimum_length,
                environment::CITADEL_PASSWORDS_MINIMUM_LENGTH,
            )?,
            browser_session_lifetime_days: patch_value(
                &self.browser_session_lifetime_days,
                &saved.browser_session_lifetime_days,
                &overrides.browser_session_lifetime_days,
                environment::CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS,
            )?,
            service_account_default_token_lifetime_days: patch_value(
                &self.service_account_default_token_lifetime_days,
                &saved.service_account_default_token_lifetime_days,
                &overrides.service_account_default_token_lifetime_days,
                environment::CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS,
            )?,
            service_account_maximum_token_lifetime_days: patch_value(
                &self.service_account_maximum_token_lifetime_days,
                &saved.service_account_maximum_token_lifetime_days,
                &overrides.service_account_maximum_token_lifetime_days,
                environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS,
            )?,
            service_account_maximum_active_tokens: patch_value(
                &self.service_account_maximum_active_tokens,
                &saved.service_account_maximum_active_tokens,
                &overrides.service_account_maximum_active_tokens,
                environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT,
            )?,
            access_token_lifetime_minutes: patch_value(
                &self.access_token_lifetime_minutes,
                &saved.access_token_lifetime_minutes,
                &overrides.access_token_lifetime_minutes,
                environment::CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES,
            )?,
            statistics_collection_interval_seconds: patch_value(
                &self.statistics_collection_interval_seconds,
                &saved.statistics_collection_interval_seconds,
                &overrides.statistics_collection_interval_seconds,
                environment::CITADEL_STATS_POLL_INTERVAL_SECONDS,
            )?,
            statistics_write_interval_seconds: patch_value(
                &self.statistics_write_interval_seconds,
                &saved.statistics_write_interval_seconds,
                &overrides.statistics_write_interval_seconds,
                environment::CITADEL_STATS_FLUSH_INTERVAL_SECONDS,
            )?,
            scripts_enabled: patch_value(
                &self.scripts_enabled,
                &saved.scripts_enabled,
                &overrides.scripts_enabled,
                environment::CITADEL_SCRIPTS_ENABLED,
            )?,
            scripts_maximum_parallel_runs: patch_value(
                &self.scripts_maximum_parallel_runs,
                &saved.scripts_maximum_parallel_runs,
                &overrides.scripts_maximum_parallel_runs,
                environment::CITADEL_SCRIPTS_MAX_PARALLEL_RUNS,
            )?,
            scripts_maximum_remote_runs: patch_value(
                &self.scripts_maximum_remote_runs,
                &saved.scripts_maximum_remote_runs,
                &overrides.scripts_maximum_remote_runs,
                environment::CITADEL_SCRIPTS_MAX_REMOTE_RUNS,
            )?,
            scripts_default_timeout_seconds: patch_value(
                &self.scripts_default_timeout_seconds,
                &saved.scripts_default_timeout_seconds,
                &overrides.scripts_default_timeout_seconds,
                environment::CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS,
            )?,
            scripts_maximum_timeout_seconds: patch_value(
                &self.scripts_maximum_timeout_seconds,
                &saved.scripts_maximum_timeout_seconds,
                &overrides.scripts_maximum_timeout_seconds,
                environment::CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS,
            )?,
            build_preparation_slots: patch_value(
                &self.build_preparation_slots,
                &saved.build_preparation_slots,
                &overrides.build_preparation_slots,
                environment::CITADEL_BUILDS_PREPARATION_SLOTS,
            )?,
            build_tracking_slots: patch_value(
                &self.build_tracking_slots,
                &saved.build_tracking_slots,
                &overrides.build_tracking_slots,
                environment::CITADEL_BUILDS_TRACKING_SLOTS,
            )?,
            build_cleanup_enabled: patch_value(
                &self.build_cleanup_enabled,
                &saved.build_cleanup_enabled,
                &overrides.build_cleanup_enabled,
                environment::CITADEL_BUILDS_RUN_CLEANUP_ENABLED,
            )?,
            build_retention_days: patch_value(
                &self.build_retention_days,
                &saved.build_retention_days,
                &overrides.build_retention_days,
                environment::CITADEL_BUILDS_RUN_RETENTION_DAYS,
            )?,
            backups_enabled: patch_value(
                &self.backups_enabled,
                &saved.backups_enabled,
                &overrides.backups_enabled,
                environment::CITADEL_BACKUPS_ENABLED,
            )?,
            backup_tracking_slots: patch_value(
                &self.backup_tracking_slots,
                &saved.backup_tracking_slots,
                &overrides.backup_tracking_slots,
                environment::CITADEL_BACKUPS_TRACKING_SLOTS,
            )?,
            backup_default_timeout_seconds: patch_value(
                &self.backup_default_timeout_seconds,
                &saved.backup_default_timeout_seconds,
                &overrides.backup_default_timeout_seconds,
                environment::CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS,
            )?,
            openapi_enabled: patch_value(
                &self.openapi_enabled,
                &saved.openapi_enabled,
                &overrides.openapi_enabled,
                environment::CITADEL_HTTP_OPENAPI_ENABLED,
            )?,
            require_direct_agent_https: patch_value(
                &self.require_direct_agent_https,
                &saved.require_direct_agent_https,
                &overrides.require_direct_agent_https,
                environment::CITADEL_CORE_AGENT_ALLOW_INSECURE,
            )?,
            agent_image_override: patch_value(
                &self.agent_image_override,
                &saved.agent_image_override,
                &overrides.agent_image_override,
                environment::CITADEL_EDGE_AGENT_IMAGE,
            )?,
            log_verbosity: patch_value(
                &self.log_verbosity,
                &saved.log_verbosity,
                &overrides.log_verbosity,
                environment::RUST_LOG,
            )?,
            log_format: patch_value(
                &self.log_format,
                &saved.log_format,
                &overrides.log_format,
                environment::CITADEL_LOG_FORMAT,
            )?,
        };
        if let PatchField::Value(level) = &self.log_verbosity {
            if !matches!(
                level.as_str(),
                "off" | "error" | "warn" | "info" | "debug" | "trace"
            ) {
                return Err(InstanceError::Validation(
                    "Choose a supported log verbosity.".into(),
                ));
            }
        }
        next.effective(overrides).validate()?;
        Ok(next)
    }
}

fn patch_value<T: Clone + PartialEq>(
    patch: &PatchField<T>,
    saved: &T,
    deployment: &Option<T>,
    environment: &str,
) -> Result<T, InstanceError> {
    match patch {
        PatchField::Missing => Ok(saved.clone()),
        PatchField::Null => Err(InstanceError::Validation(format!(
            "{environment} cannot be null."
        ))),
        PatchField::Value(value) => {
            if deployment.as_ref().is_some_and(|active| active != value) {
                return Err(InstanceError::Validation(format!(
                    "This setting is managed by {environment}."
                )));
            }
            Ok(value.clone())
        }
    }
}
