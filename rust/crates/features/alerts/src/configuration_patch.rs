use crate::AlertChannel;
use crate::AlertChannelConfiguration;
use crate::AlertError;
use crate::AlertRule;
use crate::AlertRuleConfiguration;
use serde_json::Value;

pub fn apply_rule(
    current: &AlertRule,
    patch: &Value,
) -> Result<AlertRuleConfiguration, AlertError> {
    let mut value = serde_json::to_value(AlertRuleConfiguration::from(current))
        .map_err(|e| AlertError::Storage(e.to_string()))?;
    // Name/description belong to rename/metadata, as in PatchAlertRuleHandler.
    apply_fields(
        &mut value,
        patch,
        &[
            "type",
            "severity",
            "cooldownSeconds",
            "requiredMatches",
            "threshold",
            "status",
            "channelIds",
            "limitedTo",
            "quietHours",
        ],
    )?;
    // Alert Rule inputs normalize null collections to empty.
    for field in ["channelIds", "limitedTo", "quietHours"] {
        if value[field].is_null() {
            value[field] = Value::Array(Vec::new());
        }
    }
    let mut input: AlertRuleConfiguration = serde_path_to_error::deserialize(value)
        .map_err(|error| AlertError::Validation(format!("Invalid Alert Rule patch: {error}")))?;
    input.validate()?;
    Ok(input)
}

pub fn apply_channel(
    current: &AlertChannel,
    patch: &Value,
) -> Result<AlertChannelConfiguration, AlertError> {
    let mut value = serde_json::to_value(AlertChannelConfiguration::from(current))
        .map_err(|e| AlertError::Storage(e.to_string()))?;
    apply_fields(
        &mut value,
        patch,
        &["name", "alertDestination", "url", "isActive"],
    )?;
    let mut input: AlertChannelConfiguration = serde_path_to_error::deserialize(value)
        .map_err(|error| AlertError::Validation(format!("Invalid Alert Channel patch: {error}")))?;
    input.validate()?;
    Ok(input)
}

fn apply_fields(current: &mut Value, patch: &Value, fields: &[&str]) -> Result<(), AlertError> {
    let patch = patch
        .as_object()
        .ok_or_else(|| AlertError::Validation("Configuration patch must be an object.".into()))?;
    for field in fields {
        if let Some(value) = patch.get(*field) {
            current[*field] = value.clone();
        }
    }
    Ok(())
}

/// Free installations may disable a custom Rule or remove Channels. Built-in
/// Rules may also be enabled and have Channels assigned without an upgrade.
pub fn requires_advanced_alerting(current: &AlertRule, next: &AlertRuleConfiguration) -> bool {
    let custom = current.created_by_actor_id != uuid::Uuid::from_u128(1);
    current.alert_type != next.alert_type
        || current.severity != next.severity
        || current.cooldown_seconds != next.cooldown_seconds
        || current.required_matches != next.required_matches
        || current.threshold != next.threshold
        || current.limited_to != next.limited_to
        || current.quiet_hours != next.quiet_hours
        || (custom
            && next
                .channel_ids
                .iter()
                .any(|id| !current.channel_ids.contains(id)))
        || (custom && current.status != next.status && next.status == "Enabled")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn rule() -> AlertRule {
        AlertRule {
            id: Uuid::now_v7(),
            name: "rule".into(),
            description: Some("description".into()),
            alert_type: "PlatformCpuHigh".into(),
            severity: "Warning".into(),
            cooldown_seconds: Some(60),
            required_matches: Some(3),
            threshold: Some(80.0),
            status: "Enabled".into(),
            channel_ids: vec![Uuid::now_v7()],
            limited_to: vec![],
            quiet_hours: vec![],
            created_by_actor_id: Uuid::now_v7(),
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn rule_patch_preserves_omissions_and_metadata_but_clears_explicit_optional_values() {
        let current = rule();
        let updated = apply_rule(&current,&json!({"type":"PlatformUnreachable","threshold":null,"requiredMatches":null,"name":"ignored","description":null,"createdByActorId":Uuid::nil()})).unwrap();
        assert_eq!(updated.threshold, None);
        assert_eq!(updated.required_matches, None);
        assert_eq!(updated.cooldown_seconds, Some(60));
        assert_eq!(updated.name, current.name);
        assert_eq!(updated.description, current.description);
        assert_eq!(updated.channel_ids, current.channel_ids);
        assert!(
            apply_rule(&current, &json!({"channelIds":[]}))
                .unwrap()
                .channel_ids
                .is_empty()
        );
        let cleared = apply_rule(
            &current,
            &json!({"channelIds":null,"limitedTo":null,"quietHours":null}),
        )
        .unwrap();
        assert!(cleared.channel_ids.is_empty());
        assert!(cleared.limited_to.is_empty());
        assert!(cleared.quiet_hours.is_empty());
    }

    #[test]
    fn rule_patch_validates_the_merged_configuration() {
        let current = rule();
        for invalid in [
            json!([]),
            json!({"severity":null}),
            json!({"cooldownSeconds":5}),
            json!({"status":"invalid"}),
            json!({"channelIds":[Uuid::nil()]}),
            json!({"channelIds":[current.channel_ids[0],current.channel_ids[0]]}),
        ] {
            assert!(apply_rule(&current, &invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn license_gate_preserves_free_system_rule_controls_and_downgrade_safety() {
        let mut current = rule();
        for patch in [
            serde_json::json!({"status":"Disabled"}),
            serde_json::json!({"channelIds":[]}),
            serde_json::json!({}),
        ] {
            assert!(!requires_advanced_alerting(
                &current,
                &apply_rule(&current, &patch).unwrap()
            ));
        }
        for patch in [
            serde_json::json!({"severity":"Critical"}),
            serde_json::json!({"threshold":90}),
            serde_json::json!({"channelIds":[Uuid::now_v7()]}),
        ] {
            assert!(requires_advanced_alerting(
                &current,
                &apply_rule(&current, &patch).unwrap()
            ));
        }
        current.status = "Disabled".into();
        assert!(requires_advanced_alerting(
            &current,
            &apply_rule(&current, &serde_json::json!({"status":"Enabled"})).unwrap()
        ));
        current.created_by_actor_id = Uuid::from_u128(1);
        for patch in [
            serde_json::json!({"status":"Enabled"}),
            serde_json::json!({"channelIds":[Uuid::now_v7()]}),
        ] {
            assert!(!requires_advanced_alerting(
                &current,
                &apply_rule(&current, &patch).unwrap()
            ));
        }
    }

    #[test]
    fn channel_patch_preserves_omissions_and_rejects_invalid_required_fields() {
        let current = AlertChannel {
            id: Uuid::now_v7(),
            name: "channel".into(),
            alert_destination: "Generic".into(),
            url: "https://alerts.example.test/hook".into(),
            is_active: true,
            created_by_actor_id: Uuid::now_v7(),
            created_at: chrono::Utc::now(),
        };
        let updated = apply_channel(&current, &json!({"isActive":false})).unwrap();
        assert!(!updated.is_active);
        assert_eq!(updated.url, current.url);
        assert_eq!(updated.name, current.name);
        for patch in [
            json!(null),
            json!({"url":null}),
            json!({"isActive":null}),
            json!({"name":["invalid"]}),
        ] {
            assert!(apply_channel(&current, &patch).is_err());
        }
    }
}
