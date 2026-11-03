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
    for _ in 0..33 {
        let id = Uuid::now_v7();
        ids.push(id);
        sqlx::query("INSERT INTO mfasetupsessions(id,createdat,expiresat,protectedtotpsecret,userid) VALUES($1,now(),now()+interval '1 hour',$2,$3)")
            .bind(id).bind(&envelope).bind(user).execute(pool).await.unwrap();
    }
    // Exercise each credential store with the real schema and native SQL rows.
    let credential = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,'UPDATE_SECRET','Internal')",
    )
    .bind(credential)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO internalsecretvalues(secretid,encryptedvalue) VALUES($1,$2)")
        .bind(credential)
        .bind(&envelope)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO secretproviders(id,name,providertype,configuration) VALUES($1,'update-provider','Vault',$2)")
        .bind(credential).bind(serde_json::json!({"ProtectedToken":envelope}).to_string()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO gitaccounts(id,name,domain,transport,authtype,createdbyactorid,configuration) VALUES($1,'update-git','example.test','Https','Token',$2,$3)")
        .bind(credential).bind(citadel_identity::SYSTEM_ACTOR_ID).bind(serde_json::json!({"$protected":envelope})).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO oidcproviders(id,name,displayname,issuer,clientid,scopes,createdbyactorid,clientsecretciphertext) VALUES($1,'update-oidc','Update OIDC','https://example.test','fixture','openid',$2,$3)")
        .bind(credential).bind(citadel_identity::SYSTEM_ACTOR_ID).bind(&envelope).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO usermfasettings(userid,createdat,enabledat,protectedtotpsecret) VALUES($1,now(),now(),$2)")
        .bind(user).bind(&envelope).execute(pool).await.unwrap();
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
    // Reset the malformed final page, then distinguish nullable OIDC credentials
    // from required provider tokens. Neither check changes persisted data.
    sqlx::query("UPDATE mfasetupsessions SET protectedtotpsecret=$1 WHERE id=ANY($2)")
        .bind(&envelope)
        .bind(&ids)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE oidcproviders SET clientsecretciphertext=NULL WHERE id=$1")
        .bind(credential)
        .execute(pool)
        .await
        .unwrap();
    check_target_update_database(pool, journal, None, &key)
        .await
        .unwrap();
    sqlx::query("UPDATE secretproviders SET configuration='{}' WHERE id=$1")
        .bind(credential)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        check_target_update_database(pool, journal, None, &key)
            .await
            .is_err()
    );
    for query in [
        "DELETE FROM internalsecretvalues WHERE secretid=$1",
        "DELETE FROM secretdefinitions WHERE id=$1",
        "DELETE FROM secretproviders WHERE id=$1",
        "DELETE FROM gitaccounts WHERE id=$1",
        "DELETE FROM oidcproviders WHERE id=$1",
    ] {
        sqlx::query(query)
            .bind(credential)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM usermfasettings WHERE userid=$1")
        .bind(user)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM mfasetupsessions WHERE id=ANY($1)")
        .bind(&ids)
        .execute(pool)
        .await
        .unwrap();
}
