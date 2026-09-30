//! Shared Deployment specifications and API-specific response projections.
use crate::api::resources::common::DuplicateSourceInput;
pub use citadel_deployments::DeploymentStatus;
pub use citadel_deployments::{
    ContainerRestartPolicy, DeploymentImageInfo, DeploymentSpec, LifeCycleSpec, ResourceSpec,
    StopSignal, UpdateBehavior,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
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
        let id = uuid::Uuid::from_u128(0x100);
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
