use super::*;
#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL"]
async fn configuration_cache_reuses_reads_invalidates_after_commit_and_expires_after_loss() {
    let url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let store=PostgresAlertRepository::new(pool.clone()).with_entitlements(Arc::new(crate::persistence::postgres::identity::authentication::store::StaticEntitlementService::new(true)));
    let kind = format!("audit-{}", Uuid::now_v7());
    let empty = store.configured_rules(&kind).await.unwrap();
    assert!(empty.is_empty());
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("LOCK alertrules IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await
        .unwrap();
    let warm = tokio::time::timeout(Duration::from_millis(100), store.configured_rules(&kind))
        .await
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&empty, &warm));
    tx.rollback().await.unwrap();
    let mut listener = sqlx::postgres::PgListener::connect(&url).await.unwrap();
    listener.listen("citadel_alert_rules").await.unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    let rule = store
        .create_rule(
            ActorId::new(actor),
            &AlertRuleConfiguration {
                name: kind.clone(),
                description: None,
                alert_type: kind.clone(),
                severity: citadel_alerts::AlertSeverity::Warning,
                cooldown_seconds: Some(60),
                required_matches: None,
                threshold: None,
                status: citadel_alerts::AlertRuleStatus::Enabled,
                channel_ids: vec![],
                limited_to: vec![],
                quiet_hours: vec![],
            },
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), listener.recv())
        .await
        .unwrap()
        .unwrap();
    let cached = store.configured_rules(&kind).await.unwrap();
    assert_eq!(cached.len(), 1);
    assert!(Arc::ptr_eq(
        &cached,
        &store.clone().configured_rules(&kind).await.unwrap()
    ));
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("UPDATE alertrules SET status='Disabled' WHERE id=$1")
        .bind(rule.id)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT pg_notify('citadel_alert_rules','')")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert!(Arc::ptr_eq(
        &cached,
        &store.configured_rules(&kind).await.unwrap()
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(30), listener.recv())
            .await
            .is_err()
    );
    // Simulate an external edit with its notification lost. Only definitions
    // expire; mutable alert state and entitlements never enter this cache.
    sqlx::query("UPDATE alertrules SET status='Disabled' WHERE id=$1")
        .bind(rule.id)
        .execute(&pool)
        .await
        .unwrap();
    store
        .configuration
        .entries
        .lock()
        .await
        .get_mut(&kind)
        .unwrap()
        .1 = Instant::now() - Duration::from_secs(61);
    assert!(store.configured_rules(&kind).await.unwrap().is_empty());
    store.delete_rules(&[rule.id]).await.unwrap();
    sqlx::query("DELETE FROM activityevents WHERE createdbyactorid=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
