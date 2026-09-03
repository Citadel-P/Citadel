use std::sync::Arc;

use citadel_adapters::crypto::AesGcmSecretProtector;
use citadel_adapters::resource_metadata_store::PostgresResourceMetadataStore;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_resources::{
    BindingValidation, CatalogMutationKind, ExternalSecretInput, ExternalSecretPatch, NewRegistry,
    NewResourceBinding, NewTag, RegistryPatch, RegistryStatus, ResourceBindingKind,
    ResourceBindingScope, ResourceMetadataError, ResourceMetadataService, ResourceMetadataStore,
    SecretDeliveryMode, SecretProviderInput, SecretProviderPatch, validate_binding,
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE5_DATABASE_URL"]
async fn metadata_mutations_are_atomic_and_credentials_are_protected() {
    let database_url = std::env::var("CITADEL_PHASE5_DATABASE_URL")
        .expect("CITADEL_PHASE5_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let store = Arc::new(PostgresResourceMetadataStore::new(pool.clone()));
    let actor = ActorId::new(SYSTEM_ACTOR_ID);

    let suffix = Uuid::now_v7().simple().to_string();
    let tag = store
        .create_tag(
            actor,
            &NewTag {
                name: format!("phase5-{suffix}"),
                color: "#112233".into(),
            },
        )
        .await
        .unwrap();
    let registry = store
        .create_registry(
            actor,
            &NewRegistry {
                name: format!("registry-{suffix}"),
                registry_host: "registry.example.test".into(),
                status: RegistryStatus::Active,
                configuration: json!({"$type":"Custom","Username":"user","Password":"secret"}),
                description: None,
                tag_ids: vec![tag.id],
            },
        )
        .await
        .unwrap();
    assert_eq!(registry.tags.len(), 1);
    let (activity_status, activity_info): (String, String) = sqlx::query_as(
        "SELECT status, info FROM activityevents WHERE resourceid=$1 AND eventtype='RegistryCreated'",
    )
    .bind(registry.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(activity_status, "Success");
    assert!(activity_info.contains("\"Configuration\""));
    assert!(activity_info.contains("****************"));
    assert!(!activity_info.contains("\"Password\":\"secret\""));

    let missing_tag = Uuid::now_v7();
    let failed = store
        .update_registry(
            actor,
            registry.id,
            &RegistryPatch {
                name: Some(format!("must-not-commit-{suffix}")),
                tag_ids: Some(vec![missing_tag]),
                ..RegistryPatch::default()
            },
            CatalogMutationKind::Update,
        )
        .await;
    assert!(matches!(failed, Err(ResourceMetadataError::Validation(_))));
    let unchanged = store.get_registry(registry.id).await.unwrap();
    assert_eq!(unchanged.name, registry.name);
    assert_eq!(unchanged.tags[0].id, tag.id);

    let service = ResourceMetadataService::new(
        store.clone(),
        Arc::new(AesGcmSecretProtector::new(&[17_u8; 32]).unwrap()),
    );
    let secret = service
        .create_internal_secret(&format!("secret-{suffix}"), "not-plain-text")
        .await
        .unwrap();
    let encrypted: String =
        sqlx::query_scalar("SELECT encryptedvalue FROM internalsecretvalues WHERE secretid=$1")
            .bind(secret.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(encrypted, "not-plain-text");
    assert!(!encrypted.contains("not-plain-text"));

    validate_binding(BindingValidation {
        name: "TOKEN",
        kind: ResourceBindingKind::Secret,
        scope: ResourceBindingScope::Global,
        resource_id: None,
        value: None,
        secret_id: Some(secret.id),
        delivery: Some(SecretDeliveryMode::EnvironmentVariable),
        target_path: None,
    })
    .unwrap();
    let bindings = store
        .create_binding(&NewResourceBinding {
            name: "TOKEN".into(),
            kind: ResourceBindingKind::Secret,
            scope: Some(ResourceBindingScope::Global),
            resource_id: None,
            value: None,
            secret_id: Some(secret.id),
            secret_delivery_mode: Some(SecretDeliveryMode::EnvironmentVariable),
            target_path: None,
        })
        .await
        .unwrap();
    assert_eq!(bindings.entries.len(), 1);
    assert_eq!(bindings.entries[0].value, None);

    let provider = service
        .create_secret_provider(SecretProviderInput {
            name: format!("vault-{suffix}"),
            address: "https://vault.example.test/".into(),
            mount_path: "/secret/".into(),
            token: "vault-token".into(),
        })
        .await
        .unwrap();
    let provider_configuration: String =
        sqlx::query_scalar("SELECT configuration FROM secretproviders WHERE id=$1")
            .bind(provider.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!provider_configuration.contains("vault-token"));

    let updated_provider = service
        .update_secret_provider(
            provider.id,
            SecretProviderPatch {
                name: Some(format!("vault-updated-{suffix}")),
                ..SecretProviderPatch::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated_provider.address, "https://vault.example.test");
    assert_eq!(updated_provider.mount_path, "secret");
    let preserved_configuration: String =
        sqlx::query_scalar("SELECT configuration FROM secretproviders WHERE id=$1")
            .bind(provider.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(preserved_configuration, provider_configuration);

    let external_secret = store
        .create_external_secret(&ExternalSecretInput {
            name: format!("PHASE5_{suffix}").to_uppercase(),
            provider_id: provider.id,
            external_path: "services/citadel".into(),
            external_key: "token".into(),
            external_version: Some(3),
        })
        .await
        .unwrap();
    let omitted_version: ExternalSecretPatch =
        serde_json::from_value(json!({"externalKey":"rotated-token"})).unwrap();
    let external_secret = store
        .update_external_secret(external_secret.id, &omitted_version)
        .await
        .unwrap();
    assert_eq!(
        external_secret.external_key.as_deref(),
        Some("rotated-token")
    );
    assert_eq!(external_secret.external_version, Some(3));
    let cleared_version: ExternalSecretPatch =
        serde_json::from_value(json!({"externalVersion":null})).unwrap();
    let external_secret = store
        .update_external_secret(external_secret.id, &cleared_version)
        .await
        .unwrap();
    assert_eq!(external_secret.external_version, None);
    store
        .delete_secret_definition(external_secret.id)
        .await
        .unwrap();

    store.delete_secret_provider(provider.id).await.unwrap();
    store
        .delete_binding(bindings.entries[0].id, ResourceBindingScope::Global, None)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM secretdefinitions WHERE id=$1 OR name=$2",
        )
        .bind(secret.id)
        .bind(format!("secret-{suffix}"))
        .fetch_one(&pool)
        .await
        .unwrap(),
        0,
        "an unreferenced secret created for a binding must not leak"
    );
    store
        .delete_registries(actor, &[registry.id])
        .await
        .unwrap();
    store.delete_tag(tag.id).await.unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE5_DATABASE_URL"]
async fn concurrent_tag_creation_keeps_one_normalized_name() {
    let database_url = std::env::var("CITADEL_PHASE5_DATABASE_URL")
        .expect("CITADEL_PHASE5_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let store = Arc::new(PostgresResourceMetadataStore::new(pool));
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let suffix = Uuid::now_v7().simple().to_string();
    let first = NewTag {
        name: format!("Concurrent-{suffix}"),
        color: "#112233".into(),
    };
    let second = NewTag {
        name: format!("concurrent-{suffix}"),
        color: "#445566".into(),
    };

    let (first_result, second_result) = tokio::join!(
        store.create_tag(actor, &first),
        store.create_tag(actor, &second)
    );
    let results = [first_result, second_result];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(ResourceMetadataError::Conflict(_))))
            .count(),
        1
    );

    let created = results.into_iter().find_map(Result::ok).unwrap();
    store.delete_tag(created.id).await.unwrap();
}
