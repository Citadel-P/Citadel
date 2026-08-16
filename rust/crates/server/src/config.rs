use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

const DEFAULT_API_PORT: u16 = 8000;
const DEFAULT_MONITORING_INTERVAL_SECONDS: u64 = 10;
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
    pub listen_address: SocketAddr,
    pub database_url: String,
    pub database_max_connections: u32,
    pub docker_socket: PathBuf,
    pub docker_request_timeout: Duration,
    pub probe_interval: Duration,
    pub event_queue_capacity: usize,
    pub shutdown_timeout: Duration,
    pub agent: Option<AgentConfig>,
    pub realtime: Option<RealtimeConfig>,
    database_source: &'static str,
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
    pub actor_id: Uuid,
    pub platform_id: Uuid,
    pub token_hash: [u8; 32],
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
    pub listen_address: String,
    pub database_source: &'static str,
    pub database_host: String,
    pub database_port: u16,
    pub database_name: String,
    pub database_max_connections: u32,
    pub docker_socket: String,
    pub docker_request_timeout_seconds: u64,
    pub probe_interval_seconds: u64,
    pub event_queue_capacity: usize,
    pub shutdown_timeout_seconds: u64,
    pub agent_configured: bool,
    pub agent_address: Option<String>,
    pub agent_allow_insecure: Option<bool>,
    pub agent_operation_timeout_seconds: Option<u64>,
    pub agent_reconnect_delay_seconds: Option<u64>,
    pub realtime_configured: bool,
    pub realtime_actor_id: Option<Uuid>,
    pub realtime_platform_id: Option<Uuid>,
    pub realtime_queue_capacity: Option<usize>,
    pub realtime_max_connections: Option<usize>,
    pub realtime_subscribe_timeout_seconds: Option<u64>,
    pub realtime_send_timeout_seconds: Option<u64>,
    pub realtime_authorization_recheck_seconds: Option<u64>,
    pub realtime_snapshot_limit: Option<usize>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let port = parse_env("Transport__ApiPort", DEFAULT_API_PORT)?;
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
        let shutdown_timeout_seconds = nonzero_seconds(
            "CITADEL_RUST_SHUTDOWN_TIMEOUT_SECONDS",
            DEFAULT_SHUTDOWN_TIMEOUT_SECONDS,
        )?;
        let agent = agent_config()?;
        let realtime = realtime_config()?;

        Ok(Self {
            listen_address,
            database_url,
            database_max_connections,
            docker_socket: env::var_os("CITADEL_RUST_DOCKER_SOCKET")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/var/run/docker.sock")),
            docker_request_timeout: Duration::from_secs(docker_request_timeout_seconds),
            probe_interval: Duration::from_secs(probe_interval_seconds),
            event_queue_capacity,
            shutdown_timeout: Duration::from_secs(shutdown_timeout_seconds),
            agent,
            realtime,
            database_source,
        })
    }

    pub fn effective(&self) -> Result<EffectiveConfig, ConfigError> {
        let database = Url::parse(&self.database_url).map_err(|error| ConfigError::Invalid {
            name: "database URL",
            message: error.to_string(),
        })?;
        Ok(EffectiveConfig {
            listen_address: self.listen_address.to_string(),
            database_source: self.database_source,
            database_host: database.host_str().unwrap_or_default().to_owned(),
            database_port: database.port().unwrap_or(5432),
            database_name: database.path().trim_start_matches('/').to_owned(),
            database_max_connections: self.database_max_connections,
            docker_socket: self.docker_socket.display().to_string(),
            docker_request_timeout_seconds: self.docker_request_timeout.as_secs(),
            probe_interval_seconds: self.probe_interval.as_secs(),
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
            realtime_actor_id: self.realtime.as_ref().map(|realtime| realtime.actor_id),
            realtime_platform_id: self.realtime.as_ref().map(|realtime| realtime.platform_id),
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
        })
    }
}

fn realtime_config() -> Result<Option<RealtimeConfig>, ConfigError> {
    let actor_id = env::var("CITADEL_RUST_REALTIME_ACTOR_ID").ok();
    let platform_id = env::var("CITADEL_RUST_REALTIME_PLATFORM_ID").ok();
    let token = env::var("CITADEL_RUST_REALTIME_TOKEN").ok();
    match (actor_id, platform_id, token) {
        (None, None, None) => Ok(None),
        (Some(actor_id), Some(platform_id), Some(token)) => {
            if token.len() < 32 {
                return Err(ConfigError::Invalid {
                    name: "CITADEL_RUST_REALTIME_TOKEN",
                    message: "must contain at least 32 characters".to_owned(),
                });
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
                actor_id: parse_uuid("CITADEL_RUST_REALTIME_ACTOR_ID", &actor_id)?,
                platform_id: parse_uuid("CITADEL_RUST_REALTIME_PLATFORM_ID", &platform_id)?,
                token_hash: Sha256::digest(token.as_bytes()).into(),
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
        _ => Err(ConfigError::Invalid {
            name: "CITADEL_RUST_REALTIME_ACTOR_ID/CITADEL_RUST_REALTIME_PLATFORM_ID/CITADEL_RUST_REALTIME_TOKEN",
            message: "all three values must be set together".to_owned(),
        }),
    }
}

fn parse_uuid(name: &'static str, value: &str) -> Result<Uuid, ConfigError> {
    Uuid::parse_str(value).map_err(|error| ConfigError::Invalid {
        name,
        message: error.to_string(),
    })
}

fn agent_config() -> Result<Option<AgentConfig>, ConfigError> {
    let address = env::var("CITADEL_RUST_AGENT_ADDRESS").ok();
    let private_key_path = env::var_os("CITADEL_RUST_AGENT_PRIVATE_KEY_PATH");
    match (address, private_key_path) {
        (None, None) => Ok(None),
        (Some(address), Some(private_key_path)) if !address.trim().is_empty() => {
            let timeout = required_nonzero_seconds("CITADEL_RUST_AGENT_TIMEOUT_SECONDS")?;
            let allow_insecure = parse_env("AgentTransport__AllowInsecure", true)?;
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
}
