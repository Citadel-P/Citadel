use super::*;
use citadel_backups::BackupPersistence;

pub(super) async fn before_ready(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    repository: &Value,
) {
    let id = repository["id"].as_str().unwrap();
    let path = format!("/api/v1/backupRepositories/{id}");
    let reader = seed_regular_user(pool).await;
    for (principal, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(reader), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(
                app,
                Method::PATCH,
                &path,
                principal,
                Some(json!({"name":"unauthorized rename"}))
            )
            .await
            .status(),
            expected
        );
    }
    // Renaming must not erase the destination or replace the encryption password.
    let updated_name = format!("updated repository {id}");
    let response = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"name":format!(" {updated_name} "),"passwordSecretId":Uuid::now_v7()})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated = response_json(response).await;
    assert!(updated.get("description").is_none());
    assert_eq!(updated["spec"], repository["spec"]);
    assert_eq!(updated["passwordSecretId"], repository["passwordSecretId"]);
    assert_eq!(updated["name"], updated_name);
    for patch in [
        json!([]),
        json!({"name":null}),
        json!({"name":42}),
        json!({"name":" "}),
        json!({"name":"x".repeat(129)}),
        json!({"spec":{"$type":"FileSystem","location":"Unknown","path":"/tmp/backups"}}),
        json!({"spec":{"$type":"S3Compatible","bucketLookup":"Unknown"}}),
        json!({"spec":{"$type":"FileSystem","path":""}}),
    ] {
        assert_eq!(
            request(app, Method::PATCH, &path, Some(admin.clone()), Some(patch))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut spec = repository["spec"].clone();
    spec["path"] = json!(" /tmp/changed-backups ");
    let store = PostgresBackupPersistence::new(pool.clone());
    let repo_id = Uuid::parse_str(id).unwrap();
    let operation = Uuid::now_v7();
    assert!(
        store
            .acquire_repository_operation(
                repo_id,
                operation,
                "Validate",
                chrono::Utc::now() + Duration::minutes(2)
            )
            .await
            .unwrap()
    );
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"spec":spec,"name":"must not persist"}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        store.get_repository(repo_id).await.unwrap().name,
        updated_name
    );
    store
        .release_repository_operation(repo_id, operation)
        .await
        .unwrap();
    let changed = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"spec":spec})),
    )
    .await;
    assert_eq!(changed.status(), StatusCode::OK);
    let changed = response_json(changed).await;
    assert_eq!(changed["spec"]["path"], "/tmp/changed-backups");
    assert_eq!(changed["name"], updated_name);
    let mut denied = repository["spec"].clone();
    denied["path"] = json!("/outside-allowed-backup-path");
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"spec":denied}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    for operation in ["initialize", "check", "prune"] {
        let response = request(
            app,
            Method::POST,
            &format!("{path}/{operation}"),
            Some(admin.clone()),
            Some(json!({"location":"Core","platformId":null})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("CITADEL_BACKUPS_ALLOWED_CORE_PATH_N"));
        assert_eq!(
            store.get_repository(repo_id).await.unwrap().control_state,
            citadel_primitives::ResourceControlState::Idle
        );
    }
    // Readiness checks still return their recorded validation result.
    let validation = request(
        app,
        Method::POST,
        &format!("{path}/validate"),
        Some(admin.clone()),
        Some(json!({"location":"Core","platformId":null})),
    )
    .await;
    assert_eq!(validation.status(), StatusCode::OK);
    assert_ne!(response_json(validation).await["status"], "Ready");
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"spec":repository["spec"]}))
        )
        .await
        .status(),
        StatusCode::OK
    );
}

pub(super) async fn after_ready(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    repository: &Value,
) {
    let id = repository["id"].as_str().unwrap();
    let path = format!("/api/v1/backupRepositories/{id}");
    // BackupEntitiesTests.BackupRepository_ShouldRejectLocationChangeAfterReady.
    let store = PostgresBackupPersistence::new(pool.clone());
    let repo_id = Uuid::parse_str(id).unwrap();
    let before = store.get_repository(repo_id).await.unwrap();
    for _ in 0..2 {
        let response = request(
            app,
            Method::POST,
            &format!("{path}/initialize"),
            Some(admin.clone()),
            Some(json!({"location":"Core","platformId":null})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let problem: Value = serde_json::from_slice(&body).unwrap();
        assert!(
            problem["detail"]
                .as_str()
                .unwrap()
                .contains("Use Validate or Check")
        );
    }
    let after = store.get_repository(repo_id).await.unwrap();
    assert_eq!(after.status, citadel_backups::BackupRepositoryStatus::Ready);
    assert_eq!(after.last_checked_at, before.last_checked_at);
    assert_eq!(after.row_version, before.row_version);
    let lease_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM backuprepositoryleases WHERE backuprepositoryid=$1",
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(lease_count, 0);
    let mut spec = repository["spec"].clone();
    spec["path"] = json!("/other");
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"spec":spec,"name":"must roll back"}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let stored: (Value, String) =
        sqlx::query_as("SELECT spec,name FROM backuprepositories WHERE id=$1")
            .bind(Uuid::parse_str(id).unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(stored.0, repository["spec"]);
    assert_eq!(stored.1, before.name);
    let same = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"spec":repository["spec"]})),
    )
    .await;
    assert_eq!(same.status(), StatusCode::OK);
    assert!(response_json(same).await.get("description").is_none());
    let renamed_name = format!("Renamed repository {id}");
    let other_name = format!("Other repository {id}");
    let renamed = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"name":format!(" {renamed_name} ")})),
    )
    .await;
    assert_eq!(renamed.status(), StatusCode::OK);
    let renamed = response_json(renamed).await;
    assert_eq!(renamed["name"], renamed_name);
    assert_eq!(renamed["id"], repository["id"]);
    assert_eq!(renamed["spec"], repository["spec"]);
    assert_eq!(renamed["passwordSecretId"], repository["passwordSecretId"]);
    assert_eq!(renamed["status"], "Ready");
    let normalized: String =
        sqlx::query_scalar("SELECT normalizedname FROM backuprepositories WHERE id=$1")
            .bind(repo_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(normalized, renamed_name.to_uppercase());
    let duplicate = request(app, Method::POST, "/api/v1/backupRepositories", Some(admin.clone()),
        Some(json!({"name":other_name,"passwordSecretId":repository["passwordSecretId"],"spec":repository["spec"]}))).await;
    assert_eq!(duplicate.status(), StatusCode::OK);
    let duplicate_id = response_json(duplicate).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let conflict = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"name":other_name.to_uppercase()})),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let unchanged = store.get_repository(repo_id).await.unwrap();
    assert_eq!(unchanged.name, renamed_name);
    assert_eq!(unchanged.spec, before.spec);
    assert_eq!(
        request(
            app,
            Method::DELETE,
            &format!("/api/v1/backupRepositories/{duplicate_id}"),
            Some(admin.clone()),
            None
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &format!("/api/v1/backupRepositories/{}", Uuid::now_v7()),
            Some(admin.clone()),
            Some(json!({}))
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
}
