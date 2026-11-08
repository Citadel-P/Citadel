//! Resolves non-secret installation choices before constructing startup-bound consumers.
use super::environment;
use super::{Config, ConfigError};
use citadel_instance::{InstanceConfiguration, InstanceOverrides};
use std::time::Duration;

impl InstanceEnvironment {
    pub(super) fn read(
        get: impl Fn(&str) -> Option<String>,
    ) -> Result<InstanceOverrides, ConfigError> {
        Ok(InstanceOverrides {
            password_minimum_length: get(environment::CITADEL_PASSWORDS_MINIMUM_LENGTH)
                .map(|v| v.parse::<u32>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_PASSWORDS_MINIMUM_LENGTH,
                    message: error.to_string(),
                })?,
            browser_session_lifetime_days: get(
                environment::CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS,
            )
            .map(|v| v.parse::<u32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS,
                message: error.to_string(),
            })?,
            service_account_default_token_lifetime_days: get(
                environment::CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS,
            )
            .map(|v| v.parse::<i32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS,
                message: error.to_string(),
            })?,
            service_account_maximum_token_lifetime_days: get(
                environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS,
            )
            .map(|v| v.parse::<i32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS,
                message: error.to_string(),
            })?,
            service_account_maximum_active_tokens: get(
                environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT,
            )
            .map(|v| v.parse::<i32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT,
                message: error.to_string(),
            })?,
            access_token_lifetime_minutes: get(
                environment::CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES,
            )
            .map(|v| v.parse::<u32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES,
                message: error.to_string(),
            })?,
            statistics_collection_interval_seconds: get(
                environment::CITADEL_STATS_POLL_INTERVAL_SECONDS,
            )
            .map(|v| v.parse::<u32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_STATS_POLL_INTERVAL_SECONDS,
                message: error.to_string(),
            })?,
            statistics_write_interval_seconds: get(
                environment::CITADEL_STATS_FLUSH_INTERVAL_SECONDS,
            )
            .map(|v| v.parse::<u32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_STATS_FLUSH_INTERVAL_SECONDS,
                message: error.to_string(),
            })?,
            scripts_enabled: get(environment::CITADEL_SCRIPTS_ENABLED)
                .map(|v| v.parse::<bool>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_SCRIPTS_ENABLED,
                    message: error.to_string(),
                })?,
            scripts_maximum_parallel_runs: get(environment::CITADEL_SCRIPTS_MAX_PARALLEL_RUNS)
                .map(|v| v.parse::<usize>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_SCRIPTS_MAX_PARALLEL_RUNS,
                    message: error.to_string(),
                })?,
            scripts_maximum_remote_runs: get(environment::CITADEL_SCRIPTS_MAX_REMOTE_RUNS)
                .map(|v| v.parse::<usize>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_SCRIPTS_MAX_REMOTE_RUNS,
                    message: error.to_string(),
                })?,
            scripts_default_timeout_seconds: get(
                environment::CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS,
            )
            .map(|v| v.parse::<u32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS,
                message: error.to_string(),
            })?,
            scripts_maximum_timeout_seconds: get(environment::CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS)
                .map(|v| v.parse::<u32>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS,
                    message: error.to_string(),
                })?,
            build_preparation_slots: get(environment::CITADEL_BUILDS_PREPARATION_SLOTS)
                .map(|v| v.parse::<usize>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_BUILDS_PREPARATION_SLOTS,
                    message: error.to_string(),
                })?,
            build_tracking_slots: get(environment::CITADEL_BUILDS_TRACKING_SLOTS)
                .map(|v| v.parse::<usize>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_BUILDS_TRACKING_SLOTS,
                    message: error.to_string(),
                })?,
            build_cleanup_enabled: get(environment::CITADEL_BUILDS_RUN_CLEANUP_ENABLED)
                .map(|v| v.parse::<bool>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_BUILDS_RUN_CLEANUP_ENABLED,
                    message: error.to_string(),
                })?,
            build_retention_days: get(environment::CITADEL_BUILDS_RUN_RETENTION_DAYS)
                .map(|v| v.parse::<i32>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_BUILDS_RUN_RETENTION_DAYS,
                    message: error.to_string(),
                })?,
            backups_enabled: get(environment::CITADEL_BACKUPS_ENABLED)
                .map(|v| v.parse::<bool>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_BACKUPS_ENABLED,
                    message: error.to_string(),
                })?,
            backup_tracking_slots: get(environment::CITADEL_BACKUPS_TRACKING_SLOTS)
                .map(|v| v.parse::<usize>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_BACKUPS_TRACKING_SLOTS,
                    message: error.to_string(),
                })?,
            backup_default_timeout_seconds: get(
                environment::CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS,
            )
            .map(|v| v.parse::<u32>())
            .transpose()
            .map_err(|error| ConfigError::Invalid {
                name: environment::CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS,
                message: error.to_string(),
            })?,
            openapi_enabled: get(environment::CITADEL_HTTP_OPENAPI_ENABLED)
                .map(|v| v.parse::<bool>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_HTTP_OPENAPI_ENABLED,
                    message: error.to_string(),
                })?,
            require_direct_agent_https: get(environment::CITADEL_CORE_AGENT_ALLOW_INSECURE)
                .map(|v| v.parse::<bool>())
                .transpose()
                .map_err(|error| ConfigError::Invalid {
                    name: environment::CITADEL_CORE_AGENT_ALLOW_INSECURE,
                    message: error.to_string(),
                })?
                .map(|value| !value),
            agent_image_override: get(environment::CITADEL_EDGE_AGENT_IMAGE),
            log_verbosity: get(environment::RUST_LOG),
            log_format: get(environment::CITADEL_LOG_FORMAT).map(|v| {
                match v.trim().to_ascii_lowercase().as_str() {
                    "text" => "Text".into(),
                    "json" => "Json".into(),
                    _ => v,
                }
            }),
        })
    }
}

struct InstanceEnvironment;

impl Config {
    pub fn resolve_instance_configuration(
        &mut self,
        saved: &InstanceConfiguration,
    ) -> Result<(), ConfigError> {
        let overrides = InstanceEnvironment::read(|name| std::env::var(name).ok())?;
        self.apply_instance_configuration(saved, overrides)
    }

    fn apply_instance_configuration(
        &mut self,
        saved: &InstanceConfiguration,
        overrides: InstanceOverrides,
    ) -> Result<(), ConfigError> {
        let effective = saved.effective(&overrides);
        effective.validate().map_err(|error| ConfigError::Invalid {
            name: "Instance settings",
            message: error.to_string(),
        })?;
        let c = &effective;
        self.identity.password_policy = citadel_identity::PasswordPolicy::new(
            c.password_minimum_length as usize,
        )
        .map_err(|error| ConfigError::Invalid {
            name: "Minimum password length",
            message: error.to_string(),
        })?;
        self.identity.refresh_token_lifetime =
            Duration::from_secs(u64::from(c.browser_session_lifetime_days) * 86400);
        self.identity.access_token_lifetime =
            Duration::from_secs(u64::from(c.access_token_lifetime_minutes) * 60);
        self.identity.service_account_limits = citadel_identity::ServiceAccountLimitsDetails {
            default_token_lifetime_days: i64::from(c.service_account_default_token_lifetime_days),
            maximum_token_lifetime_days: i64::from(c.service_account_maximum_token_lifetime_days),
            maximum_active_tokens_per_account: i64::from(c.service_account_maximum_active_tokens),
        };
        self.probe_interval =
            Duration::from_secs(u64::from(c.statistics_collection_interval_seconds));
        self.stats_flush_interval =
            Duration::from_secs(u64::from(c.statistics_write_interval_seconds));
        self.scripts.enabled = c.scripts_enabled;
        self.scripts.max_parallel_runs = c.scripts_maximum_parallel_runs;
        self.scripts.max_remote_runs = c.scripts_maximum_remote_runs;
        self.scripts.default_timeout_seconds = c.scripts_default_timeout_seconds as i32;
        self.scripts.max_timeout_seconds = c.scripts_maximum_timeout_seconds as i32;
        self.scripts
            .validate()
            .map_err(|error| ConfigError::Invalid {
                name: "Scripts",
                message: error.to_string(),
            })?;
        self.build_preparation_slots = c.build_preparation_slots;
        self.build_tracking_slots = c.build_tracking_slots;
        self.build_retention_days = c.build_cleanup_enabled.then_some(c.build_retention_days);
        self.backup_workers.enabled = c.backups_enabled;
        self.backup_workers.tracking_slots = c.backup_tracking_slots;
        self.execution.backups.settings.default_timeout =
            Duration::from_secs(u64::from(c.backup_default_timeout_seconds));
        self.transport.openapi_enabled = c.openapi_enabled;
        self.execution.edge_agent.allow_insecure = !c.require_direct_agent_https;
        if let Some(agent) = &mut self.agent {
            agent.allow_insecure = !c.require_direct_agent_https;
        }
        if !c.agent_image_override.is_empty() {
            self.execution
                .edge_agent
                .image
                .clone_from(&c.agent_image_override);
        }
        self.instance_configuration = effective;
        self.instance_overrides = overrides;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_presence_is_separate_from_defaults_for_every_promoted_field() {
        let absent = InstanceEnvironment::read(|_| None).unwrap();
        assert_eq!(absent, InstanceOverrides::default());
        let defaults = InstanceConfiguration::default();
        assert_eq!(defaults.effective(&absent), defaults);
        let cases = [
            (
                "CITADEL_PASSWORDS_MINIMUM_LENGTH",
                "passwordMinimumLength",
                "20",
            ),
            (
                "CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS",
                "browserSessionLifetimeDays",
                "20",
            ),
            (
                "CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS",
                "serviceAccountDefaultTokenLifetimeDays",
                "20",
            ),
            (
                "CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS",
                "serviceAccountMaximumTokenLifetimeDays",
                "20",
            ),
            (
                "CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT",
                "serviceAccountMaximumActiveTokens",
                "20",
            ),
            (
                "CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES",
                "accessTokenLifetimeMinutes",
                "20",
            ),
            (
                "CITADEL_STATS_POLL_INTERVAL_SECONDS",
                "statisticsCollectionIntervalSeconds",
                "20",
            ),
            (
                "CITADEL_STATS_FLUSH_INTERVAL_SECONDS",
                "statisticsWriteIntervalSeconds",
                "20",
            ),
            ("CITADEL_SCRIPTS_ENABLED", "scriptsEnabled", "false"),
            (
                "CITADEL_SCRIPTS_MAX_PARALLEL_RUNS",
                "scriptsMaximumParallelRuns",
                "20",
            ),
            (
                "CITADEL_SCRIPTS_MAX_REMOTE_RUNS",
                "scriptsMaximumRemoteRuns",
                "20",
            ),
            (
                "CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS",
                "scriptsDefaultTimeoutSeconds",
                "20",
            ),
            (
                "CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS",
                "scriptsMaximumTimeoutSeconds",
                "20",
            ),
            (
                "CITADEL_BUILDS_PREPARATION_SLOTS",
                "buildPreparationSlots",
                "20",
            ),
            ("CITADEL_BUILDS_TRACKING_SLOTS", "buildTrackingSlots", "20"),
            (
                "CITADEL_BUILDS_RUN_CLEANUP_ENABLED",
                "buildCleanupEnabled",
                "false",
            ),
            (
                "CITADEL_BUILDS_RUN_RETENTION_DAYS",
                "buildRetentionDays",
                "20",
            ),
            ("CITADEL_BACKUPS_ENABLED", "backupsEnabled", "false"),
            (
                "CITADEL_BACKUPS_TRACKING_SLOTS",
                "backupTrackingSlots",
                "20",
            ),
            (
                "CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS",
                "backupDefaultTimeoutSeconds",
                "20",
            ),
            ("CITADEL_HTTP_OPENAPI_ENABLED", "openapiEnabled", "false"),
            (
                "CITADEL_CORE_AGENT_ALLOW_INSECURE",
                "requireDirectAgentHttps",
                "false",
            ),
            ("CITADEL_EDGE_AGENT_IMAGE", "agentImageOverride", ""),
            ("RUST_LOG", "logVerbosity", "warn"),
            ("CITADEL_LOG_FORMAT", "logFormat", "json"),
        ];
        for (environment, field, value) in cases {
            let overrides =
                InstanceEnvironment::read(|name| (name == environment).then(|| value.into()))
                    .unwrap();
            let values = serde_json::to_value(&overrides).unwrap();
            assert!(
                !values[field].is_null(),
                "{environment} must be explicitly managed"
            );
            assert_eq!(
                values
                    .as_object()
                    .unwrap()
                    .values()
                    .filter(|value| !value.is_null())
                    .count(),
                1
            );
            let effective = serde_json::to_value(defaults.effective(&overrides)).unwrap();
            assert_eq!(effective[field], values[field]);
        }
    }
    #[test]
    fn deployment_module_filters_are_preserved() {
        let overrides = InstanceEnvironment::read(|name| {
            (name == "RUST_LOG").then(|| "info,sqlx=warn,citadel_server=debug".into())
        })
        .unwrap();
        assert_eq!(
            overrides.log_verbosity.as_deref(),
            Some("info,sqlx=warn,citadel_server=debug")
        );
    }
}
