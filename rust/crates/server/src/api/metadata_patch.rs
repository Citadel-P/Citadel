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

impl<T> From<MetadataPatch<T>> for citadel_registries::MetadataPatch<T> {
    fn from(value: MetadataPatch<T>) -> Self {
        match value {
            MetadataPatch::Missing => Self::Missing,
            MetadataPatch::Null => Self::Null,
            MetadataPatch::Value(value) => Self::Value(value),
        }
    }
}
impl<T> From<MetadataPatch<T>> for citadel_bindings::MetadataPatch<T> {
    fn from(value: MetadataPatch<T>) -> Self {
        match value {
            MetadataPatch::Missing => Self::Missing,
            MetadataPatch::Null => Self::Null,
            MetadataPatch::Value(value) => Self::Value(value),
        }
    }
}

impl<T> From<citadel_registries::MetadataPatch<T>> for MetadataPatch<T> {
    fn from(value: citadel_registries::MetadataPatch<T>) -> Self {
        match value {
            citadel_registries::MetadataPatch::Missing => Self::Missing,
            citadel_registries::MetadataPatch::Null => Self::Null,
            citadel_registries::MetadataPatch::Value(value) => Self::Value(value),
        }
    }
}
impl<T> From<citadel_bindings::MetadataPatch<T>> for MetadataPatch<T> {
    fn from(value: citadel_bindings::MetadataPatch<T>) -> Self {
        match value {
            citadel_bindings::MetadataPatch::Missing => Self::Missing,
            citadel_bindings::MetadataPatch::Null => Self::Null,
            citadel_bindings::MetadataPatch::Value(value) => Self::Value(value),
        }
    }
}
