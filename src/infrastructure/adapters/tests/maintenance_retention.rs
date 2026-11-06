use citadel_adapters::persistence::postgres::resource_copies::PostgresCopyRepository;
use citadel_primitives::ActorId;
use citadel_resource_copies::CopyRepository;
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn copy_review_expiry_preserves_started_operations_and_skips_locked_reviews() {
    let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',2,0,1000,0,0,'{}','Online')")
        .bind(platform).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO resourcecopies(id,actorid,kind,sourceid,name,platformid,plan,status,createdat)
        SELECT gen_random_uuid(),$1,'Deployment',gen_random_uuid(),n::text,$2,'{}','Review',now()-interval '2 days' FROM generate_series(1,102) n")
        .bind(actor).bind(platform).execute(&pool).await.unwrap();
    let mut protected = Vec::new();
    let mut pending = Vec::new();
    for status in [
        "Review",
        "Restoring",
        "Cancelling",
        "Cleaning",
        "Succeeded",
        "Failed",
        "Cancelled",
        "Cleaned",
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO resourcecopies(id,actorid,kind,sourceid,name,platformid,plan,status,createdat) VALUES($1,$2,'Deployment',$1,$1::text,$3,'{}',$4,CASE WHEN $4='Review' THEN now() ELSE now()-interval '2 days' END)")
            .bind(id).bind(actor).bind(platform).bind(status).execute(&pool).await.unwrap();
        protected.push(id);
        if ["Restoring", "Cancelling", "Cleaning"].contains(&status) {
            pending.push((ActorId::new(actor), id));
        }
    }
    let mut lock = pool.begin().await.unwrap();
    let held: Uuid = sqlx::query_scalar("SELECT id FROM resourcecopies WHERE status='Review' AND createdat<now()-interval '1 day' LIMIT 1 FOR UPDATE")
        .fetch_one(&mut *lock).await.unwrap();
    let copies = PostgresCopyRepository::new(pool.clone());
    let actual = tokio::time::timeout(std::time::Duration::from_secs(5), copies.pending())
        .await
        .expect("expiry must skip the locked review")
        .unwrap();
    assert_eq!(actual.len(), pending.len());
    assert!(pending.iter().all(|entry| actual.contains(entry)));
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM resourcecopies WHERE status='Review' AND createdat<now()-interval '1 day'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(
        remaining, 2,
        "one bounded batch deletes at most 100 reviews"
    );
    // Simulate the user's start transaction winning the race with expiry.
    sqlx::query("UPDATE resourcecopies SET status='Restoring' WHERE id=$1")
        .bind(held)
        .execute(&mut *lock)
        .await
        .unwrap();
    lock.commit().await.unwrap();
    protected.push(held);
    let actual = copies.pending().await.unwrap();
    assert!(actual.contains(&(ActorId::new(actor), held)));
    let remaining: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM resourcecopies")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(remaining.len(), protected.len());
    assert!(protected.iter().all(|id| remaining.contains(id)));
    pool.close().await;
}
