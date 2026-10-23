use citadel_adapters::{
    persistence::postgres::bindings::{
        PostgresBindingRepository, secret_resolver::PostgresSecretValueResolver,
    },
    security::identity::crypto::AesGcmSecretProtector,
};
use citadel_bindings::{
    BindingRepository, NewResourceBinding, ResourceBindingInput, ResourceBindingKind,
    ResourceBindingScope, SecretDeliveryMode, SecretService, SecretUseScope,
};
use citadel_primitives::ResourceType;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires disposable CITADEL_METADATA_DATABASE_URL"]
async fn known_secret_ids_cannot_cross_scopes_and_resolution_rechecks_grants() {
    let url = std::env::var("CITADEL_METADATA_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let store = Arc::new(PostgresBindingRepository::new(pool.clone()));
    let protector = Arc::new(AesGcmSecretProtector::new(&[17; 32]).unwrap());
    let service = SecretService::new(store.clone(), protector.clone());
    let resolver = PostgresSecretValueResolver::new(pool.clone(), protector).unwrap();
    let a = Uuid::now_v7();
    let b = Uuid::now_v7();
    for id in [a, b] {
        sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$2,$3,'WebEditor','{}','{}')")
            .bind(id).bind(format!("security-{id}")).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    }
    let secret = service
        .create_internal_secret(
            "SCOPED",
            "synthetic-secret",
            SecretUseScope::Resource {
                resource_type: ResourceType::Stack,
                resource_id: a,
            },
        )
        .await
        .unwrap();
    let global = service
        .create_internal_secret("GLOBAL", "synthetic-global", SecretUseScope::Global)
        .await
        .unwrap();
    let binding = |resource, secret_id| NewResourceBinding {
        name: "TOKEN".into(),
        kind: ResourceBindingKind::Secret,
        scope: Some(ResourceBindingScope::Stack),
        resource_id: Some(resource),
        value: None,
        secret_id: Some(secret_id),
        secret_delivery_mode: Some(SecretDeliveryMode::EnvironmentVariable),
        target_path: None,
    };
    store.create_binding(&binding(a, secret.id)).await.unwrap();
    assert!(store.create_binding(&binding(b, secret.id)).await.is_err());
    let own = store.create_binding(&binding(b, global.id)).await.unwrap();
    let patch = ResourceBindingInput {
        id: own.entries[0].id,
        name: "TOKEN".into(),
        kind: ResourceBindingKind::Secret,
        value: None,
        secret_id: Some(secret.id),
        secret_delivery_mode: Some(SecretDeliveryMode::EnvironmentVariable),
        target_path: None,
    };
    assert!(
        store
            .update_binding(&patch, ResourceBindingScope::Stack, Some(b))
            .await
            .is_err()
    );
    assert_eq!(
        resolver
            .resolve(secret.id, "Stack", a)
            .await
            .unwrap()
            .as_str(),
        "synthetic-secret"
    );
    assert!(resolver.resolve(secret.id, "Stack", b).await.is_err());
    assert_eq!(
        resolver
            .resolve(global.id, "Stack", b)
            .await
            .unwrap()
            .as_str(),
        "synthetic-global"
    );
    assert!(
        store
            .list_secret_definitions(SecretUseScope::Resource {
                resource_type: ResourceType::Stack,
                resource_id: b
            })
            .await
            .unwrap()
            .iter()
            .all(|s| s.id != secret.id)
    );
    // Revoking a use grant invalidates an already persisted binding at resolution.
    sqlx::query(
        "DELETE FROM secretusegrants WHERE secretid=$1 AND scope='Stack' AND resourceid=$2",
    )
    .bind(secret.id)
    .bind(a)
    .execute(&pool)
    .await
    .unwrap();
    assert!(resolver.resolve(secret.id, "Stack", a).await.is_err());
    // Backup credentials obey the same use boundary at create and patch time.
    use citadel_backups::{BackupPersistence, BackupRepositoryConfiguration};
    let backups = citadel_adapters::persistence::postgres::backups::PostgresBackupPersistence::new(
        pool.clone(),
    );
    let mut input: BackupRepositoryConfiguration = serde_json::from_value(serde_json::json!({
        "name": format!("security-repository-{}", Uuid::now_v7()),
        "spec": {"$type":"FileSystem","location":"Core","path":"/tmp/disposable-backup-fixture"},
        "passwordSecretId":secret.id
    }))
    .unwrap();
    assert!(
        backups
            .create_repository(
                citadel_primitives::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
                &input
            )
            .await
            .is_err()
    );
    input.password_secret_id = global.id;
    let repository = backups
        .create_repository(
            citadel_primitives::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
            &input,
        )
        .await
        .unwrap();

    // Password identity is immutable in the metadata patch API.
    let unchanged = backups
        .update_repository(
            repository.id,
            &serde_json::json!({"passwordSecretId":secret.id}),
        )
        .await
        .unwrap();
    assert_eq!(unchanged.password_secret_id, global.id);
    input.name = format!("security-s3-{}", Uuid::now_v7());
    input.spec = serde_json::from_value(serde_json::json!({"$type":"S3Compatible","endpoint":"https://fixture.invalid","bucket":"fixture","accessKeySecretId":global.id,"secretKeySecretId":global.id})).unwrap();
    let repository = backups
        .create_repository(
            citadel_primitives::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
            &input,
        )
        .await
        .unwrap();
    let operation = Uuid::now_v7();
    assert!(
        backups
            .acquire_repository_operation(
                repository.id,
                operation,
                "Credentials",
                chrono::Utc::now() + chrono::Duration::minutes(5)
            )
            .await
            .unwrap()
    );
    let credentials = citadel_backups::BackupRepositoryCredentials {
        access_key_secret_id: secret.id,
        secret_key_secret_id: global.id,
        session_token_secret_id: None,
    };
    assert!(matches!(
        backups
            .commit_repository_credentials(
                repository.id,
                operation,
                repository.row_version,
                &credentials
            )
            .await,
        Err(citadel_backups::BackupError::Validation(_))
    ));
    backups
        .release_repository_operation(repository.id, operation)
        .await
        .unwrap();
    // Credentials created while configuring a new resource are private to that creator.
    let pending = service
        .create_internal_secret(
            "PENDING",
            "pending-value",
            SecretUseScope::Creator(citadel_primitives::ActorId::new(
                citadel_identity::SYSTEM_ACTOR_ID,
            )),
        )
        .await
        .unwrap();
    input.name = format!("security-pending-{}", Uuid::now_v7());
    input.password_secret_id = pending.id;
    let wrong_actor = citadel_primitives::ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,type) VALUES($1,'User')")
        .bind(wrong_actor.value())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        backups
            .create_repository(wrong_actor, &input)
            .await
            .is_err()
    );
    let repository = backups
        .create_repository(
            citadel_primitives::ActorId::new(citadel_identity::SYSTEM_ACTOR_ID),
            &input,
        )
        .await
        .unwrap();
    assert_eq!(
        resolver
            .resolve(pending.id, "BackupRepository", repository.id)
            .await
            .unwrap()
            .as_str(),
        "pending-value"
    );
    assert!(
        resolver
            .resolve(pending.id, "BackupRepository", Uuid::now_v7())
            .await
            .is_err()
    );
}
