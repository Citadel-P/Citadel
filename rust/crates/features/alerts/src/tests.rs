use crate::service::retry_delay;
use crate::*;
use uuid::Uuid;
#[test]
fn create_validation_collects_field_errors_and_preserves_cooldown_boundaries() {
    let mut input: AlertRuleConfiguration = serde_json::from_value(serde_json::json!({
        "type":"PlatformCpuHigh","severity":"Warning","cooldownSeconds":5
    }))
    .unwrap();
    let AlertError::FieldValidation(fields) = input.validate_create().unwrap_err() else {
        panic!("expected field-level validation");
    };
    assert_eq!(
        serde_json::to_value(fields).unwrap(),
        serde_json::json!({
            "CooldownSeconds":["Cooldown must be between 10s and 24h."],
            "RequiredMatches":["'Required Matches' must not be empty."],
            "Threshold":["'Threshold' must not be empty."]
        })
    );
    input.required_matches = Some(3);
    input.threshold = Some(85.0);
    for (cooldown, valid) in [
        (None, true),
        (Some(9), false),
        (Some(10), true),
        (Some(86400), true),
        (Some(86401), false),
    ] {
        input.cooldown_seconds = cooldown;
        assert_eq!(input.validate_create().is_ok(), valid);
        if valid {
            assert!(input.validate().is_ok());
        } else {
            assert!(matches!(input.validate(), Err(AlertError::InvalidCooldown)));
        }
    }
}

#[test]
fn accepts_supported_shoutrrr_urls_and_rejects_unknown_destinations() {
    let mut input = AlertChannelConfiguration {
        name: "ops".into(),
        alert_destination: "Telegram".into(),
        url: "telegram://token@telegram?chats=123".into(),
        is_active: true,
    };
    input.validate().unwrap();
    input.alert_destination = "Unknown".into();
    assert!(matches!(input.validate(), Err(AlertError::Validation(_))));
}

#[test]
fn rejects_duplicate_channel_bindings() {
    let id = Uuid::now_v7();
    let mut input = AlertRuleConfiguration {
        name: "failure".into(),
        description: None,
        alert_type: "BuildRunFailed".into(),
        severity: "Critical".into(),
        cooldown_seconds: Some(60),
        required_matches: None,
        threshold: None,
        status: "Enabled".into(),
        channel_ids: vec![id, id],
        limited_to: vec![],
        quiet_hours: vec![],
    };
    assert!(matches!(input.validate(), Err(AlertError::Validation(_))));
}

#[test]
fn delivery_retry_backoff_is_bounded() {
    assert_eq!(retry_delay(1), chrono::Duration::seconds(1));
    assert_eq!(retry_delay(4), chrono::Duration::minutes(2));
    assert_eq!(retry_delay(i32::MAX), chrono::Duration::minutes(30));
}
