use super::*;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    edge::v1::{EdgeCommandKind, core_envelope},
    networks::v1::{InspectNetworkRequest, InspectNetworkResponse},
    shared_models::v1::VolumeResponse,
    volumes::v1::InspectVolumeRequest,
};
use citadel_server::realtime_groups::{Group, GroupReadPort};
use prost::Message;

// Ports SwarmNodeLocalResourceEndpointTests: node-scoped lists, exact-node
// inspection, permission checks, and the existing frontend snapshot contract.
#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn node_resources_are_authorized_routed_and_synchronized_without_a_manager_fallback() {
    let f = fixture().await;
    let network = RuntimeNetworkSummary {
        id: "node-bridge".into(),
        name: "node-bridge".into(),
        ..Default::default()
    };
    let volume = RuntimeVolumeSummary {
        name: "node-data".into(),
        ..Default::default()
    };
    sqlx::query("INSERT INTO swarmnodenetworkprojections(platformid,dockernodeid,dockernetworkid,resource,observedat) VALUES($1,'node-1','node-bridge',$2,now())")
        .bind(f.platform_id).bind(json!(network)).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodevolumeprojections(platformid,dockernodeid,volumename,resource,observedat) VALUES($1,'node-1','node-data',$2,now())")
        .bind(f.platform_id).bind(json!(volume)).execute(&f.pool).await.unwrap();
    for (resource, property) in [("networks", "networks"), ("volumes", "volumes")] {
        let path = format!("/api/v1/{resource}/{}", f.platform_id);
        assert_eq!(
            send(&f, &path, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        let result = json_body(send(&f, &path, Some(f.administrator.clone())).await).await;
        let node = result[property]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["dockerNodeId"] == "node-1")
            .unwrap();
        assert!(!node["isStale"].as_bool().unwrap());
        assert_eq!(node["capabilities"]["canRead"], true);
    }
    let groups = super::realtime_groups::reader(&f);
    let snapshot = groups
        .read(
            &f.administrator,
            &Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap(),
            None,
        )
        .await
        .unwrap();
    let event = snapshot
        .events
        .iter()
        .find(|e| e.target == "SwarmNodeLocalResourcesUpdated")
        .unwrap();
    assert_eq!(event.arguments[0]["platformId"], json!(f.platform_id));
    assert_eq!(event.arguments[0]["networks"][0]["dockerNodeId"], "node-1");
    assert_eq!(
        event.arguments[0]["volumes"].as_array().unwrap().len(),
        2,
        "keep connected-manager Volumes in the full snapshot"
    );

    let registry = &f.edge;
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
    for (resource, id, kind) in [
        ("networks", "node-bridge", EdgeCommandKind::NetworkInspect),
        ("volumes", "node-data", EdgeCommandKind::VolumeInspect),
    ] {
        let url = format!(
            "/api/v1/{resource}/{}/{id}?dockerNodeId=node-1",
            f.platform_id
        );
        let mut denied = f.administrator.clone();
        denied.roles.clear();
        denied.actor_id = ActorId::new(Uuid::now_v7());
        assert_eq!(
            send(&f, &url, Some(denied)).await.status(),
            StatusCode::FORBIDDEN
        );
        assert!(commands.try_recv().is_err());
        let (response, ()) = tokio::join!(send(&f, &url, Some(f.administrator.clone())), async {
            let command = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
                .await
                .unwrap()
                .unwrap();
            let command_id = Uuid::parse_str(&command.command_id).unwrap();
            let Some(core_envelope::Body::Command(request)) = command.body else {
                panic!("expected command")
            };
            assert_eq!(request.kind, kind as i32);
            let response = if kind == EdgeCommandKind::NetworkInspect {
                assert_eq!(
                    InspectNetworkRequest::decode(request.payload.as_slice())
                        .unwrap()
                        .id,
                    id
                );
                InspectNetworkResponse {
                    id: id.into(),
                    name: id.into(),
                    ..Default::default()
                }
                .encode_to_vec()
            } else {
                assert_eq!(
                    InspectVolumeRequest::decode(request.payload.as_slice())
                        .unwrap()
                        .name,
                    id
                );
                VolumeResponse {
                    name: id.into(),
                    ..Default::default()
                }
                .encode_to_vec()
            };
            session.output(command_id, response);
            session.complete(command_id, true);
        });
        let inspected = json_body(response).await;
        assert_eq!(inspected["dockerNodeId"], "node-1");
        assert!(other_commands.try_recv().is_err());
    }
    registry.remove(&session);
    assert_eq!(
        send(
            &f,
            &format!(
                "/api/v1/volumes/{}/node-data?dockerNodeId=node-1",
                f.platform_id
            ),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}
