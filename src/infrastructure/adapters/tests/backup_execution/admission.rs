use super::*;
use citadel_adapters::persistence::postgres::backups::data_admission::PostgresBackupDataAdmission;
use citadel_backups::{BackupDataAdmission, BackupDataOperation, BackupExecutionHost};

pub(super) async fn verify(
    pool: &sqlx::PgPool,
    actor: ActorId,
    platform: Uuid,
    input: &BackupRepositoryConfiguration,
) {
    let store = PostgresBackupPersistence::new(pool.clone());
    let admission = PostgresBackupDataAdmission::new(pool.clone());
    let mut operations = Vec::new();
    for i in 0..7 {
        let mut input = input.clone();
        input.name = format!("capacity-fixture-{i}-{}", Uuid::now_v7());
        let repository = store.create_repository(actor, &input).await.unwrap();
        let owner = Uuid::now_v7();
        assert!(
            store
                .acquire_repository_operation(
                    repository.id,
                    owner,
                    "Check",
                    Utc::now() + Duration::minutes(5)
                )
                .await
                .unwrap()
        );
        operations.push(BackupDataOperation {
            id: Uuid::now_v7(),
            repository_id: repository.id,
            owner_id: owner,
            attempt_id: owner,
            platform_id: platform,
            node_id: None,
        });
    }
    let hosts: Vec<_> = (0..3)
        .map(|i| BackupExecutionHost {
            daemon_id: format!("data-fixture-{i}-{}", Uuid::now_v7()),
            authority_id: Uuid::now_v7(),
            generation: 1,
            data_slots: 4,
            heavy_slots: 2,
        })
        .collect();
    // Three independent physical hosts admit six operations. The combined
    // budget remains stricter than the per-data budget.
    for (i, op) in operations[..6].iter().enumerate() {
        assert!(admission.reserve(op, &hosts[i / 2]).await.unwrap());
    }
    assert!(!admission.reserve(&operations[6], &hosts[0]).await.unwrap());
    // A second connection/authority to the same daemon cannot multiply slots.
    let mut alias = hosts[0].clone();
    alias.authority_id = Uuid::now_v7();
    alias.heavy_slots = 8;
    assert!(!admission.reserve(&operations[6], &alias).await.unwrap());
    let mut stale = operations[0].clone();
    stale.attempt_id = Uuid::now_v7();
    admission.release(&stale, &hosts[0]).await.unwrap();
    assert!(!admission.reserve(&operations[6], &hosts[0]).await.unwrap());
    // Transaction-start time must not revive a lease that expires while renewal
    // is waiting for its row lock.
    sqlx::query("UPDATE backuprepositoryleases SET expiresat=clock_timestamp()+INTERVAL '200 milliseconds' WHERE ownerrunid=$1")
        .bind(operations[1].owner_id).execute(pool).await.unwrap();
    let mut held = pool.begin().await.unwrap();
    sqlx::query("SELECT ownerrunid FROM backuprepositoryleases WHERE ownerrunid=$1 FOR UPDATE")
        .bind(operations[1].owner_id)
        .fetch_one(&mut *held)
        .await
        .unwrap();
    let renewal = store.renew_leases(
        operations[1].repository_id,
        operations[1].owner_id,
        operations[1].attempt_id,
    );
    tokio::pin!(renewal);
    tokio::select! {
        result = &mut renewal => panic!("renewal bypassed its ownership lock: {result:?}"),
        () = tokio::time::sleep(std::time::Duration::from_millis(300)) => {}
    }
    held.commit().await.unwrap();
    assert!(
        !renewal.await.unwrap(),
        "an expired owner cannot revive its lease"
    );
    // TTL expiration never releases host or business ownership.
    sqlx::query("UPDATE backuprepositoryleases SET expiresat=CURRENT_TIMESTAMP-INTERVAL '1 second' WHERE ownerrunid=$1").bind(operations[0].owner_id).execute(pool).await.unwrap();
    citadel_adapters::persistence::postgres::maintenance::expire_leases(pool)
        .await
        .unwrap();
    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM backuprepositoryleases WHERE ownerrunid=$1")
            .bind(operations[0].owner_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        retained, 1,
        "periodic expiry cannot erase unresolved backup ownership"
    );
    assert!(
        !store
            .renew_leases(
                operations[0].repository_id,
                operations[0].owner_id,
                operations[0].attempt_id
            )
            .await
            .unwrap()
    );
    assert!(!admission.reserve(&operations[6], &hosts[0]).await.unwrap());
    assert!(
        admission
            .pending_cleanup(None)
            .await
            .unwrap()
            .iter()
            .any(|(op, _)| op.id == operations[0].id)
    );
    admission.finish_cleanup(&stale, &hosts[0]).await.unwrap();
    assert!(!admission.reserve(&operations[6], &hosts[0]).await.unwrap());
    admission
        .finish_cleanup(&operations[0], &hosts[0])
        .await
        .unwrap();
    assert!(admission.reserve(&operations[6], &hosts[0]).await.unwrap());
    for (i, op) in operations.iter().enumerate() {
        admission
            .release(op, &hosts[if i == 6 { 0 } else { i / 2 }])
            .await
            .unwrap();
        store
            .release_repository_operation(op.repository_id, op.owner_id)
            .await
            .unwrap();
        sqlx::query("DELETE FROM backuprepositories WHERE id=$1")
            .bind(op.repository_id)
            .execute(pool)
            .await
            .unwrap();
    }
    for host in hosts {
        sqlx::query("DELETE FROM buildexecutors WHERE daemonid=$1")
            .bind(host.daemon_id)
            .execute(pool)
            .await
            .unwrap();
    }
}

pub(super) async fn verify_source_deferral(
    pool: &sqlx::PgPool,
    actor: ActorId,
    platform: Uuid,
    input: &BackupRepositoryConfiguration,
) {
    let store = PostgresBackupPersistence::new(pool.clone());
    let mut input = input.clone();
    input.name = format!("source-admission-{}", Uuid::now_v7());
    let repository = store.create_repository(actor, &input).await.unwrap();
    let policy = store
        .create_policy(
            actor,
            &BackupPolicyConfiguration {
                name: format!("source-admission-{}", Uuid::now_v7()),
                description: None,
                source: serde_json::from_value(
                    json!({"$type":"DockerVolume","platformId":platform,"volumeName":"parent"}),
                )
                .unwrap(),
                backup_repository_id: repository.id,
                enabled: true,
                cron: None,
                time_zone: None,
                webhook: None,
                keep_last_successful: Some(2),
                timeout_seconds: Some(60),
                alert_on_failure: false,
                run_as_actor_id: None,
                tag_ids: vec![],
            },
        )
        .await
        .unwrap();
    store
        .enqueue_backup(actor, policy.id, "Manual", None)
        .await
        .unwrap();
    let claim = store
        .claim_backup(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    let plan = BackupSourcePlan {
        display_name: "immutable group".into(),
        warnings: vec![],
        local_directory: None,
        items: vec![
            BackupSourceItem::new(platform, "a-free".into(), None, None),
            BackupSourceItem::new(platform, "z-busy".into(), None, None),
        ],
    };
    let foreign = Uuid::now_v7();
    sqlx::query("INSERT INTO backupsourceleases(sourcekey,ownerrunid,attemptid,operationtype,createdat,expiresat) VALUES($1,$2,$2,'Backup',now(),now()+INTERVAL '5 minutes')")
        .bind(plan.items[1].source_key()).bind(foreign).execute(pool).await.unwrap();
    assert!(matches!(
        store.prepare_backup_items(&claim, &plan).await,
        Err(citadel_backups::BackupError::Conflict(_))
    ));
    let partial: i64 =
        sqlx::query_scalar("SELECT count(*) FROM backupsourceleases WHERE sourcekey=$1")
            .bind(plan.items[0].source_key())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        partial, 0,
        "multi-source acquisition rolls back the complete set"
    );
    assert!(store.defer_backup(&claim).await.unwrap());
    assert!(
        store
            .claim_backup(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .is_none(),
        "backoff prevents a hot loop"
    );
    sqlx::query("DELETE FROM backupsourceleases WHERE ownerrunid=$1")
        .bind(foreign)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE backupruns SET nextattemptat=NULL WHERE id=$1")
        .bind(claim.run.id)
        .execute(pool)
        .await
        .unwrap();
    let next = store
        .claim_backup(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(next.run.id, claim.run.id);
    assert_eq!(next.run.queued_at, claim.run.queued_at);
    assert_ne!(next.attempt_id, claim.attempt_id);
    assert_eq!(
        serde_json::to_value(next.source_plan.as_ref().unwrap()).unwrap(),
        serde_json::to_value(&plan).unwrap()
    );
    let mut changed = plan.clone();
    changed.items.pop();
    assert!(
        store.prepare_backup_items(&next, &changed).await.is_err(),
        "retry cannot silently rediscover a different source set"
    );
    store.prepare_backup_items(&next, &plan).await.unwrap();
    let admission = PostgresBackupDataAdmission::new(pool.clone());
    let host = BackupExecutionHost {
        daemon_id: format!("source-test-{}", Uuid::now_v7()),
        authority_id: Uuid::now_v7(),
        generation: 1,
        data_slots: 2,
        heavy_slots: 2,
    };
    let operation = BackupDataOperation {
        id: Uuid::now_v7(),
        repository_id: repository.id,
        owner_id: next.run.id,
        attempt_id: next.attempt_id,
        platform_id: platform,
        node_id: None,
    };
    assert!(admission.reserve(&operation, &host).await.unwrap());
    assert!(
        !store.defer_backup(&next).await.unwrap(),
        "a dispatched or uncertain reservation cannot return to the queue"
    );
    admission.release(&operation, &host).await.unwrap();
    assert!(store.defer_backup(&next).await.unwrap());
    assert!(
        !store
            .renew_leases(repository.id, claim.run.id, claim.attempt_id)
            .await
            .unwrap()
    );
    store.cancel_backup(next.run.id).await.unwrap();
    sqlx::query("DELETE FROM activityevents WHERE resourceid=$1")
        .bind(policy.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backupruns WHERE backuppolicyid=$1")
        .bind(policy.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuppolicies WHERE id=$1")
        .bind(policy.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuprepositories WHERE id=$1")
        .bind(repository.id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM buildexecutors WHERE daemonid=$1")
        .bind(host.daemon_id)
        .execute(pool)
        .await
        .unwrap();
}
