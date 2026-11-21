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

    let mut all = Vec::new();
    let mut cursor = None;
    loop {
        let page = catalog.page(None, cursor, 2).await.unwrap();
        if page.is_empty() {
            break;
        }
        cursor = page.last().map(|d| (d.purpose, d.owner));
        all.extend(page.into_iter().map(|d| (d.purpose, d.owner)));
    }
    assert!(all.windows(2).all(|w| w[0] < w[1]));
    assert!(all.contains(&(Purpose::Script, owner)));
    assert!(all.contains(&(Purpose::Install, Uuid::nil())));
    assert!(all.contains(&(Purpose::Release, Uuid::nil())));
}
