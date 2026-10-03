//! Typed partial updates: missing preserves a field; null only clears nullable fields.
use citadel_automation::{AutomationAction, AutomationActionConfiguration};
use citadel_primitives::WebhookPatch;
use serde::Deserialize;

use citadel_primitives::FieldUpdate;

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAutomationActionInput {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    description: FieldUpdate<Option<String>>,
    #[serde(default)]
    #[schema(value_type = String, required = false)]
    code: FieldUpdate<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    default_args_json: FieldUpdate<Option<String>>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    enabled: FieldUpdate<bool>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    schedule_enabled: FieldUpdate<bool>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    schedule_cron: FieldUpdate<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    schedule_time_zone: FieldUpdate<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<crate::api::resources::schema_models::primitives::WebhookPatchSchema>, required = false)]
    webhook: FieldUpdate<Option<WebhookPatch>>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    alert_on_failure: FieldUpdate<bool>,
    #[serde(default)]
    #[schema(value_type = Option<uuid::Uuid>, required = false)]
    run_as_actor_id: FieldUpdate<Option<uuid::Uuid>>,
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAutomationActionMetadata {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    description: FieldUpdate<Option<String>>,
}

// Serde's default struct visitor also accepts arrays. PATCH requires a JSON
// object, as the previous handler did. Reuse the derived field deserializer
// through a map-only visitor so field paths and unknown-field checks survive.
impl<'de> Deserialize<'de> for UpdateAutomationActionInput {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> serde::de::Visitor<'de> for ObjectVisitor {
            type Value = UpdateAutomationActionInput;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an Automation Action update object")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                UpdateAutomationActionInput::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

impl<'de> Deserialize<'de> for UpdateAutomationActionMetadata {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> serde::de::Visitor<'de> for ObjectVisitor {
            type Value = UpdateAutomationActionMetadata;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an Automation Action update object")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                UpdateAutomationActionMetadata::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

impl From<UpdateAutomationActionMetadata> for UpdateAutomationActionInput {
    fn from(metadata: UpdateAutomationActionMetadata) -> Self {
        Self {
            description: metadata.description,
            ..Self::default()
        }
    }
}

impl UpdateAutomationActionInput {
    pub fn apply(self, current: AutomationAction) -> AutomationActionConfiguration {
        citadel_automation::UpdateAutomationActionInput {
            description: self.description,
            code: self.code,
            default_args_json: self.default_args_json,
            enabled: self.enabled,
            schedule_enabled: self.schedule_enabled,
            schedule_cron: self.schedule_cron,
            schedule_time_zone: self.schedule_time_zone,
            webhook: self.webhook,
            timeout_seconds: self.timeout_seconds,
            alert_on_failure: self.alert_on_failure,
            run_as_actor_id: self.run_as_actor_id,
        }
        .apply(current)
    }
}
