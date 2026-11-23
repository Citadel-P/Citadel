use super::*;
use citadel_adapters::persistence::postgres::alerts::platform_metrics::PostgresPlatformAlertBatchStore;
use citadel_alerts::platform_metrics::PlatformAlertBatchStore;

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn metric_flush_replays_the_single_observation_transitions() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let platform = Uuid::now_v7();
    let rule = Uuid::now_v7();
    let channel = Uuid::now_v7();
    let system = Uuid::from_u128(1);
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount)
        VALUES($1,$1::text,'Local',1,0,1024,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).execute(&pool).await.unwrap();
    sqlx::query("UPDATE alertrules SET status='Disabled' WHERE type='PlatformCpuHigh'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO alertrules(id,name,type,severity,status,threshold,requiredmatches,cooldownseconds,createdbyactorid,limitedto,quiethours)
        VALUES($1,'replay','PlatformCpuHigh','Critical','Enabled',80,2,60,$2,'[]','[]')")
        .bind(rule).bind(system).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO alertchannels(id,alertdestination,createdbyactorid,name,url) VALUES($1,'Generic',$2,'fixture','http://localhost.invalid')")
        .bind(channel).bind(system).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO alertrulechannels VALUES($1,$2)")
        .bind(rule)
        .bind(channel)
        .execute(&pool)
        .await
        .unwrap();
    let mut expected = Vec::new();
    for batch in [false, true] {
        let alerts = Arc::new(PostgresAlertRepository::new(pool.clone()));
        let store = PostgresPlatformAlertBatchStore::new(pool.clone(), "test", alerts.clone());
        for (step, cpu) in [0., 85., 90., 95., 95., 0., 95., 95., 95., 0.]
            .into_iter()
            .enumerate()
        {
            if step == 8 {
                // Advance through the cooldown without a clock-dependent sleep.
                sqlx::query("UPDATE alertrulestates SET lasttriggeredat=now()-interval '2 minutes' WHERE alertruleid=$1").bind(rule).execute(&pool).await.unwrap();
            }
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending) VALUES($1,$2,$3,$4,1,0,0,$5)")
                .bind(id).bind(platform).bind(step as i64).bind(cpu).bind(batch).execute(&pool).await.unwrap();
            if batch {
                assert_eq!(store.flush_pending(32).await.unwrap(), 1);
            } else {
                store
                    .observe_latest(alerts.as_ref(), platform)
                    .await
                    .unwrap();
            }
            // Compare stable identities, timestamps' presence, incident payload and delivery association.
            // Each independent run generates its own event IDs and wall-clock timestamps.
            let snapshot: serde_json::Value = sqlx::query_scalar(r#"SELECT jsonb_build_object(
                'state',(SELECT jsonb_build_array(consecutivematches,lasttriggeredat IS NOT NULL,lastobservedat IS NOT NULL,createdbyactorid) FROM alertrulestates WHERE alertruleid=$1 AND resourceid=$2),
                'events',(SELECT jsonb_agg(jsonb_build_array(type,severity,info::jsonb,resourceid,resourcename,resourcetype,deduplicationkey,openincidentkey,resolutionnote,resolvedbyactorid,resolvedat IS NOT NULL,
                  (SELECT jsonb_agg(jsonb_build_array(alertchannelid,status,attemptcount) ORDER BY alertchannelid) FROM alertdeliveryoutbox WHERE alerteventid=e.id)) ORDER BY createdat,id)
                  FROM alertevents e WHERE alertruleid=$1),
                'pending',(SELECT count(*) FROM platformstats WHERE platformid=$2 AND alertpending))"#)
                .bind(rule).bind(platform).fetch_one(&pool).await.unwrap();
            if batch {
                assert_eq!(snapshot, expected[step], "replay step {step}");
            } else {
                expected.push(snapshot);
            }
        }
        let (events, deliveries): (i64, i64) = sqlx::query_as(
            "SELECT count(DISTINCT event.id),count(outbox.id) FROM alertevents event
             LEFT JOIN alertdeliveryoutbox outbox ON outbox.alerteventid=event.id
             WHERE event.alertruleid=$1",
        )
        .bind(rule)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((events, deliveries), (2, 2));
        sqlx::query("DELETE FROM alertevents WHERE alertruleid=$1")
            .bind(rule)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM alertrulestates WHERE resourceid=$1")
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM platformstats WHERE platformid=$1")
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
    }
}
