//! PATCH preserves omitted fields; explicit null is passed to the domain's validation.
use super::spec::*;
use crate::api::resources::metadata_patch::MetadataPatch;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase")]
pub struct PatchAlertRuleInput {
    #[serde(default, rename = "type")]
    #[schema(value_type = AlertType, required = false)]
    alert_type: MetadataPatch<AlertType>,
    #[serde(default)]
    #[schema(value_type = AlertSeverity, required = false)]
    severity: MetadataPatch<AlertSeverity>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    cooldown_seconds: MetadataPatch<i32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    required_matches: MetadataPatch<i32>,
    #[serde(default)]
    #[schema(value_type = Option<f64>, required = false)]
    threshold: MetadataPatch<f64>,
    #[serde(default)]
    #[schema(value_type = AlertRuleStatus, required = false)]
    status: MetadataPatch<AlertRuleStatus>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    channel_ids: MetadataPatch<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<AlertResourceScope>>, required = false)]
    limited_to: MetadataPatch<Vec<AlertResourceScope>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<AlertQuietHour>>, required = false)]
    quiet_hours: MetadataPatch<Vec<AlertQuietHour>>,
}
impl PatchAlertRuleInput {
    pub(crate) fn into_value(self) -> Value {
        let mut fields = Map::new();
        insert(&mut fields, "type", self.alert_type);
        insert(&mut fields, "severity", self.severity);
        insert(&mut fields, "cooldownSeconds", self.cooldown_seconds);
        insert(&mut fields, "requiredMatches", self.required_matches);
        insert(&mut fields, "threshold", self.threshold);
        insert(&mut fields, "status", self.status);
        insert(&mut fields, "channelIds", self.channel_ids);
        insert(&mut fields, "limitedTo", self.limited_to);
        insert(&mut fields, "quietHours", self.quiet_hours);
        Value::Object(fields)
    }
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase")]
pub struct PatchAlertChannelInput {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    name: MetadataPatch<String>,
    #[serde(default)]
    #[schema(value_type = AlertDestination, required = false)]
    alert_destination: MetadataPatch<AlertDestination>,
    #[serde(default)]
    #[schema(value_type = String, required = false)]
    url: MetadataPatch<String>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    is_active: MetadataPatch<bool>,
}
impl PatchAlertChannelInput {
    pub(crate) fn into_value(self) -> Value {
        let mut fields = Map::new();
        insert(&mut fields, "name", self.name);
        insert(&mut fields, "alertDestination", self.alert_destination);
        insert(&mut fields, "url", self.url);
        insert(&mut fields, "isActive", self.is_active);
        Value::Object(fields)
    }
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase")]
pub struct PatchAlertRuleMetadata {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    description: MetadataPatch<String>,
}
impl PatchAlertRuleMetadata {
    pub(crate) fn into_value(self) -> Value {
        let mut fields = Map::new();
        insert(&mut fields, "description", self.description);
        Value::Object(fields)
    }
}

fn insert<T: Serialize>(fields: &mut Map<String, Value>, name: &str, patch: MetadataPatch<T>) {
    match patch {
        MetadataPatch::Missing => {}
        MetadataPatch::Null => {
            fields.insert(name.to_owned(), Value::Null);
        }
        MetadataPatch::Value(value) => {
            fields.insert(name.to_owned(), serde_json::json!(value));
        }
    }
}

// Derived struct deserialization also accepts arrays; HTTP merge patches must be objects.
macro_rules! object_patch {
    ($($name:ident),* $(,)?) => {$(
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct ObjectVisitor;
                impl<'de> serde::de::Visitor<'de> for ObjectVisitor {
                    type Value = $name;
                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("an alert update object")
                    }
                    fn visit_map<M: serde::de::MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                        $name::deserialize(serde::de::value::MapAccessDeserializer::new(map))
                    }
                }
                deserializer.deserialize_map(ObjectVisitor)
            }
        }
    )*};
}
object_patch!(
    PatchAlertRuleInput,
    PatchAlertChannelInput,
    PatchAlertRuleMetadata
);
