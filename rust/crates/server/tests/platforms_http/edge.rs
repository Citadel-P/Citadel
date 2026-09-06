use super::*;
use axum::http::Method;
use citadel_adapters::edge::EdgeRegistry;
use citadel_server::platforms_http::EdgeHttpContext;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn edge_platform_creation_and_enrollment_endpoints_preserve_ui_contract_and_permissions() {
    let fixture = fixture().await;
    let app = fixture.app.clone().layer(axum::Extension(EdgeHttpContext {
        store: citadel_adapters::edge::PostgresEdgeStore::new(fixture.pool.clone()),
        registry: EdgeRegistry::default(),
        core_url: "https://core.example.test".into(),
        agent_image: "ghcr.io/citadel-p/citadel.agent:latest".into(),
    }));
    let input = json!({"name":format!("edge-{}",Uuid::now_v7().simple()),"connectorType":"EdgeAgent","type":"Docker","address":null,"tagIds":[fixture.tag_id]});
    let result = request(
        &app,
        Method::POST,
        "/api/v1/platforms",
        Some(fixture.administrator.clone()),
        Some(input),
    )
    .await;
    assert_eq!(result.status(), StatusCode::OK);
    let created: Value =
        serde_json::from_slice(&to_bytes(result.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    let id = created["id"].as_str().unwrap();
    assert_eq!(created["status"], "Offline");
    let enrollment = format!("/api/v1/platforms/{id}/edge/enrollments");
    assert_eq!(
        request(&app, Method::POST, &enrollment, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let mut denied = fixture.administrator.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert_eq!(
        request(&app, Method::POST, &enrollment, Some(denied), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let result = request(
        &app,
        Method::POST,
        &enrollment,
        Some(fixture.administrator.clone()),
        None,
    )
    .await;
    assert_eq!(result.status(), StatusCode::OK);
    assert!(
        result.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let result: Value =
        serde_json::from_slice(&to_bytes(result.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    assert_eq!(result["platformId"], id);
    assert_eq!(
        result["instructions"]["environment"]["CITADEL_EDGE_ENROLLMENT_TOKEN"],
        result["token"]
    );
    let status = request(
        &app,
        Method::GET,
        &format!("/api/v1/platforms/{id}/edge/status"),
        Some(fixture.administrator.clone()),
        None,
    )
    .await;
    assert_eq!(status.status(), StatusCode::OK);
    let status: Value =
        serde_json::from_slice(&to_bytes(status.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    assert_eq!(status["connectionStatus"], "PendingEnrollment");
    assert!(status["enrollmentExpiresAtUtc"].is_string());
    let revoke = request(
        &app,
        Method::POST,
        &format!("/api/v1/platforms/{id}/edge/revoke"),
        Some(fixture.administrator.clone()),
        None,
    )
    .await;
    assert_eq!(revoke.status(), StatusCode::NO_CONTENT);
    let revoked: bool =
        sqlx::query_scalar("SELECT revokedatutc IS NOT NULL FROM edgeagentenrollments WHERE id=$1")
            .bind(Uuid::parse_str(result["enrollmentId"].as_str().unwrap()).unwrap())
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert!(revoked);
    let activity: i64 =
        sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1")
            .bind(Uuid::parse_str(id).unwrap())
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
    assert_eq!(activity, 1);
    let missing = request(
        &app,
        Method::GET,
        &format!("/api/v1/platforms/{}/edge/status", Uuid::now_v7()),
        Some(fixture.administrator.clone()),
        None,
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

async fn request(
    app: &Router,
    method: Method,
    path: &str,
    actor: Option<ActorPrincipal>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(actor) = actor {
        request = request.extension(actor);
    }
    app.clone()
        .oneshot(
            request
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn edge_network_and_volume_endpoints_use_the_bound_session_and_enforce_permissions() {
    use citadel_adapters::edge::EdgeTarget;
    use citadel_contracts::citadel::{
        edge::v1::{EdgeCommandKind, core_envelope},
        networks::v1::{CreateNetworkRequest, CreateNetworkResponse, ListNetworksResponse},
        shared_models::v1::VolumeResponse,
        volumes::v1::CreateVolumeRequest,
    };
    use prost::Message;
    let fixture = fixture().await;
    let platform = fixture.platform_id;
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(platform)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let registry = &fixture.lookup_state.platforms.edge;
    let target = EdgeTarget::platform(platform);
    let (session, mut receiver) = registry.register(target.clone(), Uuid::now_v7()).unwrap();
    let mut denied = fixture.administrator.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert_eq!(
        request(
            &fixture.app,
            Method::POST,
            "/api/v1/volumes",
            Some(denied),
            Some(json!({"platformId":platform,"name":"data","driver":"local"}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert!(
        receiver.try_recv().is_err(),
        "denied requests must not reach Docker"
    );

    let calls = async {
        let list = request(
            &fixture.app,
            Method::GET,
            &format!("/api/v1/networks/{platform}"),
            Some(fixture.administrator.clone()),
            None,
        )
        .await;
        assert_eq!(list.status(), StatusCode::OK);
        let created = request(&fixture.app, Method::POST, "/api/v1/networks", Some(fixture.administrator.clone()), Some(json!({"platformId":platform,"name":"edge-net","driver":"bridge","scope":"local","labels":{"purpose":"test"}}))).await;
        assert_eq!(created.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(created.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(body["id"], "network-id");
        let created = request(&fixture.app, Method::POST, "/api/v1/volumes", Some(fixture.administrator.clone()), Some(json!({"platformId":platform,"name":"data","driver":"local","options":{"type":"tmpfs"}}))).await;
        assert_eq!(created.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(created.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(body["name"], "data");
    };
    let peer = async {
        for expected in [
            EdgeCommandKind::NetworkList,
            EdgeCommandKind::NetworkCreate,
            EdgeCommandKind::VolumeCreate,
        ] {
            let envelope = receiver.recv().await.unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected command")
            };
            assert_eq!(command.kind, expected as i32);
            let response = match expected {
                EdgeCommandKind::NetworkList => ListNetworksResponse::default().encode_to_vec(),
                EdgeCommandKind::NetworkCreate => {
                    let request = CreateNetworkRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(request.name, "edge-net");
                    assert_eq!(request.labels["purpose"], "test");
                    CreateNetworkResponse {
                        id: "network-id".into(),
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::VolumeCreate => {
                    let request = CreateVolumeRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(request.name, "data");
                    assert_eq!(request.options["type"], "tmpfs");
                    VolumeResponse {
                        name: "data".into(),
                        driver: "local".into(),
                        ..Default::default()
                    }
                    .encode_to_vec()
                }
                _ => unreachable!(),
            };
            session.output(id, response);
            session.complete(id, true);
        }
    };
    tokio::time::timeout(StdDuration::from_secs(10), async {
        tokio::join!(calls, peer);
    })
    .await
    .unwrap();
    registry.disconnect(&target);
    assert_eq!(
        request(
            &fixture.app,
            Method::GET,
            &format!("/api/v1/networks/{platform}"),
            Some(fixture.administrator.clone()),
            None
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
}
