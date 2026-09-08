use super::*;
use citadel_adapters::actor_store::PostgresActorStore;
use citadel_identity::ActorStore;

// Ports ActorEndpointTests.ActorEndpoints_ShouldReadAndPersistEnabledState and
// extends the AdministratorGuard concurrency/last-administrator scenarios.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn actors_read_and_patch_use_actor_ids_and_preserve_administrator_access() {
    let mut f = fixture().await;
    f.app = citadel_server::actors_http::router(Arc::new(PostgresActorStore::new(f.pool.clone())));
    let (user, actor) = seed_user_record(&f.pool, &format!("actor-{}", Uuid::now_v7()), true).await;
    let path = format!("/api/v1/actors/{actor}");
    let value = json(send(&f, &path, Some(f.administrator.clone())).await).await;
    assert_eq!(value["id"], actor.to_string());
    assert_eq!(value["type"], "User");
    assert_eq!(value["isEnabled"], true);
    for body in [r#"{}"#, r#"{"isEnabled":"false"}"#, "{"] {
        assert_eq!(
            send_raw_json(
                &f,
                Method::PATCH,
                &format!("{path}/enabled"),
                body,
                Some(f.administrator.clone())
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        send(&f, "/api/v1/actors/invalid", Some(f.administrator.clone()))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &f,
            &format!("/api/v1/actors/{user}"),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&f, &path, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let mut reader = f.administrator.clone();
    reader.roles.clear();
    assert_eq!(
        send(&f, &path, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    let patch = format!("{path}/enabled");
    assert_eq!(
        send_raw_json(
            &f,
            Method::PATCH,
            &patch,
            r#"{"isEnabled":false}"#,
            Some(reader)
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send_raw_json(&f, Method::PATCH, &patch, r#"{"isEnabled":false}"#, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = send_raw_json(
        &f,
        Method::PATCH,
        &patch,
        r#"{"isEnabled":false}"#,
        Some(f.administrator.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await["isEnabled"], false);
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT isenabled FROM actors WHERE id=$1")
            .bind(actor)
            .fetch_one(&f.pool)
            .await
            .unwrap()
    );
    assert_eq!(
        send_raw_json(
            &f,
            Method::PATCH,
            &format!("/api/v1/actors/{SYSTEM_ACTOR_ID}/enabled"),
            r#"{"isEnabled":false}"#,
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    demote_other_administrators(&f.pool, f.administrator.actor_id.value()).await;
    let response = send_raw_json(
        &f,
        Method::PATCH,
        &format!(
            "/api/v1/actors/{}/enabled",
            f.administrator.actor_id.value()
        ),
        r#"{"isEnabled":false}"#,
        Some(f.administrator.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(enabled_administrator_count(&f.pool).await, 1);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn concurrent_actor_disables_cannot_remove_both_administrators() {
    let f = fixture().await;
    let (_, actor) =
        seed_user_record(&f.pool, &format!("other-admin-{}", Uuid::now_v7()), true).await;
    demote_other_administrators(&f.pool, f.administrator.actor_id.value()).await;
    assign_role(&f.pool, actor, ADMIN_ROLE_ID).await;
    let store = PostgresActorStore::new(f.pool.clone());
    let (a, b) = tokio::join!(
        store.set_enabled(actor, false),
        store.set_enabled(f.administrator.actor_id.value(), false)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert_eq!(enabled_administrator_count(&f.pool).await, 1);
}
