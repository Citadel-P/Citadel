use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BuildProjectActivitySnapshot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub context_path: String,
    pub dockerfile_path: String,
    pub target: Option<String>,
    pub builder_kind: String,
    pub platform_id: Option<Uuid>,
    pub build_agent_pool_id: Option<Uuid>,
    pub registry_id: Uuid,
    pub image_repository: String,
    pub tag_templates: Vec<String>,
    pub webhook: Option<citadel_primitives::WebhookConfig>,
    pub timeout_seconds: i32,
    pub retention_run_count: i32,
    pub build_secrets: Vec<BuildSecretActivitySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BuildSecretActivitySnapshot {
    pub id: String,
    pub secret_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BuildAgentPoolActivitySnapshot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub provider: String,
    pub provider_spec: Value,
    pub architecture: String,
    pub region: String,
    pub instance_type: String,
    pub max_active_builders: i32,
    pub queue_timeout_seconds: i32,
    pub provisioning_timeout_seconds: i32,
    pub registration_timeout_seconds: i32,
    pub heartbeat_timeout_seconds: i32,
    pub cleanup_timeout_seconds: i32,
    pub maximum_instance_lifetime_seconds: i32,
    pub failure_retention_minutes: i32,
    pub last_validation_status: String,
    pub last_validation_message: Option<String>,
    pub last_validated_at: Option<DateTime<Utc>>,
}
