//! HTTP decoding, including legacy null and PATCH semantics.
use super::spec::{DeploymentSpec, DuplicateSourceInput};
use citadel_deployments::FieldPatch;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDeploymentInput {
    pub name: String,
    pub platform_id: Uuid,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    #[schema(nullable)]
    pub tag_ids: Vec<Uuid>,
    pub duplicate_source: Option<DuplicateSourceInput>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchDeploymentInput {
    // The editor submits the full form. Like .NET, configuration PATCH only
    // applies platform/spec; identity, rename and metadata have their own paths.
    #[serde(rename = "id")]
    pub _id: Option<Uuid>,
    #[serde(rename = "name")]
    pub _name: Option<String>,
    #[serde(rename = "description")]
    pub _description: Option<String>,
    pub platform_id: Option<Uuid>,
    pub spec: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchDeploymentMetadataInput {
    #[serde(default, deserialize_with = "deserialize_field_patch")]
    #[schema(value_type = Option<String>, required = false)]
    pub description: FieldPatch<String>,
    #[serde(default, rename = "tags")]
    pub _tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameDeploymentInput {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyDeploymentInput {
    pub id: Uuid,
    #[serde(default)]
    pub recreate: Option<bool>,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}
impl From<CreateDeploymentInput> for citadel_deployments::CreateDeployment {
    fn from(value: CreateDeploymentInput) -> Self {
        Self {
            name: value.name,
            platform_id: value.platform_id,
            description: value.description,
            spec: value.spec.into(),
            tag_ids: value.tag_ids,
            duplicate_source: value.duplicate_source.map(Into::into),
        }
    }
}
impl From<PatchDeploymentMetadataInput> for citadel_deployments::UpdateDeploymentMetadata {
    fn from(value: PatchDeploymentMetadataInput) -> Self {
        Self {
            description: value.description,
        }
    }
}

impl From<AdoptContainerInput> for citadel_deployments::adoption::AdoptContainer {
    fn from(value: AdoptContainerInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            spec: value.spec.into(),
            preview_fingerprint: value.preview_fingerprint,
            tag_ids: value.tag_ids,
            import_sensitive_environment_as_secrets: value.import_sensitive_environment_as_secrets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::spec::UpdateBehavior;
    use super::*;
    #[test]
    fn create_contract_accepts_null_tag_ids() {
        let input = serde_json::from_value::<CreateDeploymentInput>(serde_json::json!({
            "name": "web",
            "platformId": Uuid::now_v7(),
            "description": null,
            "spec": {
                "image": { "$type": "Local", "imageId": "image" },
                "updateBehavior": "Disabled",
                "lifeCycleSpec": null,
                "resourceSpec": null,
                "labels": null,
                "ports": null,
                "volumes": null,
                "networks": null,
                "command": null,
                "environmentVariables": null
            },
            "tagIds": null,
            "duplicateSource": null
        }))
        .unwrap();

        assert!(input.tag_ids.is_empty());
    }

    #[test]
    fn create_contract_defaults_omitted_update_behavior_to_disabled() {
        // The existing UI omits this field when automatic updates are not selected.
        let input: CreateDeploymentInput = serde_json::from_value(serde_json::json!({
            "name":"nginx",
            "platformId":"01a071b6-cc32-7c42-afb2-9656711f2aaa",
            "spec":{
                "image":{"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"},
                "ports":[],"networks":["bridge"]
            }
        })).unwrap();
        assert_eq!(input.spec.update_behavior, UpdateBehavior::Disabled);
        assert!(
            citadel_deployments::DeploymentSpec::from(input.spec.clone())
                .validate()
                .is_ok()
        );
        assert_eq!(input.spec.networks, Some(vec!["bridge".into()]));
        assert_eq!(
            citadel_deployments::DeploymentSpec::from(input.spec.clone())
                .to_storage_value()
                .unwrap()["UpdateBehavior"],
            "Disabled"
        );
    }
}

fn deserialize_field_patch<'de, D, T>(deserializer: D) -> Result<FieldPatch<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|value| match value {
        Some(value) => FieldPatch::Set(value),
        None => FieldPatch::Clear,
    })
}

#[derive(Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdoptContainerInput {
    pub name: String,
    pub description: Option<String>,
    pub spec: DeploymentSpec,
    pub preview_fingerprint: String,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub import_sensitive_environment_as_secrets: bool,
}
