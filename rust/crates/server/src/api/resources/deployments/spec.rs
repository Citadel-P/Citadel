//! Server-owned schema and wire representations of Deployment value objects.
use crate::api::resources::common::{AutoUpdateStatus, DuplicateSourceInput};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = UpdateBehavior)]
pub enum UpdateBehavior {
    #[default]
    #[serde(alias = "disabled")]
    Disabled,
    #[serde(alias = "notify")]
    Notify,
    #[serde(alias = "autoDeploy", alias = "autodeploy")]
    AutoDeploy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nano_cpus: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LifeCycleSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_timeout: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_signal: Option<StopSignal>,
    #[serde(default)]
    pub restart_policy: ContainerRestartPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentSpec {
    pub image: DeploymentImageInfo,
    #[serde(default)]
    #[schema(default = UpdateBehavior::default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = deployments::model::AutoUpdateState)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateState {
    pub last_checked_at: DateTime<Utc>,
    pub status: AutoUpdateStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
}

impl From<citadel_deployments::UpdateBehavior> for UpdateBehavior {
    fn from(value: citadel_deployments::UpdateBehavior) -> Self {
        match value {
            citadel_deployments::UpdateBehavior::Disabled => Self::Disabled,
            citadel_deployments::UpdateBehavior::Notify => Self::Notify,
            citadel_deployments::UpdateBehavior::AutoDeploy => Self::AutoDeploy,
        }
    }
}

impl From<UpdateBehavior> for citadel_deployments::UpdateBehavior {
    fn from(value: UpdateBehavior) -> Self {
        match value {
            UpdateBehavior::Disabled => Self::Disabled,
            UpdateBehavior::Notify => Self::Notify,
            UpdateBehavior::AutoDeploy => Self::AutoDeploy,
        }
    }
}

impl From<citadel_deployments::StopSignal> for StopSignal {
    fn from(value: citadel_deployments::StopSignal) -> Self {
        match value {
            citadel_deployments::StopSignal::SIGTERM => Self::SIGTERM,
            citadel_deployments::StopSignal::SIGKILL => Self::SIGKILL,
            citadel_deployments::StopSignal::SIGINT => Self::SIGINT,
            citadel_deployments::StopSignal::SIGQUIT => Self::SIGQUIT,
        }
    }
}

impl From<StopSignal> for citadel_deployments::StopSignal {
    fn from(value: StopSignal) -> Self {
        match value {
            StopSignal::SIGTERM => Self::SIGTERM,
            StopSignal::SIGKILL => Self::SIGKILL,
            StopSignal::SIGINT => Self::SIGINT,
            StopSignal::SIGQUIT => Self::SIGQUIT,
        }
    }
}

impl From<citadel_deployments::ContainerRestartPolicy> for ContainerRestartPolicy {
    fn from(value: citadel_deployments::ContainerRestartPolicy) -> Self {
        match value {
            citadel_deployments::ContainerRestartPolicy::No => Self::No,
            citadel_deployments::ContainerRestartPolicy::Always => Self::Always,
            citadel_deployments::ContainerRestartPolicy::OnFailure => Self::OnFailure,
            citadel_deployments::ContainerRestartPolicy::UnlessStopped => Self::UnlessStopped,
        }
    }
}

impl From<ContainerRestartPolicy> for citadel_deployments::ContainerRestartPolicy {
    fn from(value: ContainerRestartPolicy) -> Self {
        match value {
            ContainerRestartPolicy::No => Self::No,
            ContainerRestartPolicy::Always => Self::Always,
            ContainerRestartPolicy::OnFailure => Self::OnFailure,
            ContainerRestartPolicy::UnlessStopped => Self::UnlessStopped,
        }
    }
}

impl From<citadel_deployments::DeploymentImageInfo> for DeploymentImageInfo {
    fn from(value: citadel_deployments::DeploymentImageInfo) -> Self {
        match value {
            citadel_deployments::DeploymentImageInfo::Local { image_id } => {
                Self::Local { image_id }
            }
            citadel_deployments::DeploymentImageInfo::External {
                registry_id,
                image_tag,
                resolved_digest,
            } => Self::External {
                registry_id,
                image_tag,
                resolved_digest,
            },
            citadel_deployments::DeploymentImageInfo::Build {
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

impl From<DeploymentImageInfo> for citadel_deployments::DeploymentImageInfo {
    fn from(value: DeploymentImageInfo) -> Self {
        match value {
            DeploymentImageInfo::Local { image_id } => Self::Local { image_id },
            DeploymentImageInfo::External {
                registry_id,
                image_tag,
                resolved_digest,
            } => Self::External {
                registry_id,
                image_tag,
                resolved_digest,
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

impl From<citadel_deployments::ResourceSpec> for ResourceSpec {
    fn from(value: citadel_deployments::ResourceSpec) -> Self {
        Self {
            nano_cpus: value.nano_cpus,
            memory_limit: value.memory_limit,
        }
    }
}

impl From<ResourceSpec> for citadel_deployments::ResourceSpec {
    fn from(value: ResourceSpec) -> Self {
        Self {
            nano_cpus: value.nano_cpus,
            memory_limit: value.memory_limit,
        }
    }
}

impl From<citadel_deployments::LifeCycleSpec> for LifeCycleSpec {
    fn from(value: citadel_deployments::LifeCycleSpec) -> Self {
        Self {
            stop_timeout: value.stop_timeout,
            stop_signal: value.stop_signal.map(Into::into),
            restart_policy: value.restart_policy.into(),
        }
    }
}

impl From<LifeCycleSpec> for citadel_deployments::LifeCycleSpec {
    fn from(value: LifeCycleSpec) -> Self {
        Self {
            stop_timeout: value.stop_timeout,
            stop_signal: value.stop_signal.map(Into::into),
            restart_policy: value.restart_policy.into(),
        }
    }
}

impl From<citadel_deployments::DeploymentSpec> for DeploymentSpec {
    fn from(value: citadel_deployments::DeploymentSpec) -> Self {
        Self {
            image: value.image.into(),
            update_behavior: value.update_behavior.into(),
            life_cycle_spec: value.life_cycle_spec.map(Into::into),
            resource_spec: value.resource_spec.map(Into::into),
            labels: value.labels,
            ports: value.ports,
            volumes: value.volumes,
            networks: value.networks,
            command: value.command,
            environment_variables: value.environment_variables,
        }
    }
}

impl From<DeploymentSpec> for citadel_deployments::DeploymentSpec {
    fn from(value: DeploymentSpec) -> Self {
        Self {
            image: value.image.into(),
            update_behavior: value.update_behavior.into(),
            life_cycle_spec: value.life_cycle_spec.map(Into::into),
            resource_spec: value.resource_spec.map(Into::into),
            labels: value.labels,
            ports: value.ports,
            volumes: value.volumes,
            networks: value.networks,
            command: value.command,
            environment_variables: value.environment_variables,
        }
    }
}

impl TryFrom<citadel_deployments::AutoUpdateState> for AutoUpdateState {
    type Error = serde_json::Error;

    fn try_from(value: citadel_deployments::AutoUpdateState) -> Result<Self, Self::Error> {
        Ok(Self {
            last_checked_at: value.last_checked_at,
            status: serde_json::from_value(value.status.into())?,
            current_digest: value.current_digest,
            remote_digest: value.remote_digest,
            last_error: value.last_error,
        })
    }
}

impl From<AutoUpdateState> for citadel_deployments::AutoUpdateState {
    fn from(value: AutoUpdateState) -> Self {
        Self {
            last_checked_at: value.last_checked_at,
            status: value.status.as_str().to_owned(),
            current_digest: value.current_digest,
            remote_digest: value.remote_digest,
            last_error: value.last_error,
        }
    }
}

impl From<citadel_deployments::DuplicateWarning> for DuplicateWarning {
    fn from(value: citadel_deployments::DuplicateWarning) -> Self {
        Self {
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

impl From<DuplicateWarning> for citadel_deployments::DuplicateWarning {
    fn from(value: DuplicateWarning) -> Self {
        Self {
            code: value.code,
            message: value.message,
            field_path: value.field_path,
        }
    }
}

impl TryFrom<citadel_deployments::DuplicateSource> for DuplicateSourceInput {
    type Error = serde_json::Error;

    fn try_from(value: citadel_deployments::DuplicateSource) -> Result<Self, Self::Error> {
        Ok(Self {
            resource_type: serde_json::from_value(value.resource_type.into())?,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
        })
    }
}

impl From<DuplicateSourceInput> for citadel_deployments::DuplicateSource {
    fn from(value: DuplicateSourceInput) -> Self {
        Self {
            resource_type: value.resource_type.as_database_str().to_owned(),
            resource_id: value.resource_id,
            resource_name: value.resource_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::resources::deployments::spec::*;
    use serde_json::json;

    #[test]
    fn wire_conversion_preserves_all_spec_fields_and_persisted_provenance() {
        let id = Uuid::from_u128(0x100);
        for image in [
            json!({"$type":"Local", "imageId":"sha256:local"}),
            json!({"$type":"External", "registryId":id, "imageTag":"nginx:latest", "resolvedDigest":"sha256:external"}),
            json!({"$type":"Build", "buildProjectId":id, "redeployOnBuild":true,
                "resolvedImageReference":"registry/image:next", "resolvedDigest":"sha256:next", "resolvedBuildRunId":id,
                "appliedImageReference":"registry/image:current", "appliedDigest":"sha256:current", "appliedBuildRunId":id,
                "appliedAt":"2026-09-19T12:00:00Z"}),
        ] {
            let wire: DeploymentSpec = serde_json::from_value(json!({
                "image":image, "updateBehavior":"autodeploy",
                "lifeCycleSpec":{"stopSignal":"sigterm","restartPolicy":"unlessStopped"},
                "resourceSpec":{"nanoCpus":1.5,"memoryLimit":256.0},
                "labels":{"app":"web"}, "ports":["8080:80"], "volumes":["data:/data"],
                "networks":["bridge"], "command":["serve"], "environmentVariables":["LOG_LEVEL=info"]
            })).unwrap();
            let business: citadel_deployments::DeploymentSpec = wire.clone().into();
            let stored = business.to_storage_value().unwrap();
            assert_eq!(stored["Image"]["$type"], image["$type"]);
            assert!(stored.get("image").is_none());
            let restored = citadel_deployments::DeploymentSpec::from_storage_value(stored).unwrap();
            let roundtrip: DeploymentSpec = restored.into();
            assert_eq!(roundtrip, wire);
            assert_eq!(serde_json::to_value(roundtrip).unwrap()["image"], image);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum DeploymentStatus {
    Unknown,
    Created,
    Pending,
    Applying,
    Healthy,
    Degraded,
    Failed,
    Stopped,
}
