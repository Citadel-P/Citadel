use super::*;
use citadel_adapters::edge::EdgeTarget;
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind, core_envelope},
    images::v1::{
        GetExposedPortsRequest, GetExposedPortsResponse, InspectImageRequest, InspectImageResponse,
    },
};
use prost::Message;

// Ports ImageEndpointTests and SwarmNodeLocalResourceEndpointTests' inspect
// contract, adding a wrong-node/no-fallback regression at the real HTTP boundary.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn image_inspect_authorizes_and_uses_the_exact_node_with_existing_ui_shape() {
    let f = fixture().await;
    let url = format!(
        "/api/v1/images/{}/sha256:abc?dockerNodeId=node-1",
        f.platform_id
    );
    assert_eq!(
        send(&f, &url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let principal = super::lookup::subject(&f).await;
    assert_eq!(
        send(&f, &url, Some(principal.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        citadel_domain::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let registry = &f.lookup_state.platforms.edge;
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (_other, mut other_commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "other-node".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (response, ()) = tokio::join!(send(&f, &url, Some(principal.clone())), async {
        let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("image command")
        };
        assert_eq!(command.kind, EdgeCommandKind::ImageInspect as i32);
        assert_eq!(
            InspectImageRequest::decode(command.payload.as_slice())
                .unwrap()
                .id,
            "sha256:abc"
        );
        let id = Uuid::parse_str(&command.command_id).unwrap();
        session.output(
            id,
            InspectImageResponse {
                id: "sha256:abc".into(),
                repo_tags: vec!["registry:5000/app:v1".into()],
                env: vec!["MODE=production".into()],
                exposed_ports: vec!["80/tcp".into()],
                ..Default::default()
            }
            .encode_to_vec(),
        );
        session.complete(id, true);
    });
    let status = response.status();
    let body = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "registry:5000/app");
    assert_eq!(body["tag"], "v1");
    assert_eq!(body["dockerNodeId"], "node-1");
    assert_eq!(body["exposedPorts"], json!(["80/tcp"]));
    assert_eq!(body["capabilities"]["canRead"], true);
    assert!(body["containers"].is_array() && body["layers"].is_array());
    assert!(other_commands.try_recv().is_err());
    // Port GetExposedPortsTests at the HTTP/SQL/transport boundary: the form's
    // Citadel image UUID must resolve to its Docker content ID on this node.
    let image_id = Uuid::now_v7();
    sqlx::query("INSERT INTO swarmnodeimageprojections(id,contentidentity,dockerimageid,dockernodeid,observedat,platformid,resource) VALUES($1,'sha256:abc','sha256:abc','node-1',now(),$2,'{}')")
        .bind(image_id).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let ports_url = format!("/api/v1/images/{}/{image_id}/_ports", f.platform_id);
    assert_eq!(
        send(&f, &ports_url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let (ports_response, ()) = tokio::join!(send(&f, &ports_url, Some(principal.clone())), async {
        let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("ports command")
        };
        assert_eq!(command.kind, EdgeCommandKind::ImageExposedPorts as i32);
        assert_eq!(
            GetExposedPortsRequest::decode(command.payload.as_slice())
                .unwrap()
                .id,
            "sha256:abc"
        );
        let command_id = Uuid::parse_str(&command.command_id).unwrap();
        session.output(
            command_id,
            GetExposedPortsResponse {
                ports: vec!["80/tcp".into()],
            }
            .encode_to_vec(),
        );
        session.complete(command_id, true);
    });
    assert_eq!(ports_response.status(), StatusCode::OK);
    assert_eq!(json_body(ports_response).await, json!({"ports":["80/tcp"]}));
    assert_eq!(
        send(
            &f,
            &ports_url.replace(&image_id.to_string(), &Uuid::nil().to_string()),
            Some(principal.clone())
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &f,
            &ports_url.replace(&f.platform_id.to_string(), &Uuid::now_v7().to_string()),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert!(other_commands.try_recv().is_err());
    assert_eq!(
        send(
            &f,
            &url.replace("node-1", "missing-node"),
            Some(principal.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE swarmnodeprojections SET isstale=true WHERE platformid=$1 AND dockernodeid='node-1'").bind(f.platform_id).execute(&f.pool).await.unwrap();
    assert_eq!(
        send(&f, &url, Some(principal)).await.status(),
        StatusCode::CONFLICT
    );
    assert!(commands.try_recv().is_err());
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}
