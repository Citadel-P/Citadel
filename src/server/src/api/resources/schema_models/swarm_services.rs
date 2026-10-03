//! Server-owned OpenAPI descriptions of swarm-services values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use uuid::Uuid;

enum_schema!(
    SwarmServiceHealthSchema,
    "SwarmServiceHealth",
    citadel_swarm_services::SwarmServiceHealth,
    [
        Unknown,
        Healthy,
        Progressing,
        Degraded,
        Failed,
        Created,
        Stopped
    ]
);

enum_schema!(
    SwarmServiceSynchronizationStateSchema,
    "SwarmServiceSynchronizationState",
    citadel_swarm_services::SwarmServiceSynchronizationState,
    [
        NeverApplied,
        DesiredChangesPending,
        InSync,
        Drifted,
        RuntimeMissing,
        OutcomeUnknown,
        OwnershipConflict
    ]
);

enum_schema!(
    SwarmServiceOperationKindSchema,
    "SwarmServiceOperationKind",
    citadel_swarm_services::SwarmServiceOperationKind,
    [Apply, Scale, ForceUpdate, Delete]
);

enum_schema!(
    SwarmServiceOperationStateSchema,
    "SwarmServiceOperationState",
    citadel_swarm_services::SwarmServiceOperationState,
    [
        Prepared,
        Canceled,
        PendingAcceptance,
        Accepted,
        Rejected,
        NotAccepted,
        OutcomeUnknown,
        Completed,
        OwnershipConflict
    ]
);

enum_schema!(
    SwarmServiceOwnershipSchema,
    "SwarmServiceOwnership",
    citadel_swarm_services::SwarmServiceOwnership,
    [
        Unmanaged,
        DockerStackExternal,
        CitadelService,
        CitadelStack,
        OwnershipConflict,
        System
    ]
);

enum_schema!(
    SchedulingModeSchema,
    "SchedulingMode",
    citadel_swarm_services::SchedulingMode,
    [Replicated, Global]
);

enum_schema!(
    PortPublishModeSchema,
    "PortPublishMode",
    citadel_swarm_services::PortPublishMode,
    [Ingress, Host]
);

enum_schema!(
    MountKindSchema,
    "MountKind",
    citadel_swarm_services::MountKind,
    [Volume, Bind, Tmpfs]
);

enum_schema!(
    RestartConditionSchema,
    "RestartCondition",
    citadel_swarm_services::RestartCondition,
    [None, OnFailure, Any]
);

enum_schema!(
    UpdateOrderSchema,
    "UpdateOrder",
    citadel_swarm_services::UpdateOrder,
    [StopFirst, StartFirst]
);

enum_schema!(
    UpdateFailureActionSchema,
    "UpdateFailureAction",
    citadel_swarm_services::UpdateFailureAction,
    [Continue, Pause, Rollback]
);

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
#[schema(as = SwarmServiceImageInfo)]
pub enum SwarmServiceImageInfoSchema {
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

impl From<citadel_swarm_services::SwarmServiceImageInfo> for SwarmServiceImageInfoSchema {
    fn from(value: citadel_swarm_services::SwarmServiceImageInfo) -> Self {
        match value {
            citadel_swarm_services::SwarmServiceImageInfo::External {
                registry_id,
                image_tag,
                resolved_digest,
            } => Self::External {
                registry_id,
                image_tag,
                resolved_digest,
            },
            citadel_swarm_services::SwarmServiceImageInfo::Build {
                build_project_id,
                resolved_image_reference,
                resolved_digest,
                resolved_build_run_id,
            } => Self::Build {
                build_project_id,
                resolved_image_reference,
                resolved_digest,
                resolved_build_run_id,
            },
        }
    }
}

schema_model! {
    citadel_swarm_services::SwarmServicePort =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServicePort)]
    pub struct SwarmServicePortSchema {
        pub target_port: i32,
        #[serde(default)]
        pub published_port: Option<i32>,
        #[serde(default = "tcp")]
        pub protocol: String,
        #[serde(default)]
        #[schema(value_type = crate::api::resources::schema_models::swarm_services::PortPublishModeSchema)]
        pub publish_mode: citadel_swarm_services::PortPublishMode,
    }
}

schema_model! {
    citadel_swarm_services::SwarmServiceMount =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServiceMount)]
    pub struct SwarmServiceMountSchema {
        #[schema(value_type = crate::api::resources::schema_models::swarm_services::MountKindSchema)]
        pub kind: citadel_swarm_services::MountKind,
        pub source: String,
        pub target: String,
        #[serde(default)]
        pub read_only: bool,
    }
}

schema_model! {
    citadel_swarm_services::SwarmServiceSecretReference =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServiceSecretReference)]
    pub struct SwarmServiceSecretReferenceSchema {
        pub secret_id: String,
        pub secret_name: String,
        pub target_name: String,
    }
}

schema_model! {
    citadel_swarm_services::SwarmServiceConfigReference =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServiceConfigReference)]
    pub struct SwarmServiceConfigReferenceSchema {
        pub config_id: String,
        pub config_name: String,
        pub target_name: String,
    }
}

schema_model! {
    citadel_swarm_services::SwarmServiceResources =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServiceResources)]
    pub struct SwarmServiceResourcesSchema {
        pub limit_nano_cpus: Option<i64>,
        pub limit_memory_bytes: Option<i64>,
        pub reservation_nano_cpus: Option<i64>,
        pub reservation_memory_bytes: Option<i64>,
    }
}

schema_model! {
    citadel_swarm_services::SwarmServiceHealthCheck =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServiceHealthCheck)]
    pub struct SwarmServiceHealthCheckSchema {
        pub test: Vec<String>,
        pub interval_nanoseconds: Option<i64>,
        pub timeout_nanoseconds: Option<i64>,
        pub retries: Option<i32>,
        pub start_period_nanoseconds: Option<i64>,
    }
}

schema_model! {
    citadel_swarm_services::SwarmServiceRestartPolicy =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServiceRestartPolicy)]
    pub struct SwarmServiceRestartPolicySchema {
        #[serde(default)]
        #[schema(value_type = crate::api::resources::schema_models::swarm_services::RestartConditionSchema)]
        pub condition: citadel_swarm_services::RestartCondition,
        pub delay_nanoseconds: Option<i64>,
        pub maximum_attempts: Option<i32>,
        pub window_nanoseconds: Option<i64>,
    }
}

schema_model! {
    citadel_swarm_services::SwarmServiceUpdatePolicy =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmServiceUpdatePolicy)]
    pub struct SwarmServiceUpdatePolicySchema {
        #[serde(default = "one")]
        pub parallelism: i32,
        pub delay_nanoseconds: Option<i64>,
        #[serde(default)]
        #[schema(value_type = crate::api::resources::schema_models::swarm_services::UpdateOrderSchema)]
        pub order: citadel_swarm_services::UpdateOrder,
        #[serde(default)]
        #[schema(value_type = crate::api::resources::schema_models::swarm_services::UpdateFailureActionSchema)]
        pub failure_action: citadel_swarm_services::UpdateFailureAction,
    }
}
