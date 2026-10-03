use super::*;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    platform_id: Uuid,
    runtime: &CompletingStackRuntime,
) {
    let compose =
        "services:\n  web:\n    image: nginx:alpine\n    environment:\n      API_KEY: ${API_KEY}\n";
    let created = response_json(request(app, Method::POST, "/api/v1/stacks", Some(admin.clone()), Some(json!({
        "name": format!("failed-stack-{}", Uuid::now_v7()), "platformId": platform_id,
        "stackSource": "WebEditor", "spec": {"$type": "WebEditor", "composeFile": compose, "updateBehavior": "Disabled"},
    }))).await).await;
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    runtime.apply_failure.store(2, Ordering::Relaxed);
    let response = request(
        app,
        Method::POST,
        "/api/v1/stacks/apply",
        Some(admin.clone()),
        Some(json!({"id": id})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let progress = response_json(response).await;
    runtime.apply_failure.store(0, Ordering::Relaxed);
    assert!(
        progress
            .to_string()
            .contains("port 8080 is already allocated"),
        "{progress}"
    );
    assert!(!progress.to_string().contains("fixture-secret-value"));
    let items = progress.as_array().unwrap();
    assert_eq!(
        items.last().unwrap()["message"],
        "Stack deployment failed (exit code 1)."
    );
    let pulled = items
        .iter()
        .find(|item| item["progressMessage"] == "beszel Pulled")
        .unwrap();
    assert!(pulled["severity"].is_null());
    for item in items.iter().filter(|item| {
        item["progressMessage"]
            .as_str()
            .is_some_and(|text| text.contains("Error response from daemon"))
    }) {
        assert_eq!(item["severity"], "error");
    }

    let info: Value = sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1 AND eventtype='StackApplied' AND status='Failure' ORDER BY createdat DESC LIMIT 1")
        .bind(id).fetch_one(pool).await.unwrap();
    assert!(
        info["Result"]["Message"]
            .as_str()
            .unwrap()
            .contains("port 8080 is already allocated")
    );
    assert!(!info.to_string().contains("fixture-secret-value"));
    let summary = info["Result"]["Message"].as_str().unwrap();
    assert!(!summary.contains("Pulled"));
    assert!(!summary.contains("Container beszel-agent"));
    assert_eq!(summary.matches("Error response from daemon").count(), 1);

    assert_eq!(
        info["Stack"]["StackRelease"]["Spec"]["ComposeFile"],
        compose
    );
    let detail = response_json(
        request(
            app,
            Method::GET,
            &format!("/api/v1/stacks/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(detail["status"], "Failed");
    assert!(
        detail
            .to_string()
            .contains("port 8080 is already allocated")
    );
    assert_eq!(
        request(
            app,
            Method::DELETE,
            "/api/v1/stacks",
            Some(admin.clone()),
            Some(json!([id]))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT,
    );
}
