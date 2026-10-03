pub use citadel_builds::{BuildAgentPoolValidationStatus, BuildRunStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildArgSpec {
    pub name: String,
    pub value: Option<String>,
}

impl From<citadel_builds::BuildArgSpec> for BuildArgSpec {
    fn from(value: citadel_builds::BuildArgSpec) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}

impl From<BuildArgSpec> for citadel_builds::BuildArgSpec {
    fn from(value: BuildArgSpec) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BuildSecretSpec {
    pub id: String,
    pub secret_id: Uuid,
}

impl From<citadel_builds::BuildSecretSpec> for BuildSecretSpec {
    fn from(value: citadel_builds::BuildSecretSpec) -> Self {
        Self {
            id: value.id,
            secret_id: value.secret_id,
        }
    }
}

impl From<BuildSecretSpec> for citadel_builds::BuildSecretSpec {
    fn from(value: BuildSecretSpec) -> Self {
        Self {
            id: value.id,
            secret_id: value.secret_id,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum BuildProjectBuilderKind {
    #[default]
    Platform,
    BuildAgentPool,
}
impl BuildProjectBuilderKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Platform => "Platform",
            Self::BuildAgentPool => "BuildAgentPool",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum BuildRunTrigger {
    Manual,
    Automation,
    Schedule,
    Webhook,
    Dependency,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum BuildAgentPoolProvider {
    AwsEc2,
    SelfManagedVm,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum BuildAgentPoolConnectionMode {
    #[default]
    InboundAgent,
    EdgeAgent,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum CpuArchitecture {
    #[default]
    Amd64,
    Arm64,
}
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(tag = "$type")]
pub enum BuildAgentPoolProviderSpec {
    AwsEc2(AwsEc2BuildAgentPoolProviderSpec),
    SelfManagedVm(SelfManagedVmBuildAgentPoolProviderSpec),
}
// AWS configuration is stored for review, but the runtime currently only executes
// builds on self-managed pools. Preserve its existing sparse configuration support.
#[derive(Debug, Clone, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct AwsEc2BuildAgentPoolProviderSpec {
    pub region: String,
    pub instance_type: String,
    pub architecture: CpuArchitecture,
    pub ami_id: String,
    pub root_volume_size_gb: i32,
    pub subnet_id: String,
    pub security_group_ids: Vec<String>,
    pub instance_profile_name: Option<String>,
    pub assign_public_ip: bool,
    pub aws_credential_secret_id: Option<Uuid>,
    pub assume_role_arn: Option<String>,
    pub key_pair_name: Option<String>,
    pub tags: Option<std::collections::BTreeMap<String, String>>,
}
#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelfManagedVmBuildAgentPoolProviderSpec {
    pub endpoint: Option<String>,
    #[serde(default)]
    pub architecture: CpuArchitecture,
    #[serde(default = "one_worker")]
    pub max_workers: i32,
    pub registration_secret_id: Option<Uuid>,
    pub labels: Option<Vec<String>>,
    #[serde(default)]
    pub connection_mode: BuildAgentPoolConnectionMode,
}
fn one_worker() -> i32 {
    1
}
