use super::*;

pub(super) async fn verify(app: &Router, admin: &ActorPrincipal, id: Uuid) {
    let response = request(
        app,
        Method::GET,
        &format!("/api/v1/swarmServices/{id}/duplicate-draft"),
        Some(admin.clone()),
        None,
    )
    .await;
    let status = response.status();
    let body = response_json(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .all(Value::is_string)
    );
    let draft = body["draft"].clone();
    assert_eq!(draft["duplicateSource"]["resourceId"], id.to_string());
    assert_eq!(draft["duplicateSource"]["resourceType"], "SwarmService");
    let typed: citadel_server::api::resources::swarm_services::requests::CreateSwarmServiceInput =
        serde_json::from_value(draft.clone()).unwrap();
    let domain: citadel_swarm_services::CreateSwarmService = typed.into();
    assert_eq!(domain.duplicate_source.unwrap().resource_id, id);
    let mut invalid = draft.clone();
    invalid["spec"]["webhook"] = json!({"provider": "Invalid"});
    let response = request(
        app,
        Method::POST,
        "/api/v1/swarmServices",
        None,
        Some(invalid.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = request(
        app,
        Method::POST,
        "/api/v1/swarmServices",
        Some(admin.clone()),
        Some(invalid),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let mut invalid = draft;
    invalid["duplicateSource"]["resourceType"] = json!("Deployment");
    let response = request(
        app,
        Method::POST,
        "/api/v1/swarmServices",
        Some(admin.clone()),
        Some(invalid),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
