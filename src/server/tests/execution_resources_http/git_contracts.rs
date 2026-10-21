use super::*;

pub(super) async fn verify(app: &Router, pool: &sqlx::PgPool, admin: &ActorPrincipal) {
    let invalid = json!({"name":"invalid", "url":"https://example.com/repo.git", "defaultBranch":"main", "webhook":{"provider":"Invalid"}});
    let response = request(
        app,
        Method::POST,
        "/api/v1/gitRepositories",
        None,
        Some(invalid.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = request(
        app,
        Method::POST,
        "/api/v1/gitRepositories",
        Some(admin.clone()),
        Some(invalid),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let created = response_json(request(app, Method::POST, "/api/v1/gitRepositories", Some(admin.clone()), Some(json!({
        "name":"native-contract-fixture", "url":"https://example.com/repo.git", "defaultBranch":"main", "syncMode":"Manual",
        "webhook":{"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"fixture-secret","branchFilter":"main"}
    }))).await).await;
    assert_eq!(created["controlState"], "Queued");
    assert_eq!(created["syncStatus"]["status"], "Pending");
    assert!(created["syncStatus"]["lastSyncedAt"].is_string());
    let id = created["id"].as_str().unwrap();
    let path = format!("/api/v1/gitRepositories/{id}");
    let cfg_path = format!("{path}/_cfg");
    assert_eq!(created["webhook"]["provider"], "Generic");
    for get_path in [&path, &cfg_path] {
        let read =
            response_json(request(app, Method::GET, get_path, Some(admin.clone()), None).await)
                .await;
        assert_eq!(read["webhook"], created["webhook"]);
        if get_path == &path {
            assert_eq!(read["syncStatus"], created["syncStatus"]);
        }
    }
    let hooks = json!({"onClone":{"path":"./","commands":["echo clone"]},"onPull":{"path":"scripts","commands":["echo pull"]}});
    let saved = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(hooks.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(saved["onClone"], hooks["onClone"]);
    assert_eq!(saved["onPull"], hooks["onPull"]);
    for (level, can_write) in [(1, false), (2, true), (4, true)] {
        let reader = seed_regular_user(pool).await;
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,$3,$4,$5,0)")
            .bind(Uuid::now_v7()).bind(reader.actor_id.value()).bind(level)
            .bind(Uuid::parse_str(id).unwrap()).bind(ResourceType::GitRepository as i32)
            .execute(pool).await.unwrap();
        let config =
            response_json(request(app, Method::GET, &cfg_path, Some(reader.clone()), None).await)
                .await;
        assert_eq!(config["onClone"], hooks["onClone"]);
        let view =
            response_json(request(app, Method::GET, &path, Some(reader.clone()), None).await).await;
        assert_eq!(view["capabilities"]["canWrite"], can_write);
        assert_eq!(
            request(app, Method::PATCH, &path, Some(reader), Some(hooks.clone()))
                .await
                .status(),
            if can_write {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            }
        );
    }
    for directory in ["../outside", "/tmp", "C:/temp"] {
        assert_eq!(
            request(
                app,
                Method::PATCH,
                &path,
                Some(admin.clone()),
                Some(json!({"onPull":{"path":directory,"commands":["echo invalid"]}}))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let omitted = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"defaultBranch":"develop"})),
        )
        .await,
    )
    .await;
    assert_eq!(omitted["webhook"], created["webhook"]);
    assert_eq!(omitted["onClone"], hooks["onClone"]);
    assert_eq!(omitted["onPull"], hooks["onPull"]);
    for (principal, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(admin.clone()), StatusCode::BAD_REQUEST),
    ] {
        let response = request(
            app,
            Method::PATCH,
            &path,
            principal,
            Some(json!({"webhook":{"authScheme":"Invalid"}})),
        )
        .await;
        assert_eq!(response.status(), expected);
    }
    let after =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    assert_eq!(after["webhook"], created["webhook"]);
    let rotated = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"webhook":{"secret":"rotated-secret"}})),
        )
        .await,
    )
    .await;
    for field in ["enabled", "provider", "authScheme", "branchFilter"] {
        assert_eq!(
            rotated["webhook"][field], created["webhook"][field],
            "{field}"
        );
    }
    let cleared = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"webhook":null})),
        )
        .await,
    )
    .await;
    assert!(cleared["webhook"].is_null());
    let response = request(
        app,
        Method::DELETE,
        "/api/v1/gitRepositories",
        Some(admin.clone()),
        Some(json!({"ids":[id]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
