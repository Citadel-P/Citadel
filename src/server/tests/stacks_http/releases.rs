use super::*;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    platform_id: Uuid,
    runtime: &CompletingStackRuntime,
) {
    let created = json_request(app, admin, Method::POST, "/api/v1/stacks", Some(json!({
        "name":format!("release-history-{}", Uuid::now_v7()),
        "platformId":platform_id, "stackSource":"WebEditor",
        "spec":{"$type":"WebEditor","composeFile":compose("initial"),"updateBehavior":"Disabled"}
    }))).await;
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    let initial_id = created["currentStackReleaseId"].clone();
    edit(app, admin, id, "deployed").await;
    assert_eq!(
        count(pool, id).await,
        1,
        "saving an initial draft reuses it"
    );
    apply(app, admin, id).await;
    let first = detail(app, admin, id).await;
    assert_eq!(first["version"], "1");
    assert_eq!(first["currentStackReleaseId"], initial_id);
    assert_eq!(history(app, admin, id).await, json!([]));

    edit(app, admin, id, "intermediate").await;
    assert_eq!(
        count(pool, id).await,
        2,
        "preserve the deployed snapshot once"
    );
    assert_eq!(
        history(app, admin, id).await,
        json!([]),
        "a save must not add visible release history"
    );
    edit(app, admin, id, "final").await;
    assert_eq!(
        count(pool, id).await,
        2,
        "repeated saves must not duplicate snapshots"
    );
    assert_eq!(detail(app, admin, id).await["version"], "1");
    assert_eq!(history(app, admin, id).await, json!([]));

    // Older Rust versions copied every save as Healthy. Keep those rows but
    // show only the first preserved snapshot of each historical version.
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,status,version,spec,createdat,createdbyactorid) SELECT $1,r.stackid,r.platformid,r.status,r.version,r.spec,r.createdat,r.createdbyactorid FROM stackreleases r JOIN stacks s ON s.currentstackreleaseid=r.id WHERE s.id=$2")
        .bind(Uuid::now_v7()).bind(id).execute(pool).await.unwrap();
    apply(app, admin, id).await;
    let second = detail(app, admin, id).await;
    assert_eq!(second["version"], "2");
    assert_ne!(second["currentStackReleaseId"], initial_id);
    let previous = history(app, admin, id).await;
    assert_eq!(previous.as_array().unwrap().len(), 1);
    assert_eq!(previous[0]["version"], "1");
    assert_eq!(
        previous[0]["spec"]["composeFile"],
        compose("deployed"),
        "history must retain the deployed configuration, not an intermediate save"
    );
    assert_eq!(previous[0]["status"], "Healthy");
    let before = count(pool, id).await;
    apply(app, admin, id).await;
    assert_eq!(
        detail(app, admin, id).await["currentStackReleaseId"],
        second["currentStackReleaseId"]
    );
    assert_eq!(
        count(pool, id).await,
        before,
        "unchanged redeploy reuses the current version"
    );

    edit(app, admin, id, "fails").await;
    runtime.apply_failure.store(1, Ordering::Relaxed);
    let _ = json_request(
        app,
        admin,
        Method::POST,
        "/api/v1/stacks/apply",
        Some(json!({"id":id})),
    )
    .await;
    runtime.apply_failure.store(0, Ordering::Relaxed);
    let failed = detail(app, admin, id).await;
    assert_eq!(failed["status"], "Failed");
    assert_eq!(failed["version"], "3");
    let previous = history(app, admin, id).await;
    assert_eq!(previous.as_array().unwrap().len(), 2);
    assert!(
        previous
            .as_array()
            .unwrap()
            .iter()
            .all(|release| release["status"] == "Healthy" && release["version"] != "3")
    );
    edit(app, admin, id, "retry").await;
    let before = count(pool, id).await;
    apply(app, admin, id).await;
    let retried = detail(app, admin, id).await;
    assert_eq!(
        retried["currentStackReleaseId"],
        failed["currentStackReleaseId"]
    );
    assert_eq!(retried["version"], "3");
    assert_eq!(
        count(pool, id).await,
        before,
        "retry a failed version without creating another release"
    );

    let first_release = previous
        .as_array()
        .unwrap()
        .iter()
        .find(|release| release["version"] == "1")
        .unwrap();
    let rolled_back = json_request(
        app,
        admin,
        Method::POST,
        "/api/v1/stacks/rollback",
        Some(json!({"stackId":id,"releaseId":first_release["id"]})),
    )
    .await;
    assert_eq!(
        rolled_back.as_array().unwrap().last().unwrap()["stackStatus"],
        "Healthy"
    );
    let current = detail(app, admin, id).await;
    assert_eq!(current["version"], "4");
    assert_eq!(current["spec"]["composeFile"], compose("deployed"));

    let response = request(
        app,
        Method::DELETE,
        "/api/v1/stacks",
        Some(admin.clone()),
        Some(json!([id])),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

fn compose(tag: &str) -> String {
    format!("services:\n  app:\n    image: nginx:{tag}\n")
}

async fn count(pool: &sqlx::PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM stackreleases WHERE stackid=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn edit(app: &Router, admin: &ActorPrincipal, id: Uuid, tag: &str) {
    json_request(
        app,
        admin,
        Method::PATCH,
        &format!("/api/v1/stacks/{id}"),
        Some(json!({"spec":{"composeFile":compose(tag)}})),
    )
    .await;
}

async fn apply(app: &Router, admin: &ActorPrincipal, id: Uuid) {
    let result = json_request(
        app,
        admin,
        Method::POST,
        "/api/v1/stacks/apply",
        Some(json!({"id":id})),
    )
    .await;
    assert_eq!(
        result.as_array().unwrap().last().unwrap()["stackStatus"],
        "Healthy"
    );
}

async fn detail(app: &Router, admin: &ActorPrincipal, id: Uuid) -> Value {
    json_request(
        app,
        admin,
        Method::GET,
        &format!("/api/v1/stacks/{id}"),
        None,
    )
    .await
}

async fn history(app: &Router, admin: &ActorPrincipal, id: Uuid) -> Value {
    json_request(
        app,
        admin,
        Method::GET,
        &format!("/api/v1/stacks/{id}/releases"),
        None,
    )
    .await["releases"]
        .clone()
}

async fn json_request(
    app: &Router,
    admin: &ActorPrincipal,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Value {
    let response = request(app, method, path, Some(admin.clone()), body).await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}
