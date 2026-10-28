use super::*;
use crate::persistence::postgres::backups::PostgresBackupPersistence;
use citadel_primitives::ActorId;
struct Helpers(Vec<String>);
impl Drop for Helpers {
    fn drop(&mut self) {
        for name in &self.0 {
            let _ = std::process::Command::new("docker")
                .args(["rm", "--force", name])
                .output();
        }
    }
}
struct UnusedSecrets;
impl BackupSecretResolver for UnusedSecrets {
    fn resolve(
        &self,
        _: Uuid,
        _: Uuid,
    ) -> BoxFuture<'_, Result<zeroize::Zeroizing<String>, BackupError>> {
        Box::pin(async { panic!("fixture has no credentials") })
    }
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL and Docker with postgres:17-alpine"]
async fn local_restart_reconciles_exact_helpers_and_fences_late_starts() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let secret = Uuid::now_v7();
    let platform = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(secret)
    .bind(format!("helper-{secret}"))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'http://localhost/' || $1::text,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).bind(format!("helper-{platform}")).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,'Global','00000000-0000-0000-0000-000000000000')").bind(secret).execute(&pool).await.unwrap();
    let store = PostgresBackupPersistence::new(pool.clone());
    let repository=store.create_repository(ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),&BackupRepositoryConfiguration{name:format!("helper-{}",Uuid::now_v7()),spec:serde_json::from_value(serde_json::json!({"$type":"FileSystem","location":"Core","path":"/tmp/citadel-review-unused"})).unwrap(),password_secret_id:secret}).await.unwrap();
    let old = DockerResticBackupExecutor::new(
        "docker",
        "postgres:17-alpine",
        Arc::new(UnusedSecrets),
        16384,
        pool.clone(),
    );
    let new = DockerResticBackupExecutor::new(
        "docker",
        "postgres:17-alpine",
        Arc::new(UnusedSecrets),
        16384,
        pool.clone(),
    );
    let cancel = CancellationToken::new();
    let staging = std::env::temp_dir().join(format!("citadel-review-staging-{}", Uuid::now_v7()));
    std::fs::create_dir(&staging).unwrap();
    let other_staging = staging.join(Uuid::now_v7().simple().to_string());
    std::fs::create_dir(&other_staging).unwrap();
    std::fs::write(other_staging.join("owned"), "another operation").unwrap();
    std::fs::File::open(&other_staging)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - Duration::from_secs(48 * 3600))
        .unwrap();
    let builder = crate::external::backups::system_recovery::PgDumpSystemBackupBuilder::new(
        pool.clone(),
        url.clone(),
        "missing-citadel-test-pg-dump",
        staging.clone(),
        16384,
    );
    let failed_bundle = Uuid::now_v7();
    let error = builder.build(failed_bundle, &cancel).await.unwrap_err();
    assert!(!matches!(error, BackupError::CleanupPending));
    assert!(!staging.join(failed_bundle.simple().to_string()).exists());
    assert!(
        other_staging.join("owned").exists(),
        "age must not authorize staging deletion"
    );
    let existing_owner = Uuid::now_v7();
    std::fs::create_dir(staging.join(existing_owner.simple().to_string())).unwrap();
    assert!(matches!(
        builder.build(existing_owner, &cancel).await,
        Err(BackupError::CleanupPending)
    ));
    std::fs::remove_dir_all(staging).unwrap();
    new.reconcile_cleanup(None, &cancel).await.unwrap();
    let mut helpers = Helpers(Vec::new());
    let unrelated = format!("citadel-review-unrelated-{}", Uuid::now_v7());
    helpers.0.push(unrelated.clone());
    let made = std::process::Command::new("docker")
        .args([
            "create",
            "--name",
            &unrelated,
            "--label",
            "com.citadel.system-role=backup-helper",
            "--entrypoint",
            "/bin/sh",
            "postgres:17-alpine",
            "-c",
            "sleep 60",
        ])
        .output()
        .unwrap();
    assert!(made.status.success());
    for scenario in ["active", "removed", "lost-create-response"] {
        let owner = Uuid::now_v7();
        assert!(
            store
                .acquire_repository_operation(
                    repository.id,
                    owner,
                    "Check",
                    chrono::Utc::now() + chrono::Duration::minutes(5)
                )
                .await
                .unwrap()
        );
        let operation = ResticOperation::new(
            &old,
            OperationOwner {
                repository: repository.id,
                run: owner,
                attempt: owner,
            },
            false,
            None,
        );
        let scope = operation
            .admit_local(platform, None, &cancel)
            .await
            .unwrap();
        let name = format!("citadel-review-helper-{owner}");
        helpers.0.push(name.clone());
        let output = operation
            .execute(
                vec![
                    "create".into(),
                    "--name".into(),
                    name.clone().into(),
                    "--label".into(),
                    format!("citadel.backup.restoreHelperId={owner}").into(),
                    "--label".into(),
                    "com.citadel.system-role=backup-helper".into(),
                    "--entrypoint".into(),
                    "/bin/sh".into(),
                    "postgres:17-alpine".into(),
                    "-c".into(),
                    "sleep 60".into(),
                ],
                vec![],
                Duration::from_secs(10),
                &cancel,
            )
            .await
            .unwrap();
        let id = restore_helper_identity(&output).unwrap();
        operation
            .execute(
                vec!["start".into(), id.clone().into()],
                vec![],
                Duration::from_secs(10),
                &cancel,
            )
            .await
            .unwrap();
        if scenario == "removed" {
            operation.cleanup(&id).await.unwrap();
        } else if scenario == "lost-create-response" {
            let (record, _) = operation.local_scope().unwrap().unwrap();
            // Inject the persistence boundary after Docker create but before its
            // response is recorded, then recover through the production path.
            sqlx::query("UPDATE backupdatahelpers SET containerid=NULL WHERE operationid=$1")
                .bind(record.id)
                .execute(&pool)
                .await
                .unwrap();
            operation
                .cleanup_ambiguous_restore_helper(&name, owner)
                .await
                .unwrap();
            let remembered: String = sqlx::query_scalar(
                "SELECT containerid FROM backupdatahelpers WHERE operationid=$1",
            )
            .bind(record.id)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(remembered, id);
        }
        let (record, mut wrong_host) = operation.local_scope().unwrap().unwrap();
        wrong_host.daemon_id = "changed-daemon".into();
        assert!(
            operation
                .reconcile_local_helpers(&record, &wrong_host, None)
                .await
                .is_err()
        );
        // Docker unavailable cannot fabricate a cleanup receipt either.
        let unavailable = DockerResticBackupExecutor::new(
            "missing-citadel-test-docker",
            "postgres:17-alpine",
            Arc::new(UnusedSecrets),
            16384,
            pool.clone(),
        );
        let context = ResticOperation::new(
            &unavailable,
            OperationOwner {
                repository: repository.id,
                run: owner,
                attempt: owner,
            },
            false,
            None,
        );
        assert!(
            context
                .reconcile_local_helpers(&record, &wrong_host, None)
                .await
                .is_err()
        );
        drop(scope); // lose process-owned capacity before the durable receipt
        sqlx::query("UPDATE backuprepositoryleases SET expiresat=clock_timestamp()-INTERVAL '1 second' WHERE ownerrunid=$1").bind(owner).execute(&pool).await.unwrap();
        new.reconcile_cleanup(None, &cancel).await.unwrap();
        let retained: i64 =
            sqlx::query_scalar("SELECT count(*) FROM backuprepositoryleases WHERE ownerrunid=$1")
                .bind(owner)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(retained, 0);
        assert!(
            operation
                .execute(
                    vec!["start".into(), id.clone().into()],
                    vec![],
                    Duration::from_secs(5),
                    &cancel
                )
                .await
                .is_err()
        );
        assert!(
            operation
                .execute(
                    vec![
                        "create".into(),
                        "--name".into(),
                        name.into(),
                        "postgres:17-alpine".into()
                    ],
                    vec![],
                    Duration::from_secs(5),
                    &cancel
                )
                .await
                .is_err()
        );
        assert!(
            !std::process::Command::new("docker")
                .args(["inspect", &id])
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    // A missing host reservation does not clear unconfirmed Core staging work.
    let owner = Uuid::now_v7();
    assert!(
        store
            .acquire_repository_operation(
                repository.id,
                owner,
                "Check",
                chrono::Utc::now() + chrono::Duration::minutes(5)
            )
            .await
            .unwrap()
    );
    old.admission
        .begin_core_work(repository.id, owner, owner)
        .await
        .unwrap();
    sqlx::query("UPDATE backuprepositoryleases SET expiresat=clock_timestamp()-INTERVAL '1 second' WHERE ownerrunid=$1")
        .bind(owner).execute(&pool).await.unwrap();
    new.reconcile_cleanup(None, &cancel).await.unwrap();
    let retained: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuprepositoryleases WHERE ownerrunid=$1 AND recoveryfenced AND coreworkactive)")
        .bind(owner).fetch_one(&pool).await.unwrap();
    assert!(retained);
    assert!(
        old.admission
            .begin_core_work(repository.id, owner, owner)
            .await
            .is_err()
    );
    old.admission
        .finish_core_work(repository.id, owner, owner)
        .await
        .unwrap();
    new.reconcile_cleanup(None, &cancel).await.unwrap();
    let retained: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM backuprepositoryleases WHERE ownerrunid=$1)",
    )
    .bind(owner)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!retained);
    assert!(
        std::process::Command::new("docker")
            .args(["inspect", &unrelated])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        std::process::Command::new("docker")
            .args(["rm", "--force", &unrelated])
            .output()
            .unwrap()
            .status
            .success()
    );
}
