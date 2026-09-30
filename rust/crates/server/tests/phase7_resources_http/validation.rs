use super::*;

pub(super) async fn verify_inputs(app: &Router, admin: &ActorPrincipal, repository: Uuid) {
    for (path, input, field) in [
        (
            "/api/v1/alertRules",
            json!({"requiredMatches":"bad"}),
            "requiredMatches",
        ),
        (
            "/api/v1/alertRules/channels",
            json!({"isActive":"bad"}),
            "isActive",
        ),
        (
            "/api/v1/backupPolicies",
            json!({"timeoutSeconds":"bad"}),
            "timeoutSeconds",
        ),
        ("/api/v1/backupRepositories", json!({"name":42}), "name"),
        ("/api/v1/gitRepositories", json!({"url":false}), "url"),
    ] {
        invalid(app, admin, Method::POST, path, input, field).await;
    }
    let path = format!("/api/v1/gitRepositories/{repository}");
    let before =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    invalid(app, admin, Method::PATCH, &path, json!({"url":""}), "URL").await;
    let after =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    assert_eq!(before, after, "invalid Git configuration must not persist");
}

pub(super) async fn verify_policy(app: &Router, admin: &ActorPrincipal, policy: &Value) {
    let path = format!("/api/v1/backupPolicies/{}", policy["id"].as_str().unwrap());
    let before =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    invalid(
        app,
        admin,
        Method::PATCH,
        &path,
        json!({"timeoutSeconds":"bad"}),
        "timeoutSeconds",
    )
    .await;
    let after =
        response_json(request(app, Method::GET, &path, Some(admin.clone()), None).await).await;
    assert_eq!(before, after, "invalid Backup policy must not persist");
}

async fn invalid(
    app: &Router,
    admin: &ActorPrincipal,
    method: Method,
    path: &str,
    input: Value,
    field: &str,
) {
    let response = request(app, method, path, Some(admin.clone()), Some(input)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    let body = problem_json(response).await;
    assert!(body["errors"].is_object(), "{path}: {body}");
    assert!(
        body["errors"]
            .to_string()
            .to_lowercase()
            .contains(&field.to_lowercase()),
        "{path}: {body}"
    );
}

pub(super) async fn problem_json(response: axum::response::Response) -> Value {
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap()
}
