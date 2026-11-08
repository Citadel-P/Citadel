//! Resolves non-secret installation choices before constructing startup-bound consumers.
use super::environment;
use super::{Config, ConfigError, LogFormat};
use citadel_instance::{InstanceConfiguration, InstanceOverrides};
use std::time::Duration;

pub(super) fn read_overrides(
    get: impl Fn(&str) -> Option<String>,
) -> Result<InstanceOverrides, ConfigError> {
    Ok(InstanceOverrides {
        password_minimum_length: optional::<u32>(
            &get,
            environment::CITADEL_PASSWORDS_MINIMUM_LENGTH,
        )?,
        browser_session_lifetime_days: optional::<u32>(
            &get,
            environment::CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS,
        )?,
        service_account_default_token_lifetime_days: optional::<i32>(
            &get,
            environment::CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS,
        )?,
        service_account_maximum_token_lifetime_days: optional::<i32>(
            &get,
            environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS,
        )?,
        service_account_maximum_active_tokens: optional::<i32>(
            &get,
            environment::CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT,
        )?,
        access_token_lifetime_minutes: optional::<u32>(
            &get,
            environment::CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES,
        )?,
        statistics_collection_interval_seconds: optional::<u32>(
            &get,
            environment::CITADEL_STATS_POLL_INTERVAL_SECONDS,
        )?,
        statistics_write_interval_seconds: optional::<u32>(
            &get,
            environment::CITADEL_STATS_FLUSH_INTERVAL_SECONDS,
        )?,
        scripts_enabled: optional::<bool>(&get, environment::CITADEL_SCRIPTS_ENABLED)?,
        scripts_maximum_parallel_runs: optional::<usize>(
            &get,
            environment::CITADEL_SCRIPTS_MAX_PARALLEL_RUNS,
        )?,
        scripts_maximum_remote_runs: optional::<usize>(
            &get,
            environment::CITADEL_SCRIPTS_MAX_REMOTE_RUNS,
        )?,
        scripts_default_timeout_seconds: optional::<u32>(
            &get,
            environment::CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS,
        )?,
        scripts_maximum_timeout_seconds: optional::<u32>(
            &get,
            environment::CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS,
        )?,
        build_preparation_slots: optional::<usize>(
            &get,
            environment::CITADEL_BUILDS_PREPARATION_SLOTS,
        )?,
        build_tracking_slots: optional::<usize>(&get, environment::CITADEL_BUILDS_TRACKING_SLOTS)?,
        build_cleanup_enabled: optional::<bool>(
            &get,
            environment::CITADEL_BUILDS_RUN_CLEANUP_ENABLED,
        )?,
        build_retention_days: optional::<i32>(
            &get,
            environment::CITADEL_BUILDS_RUN_RETENTION_DAYS,
        )?,
        backups_enabled: optional::<bool>(&get, environment::CITADEL_BACKUPS_ENABLED)?,
        backup_tracking_slots: optional::<usize>(
            &get,
            environment::CITADEL_BACKUPS_TRACKING_SLOTS,
        )?,
        backup_default_timeout_seconds: optional::<u32>(
            &get,
            environment::CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS,
        )?,
        openapi_enabled: optional::<bool>(&get, environment::CITADEL_HTTP_OPENAPI_ENABLED)?,
        require_direct_agent_https: optional::<bool>(
            &get,
            environment::CITADEL_CORE_AGENT_ALLOW_INSECURE,
        )?
        .map(|value| !value),
        agent_image_override: get(environment::CITADEL_EDGE_AGENT_IMAGE),
        log_verbosity: get(environment::RUST_LOG),
        log_format: optional::<super::LogFormat>(&get, environment::CITADEL_LOG_FORMAT)?
            .map(Into::into),
    })
}

fn optional<T: std::str::FromStr>(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<Option<T>, ConfigError>
where
    T::Err: std::fmt::Display,
{
    get(name)
        .map(|value| {
            value.parse().map_err(|error: T::Err| ConfigError::Invalid {
                name,
                message: error.to_string(),
            })
        })
        .transpose()
}

impl Config {
    /// Freshness follows the resolved collection cadence, with the existing floor.
    pub fn statistics_sample_max_age(&self) -> Duration {
        self.probe_interval
            .saturating_mul(3)
            .max(Duration::from_secs(30))
    }

    /// Only the deployment-configured Core connector can prevent Core construction.
    pub fn deployment_core_agent_requires_http(&self) -> bool {
        self.agent.as_ref().is_some_and(|agent| {
            url::Url::parse(&agent.address).is_ok_and(|url| url.scheme() == "http")
        })
    }

    pub fn resolve_instance_configuration(
        &mut self,
        saved: &InstanceConfiguration,
    ) -> Result<(), ConfigError> {
        self.apply_instance_configuration(saved, self.instance_overrides.clone())
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
        effective
            .validate_deployment(self.deployment_core_agent_requires_http())
            .map_err(|error| ConfigError::Invalid {
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

impl From<citadel_instance::LogFormat> for LogFormat {
    fn from(value: citadel_instance::LogFormat) -> Self {
        match value {
            citadel_instance::LogFormat::Text => Self::Text,
            citadel_instance::LogFormat::Json => Self::Json,
        }
    }
}
impl From<LogFormat> for citadel_instance::LogFormat {
    fn from(value: LogFormat) -> Self {
        match value {
            LogFormat::Text => Self::Text,
            LogFormat::Json => Self::Json,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_presence_is_separate_from_defaults_for_every_promoted_field() {
        let absent = read_overrides(|_| None).unwrap();
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
                read_overrides(|name| (name == environment).then(|| value.into())).unwrap();
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
    #[tokio::test(start_paused = true)]
    async fn resolved_collection_cadence_reaches_consumers() {
        // Isolate actual environment parsing in a child test process; no mutation
        // of this test runner's process-global environment or running services.
        if std::env::var("CITADEL_TEST_CADENCE_CHILD").as_deref() != Ok("1") {
            let root =
                std::env::temp_dir().join(format!("citadel-cadence-{}", uuid::Uuid::now_v7()));
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "config::instance::tests::resolved_collection_cadence_reaches_consumers",
                    "--nocapture",
                ])
                .env_clear()
                .env("CITADEL_TEST_CADENCE_CHILD", "1")
                .env("CITADEL_DATA_ROOT", &root)
                .env(
                    "CITADEL_DATABASE_URL",
                    "postgres://unused:unused@localhost/unused",
                )
                .env("CITADEL_TRANSPORT_MODE", "Disabled")
                .env("CITADEL_PUBLIC_URL", "http://localhost:8000")
                .output()
                .unwrap();
            let _ = std::fs::remove_dir_all(root);
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let mut config = Config::from_env_for_startup().unwrap();
        let mut saved = InstanceConfiguration {
            statistics_collection_interval_seconds: 17,
            ..Default::default()
        };
        config.resolve_instance_configuration(&saved).unwrap();
        let now = tokio::time::Instant::now();
        let mut samples = crate::workers::schedule::interval("local-stats", config.probe_interval);
        let first = samples.tick().await;
        assert!(first >= now + Duration::from_secs(17));
        assert!(first < now + Duration::from_secs(18));
        assert_eq!(samples.tick().await - first, Duration::from_secs(17));
        assert_eq!(config.statistics_sample_max_age(), Duration::from_secs(51));
        saved.statistics_collection_interval_seconds = 2;
        config.resolve_instance_configuration(&saved).unwrap();
        assert_eq!(config.statistics_sample_max_age(), Duration::from_secs(30));
    }

    #[test]
    fn lower_environment_maxima_are_validated_after_saved_defaults_are_merged() {
        let overrides = read_overrides(|name| match name {
            "CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS" => Some("60".into()),
            "CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS" => Some("20".into()),
            _ => None,
        })
        .unwrap();
        assert!(
            InstanceConfiguration::default()
                .effective(&overrides)
                .validate()
                .is_err()
        );
        let saved = InstanceConfiguration {
            scripts_default_timeout_seconds: 23,
            service_account_default_token_lifetime_days: 10,
            ..Default::default()
        };
        let effective = saved.effective(&overrides);
        effective.validate().unwrap();
        assert_eq!(effective.scripts_default_timeout_seconds, 23);
        assert_eq!(effective.service_account_default_token_lifetime_days, 10);
        assert!(
            InstanceConfiguration {
                scripts_default_timeout_seconds: 61,
                ..saved.clone()
            }
            .effective(&overrides)
            .validate()
            .is_err()
        );
        assert!(
            InstanceConfiguration {
                service_account_default_token_lifetime_days: 21,
                ..saved
            }
            .effective(&overrides)
            .validate()
            .is_err()
        );
    }

    #[test]
    fn deployment_module_filters_are_preserved() {
        let overrides = read_overrides(|name| {
            (name == "RUST_LOG").then(|| "info,sqlx=warn,citadel_server=debug".into())
        })
        .unwrap();
        assert_eq!(
            overrides.log_verbosity.as_deref(),
            Some("info,sqlx=warn,citadel_server=debug")
        );
    }
}
