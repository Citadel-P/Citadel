use super::*;
use citadel_builds::{BuildError, BuildLog};

pub(super) async fn verify(pool: &sqlx::PgPool, actor: ActorId, input: &BuildProjectConfiguration) {
    let store = PostgresBuildRepository::new(pool.clone());
    let mut input = input.clone();
    input.name = format!("attempt-fence-{}", Uuid::now_v7());
    let project = store.create(actor, &input).await.unwrap();
    let queued = store.enqueue(actor, project.id, "Manual").await.unwrap();
    let first = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.run.id, queued.id);
    let persisted: Uuid = sqlx::query_scalar("SELECT attemptid FROM buildruns WHERE id=$1")
        .bind(queued.id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(persisted, first.attempt_id);
    let log = BuildLog {
        stream: "stdout".into(),
        message: "owned output".into(),
    };
    store
        .append_log(queued.id, first.attempt_id, &log)
        .await
        .unwrap();

    // The only retry path is a verified pre-start capacity refusal.
    let deferred = BuildExecutionResult {
        status: citadel_builds::BuildRunStatus::Queued,
        exit_code: None,
        image_digest: None,
        resolved_commit_sha: Some("a".repeat(40)),
        image_references: vec!["fixture:pinned".into()],
        error_code: Some("build.capacity_wait".into()),
        error_message: Some("Waiting for executor build capacity.".into()),
        logs: vec![],
    };
    assert!(store.finish(&first, &deferred, None).await.unwrap());
    assert!(!store.finish(&first, &deferred, None).await.unwrap());
    let waiting = store.get_run(queued.id).await.unwrap();
    assert_eq!(waiting.status, citadel_builds::BuildRunStatus::Queued);
    assert_eq!(waiting.queued_at, queued.queued_at);
    assert_eq!(waiting.resolved_commit_sha, deferred.resolved_commit_sha);
    assert_eq!(waiting.image_references, deferred.image_references);
    let backed_off: bool = sqlx::query_scalar("SELECT nextattemptat>now() AND nextattemptat<now()+interval '20 seconds' FROM buildruns WHERE id=$1")
        .bind(queued.id).fetch_one(pool).await.unwrap();
    assert!(backed_off);
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none()
    );
    // Independent work remains eligible behind the delayed oldest run.
    let mut other_input = input.clone();
    other_input.name = format!("eligible-{}", Uuid::now_v7());
    let other_platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)").bind(other_platform).bind(format!("independent-{other_platform}")).execute(pool).await.unwrap();
    other_input.platform_id = Some(other_platform);
    let other = store.create(actor, &other_input).await.unwrap();
    let other_run = store.enqueue(actor, other.id, "Manual").await.unwrap();
    let other_claim = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(other_claim.run.id, other_run.id);
    let mut failed = success_result('a');
    failed.status = citadel_builds::BuildRunStatus::Failed;
    assert!(store.finish(&other_claim, &failed, None).await.unwrap());
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(other.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildruns WHERE buildprojectid=$1")
        .bind(other.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM resourcetags WHERE resourcetype='Build' AND resourceid=$1")
        .bind(other.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildprojects WHERE id=$1")
        .bind(other.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE buildruns SET nextattemptat=now()-interval '1 second' WHERE id=$1")
        .bind(queued.id)
        .execute(pool)
        .await
        .unwrap();
    let mut second = store
        .claim_next(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.run.id, first.run.id);
    assert_eq!(second.run.queued_at, queued.queued_at);
    assert_ne!(second.attempt_id, first.attempt_id);
    assert!(
        !store
            .finish(&first, &success_result('a'), None)
            .await
            .unwrap()
    );
    assert!(matches!(
        store.append_log(queued.id, first.attempt_id, &log).await,
        Err(BuildError::Conflict(_))
    ));
    assert_eq!(store.logs(queued.id).await.unwrap().len(), 1);
    assert_eq!(
        store.get(project.id).await.unwrap().current_run_id,
        Some(queued.id)
    );

    // Hold an ownership change open while old writes wait for the row. They
    // must recheck the token after the concurrent transaction commits.
    let mut tx = pool.begin().await.unwrap();
    let replacement = Uuid::now_v7();
    sqlx::query("UPDATE buildruns SET attemptid=$2 WHERE id=$1")
        .bind(queued.id)
        .bind(replacement)
        .execute(&mut *tx)
        .await
        .unwrap();
    {
        let outcome = success_result('a');
        let finish = store.finish(&second, &outcome, None);
        let append = store.append_log(queued.id, second.attempt_id, &log);
        tokio::pin!(finish, append);
        tokio::select! {
            result = &mut finish => panic!("completion bypassed the ownership lock: {result:?}"),
            result = &mut append => panic!("log write bypassed the ownership lock: {result:?}"),
            () = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
        }
        tx.commit().await.unwrap();
        // Both queries may acquire row locks while rechecking the new token.
        // Keep polling both futures so either can complete its rollback.
        let (finished, appended) =
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                tokio::join!(finish, append)
            })
            .await
            .expect("stale writers must finish after ownership changes");
        assert!(!finished.unwrap());
        assert!(matches!(appended, Err(BuildError::Conflict(_))));
    }
    second.attempt_id = replacement;

    // The attempt token alone cannot authorize writes after losing project ownership.
    sqlx::query("UPDATE buildprojects SET currentrunid=NULL WHERE id=$1")
        .bind(project.id)
        .execute(pool)
        .await
        .unwrap();
    assert!(matches!(
        store.append_log(queued.id, second.attempt_id, &log).await,
        Err(BuildError::Conflict(_))
    ));
    assert!(
        !store
            .finish(&second, &success_result('a'), None)
            .await
            .unwrap()
    );
    sqlx::query("UPDATE buildprojects SET currentrunid=$2 WHERE id=$1")
        .bind(project.id)
        .bind(queued.id)
        .execute(pool)
        .await
        .unwrap();
    store
        .append_log(queued.id, second.attempt_id, &log)
        .await
        .unwrap();
    assert!(
        store
            .finish(&second, &success_result('b'), None)
            .await
            .unwrap()
    );
    assert!(
        !store
            .finish(&first, &success_result('a'), None)
            .await
            .unwrap()
    );
    assert!(
        !store
            .finish(&second, &success_result('b'), None)
            .await
            .unwrap()
    );
    assert_eq!(
        store.get_run(queued.id).await.unwrap().image_digest,
        success_result('b').image_digest
    );
    assert!(matches!(
        store.append_log(queued.id, second.attempt_id, &log).await,
        Err(BuildError::Conflict(_))
    ));
    let expired = store.enqueue(actor, project.id, "Manual").await.unwrap();
    sqlx::query("UPDATE buildruns SET queuedat=now()-interval '25 hours' WHERE id=$1")
        .bind(expired.id)
        .execute(pool)
        .await
        .unwrap();
    // Queue expiry is enforced before admission, including runs that have
    // never obtained an executor. Expired work must not be claimed first.
    assert!(
        store
            .claim_next(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none()
    );
    let expired = store.get_run(expired.id).await.unwrap();
    assert_eq!(expired.status, citadel_builds::BuildRunStatus::Failed);
    assert_eq!(expired.error_code.as_deref(), Some("build.queue_timeout"));
    assert!(
        store
            .get(project.id)
            .await
            .unwrap()
            .current_run_id
            .is_none()
    );
    sqlx::query("DELETE FROM buildcompletionqueue WHERE buildrunid=$1")
        .bind(queued.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(project.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildruns WHERE buildprojectid=$1")
        .bind(project.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM resourcetags WHERE resourcetype='Build' AND resourceid=$1")
        .bind(project.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildprojects WHERE id=$1")
        .bind(project.id)
        .execute(pool)
        .await
        .unwrap();
}
