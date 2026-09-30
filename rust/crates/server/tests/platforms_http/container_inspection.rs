use super::*;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
use citadel_contracts::citadel::{
    containers::v1::InspectContainerRequest,
    edge::v1::{EdgeCommandKind, core_envelope},
    shared_models::v1::{ContainerConfig, InspectContainerResponse},
};
use citadel_identity::{ResourceAccessInput, UserPatchMutation, UserRepository};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};

async fn set_inspect_access(
    f: &Fixture,
    reader: &ActorPrincipal,
    kind: ResourceType,
    id: Uuid,
    inspect: bool,
) {
    // ACL changes must pass through the repository to invalidate cached permissions.
    PostgresUserRepository::new(f.pool.clone())
        .patch(
            reader.subject_id,
            &UserPatchMutation {
                resource_accesses: Some(vec![ResourceAccessInput {
                    resource_type: kind,
                    resource_id: id,
                    permission_level: PermissionLevel::Read,
                    specific_permissions: if inspect {
                        vec![SpecificPermission::Inspect]
                    } else {
                        vec![]
                    },
                }]),
                ..Default::default()
            },
            f.administrator.actor_id,
            Utc::now(),
            true,
        )
        .await
        .unwrap();
}
use citadel_server::realtime_groups::{Group, GroupReadPort};
use prost::Message;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn inspection_resolves_ui_ids_enforces_inspect_permission_and_routes_to_the_owning_node() {
    let mut f = fixture().await;
    let id = Uuid::now_v7();
    let docker_id = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    sqlx::query("INSERT INTO containers(id,dockercontainerid,dockerimageid,name,platformid,created,updated,state,ports) VALUES($1,$2,'sha256:fixture','inspected',$3,1,1,'Running','{}')")
        .bind(id).bind(&docker_id).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let socket = std::env::temp_dir().join(format!("inspect-{}.sock", Uuid::now_v7()));
    let listener = UnixListener::bind(&socket).unwrap();
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let calls = requests.clone();
    let document = Arc::new(tokio::sync::RwLock::new(
        json!({"Id":docker_id, "Created":"2026-01-01", "Name":"/inspected", "Config":{"Image":"busybox:latest", "Env":["API_TOKEN=not-for-browser","LOG_LEVEL=info"]},"State":{"Status":"running","StartedAt":"2026-01-01T12:00:00Z"},
            "Mounts":[{"Name":"data","Destination":"/data"},{"Source":"/host","Destination":"/bind"}],
            "HostConfig":{"PortBindings":{"80/tcp":[{"HostIp":"127.0.0.1","HostPort":"8080"}],"53/udp":null}},
            "NetworkSettings":{"Networks":{"bridge":{"NetworkID":"network-id"},"fallback":{"NetworkID":""}}}}),
    ));
    let inspected = document.clone();
    let docker = DockerClient::new(&socket, StdDuration::from_secs(2)).unwrap();
    let expected = format!("GET /v1.49/containers/{docker_id}/json ");
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0 && bytes.len() < 16384);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            calls
                .lock()
                .await
                .push(request.lines().next().unwrap().into());
            let body = if request.starts_with("GET /version ") {
                json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})
            } else {
                assert!(request.starts_with(&expected), "{request}");
                inspected.read().await.clone()
            }
            .to_string();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });
    let mut state = f.lookup_state.platforms.clone();
    state.docker = docker;
    f.app = platforms_http::router(state);
    let url = format!("/api/v1/containers/{docker_id}/inspect");
    assert_eq!(
        send(&f, &url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let reader = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    assert_eq!(
        send(&f, &url, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    assert!(requests.lock().await.is_empty());
    set_inspect_access(&f, &reader, ResourceType::Platform, f.platform_id, true).await;
    for reference in [&docker_id, &docker_id[..12].to_string(), &id.to_string()] {
        let response = send(
            &f,
            &format!("/api/v1/containers/{reference}/inspect"),
            Some(reader.clone()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()["cache-control"]
                .to_str()
                .unwrap()
                .contains("no-store")
        );
        let result = json_body(response).await;
        assert_eq!(result["id"], docker_id);
        assert_eq!(result["state"]["status"], "Running");
        assert_eq!(result["config"]["image"], "busybox:latest");
        assert_eq!(
            result["config"]["env"],
            json!(["API_TOKEN=********", "LOG_LEVEL=info"])
        );
        assert!(!result.to_string().contains("not-for-browser"));
    }
    // The /info and /data routes require Read, not the raw Inspect grant.
    let info_reader = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        info_reader.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    for reference in [&docker_id, &id.to_string()] {
        let info_response = send(
            &f,
            &format!("/api/v1/containers/{reference}/info"),
            Some(info_reader.clone()),
        )
        .await;
        assert_eq!(info_response.status(), StatusCode::OK);
        let info = json_body(info_response).await;
        assert_eq!(info["containerId"], docker_id);
        assert_eq!(info["volumes"], json!(["data"]));
        assert_eq!(
            info["networks"],
            json!({"bridge":"network-id","fallback":"fallback"})
        );
        assert_eq!(info["ports"]["80/tcp"][0]["hostPort"], "8080");
        assert!(info["ports"].as_object().unwrap().contains_key("53/udp"));
        assert!(info["ports"]["53/udp"].is_null());
        assert_eq!(info["startedAt"], "2026-01-01T12:00:00Z");
        assert_eq!(info["capabilities"]["canRead"], true);
        assert!(!info.to_string().contains("not-for-browser"));
        let data = json_body(
            send(
                &f,
                &format!("/api/v1/containers/{reference}/data"),
                Some(info_reader.clone()),
            )
            .await,
        )
        .await;
        assert_eq!(data["id"], docker_id);
        assert_eq!(data["capabilities"]["canRead"], true);
        assert!(data["containerStat"].is_null());
    }
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,spec,status) VALUES($1,$2,$3,$4,'{}','Running')")
        .bind(deployment).bind(format!("inspection-{deployment}")).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$2 WHERE id=$1")
        .bind(id)
        .bind(deployment)
        .execute(&f.pool)
        .await
        .unwrap();
    let deployment_reader = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        deployment_reader.actor_id.value(),
        ResourceType::Deployment,
        deployment,
        0,
    )
    .await;
    let deployment_info = format!("/api/v1/deployments/{deployment}/info");
    let deployment_inspect = format!("/api/v1/deployments/{deployment}/inspect");
    assert_eq!(
        send(&f, &deployment_info, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &deployment_info, Some(reader.clone()))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let info = send(&f, &deployment_info, Some(deployment_reader.clone())).await;
    assert_eq!(info.status(), StatusCode::OK);
    assert_eq!(json_body(info).await["containerId"], docker_id);
    assert_eq!(
        send(&f, &deployment_inspect, Some(deployment_reader.clone()))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    set_inspect_access(
        &f,
        &deployment_reader,
        ResourceType::Deployment,
        deployment,
        true,
    )
    .await;
    let inspected = send(&f, &deployment_inspect, Some(deployment_reader.clone())).await;
    assert_eq!(inspected.status(), StatusCode::OK);
    assert!(
        !json_body(inspected)
            .await
            .to_string()
            .contains("not-for-browser")
    );
    sqlx::query("UPDATE containers SET deploymentid=NULL WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &deployment_info, Some(deployment_reader))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );

    for reference in [
        "short",
        "not-a-container",
        "00000000-0000-0000-0000-000000000000",
    ] {
        assert_eq!(
            send(
                &f,
                &format!("/api/v1/containers/{reference}/inspect"),
                Some(reader.clone())
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        send(
            &f,
            &format!("/api/v1/containers/{}/inspect", Uuid::now_v7()),
            Some(reader.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let stack = Uuid::now_v7();
    let spec: citadel_stacks::StackSpec =
        serde_json::from_value(json!({"$type":"WebEditor","composeFile":"services: {}"})).unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$2,$3,'WebEditor',$4,$5)")
        .bind(stack).bind(format!("inspection-{stack}")).bind(SYSTEM_ACTOR_ID)
        .bind(citadel_stacks::StackDriftPolicy::default().to_storage_value().unwrap())
        .bind(citadel_stacks::StackUpdateState::new(&spec).to_storage_value().unwrap())
        .execute(&f.pool).await.unwrap();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')")
        .bind(release).bind(stack).bind(f.platform_id).bind(SYSTEM_ACTOR_ID)
        .bind(spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE containers SET stackid=$2 WHERE id=$1")
        .bind(id)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    let scoped_url = format!("/api/v1/stacks/{stack}/containers/{docker_id}/inspect");
    let scoped_reader = super::lookup::subject(&f).await;
    let data_url = format!("/api/v1/stacks/{stack}/data");
    assert_eq!(
        send(&f, &data_url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &data_url, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    let before = requests.lock().await.len();
    assert_eq!(
        send(&f, &scoped_url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    // Platform Inspect alone must not grant Stack Inspect.
    assert_eq!(
        send(&f, &scoped_url, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    super::lookup::grant(
        &f,
        scoped_reader.actor_id.value(),
        ResourceType::Stack,
        stack,
        0,
    )
    .await;
    let data_response = send(&f, &data_url, Some(scoped_reader.clone())).await;
    assert_eq!(data_response.status(), StatusCode::OK);
    let data = json_body(data_response).await;
    assert_eq!(data["containers"].as_array().unwrap().len(), 1);
    assert_eq!(data["containers"][0]["id"], docker_id);
    assert_eq!(data["containers"][0]["stackId"], stack.to_string());
    let realtime = super::realtime_groups::reader(&f);
    let stack_snapshot = realtime
        .read(
            &scoped_reader,
            &Group::parse(&format!("stack-info:{stack}")).unwrap(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        stack_snapshot.events[0].target,
        "ReceiveStackContainersInfo"
    );
    assert_eq!(stack_snapshot.events[0].arguments[0], data["containers"]);
    let container_snapshot = realtime
        .read(
            &scoped_reader,
            &Group::parse(&format!("container-info:{id}")).unwrap(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(container_snapshot.events[0].target, "ReceiveContainerInfo");
    assert_eq!(
        container_snapshot.events[0].arguments[0],
        data["containers"][0]
    );

    assert_eq!(
        send(&f, &scoped_url, Some(scoped_reader.clone()))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(requests.lock().await.len(), before);
    set_inspect_access(&f, &scoped_reader, ResourceType::Stack, stack, true).await;
    // This route requires Stack Read + Inspect, without Platform ACL.
    let scoped = json_body(send(&f, &scoped_url, Some(scoped_reader.clone())).await).await;
    assert_eq!(
        scoped["config"]["env"],
        json!(["API_TOKEN=********", "LOG_LEVEL=info"])
    );
    let before = requests.lock().await.len();
    for path in [
        format!(
            "/api/v1/stacks/{}/containers/{docker_id}/inspect",
            Uuid::now_v7()
        ),
        format!(
            "/api/v1/stacks/{stack}/containers/{}/inspect",
            Uuid::now_v7()
        ),
    ] {
        assert_eq!(
            send(&f, &path, Some(f.administrator.clone()))
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    sqlx::query("UPDATE containers SET stackid=NULL WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &scoped_url, Some(scoped_reader.clone()))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let data = json_body(send(&f, &data_url, Some(scoped_reader.clone())).await).await;
    assert_eq!(data["containers"], json!([]));
    for invalid in ["not-a-uuid", "00000000-0000-0000-0000-000000000000"] {
        assert_eq!(
            send(
                &f,
                &format!("/api/v1/stacks/{invalid}/data"),
                Some(f.administrator.clone())
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(requests.lock().await.len(), before);
    sqlx::query("UPDATE containers SET stackid=$2 WHERE id=$1")
        .bind(id)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    document.write().await["Id"] = json!("different");
    assert_eq!(
        send(&f, &url, Some(reader.clone())).await.status(),
        StatusCode::CONFLICT
    );

    // Availability and current permissions must be checked before Docker I/O.
    let calls_before = requests.lock().await.len();
    sqlx::query("UPDATE containers SET projectionstalesince=1 WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &url, Some(reader.clone())).await.status(),
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE containers SET projectionstalesince=NULL WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &url, Some(reader.clone())).await.status(),
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE platforms SET status='Online' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    set_inspect_access(&f, &reader, ResourceType::Platform, f.platform_id, false).await;
    assert_eq!(
        send(&f, &url, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(requests.lock().await.len(), calls_before);
    set_inspect_access(&f, &reader, ResourceType::Platform, f.platform_id, true).await;

    // Worker inspection must use its node session, never the connected manager.
    sqlx::query("UPDATE containers SET dockernodeid='node-1' WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE platforms SET platformdescriptor=jsonb_set(platformdescriptor::jsonb,'{nodeID}','\"other-manager\"') WHERE id=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    let registry = &f.lookup_state.platforms.edge;
    let (session, mut outbound) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let calls_before = requests.lock().await.len();
    for (path, principal) in [(&url, &reader), (&scoped_url, &scoped_reader)] {
        let (response, ()) = tokio::join!(send(&f, path, Some(principal.clone())), async {
            let envelope = outbound.recv().await.unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected inspection command")
            };
            assert_eq!(command.kind, EdgeCommandKind::ContainerInspect as i32);
            assert_eq!(
                InspectContainerRequest::decode(command.payload.as_slice())
                    .unwrap()
                    .container_id,
                docker_id
            );
            let command_id = Uuid::parse_str(&command.command_id).unwrap();
            session.output(
                command_id,
                InspectContainerResponse {
                    id: docker_id.clone(),
                    config: Some(ContainerConfig {
                        image: Some("busybox:latest".into()),
                        env: vec!["API_KEY=worker-private".into()],
                        ..Default::default()
                    }),
                    ..Default::default()
                }
                .encode_to_vec(),
            );
            session.complete(command_id, true);
        });
        assert_eq!(response.status(), StatusCode::OK);
        let result = json_body(response).await;
        assert_eq!(result["config"]["image"], "busybox:latest");
        assert_eq!(result["config"]["env"], json!(["API_KEY=********"]));
        assert_eq!(requests.lock().await.len(), calls_before);
    }
    registry.disconnect_platform(f.platform_id);
    assert_eq!(
        send(&f, &url, Some(reader)).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(requests.lock().await.len(), calls_before);
    server.abort();
    f.docker_server.abort();
    std::fs::remove_file(socket).unwrap();
}
