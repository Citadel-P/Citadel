use crate::api::resources::common::DuplicateSourceInput;
pub use citadel_primitives::WebhookConfig;
pub use citadel_swarm_services::{
    MountKind, PortPublishMode, RestartCondition, SchedulingMode, SwarmServiceConfigReference,
    SwarmServiceHealthCheck, SwarmServiceImageInfo, SwarmServiceMount, SwarmServicePort,
    SwarmServiceResources, SwarmServiceRestartPolicy, SwarmServiceSecretReference,
    SwarmServiceUpdatePolicy, UpdateBehavior, UpdateFailureAction, UpdateOrder,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceSpec {
    #[schema(value_type = crate::api::resources::schema_models::swarm_services::SwarmServiceImageInfoSchema)]
    pub image: SwarmServiceImageInfo,
    #[serde(default)]
    #[schema(value_type = crate::api::resources::schema_models::primitives::UpdateBehaviorSchema)]
    pub update_behavior: UpdateBehavior,
    #[serde(default)]
    #[schema(value_type = crate::api::resources::schema_models::swarm_services::SchedulingModeSchema)]
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
    #[schema(value_type = Option<crate::api::resources::schema_models::swarm_services::SwarmServiceHealthCheckSchema>)]
    pub health_check: Option<SwarmServiceHealthCheck>,
    pub stop_grace_period_nanoseconds: Option<i64>,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::schema_models::swarm_services::SwarmServicePortSchema>)]
    pub ports: Vec<SwarmServicePort>,
    #[serde(default)]
    pub network_ids: Vec<String>,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::schema_models::swarm_services::SwarmServiceMountSchema>)]
    pub mounts: Vec<SwarmServiceMount>,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::schema_models::swarm_services::SwarmServiceSecretReferenceSchema>)]
    pub secrets: Vec<SwarmServiceSecretReference>,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::schema_models::swarm_services::SwarmServiceConfigReferenceSchema>)]
    pub configs: Vec<SwarmServiceConfigReference>,
    #[schema(value_type = Option<crate::api::resources::schema_models::swarm_services::SwarmServiceResourcesSchema>)]
    pub resources: Option<SwarmServiceResources>,
    #[serde(default)]
    pub placement_constraints: Vec<String>,
    #[schema(value_type = Option<crate::api::resources::schema_models::swarm_services::SwarmServiceRestartPolicySchema>)]
    pub restart_policy: Option<SwarmServiceRestartPolicy>,
    #[schema(value_type = Option<crate::api::resources::schema_models::swarm_services::SwarmServiceUpdatePolicySchema>)]
    pub update_policy: Option<SwarmServiceUpdatePolicy>,
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookConfigSchema>)]
    pub webhook: Option<WebhookConfig>,
}

impl TryFrom<citadel_swarm_services::SwarmServiceSpec> for SwarmServiceSpec {
    type Error = serde_json::Error;

    fn try_from(value: citadel_swarm_services::SwarmServiceSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            image: value.image,
            update_behavior: value.update_behavior,
            scheduling_mode: value.scheduling_mode,
            replicas: value.replicas,
            command: value.command,
            arguments: value.arguments,
            environment: value.environment,
            labels: value.labels,
            user: value.user,
            working_directory: value.working_directory,
            health_check: value.health_check,
            stop_grace_period_nanoseconds: value.stop_grace_period_nanoseconds,
            ports: value.ports,
            network_ids: value.network_ids,
            mounts: value.mounts,
            secrets: value.secrets,
            configs: value.configs,
            resources: value.resources,
            placement_constraints: value.placement_constraints,
            restart_policy: value.restart_policy,
            update_policy: value.update_policy,
            webhook: value.webhook,
        })
    }
}

impl From<SwarmServiceSpec> for citadel_swarm_services::SwarmServiceSpec {
    fn from(value: SwarmServiceSpec) -> Self {
        Self {
            image: value.image,
            update_behavior: value.update_behavior,
            scheduling_mode: value.scheduling_mode,
            replicas: value.replicas,
            command: value.command,
            arguments: value.arguments,
            environment: value.environment,
            labels: value.labels,
            user: value.user,
            working_directory: value.working_directory,
            health_check: value.health_check,
            stop_grace_period_nanoseconds: value.stop_grace_period_nanoseconds,
            ports: value.ports,
            network_ids: value.network_ids,
            mounts: value.mounts,
            secrets: value.secrets,
            configs: value.configs,
            resources: value.resources,
            placement_constraints: value.placement_constraints,
            restart_policy: value.restart_policy,
            update_policy: value.update_policy,
            webhook: value.webhook,
        }
    }
}

impl TryFrom<citadel_swarm_services::SwarmServiceDuplicateSource> for DuplicateSourceInput {
    type Error = serde_json::Error;

    fn try_from(
        value: citadel_swarm_services::SwarmServiceDuplicateSource,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            resource_id: value.resource_id,
            resource_type: serde_json::from_value(value.resource_type.into())?,
            resource_name: value.resource_name,
        })
    }
}

impl From<DuplicateSourceInput> for citadel_swarm_services::SwarmServiceDuplicateSource {
    fn from(value: DuplicateSourceInput) -> Self {
        Self {
            resource_id: value.resource_id,
            resource_type: value.resource_type.as_database_str().to_owned(),
            resource_name: value.resource_name,
        }
    }
}

const fn one_option() -> Option<i32> {
    Some(1)
}
