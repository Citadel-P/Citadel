use super::*;
use citadel_builds::{BuildChange, BuildRunStatus};

pub(super) async fn verify(pool: &sqlx::PgPool, actor: ActorId, input: &BuildProjectConfiguration) {
    let store = PostgresBuildRepository::new(pool.clone());
    let mut input = input.clone();
    input.name = format!("claim-a-{}", Uuid::now_v7());
    let a = store.create(actor, &input).await.unwrap();
    input.name = format!("claim-b-{}", Uuid::now_v7());
    let b = store.create(actor, &input).await.unwrap();
    let cutoff = Utc::now() - Duration::minutes(5);
    let run = store.enqueue(actor, a.id, "Manual").await.unwrap();
    let first = store.claim_next(cutoff).await.unwrap();
    assert_eq!(first.claim.unwrap().run.id, run.id);
    assert_eq!(first.change, Some(BuildChange::ProjectRuns(a.id)));

    assert_read_only_probe(&store, pool, cutoff).await;
    sqlx::query("UPDATE buildruns SET startedat=now()-interval '2 days' WHERE id=$1")
        .bind(run.id)
        .execute(pool)
        .await
        .unwrap();
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-build-claim'))")
        .execute(&mut *lock)
        .await
        .unwrap();
    let busy = store.claim_next(cutoff).await.unwrap();
    assert!(busy.claim.is_none() && busy.change.is_none());
    assert_eq!(
        store.get_run(run.id).await.unwrap().status,
        BuildRunStatus::Preparing
    );
    lock.rollback().await.unwrap();

    let recovered = store.claim_next(cutoff).await.unwrap();
    assert!(recovered.claim.is_none());
    assert_eq!(recovered.change, Some(BuildChange::ProjectRuns(a.id)));
    assert_eq!(
        store.get_run(run.id).await.unwrap().status,
        BuildRunStatus::Interrupted
    );

    let expired = store.enqueue(actor, a.id, "Manual").await.unwrap();
    sqlx::query("UPDATE buildruns SET nextattemptat=now()+interval '1 hour' WHERE id=$1")
        .bind(expired.id)
        .execute(pool)
        .await
        .unwrap();
    assert_read_only_probe(&store, pool, cutoff).await;
    sqlx::query("UPDATE buildruns SET queuedat=now()-interval '2 days',nextattemptat=now()+interval '1 hour' WHERE id=$1")
        .bind(expired.id).execute(pool).await.unwrap();
    let expiry = store.claim_next(cutoff).await.unwrap();
    assert!(expiry.claim.is_none());
    assert_eq!(expiry.change, Some(BuildChange::ProjectRuns(a.id)));
    assert_eq!(
        store.get_run(expired.id).await.unwrap().status,
        BuildRunStatus::Failed
    );

    let stale = store.enqueue(actor, a.id, "Manual").await.unwrap();
    store.claim_next(cutoff).await.unwrap().claim.unwrap();
    sqlx::query("UPDATE buildruns SET startedat=now()-interval '2 days' WHERE id=$1")
        .bind(stale.id)
        .execute(pool)
        .await
        .unwrap();
    let other = store.enqueue(actor, b.id, "Manual").await.unwrap();
    let mixed = store.claim_next(cutoff).await.unwrap();
    assert_eq!(mixed.change, Some(BuildChange::Resync));
    let claim = mixed.claim.unwrap();
    assert_eq!(claim.run.id, other.id);
    assert_eq!(
        store.get_run(stale.id).await.unwrap().status,
        BuildRunStatus::Interrupted
    );
    assert!(
        store
            .finish(&claim, &success_result('a'), None)
            .await
            .unwrap()
    );
    let projects = [a.id, b.id];
    sqlx::query("DELETE FROM buildruns WHERE buildprojectid=ANY($1)")
        .bind(projects.as_slice())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildprojects WHERE id=ANY($1)")
        .bind(projects.as_slice())
        .execute(pool)
        .await
        .unwrap();
}

async fn assert_read_only_probe(
    store: &PostgresBuildRepository,
    pool: &sqlx::PgPool,
    cutoff: chrono::DateTime<Utc>,
) {
    let mut lock = pool.begin().await.unwrap();
    // Readers can proceed, but even a no-op UPDATE would block on this lock.
    sqlx::query("LOCK TABLE buildruns IN SHARE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), store.claim_next(cutoff))
        .await
        .expect("idle claim must finish without a write lock")
        .unwrap();
    assert!(result.claim.is_none() && result.change.is_none());
    lock.rollback().await.unwrap();
}
