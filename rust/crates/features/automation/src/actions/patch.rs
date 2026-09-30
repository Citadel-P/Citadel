//! Typed partial updates: missing preserves a field; null only clears nullable fields.
use super::{AutomationAction, AutomationActionConfiguration};
use citadel_primitives::WebhookPatch;
use serde::Deserialize;

use citadel_primitives::FieldUpdate;

#[derive(Debug, Default, Deserialize)]
#[serde(remote = "Self", rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAutomationActionInput {
    #[serde(default)]
    pub description: FieldUpdate<Option<String>>,
    #[serde(default)]
    pub code: FieldUpdate<String>,
    #[serde(default)]
    pub default_args_json: FieldUpdate<Option<String>>,
    #[serde(default)]
    pub enabled: FieldUpdate<bool>,
    #[serde(default)]
    pub schedule_enabled: FieldUpdate<bool>,
    #[serde(default)]
    pub schedule_cron: FieldUpdate<Option<String>>,
    #[serde(default)]
    pub schedule_time_zone: FieldUpdate<Option<String>>,
    #[serde(default)]
    pub webhook: FieldUpdate<Option<WebhookPatch>>,
    #[serde(default)]
    pub timeout_seconds: FieldUpdate<Option<i32>>,
    #[serde(default)]
    pub alert_on_failure: FieldUpdate<bool>,
    #[serde(default)]
    pub run_as_actor_id: FieldUpdate<Option<uuid::Uuid>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(remote = "Self", rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAutomationActionMetadata {
    #[serde(default)]
    pub description: FieldUpdate<Option<String>>,
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
        AutomationActionConfiguration {
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
            webhook: match self.webhook {
                FieldUpdate::Missing => current.webhook,
                FieldUpdate::Value(None) => None,
                FieldUpdate::Value(Some(patch)) => {
                    Some(patch.apply(current.webhook.unwrap_or_default()))
                }
            },
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

    fn current() -> AutomationAction {
        AutomationAction {
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
            control_state: citadel_primitives::ResourceControlState::Idle,
            current_run_id: None,
            row_version: 1,

            updated_at: chrono::Utc::now(),
            last_scheduled_run_at: None,
            tags: Vec::new(),
            latest_run: None,
            audit: citadel_primitives::AuditMetadata {
                created_at: chrono::Utc::now(),
                created_by_actor_id: citadel_primitives::ActorId::new(uuid::Uuid::now_v7()),
            },
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
    fn nested_webhook_patch_preserves_enabled_and_authentication() {
        let mut before = current();
        before.webhook = Some(
            serde_json::from_value(json!({
                "enabled": true, "provider": "Generic", "authScheme": "BearerToken",
                "secret": "original", "branchFilter": "main"
            }))
            .unwrap(),
        );
        let patch: UpdateAutomationActionInput = serde_json::from_value(json!({
            "webhook": {"secret": "rotated"}
        }))
        .unwrap();
        let after = patch.apply(before.clone()).webhook.unwrap();
        let mut expected = before.webhook.unwrap();
        expected.secret = Some("rotated".into());
        assert_eq!(after, expected);
    }

    #[test]
    fn invalid_schedule_is_rejected_when_saving_configuration() {
        for (cron, zone) in [
            ("0,99 * * * *", "UTC"),
            ("0,bad * * * *", "UTC"),
            ("0 * * * *", "invalid-zone"),
        ] {
            let mut config = UpdateAutomationActionInput::default().apply(current());
            config.schedule_cron = Some(cron.into());
            config.schedule_time_zone = Some(zone.into());
            assert!(
                config
                    .validate(citadel_primitives::ActorId::new(uuid::Uuid::now_v7()))
                    .is_err()
            );
        }
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
