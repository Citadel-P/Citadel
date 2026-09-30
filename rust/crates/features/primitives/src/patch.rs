use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PatchField<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<T> PatchField<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
}

impl<T: Serialize> Serialize for PatchField<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Missing | Self::Null => serializer.serialize_none(),
            Self::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for PatchField<T>
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

impl<T: Clone> PatchField<T> {
    #[must_use]
    pub fn merge_optional(&self, current: Option<&T>) -> Option<T> {
        match self {
            Self::Missing => current.cloned(),
            Self::Null => None,
            Self::Value(value) => Some(value.clone()),
        }
    }
}

impl<T> PatchField<T> {
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> PatchField<U> {
        match self {
            Self::Missing => PatchField::Missing,
            Self::Null => PatchField::Null,
            Self::Value(value) => PatchField::Value(map(value)),
        }
    }
}

/// Omission-aware field whose value controls nullability: `FieldUpdate<bool>`
/// rejects null, while `FieldUpdate<Option<bool>>` accepts an explicit clear.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum FieldUpdate<T> {
    #[default]
    Missing,
    Value(T),
}
impl<T> FieldUpdate<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    pub fn apply(self, current: T) -> T {
        match self {
            Self::Missing => current,
            Self::Value(value) => value,
        }
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for FieldUpdate<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::Value)
    }
}
impl<T: Serialize> Serialize for FieldUpdate<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Missing => s.serialize_none(),
            Self::Value(value) => value.serialize(s),
        }
    }
}

/// RFC 7396 object merge; arrays/scalars replace, null removes object members.
pub fn merge_json(target: &mut serde_json::Value, patch: &serde_json::Value) {
    let serde_json::Value::Object(patch) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = serde_json::json!({});
    }
    let target = target.as_object_mut().expect("object initialized above");
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
        } else {
            merge_json(
                target.entry(key.clone()).or_insert(serde_json::Value::Null),
                value,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json};
    #[derive(Default, Deserialize, Serialize)]
    struct Patch {
        #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
        enabled: FieldUpdate<bool>,
        #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
        description: FieldUpdate<Option<String>>,
    }
    #[test]
    fn patch_preserves_omission_null_and_false() {
        for value in [
            json!({}),
            json!({"enabled":false}),
            json!({"description":null}),
        ] {
            let patch: Patch = from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(patch).unwrap(), value);
        }
        assert!(from_value::<Patch>(json!({"enabled":null})).is_err());
    }
    #[test]
    fn nested_merge_preserves_siblings_and_removes_nulls() {
        let mut value = json!({"webhook":{"enabled":true,"secret":"old"},"items":[1,2]});
        merge_json(&mut value, &json!({"webhook":{"secret":"new"},"items":[3]}));
        assert_eq!(
            value,
            json!({"webhook":{"enabled":true,"secret":"new"},"items":[3]})
        );
        merge_json(&mut value, &json!({"webhook":{"secret":null}}));
        assert_eq!(value["webhook"], json!({"enabled":true}));
        let mut empty = serde_json::Value::Null;
        merge_json(&mut empty, &json!({"removed":null}));
        assert_eq!(empty, json!({}));
    }
}
