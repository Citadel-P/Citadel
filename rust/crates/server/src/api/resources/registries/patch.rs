use super::spec::RegistryKind;
use citadel_primitives::PatchField;
use serde::{Deserialize, Serialize};

/// Partial provider settings; omission preserves a value and null removes it.
#[derive(Debug, Clone, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistryConfigurationPatch {
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[serde(rename = "$type")]
    #[schema(value_type = Option<RegistryKind>, required = false)]
    pub kind: PatchField<RegistryKind>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<bool>, required = false)]
    pub auth_enabled: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub user_name: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub password: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub pat: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub access_key: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<bool>, required = false)]
    pub authentication_required: PatchField<bool>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub secret_access_key: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub region: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub instance_url: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub name_space: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<bool>, required = false)]
    pub ghcr_auth_enabled: PatchField<bool>,
}
