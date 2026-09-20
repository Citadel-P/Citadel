use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = swarm_services::model::UpdateBehavior)]
pub enum UpdateBehavior {
    #[default]
    Disabled,
    Notify,
    AutoDeploy,
}

impl From<citadel_swarm_services::UpdateBehavior> for UpdateBehavior {
    fn from(value: citadel_swarm_services::UpdateBehavior) -> Self {
        match value {
            citadel_swarm_services::UpdateBehavior::Disabled => Self::Disabled,
            citadel_swarm_services::UpdateBehavior::Notify => Self::Notify,
            citadel_swarm_services::UpdateBehavior::AutoDeploy => Self::AutoDeploy,
        }
    }
}

impl From<UpdateBehavior> for citadel_swarm_services::UpdateBehavior {
    fn from(value: UpdateBehavior) -> Self {
        match value {
            UpdateBehavior::Disabled => Self::Disabled,
            UpdateBehavior::Notify => Self::Notify,
            UpdateBehavior::AutoDeploy => Self::AutoDeploy,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum SchedulingMode {
    #[default]
    Replicated,
    Global,
}

impl From<citadel_swarm_services::SchedulingMode> for SchedulingMode {
    fn from(value: citadel_swarm_services::SchedulingMode) -> Self {
        match value {
            citadel_swarm_services::SchedulingMode::Replicated => Self::Replicated,
            citadel_swarm_services::SchedulingMode::Global => Self::Global,
        }
    }
}

impl From<SchedulingMode> for citadel_swarm_services::SchedulingMode {
    fn from(value: SchedulingMode) -> Self {
        match value {
            SchedulingMode::Replicated => Self::Replicated,
            SchedulingMode::Global => Self::Global,
        }
    }
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

impl From<citadel_swarm_services::PortPublishMode> for PortPublishMode {
    fn from(value: citadel_swarm_services::PortPublishMode) -> Self {
        match value {
            citadel_swarm_services::PortPublishMode::Ingress => Self::Ingress,
            citadel_swarm_services::PortPublishMode::Host => Self::Host,
        }
    }
}

impl From<PortPublishMode> for citadel_swarm_services::PortPublishMode {
    fn from(value: PortPublishMode) -> Self {
        match value {
            PortPublishMode::Ingress => Self::Ingress,
            PortPublishMode::Host => Self::Host,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum MountKind {
    Volume,
    Bind,
    Tmpfs,
}

impl From<citadel_swarm_services::MountKind> for MountKind {
    fn from(value: citadel_swarm_services::MountKind) -> Self {
        match value {
            citadel_swarm_services::MountKind::Volume => Self::Volume,
            citadel_swarm_services::MountKind::Bind => Self::Bind,
            citadel_swarm_services::MountKind::Tmpfs => Self::Tmpfs,
        }
    }
}

impl From<MountKind> for citadel_swarm_services::MountKind {
    fn from(value: MountKind) -> Self {
        match value {
            MountKind::Volume => Self::Volume,
            MountKind::Bind => Self::Bind,
            MountKind::Tmpfs => Self::Tmpfs,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum RestartCondition {
    None,
    OnFailure,
    #[default]
    Any,
}

impl From<citadel_swarm_services::RestartCondition> for RestartCondition {
    fn from(value: citadel_swarm_services::RestartCondition) -> Self {
        match value {
            citadel_swarm_services::RestartCondition::None => Self::None,
            citadel_swarm_services::RestartCondition::OnFailure => Self::OnFailure,
            citadel_swarm_services::RestartCondition::Any => Self::Any,
        }
    }
}

impl From<RestartCondition> for citadel_swarm_services::RestartCondition {
    fn from(value: RestartCondition) -> Self {
        match value {
            RestartCondition::None => Self::None,
            RestartCondition::OnFailure => Self::OnFailure,
            RestartCondition::Any => Self::Any,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum UpdateOrder {
    #[default]
    StopFirst,
    StartFirst,
}

impl From<citadel_swarm_services::UpdateOrder> for UpdateOrder {
    fn from(value: citadel_swarm_services::UpdateOrder) -> Self {
        match value {
            citadel_swarm_services::UpdateOrder::StopFirst => Self::StopFirst,
            citadel_swarm_services::UpdateOrder::StartFirst => Self::StartFirst,
        }
    }
}

impl From<UpdateOrder> for citadel_swarm_services::UpdateOrder {
    fn from(value: UpdateOrder) -> Self {
        match value {
            UpdateOrder::StopFirst => Self::StopFirst,
            UpdateOrder::StartFirst => Self::StartFirst,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum UpdateFailureAction {
    Continue,
    #[default]
    Pause,
    Rollback,
}

impl From<citadel_swarm_services::UpdateFailureAction> for UpdateFailureAction {
    fn from(value: citadel_swarm_services::UpdateFailureAction) -> Self {
        match value {
            citadel_swarm_services::UpdateFailureAction::Continue => Self::Continue,
            citadel_swarm_services::UpdateFailureAction::Pause => Self::Pause,
            citadel_swarm_services::UpdateFailureAction::Rollback => Self::Rollback,
        }
    }
}

impl From<UpdateFailureAction> for citadel_swarm_services::UpdateFailureAction {
    fn from(value: UpdateFailureAction) -> Self {
        match value {
            UpdateFailureAction::Continue => Self::Continue,
            UpdateFailureAction::Pause => Self::Pause,
            UpdateFailureAction::Rollback => Self::Rollback,
        }
    }
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

impl From<citadel_swarm_services::SwarmServiceImageInfo> for SwarmServiceImageInfo {
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

impl From<SwarmServiceImageInfo> for citadel_swarm_services::SwarmServiceImageInfo {
    fn from(value: SwarmServiceImageInfo) -> Self {
        match value {
            SwarmServiceImageInfo::External {
                registry_id,
                image_tag,
                resolved_digest,
            } => Self::External {
                registry_id,
                image_tag,
                resolved_digest,
            },
            SwarmServiceImageInfo::Build {
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

impl From<citadel_swarm_services::SwarmServicePort> for SwarmServicePort {
    fn from(value: citadel_swarm_services::SwarmServicePort) -> Self {
        Self {
            target_port: value.target_port,
            published_port: value.published_port,
            protocol: value.protocol,
            publish_mode: value.publish_mode.into(),
        }
    }
}

impl From<SwarmServicePort> for citadel_swarm_services::SwarmServicePort {
    fn from(value: SwarmServicePort) -> Self {
        Self {
            target_port: value.target_port,
            published_port: value.published_port,
            protocol: value.protocol,
            publish_mode: value.publish_mode.into(),
        }
    }
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

impl From<citadel_swarm_services::SwarmServiceMount> for SwarmServiceMount {
    fn from(value: citadel_swarm_services::SwarmServiceMount) -> Self {
        Self {
            kind: value.kind.into(),
            source: value.source,
            target: value.target,
            read_only: value.read_only,
        }
    }
}

impl From<SwarmServiceMount> for citadel_swarm_services::SwarmServiceMount {
    fn from(value: SwarmServiceMount) -> Self {
        Self {
            kind: value.kind.into(),
            source: value.source,
            target: value.target,
            read_only: value.read_only,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceSecretReference {
    pub secret_id: String,
    pub secret_name: String,
    pub target_name: String,
}

impl From<citadel_swarm_services::SwarmServiceSecretReference> for SwarmServiceSecretReference {
    fn from(value: citadel_swarm_services::SwarmServiceSecretReference) -> Self {
        Self {
            secret_id: value.secret_id,
            secret_name: value.secret_name,
            target_name: value.target_name,
        }
    }
}

impl From<SwarmServiceSecretReference> for citadel_swarm_services::SwarmServiceSecretReference {
    fn from(value: SwarmServiceSecretReference) -> Self {
        Self {
            secret_id: value.secret_id,
            secret_name: value.secret_name,
            target_name: value.target_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceConfigReference {
    pub config_id: String,
    pub config_name: String,
    pub target_name: String,
}

impl From<citadel_swarm_services::SwarmServiceConfigReference> for SwarmServiceConfigReference {
    fn from(value: citadel_swarm_services::SwarmServiceConfigReference) -> Self {
        Self {
            config_id: value.config_id,
            config_name: value.config_name,
            target_name: value.target_name,
        }
    }
}

impl From<SwarmServiceConfigReference> for citadel_swarm_services::SwarmServiceConfigReference {
    fn from(value: SwarmServiceConfigReference) -> Self {
        Self {
            config_id: value.config_id,
            config_name: value.config_name,
            target_name: value.target_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceResources {
    pub limit_nano_cpus: Option<i64>,
    pub limit_memory_bytes: Option<i64>,
    pub reservation_nano_cpus: Option<i64>,
    pub reservation_memory_bytes: Option<i64>,
}

impl From<citadel_swarm_services::SwarmServiceResources> for SwarmServiceResources {
    fn from(value: citadel_swarm_services::SwarmServiceResources) -> Self {
        Self {
            limit_nano_cpus: value.limit_nano_cpus,
            limit_memory_bytes: value.limit_memory_bytes,
            reservation_nano_cpus: value.reservation_nano_cpus,
            reservation_memory_bytes: value.reservation_memory_bytes,
        }
    }
}

impl From<SwarmServiceResources> for citadel_swarm_services::SwarmServiceResources {
    fn from(value: SwarmServiceResources) -> Self {
        Self {
            limit_nano_cpus: value.limit_nano_cpus,
            limit_memory_bytes: value.limit_memory_bytes,
            reservation_nano_cpus: value.reservation_nano_cpus,
            reservation_memory_bytes: value.reservation_memory_bytes,
        }
    }
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

impl From<citadel_swarm_services::SwarmServiceHealthCheck> for SwarmServiceHealthCheck {
    fn from(value: citadel_swarm_services::SwarmServiceHealthCheck) -> Self {
        Self {
            test: value.test,
            interval_nanoseconds: value.interval_nanoseconds,
            timeout_nanoseconds: value.timeout_nanoseconds,
            retries: value.retries,
            start_period_nanoseconds: value.start_period_nanoseconds,
        }
    }
}

impl From<SwarmServiceHealthCheck> for citadel_swarm_services::SwarmServiceHealthCheck {
    fn from(value: SwarmServiceHealthCheck) -> Self {
        Self {
            test: value.test,
            interval_nanoseconds: value.interval_nanoseconds,
            timeout_nanoseconds: value.timeout_nanoseconds,
            retries: value.retries,
            start_period_nanoseconds: value.start_period_nanoseconds,
        }
    }
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

impl From<citadel_swarm_services::SwarmServiceRestartPolicy> for SwarmServiceRestartPolicy {
    fn from(value: citadel_swarm_services::SwarmServiceRestartPolicy) -> Self {
        Self {
            condition: value.condition.into(),
            delay_nanoseconds: value.delay_nanoseconds,
            maximum_attempts: value.maximum_attempts,
            window_nanoseconds: value.window_nanoseconds,
        }
    }
}

impl From<SwarmServiceRestartPolicy> for citadel_swarm_services::SwarmServiceRestartPolicy {
    fn from(value: SwarmServiceRestartPolicy) -> Self {
        Self {
            condition: value.condition.into(),
            delay_nanoseconds: value.delay_nanoseconds,
            maximum_attempts: value.maximum_attempts,
            window_nanoseconds: value.window_nanoseconds,
        }
    }
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

impl From<citadel_swarm_services::SwarmServiceUpdatePolicy> for SwarmServiceUpdatePolicy {
    fn from(value: citadel_swarm_services::SwarmServiceUpdatePolicy) -> Self {
        Self {
            parallelism: value.parallelism,
            delay_nanoseconds: value.delay_nanoseconds,
            order: value.order.into(),
            failure_action: value.failure_action.into(),
        }
    }
}

impl From<SwarmServiceUpdatePolicy> for citadel_swarm_services::SwarmServiceUpdatePolicy {
    fn from(value: SwarmServiceUpdatePolicy) -> Self {
        Self {
            parallelism: value.parallelism,
            delay_nanoseconds: value.delay_nanoseconds,
            order: value.order.into(),
            failure_action: value.failure_action.into(),
        }
    }
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

impl From<citadel_swarm_services::SwarmServiceWebhookConfig> for SwarmServiceWebhookConfig {
    fn from(value: citadel_swarm_services::SwarmServiceWebhookConfig) -> Self {
        Self {
            enabled: value.enabled,
            provider: value.provider,
            auth_scheme: value.auth_scheme,
            secret: value.secret,
            branch_filter: value.branch_filter,
        }
    }
}

impl From<SwarmServiceWebhookConfig> for citadel_swarm_services::SwarmServiceWebhookConfig {
    fn from(value: SwarmServiceWebhookConfig) -> Self {
        Self {
            enabled: value.enabled,
            provider: value.provider,
            auth_scheme: value.auth_scheme,
            secret: value.secret,
            branch_filter: value.branch_filter,
        }
    }
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

impl From<citadel_swarm_services::SwarmServiceSpec> for SwarmServiceSpec {
    fn from(value: citadel_swarm_services::SwarmServiceSpec) -> Self {
        Self {
            image: value.image.into(),
            update_behavior: value.update_behavior.into(),
            scheduling_mode: value.scheduling_mode.into(),
            replicas: value.replicas,
            command: value.command,
            arguments: value.arguments,
            environment: value.environment,
            labels: value.labels,
            user: value.user,
            working_directory: value.working_directory,
            health_check: value.health_check.map(|item| item.into()),
            stop_grace_period_nanoseconds: value.stop_grace_period_nanoseconds,
            ports: value.ports.into_iter().map(|item| item.into()).collect(),
            network_ids: value.network_ids,
            mounts: value.mounts.into_iter().map(|item| item.into()).collect(),
            secrets: value.secrets.into_iter().map(|item| item.into()).collect(),
            configs: value.configs.into_iter().map(|item| item.into()).collect(),
            resources: value.resources.map(|item| item.into()),
            placement_constraints: value.placement_constraints,
            restart_policy: value.restart_policy.map(|item| item.into()),
            update_policy: value.update_policy.map(|item| item.into()),
            webhook: value.webhook.map(|item| item.into()),
        }
    }
}

impl From<SwarmServiceSpec> for citadel_swarm_services::SwarmServiceSpec {
    fn from(value: SwarmServiceSpec) -> Self {
        Self {
            image: value.image.into(),
            update_behavior: value.update_behavior.into(),
            scheduling_mode: value.scheduling_mode.into(),
            replicas: value.replicas,
            command: value.command,
            arguments: value.arguments,
            environment: value.environment,
            labels: value.labels,
            user: value.user,
            working_directory: value.working_directory,
            health_check: value.health_check.map(|item| item.into()),
            stop_grace_period_nanoseconds: value.stop_grace_period_nanoseconds,
            ports: value.ports.into_iter().map(|item| item.into()).collect(),
            network_ids: value.network_ids,
            mounts: value.mounts.into_iter().map(|item| item.into()).collect(),
            secrets: value.secrets.into_iter().map(|item| item.into()).collect(),
            configs: value.configs.into_iter().map(|item| item.into()).collect(),
            resources: value.resources.map(|item| item.into()),
            placement_constraints: value.placement_constraints,
            restart_policy: value.restart_policy.map(|item| item.into()),
            update_policy: value.update_policy.map(|item| item.into()),
            webhook: value.webhook.map(|item| item.into()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwarmServiceDuplicateSource {
    pub resource_id: Uuid,
    pub resource_type: String,
    pub resource_name: String,
}

impl From<citadel_swarm_services::SwarmServiceDuplicateSource> for SwarmServiceDuplicateSource {
    fn from(value: citadel_swarm_services::SwarmServiceDuplicateSource) -> Self {
        Self {
            resource_id: value.resource_id,
            resource_type: value.resource_type,
            resource_name: value.resource_name,
        }
    }
}

impl From<SwarmServiceDuplicateSource> for citadel_swarm_services::SwarmServiceDuplicateSource {
    fn from(value: SwarmServiceDuplicateSource) -> Self {
        Self {
            resource_id: value.resource_id,
            resource_type: value.resource_type,
            resource_name: value.resource_name,
        }
    }
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

impl From<citadel_swarm_services::AutoUpdateState> for AutoUpdateState {
    fn from(value: citadel_swarm_services::AutoUpdateState) -> Self {
        Self {
            last_checked_at: value.last_checked_at,
            status: value.status,
            current_digest: value.current_digest,
            remote_digest: value.remote_digest,
            last_error: value.last_error,
        }
    }
}

impl From<AutoUpdateState> for citadel_swarm_services::AutoUpdateState {
    fn from(value: AutoUpdateState) -> Self {
        Self {
            last_checked_at: value.last_checked_at,
            status: value.status,
            current_digest: value.current_digest,
            remote_digest: value.remote_digest,
            last_error: value.last_error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = swarm_services::model::TagSummary)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

impl From<citadel_swarm_services::TagSummary> for TagSummary {
    fn from(value: citadel_swarm_services::TagSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            color: value.color,
        }
    }
}

impl From<TagSummary> for citadel_swarm_services::TagSummary {
    fn from(value: TagSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            color: value.color,
        }
    }
}

fn tcp() -> String {
    "tcp".to_owned()
}

fn github_provider() -> String {
    "GitHub".to_owned()
}

fn github_hmac_sha256() -> String {
    "GitHubHmacSha256".to_owned()
}

const fn one() -> i32 {
    1
}

const fn one_option() -> Option<i32> {
    Some(1)
}
