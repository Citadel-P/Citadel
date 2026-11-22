//! One fresh database per run: independent connections exercise committed notifications and admission.
use chrono::{DateTime, Utc};
use citadel_adapters::persistence::postgres::{
    instance::PostgresSchedules, scripts::PostgresScriptRepository,
};
use citadel_database::MigrationRunner;
use citadel_instance::{SchedulePurpose as Purpose, ScheduleReader};
use citadel_scripts::ScriptRepository;
use sqlx::{PgPool, postgres::PgListener};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires an isolated CITADEL_EXECUTION_DATABASE_URL"]
async fn catalog_changes_require_commit_and_failed_admission_can_retry() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let catalog = PostgresSchedules(pool.clone());
    let scripts = PostgresScriptRepository::new(pool.clone());
    let actor = citadel_identity::SYSTEM_ACTOR_ID;
    let owner = Uuid::now_v7();
    let mut listener = PgListener::connect(&url).await.unwrap();
    listener.listen("citadel_schedules").await.unwrap();
    let mut change = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO scripts(id,name,code,enabled,scheduleenabled,schedulecron,scheduletimezone,alertonfailure,timeoutseconds,createdbyactorid,runasactorid)
                 VALUES($1,$1::text,'return true',true,true,'* * * * *','UTC',false,30,$2,$2)")
        .bind(owner).bind(actor).execute(&mut *change).await.unwrap();
    assert!(catalog.one(Purpose::Script, owner).await.unwrap().is_none());
    change.commit().await.unwrap();
    let wake = tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(wake.payload(), format!("Script:{owner}"));
    let minute = DateTime::from_timestamp(Utc::now().timestamp().div_euclid(60) * 60, 0).unwrap();
    let def = catalog.one(Purpose::Script, owner).await.unwrap().unwrap();
    assert_eq!(def.script_timeout_seconds, Some(30));
    assert_eq!(def.next_time(minute), Some(minute));

    // Abort after creating the execution but before the watermark/transaction can commit.
    sqlx::raw_sql(
        "CREATE FUNCTION reject_test_admission() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'injected admission failure'; END $$;
        CREATE TRIGGER reject_test_admission BEFORE UPDATE OF lastscheduledrunat ON scripts
        FOR EACH ROW EXECUTE FUNCTION reject_test_admission()",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(scripts.enqueue_scheduled(owner, minute, 30).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM scriptruns WHERE scriptid=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(
        catalog
            .one(Purpose::Script, owner)
            .await
            .unwrap()
            .unwrap()
            .anchor
            .is_none()
    );
    sqlx::raw_sql(
        "DROP TRIGGER reject_test_admission ON scripts; DROP FUNCTION reject_test_admission()",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        scripts
            .enqueue_scheduled(owner, minute, 30)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        scripts
            .enqueue_scheduled(owner, minute, 30)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        catalog
            .one(Purpose::Script, owner)
            .await
            .unwrap()
            .unwrap()
            .anchor,
        Some(minute)
    );

    let current_status = || async {
        catalog
            .one(Purpose::Script, owner)
            .await
            .unwrap()
            .unwrap()
            .active_run_status
    };
    assert_eq!(current_status().await.as_deref(), Some("Queued"));
    let mut run_change = pool.begin().await.unwrap();
    sqlx::query("UPDATE scriptruns SET status='Running' WHERE scriptid=$1")
        .bind(owner)
        .execute(&mut *run_change)
        .await
        .unwrap();
    assert_eq!(current_status().await.as_deref(), Some("Queued"));
    run_change.commit().await.unwrap();
    let notice = tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(notice.payload(), "Catalog");
    assert_eq!(current_status().await.as_deref(), Some("Running"));
    sqlx::query("UPDATE scriptruns SET trigger='Manual' WHERE scriptid=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(current_status().await, None);

    // A backup can remain active while it applies retention.
    for statement in [
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$1::text,'InternalEncrypted')",
        "INSERT INTO backuprepositories(id,name,normalizedname,createdbyactorid,passwordsecretid,spec,status,type)
         VALUES($1,$1::text,$1::text,$2,$1,'{}','Ready','Local')",
        "INSERT INTO backuppolicies(id,name,normalizedname,createdbyactorid,runasactorid,backuprepositoryid,source,cron)
         VALUES($1,$1::text,$1::text,$2,$2,$1,'{}','0 1 * * *')",
        "INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,sourcesnapshot,trigger,status,snapshotavailability,triggeredbyactorid)
         VALUES($1,$1,$1,'fixture','Local','{}','Schedule','ApplyingRetention','Pending',$2)",
        "UPDATE backuppolicies SET currentrunid=$1 WHERE id=$1",
    ] {
        let mut query = sqlx::query(sqlx::AssertSqlSafe(statement)).bind(owner);
        if statement.contains("$2") {
            query = query.bind(actor);
        }
        query.execute(&pool).await.unwrap();
    }
    let backup = || async { catalog.one(Purpose::Backup, owner).await.unwrap().unwrap() };
    assert_eq!(
        backup().await.active_run_status.as_deref(),
        Some("ApplyingRetention")
    );
    sqlx::query("UPDATE backupruns SET status='Succeeded' WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    assert!(backup().await.active_run_status.is_none());

    // Filter before applying the page size, across both selected purpose families.
    let policy = citadel_instance::ScheduleCatalogPolicy {
        max_script_timeout: 30,
        ..Default::default()
    };
    let kinds = [Purpose::Release, Purpose::Install];
    let first = catalog.page(&kinds, "CoRe", 0, 1, &policy).await.unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].purpose, Purpose::Install);
    assert!(first[0].resource_status.is_none());
    let next = catalog.page(&kinds, "CoRe", 1, 1, &policy).await.unwrap();
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].purpose, Purpose::Release);
    assert!(next[0].resource_status.is_none());
    assert!(
        catalog
            .page(&kinds, "%", 0, 1, &policy)
            .await
            .unwrap()
            .is_empty()
    );
    let mut all = Vec::new();
    loop {
        let page = catalog
            .page(&Purpose::ALL, "", all.len() as i64, 2, &policy)
            .await
            .unwrap();
        if page.is_empty() {
            break;
        }
        all.extend(page.into_iter().map(|d| (d.purpose, d.owner)));
    }
    assert!(all.windows(2).all(|w| w[0] < w[1]));
    let discovered = catalog.discover(Purpose::Install, None, 1).await.unwrap();
    assert_eq!(discovered[0].owner, Uuid::nil());
    assert!(
        catalog
            .discover(Purpose::Install, Some(Uuid::nil()), 1)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(all.contains(&(Purpose::Script, owner)));
    assert!(all.contains(&(Purpose::Install, Uuid::nil())));
    assert!(all.contains(&(Purpose::Release, Uuid::nil())));
    let counts = catalog.counts("", &policy).await.unwrap();
    assert_eq!(
        counts.iter().map(|row| row.total).sum::<i64>(),
        all.len() as i64
    );
    for row in counts {
        let (purpose, count) = (row.purpose, row.total);
        assert_eq!(row.attention, 0);
        assert_eq!(
            count as usize,
            all.iter().filter(|(p, _)| *p == purpose).count()
        );
    }
    let mut matched: Vec<_> = catalog
        .counts("CoRe", &policy)
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.purpose, r.total))
        .collect();
    matched.sort();
    assert_eq!(matched, [(Purpose::Install, 1), (Purpose::Release, 1)]);
    assert!(catalog.counts("%", &policy).await.unwrap().is_empty());

    // A more recent manual run must not replace the scheduled outcome.
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,sourcesnapshot,trigger,status,snapshotavailability,triggeredbyactorid,queuedat)
                 VALUES($1,$2,$2,'fixture','Local','{}','Manual','Failed','Pending',$3,now()+interval '1 second')")
        .bind(Uuid::now_v7()).bind(owner).bind(actor).execute(&pool).await.unwrap();
    let backups = catalog
        .page(&[Purpose::Backup], "", 0, 15, &policy)
        .await
        .unwrap();
    let run = backups[0].latest_run.as_ref().unwrap();
    assert_eq!(run.id, owner);
    assert_eq!(run.status, "Succeeded");
    assert!(
        catalog
            .page(&[Purpose::Script], "", 0, 15, &policy)
            .await
            .unwrap()[0]
            .latest_run
            .is_none()
    );
    let mut filter = citadel_instance::ScheduleCatalogPolicy {
        max_script_timeout: 10,
        attention_only: true,
        ..Default::default()
    };
    let blocked = catalog
        .page(&Purpose::ALL, "", 0, 1, &filter)
        .await
        .unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0].purpose, Purpose::Script);
    assert!(blocked[0].needs_attention);
    assert_eq!(
        catalog
            .counts("", &filter)
            .await
            .unwrap()
            .iter()
            .map(|r| r.attention)
            .sum::<i64>(),
        1
    );
    filter.blockers.insert(
        Purpose::Script,
        citadel_instance::SchedulePolicyBlocker::EligibilityUnknown,
    );
    assert_eq!(
        catalog
            .page(&[Purpose::Script], "", 0, 15, &filter)
            .await
            .unwrap()
            .len(),
        1
    );
    filter.max_script_timeout = 30;
    assert!(
        catalog
            .page(&[Purpose::Script], "", 0, 15, &filter)
            .await
            .unwrap()
            .is_empty()
    );
    filter.blockers.insert(
        Purpose::Backup,
        citadel_instance::SchedulePolicyBlocker::Blocked("Requires Automated Operations."),
    );
    assert_eq!(
        catalog
            .page(&[Purpose::Backup], "", 0, 15, &filter)
            .await
            .unwrap()
            .len(),
        1
    );
    sqlx::query("UPDATE backuppolicies SET enabled=false WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        catalog
            .page(&[Purpose::Backup], "", 0, 15, &filter)
            .await
            .unwrap()
            .is_empty()
    );

    sqlx::query("UPDATE scripts SET scheduleenabled=false WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let page = catalog
        .page(&[Purpose::Script], "", 0, 25, &policy)
        .await
        .unwrap();
    assert_eq!(page[0].resource_status.as_deref(), Some("Enabled"));
    assert!(!page[0].enabled);

    sqlx::query("INSERT INTO scripts(id,name,code,enabled,scheduleenabled,schedulecron,scheduletimezone,alertonfailure,timeoutseconds,createdbyactorid,runasactorid)
        SELECT n::text::uuid, 'Paged script ' || n, 'return true', true, true, '* * * * *','UTC',false,30,$1,$1
        FROM (SELECT lpad(to_hex(i),32,'0') AS n FROM generate_series(1,200) i) fixture")
        .bind(actor).execute(&pool).await.unwrap();
    let scripts = [Purpose::Script];
    let last = catalog
        .page(&scripts, "Paged script", 175, 25, &policy)
        .await
        .unwrap();
    assert_eq!(last.len(), 25);
    assert_eq!(
        (last[0].owner, last[24].owner),
        (Uuid::from_u128(176), Uuid::from_u128(200))
    );
    assert_eq!(
        catalog
            .page(&scripts, "Paged script", 0, 200, &policy)
            .await
            .unwrap()
            .len(),
        200
    );
}
