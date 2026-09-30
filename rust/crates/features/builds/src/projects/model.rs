use crate::*;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildArgSpec {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildSecretSpec {
    pub id: String,
    pub secret_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct BuildProject {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub context_path: String,
    pub dockerfile_path: String,
    pub target: Option<String>,
    pub build_args: Vec<BuildArgSpec>,
    pub build_secrets: Vec<BuildSecretSpec>,
    pub builder_kind: String,
    pub platform_id: Option<Uuid>,
    pub build_agent_pool_id: Option<Uuid>,
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Vec<String>,
    pub webhook: Option<citadel_primitives::WebhookConfig>,
    pub timeout_seconds: i32,
    pub retention_run_count: i32,
    pub current_run_id: Option<Uuid>,
    pub control_state: citadel_primitives::ResourceControlState,
    pub control_started_at: Option<i64>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
    pub audit: citadel_primitives::AuditMetadata,

    // Related data populated by full resource reads.
    pub tags: Vec<citadel_tags::TagSummary>,
    pub latest_run: Option<crate::BuildRun>,
}
