use std::path::PathBuf;

use citadel_adapters::citadel_system_backup::{
    CitadelSystemRestoreOptions, PostgresCitadelSystemBackupBuilder, restore_citadel_system,
};
use citadel_backups::CitadelSystemBackupBuilder;
use citadel_database::MigrationRunner;
use citadel_identity::SYSTEM_ACTOR_ID;
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires dedicated CITADEL_PHASE7_RECOVERY_SOURCE_DATABASE_URL and CITADEL_PHASE7_RECOVERY_TARGET_DATABASE_URL databases plus pg_dump and pg_restore"]
async fn system_bundle_restores_state_into_a_clean_database() {
    let source_url = std::env::var("CITADEL_PHASE7_RECOVERY_SOURCE_DATABASE_URL").unwrap();
    let target_url = std::env::var("CITADEL_PHASE7_RECOVERY_TARGET_DATABASE_URL").unwrap();
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
    let marker_name = format!("phase7-recovery-{}", marker_id.simple());
    sqlx::query(
        "INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,lower($2),'#334455',$3)",
    )
    .bind(marker_id)
    .bind(&marker_name)
    .bind(SYSTEM_ACTOR_ID)
    .execute(&source)
    .await
    .unwrap();

    let staging = std::env::temp_dir().join(format!(
        "citadel-phase7-recovery-{}",
        Uuid::now_v7().simple()
    ));
    let cancellation = CancellationToken::new();
    let builder = PostgresCitadelSystemBackupBuilder::new(
        source.clone(),
        source_url.clone(),
        std::env::var_os("CITADEL_PG_DUMP").unwrap_or_else(|| "pg_dump".into()),
        staging.clone(),
        256 * 1024,
    );
    let bundle = builder.build(Uuid::now_v7(), &cancellation).await.unwrap();
    source.close().await;

    let target = PgPoolOptions::new()
        .max_connections(1)
        .connect(&target_url)
        .await
        .unwrap();
    target.close().await;
    restore_citadel_system(
        &CitadelSystemRestoreOptions {
            bundle: bundle.clone(),
            database_url: target_url.clone(),
            pg_restore: std::env::var_os("CITADEL_PG_RESTORE")
                .unwrap_or_else(|| "pg_restore".into()),
            maximum_output: 256 * 1024,
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
    restored.close().await;

    cleanup_marker(&source_url, marker_id).await;
    cleanup_marker(&target_url, marker_id).await;
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
