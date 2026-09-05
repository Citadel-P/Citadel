use chrono::{Duration, Timelike, Utc};
use citadel_adapters::backup_authorization::IdentityBackupRunAuthorizer;
use citadel_adapters::backup_executor::{DockerResticBackupExecutor, PostgresBackupSecretResolver};
use citadel_adapters::backup_source_planner::PostgresBackupSourcePlanner;
use citadel_adapters::backup_store::PostgresBackupStore;
use citadel_adapters::crypto::{
    AesGcmSecretProtector, Argon2PasswordHasher, JwtSessionTokenCodec,
    OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_backups::{
    BackupExecutionResult, BackupExecutor, BackupPolicyInput, BackupRepositoryInput,
    BackupRunAuthorizer, BackupRunItemResult, BackupSourceItem, BackupSourcePlan,
    BackupSourcePlanner, BackupStore,
};
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_identity::{
    ADMIN_ROLE_ID, IdentityService, NoopServiceAccountLastUsedTracker, SYSTEM_ACTOR_ID, SystemClock,
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn backup_claims_are_repository_exclusive_and_late_results_do_not_overwrite_recovery() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&database_url)
        .await
        .unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    let secret = Uuid::now_v7();
    let platform = Uuid::now_v7();
    let tag = Uuid::now_v7();
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,email,name) VALUES($1,$2,$3,$4,$5)")
        .bind(user)
        .bind(actor.value())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("{user}@example.test"))
        .bind(format!("backup-user-{}", user.simple()))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,lower($2),'#445566',$3)")
        .bind(tag)
        .bind(format!("backup-tag-{}", tag.simple()))
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(secret)
    .bind(format!("backup-secret-{}", secret.simple()))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost','Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
        .bind(platform)
        .bind(format!("backup-platform-{}", platform.simple()))
        .execute(&pool)
        .await
        .unwrap();

    let store = PostgresBackupStore::new(pool.clone());
    let mut repository_input = BackupRepositoryInput {
        name: format!("backup-repository-{}", Uuid::now_v7().simple()),
        description: None,
        spec: json!({"$type":"FileSystem","location":"Core","platformId":null,"path":"/tmp/citadel-backups"}),
        password_secret_id: secret,
    };
    repository_input.validate().unwrap();
    let repository = store
        .create_repository(actor, &repository_input)
        .await
        .unwrap();
    let (_, validation) = store
        .record_repository_operation(repository.id, "Validate", "Core", None, true, None)
        .await
        .unwrap();
    assert_eq!(validation.status, "Ready");
    let persisted_validation: (String, String) =
        sqlx::query_as("SELECT location,status FROM backuprepositoryvalidations WHERE id=$1")
            .bind(validation.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(persisted_validation, ("Core".into(), "Ready".into()));

    let mut policies = Vec::new();
    for suffix in ["one", "two"] {
        let mut input = BackupPolicyInput {
            name: format!("backup-policy-{suffix}-{}", Uuid::now_v7().simple()),
            description: None,
            source: json!({"$type":"DockerVolume","platformId":platform,"volumeName":format!("volume-{suffix}")}),
            backup_repository_id: repository.id,
            enabled: true,
            cron: None,
            time_zone: None,
            webhook: None,
            keep_last_successful: Some(2),
            timeout_seconds: Some(60),
            alert_on_failure: true,
            run_as_actor_id: None,
            tag_ids: vec![tag],
        };
        input.validate(actor).unwrap();
        policies.push(store.create_policy(actor, &input).await.unwrap());
    }
    let persisted_tags: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resourcetags WHERE resourcetype='BackupPolicy' AND resourceid=ANY($1) AND tagid=$2",
    )
    .bind(policies.iter().map(|policy| policy.id).collect::<Vec<_>>())
    .bind(tag)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(persisted_tags, 2);
    let mut invalid_policy = BackupPolicyInput {
        name: format!("invalid-tag-policy-{}", Uuid::now_v7().simple()),
        description: None,
        source: json!({"$type":"DockerVolume","platformId":platform,"volumeName":"invalid"}),
        backup_repository_id: repository.id,
        enabled: true,
        cron: None,
        time_zone: None,
        webhook: None,
        keep_last_successful: Some(2),
        timeout_seconds: Some(60),
        alert_on_failure: true,
        run_as_actor_id: None,
        tag_ids: vec![Uuid::now_v7()],
    };
    invalid_policy.validate(actor).unwrap();
    assert!(matches!(
        store.create_policy(actor, &invalid_policy).await,
        Err(citadel_backups::BackupError::Validation(_))
    ));
    let rolled_back: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuppolicies WHERE normalizedname=$1)")
            .bind(invalid_policy.name.to_uppercase())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!rolled_back);
    let first = store
        .enqueue_backup(actor, policies[0].id, "Manual")
        .await
        .unwrap();
    let _second = store
        .enqueue_backup(actor, policies[1].id, "Manual")
        .await
        .unwrap();
    assert_eq!(first.snapshot_availability, "Pending");

    let (left, right) = tokio::join!(
        store.claim_backup(Utc::now() - Duration::minutes(5)),
        store.claim_backup(Utc::now() - Duration::minutes(5))
    );
    let claims = [left.unwrap(), right.unwrap()];
    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
    let first_claim = claims.into_iter().flatten().next().unwrap();
    let guarded_claim = first_claim.clone();
    let identity = Arc::new(IdentityService::new(
        Arc::new(PostgresIdentityStore::new(pool.clone())),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[71_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let authorizer = IdentityBackupRunAuthorizer::new(identity);
    assert!(authorizer.authorize_backup(&guarded_claim).await.is_err());
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor.value())
        .bind(ADMIN_ROLE_ID)
        .execute(&pool)
        .await
        .unwrap();
    authorizer.authorize_backup(&guarded_claim).await.unwrap();
    let planner = PostgresBackupSourcePlanner::new(pool.clone());
    let plan = planner
        .plan(&first_claim, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(plan.items.len(), 1);
    store
        .prepare_backup_items(&first_claim, &plan)
        .await
        .unwrap();
    let prepared = store.get_run(first_claim.run.id).await.unwrap();
    assert_eq!(prepared.items.len(), 1);
    assert_eq!(prepared.items[0].status, "Queued");
    store
        .finish_backup(&first_claim, &success_result("a", Some(plan.items[0].id)))
        .await
        .unwrap();
    let completed = store.get_run(first_claim.run.id).await.unwrap();
    assert_eq!(completed.items[0].status, "Succeeded");

    let late_claim = store
        .claim_backup(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    sqlx::query(
        "UPDATE backupruns SET status='Interrupted',completedat=CURRENT_TIMESTAMP WHERE id=$1",
    )
    .bind(late_claim.run.id)
    .execute(&pool)
    .await
    .unwrap();
    store
        .finish_backup(&late_claim, &success_result("b", None))
        .await
        .unwrap();
    assert_eq!(
        store.get_run(late_claim.run.id).await.unwrap().status,
        "Interrupted"
    );
    assert!(
        store
            .backup_logs(late_claim.run.id)
            .await
            .unwrap()
            .is_empty()
    );

    sqlx::query("UPDATE backuppolicies SET cron='* * * * *',timezone='UTC',controlstate='Idle',currentrunid=NULL WHERE id=$1")
        .bind(policies[0].id)
        .execute(&pool)
        .await
        .unwrap();
    let scheduled_minute = Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let (left, right) = tokio::join!(
        store.enqueue_scheduled_backup(policies[0].id, scheduled_minute),
        store.enqueue_scheduled_backup(policies[0].id, scheduled_minute)
    );
    assert_eq!(usize::from(left.unwrap()) + usize::from(right.unwrap()), 1);
    let scheduled_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM backupruns WHERE backuppolicyid=$1 AND trigger='Schedule'",
    )
    .bind(policies[0].id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(scheduled_count, 1);

    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let executor = DockerResticBackupExecutor::new(
        "docker-command-must-not-run",
        "restic/restic:test",
        Arc::new(
            PostgresBackupSecretResolver::new(
                pool.clone(),
                Arc::new(AesGcmSecretProtector::new(&[92_u8; 32]).unwrap()),
            )
            .unwrap(),
        ),
        4096,
        pool.clone(),
    );
    let rejected = executor
        .backup(
            &guarded_claim,
            &BackupSourcePlan {
                display_name: "fixture".into(),
                items: vec![BackupSourceItem::new(
                    platform,
                    "volume-one".into(),
                    None,
                    None,
                )],
                warnings: vec![],
                local_directory: None,
            },
            &CancellationToken::new(),
        )
        .await;
    assert_eq!(rejected.status, "Failed");
    assert!(
        rejected
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("Edge Agent backup execution"))
    );

    sqlx::query("DELETE FROM backuprunlogs WHERE backuprunid IN (SELECT id FROM backupruns WHERE backuprepositoryid=$1)").bind(repository.id).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM backuprepositoryleases WHERE backuprepositoryid=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backupruns WHERE backuprepositoryid=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM resourcetags WHERE resourcetype='BackupPolicy' AND resourceid=ANY($1)",
    )
    .bind(policies.iter().map(|policy| policy.id).collect::<Vec<_>>())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM backuppolicies WHERE backuprepositoryid=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tags WHERE id=$1")
        .bind(tag)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuprepositories WHERE id=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
        .bind(secret)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actorroles WHERE actorid=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
}

fn success_result(marker: &str, item_id: Option<Uuid>) -> BackupExecutionResult {
    let snapshot = marker.repeat(64);
    BackupExecutionResult {
        status: "Succeeded",
        snapshot_availability: "Available",
        restic_snapshot_id: Some(snapshot.clone()),
        parent_snapshot_id: None,
        files_processed: Some(1),
        bytes_processed: Some(1),
        bytes_added: Some(1),
        exit_code: Some(0),
        error_code: None,
        error_message: None,
        warnings: vec![],
        logs: vec![],
        items: item_id
            .map(|id| BackupRunItemResult {
                id,
                status: "Succeeded",
                restic_snapshot_id: Some(snapshot),
                parent_snapshot_id: None,
                files_processed: Some(1),
                bytes_processed: Some(1),
                bytes_added: Some(1),
                exit_code: Some(0),
                error_code: None,
                error_message: None,
            })
            .into_iter()
            .collect(),
    }
}
