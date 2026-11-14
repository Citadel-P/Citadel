use super::*;
use citadel_adapters::persistence::postgres::alerts::platform_metrics::PostgresPlatformAlertBatchStore;
use citadel_alerts::platform_metrics::PlatformAlertBatchStore;

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn cold_metric_flush_completes_with_one_query_connection() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,'Local',2,0,1024,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).execute(&pool).await.unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    let rule = Uuid::now_v7();
    sqlx::query("INSERT INTO alertrules(id,name,type,severity,status,threshold,requiredmatches,createdbyactorid,limitedto,quiethours) VALUES($1,'pool-one','PlatformCpuHigh','Critical','Enabled',10,1,$3,$2,'[]')")
        .bind(rule).bind(json!([{"resourceId":platform,"resourceType":"Platform"}])).bind(actor).execute(&pool).await.unwrap();
    let sample = Uuid::now_v7();
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending) VALUES($1,$2,1,20,20,0,0,true)")
        .bind(sample).bind(platform).execute(&pool).await.unwrap();
    struct DatabaseEntitlements(sqlx::PgPool);
    impl citadel_alerts::AlertEntitlements for DatabaseEntitlements {
        fn advanced_alerting(
            &self,
        ) -> futures_util::future::BoxFuture<'_, Result<bool, AlertError>> {
            Box::pin(async {
                sqlx::query_scalar("SELECT true")
                    .fetch_one(&self.0)
                    .await
                    .map_err(|e| AlertError::Storage(e.to_string()))
            })
        }
    }
    let notifications = Arc::new(AtomicUsize::new(0));
    let changed = notifications.clone();
    let alerts = PostgresAlertRepository::new(pool.clone())
        .with_entitlements(Arc::new(DatabaseEntitlements(pool.clone())))
        .with_change_notifier(Arc::new(move || {
            changed.fetch_add(1, Ordering::SeqCst);
        }));
    let store = PostgresPlatformAlertBatchStore::new(pool.clone(), "0.1.5", Arc::new(alerts));
    let backlog = store.backlog().await.unwrap();
    assert!(backlog.pending > 0 && backlog.blocks_compaction);
    assert_eq!(backlog.oldest_created, Some(1));
    // A failure after alert evaluation must neither notify nor commit events.
    sqlx::raw_sql("CREATE FUNCTION fixture_reject_metric_ack() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NOT NEW.alertpending THEN RAISE EXCEPTION 'fixture acknowledgement failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fixture_reject_metric_ack BEFORE UPDATE ON platformstats FOR EACH ROW EXECUTE FUNCTION fixture_reject_metric_ack();").execute(&pool).await.unwrap();
    assert!(store.flush_pending(500).await.is_err());
    assert_eq!(notifications.load(Ordering::SeqCst), 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM alertevents WHERE alertruleid=$1")
        .bind(rule)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::raw_sql("DROP TRIGGER fixture_reject_metric_ack ON platformstats; DROP FUNCTION fixture_reject_metric_ack();").execute(&pool).await.unwrap();
    let outcome = store.flush_pending(500).await;
    eprintln!("pool_of_one_flush={outcome:?}");
    assert!(
        outcome.is_ok(),
        "cold metric flush must not acquire inside its transaction: {outcome:?}"
    );
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM alertevents WHERE alertruleid=$1")
        .bind(rule)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(events, 1);
    assert_eq!(notifications.load(Ordering::SeqCst), 1);
    let pending: bool = sqlx::query_scalar("SELECT alertpending FROM platformstats WHERE id=$1")
        .bind(sample)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!pending);
    assert_eq!(store.backlog().await.unwrap().pending, 0);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn shared_parent_validation_is_transaction_local_and_preserves_incident_state() {
    let admin_url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    let admin = sqlx::PgPool::connect(&admin_url).await.unwrap();
    let database = format!("citadel_alert_parents_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(&admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&admin_url).unwrap();
    url.set_path(&database);
    MigrationRunner::migrate(url.as_str()).await.unwrap();
    let pool = sqlx::PgPool::connect(url.as_str()).await.unwrap();
    // This database is owned by the fixture; only the one shared rule participates.
    sqlx::query("UPDATE alertrules SET status='Disabled'")
        .execute(&pool)
        .await
        .unwrap();
    let rule = Uuid::now_v7();
    sqlx::query("INSERT INTO alertrules(id,name,type,severity,status,threshold,requiredmatches,createdbyactorid,limitedto,quiethours) VALUES($1,'shared-parent','PlatformCpuHigh','Critical','Enabled',80,2,'00000000-0000-0000-0000-000000000001','[]','[]')")
        .bind(rule).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) SELECT id,id::text,'Local',2,0,1024,id::text,0,'{\"$type\":\"Docker\"}','Online',0 FROM (SELECT gen_random_uuid() id FROM generate_series(1,32)) p")
        .execute(&pool).await.unwrap();
    let measured: bool = sqlx::query_scalar(
        "SELECT current_setting('shared_preload_libraries') LIKE '%pg_stat_statements%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    if measured {
        sqlx::query("CREATE EXTENSION pg_stat_statements")
            .execute(&pool)
            .await
            .unwrap();
    }
    async fn checks(pool: &sqlx::PgPool) -> i64 {
        sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query='SELECT status FROM alertrules WHERE id=$1 FOR SHARE'")
            .fetch_one(pool).await.unwrap()
    }
    let notifications = Arc::new(AtomicUsize::new(0));
    let changed = notifications.clone();
    let alerts = Arc::new(
        PostgresAlertRepository::new(pool.clone()).with_change_notifier(Arc::new(move || {
            changed.fetch_add(1, Ordering::SeqCst);
        })),
    );
    let store = Arc::new(PostgresPlatformAlertBatchStore::new(
        pool.clone(),
        "0.1.8",
        alerts,
    ));
    for (created, cpu, open, expected_notifications) in
        [(1_i64, 90., 0_i64, 0), (2, 90., 32, 1), (3, 10., 0, 2)]
    {
        sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending) SELECT gen_random_uuid(),id,$1,$2,20,0,0,true FROM platforms")
            .bind(created).bind(cpu).execute(&pool).await.unwrap();
        if created == 1 {
            // Block every resource advisory lock, so evaluation must wait after
            // validating a parent. Configuration cannot disable it mid-flush.
            let mut held = pool.begin().await.unwrap();
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text||':'||id::text,0)) FROM platforms ORDER BY id")
                .bind(rule.to_string()).execute(&mut *held).await.unwrap();
            let running = store.clone();
            let flush = tokio::spawn(async move { running.flush_pending(500).await });
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                loop {
                    let locked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE relation='alertrules'::regclass AND mode='RowShareLock' AND granted)")
                        .fetch_one(&pool).await.unwrap();
                    if locked { break; }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }).await.unwrap();
            let mut edit = pool.begin().await.unwrap();
            sqlx::query("SET LOCAL lock_timeout='100ms'")
                .execute(&mut *edit)
                .await
                .unwrap();
            assert!(
                sqlx::query("UPDATE alertrules SET status='Disabled' WHERE id=$1")
                    .bind(rule)
                    .execute(&mut *edit)
                    .await
                    .is_err()
            );
            edit.rollback().await.unwrap();
            flush.abort();
            assert!(flush.await.unwrap_err().is_cancelled());
            held.rollback().await.unwrap();
            assert_eq!(store.backlog().await.unwrap().pending, 32);
            assert_eq!(notifications.load(Ordering::SeqCst), 0);
        }
        let before = if measured { checks(&pool).await } else { 0 };
        assert_eq!(store.flush_pending(500).await.unwrap(), 32);
        if measured {
            assert_eq!(
                checks(&pool).await - before,
                1,
                "one parent validation for 32 resources per transaction"
            );
        }
        let active: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM alertevents WHERE alertruleid=$1 AND resolvedat IS NULL",
        )
        .bind(rule)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(active, open);
        assert_eq!(notifications.load(Ordering::SeqCst), expected_notifications);
    }
    // Cached definitions intentionally remain Enabled after a lost invalidation.
    // Neither successful parent validation nor mutable state may cross COMMIT.
    for delete in [false, true] {
        if delete {
            sqlx::query("DELETE FROM alertrules WHERE id=$1")
                .bind(rule)
                .execute(&pool)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE alertrules SET status='Disabled' WHERE id=$1")
                .bind(rule)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query("UPDATE platformstats SET alertpending=true,cpuusage=90")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(store.flush_pending(500).await.unwrap(), 96);
        let active: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM alertevents WHERE alertruleid=$1 AND resolvedat IS NULL",
        )
        .bind(rule)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(active, 0);
        assert_eq!(notifications.load(Ordering::SeqCst), 2);
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {database} WITH (FORCE)"
    )))
    .execute(&admin)
    .await
    .unwrap();
    admin.close().await;
}
