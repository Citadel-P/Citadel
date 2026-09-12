//! Typed partial updates: missing preserves a field; null only clears nullable fields.
use super::{AutomationActionInput, AutomationActionView};
use citadel_resources::RepoWebhookConfig;
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

impl<T> Change<T> {
    fn apply(self, current: T) -> T {
        match self {
            Self::Missing => current,
            Self::Value(value) => value,
        }
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
    pub fn apply(self, current: AutomationActionView) -> AutomationActionInput {
        AutomationActionInput {
            name: current.name,
            description: self.description.apply(current.description),
            code: self.code.apply(current.code),
            default_args_json: self
                .default_args_json
                .apply(Some(current.default_args_json)),
            enabled: self.enabled.apply(current.enabled),
            schedule_enabled: self.schedule_enabled.apply(current.schedule_enabled),
            schedule_cron: self.schedule_cron.apply(current.schedule_cron),
            schedule_time_zone: self
                .schedule_time_zone
                .apply(Some(current.schedule_time_zone)),
            webhook: self.webhook.apply(current.webhook),
            timeout_seconds: self.timeout_seconds.apply(Some(current.timeout_seconds)),
            alert_on_failure: self.alert_on_failure.apply(current.alert_on_failure),
            run_as_actor_id: self.run_as_actor_id.apply(Some(current.run_as_actor_id)),
            tag_ids: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn current() -> AutomationActionView {
        AutomationActionView {
            id: uuid::Uuid::now_v7(),
            name: "action".into(),
            description: Some("description".into()),
            code: "original".into(),
            default_args_json: "{}".into(),
            enabled: true,
            schedule_enabled: true,
            schedule_cron: Some("0 * * * *".into()),
            schedule_time_zone: "UTC".into(),
            webhook: Some(serde_json::from_value(json!({"enabled":false})).unwrap()),
            timeout_seconds: 300,
            alert_on_failure: true,
            run_as_actor_id: uuid::Uuid::now_v7(),
            control_state: "Idle".into(),
            current_run_id: None,
            row_version: 1,
            created_by_actor_id: uuid::Uuid::now_v7(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_scheduled_run_at: None,
            tags: Vec::new(),
            latest_run: None,
        }
    }

    #[test]
    fn partial_update_preserves_omitted_fields_and_applies_false() {
        let before = current();
        let patch: UpdateAutomationActionInput =
            serde_json::from_value(json!({"code":"edited", "enabled":false})).unwrap();
        let after = patch.apply(before.clone());
        assert_eq!(after.code, "edited");
        assert!(!after.enabled);
        assert_eq!(after.name, before.name);
        assert_eq!(after.description, before.description);
        assert_eq!(after.default_args_json, Some(before.default_args_json));
        assert_eq!(after.schedule_cron, before.schedule_cron);
        assert_eq!(after.schedule_time_zone, Some(before.schedule_time_zone));
        assert_eq!(after.timeout_seconds, Some(before.timeout_seconds));
        assert_eq!(after.run_as_actor_id, Some(before.run_as_actor_id));
        assert_eq!(after.webhook, before.webhook);
    }

    #[test]
    fn explicit_null_clears_nullable_fields() {
        let patch: UpdateAutomationActionInput = serde_json::from_value(json!({
            "description":null,"defaultArgsJson":null,"scheduleCron":null,
            "scheduleTimeZone":null,"webhook":null,"timeoutSeconds":null,"runAsActorId":null
        }))
        .unwrap();
        let after = patch.apply(current());
        assert!(after.description.is_none());
        assert!(after.default_args_json.is_none());
        assert!(after.schedule_cron.is_none());
        assert!(after.schedule_time_zone.is_none());
        assert!(after.webhook.is_none());
        assert!(after.timeout_seconds.is_none());
        assert!(after.run_as_actor_id.is_none());
    }

    #[test]
    fn rejects_null_required_values_wrong_types_and_unknown_fields() {
        for value in [
            json!({"code":null}),
            json!({"enabled":null}),
            json!({"scheduleEnabled":null}),
            json!({"alertOnFailure":null}),
            json!({"timeoutSeconds":"bad"}),
            json!({"runAsActorId":42}),
            json!({"name":"renamed"}),
            json!({"tagIds":[]}),
            json!([]),
        ] {
            assert!(
                serde_json::from_value::<UpdateAutomationActionInput>(value.clone()).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn metadata_only_accepts_description_and_preserves_omitted_values() {
        let metadata: UpdateAutomationActionMetadata = serde_json::from_str("{}").unwrap();
        let patch: UpdateAutomationActionInput = metadata.into();
        assert_eq!(
            patch.apply(current()).description.as_deref(),
            Some("description")
        );
        let metadata: UpdateAutomationActionMetadata =
            serde_json::from_str(r#"{"description":null}"#).unwrap();
        let patch: UpdateAutomationActionInput = metadata.into();
        assert!(patch.apply(current()).description.is_none());
        assert!(
            serde_json::from_str::<UpdateAutomationActionMetadata>(r#"{"code":"edited"}"#).is_err()
        );
        assert!(serde_json::from_str::<UpdateAutomationActionMetadata>("[]").is_err());
    }
}
