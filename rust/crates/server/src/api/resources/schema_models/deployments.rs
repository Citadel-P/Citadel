//! Server-owned OpenAPI descriptions of deployments values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use std::collections::BTreeMap;

enum_schema!(
    DeploymentStatusSchema,
    "DeploymentStatus",
    citadel_deployments::DeploymentStatus,
    [
        Unknown, Created, Pending, Applying, Healthy, Degraded, Failed, Stopped
    ]
);

enum_schema!(
    StopSignalSchema,
    "StopSignal",
    citadel_deployments::StopSignal,
    [SIGTERM, SIGKILL, SIGINT, SIGQUIT]
);

enum_schema!(
    ContainerRestartPolicySchema,
    "ContainerRestartPolicy",
    citadel_deployments::ContainerRestartPolicy,
    [No, Always, OnFailure, UnlessStopped]
);

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
#[schema(as = DeploymentImageInfo)]
pub enum DeploymentImageInfoSchema {
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

impl From<citadel_deployments::DeploymentImageInfo> for DeploymentImageInfoSchema {
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

schema_model! {
    citadel_deployments::ResourceSpec =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ResourceSpec)]
    pub struct ResourceSpecSchema {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub nano_cpus: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub memory_limit: Option<f32>,
    }
}

schema_model! {
    citadel_deployments::LifeCycleSpec =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = LifeCycleSpec)]
    pub struct LifeCycleSpecSchema {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stop_timeout: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(value_type = Option < crate::api::resources::schema_models::deployments::StopSignalSchema >)]
        pub stop_signal: Option<citadel_deployments::StopSignal>,
        #[serde(default)]
        #[schema(value_type = crate::api::resources::schema_models::deployments::ContainerRestartPolicySchema)]
        pub restart_policy: citadel_deployments::ContainerRestartPolicy,
    }
}

schema_model! {
    citadel_deployments::DeploymentSpec =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = DeploymentSpec)]
    pub struct DeploymentSpecSchema {
        #[schema(value_type = crate::api::resources::schema_models::deployments::DeploymentImageInfoSchema)]
        pub image: citadel_deployments::DeploymentImageInfo,
        #[serde(default)]
        # [schema (default = citadel_deployments::UpdateBehavior::default)]
        #[schema(value_type = crate::api::resources::schema_models::primitives::UpdateBehaviorSchema)]
        pub update_behavior: citadel_deployments::UpdateBehavior,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(value_type = Option < crate::api::resources::schema_models::deployments::LifeCycleSpecSchema >)]
        pub life_cycle_spec: Option<citadel_deployments::LifeCycleSpec>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(value_type = Option < crate::api::resources::schema_models::deployments::ResourceSpecSchema >)]
        pub resource_spec: Option<citadel_deployments::ResourceSpec>,
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
}
