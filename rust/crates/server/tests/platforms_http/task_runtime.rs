use super::*;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    containers::v1::InspectContainerRequest,
    edge::v1::{EdgeCommandKind, core_envelope},
    shared_models::v1::{ContainerConfig, InspectContainerResponse},
};
use citadel_primitives::{ResourceType, SpecificPermission};
use prost::Message;

// Ports SwarmEndpointTests' Task runtime authorization, exact identity and
// redaction cases through HTTP + PostgreSQL + the real Edge command protocol.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn task_inspection_and_terminal_preserve_permissions_node_identity_and_redaction() {
    let f = fixture().await;
    let base = format!("/api/v1/platforms/{}/swarm/tasks/task-1", f.platform_id);
    let reader = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    for operation in ["inspect", "terminal"] {
        let url = format!("{base}/{operation}");
        assert_eq!(
            send(&f, &url, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&f, &url, Some(reader.clone())).await.status(),
            StatusCode::FORBIDDEN
        );
    }
    sqlx::query(
        "UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2 AND resourceid=$3",
    )
    .bind(SpecificPermission::Terminal as i32)
    .bind(reader.actor_id.value())
    .bind(f.platform_id)
    .execute(&f.pool)
    .await
    .unwrap();
    let terminal = send(&f, &format!("{base}/terminal"), Some(reader.clone())).await;
    let status = terminal.status();
    let terminal = json_body(terminal).await;
    assert_eq!(status, StatusCode::OK, "{terminal}");
    assert_eq!(terminal, json!({"dockerContainerId":"container-1"}));
    assert_eq!(
        send(&f, &format!("{base}/inspect"), Some(reader.clone()))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query(
        "UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2 AND resourceid=$3",
    )
    .bind(SpecificPermission::Inspect as i32)
    .bind(reader.actor_id.value())
    .bind(f.platform_id)
    .execute(&f.pool)
    .await
    .unwrap();
    assert_eq!(
        send(&f, &format!("{base}/terminal"), Some(reader.clone()))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let registry = &f.lookup_state.platforms.edge;
    let (session, mut commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (other, mut other_commands) = registry
        .register(
            EdgeTarget::node(f.platform_id, "other".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let url = format!("{base}/inspect");
    let (response, ()) = tokio::join!(send(&f, &url, Some(reader.clone())), async {
        let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("expected inspection")
        };
        assert_eq!(command.kind, EdgeCommandKind::ContainerInspect as i32);
        let request = InspectContainerRequest::decode(command.payload.as_slice()).unwrap();
        assert_eq!(request.container_id, "container-1");
        assert!(other_commands.try_recv().is_err());
        let command_id = Uuid::parse_str(&envelope.command_id).unwrap();
        session.output(
            command_id,
            InspectContainerResponse {
                id: "container-1".into(),
                config: Some(ContainerConfig {
                    env: vec!["API_TOKEN=private".into(), "LOG_LEVEL=info".into()],
                    ..Default::default()
                }),
                ..Default::default()
            }
            .encode_to_vec(),
        );
        session.complete(command_id, true);
    });
    let status = response.status();
    let body = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["id"], "container-1");
    assert_eq!(
        body["config"]["env"],
        json!(["API_TOKEN=********", "LOG_LEVEL=info"])
    );
    for (column, value) in [("state", "Shutdown"), ("dockernodeid", "replacement")] {
        let sql = if column == "state" {
            "UPDATE swarmtaskprojections SET state=$2 WHERE platformid=$1"
        } else {
            "UPDATE swarmtaskprojections SET state='Running',dockernodeid=$2 WHERE platformid=$1"
        };
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(f.platform_id)
            .bind(value)
            .execute(&f.pool)
            .await
            .unwrap();
        assert_eq!(
            send(&f, &url, Some(reader.clone())).await.status(),
            StatusCode::CONFLICT
        );
        assert!(
            commands.try_recv().is_err(),
            "must not inspect a replacement target"
        );
    }
    assert_eq!(
        send(&f, &url.replace("task-1", "missing"), Some(reader.clone()))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &url, Some(reader)).await.status(),
        StatusCode::CONFLICT
    );
    registry.remove(&session);
    registry.remove(&other);
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}
