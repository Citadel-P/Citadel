//! Startup settings for external tools and workload execution.
use super::ConfigError;
use std::{ffi::OsString, io::Read, path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub struct PathsConfig {
    pub data_root: PathBuf,
    pub host_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ExternalToolsConfig {
    pub docker: OsString,
    pub restic: OsString,
    pub pg_dump: OsString,
    pub shoutrrr: OsString,
}

#[derive(Debug, Clone)]
pub struct AutomationExecutionConfig {
    pub deno: OsString,
    pub work_root: PathBuf,
    pub cache_root: PathBuf,
    pub internal_base_url: String,
    pub maximum_log_bytes: usize,
    pub allow_net: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EdgeAgentConfig {
    pub image: String,
    pub allow_insecure: bool,
    pub node_agent_ca_bundle: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub paths: PathsConfig,
    pub tools: ExternalToolsConfig,
    pub automation: AutomationExecutionConfig,
    pub edge_agent: EdgeAgentConfig,
    pub restic_image: String,
    pub volume_helper_image: String,
}

impl ExecutionConfig {
    pub(super) fn from_env() -> Result<Self, ConfigError> {
        Self::read(|key| std::env::var_os(key))
    }

    // An injected source keeps alias/default tests independent of process-global env.
    fn read(get: impl Fn(&str) -> Option<OsString>) -> Result<Self, ConfigError> {
        let string = |key: &str| get(key).and_then(|value| value.into_string().ok());
        let data_root =
            PathBuf::from(get("CITADEL_DATA_ROOT").unwrap_or_else(|| "/app/data".into()));
        let maximum_log_bytes = string("Automations__MaxLogBytes")
            .map(|value| value.parse::<usize>())
            .transpose()
            .map_err(|error| invalid("Automations__MaxLogBytes", error))?
            .unwrap_or(1024 * 1024)
            .clamp(1024, 16 * 1024 * 1024);
        let allow_insecure = string("AgentTransport__AllowInsecure")
            .map(|value| value.parse::<bool>())
            .transpose()
            .map_err(|error| invalid("AgentTransport__AllowInsecure", error))?
            .unwrap_or(true);
        let absolute = |name, default| {
            std::path::absolute(get(name).map(PathBuf::from).unwrap_or(default))
                .map_err(|error| invalid(name, error))
        };
        let node_agent_ca_bundle = get("CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH")
            .map(|path| read_ca_bundle(PathBuf::from(path)))
            .transpose()?;
        Ok(Self {
            tools: ExternalToolsConfig {
                docker: get("CITADEL_DOCKER_PATH").unwrap_or_else(|| "docker".into()),
                restic: get("CITADEL_RESTIC_PATH").unwrap_or_else(|| "restic".into()),
                pg_dump: get("CITADEL_PG_DUMP_PATH").unwrap_or_else(|| "pg_dump".into()),
                shoutrrr: get("CITADEL_SHOUTRRR_PATH").unwrap_or_else(|| "shoutrrr".into()),
            },
            automation: AutomationExecutionConfig {
                deno: get("Automations__DenoPath")
                    .or_else(|| get("CITADEL_DENO_PATH"))
                    .unwrap_or_else(|| "deno".into()),
                work_root: absolute("Automations__WorkDir", data_root.join("automations/runs"))?,
                cache_root: absolute(
                    "Automations__DenoCacheDir",
                    data_root.join("automations/deno-cache"),
                )?,
                internal_base_url: string("Automations__InternalBaseUrl")
                    .or_else(|| string("CITADEL_INTERNAL_BASE_URL"))
                    .unwrap_or_else(|| "http://127.0.0.1:8000".into()),
                maximum_log_bytes,
                allow_net: string("Automations__AllowNet"),
            },
            paths: PathsConfig {
                data_root,
                host_root: get("CITADEL_HOST_ROOT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| "/host".into()),
            },
            edge_agent: EdgeAgentConfig {
                image: string("CITADEL_EDGE_AGENT_IMAGE")
                    .unwrap_or_else(|| "ghcr.io/citadel-p/citadel.agent:latest".into()),
                allow_insecure,
                node_agent_ca_bundle,
            },
            restic_image: string("CITADEL_RESTIC_IMAGE")
                .unwrap_or_else(|| "restic/restic:0.18.1".into()),
            volume_helper_image: string("CITADEL_VOLUME_HELPER_IMAGE")
                .unwrap_or_else(|| "ghcr.io/citadel-p/citadel.agent:latest".into()),
        })
    }
}

fn invalid(name: &'static str, error: impl std::fmt::Display) -> ConfigError {
    ConfigError::Invalid {
        name,
        message: error.to_string(),
    }
}

fn read_ca_bundle(path: PathBuf) -> Result<Arc<[u8]>, ConfigError> {
    const NAME: &str = "CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH";
    let mut data = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| invalid(NAME, error))?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|error| invalid(NAME, error))?;
    if data.is_empty() || data.len() > 1024 * 1024 {
        return Err(invalid(
            NAME,
            "Node-agent CA bundle must be between 1 byte and 1 MiB",
        ));
    }
    Ok(data.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(values: &[(&str, &str)]) -> Result<ExecutionConfig, ConfigError> {
        ExecutionConfig::read(|key| {
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).into())
        })
    }

    #[test]
    fn defaults_preserve_execution_contract() {
        let config = parse(&[]).unwrap();
        assert_eq!(config.paths.data_root, PathBuf::from("/app/data"));
        assert_eq!(config.paths.host_root, PathBuf::from("/host"));
        assert_eq!(
            config.automation.work_root,
            PathBuf::from("/app/data/automations/runs")
        );
        assert_eq!(config.automation.internal_base_url, "http://127.0.0.1:8000");
        assert_eq!(config.automation.maximum_log_bytes, 1024 * 1024);
        assert_eq!(config.tools.restic, OsString::from("restic"));
        assert_eq!(config.restic_image, "restic/restic:0.18.1");
        assert!(config.edge_agent.allow_insecure);
        assert!(config.edge_agent.node_agent_ca_bundle.is_none());
    }

    #[test]
    fn aliases_and_primary_values_keep_their_precedence() {
        let aliases = [
            ("CITADEL_DENO_PATH", "/tools/deno"),
            ("CITADEL_INTERNAL_BASE_URL", "http://core:8000"),
        ];
        let config = parse(&aliases).unwrap();
        assert_eq!(config.automation.deno, OsString::from("/tools/deno"));
        assert_eq!(config.automation.internal_base_url, "http://core:8000");
        let mut values = aliases.to_vec();
        values.extend([
            ("Automations__DenoPath", "/primary/deno"),
            ("Automations__InternalBaseUrl", "http://primary:8000"),
            ("CITADEL_DATA_ROOT", "/data"),
            ("Automations__MaxLogBytes", "1"),
            ("AgentTransport__AllowInsecure", "false"),
        ]);
        let config = parse(&values).unwrap();
        assert_eq!(config.automation.deno, OsString::from("/primary/deno"));
        assert_eq!(config.automation.internal_base_url, "http://primary:8000");
        assert_eq!(
            config.automation.cache_root,
            PathBuf::from("/data/automations/deno-cache")
        );
        assert_eq!(config.automation.maximum_log_bytes, 1024);
        assert!(!config.edge_agent.allow_insecure);
        assert_eq!(
            parse(&[("Automations__MaxLogBytes", "999999999")])
                .unwrap()
                .automation
                .maximum_log_bytes,
            16 * 1024 * 1024
        );
    }

    #[test]
    fn malformed_settings_fail_at_configuration_boundary() {
        for (key, value) in [
            ("Automations__MaxLogBytes", "invalid"),
            ("AgentTransport__AllowInsecure", "yes"),
            (
                "CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH",
                "/missing-citadel-ca",
            ),
        ] {
            assert!(
                matches!(parse(&[(key,value)]), Err(ConfigError::Invalid { name, .. }) if name == key)
            );
        }
    }

    #[test]
    fn ca_bundle_is_bounded_and_loaded_before_services() {
        let path = std::env::temp_dir().join(format!("citadel-ca-{}", uuid::Uuid::now_v7()));
        std::fs::write(&path, b"certificate").unwrap();
        let config = ExecutionConfig::read(|key| {
            (key == "CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH").then(|| path.clone().into_os_string())
        })
        .unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            config.edge_agent.node_agent_ca_bundle.as_deref(),
            Some(b"certificate".as_slice())
        );
        for bytes in [vec![], vec![0; 1024 * 1024 + 1]] {
            std::fs::write(&path, bytes).unwrap();
            assert!(read_ca_bundle(path.clone()).is_err());
        }
        std::fs::remove_file(path).unwrap();
    }
}
