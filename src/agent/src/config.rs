//! Process settings are read once, before composition or listener creation.
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::VerifyingKey;
use url::Url;
use zeroize::Zeroizing;

pub struct AgentConfig {
    pub mode: AgentMode,
    pub port: u16,
    pub docker_endpoint: DockerEndpoint,
    pub host_root: PathBuf,
    pub runtime_container: Option<String>,
}

pub enum AgentMode {
    Direct(DirectConfig),
    Edge(EdgeConfig),
}

pub struct DirectConfig {
    pub hub_public_key: VerifyingKey,
    pub tls: Option<DirectTls>,
}

pub struct DirectTls {
    pub certificate_path: PathBuf,
    pub private_key_path: PathBuf,
}

#[derive(Clone)]
pub struct EdgeConfig {
    pub core_url: Url,
    pub profile: EdgeProfile,
    pub enrollment_token: Option<Zeroizing<String>>,
    pub key_path: PathBuf,
    pub identity_path: PathBuf,
    pub core_ca_path: Option<PathBuf>,
}

#[derive(Clone)]
pub enum EdgeProfile {
    Ordinary,
    BuildPool,
    SwarmNode(SwarmIdentity),
}

#[derive(Clone)]
pub struct SwarmIdentity {
    pub bootstrap_file: PathBuf,
    pub platform_id: String,
    pub service_id: String,
    pub task_id: String,
    pub node_id: String,
    pub node_hostname: String,
    pub cluster_id: String,
}

pub use citadel_adapters::connectors::docker::DockerEndpoint;

#[derive(Debug, thiserror::Error)]
#[error("{variable}: {reason}")]
pub struct ConfigError {
    pub variable: &'static str,
    pub reason: &'static str,
}

fn invalid(variable: &'static str, reason: &'static str) -> ConfigError {
    ConfigError { variable, reason }
}

impl AgentConfig {
    pub fn from_environment() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup(
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let port = optional(&mut lookup, "CITADEL_AGENT_PORT")
            .map(|value| value.parse::<u16>().ok().filter(|port| *port > 0))
            .unwrap_or(Some(9000))
            .ok_or_else(|| {
                invalid(
                    "CITADEL_AGENT_PORT",
                    "must be an integer between 1 and 65535",
                )
            })?;
        let mode = if lookup("CITADEL_AGENT_MODE")
            .is_some_and(|value| value.eq_ignore_ascii_case("edge"))
        {
            AgentMode::Edge(EdgeConfig::parse(&mut lookup)?)
        } else {
            AgentMode::Direct(DirectConfig::parse(&mut lookup)?)
        };
        let docker_endpoint = optional(&mut lookup, "DOCKER_HOST")
            .unwrap_or_else(|| "unix:///var/run/docker.sock".into())
            .parse::<DockerEndpoint>()
            .map_err(|_| {
                invalid(
                    "DOCKER_HOST",
                    "must be unix:///path, tcp://host:port or http://host:port",
                )
            })?;
        let host_root = path(&mut lookup, "CITADEL_HOST_ROOT", "/host")?;
        Ok(Self {
            mode,
            port,
            docker_endpoint,
            host_root,
            runtime_container: optional(&mut lookup, "HOSTNAME"),
        })
    }

    pub fn listen_address(&self) -> SocketAddr {
        let ip = match &self.mode {
            AgentMode::Direct(_) => Ipv4Addr::UNSPECIFIED,
            AgentMode::Edge(_) => Ipv4Addr::LOCALHOST,
        };
        SocketAddr::new(IpAddr::V4(ip), self.port)
    }
}

impl DirectConfig {
    fn parse(lookup: &mut impl FnMut(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let key = required(lookup, "HUB_PUBLIC_KEY")?;
        let bytes = STANDARD
            .decode(key)
            .ok()
            .and_then(|value| <[u8; 32]>::try_from(value).ok())
            .ok_or_else(|| {
                invalid(
                    "HUB_PUBLIC_KEY",
                    "must be a base64 encoded 32-byte Ed25519 public key",
                )
            })?;
        let hub_public_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| invalid("HUB_PUBLIC_KEY", "must be a valid Ed25519 public key"))?;
        let mode = optional(lookup, "CITADEL_AGENT_TLS_MODE").unwrap_or_else(|| "Disabled".into());
        let certificate = optional(lookup, "CITADEL_AGENT_TLS_CERTIFICATE_PATH");
        let private_key = optional(lookup, "CITADEL_AGENT_TLS_PRIVATE_KEY_PATH");
        let tls = if mode.eq_ignore_ascii_case("Disabled") {
            if certificate.is_some() || private_key.is_some() {
                return Err(invalid(
                    "CITADEL_AGENT_TLS_MODE",
                    "certificate settings require Direct TLS",
                ));
            }
            None
        } else if mode.eq_ignore_ascii_case("Direct") {
            Some(DirectTls {
                certificate_path: certificate
                    .ok_or_else(|| {
                        invalid(
                            "CITADEL_AGENT_TLS_CERTIFICATE_PATH",
                            "is required for Direct TLS",
                        )
                    })?
                    .into(),
                private_key_path: private_key
                    .ok_or_else(|| {
                        invalid(
                            "CITADEL_AGENT_TLS_PRIVATE_KEY_PATH",
                            "is required for Direct TLS",
                        )
                    })?
                    .into(),
            })
        } else {
            return Err(invalid(
                "CITADEL_AGENT_TLS_MODE",
                "must be Direct or Disabled",
            ));
        };
        Ok(Self {
            hub_public_key,
            tls,
        })
    }
}

impl EdgeConfig {
    fn parse(lookup: &mut impl FnMut(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let raw_url = required(lookup, "CITADEL_CORE_URL")?;
        let core_url = Url::parse(&raw_url)
            .map_err(|_| invalid("CITADEL_CORE_URL", "must be an HTTP(S) origin"))?;
        if !matches!(core_url.scheme(), "http" | "https")
            || core_url.host_str().is_none()
            || !core_url.username().is_empty()
            || core_url.password().is_some()
            || core_url.path() != "/"
            || core_url.query().is_some()
            || core_url.fragment().is_some()
        {
            return Err(invalid(
                "CITADEL_CORE_URL",
                "must be an HTTP(S) origin without credentials, path, query or fragment",
            ));
        }
        let core_ca_path =
            optional(lookup, "CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH").map(PathBuf::from);
        if core_ca_path.is_some() && core_url.scheme() != "https" {
            return Err(invalid(
                "CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH",
                "requires an HTTPS Core URL",
            ));
        }
        let profile =
            optional(lookup, "CITADEL_EDGE_AGENT_PROFILE").unwrap_or_else(|| "edge-agent".into());
        let profile = match profile.to_ascii_lowercase().as_str() {
            "edge-agent" => EdgeProfile::Ordinary,
            "edge-build-agent" => EdgeProfile::BuildPool,
            "swarm-node" => EdgeProfile::SwarmNode(SwarmIdentity {
                bootstrap_file: required(lookup, "CITADEL_EDGE_BOOTSTRAP_FILE")?.into(),
                platform_id: required(lookup, "CITADEL_PLATFORM_ID")?,
                service_id: required(lookup, "CITADEL_SWARM_SERVICE_ID")?,
                task_id: required(lookup, "CITADEL_SWARM_TASK_ID")?,
                node_id: required(lookup, "CITADEL_SWARM_NODE_ID")?,
                node_hostname: required(lookup, "CITADEL_SWARM_NODE_HOSTNAME")?,
                cluster_id: required(lookup, "CITADEL_SWARM_CLUSTER_ID")?,
            }),
            _ => {
                return Err(invalid(
                    "CITADEL_EDGE_AGENT_PROFILE",
                    "must be edge-agent, edge-build-agent or swarm-node",
                ));
            }
        };
        Ok(Self {
            core_url,
            profile,
            core_ca_path,
            enrollment_token: optional(lookup, "CITADEL_EDGE_ENROLLMENT_TOKEN").map(Zeroizing::new),
            key_path: path(
                lookup,
                "CITADEL_EDGE_AGENT_KEY_PATH",
                "/app/data/edge-agent.key",
            )?,
            identity_path: path(
                lookup,
                "CITADEL_EDGE_IDENTITY_PATH",
                "/app/data/edge-agent.identity.json",
            )?,
        })
    }
}

fn optional(lookup: &mut impl FnMut(&str) -> Option<String>, name: &str) -> Option<String> {
    lookup(name)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn required(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    optional(lookup, name).ok_or_else(|| invalid(name, "is required"))
}

fn path(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &'static str,
    default: &str,
) -> Result<PathBuf, ConfigError> {
    let value = lookup(name).unwrap_or_else(|| default.into());
    if value.trim().is_empty() {
        return Err(invalid(name, "must not be empty"));
    }
    Ok(value.into())
}
