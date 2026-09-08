use super::*;
use citadel_backups::BackupStore;

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
                Some(json!({"description":"updated repository"}))
            )
            .await
            .status(),
            expected
        );
    }
    // BackupEndpointTests' description-only patch must not erase the destination.
    let response = request(app, Method::PATCH, &path, Some(admin.clone()), Some(json!({"description":" updated repository ","passwordSecretId":Uuid::now_v7(),"name":"ignored"}))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated = response_json(response).await;
    assert_eq!(updated["description"], "updated repository");
    assert_eq!(updated["spec"], repository["spec"]);
    assert_eq!(updated["passwordSecretId"], repository["passwordSecretId"]);
    assert_eq!(updated["name"], repository["name"]);
    for patch in [
        json!([]),
        json!({"description":true}),
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
    let store = PostgresBackupStore::new(pool.clone());
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
            Some(json!({"spec":spec,"description":"must not persist"}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        store
            .get_repository(repo_id)
            .await
            .unwrap()
            .description
            .as_deref(),
        Some("updated repository")
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
    assert_eq!(changed["description"], "updated repository");
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
    let mut spec = repository["spec"].clone();
    spec["path"] = json!("/other");
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"spec":spec,"description":"must roll back"}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let stored: (Value, Option<String>) =
        sqlx::query_as("SELECT spec,description FROM backuprepositories WHERE id=$1")
            .bind(Uuid::parse_str(id).unwrap())
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(stored.0, repository["spec"]);
    assert_eq!(stored.1.as_deref(), Some("updated repository"));
    let same = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"spec":repository["spec"],"description":null})),
    )
    .await;
    assert_eq!(same.status(), StatusCode::OK);
    assert!(response_json(same).await["description"].is_null());
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
