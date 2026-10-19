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
    pub git_known_hosts: PathBuf,
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
    pub(super) fn from_env(http_port: u16) -> Result<Self, ConfigError> {
        Self::read(|key| std::env::var_os(key), http_port)
    }

    // An injected source keeps configuration tests independent of process-global env.
    fn read(get: impl Fn(&str) -> Option<OsString>, http_port: u16) -> Result<Self, ConfigError> {
        let string = |key: &str| get(key).and_then(|value| value.into_string().ok());
        let data_root =
            PathBuf::from(get("CITADEL_DATA_ROOT").unwrap_or_else(|| "/app/data".into()));
        let maximum_log_bytes = string("CITADEL_AUTOMATIONS_MAX_LOG_BYTES")
            .map(|value| value.parse::<usize>())
            .transpose()
            .map_err(|error| invalid("CITADEL_AUTOMATIONS_MAX_LOG_BYTES", error))?
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
            .filter_map(|i| {
                string(&format!(
                    "CITADEL_EDGE_AGENT_SUPPORTED_NODE_ARCHITECTURE_{i}"
                ))
            })
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
                "CITADEL_EDGE_AGENT_SUPPORTED_NODE_ARCHITECTURE_N",
                "supported values are amd64 and arm64",
            ));
        }
        let setup_policy = citadel_platforms::node_agents::setup::NodeAgentSetupPolicy {
            bootstrap_lifetime: std::time::Duration::from_secs(
                bounded("CITADEL_EDGE_AGENT_NODE_AGENT_BOOTSTRAP_MINUTES", 10, 1, 30)? as u64 * 60,
            ),
            setup_timeout: std::time::Duration::from_secs(
                bounded("CITADEL_EDGE_AGENT_NODE_AGENT_SETUP_MINUTES", 5, 1, 20)? as u64 * 60,
            ),
            supported_architectures: architectures,
            limits: citadel_platforms::node_agents::setup::NodeAgentLimits {
                nano_cpus: bounded(
                    "CITADEL_EDGE_AGENT_NODE_AGENT_LIMIT_NANO_CPUS",
                    500_000_000,
                    100_000_000,
                    2_000_000_000,
                )?,
                memory_bytes: bounded(
                    "CITADEL_EDGE_AGENT_NODE_AGENT_LIMIT_MEMORY_BYTES",
                    536_870_912,
                    134_217_728,
                    2_147_483_648,
                )?,
                pids: bounded("CITADEL_EDGE_AGENT_NODE_AGENT_PIDS_LIMIT", 256, 64, 4096)?,
            },
        };
        let image = if let Some(image) = string("CITADEL_EDGE_AGENT_IMAGE") {
            image
        } else {
            let repository = string("CITADEL_EDGE_AGENT_IMAGE_REPOSITORY")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "ghcr.io/citadel-p/citadel.agent".into());
            let version = string("CITADEL_EDGE_AGENT_IMAGE_TAG")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| env!("CITADEL_BUILD_VERSION").into());
            format!("{}:{}", repository.trim(), image_tag(&version))
        };
        let allow_insecure = string("CITADEL_CORE_AGENT_ALLOW_INSECURE")
            .map(|value| value.parse::<bool>())
            .transpose()
            .map_err(|error| invalid("CITADEL_CORE_AGENT_ALLOW_INSECURE", error))?
            .unwrap_or(true);
        let absolute = |name, default| {
            std::path::absolute(get(name).map(PathBuf::from).unwrap_or(default))
                .map_err(|error| invalid(name, error))
        };
        let node_agent_ca_bundle = get("CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH")
            .map(|path| read_ca_bundle(PathBuf::from(path)))
            .transpose()?;
        let allowed_core_paths: Vec<_> = (0..128)
            .filter_map(|i| {
                get(&format!("CITADEL_BACKUPS_ALLOWED_CORE_PATH_{i}")).map(PathBuf::from)
            })
            .collect();
        if allowed_core_paths.iter().any(|p: &PathBuf| {
            p.as_os_str().is_empty() || p.to_str().is_some_and(|s| s.trim().is_empty())
        }) {
            return Err(invalid(
                "CITADEL_BACKUPS_ALLOWED_CORE_PATH_N",
                "paths cannot be empty",
            ));
        }
        let backups = BackupExecutionConfig {
            settings: citadel_adapters::external::backups::settings::BackupExecutionSettings {
                working_directory: absolute(
                    "CITADEL_BACKUPS_WORKING_DIRECTORY",
                    data_root.join("backups/work"),
                )?,
                allowed_core_paths: if allowed_core_paths.is_empty() {
                    vec![data_root.join("backups/repositories")]
                } else {
                    allowed_core_paths
                },
                default_timeout: std::time::Duration::from_secs(bounded(
                    "CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS",
                    120,
                    5,
                    86400,
                )? as u64),
                maximum_log_line_bytes: bounded(
                    "CITADEL_BACKUPS_MAX_LOG_LINE_BYTES",
                    8192,
                    1024,
                    16 * 1024 * 1024,
                )? as usize,
            },
            core_data_path: absolute("CITADEL_BACKUPS_CORE_DATA_PATH", data_root.clone())?,
            maximum_log_bytes: bounded(
                "CITADEL_BACKUPS_MAX_LOG_BYTES",
                1_048_576,
                1024,
                64 * 1024 * 1024,
            )? as usize,
            repository_lease_seconds: bounded(
                "CITADEL_BACKUPS_REPOSITORY_LEASE_SECONDS",
                300,
                30,
                604800,
            )? as i32,
            source_lease_seconds: bounded("CITADEL_BACKUPS_SOURCE_LEASE_SECONDS", 300, 30, 604800)?
                as i32,
        };
        Ok(Self {
            git_known_hosts: absolute(
                "CITADEL_GIT_KNOWN_HOSTS_PATH",
                data_root.join("git-known-hosts"),
            )?,
            backups,
            tools: ExternalToolsConfig {
                docker: get("CITADEL_DOCKER_PATH").unwrap_or_else(|| "docker".into()),
                restic: get("CITADEL_RESTIC_PATH").unwrap_or_else(|| "restic".into()),
                pg_dump: get("CITADEL_PG_DUMP_PATH").unwrap_or_else(|| "pg_dump".into()),
                shoutrrr: get("CITADEL_SHOUTRRR_PATH").unwrap_or_else(|| "shoutrrr".into()),
            },
            automation: AutomationExecutionConfig {
                deno: get("CITADEL_DENO_PATH").unwrap_or_else(|| "deno".into()),
                work_root: absolute(
                    "CITADEL_AUTOMATIONS_WORK_DIR",
                    data_root.join("automations/runs"),
                )?,
                cache_root: absolute(
                    "CITADEL_AUTOMATIONS_DENO_CACHE_DIR",
                    data_root.join("automations/deno-cache"),
                )?,
                internal_base_url: string("CITADEL_INTERNAL_BASE_URL")
                    .unwrap_or_else(|| format!("http://127.0.0.1:{http_port}")),
                maximum_log_bytes,
                allow_net: string("CITADEL_AUTOMATIONS_ALLOW_NET"),
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
                ca_certificate: get("CITADEL_CORE_AGENT_CA_CERTIFICATE_PATH")
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
    const NAME: &str = "CITADEL_CORE_AGENT_CA_CERTIFICATE_PATH";
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
        ExecutionConfig::read(
            |key| {
                values
                    .iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, value)| (*value).into())
            },
            8000,
        )
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
    fn explicit_tools_and_execution_values_reach_consumers() {
        let values = [
            ("CITADEL_DENO_PATH", "/primary/deno"),
            ("CITADEL_INTERNAL_BASE_URL", "http://primary:8000"),
            ("CITADEL_RESTIC_PATH", "/tools/restic"),
            ("CITADEL_PG_DUMP_PATH", "/tools/pg_dump"),
            ("CITADEL_DATA_ROOT", "/data"),
            ("CITADEL_AUTOMATIONS_MAX_LOG_BYTES", "1"),
            ("CITADEL_CORE_AGENT_ALLOW_INSECURE", "false"),
        ];
        let config = parse(&values).unwrap();
        assert_eq!(config.automation.deno, OsString::from("/primary/deno"));
        assert_eq!(config.tools.restic, OsString::from("/tools/restic"));
        assert_eq!(config.tools.pg_dump, OsString::from("/tools/pg_dump"));
        assert_eq!(config.automation.internal_base_url, "http://primary:8000");
        assert_eq!(
            config.automation.cache_root,
            PathBuf::from("/data/automations/deno-cache")
        );
        assert_eq!(config.automation.maximum_log_bytes, 1024);
        assert!(!config.edge_agent.allow_insecure);
        assert_eq!(
            parse(&[("CITADEL_AUTOMATIONS_MAX_LOG_BYTES", "999999999")])
                .unwrap()
                .automation
                .maximum_log_bytes,
            16 * 1024 * 1024
        );
    }

    #[test]
    fn malformed_settings_fail_at_configuration_boundary() {
        for (key, value) in [
            ("CITADEL_AUTOMATIONS_MAX_LOG_BYTES", "invalid"),
            ("CITADEL_CORE_AGENT_ALLOW_INSECURE", "yes"),
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
        let config = ExecutionConfig::read(
            |key| {
                (key == "CITADEL_NODE_AGENT_CA_CERTIFICATE_PATH")
                    .then(|| path.clone().into_os_string())
            },
            8000,
        )
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
