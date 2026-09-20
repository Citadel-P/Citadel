use crate::*;
use crate::{
    jobs::schedule::cron_is_due, runs::logs::allow_net_authority,
    service::source::automation_source,
};
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use uuid::Uuid;
fn input() -> AutomationActionConfiguration {
    AutomationActionConfiguration {
        name: "prune-images".to_owned(),
        description: None,
        code: "console.log('ok')".to_owned(),
        default_args_json: Some("{}".to_owned()),
        enabled: true,
        schedule_enabled: false,
        schedule_cron: None,
        schedule_time_zone: None,
        webhook: None,
        timeout_seconds: Some(30),
        alert_on_failure: true,
        run_as_actor_id: None,
        tag_ids: vec![],
    }
}

#[test]
fn validates_json_timeout_schedule_and_default_actor() {
    let actor = ActorId::new(Uuid::now_v7());
    let mut valid = input();
    valid.validate(actor).unwrap();
    assert_eq!(valid.run_as_actor_id, Some(actor.value()));
    let mut invalid = input();
    invalid.default_args_json = Some("[]".to_owned());
    assert!(invalid.validate(actor).is_err());
    assert_eq!(
        allow_net_authority("http://127.0.0.1:8000"),
        "127.0.0.1:8000"
    );
    let mut invalid = input();
    invalid.schedule_enabled = true;
    assert!(invalid.validate(actor).is_err());
}

#[test]
fn paid_trigger_policy_matches_dotnet_expansion_and_disable_cases() {
    let mut proposed = input();
    proposed.schedule_enabled = true;
    proposed.schedule_cron = Some("*/5 * * * *".into());
    proposed.validate(ActorId::new(Uuid::now_v7())).unwrap();
    assert_eq!(proposed.schedule_time_zone.as_deref(), Some("UTC"));
    assert!(changes_paid_trigger(None, &proposed));
    let current = AutomationAction {
        id: Uuid::now_v7(),
        name: proposed.name.clone(),
        description: None,
        code: proposed.code.clone(),
        default_args_json: "{}".into(),
        enabled: true,
        schedule_enabled: true,
        schedule_cron: proposed.schedule_cron.clone(),
        schedule_time_zone: "UTC".into(),
        webhook: None,
        timeout_seconds: 30,
        alert_on_failure: true,
        run_as_actor_id: proposed.run_as_actor_id.unwrap(),
        control_state: "Idle".into(),
        current_run_id: None,
        row_version: 1,
        created_by_actor_id: Uuid::now_v7(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_scheduled_run_at: None,
        tags: vec![],
        latest_run: None,
    };
    proposed.code = "console.log('changed');".into();
    assert!(!changes_paid_trigger(Some(&current), &proposed));
    proposed.schedule_cron = Some("*/10 * * * *".into());
    assert!(changes_paid_trigger(Some(&current), &proposed));
    proposed.schedule_enabled = false;
    assert!(!changes_paid_trigger(Some(&current), &proposed));
    proposed.webhook = Some(serde_json::from_value(serde_json::json!({"enabled":true})).unwrap());
    let with_webhook = AutomationAction {
        webhook: proposed.webhook.clone(),
        ..current
    };
    assert!(!changes_paid_trigger(Some(&with_webhook), &proposed));
    proposed.webhook = Some(
        serde_json::from_value(serde_json::json!({"enabled":true,"secret":"changed"})).unwrap(),
    );
    assert!(changes_paid_trigger(Some(&with_webhook), &proposed));
    proposed.enabled = false;
    assert!(!changes_paid_trigger(Some(&with_webhook), &proposed));
}

#[test]
fn validates_webhook_authentication_and_bounded_configuration() {
    let actor = ActorId::new(Uuid::now_v7());
    let mut proposed = input();
    proposed.webhook = Some(
        serde_json::from_value(
            serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken"}),
        )
        .unwrap(),
    );
    assert!(proposed.validate(actor).is_err());
    proposed.webhook = Some(
            serde_json::from_value(serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"webhook-shared-secret"})).unwrap(),
        );
    proposed.validate(actor).unwrap();
    proposed.schedule_cron = Some("*".repeat(129));
    assert!(proposed.validate(actor).is_err());
    proposed.schedule_cron = None;
    proposed.run_as_actor_id = Some(Uuid::nil());
    assert!(proposed.validate(actor).is_err());
}

#[test]
fn cron_supports_steps_ranges_lists_and_time_zones() {
    let now = DateTime::parse_from_rfc3339("2026-07-14T08:30:00Z")
        .unwrap()
        .with_timezone(&Utc);
    assert!(cron_is_due(Some("30 10 * * 2"), "Europe/Paris", now));
    assert!(cron_is_due(Some("*/15 8-10 * * 1,2"), "UTC", now));
    assert!(!cron_is_due(Some("31 10 * * *"), "Europe/Paris", now));
    assert!(!cron_is_due(Some("* * *"), "UTC", now));
}

#[test]
fn cron_restricted_days_match_dotnet_cron_schedule_tests() {
    for (expression, timestamp, expected) in [
        ("0 9 1 * 1", "2026-07-01T09:00:00Z", true),
        ("0 9 1 * 1", "2026-06-08T09:00:00Z", true),
        ("0 9 1 * 1", "2026-06-09T09:00:00Z", false),
        ("0 9 * * 1", "2026-06-08T09:00:00Z", true),
        ("0 9 * * 1", "2026-06-09T09:00:00Z", false),
        ("0 9 8 * *", "2026-06-08T09:00:00Z", true),
        ("0 9 8 * *", "2026-06-09T09:00:00Z", false),
        ("0 9 * * 0", "2026-06-07T09:00:00Z", true),
        ("0 9 * * 7", "2026-06-07T09:00:00Z", true),
    ] {
        let now = DateTime::parse_from_rfc3339(timestamp)
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            cron_is_due(Some(expression), "UTC", now),
            expected,
            "{expression} at {timestamp}"
        );
    }
}

#[test]
fn log_redaction_masks_common_credentials() {
    let output =
        redact_logs("Bearer abc.def token=secret api_key=value password=hunter2 secret=hidden");
    assert_eq!(
        output,
        "Bearer [redacted] token=[redacted] api_key=[redacted] password=[redacted] secret=[redacted]"
    );
    assert_eq!(
        redact_logs("first\nBearer secret\npassword=value\nlast"),
        "first\nBearer [redacted]\npassword=[redacted]\nlast"
    );
}

#[test]
fn generated_script_exposes_typed_clients_without_persisting_the_token() {
    let now = Utc::now();
    let run = AutomationRun {
        id: Uuid::now_v7(),
        action_id: Uuid::now_v7(),
        action_name: "test".to_owned(),
        trigger: "Manual".to_owned(),
        status: "Running".to_owned(),
        run_as_actor_id: Uuid::now_v7(),
        triggered_by_actor_id: None,
        args_json: r#"{"value":1}"#.to_owned(),
        code_snapshot: Some("console.log(args.value);".to_owned()),
        code_hash: "hash".to_owned(),
        timeout_seconds: 30,
        queued_at: now,
        started_at: Some(now),
        finished_at: None,
        duration_ms: None,
        exit_code: None,
        logs: None,
        error_message: None,
    };
    let source = automation_source(
        "http://127.0.0.1:8000/",
        "temporary-token",
        &run,
        r#"[{"key":"listPlatforms","group":"platforms","method":"GET","path":"/api/v1/platforms"}]"#,
    );
    assert!(source.contains("__citadelGroups[endpoint.group]"));
    assert!(source.contains("listPlatforms"));
    assert!(source.contains("temporary-token"));
    assert!(source.ends_with("console.log(args.value);\n"));
}
