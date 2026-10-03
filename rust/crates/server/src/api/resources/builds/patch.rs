use super::spec::*;
use crate::api::error::ApiError;
use citadel_primitives::WebhookPatch;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use citadel_primitives::FieldUpdate;

#[derive(Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateBuildProjectInput {
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    description: FieldUpdate<Option<String>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = bool, required = false)]
    enabled: FieldUpdate<bool>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Uuid, required = false)]
    git_repository_id: FieldUpdate<Uuid>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    branch: FieldUpdate<Option<String>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    context_path: FieldUpdate<Option<String>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    dockerfile_path: FieldUpdate<Option<String>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    target: FieldUpdate<Option<String>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<Vec<BuildArgSpec>>, required = false)]
    build_args: FieldUpdate<Option<Vec<BuildArgSpec>>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<Vec<BuildSecretSpec>>, required = false)]
    build_secrets: FieldUpdate<Option<Vec<BuildSecretSpec>>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = BuildProjectBuilderKind, required = false)]
    builder_kind: FieldUpdate<BuildProjectBuilderKind>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<Uuid>, required = false)]
    platform_id: FieldUpdate<Option<Uuid>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<Uuid>, required = false)]
    build_agent_pool_id: FieldUpdate<Option<Uuid>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = bool, required = false)]
    push_to_registry: FieldUpdate<bool>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<Uuid>, required = false)]
    registry_id: FieldUpdate<Option<Uuid>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = String, required = false)]
    image_repository: FieldUpdate<String>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<Vec<String>>, required = false)]
    tag_templates: FieldUpdate<Option<Vec<String>>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookPatchSchema>, required = false)]
    webhook: FieldUpdate<Option<WebhookPatch>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    retention_run_count: FieldUpdate<Option<i32>>,
}

#[derive(Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateBuildAgentPoolInput {
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    description: FieldUpdate<Option<String>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<bool>, required = false)]
    enabled: FieldUpdate<Option<bool>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<BuildAgentPoolProviderSpec>, required = false)]
    provider_spec: FieldUpdate<Option<BuildAgentPoolProviderSpec>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    max_active_builders: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    queue_timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    provisioning_timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    registration_timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    heartbeat_timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    cleanup_timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    maximum_instance_lifetime_seconds: FieldUpdate<Option<i32>>,
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    failure_retention_minutes: FieldUpdate<Option<i32>>,
}

#[derive(Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildMetadataPatch {
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    description: FieldUpdate<Option<String>>,
}

pub(crate) fn typed_patch<T: serde::de::DeserializeOwned + serde::Serialize>(
    patch: serde_json::Value,
) -> Result<serde_json::Value, ApiError> {
    if !patch.is_object() {
        return Err(ApiError::Validation(
            "Build update must be an object.".into(),
        ));
    }
    let patch: T =
        serde_json::from_value(patch).map_err(|error| ApiError::Validation(error.to_string()))?;
    serde_json::to_value(patch).map_err(ApiError::internal)
}
