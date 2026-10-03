use chrono::{Duration, Timelike, Utc};
use citadel_adapters::external::backups::restic::DockerResticBackupExecutor;
use citadel_adapters::persistence::postgres::backups::PostgresBackupPersistence;
use citadel_adapters::persistence::postgres::backups::secrets::PostgresBackupSecretResolver;
use citadel_adapters::persistence::postgres::backups::source_planner::PostgresBackupSourcePlanner;
use citadel_adapters::persistence::postgres::identity::authentication::store::PostgresIdentityStore;
use citadel_adapters::persistence::postgres::identity::authentication::store::StaticEntitlementService;
use citadel_adapters::security::identity::backup_authorization::IdentityBackupRunAuthorizer;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_adapters::security::identity::crypto::Argon2PasswordHasher;
use citadel_adapters::security::identity::crypto::JwtSessionTokenCodec;
use citadel_adapters::security::identity::crypto::OpaqueServiceAccountTokenCodec;
use citadel_backups::{
    BackupExecutionResult, BackupExecutor, BackupPersistence, BackupPolicyConfiguration,
    BackupRepositoryConfiguration, BackupRunAuthorizer, BackupRunItemResult, BackupSourceItem,
    BackupSourcePlan, BackupSourcePlanner,
};
use citadel_database::MigrationRunner;
use citadel_identity::{
    ADMIN_ROLE_ID, IdentityService, NoopServiceAccountLastUsedTracker, SYSTEM_ACTOR_ID, SystemClock,
};
use citadel_primitives::ActorId;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn backup_claims_are_repository_exclusive_and_late_results_do_not_overwrite_recovery() {
    let database_url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
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
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Online',0)")
        .bind(platform)
        .bind(format!("backup-platform-{}", platform.simple()))
        .execute(&pool)
        .await
        .unwrap();

    let store = PostgresBackupPersistence::new(pool.clone()).with_lease_options(60, 90, 30);
    let mut repository_input = BackupRepositoryConfiguration {
        name: format!("backup-repository-{}", Uuid::now_v7().simple()),
        description: None,
        spec: serde_json::from_value(json!({"$type":"FileSystem","location":"Core","platformId":null,"path":"/tmp/citadel-backups"})).unwrap(),
        password_secret_id: secret,
    };
    repository_input.validate().unwrap();
    let repository = store
        .create_repository(actor, &repository_input)
        .await
        .unwrap();
    let expired_operation = Uuid::now_v7();
    assert!(
        store
            .acquire_repository_operation(
                repository.id,
                expired_operation,
                "Validate",
                Utc::now() - Duration::seconds(1)
            )
            .await
            .unwrap()
    );
    let operation_id = Uuid::now_v7();
    assert!(
        store
            .acquire_repository_operation(
                repository.id,
                operation_id,
                "Validate",
                Utc::now() + Duration::minutes(5)
            )
            .await
            .unwrap()
    );
    assert!(matches!(
        store
            .record_repository_operation(
                repository.id,
                citadel_backups::BackupRepositoryOperation {
                    operation_id: expired_operation,
                    operation: "Validate",
                    location: "Core",
                    platform_id: None,
                    succeeded: false,
                    message: Some("stale result"),
                }
            )
            .await,
        Err(citadel_backups::BackupError::Conflict(_))
    ));
    let owner: Uuid = sqlx::query_scalar(
        "SELECT ownerrunid FROM backuprepositoryleases WHERE backuprepositoryid=$1",
    )
    .bind(repository.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(owner, operation_id);
    let (_, validation) = store
        .record_repository_operation(
            repository.id,
            citadel_backups::BackupRepositoryOperation {
                operation_id,
                operation: "Validate",
                location: "Core",
                platform_id: None,
                succeeded: true,
                message: None,
            },
        )
        .await
        .unwrap();
    store
        .release_repository_operation(repository.id, operation_id)
        .await
        .unwrap();
    assert_eq!(
        validation.status,
        citadel_backups::BackupRepositoryValidationStatus::Ready
    );
    let persisted_validation: (String, String) =
        sqlx::query_as("SELECT location,status FROM backuprepositoryvalidations WHERE id=$1")
            .bind(validation.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(persisted_validation, ("Core".into(), "Ready".into()));

    let mut policies = Vec::new();
    for suffix in ["one", "two"] {
        let mut input = BackupPolicyConfiguration {
            name: format!("backup-policy-{suffix}-{}", Uuid::now_v7().simple()),
            description: None,
            source: serde_json::from_value(json!({"$type":"DockerVolume","platformId":platform,"volumeName":format!("volume-{suffix}")})).unwrap(),
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
        let created = store.create_policy(actor, &input).await.unwrap();
        assert_eq!(created.tags.len(), 1);
        assert_eq!(created.tags[0].id, tag);
        assert!(created.latest_run.is_none());
        policies.push(created);
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
    for policy in &policies {
        let created: (String, Uuid, String) = sqlx::query_as(
            "SELECT status,createdbyactorid,info FROM activityevents WHERE resourceid=$1 AND eventtype='BackupPolicyCreated'",
        ).bind(policy.id).fetch_one(&pool).await.unwrap();
        assert_eq!(created.0, "Success");
        assert_eq!(created.1, actor.value());
        let info: serde_json::Value = serde_json::from_str(&created.2).unwrap();
        assert_eq!(info["Policy"]["Id"], policy.id.to_string());
        assert_eq!(info["Policy"]["Name"], policy.name);
    }
    let mut invalid_policy = BackupPolicyConfiguration {
        name: format!("invalid-tag-policy-{}", Uuid::now_v7().simple()),
        description: None,
        source: serde_json::from_value(
            json!({"$type":"DockerVolume","platformId":platform,"volumeName":"invalid"}),
        )
        .unwrap(),
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
    let rolled_back_activity: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM activityevents WHERE resourcename=$1)")
            .bind(&invalid_policy.name)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!rolled_back_activity);
    let first = store
        .enqueue_backup(actor, policies[0].id, "Manual")
        .await
        .unwrap();
    let _second = store
        .enqueue_backup(actor, policies[1].id, "Manual")
        .await
        .unwrap();
    assert_eq!(
        first.snapshot_availability,
        citadel_backups::BackupSnapshotAvailability::Pending
    );

    let (left, right) = tokio::join!(
        store.claim_backup(Utc::now() - Duration::minutes(5)),
        store.claim_backup(Utc::now() - Duration::minutes(5))
    );
    let claims = [left.unwrap(), right.unwrap()];
    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
    let first_claim = claims.into_iter().flatten().next().unwrap();
    assert_eq!(
        lease_durations(&pool, first_claim.run.id).await,
        (180.0, 210.0)
    );
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
    // Use the mutation boundary so the authorization cache observes the grant.
    use citadel_identity::UserRepository;
    citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository::new(pool.clone())
        .add_role(user, ADMIN_ROLE_ID, ActorId::new(SYSTEM_ACTOR_ID), Utc::now(), true)
        .await.unwrap();
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
    assert_eq!(
        prepared.items[0].status,
        citadel_backups::BackupRunItemStatus::Queued
    );
    // A new request owns the policy before attempting its repository. Completion
    // must wait at the policy without taking the repository in reverse order.
    let mut enqueue = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *enqueue)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM backuppolicies WHERE id=$1 FOR UPDATE")
        .bind(first_claim.policy.id)
        .fetch_one(&mut *enqueue)
        .await
        .unwrap();
    let result = success_result("a", Some(plan.items[0].id));
    let completion = store.finish_backup(&first_claim, &result);
    tokio::pin!(completion);
    tokio::select! {
        result = &mut completion => panic!("completion bypassed policy: {result:?}"),
        () = async {
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))").bind(pid).fetch_one(&pool).await.unwrap();
                    if waiting { break; }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }).await.unwrap();
        } => {}
    }
    sqlx::query("SELECT id FROM backuprepositories WHERE id=$1 FOR UPDATE NOWAIT")
        .bind(first_claim.repository.id)
        .fetch_one(&mut *enqueue)
        .await
        .unwrap();
    enqueue.commit().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), completion)
        .await
        .unwrap()
        .unwrap();
    let completed = store.get_run(first_claim.run.id).await.unwrap();
    assert_eq!(
        completed.items[0].status,
        citadel_backups::BackupRunItemStatus::Succeeded
    );
    store.finish_backup(&first_claim, &result).await.unwrap();
    assert_run_activities(&pool, &first_claim.run, "Succeeded", "Success", true).await;
    store
        .enqueue_restore(citadel_backups::BackupRestoreRequest {
            actor,
            backup_run_id: first_claim.run.id,
            target_platform_id: platform,
            target_volume_name: "configured-lease-restore".into(),
            overwrite_existing: false,
            target_docker_node_id: None,
            source_backup_run_item_id: Some(completed.items[0].id),
        })
        .await
        .unwrap();
    let restore = store
        .claim_restore(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lease_durations(&pool, restore.run.id).await, (120.0, 150.0));
    store
        .finish_restore(
            &restore,
            &citadel_backups::RestoreExecutionResult {
                status: citadel_backups::BackupRestoreStatus::Succeeded,
                exit_code: Some(0),
                error_code: None,
                error_message: None,
                logs: vec![],
            },
        )
        .await
        .unwrap();

    let late_claim = store
        .claim_backup(Utc::now() - Duration::minutes(5))
        .await
        .unwrap()
        .unwrap();
    // Port worker startup interruption: periodic maintenance must preserve a
    // fresh execution, but exclusive Core startup recovery interrupts it.
    citadel_adapters::persistence::postgres::maintenance::reconcile(&pool)
        .await
        .unwrap();
    assert_ne!(
        store.get_run(late_claim.run.id).await.unwrap().status,
        citadel_backups::BackupRunStatus::Interrupted
    );
    citadel_adapters::persistence::postgres::maintenance::recover_on_startup(&pool)
        .await
        .unwrap();
    citadel_adapters::persistence::postgres::maintenance::recover_on_startup(&pool)
        .await
        .unwrap();
    store
        .finish_backup(&late_claim, &success_result("late", None))
        .await
        .unwrap();
    assert_run_activities(&pool, &late_claim.run, "Interrupted", "Failure", true).await;

    // The Activities tab reads this authorized, resource-scoped query.
    use citadel_activities::{
        ActivityAccess, ActivityFilter, ActivityQueryStore, ActivityResourceType,
    };
    let activity_store =
        citadel_adapters::persistence::postgres::activities::store::PostgresActivityStore::new(
            pool.clone(),
        );
    let page = activity_store
        .list_authorized(
            &ActivityAccess {
                actor_id: actor,
                administrator: false,
            },
            ActivityFilter {
                resource_id: Some(first_claim.policy.id),
                resource_type: Some(ActivityResourceType::BackupPolicy),
                ..Default::default()
            }
            .validated()
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.total_count, 4);
    assert!(
        page.items
            .iter()
            .all(|activity| activity.resource_id == Some(first_claim.policy.id))
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
    let schedule: (chrono::DateTime<Utc>, String, Uuid) = sqlx::query_as("SELECT p.lastscheduledrunat,r.trigger,r.triggeredbyactorid FROM backuppolicies p JOIN backupruns r ON r.id=p.currentrunid WHERE p.id=$1")
        .bind(policies[0].id).fetch_one(&pool).await.unwrap();
    assert_eq!(schedule.0, scheduled_minute);
    assert_eq!(schedule.1, "Schedule");
    assert_eq!(schedule.2, policies[0].run_as_actor_id);
    let scheduled = store
        .list_runs(actor, true, Some(policies[0].id), 1)
        .await
        .unwrap()
        .remove(0);
    assert!(store.cancel_backup(scheduled.id).await.unwrap());
    assert!(!store.cancel_backup(scheduled.id).await.unwrap());
    assert_run_activities(&pool, &scheduled, "Cancelled", "Warning", false).await;
    sqlx::query("UPDATE backuppolicies SET controlstate='Idle',currentrunid=NULL WHERE id=$1")
        .bind(policies[0].id)
        .execute(&pool)
        .await
        .unwrap();
    let restarted = PostgresBackupPersistence::new(pool.clone());
    assert!(
        !restarted
            .enqueue_scheduled_backup(policies[0].id, scheduled_minute)
            .await
            .unwrap()
    );
    sqlx::query("UPDATE backuppolicies SET enabled=false WHERE id=$1")
        .bind(policies[0].id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !restarted
            .enqueue_scheduled_backup(policies[0].id, scheduled_minute + Duration::minutes(1))
            .await
            .unwrap()
    );
    let last: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT lastscheduledrunat FROM backuppolicies WHERE id=$1")
            .bind(policies[0].id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(last, scheduled_minute);
    sqlx::query("UPDATE backuppolicies SET enabled=true WHERE id=$1")
        .bind(policies[0].id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        restarted
            .enqueue_scheduled_backup(policies[0].id, scheduled_minute + Duration::minutes(1))
            .await
            .unwrap()
    );

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
    assert_eq!(rejected.status, citadel_backups::BackupRunStatus::Failed);
    assert!(
        rejected
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("Edge Agent is disconnected or unavailable"))
    );
    edge_failure_cleans_helper_on_the_exact_node(executor, &guarded_claim, platform).await;
    edge_restore_uses_the_saved_snapshot_root(&pool, &guarded_claim, platform).await;

    // Finish the queued scheduled run, then cover the remaining executor outcomes.
    for (outcome, activity_status) in [
        ("Failed", "Failure"),
        ("TimedOut", "Failure"),
        ("Rejected", "Failure"),
        ("SucceededWithWarnings", "Warning"),
    ] {
        let claim = store
            .claim_backup(Utc::now() - Duration::minutes(5))
            .await
            .unwrap()
            .unwrap();
        let mut result = success_result("outcome", None);
        result.status = outcome.parse().unwrap();
        result.snapshot_availability = citadel_backups::BackupSnapshotAvailability::Missing;
        result.error_message = Some("Test outcome".into());
        store.finish_backup(&claim, &result).await.unwrap();
        store.finish_backup(&claim, &result).await.unwrap();
        assert_run_activities(&pool, &claim.run, outcome, activity_status, true).await;
        if outcome != "SucceededWithWarnings" {
            store
                .enqueue_backup(actor, claim.policy.id, "Manual")
                .await
                .unwrap();
        }
    }

    let webhook = json!({"enabled": true, "provider":"Generic", "authScheme":"BearerToken", "secret":"fixture-key"});
    sqlx::query("UPDATE backuppolicies SET webhook=$2 WHERE id=$1")
        .bind(policies[0].id)
        .bind(&webhook)
        .execute(&pool)
        .await
        .unwrap();
    let webhook_run = store
        .enqueue_webhook(
            policies[0].id,
            &serde_json::from_value(webhook.clone()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(webhook_run.triggered_by_actor_id, SYSTEM_ACTOR_ID);
    assert_eq!(webhook_run.trigger, "Webhook");
    assert!(store.cancel_backup(webhook_run.id).await.unwrap());
    assert_run_activities(&pool, &webhook_run, "Cancelled", "Warning", false).await;

    sqlx::query("DELETE FROM activityevents WHERE resourceid=ANY($1)")
        .bind(policies.iter().map(|policy| policy.id).collect::<Vec<_>>())
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("DELETE FROM backuprunlogs WHERE backuprunid IN (SELECT id FROM backupruns WHERE backuprepositoryid=$1)").bind(repository.id).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM backuprepositoryleases WHERE backuprepositoryid=$1")
        .bind(repository.id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM backuprestoreruns WHERE id=$1")
        .bind(restore.run.id)
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

async fn edge_failure_cleans_helper_on_the_exact_node(
    executor: DockerResticBackupExecutor,
    claim: &citadel_backups::BackupClaim,
    platform: Uuid,
) {
    use citadel_adapters::connectors::edge::EdgeRegistry;
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        containers::v1::{CreateContainerRequest, CreateContainerResponse},
        edge::v1::{EdgeCommandKind, core_envelope},
        shared_models::v1::PlatformInfoResponse,
    };
    use prost::Message;
    let registry = EdgeRegistry::default();
    let (_wrong, mut wrong_outbound) = registry
        .register(
            EdgeTarget::node(platform, "node-one".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (selected, mut outbound) = registry
        .register(
            EdgeTarget::node(platform, "node-two".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let executor = executor.with_edge(registry);
    let plan = BackupSourcePlan {
        display_name: "fixture".into(),
        items: vec![BackupSourceItem::new(
            platform,
            "volume-one".into(),
            Some("node-two".into()),
            None,
        )],
        warnings: vec![],
        local_directory: None,
    };
    let cancellation = CancellationToken::new();
    let run = executor.backup(claim, &plan, &cancellation);
    let agent = async {
        for kind in [
            EdgeCommandKind::PlatformGetInfo,
            EdgeCommandKind::ContainerCreate,
            EdgeCommandKind::ContainerStart,
            EdgeCommandKind::ContainerDelete,
        ] {
            let message = tokio::time::timeout(std::time::Duration::from_secs(5), outbound.recv())
                .await
                .unwrap()
                .unwrap();
            let id = Uuid::parse_str(&message.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = message.body else {
                panic!("expected command")
            };
            assert_eq!(command.node_id, "node-two");
            assert_eq!(command.kind, kind as i32);
            if kind == EdgeCommandKind::PlatformGetInfo {
                selected.output(
                    id,
                    PlatformInfoResponse {
                        agent_runtime_image: "citadel-agent:installed-on-node-two".into(),
                        ..Default::default()
                    }
                    .encode_to_vec(),
                );
            }
            if kind == EdgeCommandKind::ContainerCreate {
                let request = CreateContainerRequest::decode(command.payload.as_slice()).unwrap();
                assert_eq!(request.image_id, "citadel-agent:installed-on-node-two");
                assert!(!request.cap_add.iter().any(|cap| cap == "CHOWN"));
                selected.output(
                    id,
                    CreateContainerResponse {
                        container_id: "helper-on-node-two".into(),
                    }
                    .encode_to_vec(),
                );
            }
            selected.complete(id, kind != EdgeCommandKind::ContainerStart);
        }
    };
    let (result, ()) = tokio::join!(run, agent);
    assert_eq!(result.status, citadel_backups::BackupRunStatus::Failed);
    assert_eq!(result.items.len(), 1);
    assert_eq!(
        result.items[0].status,
        citadel_backups::BackupRunItemStatus::Failed
    );
    assert!(wrong_outbound.try_recv().is_err());
    assert_eq!(selected.pending_count(), 0);
}

// Verify that restore resolves the saved snapshot root across connectors.
// The peer validates canonical Agent commands, not a real Restic installation.
async fn edge_restore_uses_the_saved_snapshot_root(
    pool: &sqlx::PgPool,
    backup: &citadel_backups::BackupClaim,
    platform: Uuid,
) {
    use citadel_adapters::connectors::edge::EdgeRegistry;
    use citadel_adapters::connectors::edge::EdgeTarget;
    use citadel_backups::{BackupRestoreRun, BackupSecretResolver, RestoreClaim};
    use citadel_contracts::citadel::{
        containers::v1::{
            CreateContainerRequest, CreateContainerResponse, ExecBinaryRequest, ExecExit,
            ExecOutput, ExecServerMessage, exec_server_message,
        },
        edge::v1::{EdgeCommandKind as Kind, core_envelope},
        shared_models::v1::PlatformInfoResponse,
        volumes::v1::ListVolumesResponse,
    };
    use futures_util::future::BoxFuture;
    use prost::Message;
    use zeroize::Zeroizing;
    struct Password;
    impl BackupSecretResolver for Password {
        fn resolve(
            &self,
            _: Uuid,
        ) -> BoxFuture<'_, Result<Zeroizing<String>, citadel_backups::BackupError>> {
            Box::pin(async { Ok(Zeroizing::new("fixture-password".into())) })
        }
    }
    for root in ["/data", "/source", "/unexpected"] {
        let registry = EdgeRegistry::default();
        let (_wrong, mut other) = registry
            .register(
                EdgeTarget::node(platform, "source-node".into()),
                Uuid::now_v7(),
            )
            .unwrap();
        let (session, mut receiver) = registry
            .register(
                EdgeTarget::node(platform, "restore-node".into()),
                Uuid::now_v7(),
            )
            .unwrap();
        let executor = DockerResticBackupExecutor::new(
            "docker-must-not-run",
            "restic:test",
            Arc::new(Password),
            4096,
            pool.clone(),
        )
        .with_edge(registry);
        let snapshot = "a".repeat(64);
        let mut repository = backup.repository.clone();
        repository.repository_type = "S3Compatible".into();
        repository.spec = serde_json::from_value(json!({"$type":"S3Compatible", "endpoint":"http://s3.test", "bucket":"backups",
            "accessKeySecretId":Uuid::now_v7(), "secretKeySecretId":Uuid::now_v7(), "allowInsecureHttp":true})).unwrap();
        let mut source = backup.run.clone();
        source.restic_snapshot_id = Some(snapshot.clone());
        let claim = RestoreClaim {
            repository,
            source,
            source_item: None,
            run: BackupRestoreRun {
                id: Uuid::now_v7(),
                backup_run_id: backup.run.id,
                backup_repository_id: backup.repository.id,
                source_backup_run_item_id: None,
                target_platform_id: platform,
                target_docker_node_id: Some("restore-node".into()),
                target_volume_name: "target".into(),
                overwrite_existing: false,
                status: citadel_backups::BackupRestoreStatus::Running,
                queued_at: Utc::now(),
                started_at: Some(Utc::now()),
                completed_at: None,
                exit_code: None,
                error_code: None,
                error_message: None,
                triggered_by_actor_id: SYSTEM_ACTOR_ID,
            },
        };
        let cancellation = CancellationToken::new();
        let peer = async {
            let mut kinds = vec![
                Kind::VolumeList,
                Kind::PlatformGetInfo,
                Kind::ContainerCreate,
                Kind::ContainerStart,
                Kind::ContainerExecBinary,
            ];
            if root != "/unexpected" {
                kinds.push(Kind::ContainerExecBinary);
            }
            kinds.push(Kind::ContainerDelete);
            let mut metadata = true;
            for kind in kinds {
                let envelope =
                    tokio::time::timeout(std::time::Duration::from_secs(5), receiver.recv())
                        .await
                        .unwrap()
                        .unwrap();
                let id = Uuid::parse_str(&envelope.command_id).unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("expected command");
                };
                assert_eq!(command.kind, kind as i32);
                assert_eq!(command.node_id, "restore-node");
                match kind {
                    Kind::PlatformGetInfo => {
                        session.output(
                            id,
                            PlatformInfoResponse {
                                agent_runtime_image: if root == "/data" {
                                    String::new()
                                } else {
                                    "citadel-agent:installed-on-restore-node".into()
                                },
                                ..Default::default()
                            }
                            .encode_to_vec(),
                        );
                    }
                    Kind::VolumeList => {
                        session.output(id, ListVolumesResponse::default().encode_to_vec());
                    }
                    Kind::ContainerCreate => {
                        let request =
                            CreateContainerRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(
                            request.image_id,
                            if root == "/data" {
                                "restic:test"
                            } else {
                                "citadel-agent:installed-on-restore-node"
                            }
                        );
                        assert_eq!(request.mounts[0].source.as_deref(), Some("target"));
                        assert_eq!(request.mounts[0].target.as_deref(), Some("/target"));
                        assert!(request.cap_add.iter().any(|cap| cap == "CHOWN"));
                        session.output(
                            id,
                            CreateContainerResponse {
                                container_id: "restore-helper".into(),
                            }
                            .encode_to_vec(),
                        );
                    }
                    Kind::ContainerExecBinary => {
                        let request =
                            ExecBinaryRequest::decode(command.payload.as_slice()).unwrap();
                        assert_eq!(request.container_id, "restore-helper");
                        assert_eq!(request.env["RESTIC_PASSWORD"], "fixture-password");
                        if metadata {
                            assert_eq!(request.cmd, ["restic", "snapshots", "--json", &snapshot]);
                            session.output(
                                id,
                                ExecServerMessage {
                                    msg: Some(exec_server_message::Msg::Output(ExecOutput {
                                        data: serde_json::to_vec(
                                            &json!([{"id":snapshot, "paths":[root]}]),
                                        )
                                        .unwrap(),
                                        stream: 0,
                                    })),
                                }
                                .encode_to_vec(),
                            );
                        } else {
                            assert_eq!(
                                request.cmd,
                                [
                                    "restic",
                                    "restore",
                                    &format!("{snapshot}:{root}"),
                                    "--target",
                                    "/target",
                                    "--delete"
                                ]
                            );
                        }
                        metadata = false;
                        session.output(
                            id,
                            ExecServerMessage {
                                msg: Some(exec_server_message::Msg::Exit(ExecExit {
                                    exit_code: 0,
                                })),
                            }
                            .encode_to_vec(),
                        );
                    }
                    _ => {}
                }
                session.complete(id, true);
            }
        };
        let (result, ()) = tokio::join!(executor.restore(&claim, &cancellation), peer);
        assert_eq!(
            result.status,
            if root == "/unexpected" {
                citadel_backups::BackupRestoreStatus::Failed
            } else {
                citadel_backups::BackupRestoreStatus::Succeeded
            }
        );
        assert!(
            other.try_recv().is_err(),
            "the source node must not receive restore commands"
        );
        assert_eq!(session.pending_count(), 0);
    }
}

fn success_result(marker: &str, item_id: Option<Uuid>) -> BackupExecutionResult {
    let snapshot = marker.repeat(64);
    BackupExecutionResult {
        status: citadel_backups::BackupRunStatus::Succeeded,
        snapshot_availability: citadel_backups::BackupSnapshotAvailability::Available,
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
                status: citadel_backups::BackupRunItemStatus::Succeeded,
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

async fn lease_durations(pool: &sqlx::PgPool, owner: Uuid) -> (f64, f64) {
    sqlx::query_as("SELECT (SELECT EXTRACT(EPOCH FROM expiresat-createdat)::float8 FROM backuprepositoryleases WHERE ownerrunid=$1), (SELECT EXTRACT(EPOCH FROM expiresat-createdat)::float8 FROM backupsourceleases WHERE ownerrunid=$1)")
        .bind(owner).fetch_one(pool).await.unwrap()
}

async fn assert_run_activities(
    pool: &sqlx::PgPool,
    run: &citadel_backups::BackupRun,
    outcome: &str,
    activity_status: &str,
    started: bool,
) {
    let events: Vec<(String, String, Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT eventtype,status,resourceid,createdbyactorid,info FROM activityevents WHERE info::jsonb->>'RunId'=$1 ORDER BY createdat,id",
    ).bind(run.id.to_string()).fetch_all(pool).await.unwrap();
    let mut expected = vec![("BackupRunQueued", "Information")];
    if started {
        expected.push(("BackupRunStarted", "Information"));
    }
    expected.push(("BackupRunCompleted", activity_status));
    assert_eq!(
        events
            .iter()
            .map(|event| (event.0.as_str(), event.1.as_str()))
            .collect::<Vec<_>>(),
        expected
    );
    for event in &events {
        assert_eq!(event.2, run.backup_policy_id);
        assert_eq!(event.3, run.triggered_by_actor_id);
        let info: serde_json::Value = serde_json::from_str(&event.4).unwrap();
        assert_eq!(info["$type"], event.0);
        assert_eq!(info["Trigger"], run.trigger);
    }
    let info: serde_json::Value = serde_json::from_str(&events.last().unwrap().4).unwrap();
    assert_eq!(info["Status"], outcome);
    assert_eq!(info["DurationMs"].is_number(), started);
}
