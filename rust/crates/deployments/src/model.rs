use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateBehavior {
    #[serde(alias = "disabled")]
    Disabled,
    #[serde(alias = "notify")]
    Notify,
    #[serde(alias = "autoDeploy", alias = "autodeploy")]
    AutoDeploy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopSignal {
    #[serde(alias = "sigterm")]
    SIGTERM,
    #[serde(alias = "sigkill")]
    SIGKILL,
    #[serde(alias = "sigint")]
    SIGINT,
    #[serde(alias = "sigquit")]
    SIGQUIT,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerRestartPolicy {
    #[serde(alias = "no")]
    #[default]
    No,
    #[serde(alias = "always")]
    Always,
    #[serde(alias = "onFailure", alias = "onfailure")]
    OnFailure,
    #[serde(alias = "unlessStopped", alias = "unlessstopped")]
    UnlessStopped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum DeploymentImageInfo {
    Local {
        #[serde(rename = "imageId")]
        image_id: String,
    },
    External {
        #[serde(rename = "registryId")]
        registry_id: Uuid,
        #[serde(rename = "imageTag")]
        image_tag: String,
        #[serde(
            rename = "resolvedDigest",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        resolved_digest: Option<String>,
    },
    Build {
        #[serde(rename = "buildProjectId")]
        build_project_id: Uuid,
        #[serde(rename = "redeployOnBuild", default)]
        redeploy_on_build: bool,
        #[serde(
            rename = "resolvedImageReference",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        resolved_image_reference: Option<String>,
        #[serde(
            rename = "resolvedDigest",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        resolved_digest: Option<String>,
        #[serde(
            rename = "resolvedBuildRunId",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        resolved_build_run_id: Option<Uuid>,
        #[serde(
            rename = "appliedImageReference",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        applied_image_reference: Option<String>,
        #[serde(
            rename = "appliedDigest",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        applied_digest: Option<String>,
        #[serde(
            rename = "appliedBuildRunId",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        applied_build_run_id: Option<Uuid>,
        #[serde(rename = "appliedAt", default, skip_serializing_if = "Option::is_none")]
        applied_at: Option<DateTime<Utc>>,
    },
}

impl DeploymentImageInfo {
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
                build_project_id,
                redeploy_on_build,
                ..
            } => Self::Build {
                build_project_id,
                redeploy_on_build,
                resolved_image_reference: None,
                resolved_digest: None,
                resolved_build_run_id: None,
                applied_image_reference: None,
                applied_digest: None,
                applied_build_run_id: None,
                applied_at: None,
            },
            local => local,
        }
    }

    #[must_use]
    pub fn preserving_build_provenance_from(self, current: &Self) -> Self {
        match (self, current) {
            (
                Self::Build {
                    build_project_id,
                    redeploy_on_build,
                    ..
                },
                Self::Build {
                    build_project_id: current_project,
                    resolved_image_reference,
                    resolved_digest,
                    resolved_build_run_id,
                    applied_image_reference,
                    applied_digest,
                    applied_build_run_id,
                    applied_at,
                    ..
                },
            ) if build_project_id == *current_project => Self::Build {
                build_project_id,
                redeploy_on_build,
                resolved_image_reference: resolved_image_reference.clone(),
                resolved_digest: resolved_digest.clone(),
                resolved_build_run_id: *resolved_build_run_id,
                applied_image_reference: applied_image_reference.clone(),
                applied_digest: applied_digest.clone(),
                applied_build_run_id: *applied_build_run_id,
                applied_at: *applied_at,
            },
            (next, _) => next.without_provenance(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nano_cpus: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifeCycleSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_timeout: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_signal: Option<StopSignal>,
    #[serde(default)]
    pub restart_policy: ContainerRestartPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentSpec {
    pub image: DeploymentImageInfo,
    pub update_behavior: UpdateBehavior,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life_cycle_spec: Option<LifeCycleSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_spec: Option<ResourceSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_variables: Option<Vec<String>>,
}

impl DeploymentSpec {
    pub fn validate(&self) -> Result<(), DeploymentError> {
        match &self.image {
            DeploymentImageInfo::Local { image_id } if image_id.trim().is_empty() => {
                return Err(DeploymentError::Validation(
                    "A local image must be selected.".to_owned(),
                ));
            }
            DeploymentImageInfo::External {
                image_tag,
                registry_id,
                ..
            } => {
                if registry_id.is_nil() || image_tag.trim().is_empty() {
                    return Err(DeploymentError::Validation(
                        "An external image requires a Registry and image reference.".to_owned(),
                    ));
                }
                if image_tag.contains('@') && self.update_behavior != UpdateBehavior::Disabled {
                    return Err(DeploymentError::Validation(
                        "Auto update is unavailable for an image pinned by digest.".to_owned(),
                    ));
                }
            }
            DeploymentImageInfo::Build {
                build_project_id, ..
            } if build_project_id.is_nil() => {
                return Err(DeploymentError::Validation(
                    "A Build Project must be selected.".to_owned(),
                ));
            }
            _ => {}
        }
        if !matches!(self.image, DeploymentImageInfo::External { .. })
            && self.update_behavior != UpdateBehavior::Disabled
        {
            return Err(DeploymentError::Validation(
                "Auto update is available only for external tagged images.".to_owned(),
            ));
        }
        if self
            .life_cycle_spec
            .as_ref()
            .and_then(|value| value.stop_timeout)
            .is_some_and(|value| value < 0)
        {
            return Err(DeploymentError::Validation(
                "Stop timeout cannot be negative.".to_owned(),
            ));
        }
        if let Some(resource) = &self.resource_spec
            && (resource.nano_cpus.is_some_and(|value| value < 0.0)
                || resource.memory_limit.is_some_and(|value| value < 0.0))
        {
            return Err(DeploymentError::Validation(
                "Resource limits cannot be negative.".to_owned(),
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn for_create(mut self) -> Self {
        self.image = self.image.without_provenance();
        self
    }

    #[must_use]
    pub fn for_update(mut self, current: &Self) -> Self {
        self.image = self.image.preserving_build_provenance_from(&current.image);
        self
    }

    pub fn to_storage_value(&self) -> Result<Value, DeploymentError> {
        serde_json::to_value(StoredDeploymentSpec::from(self)).map_err(json_error)
    }

    pub fn from_storage_value(value: Value) -> Result<Self, DeploymentError> {
        serde_json::from_value::<StoredDeploymentSpec>(value)
            .map(Self::from)
            .map_err(json_error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentCapabilities {
    pub can_view_logs: bool,
    pub can_inspect: bool,
    pub can_open_terminal: bool,
    pub can_pull: bool,
    pub can_apply: bool,
    pub can_view_resource_bindings: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateState {
    pub last_checked_at: DateTime<Utc>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentView {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub platform_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub status: String,
    pub control_state: String,
    #[serde(skip)]
    pub row_version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_update_state: Option<AutoUpdateState>,
    pub spec: DeploymentSpec,
    pub platform_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker_image_id: Option<String>,
    pub tags: Vec<TagSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_activity_view: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<DeploymentCapabilities>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentConfigView {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub spec: DeploymentSpec,
}

impl From<&DeploymentView> for DeploymentConfigView {
    fn from(value: &DeploymentView) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            platform_id: value.platform_id,
            description: value.description.clone(),
            spec: value.spec.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentsView {
    pub deployments: Vec<DeploymentView>,
    pub capabilities: ResourceCapabilities,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDeploymentInput {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: Option<DuplicateSourceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateSourceInput {
    pub resource_type: String,
    pub resource_id: Uuid,
    pub resource_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchDeploymentInput {
    pub platform_id: Option<Uuid>,
    pub spec: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchDeploymentMetadataInput {
    #[serde(default)]
    pub description: FieldPatch<String>,
    #[serde(default, rename = "tags")]
    pub _tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub enum FieldPatch<T> {
    #[default]
    Unchanged,
    Set(T),
    Clear,
}

impl<'de, T> Deserialize<'de> for FieldPatch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameDeploymentInput {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyDeploymentInput {
    pub id: Uuid,
    #[serde(default)]
    pub recreate: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentApplyError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePullProgress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStreamItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ImagePullProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DeploymentApplyError>,
}

impl DeploymentStreamItem {
    #[must_use]
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            progress_message: Some(message.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn failure(code: i64, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            error_message: Some(message.clone()),
            error: Some(DeploymentApplyError { code, message }),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeploymentFilter {
    pub tags: Vec<String>,
    pub platform_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentDuplicateDraftView {
    pub draft: CreateDeploymentInputView,
    pub warnings: Vec<DuplicateWarning>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentInputView {
    pub name: String,
    pub platform_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: DuplicateSourceInput,
}

#[derive(Debug, Clone)]
pub struct EffectiveDeploymentPermission {
    pub level_mask: i32,
    pub specific_mask: i32,
}

#[derive(Debug, Clone)]
pub struct DeletionClaim {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub docker_container_ids: Vec<String>,
    pub row_version: i64,
    pub previous_status: String,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
}

#[derive(Debug, Clone)]
pub struct ApplyClaim {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub platform_address: String,
    pub name: String,
    pub row_version: i64,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub existing_container_id: Option<Uuid>,
    pub existing_docker_container_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDeploymentImage {
    pub docker_image_id: String,
    pub digest: Option<String>,
    pub resolved_build: Option<ResolvedDeploymentBuild>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDeploymentBuild {
    pub image_reference: String,
    pub digest: Option<String>,
    pub build_run_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RuntimeDeploymentCommand {
    pub deployment_id: Uuid,
    pub name: String,
    pub image_id: String,
    pub spec: DeploymentSpec,
    pub environment_variables: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeContainerState {
    Running,
    Exited,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDeploymentResult {
    pub docker_container_id: String,
    pub docker_image_id: String,
    pub state: RuntimeContainerState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeploymentBindingSnapshot {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Kind")]
    pub kind: String,
    #[serde(rename = "Scope")]
    pub scope: String,
    #[serde(rename = "Value")]
    pub value: String,
    #[serde(rename = "SecretId", skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<Uuid>,
    #[serde(rename = "SecretDeliveryMode", skip_serializing_if = "Option::is_none")]
    pub secret_delivery_mode: Option<String>,
    #[serde(rename = "TargetPath", skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
}

#[derive(Debug)]
pub struct ResolvedDeploymentBinding {
    pub name: String,
    pub value: zeroize::Zeroizing<String>,
    pub secret: bool,
    pub snapshot: DeploymentBindingSnapshot,
}

#[derive(Debug, Default)]
pub struct ResolvedDeploymentBindings {
    pub entries: Vec<ResolvedDeploymentBinding>,
}

#[derive(Debug, thiserror::Error)]
pub enum DeploymentError {
    #[error("{0}")]
    Validation(String),
    #[error("Deployment was not found.")]
    NotFound,
    #[error("This operation is not authorized.")]
    Forbidden,
    #[error("License capability '{0}' is unavailable.")]
    LicenseRequired(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("Deployment runtime is unavailable: {0}")]
    Runtime(String),
    #[error("Deployment persistence failed: {0}")]
    Storage(String),
    #[error("Deployment operation was cancelled.")]
    Cancelled,
}

fn json_error(error: serde_json::Error) -> DeploymentError {
    DeploymentError::Storage(format!("invalid persisted Deployment spec: {error}"))
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[derive(Serialize, Deserialize)]
struct StoredDeploymentSpec {
    #[serde(rename = "Image", alias = "image")]
    image: StoredDeploymentImageInfo,
    #[serde(rename = "UpdateBehavior", alias = "updateBehavior")]
    update_behavior: UpdateBehavior,
    #[serde(rename = "LifeCycleSpec", alias = "lifeCycleSpec")]
    life_cycle_spec: Option<StoredLifeCycleSpec>,
    #[serde(rename = "ResourceSpec", alias = "resourceSpec")]
    resource_spec: Option<StoredResourceSpec>,
    #[serde(rename = "Labels", alias = "labels")]
    labels: Option<BTreeMap<String, String>>,
    #[serde(rename = "Ports", alias = "ports")]
    ports: Option<Vec<String>>,
    #[serde(rename = "Volumes", alias = "volumes")]
    volumes: Option<Vec<String>>,
    #[serde(rename = "Networks", alias = "networks")]
    networks: Option<Vec<String>>,
    #[serde(rename = "Command", alias = "command")]
    command: Option<Vec<String>>,
    #[serde(rename = "EnvironmentVariables", alias = "environmentVariables")]
    environment_variables: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "$type")]
enum StoredDeploymentImageInfo {
    Local {
        #[serde(rename = "ImageId", alias = "imageId")]
        image_id: String,
    },
    External {
        #[serde(rename = "RegistryId", alias = "registryId")]
        registry_id: Uuid,
        #[serde(rename = "ImageTag", alias = "imageTag")]
        image_tag: String,
        #[serde(rename = "ResolvedDigest", alias = "resolvedDigest")]
        resolved_digest: Option<String>,
    },
    Build {
        #[serde(rename = "BuildProjectId", alias = "buildProjectId")]
        build_project_id: Uuid,
        #[serde(rename = "RedeployOnBuild", alias = "redeployOnBuild", default)]
        redeploy_on_build: bool,
        #[serde(rename = "ResolvedImageReference", alias = "resolvedImageReference")]
        resolved_image_reference: Option<String>,
        #[serde(rename = "ResolvedDigest", alias = "resolvedDigest")]
        resolved_digest: Option<String>,
        #[serde(rename = "ResolvedBuildRunId", alias = "resolvedBuildRunId")]
        resolved_build_run_id: Option<Uuid>,
        #[serde(rename = "AppliedImageReference", alias = "appliedImageReference")]
        applied_image_reference: Option<String>,
        #[serde(rename = "AppliedDigest", alias = "appliedDigest")]
        applied_digest: Option<String>,
        #[serde(rename = "AppliedBuildRunId", alias = "appliedBuildRunId")]
        applied_build_run_id: Option<Uuid>,
        #[serde(rename = "AppliedAt", alias = "appliedAt")]
        applied_at: Option<DateTime<Utc>>,
    },
}

#[derive(Serialize, Deserialize)]
struct StoredResourceSpec {
    #[serde(rename = "NanoCpus", alias = "nanoCpus")]
    nano_cpus: Option<f32>,
    #[serde(rename = "MemoryLimit", alias = "memoryLimit")]
    memory_limit: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct StoredLifeCycleSpec {
    #[serde(rename = "StopTimeout", alias = "stopTimeout")]
    stop_timeout: Option<i32>,
    #[serde(rename = "StopSignal", alias = "stopSignal")]
    stop_signal: Option<StopSignal>,
    #[serde(rename = "RestartPolicy", alias = "restartPolicy")]
    restart_policy: ContainerRestartPolicy,
}

impl From<&DeploymentSpec> for StoredDeploymentSpec {
    fn from(value: &DeploymentSpec) -> Self {
        Self {
            image: StoredDeploymentImageInfo::from(&value.image),
            update_behavior: value.update_behavior,
            life_cycle_spec: value
                .life_cycle_spec
                .as_ref()
                .map(|item| StoredLifeCycleSpec {
                    stop_timeout: item.stop_timeout,
                    stop_signal: item.stop_signal,
                    restart_policy: item.restart_policy,
                }),
            resource_spec: value.resource_spec.as_ref().map(|item| StoredResourceSpec {
                nano_cpus: item.nano_cpus,
                memory_limit: item.memory_limit,
            }),
            labels: value.labels.clone(),
            ports: value.ports.clone(),
            volumes: value.volumes.clone(),
            networks: value.networks.clone(),
            command: value.command.clone(),
            environment_variables: value.environment_variables.clone(),
        }
    }
}

impl From<&DeploymentImageInfo> for StoredDeploymentImageInfo {
    fn from(value: &DeploymentImageInfo) -> Self {
        match value {
            DeploymentImageInfo::Local { image_id } => Self::Local {
                image_id: image_id.clone(),
            },
            DeploymentImageInfo::External {
                registry_id,
                image_tag,
                resolved_digest,
            } => Self::External {
                registry_id: *registry_id,
                image_tag: image_tag.clone(),
                resolved_digest: resolved_digest.clone(),
            },
            DeploymentImageInfo::Build {
                build_project_id,
                redeploy_on_build,
                resolved_image_reference,
                resolved_digest,
                resolved_build_run_id,
                applied_image_reference,
                applied_digest,
                applied_build_run_id,
                applied_at,
            } => Self::Build {
                build_project_id: *build_project_id,
                redeploy_on_build: *redeploy_on_build,
                resolved_image_reference: resolved_image_reference.clone(),
                resolved_digest: resolved_digest.clone(),
                resolved_build_run_id: *resolved_build_run_id,
                applied_image_reference: applied_image_reference.clone(),
                applied_digest: applied_digest.clone(),
                applied_build_run_id: *applied_build_run_id,
                applied_at: *applied_at,
            },
        }
    }
}

impl From<StoredDeploymentSpec> for DeploymentSpec {
    fn from(value: StoredDeploymentSpec) -> Self {
        Self {
            image: DeploymentImageInfo::from(value.image),
            update_behavior: value.update_behavior,
            life_cycle_spec: value.life_cycle_spec.map(|item| LifeCycleSpec {
                stop_timeout: item.stop_timeout,
                stop_signal: item.stop_signal,
                restart_policy: item.restart_policy,
            }),
            resource_spec: value.resource_spec.map(|item| ResourceSpec {
                nano_cpus: item.nano_cpus,
                memory_limit: item.memory_limit,
            }),
            labels: value.labels,
            ports: value.ports,
            volumes: value.volumes,
            networks: value.networks,
            command: value.command,
            environment_variables: value.environment_variables,
        }
    }
}

impl From<StoredDeploymentImageInfo> for DeploymentImageInfo {
    fn from(value: StoredDeploymentImageInfo) -> Self {
        match value {
            StoredDeploymentImageInfo::Local { image_id } => Self::Local { image_id },
            StoredDeploymentImageInfo::External {
                registry_id,
                image_tag,
                resolved_digest,
            } => Self::External {
                registry_id,
                image_tag,
                resolved_digest,
            },
            StoredDeploymentImageInfo::Build {
                build_project_id,
                redeploy_on_build,
                resolved_image_reference,
                resolved_digest,
                resolved_build_run_id,
                applied_image_reference,
                applied_digest,
                applied_build_run_id,
                applied_at,
            } => Self::Build {
                build_project_id,
                redeploy_on_build,
                resolved_image_reference,
                resolved_digest,
                resolved_build_run_id,
                applied_image_reference,
                applied_digest,
                applied_build_run_id,
                applied_at,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn external_spec() -> DeploymentSpec {
        DeploymentSpec {
            image: DeploymentImageInfo::External {
                registry_id: Uuid::from_u128(0x100),
                image_tag: "nginx:latest".to_owned(),
                resolved_digest: Some("sha256:old".to_owned()),
            },
            update_behavior: UpdateBehavior::Notify,
            life_cycle_spec: None,
            resource_spec: None,
            labels: Some(BTreeMap::from([(
                "Mixed.Key".to_owned(),
                "value".to_owned(),
            )])),
            ports: Some(vec!["8080:80".to_owned()]),
            volumes: None,
            networks: None,
            command: None,
            environment_variables: None,
        }
    }

    #[test]
    fn storage_contract_keeps_pascal_case_without_rewriting_map_keys() {
        let value = external_spec().to_storage_value().unwrap();
        assert_eq!(value["Image"]["$type"], "External");
        assert_eq!(value["Image"]["ImageTag"], "nginx:latest");
        assert_eq!(value["Labels"]["Mixed.Key"], "value");
        assert!(value.get("image").is_none());
        assert_eq!(
            DeploymentSpec::from_storage_value(value).unwrap(),
            external_spec()
        );
    }

    #[test]
    fn create_removes_resolved_provenance() {
        let created = external_spec().for_create();
        assert!(matches!(
            created.image,
            DeploymentImageInfo::External {
                resolved_digest: None,
                ..
            }
        ));
    }

    #[test]
    fn digest_pinned_external_image_rejects_auto_update() {
        let mut spec = external_spec();
        if let DeploymentImageInfo::External { image_tag, .. } = &mut spec.image {
            *image_tag = "nginx@sha256:abc".to_owned();
        }
        assert!(spec.validate().is_err());
    }

    #[test]
    fn create_contract_accepts_null_tag_ids() {
        let input = serde_json::from_value::<CreateDeploymentInput>(serde_json::json!({
            "name": "web",
            "platformId": Uuid::now_v7(),
            "description": null,
            "spec": {
                "image": { "$type": "Local", "imageId": "image" },
                "updateBehavior": "Disabled",
                "lifeCycleSpec": null,
                "resourceSpec": null,
                "labels": null,
                "ports": null,
                "volumes": null,
                "networks": null,
                "command": null,
                "environmentVariables": null
            },
            "tagIds": null,
            "duplicateSource": null
        }))
        .unwrap();

        assert!(input.tag_ids.is_empty());
    }

    #[test]
    fn omitted_stop_timeout_is_valid() {
        let mut spec = external_spec();
        spec.update_behavior = UpdateBehavior::Disabled;

        assert!(spec.validate().is_ok());
    }

    #[test]
    fn lifecycle_contract_defaults_an_omitted_restart_policy_to_no() {
        let value = serde_json::json!({
            "image": { "$type": "Local", "imageId": "image" },
            "updateBehavior": "Disabled",
            "lifeCycleSpec": { "stopSignal": "SIGKILL", "stopTimeout": 15 }
        });

        let spec: DeploymentSpec = serde_json::from_value(value).unwrap();

        assert_eq!(
            spec.life_cycle_spec.unwrap().restart_policy,
            ContainerRestartPolicy::No
        );
    }

    #[test]
    fn api_serialization_omits_null_properties_like_the_dotnet_contract() {
        let view = DeploymentView {
            id: Uuid::now_v7(),
            name: "web".to_owned(),
            description: None,
            platform_id: Uuid::now_v7(),
            created_at: Utc::now(),
            created_by_actor_id: Uuid::now_v7(),
            status: "Created".to_owned(),
            control_state: "Idle".to_owned(),
            row_version: 0,
            auto_update_state: Some(AutoUpdateState {
                last_checked_at: Utc::now(),
                status: "Unknown".to_owned(),
                current_digest: None,
                remote_digest: None,
                last_error: None,
            }),
            spec: DeploymentSpec {
                image: DeploymentImageInfo::External {
                    registry_id: Uuid::from_u128(0x100),
                    image_tag: "nginx:latest".to_owned(),
                    resolved_digest: None,
                },
                update_behavior: UpdateBehavior::Notify,
                life_cycle_spec: None,
                resource_spec: None,
                labels: None,
                ports: None,
                volumes: None,
                networks: None,
                command: None,
                environment_variables: None,
            },
            platform_status: "Online".to_owned(),
            platform_name: None,
            image_name: None,
            image_id: None,
            container_id: None,
            docker_container_id: None,
            docker_image_id: None,
            tags: Vec::new(),
            latest_activity_view: None,
            capabilities: None,
        };

        let value = serde_json::to_value(view).unwrap();

        assert_eq!(value["spec"]["image"]["$type"], "External");
        assert!(value.get("description").is_none());
        assert!(value.get("rowVersion").is_none());
        assert!(value.get("platformName").is_none());
        assert!(value["spec"].get("lifeCycleSpec").is_none());
        assert!(value["spec"]["image"].get("resolvedDigest").is_none());
        assert!(value["autoUpdateState"].get("lastError").is_none());
    }
}
