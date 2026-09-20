use super::*;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    containers::v1::{ContainerLogRequest, ContainerLogResponse},
    edge::v1::{EdgeCommandKind, core_envelope},
};
use prost::Message;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn managed_service_logs_require_both_service_logs_and_parent_platform_access() {
    let f = fixture().await;
    let principal = super::lookup::subject(&f).await;
    let id = Uuid::now_v7();
    let spec:citadel_swarm_services::SwarmServiceSpec=serde_json::from_value(json!({"image":{"$type":"External","registryId":Uuid::now_v7(),"imageTag":"redis"},"replicas":1})).unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,dockerserviceid,platformid,createdbyactorid,desiredspechash,health,spec,synchronizationstate,updatedat,autoupdatestate_status) VALUES($1,$2,$2,'service-1',$3,$4,'hash','Healthy',$5,'InSync',now(),'Unknown')")
        .bind(id).bind(format!("logs-{id}")).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).bind(spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    let url = format!("/api/v1/swarmServices/{id}/logs");
    assert_eq!(
        send(&f, &url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        citadel_primitives::ResourceType::SwarmService,
        id,
        0,
    )
    .await;
    assert_eq!(
        send(&f, &url, Some(principal.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query(
        "UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2 AND resourceid=$3",
    )
    .bind(citadel_primitives::SpecificPermission::Logs as i32)
    .bind(principal.actor_id.value())
    .bind(id)
    .execute(&f.pool)
    .await
    .unwrap();
    assert_eq!(
        send(&f, &url, Some(principal.clone())).await.status(),
        StatusCode::NOT_FOUND
    );
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let response = send(&f, &url, Some(principal.clone())).await;
    let status = response.status();
    let body = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["truncated"], false);
    sqlx::query("UPDATE swarmservices SET dockerserviceid=NULL WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &url, Some(principal)).await.status(),
        StatusCode::CONFLICT
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

// Ports SwarmEndpointTests' log permission, bounded-tail, projected ownership,
// and bounded connector result cases, with a real HTTP router and PostgreSQL.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn logs_authorize_validate_and_route_current_tasks_to_the_exact_node() {
    let f = fixture().await;
    let service = format!(
        "/api/v1/platforms/{}/swarm/services/service-1/logs",
        f.platform_id
    );
    let task = format!(
        "/api/v1/platforms/{}/swarm/tasks/task-1/logs",
        f.platform_id
    );
    let reader = ActorPrincipal {
        subject_id: f.actor_id,
        actor_id: ActorId::new(f.actor_id),
        name: "reader".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    for url in [&service, &task] {
        assert_eq!(send(&f, url, None).await.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            send(&f, url, Some(reader.clone())).await.status(),
            StatusCode::FORBIDDEN
        );
        for tail in ["0", "201", "-1", "no"] {
            assert_eq!(
                send(
                    &f,
                    &format!("{url}?tail={tail}"),
                    Some(f.administrator.clone())
                )
                .await
                .status(),
                StatusCode::BAD_REQUEST
            );
        }
    }
    sqlx::query(
        "UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2 AND resourceid=$3",
    )
    .bind(citadel_primitives::SpecificPermission::Logs as i32)
    .bind(f.actor_id)
    .bind(f.platform_id)
    .execute(&f.pool)
    .await
    .unwrap();
    let response = send(&f, &format!("{service}?tail=25"), Some(reader.clone())).await;
    let status = response.status();
    let value = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(
        value["lines"],
        json!(["2026-09-06T12:00:00Z task.name=web.1 héllo\n"])
    );
    assert_eq!(value["truncated"], false);
    assert_eq!(
        send(
            &f,
            &service.replace("service-1", "foreign-service"),
            Some(reader.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
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
            EdgeTarget::node(f.platform_id, "other-node".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let task_with_tail = format!("{task}?tail=12");
    let (response, ()) = tokio::join!(send(&f, &task_with_tail, Some(reader.clone())), async {
        let envelope = tokio::time::timeout(StdDuration::from_secs(3), commands.recv())
            .await
            .unwrap()
            .unwrap();
        let Some(core_envelope::Body::Command(command)) = envelope.body else {
            panic!("expected logs command")
        };
        assert_eq!(command.kind, EdgeCommandKind::ContainerLogsStream as i32);
        let request = ContainerLogRequest::decode(command.payload.as_slice()).unwrap();
        assert_eq!(request.container_id, "container-1");
        assert_eq!(request.tail, 12);
        assert_eq!(request.follow, Some(false));
        let id = Uuid::parse_str(&command.command_id).unwrap();
        // The character crosses Agent frames; decoding each frame would corrupt it.
        for bytes in [b"h\xc3".to_vec(), b"\xa9llo\n".to_vec()] {
            session.output(id, ContainerLogResponse { log: bytes }.encode_to_vec());
        }
        session.complete(id, true);
    });
    let status = response.status();
    let value = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["lines"], json!(["héllo\n"]));
    assert!(other_commands.try_recv().is_err());
    sqlx::query("UPDATE swarmtaskprojections SET dockernodeid='other-node' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &task, Some(reader.clone())).await.status(),
        StatusCode::CONFLICT
    );
    assert!(other_commands.try_recv().is_err());
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &service, Some(reader)).await.status(),
        StatusCode::CONFLICT
    );
    registry.remove(&session);
    registry.remove(&other);
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}
