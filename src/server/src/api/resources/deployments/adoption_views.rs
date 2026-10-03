use crate::api::resources::deployments::spec::DeploymentSpec;
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionSource {
    pub id: Uuid,
    pub docker_container_id: String,
    pub name: String,
    pub platform_id: Uuid,
    pub platform_name: String,
    pub state: String,
}

impl From<citadel_deployments::adoption::AdoptionSource> for AdoptionSource {
    fn from(value: citadel_deployments::adoption::AdoptionSource) -> Self {
        Self {
            id: value.id,
            docker_container_id: value.docker_container_id,
            name: value.name,
            platform_id: value.platform_id,
            platform_name: value.platform_name,
            state: value.state,
        }
    }
}

#[derive(Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionIssue {
    pub code: String,
    pub message: String,
    pub severity: AdoptionIssueSeverity,
    #[schema(required = true)]
    pub field_path: Option<String>,
}

impl TryFrom<citadel_deployments::adoption::AdoptionIssue> for AdoptionIssue {
    type Error = serde_json::Error;

    fn try_from(value: citadel_deployments::adoption::AdoptionIssue) -> Result<Self, Self::Error> {
        Ok(Self {
            code: value.code,
            message: value.message,
            severity: serde_json::from_value(value.severity.into())?,
            field_path: value.field_path,
        })
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionDeploymentDraft {
    pub name: String,
    pub platform_id: Uuid,
    #[schema(required = true)]
    pub description: Option<String>,
    #[schema(value_type = crate::api::resources::schema_models::deployments::DeploymentSpecSchema)]
    pub spec: DeploymentSpec,
    pub tag_ids: Vec<Uuid>,
}

impl From<citadel_deployments::adoption::AdoptionDeploymentDraft> for AdoptionDeploymentDraft {
    fn from(value: citadel_deployments::adoption::AdoptionDeploymentDraft) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec,
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerAdoptionDraft {
    pub source: AdoptionSource,
    pub draft: AdoptionDeploymentDraft,
    pub issues: Vec<AdoptionIssue>,
    pub preview_fingerprint: String,
    pub can_import_sensitive_environment_values: bool,
}

impl TryFrom<citadel_deployments::adoption::ContainerAdoptionDraft> for ContainerAdoptionDraft {
    type Error = serde_json::Error;

    fn try_from(
        value: citadel_deployments::adoption::ContainerAdoptionDraft,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            source: value.source.into(),
            draft: value.draft.into(),
            issues: value
                .issues
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            preview_fingerprint: value.preview_fingerprint,
            can_import_sensitive_environment_values: value.can_import_sensitive_environment_values,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize, utoipa::ToSchema)]
pub enum AdoptionIssueSeverity {
    Warning,
    Blocker,
}
