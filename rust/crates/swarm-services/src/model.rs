use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = swarm_services::model::UpdateBehavior)]
pub enum UpdateBehavior {
    #[default]
    Disabled,
    Notify,
    AutoDeploy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SchedulingMode {
    #[default]
    Replicated,
    Global,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
pub enum PortPublishMode {
    #[default]
    Ingress,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum MountKind {
    Volume,
    Bind,
    Tmpfs,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum RestartCondition {
    None,
    OnFailure,
    #[default]
    Any,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum UpdateOrder {
    #[default]
    StopFirst,
    StartFirst,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum UpdateFailureAction {
    Continue,
    #[default]
    Pause,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

fn tcp() -> String {
    "tcp".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceMount {
    pub kind: MountKind,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceSecretReference {
    pub secret_id: String,
    pub secret_name: String,
    pub target_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceConfigReference {
    pub config_id: String,
    pub config_name: String,
    pub target_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceResources {
    pub limit_nano_cpus: Option<i64>,
    pub limit_memory_bytes: Option<i64>,
    pub reservation_nano_cpus: Option<i64>,
    pub reservation_memory_bytes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceHealthCheck {
    pub test: Vec<String>,
    pub interval_nanoseconds: Option<i64>,
    pub timeout_nanoseconds: Option<i64>,
    pub retries: Option<i32>,
    pub start_period_nanoseconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceRestartPolicy {
    #[serde(default)]
    pub condition: RestartCondition,
    pub delay_nanoseconds: Option<i64>,
    pub maximum_attempts: Option<i32>,
    pub window_nanoseconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceWebhookConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "github_provider")]
    pub provider: String,
    #[serde(default = "github_hmac_sha256")]
    pub auth_scheme: String,
    pub secret: Option<String>,
    pub branch_filter: Option<String>,
}

fn github_provider() -> String {
    "GitHub".to_owned()
}

fn github_hmac_sha256() -> String {
    "GitHubHmacSha256".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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
    pub webhook: Option<SwarmServiceWebhookConfig>,
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

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateSwarmServiceInput {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    #[schema(nullable)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub duplicate_source: Option<SwarmServiceDuplicateSource>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmServiceDuplicateSource {
    pub resource_id: Uuid,
    pub resource_type: String,
    pub resource_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceDuplicateDraft {
    pub name: String,
    pub source_name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: SwarmServiceSpec,
    pub tag_ids: Vec<Uuid>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateSwarmServiceInput {
    pub spec: SwarmServiceSpec,
    pub row_version: i64,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameSwarmServiceInput {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScaleSwarmServiceInput {
    pub replicas: i32,
}

#[derive(Debug, Clone, Default)]
pub struct SwarmServiceFilter {
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = swarm_services::model::ResourceCapabilities)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceCapabilities {
    pub can_view_logs: bool,
    pub can_inspect: bool,
    pub can_apply: bool,
    pub can_view_resource_bindings: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[schema(as = swarm_services::model::AutoUpdateState)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateState {
    pub last_checked_at: DateTime<Utc>,
    pub status: String,
    pub current_digest: Option<String>,
    pub remote_digest: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceOperationView {
    pub id: Uuid,
    pub kind: String,
    pub state: String,
    pub prepared_at: DateTime<Utc>,
    pub attempted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result_code: Option<String>,
    pub warnings: Vec<String>,
    pub result_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = swarm_services::model::TagSummary)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSwarmServiceView {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub docker_name: String,
    pub docker_service_id: Option<String>,
    pub spec: SwarmServiceSpec,
    pub health: String,
    pub synchronization_state: String,
    pub control_state: String,
    pub auto_update_state: AutoUpdateState,
    pub applied_image_digest: Option<String>,
    pub has_pending_desired_changes: bool,
    pub has_runtime_drift: bool,
    pub row_version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub platform_name: Option<String>,
    pub platform_status: String,
    pub running_task_count: Option<i32>,
    pub desired_task_count: Option<i32>,
    pub update_state: Option<String>,
    pub update_message: Option<String>,
    pub current_operation: Option<SwarmServiceOperationView>,
    pub tags: Vec<TagSummary>,
    pub tasks: Option<Vec<Value>>,
    pub capabilities: Option<SwarmServiceCapabilities>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSwarmServicesView {
    pub swarm_services: Vec<ManagedSwarmServiceView>,
    pub capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceProgressItem {
    pub service_id: Uuid,
    pub operation_id: Option<Uuid>,
    pub stage: String,
    pub message: String,
    #[serde(default)]
    pub is_completed: bool,
    #[serde(default)]
    pub is_warning: bool,
    pub error_message: Option<String>,
}

impl SwarmServiceProgressItem {
    pub fn info(
        service_id: Uuid,
        operation_id: Option<Uuid>,
        stage: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            service_id,
            operation_id,
            stage: stage.to_owned(),
            message: message.into(),
            is_completed: false,
            is_warning: false,
            error_message: None,
        }
    }
    pub fn completed(service_id: Uuid, operation_id: Uuid, message: impl Into<String>) -> Self {
        Self {
            is_completed: true,
            ..Self::info(service_id, Some(operation_id), "Completed", message)
        }
    }
    pub fn failed(
        service_id: Uuid,
        operation_id: Option<Uuid>,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        Self {
            error_message: Some(message.clone()),
            ..Self::info(service_id, operation_id, "Failed", message)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceOperationClaim {
    pub operation_id: Uuid,
    pub id: Uuid,
    pub platform_id: Uuid,
    pub docker_name: String,
    pub docker_service_id: Option<String>,
    pub docker_version_index: Option<i64>,
    pub row_version: i64,
    pub desired_hash: String,
    pub spec: SwarmServiceSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDeletionClaim {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub docker_service_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeServiceResult {
    pub docker_service_id: String,
    pub version_index: i64,
    pub accepted: bool,
    pub rollout_complete: bool,
    pub rollout_error: Option<String>,
    pub runtime_hash: String,
    pub applied_digest: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceOperationKind {
    Apply,
    Scale,
    ForceUpdate,
}

impl ServiceOperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "Apply",
            Self::Scale => "Scale",
            Self::ForceUpdate => "ForceUpdate",
        }
    }
}

#[derive(Debug, thiserror::Error)]
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

fn validation(message: &str) -> SwarmServiceError {
    SwarmServiceError::Validation(message.to_owned())
}
fn json_error(error: serde_json::Error) -> SwarmServiceError {
    SwarmServiceError::Storage(format!("invalid persisted Swarm Service spec: {error}"))
}
fn exceeds(value: Option<i64>, limit: Option<i64>) -> bool {
    matches!((value, limit), (Some(value), Some(limit)) if value > limit)
}
fn ensure_unique(values: &[String], message: &str) -> Result<(), SwarmServiceError> {
    let mut unique = std::collections::HashSet::with_capacity(values.len());
    if values.iter().any(|value| !unique.insert(value)) {
        Err(validation(message))
    } else {
        Ok(())
    }
}
fn validate_references(
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

pub(crate) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

fn pascalize(value: Value) -> Value {
    rename_keys(value, true)
}
fn camelize(value: Value) -> Value {
    rename_keys(value, false)
}
fn rename_keys(value: Value, pascal: bool) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let mut chars = key.chars();
                    let key = chars.next().map_or(key.clone(), |first| {
                        let head = if pascal {
                            first.to_ascii_uppercase()
                        } else {
                            first.to_ascii_lowercase()
                        };
                        format!("{head}{}", chars.as_str())
                    });
                    let value = if key.eq_ignore_ascii_case("Labels") {
                        value
                    } else {
                        rename_keys(value, pascal)
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| rename_keys(value, pascal))
                .collect(),
        ),
        other => other,
    }
}

fn canonicalize(value: Value) -> Value {
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
