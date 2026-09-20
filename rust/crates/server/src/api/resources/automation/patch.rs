//! Typed partial updates: missing preserves a field; null only clears nullable fields.
use crate::api::resources::git_repositories::webhook::RepoWebhookConfig;
use citadel_automation::{AutomationAction, AutomationActionConfiguration};
use serde::Deserialize;

#[derive(Debug, Clone, Default)]
enum Change<T> {
    #[default]
    Missing,
    Value(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Change<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::Value)
    }
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAutomationActionInput {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    description: Change<Option<String>>,
    #[serde(default)]
    #[schema(value_type = String, required = false)]
    code: Change<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    default_args_json: Change<Option<String>>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    enabled: Change<bool>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    schedule_enabled: Change<bool>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    schedule_cron: Change<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    schedule_time_zone: Change<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<RepoWebhookConfig>, required = false)]
    webhook: Change<Option<RepoWebhookConfig>>,
    #[serde(default)]
    #[schema(value_type = Option<i32>, required = false)]
    timeout_seconds: Change<Option<i32>>,
    #[serde(default)]
    #[schema(value_type = bool, required = false)]
    alert_on_failure: Change<bool>,
    #[serde(default)]
    #[schema(value_type = Option<uuid::Uuid>, required = false)]
    run_as_actor_id: Change<Option<uuid::Uuid>>,
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(remote = "Self", rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAutomationActionMetadata {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    description: Change<Option<String>>,
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
            description: match self.description {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            code: match self.code {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            default_args_json: match self.default_args_json {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            enabled: match self.enabled {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            schedule_enabled: match self.schedule_enabled {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            schedule_cron: match self.schedule_cron {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            schedule_time_zone: match self.schedule_time_zone {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            webhook: match self.webhook {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => {
                    citadel_automation::actions::patch::Change::Value(v.map(Into::into))
                }
            },
            timeout_seconds: match self.timeout_seconds {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            alert_on_failure: match self.alert_on_failure {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
            run_as_actor_id: match self.run_as_actor_id {
                Change::Missing => citadel_automation::actions::patch::Change::Missing,
                Change::Value(v) => citadel_automation::actions::patch::Change::Value(v),
            },
        }
        .apply(current)
    }
}
