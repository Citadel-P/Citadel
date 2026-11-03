use super::{tests::Fixture, *};
use crate::{
    external::backups::system_recovery::CitadelSystemRestoreOptions,
    persistence::postgres::{job_lease::CoreJobLease, updates::check_startup_updates},
    security::identity::crypto::AesGcmSecretProtector,
};
use citadel_updates::*;
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires two fresh disposable CITADEL_CHECKPOINT_DATABASE_URL/CITADEL_CHECKPOINT_TARGET_DATABASE_URL and PostgreSQL 18 tools"]
async fn restores_checkpoint_and_releases_only_the_verified_fresh_database() {
    let source_url = std::env::var("CITADEL_CHECKPOINT_DATABASE_URL").unwrap();
    let target_url = std::env::var("CITADEL_CHECKPOINT_TARGET_DATABASE_URL").unwrap();
    for url in [&source_url, &target_url] {
        assert!(
            url::Url::parse(url)
                .unwrap()
                .path()
                .starts_with("/citadel_update_test_")
        );
    }
    assert_ne!(source_url, target_url);
    let source = PgPoolOptions::new()
        .max_connections(3)
        .connect(&source_url)
        .await
        .unwrap();
    let target = PgPoolOptions::new()
        .max_connections(3)
        .connect(&target_url)
        .await
        .unwrap();
    for pool in [&source, &target] {
        crate::persistence::postgres::updates::require_empty_restore_database(pool)
            .await
            .unwrap();
    }
    citadel_database::MigrationRunner::migrate(&source_url)
        .await
        .unwrap();
    let instance = Uuid::now_v7();
    let mut f = Fixture::new(instance);
    use base64::Engine;
    std::fs::write(
        f.root.path().join("keys/secret-encryption-key"),
        base64::engine::general_purpose::STANDARD.encode([7; 32]),
    )
    .unwrap();
    f.operation.plan.source.catalog_sha256 =
        citadel_database::MigrationRunner::catalog_sha256().unwrap();
    f.operation.plan.target.catalog_sha256 = f.operation.plan.source.catalog_sha256.clone();
    let mut journal = f.state.journal(f.operation.id).unwrap();
    for entry in &mut journal.history {
        entry.plan = f.operation.plan.clone();
    }
    journal.operation = f.operation.clone();
    f.state.save(&journal).unwrap();
    let draining = journal.history[1].clone();
    let actor = draining.actor_id.value();
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(actor)
        .execute(&source)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdat,createdbyactorid,email,name,password) VALUES($1,$2,now(),$3,'restore@example.test','restore',NULL)")
        .bind(user).bind(actor).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&source).await.unwrap();
    sqlx::query("INSERT INTO citadelinstanceidentity(id,instanceid,createdat) VALUES(1,$1,now())")
        .bind(instance)
        .execute(&source)
        .await
        .unwrap();
    sqlx::query("UPDATE instancesetupstates SET initializedat=now(),initialadministratoractorid=$1 WHERE id=1").bind(actor).execute(&source).await.unwrap();
    sqlx::query("INSERT INTO instanceupdates(id,instanceid,actorid,idempotencykey,revision,operation) VALUES($1,$2,$3,$4,1,$5)")
        .bind(draining.id).bind(instance).bind(actor).bind(draining.idempotency_key).bind(sqlx::types::Json(&draining)).execute(&source).await.unwrap();
    let envelope = AesGcmSecretProtector::new(&[7; 32])
        .unwrap()
        .protect(b"retained credential")
        .unwrap();
    sqlx::query("INSERT INTO mfasetupsessions(id,createdat,expiresat,protectedtotpsecret,userid) VALUES($1,now(),now()+interval '1 hour',$2,$3)")
        .bind(Uuid::now_v7()).bind(envelope).bind(user).execute(&source).await.unwrap();
    let token = CancellationToken::new();
    // Post-restore validation and the outcome write are atomic. Exercise the
    // same restored source rows without allowing these rejected attempts to
    // release ownership or create a misleading Activity.
    let lease = CoreJobLease::acquire(&source_url).await.unwrap();
    let finish = crate::persistence::postgres::updates::finish_checkpoint_restore;
    assert!(
        finish(&source, &f.operation, &[8; 32], &token)
            .await
            .is_err()
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(
        finish(&source, &f.operation, &[7; 32], &cancelled)
            .await
            .is_err()
    );
    sqlx::raw_sql("CREATE FUNCTION reject_recovery_outcome() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture outcome failure'; END $$; CREATE TRIGGER reject_recovery_outcome BEFORE INSERT ON activityevents FOR EACH ROW EXECUTE FUNCTION reject_recovery_outcome();")
        .execute(&source).await.unwrap();
    assert!(
        finish(&source, &f.operation, &[7; 32], &token)
            .await
            .is_err()
    );
    sqlx::raw_sql("DROP TRIGGER reject_recovery_outcome ON activityevents; DROP FUNCTION reject_recovery_outcome();").execute(&source).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, sqlx::types::Json<UpdateOperation>>(
            "SELECT operation FROM instanceupdates WHERE ownsinstallation"
        )
        .fetch_one(&source)
        .await
        .unwrap()
        .0,
        draining
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM activityevents WHERE eventtype='InstanceUpdateFailed'"
        )
        .fetch_one(&source)
        .await
        .unwrap(),
        0
    );
    drop(lease);
    let builder = f.builder(
        source.clone(),
        &source_url,
        std::env::var_os("CITADEL_PG_DUMP").unwrap(),
    );
    let receipt = prepare_update_checkpoint(&f.state, &f.owner, &f.operation, &builder, &token)
        .await
        .unwrap();
    let options = CitadelSystemRestoreOptions {
        bundle: receipt.bundle.clone(),
        database_url: target_url.clone(),
        pg_restore: std::env::var_os("CITADEL_PG_RESTORE").unwrap(),
        maximum_output: 4096,
        secret_encryption_key: Some(zeroize::Zeroizing::new(vec![7; 32])),
    };
    // A live Core's advisory lease and a wrong source build both refuse before writes.
    let lease = CoreJobLease::acquire(&target_url).await.unwrap();
    assert!(
        restore_update_checkpoint(&f.stage(), &options, "0.1.1", &token)
            .await
            .is_err()
    );
    drop(lease);
    assert!(
        restore_update_checkpoint(&f.stage(), &options, "0.1.2", &token)
            .await
            .is_err()
    );
    crate::persistence::postgres::updates::require_empty_restore_database(&target)
        .await
        .unwrap();
    let mut wrong_key = options.clone();
    wrong_key.secret_encryption_key = Some(zeroize::Zeroizing::new(vec![8; 32]));
    assert!(
        restore_update_checkpoint(&f.stage(), &wrong_key, "0.1.1", &token)
            .await
            .is_err()
    );
    crate::persistence::postgres::updates::require_empty_restore_database(&target)
        .await
        .unwrap();
    let restored = restore_update_checkpoint(&f.stage(), &options, "0.1.1", &token)
        .await
        .unwrap();
    assert_eq!(restored.phase, UpdatePhase::Failed);
    assert!(
        restored
            .message
            .unwrap()
            .contains("restored into a new database")
    );
    check_startup_updates(&target, Some(instance))
        .await
        .unwrap();
    assert!(
        check_startup_updates(&source, Some(instance))
            .await
            .is_err()
    );
    assert!(
        restore_update_checkpoint(&f.stage(), &options, "0.1.1", &token)
            .await
            .is_err(),
        "never overwrite the recovered database"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM activityevents WHERE eventtype='InstanceUpdateFailed'"
        )
        .fetch_one(&target)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        f.state.journal(f.operation.id).unwrap(),
        journal,
        "original host evidence is retained"
    );
    assert_eq!(
        sqlx::query_scalar::<_, sqlx::types::Json<UpdateOperation>>(
            "SELECT operation FROM instanceupdates WHERE id=$1"
        )
        .bind(draining.id)
        .fetch_one(&source)
        .await
        .unwrap()
        .0,
        draining
    );
    source.close().await;
    target.close().await;
}
