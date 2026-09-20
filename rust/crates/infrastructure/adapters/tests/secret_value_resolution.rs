use std::sync::Arc;

use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use citadel_adapters::persistence::postgres::bindings::secret_resolver::PostgresSecretValueResolver;
use citadel_adapters::security::identity::crypto::AesGcmSecretProtector;
use citadel_database::MigrationRunner;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn internal_and_vault_kv_v2_secrets_resolve_without_disclosing_stored_tokens() {
    let database_url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route(
                "/v1/kv/data/apps/citadel",
                get(|headers: HeaderMap| async move {
                    if headers.get("x-vault-token").and_then(|v| v.to_str().ok())
                        != Some("vault-token")
                    {
                        return (StatusCode::UNAUTHORIZED, Json(json!({"errors":[]})));
                    }
                    (
                        StatusCode::OK,
                        Json(json!({"data":{"data":{"TOKEN":"external-value"}}})),
                    )
                }),
            ),
        )
        .await
        .unwrap();
    });

    let protector = Arc::new(AesGcmSecretProtector::new(&[97_u8; 32]).unwrap());
    let internal_id = Uuid::now_v7();
    let external_id = Uuid::now_v7();
    let provider_id = Uuid::now_v7();
    let internal_envelope = protector.protect(b"internal-value").unwrap();
    let token_envelope = protector.protect(b"vault-token").unwrap();
    sqlx::query("INSERT INTO secretproviders(id,name,providertype,configuration) VALUES($1,$2,'VaultCompatibleKvV2',$3)")
        .bind(provider_id)
        .bind(format!("vault-{}", provider_id.simple()))
        .bind(json!({"Address":format!("http://{address}"),"MountPath":"kv","ProtectedToken":token_envelope}))
        .execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(internal_id)
    .bind(format!("INTERNAL_{}", internal_id.simple()))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO internalsecretvalues(secretid,encryptedvalue) VALUES($1,$2)")
        .bind(internal_id)
        .bind(&internal_envelope)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype,providerid,externalpath,externalkey,externalversion) VALUES($1,$2,'VaultCompatibleKvV2',$3,'apps/citadel','TOKEN',1)")
        .bind(external_id).bind(format!("EXTERNAL_{}", external_id.simple())).bind(provider_id).execute(&pool).await.unwrap();

    let resolver = PostgresSecretValueResolver::new(pool.clone(), protector).unwrap();
    assert_eq!(
        resolver.resolve(internal_id).await.unwrap().as_str(),
        "internal-value"
    );
    assert_eq!(
        resolver.resolve(external_id).await.unwrap().as_str(),
        "external-value"
    );
    let stored: serde_json::Value = serde_json::from_str(
        &sqlx::query_scalar::<_, String>("SELECT configuration FROM secretproviders WHERE id=$1")
            .bind(provider_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_ne!(stored["ProtectedToken"], "vault-token");

    sqlx::query("DELETE FROM secretdefinitions WHERE id=ANY($1)")
        .bind([internal_id, external_id])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM secretproviders WHERE id=$1")
        .bind(provider_id)
        .execute(&pool)
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
}
