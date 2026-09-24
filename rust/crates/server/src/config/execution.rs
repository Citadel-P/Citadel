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
    pub setup_policy: citadel_platforms::node_agents::setup::NodeAgentSetupPolicy,
    pub allow_insecure: bool,
    pub node_agent_ca_bundle: Option<Arc<[u8]>>,
    pub ca_certificate: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone)]
pub struct BackupExecutionConfig {
    pub settings: citadel_adapters::external::backups::settings::BackupExecutionSettings,
    pub core_data_path: PathBuf,
    pub maximum_log_bytes: usize,
    pub repository_lease_seconds: i32,
    pub source_lease_seconds: i32,
}

#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub backups: BackupExecutionConfig,
    pub paths: PathsConfig,
    pub tools: ExternalToolsConfig,
    pub automation: AutomationExecutionConfig,
    pub edge_agent: EdgeAgentConfig,
    pub restic_image: String,
    pub volume_helper_image: Option<String>,
    pub core_container_hostname: String,
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
        let bounded =
            |key: &'static str, default: i64, min: i64, max: i64| -> Result<i64, ConfigError> {
                let value = string(key)
                    .map(|s| s.parse::<i64>())
                    .transpose()
                    .map_err(|e| invalid(key, e))?
                    .unwrap_or(default);
                if !(min..=max).contains(&value) {
                    return Err(invalid(key, format!("must be between {min} and {max}")));
                }
                Ok(value)
            };
        let architectures: Vec<_> = (0..32)
            .filter_map(|i| string(&format!("EdgeAgent__SupportedNodeArchitectures__{i}")))
            .collect();
        let architectures = if architectures.is_empty() {
            vec!["amd64".into(), "arm64".into()]
        } else {
            architectures
        };
        if architectures
            .iter()
            .any(|s| !matches!(s.as_str(), "amd64" | "arm64"))
        {
            return Err(invalid(
                "EdgeAgent__SupportedNodeArchitectures",
                "supported values are amd64 and arm64",
            ));
        }
        let setup_policy = citadel_platforms::node_agents::setup::NodeAgentSetupPolicy {
            bootstrap_lifetime: std::time::Duration::from_secs(
                bounded("EdgeAgent__NodeAgentBootstrapMinutes", 10, 1, 30)? as u64 * 60,
            ),
            setup_timeout: std::time::Duration::from_secs(
                bounded("EdgeAgent__NodeAgentSetupMinutes", 5, 1, 20)? as u64 * 60,
            ),
            supported_architectures: architectures,
            limits: citadel_platforms::node_agents::setup::NodeAgentLimits {
                nano_cpus: bounded(
                    "EdgeAgent__NodeAgentLimitNanoCpus",
                    500_000_000,
                    100_000_000,
                    2_000_000_000,
                )?,
                memory_bytes: bounded(
                    "EdgeAgent__NodeAgentLimitMemoryBytes",
                    536_870_912,
                    134_217_728,
                    2_147_483_648,
                )?,
                pids: bounded("EdgeAgent__NodeAgentPidsLimit", 256, 64, 4096)?,
            },
        };
        let image = if let Some(image) = string("CITADEL_EDGE_AGENT_IMAGE") {
            image
        } else {
            let repository = string("EdgeAgent__AgentImageRepository")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "ghcr.io/citadel-p/citadel.agent".into());
            let version = string("EdgeAgent__AgentImageTag")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| env!("CITADEL_BUILD_VERSION").into());
            format!("{}:{}", repository.trim(), image_tag(&version))
        };
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
        let allowed_core_paths: Vec<_> = (0..128)
            .filter_map(|i| get(&format!("Backups__AllowedCorePaths__{i}")).map(PathBuf::from))
            .collect();
        let backups = BackupExecutionConfig {
            settings: citadel_adapters::external::backups::settings::BackupExecutionSettings {
                working_directory: absolute(
                    "Backups__WorkingDirectory",
                    data_root.join("backups/work"),
                )?,
                allowed_core_paths: if allowed_core_paths.is_empty() {
                    vec![data_root.join("backups/repositories")]
                } else {
                    allowed_core_paths
                },
                default_timeout: std::time::Duration::from_secs(bounded(
                    "Backups__DefaultTimeoutSeconds",
                    120,
                    5,
                    86400,
                )? as u64),
                maximum_log_line_bytes: bounded(
                    "Backups__MaxLogLineBytes",
                    8192,
                    1024,
                    16 * 1024 * 1024,
                )? as usize,
            },
            core_data_path: absolute("Backups__CoreDataPath", data_root.clone())?,
            maximum_log_bytes: bounded("Backups__MaxLogBytes", 1_048_576, 1024, 64 * 1024 * 1024)?
                as usize,
            repository_lease_seconds: bounded("Backups__RepositoryLeaseSeconds", 300, 30, 604800)?
                as i32,
            source_lease_seconds: bounded("Backups__SourceLeaseSeconds", 300, 30, 604800)? as i32,
        };
        Ok(Self {
            backups,
            tools: ExternalToolsConfig {
                docker: get("CITADEL_DOCKER_PATH").unwrap_or_else(|| "docker".into()),
                restic: get("Backups__ResticPath")
                    .or_else(|| get("CITADEL_RESTIC_PATH"))
                    .unwrap_or_else(|| "restic".into()),
                pg_dump: get("Backups__PostgresDumpPath")
                    .or_else(|| get("CITADEL_PG_DUMP_PATH"))
                    .unwrap_or_else(|| "pg_dump".into()),
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
                    .unwrap_or_else(|| {
                        format!(
                            "http://127.0.0.1:{}",
                            string("Transport__ApiPort").unwrap_or_else(|| "8000".into())
                        )
                    }),
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
                image,
                setup_policy,
                allow_insecure,
                node_agent_ca_bundle,
                ca_certificate: get("AgentTransport__CaCertificatePath")
                    .filter(|p| !p.is_empty())
                    .map(|path| read_agent_ca(PathBuf::from(path)))
                    .transpose()?,
            },
            restic_image: string("CITADEL_RESTIC_IMAGE")
                .unwrap_or_else(|| "restic/restic:0.18.1".into()),
            volume_helper_image: string("CITADEL_VOLUME_HELPER_IMAGE"),
            core_container_hostname: string("HOSTNAME").unwrap_or_default(),
        })
    }
}

fn image_tag(value: &str) -> String {
    let value = value.trim().split('+').next().unwrap_or_default();
    let value = if value.len() > 1
        && matches!(value.as_bytes()[0], b'v' | b'V')
        && value.as_bytes()[1].is_ascii_digit()
    {
        &value[1..]
    } else {
        value
    };
    let tag: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let tag = tag.trim_matches(['.', '-']);
    if tag.is_empty() {
        "latest".into()
    } else {
        tag.into()
    }
}

fn invalid(name: &'static str, error: impl std::fmt::Display) -> ConfigError {
    ConfigError::Invalid {
        name,
        message: error.to_string(),
    }
}

fn read_agent_ca(path: PathBuf) -> Result<Arc<[u8]>, ConfigError> {
    const NAME: &str = "AgentTransport__CaCertificatePath";
    let data = read_ca_bundle(path)
        .map_err(|_| invalid(NAME, "cannot read a CA bundle between 1 byte and 1 MiB"))?;
    let certificates = reqwest::Certificate::from_pem_bundle(&data)
        .map_err(|_| invalid(NAME, "must contain PEM certificates"))?;
    if certificates.is_empty() {
        return Err(invalid(NAME, "must contain PEM certificates"));
    }
    Ok(data)
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
        assert!(config.volume_helper_image.is_none());
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
