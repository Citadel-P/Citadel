//! PATCH preserves omitted fields; explicit null is passed to the domain's validation.
use super::spec::*;
use citadel_alerts::{AlertRuleStatus, AlertSeverity, AlertType};
use citadel_primitives::PatchField;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase")]
pub struct PatchAlertRuleInput {
    #[serde(default, rename = "type")]
    #[schema(value_type = crate::api::resources::schema_models::alerts::AlertTypeSchema, required = false)]
    alert_type: PatchField<AlertType>,
    #[serde(default)]
    #[schema(value_type = crate::api::resources::schema_models::alerts::AlertSeveritySchema, required = false)]
    severity: PatchField<AlertSeverity>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    cooldown_seconds: PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    required_matches: PatchField<i32>,
    #[serde(default)]
    #[schema(value_type = Option<f64>, required = false)]
    threshold: PatchField<f64>,
    #[serde(default)]
    #[schema(value_type = crate::api::resources::schema_models::alerts::AlertRuleStatusSchema, required = false)]
    status: PatchField<AlertRuleStatus>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<Uuid>>, required = false)]
    channel_ids: PatchField<Vec<Uuid>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<AlertResourceScope>>, required = false)]
    limited_to: PatchField<Vec<AlertResourceScope>>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<AlertQuietHour>>, required = false)]
    quiet_hours: PatchField<Vec<AlertQuietHour>>,
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
    name: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = AlertDestination, required = false)]
    alert_destination: PatchField<AlertDestination>,
    #[serde(default)]
    #[schema(value_type = String, required = false)]
    url: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    is_active: PatchField<bool>,
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
    description: PatchField<String>,
}
impl PatchAlertRuleMetadata {
    pub(crate) fn into_value(self) -> Value {
        let mut fields = Map::new();
        insert(&mut fields, "description", self.description);
        Value::Object(fields)
    }
}

fn insert<T: Serialize>(fields: &mut Map<String, Value>, name: &str, patch: PatchField<T>) {
    match patch {
        PatchField::Missing => {}
        PatchField::Null => {
            fields.insert(name.to_owned(), Value::Null);
        }
        PatchField::Value(value) => {
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
