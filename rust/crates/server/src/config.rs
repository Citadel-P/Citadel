use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use url::Url;

const DEFAULT_API_PORT: u16 = 8000;
const DEFAULT_MONITORING_INTERVAL_SECONDS: u64 = 10;
const DEFAULT_EVENT_QUEUE_CAPACITY: usize = 256;
const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_DOCKER_REQUEST_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_SHUTDOWN_TIMEOUT_SECONDS: u64 = 10;

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
    database_source: &'static str,
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
        })
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
