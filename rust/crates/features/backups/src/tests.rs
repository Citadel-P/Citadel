use crate::*;
use serde_json::json;
#[test]
fn repository_requires_polymorphic_type() {
    let mut input = BackupRepositoryConfiguration {
        name: "repo".into(),
        description: None,
        spec: json!({}),
        password_secret_id: Uuid::now_v7(),
    };
    assert!(matches!(input.validate(), Err(BackupError::Validation(_))))
}
#[test]
fn policy_normalizes_defaults() {
    let actor = ActorId::new(Uuid::now_v7());
    let mut input = BackupPolicyConfiguration {
        name: " policy ".into(),
        description: None,
        source: json!({"$type":"CitadelSystem"}),
        backup_repository_id: Uuid::now_v7(),
        enabled: true,
        cron: None,
        time_zone: None,
        webhook: None,
        keep_last_successful: None,
        timeout_seconds: None,
        alert_on_failure: true,
        run_as_actor_id: None,
        tag_ids: vec![],
    };
    input.validate(actor).unwrap();
    assert_eq!(input.name, "policy");
    assert_eq!(input.keep_last_successful, Some(14));
}

#[test]
fn schedule_validation_and_time_zone_matching_follow_five_field_cron() {
    let now = DateTime::parse_from_rfc3339("2026-07-14T08:30:00Z")
        .unwrap()
        .with_timezone(&Utc);
    assert!(schedule_is_due(Some("30 10 * * 2"), "Europe/Paris", now));
    assert!(!schedule_is_due(Some("31 10 * * 2"), "Europe/Paris", now));
    assert!(!valid_cron("* * *"));

    let actor = ActorId::new(Uuid::now_v7());
    let mut input = BackupPolicyConfiguration {
        name: "invalid schedule".into(),
        description: None,
        source: json!({"$type":"CitadelSystem"}),
        backup_repository_id: Uuid::now_v7(),
        enabled: true,
        cron: Some("not cron".into()),
        time_zone: Some("Nowhere/Invalid".into()),
        webhook: None,
        keep_last_successful: None,
        timeout_seconds: None,
        alert_on_failure: false,
        run_as_actor_id: None,
        tag_ids: vec![],
    };
    assert!(matches!(
        input.validate(actor),
        Err(BackupError::Validation(_))
    ));
}
