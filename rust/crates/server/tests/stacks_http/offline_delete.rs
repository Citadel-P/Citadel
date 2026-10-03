use super::*;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    platform: Uuid,
    swarm: Uuid,
    runtime: &CompletingStackRuntime,
) {
    let mut ids = Vec::new();
    for platform_id in [platform, swarm, platform] {
        let response = request(app, Method::POST, "/api/v1/stacks", Some(admin.clone()), Some(json!({
            "name": format!("offline-delete-{}", Uuid::now_v7()), "platformId": platform_id,
            "stackSource": "WebEditor", "spec": {"$type":"WebEditor", "composeFile":"services:\n  web:\n    image: nginx:alpine\n", "updateBehavior":"Disabled"},
        }))).await;
        assert_eq!(response.status(), StatusCode::OK);
        ids.push(Uuid::parse_str(response_json(response).await["id"].as_str().unwrap()).unwrap());
    }
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=ANY($1::uuid[])")
        .bind(vec![platform, swarm])
        .execute(pool)
        .await
        .unwrap();
    let before = runtime.delete_calls.load(Ordering::Relaxed);
    assert_eq!(
        request(
            app,
            Method::DELETE,
            "/api/v1/stacks",
            None,
            Some(json!(&ids[..2]))
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("UPDATE stacks SET controlstate='Processing' WHERE id=$1")
        .bind(ids[0])
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::DELETE,
            "/api/v1/stacks",
            Some(admin.clone()),
            Some(json!(&ids[..2]))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE stacks SET controlstate='Idle' WHERE id=$1")
        .bind(ids[0])
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::DELETE,
            "/api/v1/stacks",
            Some(admin.clone()),
            Some(json!(&ids[..2]))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        runtime.delete_calls.load(Ordering::Relaxed),
        before,
        "Offline deletion must not contact Docker or the Agent"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM stacks WHERE id=ANY($1::uuid[])")
        .bind(&ids[..2])
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let audit_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activityevents WHERE resourceid=ANY($1::uuid[]) AND eventtype='StackDeleted'").bind(&ids[..2]).fetch_one(pool).await.unwrap();
    assert_eq!(audit_count, 2);
    sqlx::query("UPDATE platforms SET status='Online' WHERE id=ANY($1::uuid[])")
        .bind(vec![platform, swarm])
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::DELETE,
            "/api/v1/stacks",
            Some(admin.clone()),
            Some(json!([ids[2]]))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        runtime.delete_calls.load(Ordering::Relaxed),
        before + 1,
        "Online deletion still cleans up the runtime"
    );
}
