use super::spec::*;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
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
    pub duplicate_source: Option<Value>,
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
            duplicate_source: value.duplicate_source.map(|source| serde_json::json!({"resourceType":"Stack", "resourceId":source.id, "resourceName":source.name})),
        }
    }
}
impl TryFrom<CreateStackInput> for citadel_stacks::CreateStack {
    type Error = citadel_stacks::StackError;
    fn try_from(value: CreateStackInput) -> Result<Self, Self::Error> {
        let duplicate_source = value
            .duplicate_source
            .map(|source| {
                let id = source
                    .get("resourceId")
                    .or_else(|| source.get("ResourceId"))
                    .and_then(Value::as_str)
                    .and_then(|id| Uuid::parse_str(id).ok())
                    .ok_or_else(|| {
                        citadel_stacks::StackError::Validation(
                            "Duplicate source is invalid.".into(),
                        )
                    })?;
                let name = source
                    .get("resourceName")
                    .or_else(|| source.get("ResourceName"))
                    .and_then(Value::as_str)
                    .unwrap_or("Stack")
                    .to_owned();
                Ok::<_, citadel_stacks::StackError>(citadel_stacks::DuplicateStackSource {
                    id,
                    name,
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
