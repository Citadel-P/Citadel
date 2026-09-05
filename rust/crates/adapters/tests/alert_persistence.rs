use chrono::{Duration, Utc};
use citadel_adapters::alert_store::PostgresAlertStore;
use citadel_alerts::{
    AlertChannelInput, AlertError, AlertEventFilter, AlertObservation, AlertRuleInput, AlertStore,
    NewAlertEvent,
};
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn alert_mutations_are_atomic_and_incidents_are_deduplicated() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    let notifications = Arc::new(AtomicUsize::new(0));
    let changes = notifications.clone();
    let store = PostgresAlertStore::new(pool.clone()).with_change_notifier(Arc::new(move || {
        changes.fetch_add(1, Ordering::SeqCst);
    }));
    let channel = store
        .create_channel(
            actor,
            &AlertChannelInput {
                name: format!("phase7-channel-{}", Uuid::now_v7().simple()),
                alert_destination: "Generic".into(),
                url: "https://alerts.example.test/hook".into(),
                is_active: true,
            },
        )
        .await
        .unwrap();

    let input = |channel_ids| AlertRuleInput {
        name: format!("phase7-rule-{}", Uuid::now_v7().simple()),
        description: None,
        alert_type: "BuildRunFailed".into(),
        severity: "Critical".into(),
        cooldown_seconds: Some(60),
        required_matches: None,
        threshold: None,
        status: "Enabled".into(),
        channel_ids,
        limited_to: vec![],
        quiet_hours: vec![],
    };
    assert!(matches!(
        store.create_rule(actor, &input(vec![Uuid::now_v7()])).await,
        Err(AlertError::Validation(_))
    ));
    let rule = store
        .create_rule(actor, &input(vec![channel.id]))
        .await
        .unwrap();
    let event = NewAlertEvent {
        alert_rule_id: rule.id,
        alert_type: rule.alert_type.clone(),
        severity: rule.severity.clone(),
        info: json!({"humanMessage":"Build failed."}),
        resource_id: Some(Uuid::now_v7()),
        resource_name: "build".into(),
        resource_type: "Build".into(),
        deduplication_key: format!("phase7-incident-{}", Uuid::now_v7()),
    };
    let raised = store.raise(&event).await.unwrap().unwrap();
    assert!(store.raise(&event).await.unwrap().is_none());
    assert_eq!(
        notifications.load(Ordering::SeqCst),
        1,
        "deduplicated incidents do not notify twice"
    );
    let owner = Uuid::now_v7();
    let (left, right) = tokio::join!(
        store.claim_delivery(owner, Utc::now() - Duration::minutes(2)),
        store.claim_delivery(owner, Utc::now() - Duration::minutes(2))
    );
    let claims = [left.unwrap(), right.unwrap()];
    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
    let claim = claims.into_iter().flatten().next().unwrap();
    assert_eq!(claim.event.id, raised.id);
    assert_eq!(claim.channel.id, channel.id);
    assert!(
        store
            .retry_delivery(
                claim.id,
                owner,
                Utc::now() - Duration::seconds(1),
                false,
                "temporary failure",
            )
            .await
            .unwrap()
    );
    let retried = store
        .claim_delivery(owner, Utc::now() - Duration::minutes(2))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retried.attempt_count, 1);
    assert!(store.complete_delivery(retried.id, owner).await.unwrap());
    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM alertdeliveryoutbox WHERE alerteventid=$1")
            .bind(raised.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pending, 0);

    let evaluated_rule = store
        .create_rule(
            actor,
            &AlertRuleInput {
                name: format!("phase7-evaluated-rule-{}", Uuid::now_v7().simple()),
                description: None,
                alert_type: "Phase7EvaluationProbe".into(),
                severity: "Warning".into(),
                cooldown_seconds: Some(60),
                required_matches: None,
                threshold: None,
                status: "Enabled".into(),
                channel_ids: vec![channel.id],
                limited_to: vec![],
                quiet_hours: vec![],
            },
        )
        .await
        .unwrap();
    let observed_resource = Uuid::now_v7();
    let observation = AlertObservation {
        alert_type: "Phase7EvaluationProbe".into(),
        info: json!({"HumanMessage":"Evaluation matched."}),
        resource_id: observed_resource,
        resource_name: "probe".into(),
        resource_type: "Build".into(),
        deduplication_component: "first".into(),
        observed_at: Utc::now(),
        value: None,
        matched: true,
    };
    let evaluated = store.process_event(&observation).await.unwrap().unwrap();
    assert_eq!(evaluated.alert_rule_id, evaluated_rule.id);
    let within_cooldown = AlertObservation {
        deduplication_component: "different-incident".into(),
        observed_at: observation.observed_at + Duration::seconds(1),
        ..observation
    };
    assert!(
        store
            .process_event(&within_cooldown)
            .await
            .unwrap()
            .is_none()
    );
    let evaluated_claim = store
        .claim_delivery(owner, Utc::now() - Duration::minutes(2))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(evaluated_claim.event.id, evaluated.id);
    assert!(
        store
            .complete_delivery(evaluated_claim.id, owner)
            .await
            .unwrap()
    );
    let threshold_rule = store
        .create_rule(
            actor,
            &AlertRuleInput {
                name: format!("phase7-threshold-rule-{}", Uuid::now_v7().simple()),
                description: None,
                alert_type: "PlatformCpuHigh".into(),
                severity: "Warning".into(),
                cooldown_seconds: None,
                required_matches: Some(2),
                threshold: Some(80.0),
                status: "Enabled".into(),
                channel_ids: vec![],
                limited_to: vec![],
                quiet_hours: vec![],
            },
        )
        .await
        .unwrap();
    let metric = AlertObservation {
        alert_type: "PlatformCpuHigh".into(),
        info: json!({"HumanMessage":"CPU is high."}),
        resource_id: observed_resource,
        resource_name: "probe".into(),
        resource_type: "Platform".into(),
        deduplication_component: "utilization".into(),
        observed_at: Utc::now(),
        value: Some(90.0),
        matched: true,
    };
    let before = notifications.load(Ordering::SeqCst);
    assert!(store.process_event(&metric).await.unwrap().is_none());
    assert_eq!(notifications.load(Ordering::SeqCst), before);
    let threshold_event = store
        .process_event(&AlertObservation {
            observed_at: metric.observed_at + Duration::seconds(1),
            ..metric.clone()
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(threshold_event.alert_rule_id, threshold_rule.id);
    assert_eq!(notifications.load(Ordering::SeqCst), before + 1);
    assert!(
        store
            .process_event(&AlertObservation {
                observed_at: metric.observed_at + Duration::seconds(2),
                value: Some(10.0),
                ..metric
            })
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store.get_event(threshold_event.id).await.unwrap().status,
        "Resolved"
    );
    assert_eq!(
        notifications.load(Ordering::SeqCst),
        before + 2,
        "automatic resolution also notifies"
    );
    store.acknowledge(actor, &[raised.id]).await.unwrap();
    assert_eq!(
        store.get_event(raised.id).await.unwrap().status,
        "Acknowledged"
    );
    store
        .resolve(actor, &[raised.id], Some("recovered"))
        .await
        .unwrap();
    assert_eq!(store.get_event(raised.id).await.unwrap().status, "Resolved");
    assert_eq!(
        store
            .list_events(
                actor,
                true,
                &AlertEventFilter {
                    page: 1,
                    page_size: 10,
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .total_count,
        3
    );

    let missing = Uuid::now_v7();
    assert!(matches!(
        store.delete_channels(&[channel.id, missing]).await,
        Err(AlertError::NotFound)
    ));
    assert_eq!(store.get_channel(channel.id).await.unwrap().id, channel.id);
    assert!(matches!(
        store.delete_rules(&[rule.id, missing]).await,
        Err(AlertError::NotFound)
    ));
    assert_eq!(store.get_rule(rule.id).await.unwrap().id, rule.id);

    store
        .delete_rules(&[rule.id, evaluated_rule.id, threshold_rule.id])
        .await
        .unwrap();
    store.delete_channels(&[channel.id]).await.unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
}
