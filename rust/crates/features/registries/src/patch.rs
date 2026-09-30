use citadel_primitives::PatchField;
use serde_json::Value;

fn merge_json_patch(patch: &PatchField<Value>, current: Option<&Value>) -> Option<Value> {
    match patch {
        PatchField::Missing => current.cloned(),
        PatchField::Null => None,
        PatchField::Value(patch) => {
            let mut value = current.cloned().unwrap_or(Value::Null);
            apply_json_merge_patch(&mut value, patch);
            Some(value)
        }
    }
}

use citadel_primitives::merge_json as apply_json_merge_patch;

impl crate::RegistryPatch {
    pub fn apply_to(&self, old: &crate::RegistryDetails) -> crate::NewRegistry {
        crate::NewRegistry {
            name: self.name.clone().unwrap_or_else(|| old.name.clone()),
            registry_host: self
                .registry_host
                .clone()
                .unwrap_or_else(|| old.registry_host.clone()),
            status: self.status.unwrap_or(old.status),
            configuration: merge_json_patch(&self.configuration, Some(&old.configuration))
                .unwrap_or(Value::Null),
            description: self.description.merge_optional(old.description.as_ref()),
            tag_ids: self
                .tag_ids
                .clone()
                .unwrap_or_else(|| old.tags.iter().map(|tag| tag.id).collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PatchField;
    use serde_json::json;

    use super::merge_json_patch;

    #[test]
    fn nested_catalog_configuration_uses_json_merge_patch_semantics() {
        let current = json!({
            "$type":"Custom",
            "authEnabled":true,
            "userName":"operator",
            "password":"old-secret"
        });
        let patch = PatchField::Value(json!({"password":"new-secret"}));
        assert_eq!(
            merge_json_patch(&patch, Some(&current)).unwrap(),
            json!({
                "$type":"Custom",
                "authEnabled":true,
                "userName":"operator",
                "password":"new-secret"
            })
        );

        let patch = PatchField::Value(json!({"password":null}));
        assert!(
            merge_json_patch(&patch, Some(&current))
                .unwrap()
                .get("password")
                .is_none()
        );
    }
}
