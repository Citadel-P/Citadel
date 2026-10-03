use std::path::PathBuf;

use citadel_adapters::external::backups::system_recovery::CitadelSystemRestoreOptions;
use citadel_adapters::external::backups::system_recovery::PgDumpSystemBackupBuilder;
use citadel_adapters::external::backups::system_recovery::restore_citadel_system;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_backups::CitadelSystemBackupBuilder;
use citadel_database::MigrationRunner;
use citadel_identity::SYSTEM_ACTOR_ID;
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires dedicated CITADEL_RECOVERY_SOURCE_DATABASE_URL and CITADEL_RECOVERY_TARGET_DATABASE_URL databases plus pg_dump and pg_restore"]
async fn system_bundle_restores_state_into_a_clean_database() {
    // ControlPlaneRecoveryTests.Candidate_ShouldRestoreControlPlaneAndOperateInCleanEnvironment:
    // database/instance/encrypted-state assertions. Packaged process + HTTP work
    // remain a separate acceptance boundary, not implied by this adapter test.
    let source_url = std::env::var("CITADEL_RECOVERY_SOURCE_DATABASE_URL").unwrap();
    let target_url = std::env::var("CITADEL_RECOVERY_TARGET_DATABASE_URL").unwrap();
    assert_ne!(
        database_identity(&source_url),
        database_identity(&target_url),
        "recovery source and target must be different databases"
    );
    MigrationRunner::migrate(&source_url).await.unwrap();
    MigrationRunner::migrate(&target_url).await.unwrap();

    let source = PgPoolOptions::new()
        .max_connections(2)
        .connect(&source_url)
        .await
        .unwrap();
    let marker_id = Uuid::now_v7();
    let marker_name = format!("execution-recovery-{}", marker_id.simple());
    sqlx::query(
        "INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,lower($2),'#334455',$3)",
    )
    .bind(marker_id)
    .bind(&marker_name)
    .bind(SYSTEM_ACTOR_ID)
    .execute(&source)
    .await
    .unwrap();

    let secret_id = Uuid::now_v7();
    let protector = AesGcmSecretProtector::new(&[7; 32]).unwrap();
    let protected = protector.protect(b"recovery-secret-value").unwrap();
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,'RECOVERY_SECRET','Internal')")
        .bind(secret_id).execute(&source).await.unwrap();
    sqlx::query("INSERT INTO internalsecretvalues(secretid,encryptedvalue) VALUES($1,$2)")
        .bind(secret_id)
        .bind(&protected)
        .execute(&source)
        .await
        .unwrap();

    let staging = std::env::temp_dir().join(format!(
        "citadel-execution-recovery-{}",
        Uuid::now_v7().simple()
    ));
    let cancellation = CancellationToken::new();
    let builder = PgDumpSystemBackupBuilder::new(
        source.clone(),
        source_url.clone(),
        std::env::var_os("CITADEL_PG_DUMP").unwrap_or_else(|| "pg_dump".into()),
        staging.clone(),
        256 * 1024,
    );
    let bundle = builder.build(Uuid::now_v7(), &cancellation).await.unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(bundle.join("manifest.json")).await.unwrap())
            .unwrap();
    let instance = Uuid::parse_str(manifest["instanceId"].as_str().unwrap()).unwrap();
    assert!(
        !tokio::fs::read_to_string(bundle.join("manifest.json"))
            .await
            .unwrap()
            .contains("recovery-secret-value")
    );
    source.close().await;

    let target = PgPoolOptions::new()
        .max_connections(1)
        .connect(&target_url)
        .await
        .unwrap();
    target.close().await;
    // ControlPlaneRecoveryTests.Restore_ShouldRejectDatabaseArchiveWithoutSecretEncryptionKey:
    // This fixture supplies key material through external configuration.
    // Reject before pg_restore can alter the target, even with a valid bundle.
    let sentinel = PgPoolOptions::new()
        .max_connections(1)
        .connect(&target_url)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE recovery_untouched(marker text); INSERT INTO recovery_untouched VALUES('untouched')")
        .execute(&sentinel).await.unwrap();
    for key in [None, Some(zeroize::Zeroizing::new(vec![7; 31]))] {
        let error = restore_citadel_system(
            &CitadelSystemRestoreOptions {
                bundle: bundle.clone(),
                database_url: target_url.clone(),
                pg_restore: "pg_restore".into(),
                maximum_output: 256 * 1024,
                secret_encryption_key: key,
            },
            &cancellation,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Secrets__EncryptionKey"));
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT marker FROM recovery_untouched")
                .fetch_one(&sentinel)
                .await
                .unwrap(),
            "untouched"
        );
    }
    sqlx::query("DROP TABLE recovery_untouched")
        .execute(&sentinel)
        .await
        .unwrap();
    sentinel.close().await;
    restore_citadel_system(
        &CitadelSystemRestoreOptions {
            bundle: bundle.clone(),
            database_url: target_url.clone(),
            pg_restore: std::env::var_os("CITADEL_PG_RESTORE")
                .unwrap_or_else(|| "pg_restore".into()),
            maximum_output: 256 * 1024,
            secret_encryption_key: Some(zeroize::Zeroizing::new(vec![7; 32])),
        },
        &cancellation,
    )
    .await
    .unwrap();

    let restored = PgPoolOptions::new()
        .max_connections(1)
        .connect(&target_url)
        .await
        .unwrap();
    let restored_name: String = sqlx::query_scalar("SELECT name FROM tags WHERE id=$1")
        .bind(marker_id)
        .fetch_one(&restored)
        .await
        .unwrap();
    assert_eq!(restored_name, marker_name);
    let restored_instance: Option<Uuid> =
        sqlx::query_scalar("SELECT instanceid FROM citadelinstanceidentity WHERE id=1")
            .fetch_optional(&restored)
            .await
            .unwrap();
    assert_eq!(
        restored_instance,
        Some(instance),
        "the manifest identity must be included in the dump, including the first backup"
    );
    let encrypted: String =
        sqlx::query_scalar("SELECT encryptedvalue FROM internalsecretvalues WHERE secretid=$1")
            .bind(secret_id)
            .fetch_one(&restored)
            .await
            .unwrap();
    assert_eq!(encrypted, protected);
    // Construct a fresh protector, as after a restart with the retained external key.
    let restored_protector = AesGcmSecretProtector::new(&[7; 32]).unwrap();
    assert_eq!(
        restored_protector.unprotect(&encrypted).unwrap().as_slice(),
        b"recovery-secret-value"
    );
    assert!(
        AesGcmSecretProtector::new(&[8; 32])
            .unwrap()
            .unprotect(&encrypted)
            .is_err()
    );
    restored.close().await;

    cleanup_marker(&source_url, marker_id).await;
    cleanup_marker(&target_url, marker_id).await;
    for url in [&source_url, &target_url] {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await
            .unwrap();
        sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
            .bind(secret_id)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }
    remove_directory(bundle).await;
    remove_directory(staging).await;
}

fn database_identity(database_url: &str) -> (String, u16, String) {
    let url = url::Url::parse(database_url).unwrap();
    (
        url.host_str().unwrap_or_default().to_ascii_lowercase(),
        url.port().unwrap_or(5432),
        url.path().trim_matches('/').to_owned(),
    )
}

async fn cleanup_marker(database_url: &str, marker_id: Uuid) {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tags WHERE id=$1")
        .bind(marker_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

async fn remove_directory(path: PathBuf) {
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(path).await.unwrap();
    }
}
