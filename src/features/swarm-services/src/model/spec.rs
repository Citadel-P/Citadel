use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use serde_json::Value;

use sha2::{Digest, Sha256};

use uuid::Uuid;

pub use citadel_primitives::UpdateBehavior;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulingMode {
    #[default]
    Replicated,
    Global,
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PortPublishMode {
    #[default]
    Ingress,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MountKind {
    Volume,
    Bind,
    Tmpfs,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartCondition {
    None,
    OnFailure,
    #[default]
    Any,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateOrder {
    #[default]
    StopFirst,
    StartFirst,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateFailureAction {
    Continue,
    #[default]
    Pause,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum SwarmServiceImageInfo {
    External {
        #[serde(rename = "registryId")]
        registry_id: Uuid,
        #[serde(rename = "imageTag")]
        image_tag: String,
        #[serde(rename = "resolvedDigest", default)]
        resolved_digest: Option<String>,
    },
    Build {
        #[serde(rename = "buildProjectId")]
        build_project_id: Uuid,
        #[serde(rename = "resolvedImageReference", default)]
        resolved_image_reference: Option<String>,
        #[serde(rename = "resolvedDigest", default)]
        resolved_digest: Option<String>,
        #[serde(rename = "resolvedBuildRunId", default)]
        resolved_build_run_id: Option<Uuid>,
    },
}

impl SwarmServiceImageInfo {
    #[must_use]
    pub fn without_provenance(self) -> Self {
        match self {
            Self::External {
                registry_id,
                image_tag,
                ..
            } => Self::External {
                registry_id,
                image_tag,
                resolved_digest: None,
            },
            Self::Build {
                build_project_id, ..
            } => Self::Build {
                build_project_id,
                resolved_image_reference: None,
                resolved_digest: None,
                resolved_build_run_id: None,
            },
        }
    }

    #[must_use]
    pub fn source_reference(&self) -> Option<&str> {
        match self {
            Self::External { image_tag, .. } => Some(image_tag),
            Self::Build {
                resolved_image_reference,
                ..
            } => resolved_image_reference.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServicePort {
    pub target_port: i32,
    #[serde(default)]
    pub published_port: Option<i32>,
    #[serde(default = "tcp")]
    pub protocol: String,
    #[serde(default)]
    pub publish_mode: PortPublishMode,
}

pub(crate) fn tcp() -> String {
    "tcp".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceMount {
    pub kind: MountKind,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceSecretReference {
    pub secret_id: String,
    pub secret_name: String,
    pub target_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceConfigReference {
    pub config_id: String,
    pub config_name: String,
    pub target_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceResources {
    pub limit_nano_cpus: Option<i64>,
    pub limit_memory_bytes: Option<i64>,
    pub reservation_nano_cpus: Option<i64>,
    pub reservation_memory_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceHealthCheck {
    pub test: Vec<String>,
    pub interval_nanoseconds: Option<i64>,
    pub timeout_nanoseconds: Option<i64>,
    pub retries: Option<i32>,
    pub start_period_nanoseconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceRestartPolicy {
    #[serde(default)]
    pub condition: RestartCondition,
    pub delay_nanoseconds: Option<i64>,
    pub maximum_attempts: Option<i32>,
    pub window_nanoseconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceUpdatePolicy {
    #[serde(default = "one")]
    pub parallelism: i32,
    pub delay_nanoseconds: Option<i64>,
    #[serde(default)]
    pub order: UpdateOrder,
    #[serde(default)]
    pub failure_action: UpdateFailureAction,
}

const fn one() -> i32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceSpec {
    pub image: SwarmServiceImageInfo,
    #[serde(default)]
    pub update_behavior: UpdateBehavior,
    #[serde(default)]
    pub scheduling_mode: SchedulingMode,
    #[serde(default = "one_option")]
    pub replicas: Option<i32>,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: Vec<String>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    pub user: Option<String>,
    pub working_directory: Option<String>,
    pub health_check: Option<SwarmServiceHealthCheck>,
    pub stop_grace_period_nanoseconds: Option<i64>,
    #[serde(default)]
    pub ports: Vec<SwarmServicePort>,
    #[serde(default)]
    pub network_ids: Vec<String>,
    #[serde(default)]
    pub mounts: Vec<SwarmServiceMount>,
    #[serde(default)]
    pub secrets: Vec<SwarmServiceSecretReference>,
    #[serde(default)]
    pub configs: Vec<SwarmServiceConfigReference>,
    pub resources: Option<SwarmServiceResources>,
    #[serde(default)]
    pub placement_constraints: Vec<String>,
    pub restart_policy: Option<SwarmServiceRestartPolicy>,
    pub update_policy: Option<SwarmServiceUpdatePolicy>,
    pub webhook: Option<citadel_primitives::WebhookConfig>,
}

const fn one_option() -> Option<i32> {
    Some(1)
}

impl SwarmServiceSpec {
    #[must_use]
    pub fn for_create(mut self) -> Self {
        self.image = self.image.without_provenance();
        self
    }

    pub fn validate(&self) -> Result<(), SwarmServiceError> {
        if let Some(webhook) = &self.webhook {
            webhook.validate().map_err(validation)?;
        }
        match &self.image {
            SwarmServiceImageInfo::External {
                registry_id,
                image_tag,
                ..
            } if registry_id.is_nil() || image_tag.trim().is_empty() => {
                return Err(validation(
                    "An external Registry and tagged image are required.",
                ));
            }
            SwarmServiceImageInfo::Build {
                build_project_id, ..
            } if build_project_id.is_nil() => {
                return Err(validation("A build project is required."));
            }
            _ => {}
        }
        match self.scheduling_mode {
            SchedulingMode::Replicated if self.replicas.is_none() => {
                return Err(validation(
                    "Replica count is required for a replicated Service.",
                ));
            }
            SchedulingMode::Global if self.replicas.is_some() => {
                return Err(validation(
                    "Replica count is not valid for a global Service.",
                ));
            }
            _ => {}
        }
        if self.replicas.is_some_and(|value| value < 0) {
            return Err(validation("Replica count cannot be negative."));
        }
        if self.labels.len() > 100 || self.labels.keys().any(|key| key.trim().is_empty()) {
            return Err(validation(
                "Service labels require a non-empty key and are limited to 100 entries.",
            ));
        }
        if self
            .labels
            .keys()
            .any(|key| key.to_ascii_lowercase().starts_with("com.citadel."))
        {
            return Err(validation(
                "Service labels in the com.citadel namespace are reserved.",
            ));
        }
        if self.update_behavior != UpdateBehavior::Disabled {
            let SwarmServiceImageInfo::External { image_tag, .. } = &self.image else {
                return Err(validation(
                    "Image update checks require an external tagged image.",
                ));
            };
            if image_tag.contains('@') {
                return Err(validation(
                    "Image update checks are not available for a digest-pinned image.",
                ));
            }
        }
        if self.ports.iter().any(|port| {
            !(1..=65_535).contains(&port.target_port)
                || port
                    .published_port
                    .is_some_and(|value| !(1..=65_535).contains(&value))
                || !matches!(
                    port.protocol.to_ascii_lowercase().as_str(),
                    "tcp" | "udp" | "sctp"
                )
        }) {
            return Err(validation(
                "Service ports must use a valid port number and TCP, UDP, or SCTP protocol.",
            ));
        }
        let mut ports = std::collections::HashSet::new();
        if self
            .ports
            .iter()
            .filter_map(|port| {
                port.published_port.map(|published| {
                    (
                        published,
                        port.protocol.to_ascii_lowercase(),
                        port.publish_mode,
                    )
                })
            })
            .any(|value| !ports.insert(value))
        {
            return Err(validation(
                "A published Service port can only be configured once per protocol and publish mode.",
            ));
        }
        ensure_unique(
            &self.network_ids,
            "A Service network can only be selected once.",
        )?;
        if self
            .mounts
            .iter()
            .any(|mount| mount.source.trim().is_empty() || !mount.target.starts_with('/'))
        {
            return Err(validation(
                "Every Service mount requires a source and an absolute container target path.",
            ));
        }
        ensure_unique(
            &self
                .mounts
                .iter()
                .map(|value| value.target.clone())
                .collect::<Vec<_>>(),
            "A container mount target can only be configured once.",
        )?;
        validate_references(
            &self
                .secrets
                .iter()
                .map(|value| (&value.secret_id, &value.target_name))
                .collect::<Vec<_>>(),
            "Every Swarm Secret requires a unique Secret and target name.",
        )?;
        validate_references(
            &self
                .configs
                .iter()
                .map(|value| (&value.config_id, &value.target_name))
                .collect::<Vec<_>>(),
            "Every Swarm Config requires a unique Config and target name.",
        )?;
        if self.resources.as_ref().is_some_and(|value| {
            [
                value.limit_nano_cpus,
                value.limit_memory_bytes,
                value.reservation_nano_cpus,
                value.reservation_memory_bytes,
            ]
            .into_iter()
            .flatten()
            .any(|item| item < 0)
                || exceeds(value.reservation_nano_cpus, value.limit_nano_cpus)
                || exceeds(value.reservation_memory_bytes, value.limit_memory_bytes)
        }) {
            return Err(validation(
                "Resource reservations and limits must be non-negative, and a reservation cannot exceed its limit.",
            ));
        }
        if self.command.len() > 100
            || self.arguments.len() > 200
            || self.environment.len() > 500
            || self.ports.len() > 100
            || self.network_ids.len() > 100
            || self.mounts.len() > 100
            || self.secrets.len() > 100
            || self.configs.len() > 100
            || self.placement_constraints.len() > 100
        {
            return Err(validation(
                "The Service configuration contains too many entries.",
            ));
        }
        Ok(())
    }

    pub fn to_storage_value(&self) -> Result<Value, SwarmServiceError> {
        let value = serde_json::to_value(self).map_err(json_error)?;
        Ok(pascalize(value))
    }

    pub fn from_storage_value(value: Value) -> Result<Self, SwarmServiceError> {
        serde_json::from_value(camelize(value)).map_err(json_error)
    }

    #[must_use]
    pub fn desired_hash(&self) -> String {
        let mut value = self.clone();
        match &mut value.image {
            SwarmServiceImageInfo::External {
                resolved_digest, ..
            } => *resolved_digest = None,
            SwarmServiceImageInfo::Build {
                resolved_digest,
                resolved_build_run_id,
                ..
            } => {
                *resolved_digest = None;
                *resolved_build_run_id = None;
            }
        }
        value.webhook = None;
        value.environment.sort_unstable();
        value.network_ids.sort_unstable();
        value.placement_constraints.sort_unstable();
        value.ports.sort_by_key(|port| {
            (
                port.published_port,
                port.target_port,
                port.protocol.to_ascii_lowercase(),
                port.publish_mode,
            )
        });
        value
            .mounts
            .sort_by(|left, right| left.target.cmp(&right.target));
        value
            .secrets
            .sort_by(|left, right| left.target_name.cmp(&right.target_name));
        value
            .configs
            .sort_by(|left, right| left.target_name.cmp(&right.target_name));
        let canonical = canonicalize(serde_json::to_value(value).expect("Service spec serializes"));
        Sha256::digest(serde_json::to_vec(&canonical).expect("canonical spec serializes"))
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SwarmServiceError {
    #[error("{0}")]
    Validation(String),
    #[error("Managed Swarm Service was not found.")]
    NotFound,
    #[error("This operation is not authorized.")]
    Forbidden,
    #[error("{0}")]
    Conflict(String),
    #[error("Swarm Service runtime is unavailable: {0}")]
    Runtime(String),
    #[error("Swarm Service runtime rejected the operation: {0}")]
    RuntimeRejected(String),
    #[error("Swarm Service persistence failed: {0}")]
    Storage(String),
    #[error("Swarm Service operation was cancelled.")]
    Cancelled,
}

pub(crate) fn validation(message: &str) -> SwarmServiceError {
    SwarmServiceError::Validation(message.to_owned())
}

pub(crate) fn json_error(error: serde_json::Error) -> SwarmServiceError {
    SwarmServiceError::Storage(format!("invalid persisted Swarm Service spec: {error}"))
}

pub(crate) fn exceeds(value: Option<i64>, limit: Option<i64>) -> bool {
    matches!((value, limit), (Some(value), Some(limit)) if value > limit)
}

pub(crate) fn ensure_unique(values: &[String], message: &str) -> Result<(), SwarmServiceError> {
    let mut unique = std::collections::HashSet::with_capacity(values.len());
    if values.iter().any(|value| !unique.insert(value)) {
        Err(validation(message))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_references(
    values: &[(&String, &String)],
    message: &str,
) -> Result<(), SwarmServiceError> {
    if values
        .iter()
        .any(|(id, target)| id.trim().is_empty() || target.trim().is_empty())
    {
        return Err(validation(message));
    }
    let ids = values
        .iter()
        .map(|(id, _)| (*id).clone())
        .collect::<Vec<_>>();
    let targets = values
        .iter()
        .map(|(_, target)| (*target).clone())
        .collect::<Vec<_>>();
    ensure_unique(&ids, message)?;
    ensure_unique(&targets, message)
}

pub(crate) fn pascalize(value: Value) -> Value {
    rename_keys(value, true)
}

pub(crate) fn camelize(value: Value) -> Value {
    rename_keys(value, false)
}

pub(crate) fn rename_keys(value: Value, pascal: bool) -> Value {
    use citadel_primitives::json_keys::{PropertyCase, map_property_keys};
    let case = if pascal {
        PropertyCase::Pascal
    } else {
        PropertyCase::Camel
    };
    map_property_keys(value, case, &["labels"])
}

pub(crate) fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, canonicalize(value)))
                .collect(),
        ),
        Value::Array(mut values) => {
            for value in &mut values {
                *value = canonicalize(std::mem::take(value));
            }
            Value::Array(values)
        }
        other => other,
    }
}
