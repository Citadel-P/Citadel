use super::*;
use citadel_adapters::persistence::postgres::{
    identity::authentication::store::StaticEntitlementService, updates::PostgresUpdateRepository,
};
use citadel_updates::{
    AgentUpdateKind, AgentUpdateRelease, AgentUpdateRepository, AgentUpdateTarget,
    ReleaseCheckRepository,
};

fn target(id: &str, version: &str) -> AgentUpdateTarget {
    AgentUpdateTarget {
        id: id.into(),
        name: format!("agent-{id}"),
        installed_version: Some(version.into()),
        resource_id: Uuid::now_v7(),
        kind: AgentUpdateKind::Platform,
        connected: true,
        supported: true,
        reason: None,
        nodes: 1,
        drain_nodes: vec![],
        revision: String::new(),
    }
}
async fn flush(alerts: &PostgresAlertRepository) {
    while alerts
        .process_pending_observation(Uuid::now_v7())
        .await
        .unwrap()
    {}
}
async fn counts(pool: &sqlx::PgPool, kind: &str) -> (i64, i64) {
    sqlx::query_as(
        "SELECT count(*),count(*) FILTER (WHERE resolvedat IS NULL) FROM alertevents WHERE type=$1",
    )
    .bind(kind)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn instance_availability_uses_seeded_community_rules_and_existing_outbox() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    assert!(
        url::Url::parse(&url)
            .unwrap()
            .path()
            .starts_with("/citadel_update_test_")
    );
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let updates = PostgresUpdateRepository::new(pool.clone());
    let alerts = PostgresAlertRepository::new(pool.clone())
        .with_entitlements(Arc::new(StaticEntitlementService::new(false)));
    // A successful no-update check also establishes the instance identity.
    updates.record_check(Ok(None), Utc::now()).await.unwrap();
    updates
        .observe_core_availability("0.1.2", None)
        .await
        .unwrap();
    let release = AgentUpdateRelease {
        version: "0.1.2".into(),
        image: format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "a".repeat(64)),
    };
    updates.save_release(Some(&release), None).await.unwrap();
    let mut targets = vec![
        target("one", "0.1.2"),
        target("two", "0.1.2"),
        target("three", "0.1.1"),
    ];
    targets.push(targets[2].clone()); // Alias of the same installation.
    updates
        .observe_agent_availability(&release, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "CoreUpdateAvailable").await, (0, 0));
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (1, 1));
    let event = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM alertevents WHERE type='AgentUpdatesAvailable'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(alerts.get_event(event).await.unwrap().info["Count"], 1);
    let deliveries: i64 = sqlx::query_scalar("SELECT count(*) FROM alertdeliveryoutbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(deliveries, 0, "in-app alerts need no external channel");
    updates
        .observe_agent_availability(&release, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (1, 1));
    // Failed checks retain the last valid availability observation.
    updates.save_release(None, Some("failed")).await.unwrap();
    updates
        .record_check(Err("failed"), Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (1, 1));
    targets.truncate(3);
    targets[2].connected = false;
    updates
        .observe_agent_availability(&release, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    let info = alerts.get_event(event).await.unwrap().info;
    assert_eq!(info["Count"], 0);
    assert_eq!(info["PendingAgents"].as_array().unwrap().len(), 1);
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (1, 1));
    assert!(!info.to_string().contains("sha256:"));
    let later_release = AgentUpdateRelease {
        version: "0.1.3".into(),
        ..release.clone()
    };
    updates
        .save_release(Some(&later_release), None)
        .await
        .unwrap();
    let mut uncertain = targets.clone();
    for target in &mut uncertain {
        target.connected = false;
    }
    updates
        .observe_agent_availability(&later_release, &uncertain, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (1, 1));
    assert_eq!(
        alerts.get_event(event).await.unwrap().info["TargetVersion"],
        "0.1.2"
    );
    updates.save_release(Some(&release), None).await.unwrap();
    let stale_inventory_started = Utc::now();
    let stale_targets = vec![target("three", "0.1.1")];
    targets[2].connected = true;
    targets[2].installed_version = Some("0.1.2".into());
    updates
        .observe_agent_availability(&release, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (1, 0));
    // A slower inventory probe begun before installation cannot reopen the event.
    updates
        .observe_agent_availability(&release, &stale_targets, stale_inventory_started)
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (1, 0));

    // Community may explicitly attach an existing channel to both built-in rules.
    let actor = ActorId::new(Uuid::from_u128(1));
    let channel = alerts
        .create_channel(
            actor,
            &AlertChannelConfiguration {
                name: "test".into(),
                alert_destination: "Generic".into(),
                url: "generic://localhost/never-sent".into(),
                is_active: true,
            },
        )
        .await
        .unwrap();
    for kind in ["CoreUpdateAvailable", "AgentUpdatesAvailable"] {
        let id: Uuid = sqlx::query_scalar("SELECT id FROM alertrules WHERE type=$1")
            .bind(kind)
            .fetch_one(&pool)
            .await
            .unwrap();
        let rule = alerts.get_rule(id).await.unwrap();
        assert_eq!(rule.severity, citadel_alerts::AlertSeverity::Info);
        alerts
            .update_rule(actor, id, &json!({"channelIds":[channel.id]}))
            .await
            .unwrap();
    }
    let newer = AgentUpdateRelease {
        version: "0.1.3".into(),
        ..release.clone()
    };
    updates.save_release(Some(&newer), None).await.unwrap();
    updates
        .observe_agent_availability(&newer, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (2, 1));
    targets[0].installed_version = Some("0.1.3".into());
    updates
        .observe_agent_availability(&newer, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    let current: Uuid = sqlx::query_scalar(
        "SELECT id FROM alertevents WHERE type='AgentUpdatesAvailable' AND resolvedat IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(alerts.get_event(current).await.unwrap().info["Count"], 2);
    let deliveries: i64 = sqlx::query_scalar("SELECT count(*) FROM alertdeliveryoutbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(deliveries, 1, "summary refresh does not notify again");
    let newest = AgentUpdateRelease {
        version: "0.1.4".into(),
        ..newer
    };
    updates.save_release(Some(&newest), None).await.unwrap();
    updates
        .observe_agent_availability(&newest, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (3, 1));
    assert!(
        alerts
            .get_event(current)
            .await
            .unwrap()
            .resolution_note
            .unwrap()
            .contains("Superseded")
    );

    updates
        .observe_core_availability("0.1.2", Some("0.1.3"))
        .await
        .unwrap();
    flush(&alerts).await;
    updates
        .observe_core_availability("0.1.2", Some("0.1.3"))
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "CoreUpdateAvailable").await, (1, 1));
    updates
        .record_check(Err("failure"), Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "CoreUpdateAvailable").await, (1, 1));
    let core_event: Uuid = sqlx::query_scalar(
        "SELECT id FROM alertevents WHERE type='CoreUpdateAvailable' AND resolvedat IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        alerts
            .get_event(core_event)
            .await
            .unwrap()
            .message
            .contains("is available")
    );
    alerts.acknowledge(actor, &[core_event]).await.unwrap();
    let deliveries_before: i64 = sqlx::query_scalar("SELECT count(*) FROM alertdeliveryoutbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    updates
        .observe_core_availability("0.1.2", Some("0.1.3"))
        .await
        .unwrap();
    flush(&alerts).await;
    assert!(
        alerts
            .get_event(core_event)
            .await
            .unwrap()
            .acknowledged_at
            .is_some()
    );
    let deliveries_after: i64 = sqlx::query_scalar("SELECT count(*) FROM alertdeliveryoutbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(deliveries_after, deliveries_before);
    // Even an explicit Alert grant cannot expose instance update notices to non-admins.
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,$4,0)")
        .bind(Uuid::now_v7()).bind(actor.value()).bind(core_event)
        .bind(citadel_primitives::ResourceType::Alert as i32).execute(&pool).await.unwrap();
    let filter = AlertEventFilter {
        alert_type: Some("CoreUpdateAvailable".into()),
        ..Default::default()
    };
    assert_eq!(
        alerts
            .list_events(actor, false, &filter)
            .await
            .unwrap()
            .total_count,
        0
    );
    assert_eq!(alerts.unresolved_count(actor, false).await.unwrap(), 0);
    assert_eq!(
        alerts
            .list_events(actor, true, &filter)
            .await
            .unwrap()
            .total_count,
        1
    );
    assert!(
        alerts
            .events_require_administrator(&[core_event])
            .await
            .unwrap()
    );
    updates
        .observe_core_availability("0.1.2", Some("0.1.4"))
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "CoreUpdateAvailable").await, (2, 1));
    assert!(
        alerts
            .get_event(core_event)
            .await
            .unwrap()
            .resolution_note
            .unwrap()
            .contains("Superseded")
    );
    // Installing an older release must leave the newer notice open.
    updates
        .observe_core_availability("0.1.3", None)
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "CoreUpdateAvailable").await, (2, 1));
    // Resolution still works with the rule disabled, including skipped versions.
    sqlx::query("UPDATE alertrules SET status='Disabled' WHERE type='CoreUpdateAvailable'")
        .execute(&pool)
        .await
        .unwrap();
    updates
        .observe_core_availability("0.1.5", None)
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "CoreUpdateAvailable").await, (2, 0));
    sqlx::query("UPDATE alertrules SET status='Enabled' WHERE type='CoreUpdateAvailable'")
        .execute(&pool)
        .await
        .unwrap();
    // Queued discovery just before installation must not resurrect a stale alert.
    updates
        .observe_core_availability("0.1.5", Some("0.1.6"))
        .await
        .unwrap();
    updates
        .observe_core_availability("0.1.6", None)
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "CoreUpdateAvailable").await, (2, 0));
    // Resolution is housekeeping, independent of rule enablement or external channels.
    sqlx::query("UPDATE alertrules SET status='Disabled' WHERE type='AgentUpdatesAvailable'")
        .execute(&pool)
        .await
        .unwrap();
    for target in &mut targets {
        target.installed_version = Some("0.1.4".into());
    }
    updates
        .observe_agent_availability(&newest, &targets, Utc::now())
        .await
        .unwrap();
    flush(&alerts).await;
    assert_eq!(counts(&pool, "AgentUpdatesAvailable").await, (3, 0));
    pool.close().await;
}
