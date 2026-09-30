use crate::api::resources::common::DuplicateSourceInput;
use crate::api::resources::stacks::spec::{
    StackDriftMode, StackDriftPolicy, StackImportKind, StackSource, StackSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateStackInput {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub stack_source: StackSource,
    pub spec: StackSpec,
    pub drift_policy: Option<StackDriftPolicy>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub duplicate_source: Option<DuplicateSourceInput>,
}

impl From<citadel_stacks::CreateStack> for CreateStackInput {
    fn from(value: citadel_stacks::CreateStack) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            stack_source: value.stack_source.into(),
            spec: value.spec.into(),
            drift_policy: value.drift_policy.map(|item| item.into()),
            tag_ids: value.tag_ids,
            duplicate_source: value.duplicate_source.map(|source| DuplicateSourceInput {
                resource_type: citadel_activities::ActivityResourceType::Stack,
                resource_id: source.id,
                resource_name: source.name,
            }),
        }
    }
}

impl TryFrom<CreateStackInput> for citadel_stacks::CreateStack {
    type Error = citadel_stacks::StackError;
    fn try_from(value: CreateStackInput) -> Result<Self, Self::Error> {
        let duplicate_source = value
            .duplicate_source
            .map(|source| {
                if source.resource_type != citadel_activities::ActivityResourceType::Stack
                    || source.resource_id.is_nil()
                {
                    return Err(citadel_stacks::StackError::Validation(
                        "Duplicate source must identify a Stack.".into(),
                    ));
                }
                Ok(citadel_stacks::DuplicateStackSource {
                    id: source.resource_id,
                    name: source.resource_name,
                })
            })
            .transpose()?;
        Ok(Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            stack_source: value.stack_source.into(),
            spec: value.spec.into(),
            drift_policy: value.drift_policy.map(Into::into),
            tag_ids: value.tag_ids,
            duplicate_source,
        })
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchStackInput {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub platform_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_present_nullable_string")]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub stack_source: Option<StackSource>,
    #[serde(default)]
    pub spec: Option<Value>,
    #[serde(default)]
    pub drift_policy: Option<StackDriftPolicy>,
    pub row_version: Option<i64>,
}

impl From<citadel_stacks::UpdateStack> for PatchStackInput {
    fn from(value: citadel_stacks::UpdateStack) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            stack_source: value.stack_source.map(|item| item.into()),
            spec: value.spec,
            drift_policy: value.drift_policy.map(|item| item.into()),
            row_version: value.row_version,
        }
    }
}

impl From<PatchStackInput> for citadel_stacks::UpdateStack {
    fn from(value: PatchStackInput) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            stack_source: value.stack_source.map(|item| item.into()),
            spec: value.spec,
            drift_policy: value.drift_policy.map(|item| item.into()),
            row_version: value.row_version,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameStackInput {
    pub id: Uuid,
    pub name: String,
}

impl From<citadel_stacks::RenameStack> for RenameStackInput {
    fn from(value: citadel_stacks::RenameStack) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<RenameStackInput> for citadel_stacks::RenameStack {
    fn from(value: RenameStackInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyStackInput {
    pub id: Uuid,
    #[serde(default)]
    pub recreate: Option<bool>,
}

impl From<citadel_stacks::ApplyStack> for ApplyStackInput {
    fn from(value: citadel_stacks::ApplyStack) -> Self {
        Self {
            id: value.id,
            recreate: value.recreate,
        }
    }
}

impl From<ApplyStackInput> for citadel_stacks::ApplyStack {
    fn from(value: ApplyStackInput) -> Self {
        Self {
            id: value.id,
            recreate: value.recreate,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RollbackStackInput {
    pub stack_id: Uuid,
    pub release_id: Uuid,
}

impl From<citadel_stacks::RollbackStack> for RollbackStackInput {
    fn from(value: citadel_stacks::RollbackStack) -> Self {
        Self {
            stack_id: value.stack_id,
            release_id: value.release_id,
        }
    }
}

impl From<RollbackStackInput> for citadel_stacks::RollbackStack {
    fn from(value: RollbackStackInput) -> Self {
        Self {
            stack_id: value.stack_id,
            release_id: value.release_id,
        }
    }
}

fn deserialize_present_nullable_string<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SwarmPreflightInput {
    pub(crate) name: String,
    pub(crate) platform_id: Uuid,
    pub(crate) stack_source: StackSource,
    pub(crate) spec: StackSpec,
    pub(crate) drift_policy: Option<StackDriftPolicy>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportDraftQuery {
    pub(crate) import_kind: Option<StackImportKind>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ValidateImportRequest {
    pub(crate) name: String,
    pub(crate) stack_source: StackSource,
    pub(crate) spec: StackSpec,
    pub(crate) import_kind: Option<StackImportKind>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportRequest {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) stack_source: StackSource,
    pub(crate) spec: StackSpec,
    pub(crate) preview_fingerprint: String,
    #[serde(default)]
    pub(crate) tag_ids: Vec<Uuid>,
    pub(crate) import_kind: Option<StackImportKind>,
    #[serde(default)]
    pub(crate) import_sensitive_environment_as_secrets: bool,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StackDriftPolicyInput {
    pub(crate) mode: Option<StackDriftMode>,
    pub(crate) alert_on_drift: Option<bool>,
    pub(crate) mark_degraded: Option<bool>,
    pub(crate) auto_start_stopped_containers: Option<bool>,
    pub(crate) auto_resume_paused_containers: Option<bool>,
    pub(crate) remove_extra_containers: Option<bool>,
}

impl From<StackDriftPolicyInput> for citadel_stacks::StackDriftPolicy {
    fn from(value: StackDriftPolicyInput) -> Self {
        let defaults = Self::default();
        Self {
            mode: value.mode.map(Into::into).unwrap_or(defaults.mode),
            alert_on_drift: value.alert_on_drift.unwrap_or(defaults.alert_on_drift),
            mark_degraded: value.mark_degraded.unwrap_or(defaults.mark_degraded),
            auto_start_stopped_containers: value
                .auto_start_stopped_containers
                .unwrap_or(defaults.auto_start_stopped_containers),
            auto_resume_paused_containers: value
                .auto_resume_paused_containers
                .unwrap_or(defaults.auto_resume_paused_containers),
            remove_extra_containers: value
                .remove_extra_containers
                .unwrap_or(defaults.remove_extra_containers),
        }
        .normalized()
    }
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchStackMetadataInput {
    #[serde(default, deserialize_with = "deserialize_present_nullable_string")]
    #[schema(value_type = Option<String>, required = false)]
    pub description: Option<Option<String>>,
}
