use super::*;
use citadel_backups::{BackupPersistence, BackupRepositoryCredentials};

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    secret: Uuid,
    service: &BackupService,
) {
    let original = json!({"$type":"S3Compatible","endpoint":"https://storage.example.test","bucket":"backups","prefix":"prod","accessKeySecretId":secret,"secretKeySecretId":secret,"sessionTokenSecretId":secret});
    let repository = response_json(request(app, Method::POST, "/api/v1/backupRepositories", Some(admin.clone()),
        Some(json!({"name":format!("credential-test-{}",Uuid::now_v7()),"passwordSecretId":secret,"spec":original}))).await).await;
    let id = Uuid::parse_str(repository["id"].as_str().unwrap()).unwrap();
    let path = format!("/api/v1/backupRepositories/{id}/credentials");
    let new_secret = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
    )
    .bind(new_secret)
    .bind(format!("replacement-{new_secret}"))
    .execute(pool)
    .await
    .unwrap();
    let input = json!({"accessKeySecretId":new_secret,"secretKeySecretId":new_secret,"sessionTokenSecretId":null,"location":"Core","platformId":null});
    for (actor, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(seed_regular_user(pool).await), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(app, Method::POST, &path, actor, Some(input.clone()))
                .await
                .status(),
            status
        );
    }
    use citadel_identity::{ResourceAccessInput, UserRepository};
    let users = citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository::new(pool.clone());
    for level in [
        citadel_primitives::PermissionLevel::Write,
        citadel_primitives::PermissionLevel::Execute,
    ] {
        let user = seed_regular_user(pool).await;
        users
            .add_resource_access(
                user.subject_id,
                &ResourceAccessInput {
                    resource_type: ResourceType::BackupRepository,
                    resource_id: id,
                    permission_level: level,
                    specific_permissions: vec![],
                },
                admin.actor_id,
                chrono::Utc::now(),
                true,
            )
            .await
            .unwrap();
        // Write alone cannot execute validation; Execute alone does not grant Bindings Read.
        assert_eq!(
            request(app, Method::POST, &path, Some(user), Some(input.clone()))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    sqlx::query("UPDATE backuprepositories SET status='Ready' WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let store = PostgresBackupPersistence::new(pool.clone());
    let owner = Uuid::now_v7();
    assert!(
        store
            .acquire_repository_operation(
                id,
                owner,
                "Backup",
                chrono::Utc::now() + Duration::minutes(2)
            )
            .await
            .unwrap()
    );
    assert_eq!(
        request(
            app,
            Method::POST,
            &path,
            Some(admin.clone()),
            Some(input.clone())
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    store.release_repository_operation(id, owner).await.unwrap();

    let mut invalid = input.clone();
    invalid["accessKeySecretId"] = json!(Uuid::nil());
    assert_eq!(
        request(app, Method::POST, &path, Some(admin.clone()), Some(invalid))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut failed = input.clone();
    // The fake executor rejects this credential to exercise validation failure.
    failed["accessKeySecretId"] = json!(Uuid::from_u128(1));
    assert_eq!(
        request(app, Method::POST, &path, Some(admin.clone()), Some(failed))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let unchanged = store.get_repository(id).await.unwrap();
    assert_eq!(
        serde_json::to_value(&unchanged.spec).unwrap()["accessKeySecretId"],
        json!(secret)
    );
    assert_eq!(
        unchanged.status,
        citadel_backups::BackupRepositoryStatus::Ready
    );
    let leases: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM backuprepositoryleases WHERE backuprepositoryid=$1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(leases, 0);
    let mut forbidden_destination = input.clone();
    forbidden_destination["bucket"] = json!("other");
    assert_eq!(
        request(
            app,
            Method::POST,
            &path,
            Some(admin.clone()),
            Some(forbidden_destination)
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let updated = response_json(
        request(
            app,
            Method::POST,
            &path,
            Some(admin.clone()),
            Some(input.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(updated["passwordSecretId"], json!(secret));
    assert_eq!(updated["status"], "Ready");
    assert_eq!(updated["name"], repository["name"]);
    for field in ["endpoint", "bucket", "prefix"] {
        assert_eq!(updated["spec"][field], original[field]);
    }
    assert_eq!(updated["spec"]["accessKeySecretId"], json!(new_secret));
    assert_eq!(updated["spec"]["secretKeySecretId"], json!(new_secret));
    assert!(updated["spec"]["sessionTokenSecretId"].is_null());
    // Ordinary PATCH still cannot bypass credential validation on a ready repository.
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &format!("/api/v1/backupRepositories/{id}"),
            Some(admin.clone()),
            Some(json!({"spec":original}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );

    let credentials = BackupRepositoryCredentials {
        access_key_secret_id: secret,
        secret_key_secret_id: secret,
        session_token_secret_id: Some(secret),
    };
    let before = store.get_repository(id).await.unwrap();
    // Commit requires the current unexpired lease owner and unchanged row version.
    assert!(
        store
            .commit_repository_credentials(id, owner, before.row_version, &credentials)
            .await
            .is_err()
    );
    assert!(
        store
            .acquire_repository_operation(
                id,
                owner,
                "Validate",
                chrono::Utc::now() + Duration::minutes(2)
            )
            .await
            .unwrap()
    );
    assert!(
        store
            .commit_repository_credentials(id, Uuid::now_v7(), before.row_version, &credentials)
            .await
            .is_err()
    );
    store
        .update_repository(id, &json!({"name":format!("concurrent edit {id}")}))
        .await
        .unwrap();
    assert!(
        store
            .commit_repository_credentials(id, owner, before.row_version, &credentials)
            .await
            .is_err()
    );
    let current = store.get_repository(id).await.unwrap();
    sqlx::query("UPDATE backuprepositoryleases SET expiresat=CURRENT_TIMESTAMP - INTERVAL '1 second' WHERE backuprepositoryid=$1").bind(id).execute(pool).await.unwrap();
    assert!(
        store
            .commit_repository_credentials(id, owner, current.row_version, &credentials)
            .await
            .is_err()
    );
    store.release_repository_operation(id, owner).await.unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert!(
        service
            .update_repository_credentials(id, &credentials, "Core", None, &cancellation)
            .await
            .is_err()
    );
    assert_eq!(store.get_repository(id).await.unwrap().spec, current.spec);
    // Retrying with the current credentials is harmless after an ambiguous HTTP response.
    assert_eq!(
        request(app, Method::POST, &path, Some(admin.clone()), Some(input))
            .await
            .status(),
        StatusCode::OK
    );
    store.archive_repository(id).await.unwrap();
}
