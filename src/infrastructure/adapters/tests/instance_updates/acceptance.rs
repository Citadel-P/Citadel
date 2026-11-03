use citadel_adapters::{
    persistence::postgres::updates::check_target_update_database,
    security::identity::crypto::AesGcmSecretProtector,
};
use citadel_updates::UpdateJournal;
use sqlx::PgPool;
use uuid::Uuid;

/// This is current-schema acceptance coverage, not published-version qualification.
pub(super) async fn verify_target_acceptance(pool: &PgPool, journal: &UpdateJournal, user: Uuid) {
    let key = [7; 32];
    let protector = AesGcmSecretProtector::new(&key).unwrap();
    let envelope = protector.protect(b"fixture-mfa-material").unwrap();
    // Multiple pages prove the last page is checked, not just the first row.
    let mut ids = Vec::new();
    for _ in 0..5 {
        let id = Uuid::now_v7();
        ids.push(id);
        sqlx::query("INSERT INTO mfasetupsessions(id,createdat,expiresat,protectedtotpsecret,userid) VALUES($1,now(),now()+interval '1 hour',$2,$3)")
            .bind(id).bind(&envelope).bind(user).execute(pool).await.unwrap();
    }
    check_target_update_database(
        pool,
        journal,
        Some(citadel_database::MIGRATION_MANIFEST_JSON),
        &key,
    )
    .await
    .unwrap();
    check_target_update_database(pool, journal, None, &key)
        .await
        .unwrap();
    let error = check_target_update_database(pool, journal, None, &[9; 32])
        .await
        .unwrap_err();
    assert!(!error.to_string().contains(&envelope));
    assert!(
        check_target_update_database(pool, journal, Some("{}"), &key)
            .await
            .is_err()
    );
    sqlx::query("UPDATE mfasetupsessions SET protectedtotpsecret='malformed' WHERE id=$1")
        .bind(ids.last().unwrap())
        .execute(pool)
        .await
        .unwrap();
    assert!(
        check_target_update_database(pool, journal, None, &key)
            .await
            .is_err()
    );
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT ownsinstallation FROM instanceupdates WHERE id=$1")
            .bind(journal.operation.id)
            .fetch_one(pool)
            .await
            .unwrap()
    );
    let stored: String =
        sqlx::query_scalar("SELECT protectedtotpsecret FROM mfasetupsessions WHERE id=$1")
            .bind(ids.last().unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        stored, "malformed",
        "readiness must preserve failure evidence"
    );
    sqlx::query("DELETE FROM mfasetupsessions WHERE id=ANY($1)")
        .bind(&ids)
        .execute(pool)
        .await
        .unwrap();
}
