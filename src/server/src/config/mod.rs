pub mod execution;
mod keys;

use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use citadel_identity::MfaPolicy;
use ipnet::IpNet;
use serde::Serialize;
use url::Url;
use zeroize::Zeroizing;

const DEFAULT_API_PORT: u16 = 8000;
const DEFAULT_MONITORING_INTERVAL_SECONDS: u64 = 10;
const DEFAULT_RECONCILIATION_INTERVAL_SECONDS: u64 = 30 * 60;
const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_DOCKER_REQUEST_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_SHUTDOWN_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_REALTIME_QUEUE_CAPACITY: usize = 64;
const DEFAULT_REALTIME_MAX_CONNECTIONS: usize = 8;
const DEFAULT_REALTIME_SUBSCRIBE_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_REALTIME_SEND_TIMEOUT_SECONDS: u64 = 2;
const DEFAULT_REALTIME_AUTH_RECHECK_SECONDS: u64 = 30;
const DEFAULT_REALTIME_SNAPSHOT_LIMIT: usize = 1_024;
const DEFAULT_EDGE_GRPC_PORT: u16 = 8001;
const DEFAULT_FORWARD_LIMIT: usize = 1;
const DEFAULT_BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_REQUESTS_PER_MINUTE: u64 = 600;
const DEFAULT_ACCESS_TOKEN_MINUTES: u64 = 15;
const DEFAULT_REFRESH_TOKEN_DAYS: u64 = 30;
const DEFAULT_SERVICE_ACCOUNT_LAST_USED_CAPACITY: usize = 10_000;
const DEFAULT_MFA_CHALLENGE_MINUTES: u64 = 5;
const DEFAULT_MFA_SETUP_MINUTES: u64 = 10;
const DEFAULT_MFA_MAXIMUM_FAILED_ATTEMPTS: i32 = 5;
const DEFAULT_MFA_RECOVERY_CODE_COUNT: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(
        "missing PostgreSQL configuration: set DATABASE_URL, ConnectionStrings__Postgres, or PG_USER/PG_PASSWORD/PG_DATABASE"
    )]
    MissingDatabase,
    #[error("invalid {name}: {message}")]
    Invalid { name: &'static str, message: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub updates_enabled: bool,
    pub execution: execution::ExecutionConfig,
    pub listen_address: SocketAddr,
    pub database_url: String,
    pub database_max_connections: u32,
    pub docker_socket: PathBuf,
    pub docker_request_timeout: Duration,
    pub probe_interval: Duration,
    pub reconciliation_interval: Duration,
    pub event_queue_capacity: usize,
    pub shutdown_timeout: Duration,
    pub agent: Option<AgentConfig>,
    pub realtime: Option<RealtimeConfig>,
    pub transport: TransportConfig,
    pub identity: IdentityConfig,
    pub automation: citadel_automation::AutomationOptions,
    pub node_agent_policy: citadel_platforms::node_agents::NodeAgentReconciliationPolicy,
    pub stats_flush_interval: Duration,
    pub retention_interval: Duration,
    pub stats_batch_size: usize,
    pub build_parallel_runs: usize,
    pub build_retention_days: Option<i32>,
    pub backup_workers: BackupWorkerConfig,
    database_source: &'static str,
}

#[derive(Debug, Clone)]
pub struct BackupWorkerConfig {
    pub enabled: bool,
    pub parallel_runs: usize,
    pub poll_interval: Duration,
    pub schedule_interval: Duration,
}

fn positive_usize(name: &'static str, default: usize) -> Result<usize, ConfigError> {
    let value = parse_env(name, default)?;
    if value == 0 {
        return Err(ConfigError::Invalid {
            name,
            message: "must be positive".into(),
        });
    }
    Ok(value)
}

#[derive(Clone)]
pub struct SecretBytes(Zeroizing<Vec<u8>>);

impl std::fmt::Debug for SecretBytes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl SecretBytes {
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct IdentityConfig {
    pub jwt_key_is_external: bool,
    pub encryption_key_is_external: bool,
    pub jwt_key: SecretBytes,
    pub secret_encryption_key: SecretBytes,
    pub issuer: String,
    pub audience: String,
    pub access_token_lifetime: Duration,
    pub refresh_token_lifetime: Duration,
    pub service_account_limits: citadel_identity::ServiceAccountLimitsDetails,
    pub service_account_last_used_interval: Duration,
    pub service_account_last_used_capacity: usize,
    pub mfa: MfaConfig,
    pub password_policy: citadel_identity::PasswordPolicy,
}

#[derive(Debug, Clone, Copy)]
pub struct MfaConfig {
    pub policy: MfaPolicy,
    pub challenge_lifetime: Duration,
    pub setup_lifetime: Duration,
    pub maximum_failed_attempts: i32,
    pub recovery_code_count: usize,
}

pub struct DatabaseConfig {
    pub database_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TransportMode {
    ReverseProxy,
    Direct,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub mode: TransportMode,
    pub public_url: Url,
    pub edge_agent_public_url: Url,
    pub edge_grpc_port: u16,
    pub allowed_hosts: Vec<String>,
    pub known_proxies: Vec<IpAddr>,
    pub known_networks: Vec<IpNet>,
    pub forward_limit: usize,
    pub certificate_path: Option<PathBuf>,
    pub certificate_private_key_path: Option<PathBuf>,
    pub cors_origins: Vec<String>,
    pub body_limit_bytes: usize,
    pub requests_per_minute: u64,
    pub openapi_enabled: bool,
    pub static_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub address: String,
    pub private_key_path: PathBuf,
    pub operation_timeout: Duration,
    pub allow_insecure: bool,
    pub reconnect_delay: Duration,
}

#[derive(Debug, Clone)]
pub struct RealtimeConfig {
    pub queue_capacity: usize,
    pub max_connections: usize,
    pub subscribe_timeout: Duration,
    pub send_timeout: Duration,
    pub authorization_recheck_interval: Duration,
    pub snapshot_limit: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveConfig {
    pub updates_enabled: bool,
    pub password_minimum_length: usize,
    pub service_account_limits: citadel_identity::ServiceAccountLimitsDetails,
    pub service_account_last_used_interval_seconds: u64,
    pub execution: serde_json::Value,
    pub listen_address: String,
    pub database_source: &'static str,
    pub database_host: String,
    pub database_port: u16,
    pub database_name: String,
    pub database_max_connections: u32,
    pub docker_socket: String,
    pub docker_request_timeout_seconds: u64,
    pub probe_interval_seconds: u64,
    pub reconciliation_interval_seconds: u64,
    pub retention_interval_seconds: u64,
    pub stats_flush_interval_seconds: u64,
    pub stats_batch_size: usize,
    pub stats_queue_capacity: usize,
    pub event_queue_capacity: usize,
    pub shutdown_timeout_seconds: u64,
    pub agent_configured: bool,
    pub agent_address: Option<String>,
    pub agent_allow_insecure: Option<bool>,
    pub agent_operation_timeout_seconds: Option<u64>,
    pub agent_reconnect_delay_seconds: Option<u64>,
    pub realtime_configured: bool,
    pub realtime_queue_capacity: Option<usize>,
    pub realtime_max_connections: Option<usize>,
    pub realtime_subscribe_timeout_seconds: Option<u64>,
    pub realtime_send_timeout_seconds: Option<u64>,
    pub realtime_authorization_recheck_seconds: Option<u64>,
    pub realtime_snapshot_limit: Option<usize>,
    pub transport_mode: TransportMode,
    pub public_url: String,
    pub edge_agent_public_url: String,
    pub edge_grpc_port: u16,
    pub allowed_hosts: Vec<String>,
    pub trusted_proxy_count: usize,
    pub trusted_network_count: usize,
    pub forward_limit: usize,
    pub certificate_configured: bool,
    pub cors_origins: Vec<String>,
    pub body_limit_bytes: usize,
    pub requests_per_minute: u64,
    pub openapi_enabled: bool,
    pub static_root_configured: bool,
    pub identity_issuer: String,
    pub identity_audience: String,
    pub access_token_lifetime_minutes: u64,
    pub refresh_token_lifetime_days: u64,
    pub jwt_key_configured: bool,
    pub secret_encryption_key_configured: bool,
    pub service_account_last_used_capacity: usize,
    pub mfa_policy: MfaPolicy,
    pub mfa_challenge_lifetime_minutes: u64,
    pub mfa_setup_lifetime_minutes: u64,
    pub mfa_maximum_failed_attempts: i32,
    pub mfa_recovery_code_count: usize,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let port = parse_env("Transport__ApiPort", DEFAULT_API_PORT)?;
        let transport = transport_config(port)?;
        let listen_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        let (database_url, database_source) = database_url()?;
        let database_max_connections = parse_env(
            "CITADEL_RUST_DB_MAX_CONNECTIONS",
            DEFAULT_DATABASE_MAX_CONNECTIONS,
        )?;
        if database_max_connections == 0 {
            return Err(ConfigError::Invalid {
                name: "CITADEL_RUST_DB_MAX_CONNECTIONS",
                message: "must be greater than zero".to_owned(),
            });
        }
        let event_queue_capacity = parse_env(
            "CITADEL_RUST_EVENT_QUEUE_CAPACITY",
            DEFAULT_EVENT_QUEUE_CAPACITY,
        )?;
        if event_queue_capacity == 0 {
            return Err(ConfigError::Invalid {
                name: "CITADEL_RUST_EVENT_QUEUE_CAPACITY",
                message: "must be greater than zero".to_owned(),
            });
        }
        let docker_request_timeout_seconds = nonzero_seconds(
            "CITADEL_RUST_DOCKER_TIMEOUT_SECONDS",
            DEFAULT_DOCKER_REQUEST_TIMEOUT_SECONDS,
        )?;
        let probe_interval_seconds = nonzero_seconds(
            "JobConfiguration__MonitoringInterval",
            DEFAULT_MONITORING_INTERVAL_SECONDS,
        )?;
        let reconciliation_interval_seconds = nonzero_seconds(
            "JobConfiguration__SwarmReconciliationIntervalSeconds",
            DEFAULT_RECONCILIATION_INTERVAL_SECONDS,
        )?;
        let shutdown_timeout_seconds = nonzero_seconds(
            "CITADEL_RUST_SHUTDOWN_TIMEOUT_SECONDS",
            DEFAULT_SHUTDOWN_TIMEOUT_SECONDS,
        )?;
        let execution = execution::ExecutionConfig::from_env()?;
        let agent = agent_config(execution.edge_agent.allow_insecure)?;
        let realtime = realtime_config()?;
        let identity = identity_config(&transport)?;

        Ok(Self {
            updates_enabled: parse_env("Updates__Enabled", true)?,
            execution,
            listen_address,
            database_url,
            database_max_connections,
            docker_socket: env::var_os("CITADEL_RUST_DOCKER_SOCKET")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/var/run/docker.sock")),
            docker_request_timeout: Duration::from_secs(docker_request_timeout_seconds),
            probe_interval: Duration::from_secs(probe_interval_seconds),
            reconciliation_interval: Duration::from_secs(reconciliation_interval_seconds),
            event_queue_capacity,
            shutdown_timeout: Duration::from_secs(shutdown_timeout_seconds),
            agent,
            realtime,
            transport,
            identity,
            automation: automation_options()?,
            node_agent_policy: citadel_platforms::node_agents::NodeAgentReconciliationPolicy {
                removal_grace: Duration::from_secs(
                    positive_usize("EdgeAgent__NodeAgentRemovalGraceMinutes", 10)? as u64 * 60,
                ),
                supported_architectures: {
                    let configured: Vec<_> = (0..32)
                        .filter_map(|index| {
                            env::var(format!("EdgeAgent__SupportedNodeArchitectures__{index}")).ok()
                        })
                        .collect();
                    if configured.is_empty() {
                        vec!["amd64".into(), "arm64".into()]
                    } else {
                        configured
                    }
                },
            },
            retention_interval: Duration::from_secs(nonzero_seconds(
                "CITADEL_RUST_RETENTION_INTERVAL_SECONDS",
                900,
            )?),
            stats_flush_interval: Duration::from_secs(positive_usize(
                "JobConfiguration__FlashInterval",
                60,
            )? as u64),
            stats_batch_size: positive_usize("JobConfiguration__BatchSize", 500)?,
            build_parallel_runs: positive_usize("Builds__MaxParallelRuns", 4)?,
            build_retention_days: if parse_env("Builds__RunCleanupEnabled", true)? {
                Some(
                    i32::try_from(positive_usize("Builds__RunRetentionDays", 90)?).map_err(
                        |_| ConfigError::Invalid {
                            name: "Builds__RunRetentionDays",
                            message: "must fit a positive 32-bit integer".into(),
                        },
                    )?,
                )
            } else {
                None
            },
            backup_workers: BackupWorkerConfig {
                enabled: parse_env("Backups__Enabled", true)?,
                parallel_runs: positive_usize("Backups__MaxParallelRuns", 2)?,
                poll_interval: Duration::from_secs(positive_usize(
                    "Backups__PollIntervalSeconds",
                    2,
                )? as u64),
                schedule_interval: Duration::from_secs(positive_usize(
                    "Backups__SchedulePollIntervalSeconds",
                    30,
                )? as u64),
            },
            database_source,
        })
    }

    pub fn effective(&self) -> Result<EffectiveConfig, ConfigError> {
        let database = Url::parse(&self.database_url).map_err(|error| ConfigError::Invalid {
            name: "database URL",
            message: error.to_string(),
        })?;
        Ok(EffectiveConfig {
            updates_enabled: self.updates_enabled,
            service_account_limits: self.identity.service_account_limits,
            service_account_last_used_interval_seconds: self
                .identity
                .service_account_last_used_interval
                .as_secs(),
            execution: serde_json::json!({
                "agentImage": self.execution.edge_agent.image,
                "agentCaConfigured": self.execution.edge_agent.ca_certificate.is_some(),
                "nodeAgent": {
                    "bootstrapSeconds": self.execution.edge_agent.setup_policy.bootstrap_lifetime.as_secs(),
                    "setupSeconds": self.execution.edge_agent.setup_policy.setup_timeout.as_secs(),
                    "architectures": self.execution.edge_agent.setup_policy.supported_architectures,
                    "nanoCpus": self.execution.edge_agent.setup_policy.limits.nano_cpus,
                    "memoryBytes": self.execution.edge_agent.setup_policy.limits.memory_bytes,
                    "pids": self.execution.edge_agent.setup_policy.limits.pids,
                },
                "backups": {
                    "enabled": self.backup_workers.enabled,
                    "workingDirectory": self.execution.backups.settings.working_directory,
                    "coreDataPath": self.execution.backups.core_data_path,
                    "allowedCorePaths": self.execution.backups.settings.allowed_core_paths,
                    "defaultTimeoutSeconds": self.execution.backups.settings.default_timeout.as_secs(),
                    "maximumLogLineBytes": self.execution.backups.settings.maximum_log_line_bytes,
                    "maximumLogBytes": self.execution.backups.maximum_log_bytes,
                    "repositoryLeaseSeconds": self.execution.backups.repository_lease_seconds,
                    "sourceLeaseSeconds": self.execution.backups.source_lease_seconds,
                },
                "buildRetentionDays": self.build_retention_days,
            }),
            listen_address: self.listen_address.to_string(),
            database_source: self.database_source,
            database_host: database.host_str().unwrap_or_default().to_owned(),
            database_port: database.port().unwrap_or(5432),
            database_name: database.path().trim_start_matches('/').to_owned(),
            database_max_connections: self.database_max_connections,
            docker_socket: self.docker_socket.display().to_string(),
            docker_request_timeout_seconds: self.docker_request_timeout.as_secs(),
            probe_interval_seconds: self.probe_interval.as_secs(),
            reconciliation_interval_seconds: self.reconciliation_interval.as_secs(),
            retention_interval_seconds: self.retention_interval.as_secs(),
            stats_flush_interval_seconds: self.stats_flush_interval.as_secs(),
            stats_batch_size: self.stats_batch_size,
            stats_queue_capacity: self.event_queue_capacity,
            event_queue_capacity: self.event_queue_capacity,
            shutdown_timeout_seconds: self.shutdown_timeout.as_secs(),
            agent_configured: self.agent.is_some(),
            agent_address: self.agent.as_ref().map(|agent| agent.address.clone()),
            agent_allow_insecure: self.agent.as_ref().map(|agent| agent.allow_insecure),
            agent_operation_timeout_seconds: self
                .agent
                .as_ref()
                .map(|agent| agent.operation_timeout.as_secs()),
            agent_reconnect_delay_seconds: self
                .agent
                .as_ref()
                .map(|agent| agent.reconnect_delay.as_secs()),
            realtime_configured: self.realtime.is_some(),
            realtime_queue_capacity: self
                .realtime
                .as_ref()
                .map(|realtime| realtime.queue_capacity),
            realtime_max_connections: self
                .realtime
                .as_ref()
                .map(|realtime| realtime.max_connections),
            realtime_subscribe_timeout_seconds: self
                .realtime
                .as_ref()
                .map(|realtime| realtime.subscribe_timeout.as_secs()),
            realtime_send_timeout_seconds: self
                .realtime
                .as_ref()
                .map(|realtime| realtime.send_timeout.as_secs()),
            realtime_authorization_recheck_seconds: self
                .realtime
                .as_ref()
                .map(|realtime| realtime.authorization_recheck_interval.as_secs()),
            realtime_snapshot_limit: self
                .realtime
                .as_ref()
                .map(|realtime| realtime.snapshot_limit),
            transport_mode: self.transport.mode,
            public_url: self.transport.public_url.to_string(),
            edge_agent_public_url: self.transport.edge_agent_public_url.to_string(),
            edge_grpc_port: self.transport.edge_grpc_port,
            allowed_hosts: self.transport.allowed_hosts.clone(),
            trusted_proxy_count: self.transport.known_proxies.len(),
            trusted_network_count: self.transport.known_networks.len(),
            forward_limit: self.transport.forward_limit,
            certificate_configured: self.transport.certificate_path.is_some(),
            cors_origins: self.transport.cors_origins.clone(),
            body_limit_bytes: self.transport.body_limit_bytes,
            requests_per_minute: self.transport.requests_per_minute,
            openapi_enabled: self.transport.openapi_enabled,
            static_root_configured: self.transport.static_root.is_some(),
            identity_issuer: self.identity.issuer.clone(),
            identity_audience: self.identity.audience.clone(),
            access_token_lifetime_minutes: self.identity.access_token_lifetime.as_secs() / 60,
            refresh_token_lifetime_days: self.identity.refresh_token_lifetime.as_secs() / 86_400,
            jwt_key_configured: true,
            secret_encryption_key_configured: true,
            service_account_last_used_capacity: self.identity.service_account_last_used_capacity,
            password_minimum_length: self.identity.password_policy.minimum_length(),
            mfa_policy: self.identity.mfa.policy,
            mfa_challenge_lifetime_minutes: self.identity.mfa.challenge_lifetime.as_secs() / 60,
            mfa_setup_lifetime_minutes: self.identity.mfa.setup_lifetime.as_secs() / 60,
            mfa_maximum_failed_attempts: self.identity.mfa.maximum_failed_attempts,
            mfa_recovery_code_count: self.identity.mfa.recovery_code_count,
        })
    }
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let (database_url, _) = database_url()?;
        Ok(Self { database_url })
    }
}

fn transport_config(api_port: u16) -> Result<TransportConfig, ConfigError> {
    let mode = parse_transport_mode(env::var("Transport__Mode").ok().as_deref())?;
    let edge_grpc_port = parse_env("Transport__EdgeGrpcPort", DEFAULT_EDGE_GRPC_PORT)?;
    if api_port == edge_grpc_port {
        return Err(ConfigError::Invalid {
            name: "Transport__EdgeGrpcPort",
            message: "must differ from Transport__ApiPort".to_owned(),
        });
    }

    let public_url = parse_origin(
        "Transport__PublicUrl",
        env::var("Transport__PublicUrl").ok(),
        mode,
        format!("http://localhost:{api_port}"),
    )?;
    let edge_agent_public_url = parse_origin(
        "EdgeAgent__PublicGrpcUrl",
        env::var("EdgeAgent__PublicGrpcUrl").ok(),
        mode,
        format!("http://localhost:{edge_grpc_port}"),
    )?;
    let allowed_hosts = split_values(env::var("AllowedHosts").ok().as_deref(), ';')?;
    if mode != TransportMode::Disabled {
        if allowed_hosts.is_empty() || allowed_hosts.iter().any(|host| host == "*") {
            return Err(ConfigError::Invalid {
                name: "AllowedHosts",
                message: "must explicitly include the configured public hosts".to_owned(),
            });
        }
        for host in [public_url.host_str(), edge_agent_public_url.host_str()]
            .into_iter()
            .flatten()
        {
            if !allowed_hosts
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(host))
            {
                return Err(ConfigError::Invalid {
                    name: "AllowedHosts",
                    message: format!("does not include configured public host '{host}'"),
                });
            }
        }
    }

    let known_proxies = split_values(
        env::var("Transport__ForwardedHeaders__KnownProxies")
            .ok()
            .as_deref(),
        ',',
    )?
    .into_iter()
    .map(|value| {
        value
            .parse()
            .map_err(|error: std::net::AddrParseError| ConfigError::Invalid {
                name: "Transport__ForwardedHeaders__KnownProxies",
                message: error.to_string(),
            })
    })
    .collect::<Result<Vec<_>, _>>()?;
    let known_networks = split_values(
        env::var("Transport__ForwardedHeaders__KnownNetworks")
            .ok()
            .as_deref(),
        ',',
    )?
    .into_iter()
    .map(|value| {
        value
            .parse::<IpNet>()
            .map_err(|error| ConfigError::Invalid {
                name: "Transport__ForwardedHeaders__KnownNetworks",
                message: error.to_string(),
            })
    })
    .collect::<Result<Vec<_>, _>>()?;
    let forwarded_configured = !known_proxies.is_empty() || !known_networks.is_empty();
    let certificate_path = env::var_os("Transport__Certificate__Path").map(PathBuf::from);
    let certificate_private_key_path =
        env::var_os("Transport__Certificate__PrivateKeyPath").map(PathBuf::from);
    let certificate_configured =
        certificate_path.is_some() || certificate_private_key_path.is_some();

    match mode {
        TransportMode::ReverseProxy if !forwarded_configured => {
            return Err(ConfigError::Invalid {
                name: "Transport__ForwardedHeaders",
                message: "ReverseProxy mode requires a known proxy or network".to_owned(),
            });
        }
        TransportMode::ReverseProxy if certificate_configured => {
            return Err(ConfigError::Invalid {
                name: "Transport__Certificate",
                message: "certificate settings are not valid in ReverseProxy mode".to_owned(),
            });
        }
        TransportMode::Direct if forwarded_configured => {
            return Err(ConfigError::Invalid {
                name: "Transport__ForwardedHeaders",
                message: "forwarded-header settings are not valid in Direct mode".to_owned(),
            });
        }
        TransportMode::Direct
            if certificate_path.is_none() || certificate_private_key_path.is_none() =>
        {
            return Err(ConfigError::Invalid {
                name: "Transport__Certificate",
                message: "Direct mode requires a PEM certificate and private key".to_owned(),
            });
        }
        TransportMode::Direct
            if !certificate_path.as_ref().is_some_and(|path| path.is_file())
                || !certificate_private_key_path
                    .as_ref()
                    .is_some_and(|path| path.is_file()) =>
        {
            return Err(ConfigError::Invalid {
                name: "Transport__Certificate",
                message: "certificate and private-key paths must identify readable files"
                    .to_owned(),
            });
        }
        TransportMode::Disabled if forwarded_configured || certificate_configured => {
            return Err(ConfigError::Invalid {
                name: "Transport",
                message: "Disabled mode cannot configure certificates or forwarded headers"
                    .to_owned(),
            });
        }
        _ => {}
    }

    let forward_limit = parse_env(
        "Transport__ForwardedHeaders__ForwardLimit",
        DEFAULT_FORWARD_LIMIT,
    )?;
    let body_limit_bytes = parse_env("CITADEL_RUST_BODY_LIMIT_BYTES", DEFAULT_BODY_LIMIT_BYTES)?;
    let requests_per_minute = parse_env(
        "CITADEL_RUST_REQUESTS_PER_MINUTE",
        DEFAULT_REQUESTS_PER_MINUTE,
    )?;
    if forward_limit == 0 || body_limit_bytes == 0 || requests_per_minute == 0 {
        return Err(ConfigError::Invalid {
            name: "transport numeric limits",
            message: "must be greater than zero".to_owned(),
        });
    }
    if requests_per_minute > u64::from(u32::MAX) {
        return Err(ConfigError::Invalid {
            name: "CITADEL_RUST_REQUESTS_PER_MINUTE",
            message: format!("must not exceed {}", u32::MAX),
        });
    }

    let cors_origins = indexed_env("Cors__")?;
    for origin in &cors_origins {
        parse_origin("Cors__N", Some(origin.clone()), mode, String::new())?;
    }

    Ok(TransportConfig {
        mode,
        public_url,
        edge_agent_public_url,
        edge_grpc_port,
        allowed_hosts,
        known_proxies,
        known_networks,
        forward_limit,
        certificate_path,
        certificate_private_key_path,
        cors_origins,
        body_limit_bytes,
        requests_per_minute,
        openapi_enabled: parse_env("EnableSwagger", false)?,
        static_root: env::var_os("CITADEL_RUST_STATIC_ROOT").map(PathBuf::from),
    })
}

fn parse_transport_mode(value: Option<&str>) -> Result<TransportMode, ConfigError> {
    match value.map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("reverseproxy") => {
            Ok(TransportMode::ReverseProxy)
        }
        Some(value) if value.eq_ignore_ascii_case("direct") => Ok(TransportMode::Direct),
        Some(value) if value.eq_ignore_ascii_case("disabled") => Ok(TransportMode::Disabled),
        _ => Err(ConfigError::Invalid {
            name: "Transport__Mode",
            message: "must be ReverseProxy, Direct, or Disabled".to_owned(),
        }),
    }
}

fn identity_config(transport: &TransportConfig) -> Result<IdentityConfig, ConfigError> {
    let (jwt_key, secret_encryption_key) = identity_keys_from_env()?;
    identity_config_with_keys(transport, jwt_key, secret_encryption_key)
}

/// Load explicit keys or persist generated defaults for normal startup.
pub fn identity_keys_from_env() -> Result<(SecretBytes, SecretBytes), ConfigError> {
    let root = env::var_os("CITADEL_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| "/app/data".into());
    let jwt_key = match env::var("Jwt__Key").ok().filter(|v| !v.trim().is_empty()) {
        Some(value) => Zeroizing::new(value),
        None => keys::load_or_create(&root, "jwtsecret", "Jwt__Key")?,
    };
    if jwt_key.len() < 32 {
        return Err(ConfigError::Invalid {
            name: "Jwt__Key",
            message: "must contain at least 32 bytes".to_owned(),
        });
    }
    let encrypted_secrets_key = match env::var("Secrets__EncryptionKey")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        Some(value) => Zeroizing::new(value),
        None => keys::load_or_create(&root, "secret-encryption-key", "Secrets__EncryptionKey")?,
    };
    Ok((
        SecretBytes(Zeroizing::new(jwt_key.as_bytes().to_vec())),
        decode_encryption_key(&encrypted_secrets_key)?,
    ))
}

/// Offline recovery must never generate a replacement for the original encryption key.
pub fn recovery_encryption_key_from_env() -> Result<SecretBytes, ConfigError> {
    let root = env::var_os("CITADEL_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| "/app/data".into());
    let encoded = match env::var("Secrets__EncryptionKey").ok().filter(|v| !v.trim().is_empty()) {
        Some(value) => Zeroizing::new(value),
        None => Zeroizing::new(std::fs::read_to_string(root.join("secret-encryption-key"))
            .map_err(|_| ConfigError::Invalid { name: "Secrets__EncryptionKey", message: "restore requires the original key: configure it explicitly or restore secret-encryption-key into CITADEL_DATA_ROOT".into() })?),
    };
    decode_encryption_key(&encoded)
}

fn decode_encryption_key(encoded: &str) -> Result<SecretBytes, ConfigError> {
    let bytes =
        Zeroizing::new(
            STANDARD
                .decode(encoded.trim())
                .map_err(|_| ConfigError::Invalid {
                    name: "Secrets__EncryptionKey",
                    message: "must be a base64-encoded 32-byte key".into(),
                })?,
        );
    if bytes.len() != 32 {
        return Err(ConfigError::Invalid {
            name: "Secrets__EncryptionKey",
            message: "must decode to exactly 32 bytes".into(),
        });
    }
    Ok(SecretBytes(bytes))
}

fn identity_config_with_keys(
    transport: &TransportConfig,
    jwt_key: SecretBytes,
    secret_encryption_key: SecretBytes,
) -> Result<IdentityConfig, ConfigError> {
    let issuer = env::var("Jwt__Issuer").unwrap_or_else(|_| {
        transport
            .public_url
            .as_str()
            .trim_end_matches('/')
            .to_owned()
    });
    let audience = env::var("Jwt__Audience").unwrap_or_else(|_| issuer.clone());
    if issuer.trim().is_empty() || audience.trim().is_empty() {
        return Err(ConfigError::Invalid {
            name: "Jwt__Issuer/Jwt__Audience",
            message: "must not be empty".to_owned(),
        });
    }
    let access_token_minutes = nonzero_seconds(
        "Jwt__AccessToken__ValidForMinutes",
        DEFAULT_ACCESS_TOKEN_MINUTES,
    )?;
    let refresh_token_days = nonzero_seconds(
        "Jwt__RefreshToken__ValidForDays",
        DEFAULT_REFRESH_TOKEN_DAYS,
    )?;
    let service_account_last_used_capacity = parse_env(
        "ServiceAccounts__LastUsedTrackingCapacity",
        DEFAULT_SERVICE_ACCOUNT_LAST_USED_CAPACITY,
    )?;
    if service_account_last_used_capacity == 0 {
        return Err(ConfigError::Invalid {
            name: "ServiceAccounts__LastUsedTrackingCapacity",
            message: "must be greater than zero".to_owned(),
        });
    }
    let password_policy = citadel_identity::PasswordPolicy::new(parse_env(
        "Passwords__MinimumLength",
        citadel_identity::MINIMUM_PASSWORD_CHARACTERS,
    )?)
    .map_err(|error| ConfigError::Invalid {
        name: "Passwords__MinimumLength",
        message: error.to_string(),
    })?;
    let mfa = mfa_config()?;
    Ok(IdentityConfig {
        jwt_key_is_external: env::var("Jwt__Key").is_ok_and(|v| !v.trim().is_empty()),
        encryption_key_is_external: env::var("Secrets__EncryptionKey")
            .is_ok_and(|v| !v.trim().is_empty()),
        jwt_key,
        secret_encryption_key,
        issuer,
        audience,
        access_token_lifetime: Duration::from_secs(access_token_minutes.checked_mul(60).ok_or(
            ConfigError::Invalid {
                name: "Jwt__AccessToken__ValidForMinutes",
                message: "is too large".to_owned(),
            },
        )?),
        refresh_token_lifetime: Duration::from_secs(refresh_token_days.checked_mul(86_400).ok_or(
            ConfigError::Invalid {
                name: "Jwt__RefreshToken__ValidForDays",
                message: "is too large".to_owned(),
            },
        )?),
        service_account_limits: service_account_limits()?,
        service_account_last_used_interval: Duration::from_secs(
            nonzero_seconds("ServiceAccounts__LastUsedWriteIntervalMinutes", 5)?
                .checked_mul(60)
                .ok_or(ConfigError::Invalid {
                    name: "ServiceAccounts__LastUsedWriteIntervalMinutes",
                    message: "is too large".into(),
                })?,
        ),
        service_account_last_used_capacity,
        mfa,
        password_policy,
    })
}

fn mfa_config() -> Result<MfaConfig, ConfigError> {
    let policy_value = env::var("Mfa__Policy").unwrap_or_else(|_| "Optional".to_owned());
    let policy = MfaPolicy::ALL
        .iter()
        .copied()
        .find(|candidate| {
            candidate
                .as_database_str()
                .eq_ignore_ascii_case(policy_value.trim())
        })
        .ok_or_else(|| ConfigError::Invalid {
            name: "Mfa__Policy",
            message: "must be Optional, RequiredForAdministrators, or RequiredForAllUsers"
                .to_owned(),
        })?;
    let challenge_minutes = nonzero_seconds(
        "Mfa__ChallengeLifetimeMinutes",
        DEFAULT_MFA_CHALLENGE_MINUTES,
    )?;
    let setup_minutes = nonzero_seconds("Mfa__SetupLifetimeMinutes", DEFAULT_MFA_SETUP_MINUTES)?;
    let attempts_setting = if env::var_os("Mfa__MaxFailedAttempts").is_some() {
        "Mfa__MaxFailedAttempts"
    } else {
        "Mfa__MaximumFailedAttempts" // Historical Rust alias.
    };
    let maximum_failed_attempts = parse_env(attempts_setting, DEFAULT_MFA_MAXIMUM_FAILED_ATTEMPTS)?;
    if maximum_failed_attempts <= 0 {
        return Err(ConfigError::Invalid {
            name: attempts_setting,
            message: "must be greater than zero".to_owned(),
        });
    }
    let recovery_code_count = parse_env("Mfa__RecoveryCodeCount", DEFAULT_MFA_RECOVERY_CODE_COUNT)?;
    if recovery_code_count == 0 || recovery_code_count > 100 {
        return Err(ConfigError::Invalid {
            name: "Mfa__RecoveryCodeCount",
            message: "must be between 1 and 100".to_owned(),
        });
    }
    Ok(MfaConfig {
        policy,
        challenge_lifetime: Duration::from_secs(challenge_minutes.checked_mul(60).ok_or(
            ConfigError::Invalid {
                name: "Mfa__ChallengeLifetimeMinutes",
                message: "is too large".to_owned(),
            },
        )?),
        setup_lifetime: Duration::from_secs(setup_minutes.checked_mul(60).ok_or(
            ConfigError::Invalid {
                name: "Mfa__SetupLifetimeMinutes",
                message: "is too large".to_owned(),
            },
        )?),
        maximum_failed_attempts,
        recovery_code_count,
    })
}

fn parse_origin(
    name: &'static str,
    value: Option<String>,
    mode: TransportMode,
    disabled_fallback: String,
) -> Result<Url, ConfigError> {
    let candidate = match value.filter(|value| !value.trim().is_empty()) {
        Some(value) => value,
        None if mode == TransportMode::Disabled => disabled_fallback,
        None => {
            return Err(ConfigError::Invalid {
                name,
                message: "must be configured".to_owned(),
            });
        }
    };
    let url = Url::parse(candidate.trim()).map_err(|error| ConfigError::Invalid {
        name,
        message: error.to_string(),
    })?;
    let expected_scheme = if mode == TransportMode::Disabled {
        "http"
    } else {
        "https"
    };
    if url.scheme() != expected_scheme
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfigError::Invalid {
            name,
            message: format!(
                "must be a {expected_scheme} origin without credentials, path, query, or fragment"
            ),
        });
    }
    Ok(url)
}

fn split_values(value: Option<&str>, separator: char) -> Result<Vec<String>, ConfigError> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    let entries = value
        .split(separator)
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if entries.iter().any(String::is_empty) {
        return Err(ConfigError::Invalid {
            name: "transport list",
            message: "cannot contain empty entries".to_owned(),
        });
    }
    Ok(entries)
}

fn indexed_env(prefix: &str) -> Result<Vec<String>, ConfigError> {
    let mut entries = env::vars()
        .filter_map(|(name, value)| {
            name.strip_prefix(prefix)
                .and_then(|index| index.parse::<usize>().ok())
                .map(|index| (index, value))
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|(index, _)| *index);
    if entries.iter().any(|(_, value)| value.trim().is_empty()) {
        return Err(ConfigError::Invalid {
            name: "Cors__N",
            message: "origins cannot be empty".to_owned(),
        });
    }
    Ok(entries.into_iter().map(|(_, value)| value).collect())
}

fn realtime_config() -> Result<Option<RealtimeConfig>, ConfigError> {
    if !parse_env("CITADEL_RUST_REALTIME_ENABLED", true)? {
        return Ok(None);
    }
    let queue_capacity = parse_env(
        "CITADEL_RUST_REALTIME_QUEUE_CAPACITY",
        DEFAULT_REALTIME_QUEUE_CAPACITY,
    )?;
    let snapshot_limit = parse_env(
        "CITADEL_RUST_REALTIME_SNAPSHOT_LIMIT",
        DEFAULT_REALTIME_SNAPSHOT_LIMIT,
    )?;
    let max_connections = parse_env(
        "CITADEL_RUST_REALTIME_MAX_CONNECTIONS",
        DEFAULT_REALTIME_MAX_CONNECTIONS,
    )?;
    if queue_capacity == 0 || snapshot_limit == 0 || max_connections == 0 {
        return Err(ConfigError::Invalid {
            name: "CITADEL_RUST_REALTIME_QUEUE_CAPACITY/CITADEL_RUST_REALTIME_MAX_CONNECTIONS/CITADEL_RUST_REALTIME_SNAPSHOT_LIMIT",
            message: "values must be greater than zero".to_owned(),
        });
    }
    Ok(Some(RealtimeConfig {
        queue_capacity,
        max_connections,
        subscribe_timeout: Duration::from_secs(nonzero_seconds(
            "CITADEL_RUST_REALTIME_SUBSCRIBE_TIMEOUT_SECONDS",
            DEFAULT_REALTIME_SUBSCRIBE_TIMEOUT_SECONDS,
        )?),
        send_timeout: Duration::from_secs(nonzero_seconds(
            "CITADEL_RUST_REALTIME_SEND_TIMEOUT_SECONDS",
            DEFAULT_REALTIME_SEND_TIMEOUT_SECONDS,
        )?),
        authorization_recheck_interval: Duration::from_secs(nonzero_seconds(
            "CITADEL_RUST_REALTIME_AUTH_RECHECK_SECONDS",
            DEFAULT_REALTIME_AUTH_RECHECK_SECONDS,
        )?),
        snapshot_limit,
    }))
}

fn agent_config(allow_insecure: bool) -> Result<Option<AgentConfig>, ConfigError> {
    let address = env::var("CITADEL_RUST_AGENT_ADDRESS").ok();
    let private_key_path = env::var_os("CITADEL_RUST_AGENT_PRIVATE_KEY_PATH");
    match (address, private_key_path) {
        (None, None) => Ok(None),
        (Some(address), Some(private_key_path)) if !address.trim().is_empty() => {
            let timeout = required_nonzero_seconds("CITADEL_RUST_AGENT_TIMEOUT_SECONDS")?;
            Ok(Some(AgentConfig {
                address,
                private_key_path: PathBuf::from(private_key_path),
                operation_timeout: Duration::from_secs(timeout),
                allow_insecure,
                reconnect_delay: Duration::from_secs(DEFAULT_MONITORING_INTERVAL_SECONDS),
            }))
        }
        _ => Err(ConfigError::Invalid {
            name: "CITADEL_RUST_AGENT_ADDRESS/CITADEL_RUST_AGENT_PRIVATE_KEY_PATH",
            message: "both values must be set together and the address must not be empty"
                .to_owned(),
        }),
    }
}

fn database_url() -> Result<(String, &'static str), ConfigError> {
    if let Ok(value) = env::var("DATABASE_URL") {
        validate_database_url(&value)?;
        return Ok((value, "DATABASE_URL"));
    }
    if let Ok(value) = env::var("ConnectionStrings__Postgres") {
        return Ok((
            dotnet_connection_string_to_url(&value)?,
            "ConnectionStrings__Postgres",
        ));
    }

    let user = env::var("PG_USER").map_err(|_| ConfigError::MissingDatabase)?;
    let password = env::var("PG_PASSWORD").map_err(|_| ConfigError::MissingDatabase)?;
    let database = env::var("PG_DATABASE").map_err(|_| ConfigError::MissingDatabase)?;
    let host = env::var("PG_HOST").unwrap_or_else(|_| "pg_db".to_owned());
    let port = parse_env("PG_PORT", 5432_u16)?;
    Ok((
        build_database_url(&host, port, &database, &user, &password)?,
        "PG_*",
    ))
}

fn dotnet_connection_string_to_url(value: &str) -> Result<String, ConfigError> {
    let values = value
        .split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect::<HashMap<_, _>>();
    let required = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| values.get(*key))
            .cloned()
            .ok_or(ConfigError::MissingDatabase)
    };
    let host = required(&["host", "server"])?;
    let database = required(&["database", "initial catalog"])?;
    let user = required(&["username", "user id", "user"])?;
    let password = required(&["password"])?;
    let port = values.get("port").map_or(Ok(5432), |value| {
        value
            .parse()
            .map_err(|error: std::num::ParseIntError| ConfigError::Invalid {
                name: "ConnectionStrings__Postgres Port",
                message: error.to_string(),
            })
    })?;
    build_database_url(&host, port, &database, &user, &password)
}

fn build_database_url(
    host: &str,
    port: u16,
    database: &str,
    user: &str,
    password: &str,
) -> Result<String, ConfigError> {
    let mut url = Url::parse("postgres://localhost").expect("static PostgreSQL URL is valid");
    url.set_host(Some(host)).map_err(|_| ConfigError::Invalid {
        name: "PostgreSQL host",
        message: "invalid host".to_owned(),
    })?;
    url.set_port(Some(port)).map_err(|_| ConfigError::Invalid {
        name: "PostgreSQL port",
        message: "invalid port".to_owned(),
    })?;
    url.set_username(user).map_err(|_| ConfigError::Invalid {
        name: "PostgreSQL username",
        message: "invalid username".to_owned(),
    })?;
    url.set_password(Some(password))
        .map_err(|_| ConfigError::Invalid {
            name: "PostgreSQL password",
            message: "invalid password".to_owned(),
        })?;
    url.set_path(database);
    Ok(url.into())
}

fn validate_database_url(value: &str) -> Result<(), ConfigError> {
    let url = Url::parse(value).map_err(|error| ConfigError::Invalid {
        name: "DATABASE_URL",
        message: error.to_string(),
    })?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        return Err(ConfigError::Invalid {
            name: "DATABASE_URL",
            message: "scheme must be postgres or postgresql".to_owned(),
        });
    }
    Ok(())
}

fn service_account_limits() -> Result<citadel_identity::ServiceAccountLimitsDetails, ConfigError> {
    let limits = citadel_identity::ServiceAccountLimitsDetails {
        default_token_lifetime_days: parse_env("ServiceAccounts__DefaultTokenLifetimeDays", 90)?,
        maximum_token_lifetime_days: parse_env("ServiceAccounts__MaximumTokenLifetimeDays", 365)?,
        maximum_active_tokens_per_account: parse_env(
            "ServiceAccounts__MaximumActiveTokensPerAccount",
            10,
        )?,
    };
    for (name, valid) in [
        (
            "ServiceAccounts__DefaultTokenLifetimeDays",
            limits.default_token_lifetime_days > 0,
        ),
        (
            "ServiceAccounts__MaximumTokenLifetimeDays",
            limits.maximum_token_lifetime_days >= limits.default_token_lifetime_days
                && limits.maximum_token_lifetime_days <= 365_000,
        ),
        (
            "ServiceAccounts__MaximumActiveTokensPerAccount",
            limits.maximum_active_tokens_per_account > 0,
        ),
    ] {
        if !valid {
            return Err(ConfigError::Invalid { name, message: "limits must be positive, maximum lifetime must be at least the default and no more than 365000 days".into() });
        }
    }
    Ok(limits)
}

fn automation_options() -> Result<citadel_automation::AutomationOptions, ConfigError> {
    let options = citadel_automation::AutomationOptions {
        enabled: parse_env("Automations__Enabled", true)?,
        max_parallel_runs: parse_env("Automations__MaxParallelRuns", 4)?,
        default_timeout_seconds: parse_env("Automations__DefaultTimeoutSeconds", 300)?,
        max_timeout_seconds: parse_env("Automations__MaxTimeoutSeconds", 1800)?,
        poll_interval: Duration::from_secs(parse_env("Automations__PollIntervalSeconds", 2)?),
        schedule_poll_interval: Duration::from_secs(parse_env(
            "Automations__SchedulePollIntervalSeconds",
            30,
        )?),
    };
    options.validate().map_err(|error| ConfigError::Invalid {
        name: "Automations",
        message: error.to_string(),
    })?;
    Ok(options)
}

fn parse_env<T>(name: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    env::var(name).map_or(Ok(default), |value| {
        value.parse().map_err(|error: T::Err| ConfigError::Invalid {
            name,
            message: error.to_string(),
        })
    })
}

fn nonzero_seconds(name: &'static str, default: u64) -> Result<u64, ConfigError> {
    let value = parse_env(name, default)?;
    if value == 0 {
        return Err(ConfigError::Invalid {
            name,
            message: "must be greater than zero".to_owned(),
        });
    }
    Ok(value)
}

fn required_nonzero_seconds(name: &'static str) -> Result<u64, ConfigError> {
    let value = env::var(name).map_err(|_| ConfigError::Invalid {
        name,
        message: "must be set when the Agent transport is configured".to_owned(),
    })?;
    let value = value.parse::<u64>().map_err(|error| ConfigError::Invalid {
        name,
        message: error.to_string(),
    })?;
    if value == 0 {
        return Err(ConfigError::Invalid {
            name,
            message: "must be greater than zero".to_owned(),
        });
    }
    Ok(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

impl std::str::FromStr for LogFormat {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err("must be text or json"),
        }
    }
}

pub fn log_format_from_env() -> Result<LogFormat, ConfigError> {
    parse_env("LogFormat", LogFormat::default())
}

/// Color is independent of format and defaults to interactive terminals only.
pub fn log_color_from_env() -> Result<bool, ConfigError> {
    use std::io::IsTerminal;
    parse_env("EnableLogColor", std::io::stdout().is_terminal())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotnet_connection_string_without_exposing_password() {
        let url = dotnet_connection_string_to_url(
            "Host=pg_db;Database=citadel;Username=app;Password=a b#c;Port=5433",
        )
        .unwrap();
        let parsed = Url::parse(&url).unwrap();
        assert_eq!(parsed.host_str(), Some("pg_db"));
        assert_eq!(parsed.port(), Some(5433));
        assert_eq!(parsed.path(), "/citadel");
        assert_eq!(parsed.password(), Some("a%20b%23c"));
    }

    #[test]
    fn rejects_zero_duration_settings() {
        assert!(matches!(
            nonzero_seconds("fixture", 0),
            Err(ConfigError::Invalid {
                name: "fixture",
                ..
            })
        ));
    }

    #[test]
    fn transport_mode_is_explicit_and_case_insensitive() {
        assert_eq!(
            parse_transport_mode(Some("reverseProxy")).unwrap(),
            TransportMode::ReverseProxy
        );
        assert!(parse_transport_mode(None).is_err());
        assert!(parse_transport_mode(Some("0")).is_err());
    }

    #[test]
    fn secure_transport_requires_https_origins() {
        assert!(
            parse_origin(
                "fixture",
                Some("http://citadel.example.com".to_owned()),
                TransportMode::Direct,
                String::new(),
            )
            .is_err()
        );
        assert_eq!(
            parse_origin(
                "fixture",
                Some("https://citadel.example.com".to_owned()),
                TransportMode::Direct,
                String::new(),
            )
            .unwrap()
            .host_str(),
            Some("citadel.example.com")
        );
    }
}
