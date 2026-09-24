use serde::Deserialize;

#[derive(Debug, Clone, Default)]
pub enum MetadataPatch<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for MetadataPatch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| value.map_or(Self::Null, Self::Value))
    }
}

impl<T: Clone> MetadataPatch<T> {
    #[must_use]
    pub fn merge_optional(&self, current: Option<&T>) -> Option<T> {
        match self {
            Self::Missing => current.cloned(),
            Self::Null => None,
            Self::Value(value) => Some(value.clone()),
        }
    }
}

use serde_json::Value;

fn merge_json_patch(patch: &MetadataPatch<Value>, current: Option<&Value>) -> Option<Value> {
    match patch {
        MetadataPatch::Missing => current.cloned(),
        MetadataPatch::Null => None,
        MetadataPatch::Value(patch) => {
            let mut value = current.cloned().unwrap_or(Value::Null);
            apply_json_merge_patch(&mut value, patch);
            Some(value)
        }
    }
}

fn apply_json_merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(patch) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(serde_json::Map::new());
    }
    let target = target
        .as_object_mut()
        .expect("target was initialized as an object");
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
        } else {
            apply_json_merge_patch(target.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}

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
    use super::MetadataPatch;
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
        let patch = MetadataPatch::Value(json!({"password":"new-secret"}));
        assert_eq!(
            merge_json_patch(&patch, Some(&current)).unwrap(),
            json!({
                "$type":"Custom",
                "authEnabled":true,
                "userName":"operator",
                "password":"new-secret"
            })
        );

        let patch = MetadataPatch::Value(json!({"password":null}));
        assert!(
            merge_json_patch(&patch, Some(&current))
                .unwrap()
                .get("password")
                .is_none()
        );
    }
}
