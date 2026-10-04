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
        severity: crate::AlertSeverity::Critical,
        cooldown_seconds: Some(60),
        required_matches: None,
        threshold: None,
        status: crate::AlertRuleStatus::Enabled,
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

#[test]
fn rule_status_defaults_to_enabled_and_preserves_disabled_in_audit_snapshots() {
    let input = serde_json::json!({"type":"PlatformUnreachable","severity":"Warning"});
    let default: AlertRuleConfiguration = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(default.status, AlertRuleStatus::Enabled);
    for status in AlertRuleStatus::ALL {
        let mut value = input.clone();
        value["status"] = serde_json::to_value(status).unwrap();
        let configuration: AlertRuleConfiguration = serde_json::from_value(value).unwrap();
        assert_eq!(configuration.status, *status);
        assert_eq!(configuration.snapshot(Uuid::nil()).status, status.as_str());
        assert_eq!(status.as_str().parse::<AlertRuleStatus>().unwrap(), *status);
    }
    for invalid in [
        serde_json::json!("enabled"),
        serde_json::json!("Active"),
        serde_json::Value::Null,
    ] {
        let mut value = input.clone();
        value["status"] = invalid;
        assert!(serde_json::from_value::<AlertRuleConfiguration>(value).is_err());
    }
}

#[test]
fn build_pool_availability_grace_is_configurable_and_recovery_never_matches() {
    let mut input: AlertRuleConfiguration = serde_json::from_value(serde_json::json!({
        "type":"BuildAgentPoolUnavailable", "severity":"Warning"
    }))
    .unwrap();
    input.validate_create().unwrap();
    let now = chrono::Utc::now();
    let mut rule = AlertRule {
        id: Uuid::now_v7(),
        name: input.name,
        description: None,
        alert_type: input.alert_type,
        severity: input.severity,
        cooldown_seconds: None,
        required_matches: None,
        threshold: None,
        status: AlertRuleStatus::Enabled,
        channel_ids: vec![],
        limited_to: vec![],
        quiet_hours: vec![],
        audit: citadel_primitives::AuditMetadata {
            created_at: now,
            created_by_actor_id: citadel_primitives::ActorId::new(Uuid::from_u128(1)),
        },
    };
    let mut observation = AlertObservation {
        alert_type: rule.alert_type.clone(),
        info: serde_json::json!({}),
        resource_id: Uuid::now_v7(),
        resource_name: "builder".into(),
        resource_type: "BuildAgentPool".into(),
        deduplication_component: "availability".into(),
        observed_at: now,
        value: Some(89.9),
        matched: true,
    };
    assert!(!rules::evaluation::observation_matches(&rule, &observation));
    observation.value = Some(90.0);
    assert!(rules::evaluation::observation_matches(&rule, &observation));
    rule.threshold = Some(120.0);
    assert!(!rules::evaluation::observation_matches(&rule, &observation));
    observation.value = Some(120.0);
    assert!(rules::evaluation::observation_matches(&rule, &observation));
    observation.matched = false;
    rule.threshold = Some(0.0);
    assert!(!rules::evaluation::observation_matches(&rule, &observation));
    for (seconds, valid) in [
        (-1.0, false),
        (0.0, true),
        (90.0, true),
        (86400.0, true),
        (86401.0, false),
        (f64::NAN, false),
    ] {
        let mut configuration = AlertRuleConfiguration::from(&rule);
        configuration.threshold = Some(seconds);
        assert_eq!(configuration.validate_create().is_ok(), valid);
    }
}
