use super::*;

pub(super) async fn verify_owned_drain_observations(
    pool: &sqlx::PgPool,
    repo: &PostgresUpdateRepository,
    operation: &UpdateOperation,
) {
    assert!(repo.blockers(operation).await.unwrap().is_empty());
    let mut foreign = operation.clone();
    foreign.fence = Uuid::now_v7();
    assert!(matches!(
        repo.blockers(&foreign).await,
        Err(UpdateError::Conflict(_))
    ));

    for change in [
        "revision=revision+1",
        "ownsinstallation=FALSE",
        "operation='{}'::jsonb",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE instanceupdates SET {change} WHERE id=$1"
        )))
        .bind(operation.id)
        .execute(pool)
        .await
        .unwrap();
        assert!(repo.blockers(operation).await.is_err());
        sqlx::query(
            "UPDATE instanceupdates SET revision=$2,ownsinstallation=TRUE,operation=$3 WHERE id=$1",
        )
        .bind(operation.id)
        .bind(operation.revision as i64)
        .bind(sqlx::types::Json(operation))
        .execute(pool)
        .await
        .unwrap();
    }

    let source_key = format!("update-drain-{}", Uuid::now_v7());
    sqlx::query("INSERT INTO backupsourceleases(sourcekey,createdat,expiresat,operationtype,ownerrunid) VALUES($1,now()-interval '2 days',now()-interval '1 day','Backup',$2)")
        .bind(&source_key).bind(Uuid::now_v7()).execute(pool).await.unwrap();
    assert_eq!(
        repo.blockers(operation).await.unwrap(),
        vec![UpdateDrainBlocker {
            kind: "Backup source leases".into(),
            count: 1
        }]
    );
    // Observation neither expires a lease nor releases maintenance ownership.
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM backupsourceleases WHERE sourcekey=$1")
            .bind(&source_key)
            .fetch_one(pool)
            .await
            .unwrap(),
        1
    );
    sqlx::query("DELETE FROM backupsourceleases WHERE sourcekey=$1")
        .bind(&source_key)
        .execute(pool)
        .await
        .unwrap();

    let script = Uuid::now_v7();
    let run = Uuid::now_v7();
    sqlx::query("INSERT INTO scripts(id,alertonfailure,code,createdbyactorid,enabled,name,runasactorid,scheduleenabled,scheduletimezone,timeoutseconds,controlstate) VALUES($1,false,'', $2,true,'drain-test',$2,false,'UTC',60,'Queued')")
        .bind(script).bind(operation.actor_id.value()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO scriptruns(id,scriptid,scriptname,codehash,codesnapshot,runasactorid,status,timeoutseconds,trigger) VALUES($1,$2,'drain-test','fixture','',$3,'Queued',60,'Manual')")
        .bind(run).bind(script).bind(operation.actor_id.value()).execute(pool).await.unwrap();
    assert!(
        repo.blockers(operation).await.unwrap().is_empty(),
        "queued work stays queued without blocking drain"
    );
    sqlx::query("UPDATE scriptruns SET status='Running' WHERE id=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        repo.blockers(operation).await.unwrap(),
        vec![UpdateDrainBlocker {
            kind: "Scripts".into(),
            count: 1
        }]
    );
    sqlx::query("UPDATE scriptruns SET status='Failed' WHERE id=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    let daemon = format!("fixture-drain-{}", Uuid::now_v7());
    let authority = Uuid::now_v7();
    sqlx::query("INSERT INTO buildexecutors(daemonid,authorityid,buildcapacity) VALUES($1,$2,1)")
        .bind(&daemon)
        .bind(authority)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scriptexecutorreservations(attemptid,runid,daemonid,authorityid,admissiongeneration,agentid,agenttype,cleanupconfirmed) VALUES($1,$2,$3,$4,1,$5,1,true)")
        .bind(Uuid::now_v7()).bind(run).bind(&daemon).bind(authority).bind(Uuid::now_v7()).execute(pool).await.unwrap();
    assert_eq!(
        repo.blockers(operation).await.unwrap(),
        vec![UpdateDrainBlocker {
            kind: "Script executor reservations".into(),
            count: 1
        }],
        "terminal status and cleanup evidence cannot replace reservation release"
    );
    sqlx::query("DELETE FROM scriptexecutorreservations WHERE runid=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scriptruns WHERE id=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scripts WHERE id=$1")
        .bind(script)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildexecutors WHERE daemonid=$1")
        .bind(&daemon)
        .execute(pool)
        .await
        .unwrap();

    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE instanceupdates IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(3), repo.blockers(operation))
            .await
            .unwrap()
            .is_err(),
        "a blocked read must not look idle"
    );
    lock.rollback().await.unwrap();
    assert_eq!(repo.active().await.unwrap().as_ref(), Some(operation));
    assert!(repo.blockers(operation).await.unwrap().is_empty());
}
