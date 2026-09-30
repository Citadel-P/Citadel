use super::spec::*;
use citadel_primitives::PatchField;
use citadel_primitives::WebhookPatch;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupRepositoryInput {
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    description: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<crate::api::resources::schema_models::backups::BackupRepositorySpecSchema>, required = false)]
    spec: PatchField<BackupRepositorySpec>,
}

#[derive(Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackupPolicyInput {
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    description: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<crate::api::resources::schema_models::backups::BackupSourceSpecSchema>, required = false)]
    source: PatchField<BackupSourceSpec>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<Uuid>, required = false)]
    backup_repository_id: PatchField<Uuid>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<bool>, required = false)]
    enabled: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    cron: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    time_zone: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookPatchSchema>, required = false)]
    webhook: PatchField<WebhookPatch>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    keep_last_successful: PatchField<i32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<i32>, required = false)]
    timeout_seconds: PatchField<i32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<bool>, required = false)]
    alert_on_failure: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<Uuid>, required = false)]
    run_as_actor_id: PatchField<Uuid>,
}

#[derive(Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupMetadataPatch {
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    description: PatchField<String>,
}
