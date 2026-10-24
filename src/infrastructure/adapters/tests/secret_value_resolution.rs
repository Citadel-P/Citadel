use citadel_adapters::persistence::postgres::bindings::PostgresBindingRepository;
use citadel_bindings::{
    BindingError, BindingRepository, SecretProviderTester, StoredSecretProviderPatch,
    TestSecretProviderInput,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

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
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn internal_and_vault_kv_v2_secrets_resolve_without_disclosing_stored_tokens() {
    let database_url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let trap_requests = Arc::new(AtomicUsize::new(0));
    let received = trap_requests.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new()
                .route("/trap/v1/sys/health", get(|| async { StatusCode::OK }))
                .route(
                    "/trap/v1/auth/token/lookup-self",
                    get(move |headers: HeaderMap| {
                        let received = received.clone();
                        async move {
                            received.fetch_add(1, Ordering::SeqCst);
                            assert_eq!(
                                headers.get("x-vault-token").unwrap(),
                                "explicit-test-token"
                            );
                            StatusCode::OK
                        }
                    }),
                )
                .route(
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

    for id in [internal_id, external_id] {
        sqlx::query(
            "INSERT INTO secretusegrants(secretid,scope,resourceid) VALUES($1,'Global',$2)",
        )
        .bind(id)
        .bind(Uuid::nil())
        .execute(&pool)
        .await
        .unwrap();
    }
    let resolver = PostgresSecretValueResolver::new(pool.clone(), protector.clone()).unwrap();
    assert_eq!(
        resolver
            .resolve(internal_id, "Global", Uuid::nil())
            .await
            .unwrap()
            .as_str(),
        "internal-value"
    );
    assert_eq!(
        resolver
            .resolve(external_id, "Global", Uuid::nil())
            .await
            .unwrap()
            .as_str(),
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

    let mut test = TestSecretProviderInput {
        provider_id: Some(provider_id),
        name: None,
        address: format!("http://{address}/trap"),
        mount_path: "kv".into(),
        token: None,
    };
    assert!(matches!(
        resolver.test_connection(&test).await,
        Err(BindingError::Validation(_))
    ));
    assert_eq!(trap_requests.load(Ordering::SeqCst), 0);
    test.token = Some("explicit-test-token".into());
    assert!(resolver.test_connection(&test).await.unwrap().success);
    assert_eq!(trap_requests.load(Ordering::SeqCst), 1);

    // The transaction boundary enforces this too, independently of service validation.
    let repository = PostgresBindingRepository::new(pool.clone());
    let patch = StoredSecretProviderPatch {
        name: None,
        address: Some(format!("http://{address}/trap")),
        mount_path: None,
        protected_token: None,
    };
    assert!(matches!(
        repository.update_secret_provider(provider_id, &patch).await,
        Err(BindingError::Validation(_))
    ));
    assert_eq!(
        repository
            .get_secret_provider(provider_id)
            .await
            .unwrap()
            .address,
        format!("http://{address}")
    );

    let patch = StoredSecretProviderPatch {
        protected_token: Some(protector.protect(b"explicit-test-token").unwrap()),
        ..patch
    };
    repository
        .update_secret_provider(provider_id, &patch)
        .await
        .unwrap();
    test.token = None;
    assert!(resolver.test_connection(&test).await.unwrap().success);
    assert_eq!(trap_requests.load(Ordering::SeqCst), 2);

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
