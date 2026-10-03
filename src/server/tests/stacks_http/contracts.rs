use super::*;

pub(super) async fn verify(app: &Router, admin: &ActorPrincipal, id: Uuid) {
    let metadata = format!("/api/v1/stacks/{id}/_metadata");
    // Deferred extraction must preserve authorization before validation errors.
    let response = request(
        app,
        Method::PATCH,
        &metadata,
        None,
        Some(json!({"description": 42})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = request(
        app,
        Method::PATCH,
        &metadata,
        Some(admin.clone()),
        Some(json!({"description": 42})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    for (patch, expected) in [
        (json!({"description": "metadata"}), json!("metadata")),
        (json!({}), json!("metadata")),
        (json!({"description": null}), Value::Null),
    ] {
        let response = request(
            app,
            Method::PATCH,
            &metadata,
            Some(admin.clone()),
            Some(patch),
        )
        .await;
        let status = response.status();
        let body = response_json(response).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["description"], expected);
    }
    let response = request(
        app,
        Method::GET,
        &format!("/api/v1/stacks/{id}/duplicate-draft"),
        Some(admin.clone()),
        None,
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let mut draft = body["draft"].clone();
    assert_eq!(draft["duplicateSource"]["resourceType"], "Stack");
    assert_eq!(draft["duplicateSource"]["resourceId"], id.to_string());
    let typed: citadel_server::api::resources::stacks::requests::CreateStackInput =
        serde_json::from_value(draft.clone()).unwrap();
    assert!(citadel_stacks::CreateStack::try_from(typed).is_ok());
    draft["duplicateSource"]["resourceType"] = json!("Deployment");
    let response = request(
        app,
        Method::POST,
        "/api/v1/stacks",
        Some(admin.clone()),
        Some(draft),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
