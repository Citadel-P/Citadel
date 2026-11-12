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
