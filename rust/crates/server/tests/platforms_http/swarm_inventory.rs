use super::*;
use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
use citadel_identity::{ResourceAccessInput, UserPatchMutation, UserRepository};
use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};

// Ports SwarmEndpointTests: node availability and labels, metadata-only Secrets,
// immutable Config content, inspect permissions, whole-batch preflight, stale
// versions, managed Service guards and committed partial-failure recovery.
#[derive(Default)]
pub(super) struct DockerState {
    nodes: BTreeMap<String, Value>,
    pub(super) services: BTreeMap<String, Value>,
    secrets: BTreeMap<String, Value>,
    configs: BTreeMap<String, Value>,
    pub(super) mutations: Vec<(String, Value)>,
    rejected_delete: Option<String>,
    fail_after_update: bool,
    cluster: String,
}
pub(super) async fn fixture_swarm() -> (Fixture, Arc<Mutex<DockerState>>) {
    let mut f = fixture().await;
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(&f.docker_socket).await;
    let cluster = format!("cluster-{}", f.platform_id);
    sqlx::query("UPDATE platforms SET clusterid=$2,platformdescriptor=jsonb_build_object('$type','DockerSwarm','nodeID','node-1','daemonId','daemon-test','controlAvailable',true) WHERE id=$1").bind(f.platform_id).bind(&cluster).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE swarmsecretprojections SET servicenames='[]' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmconfigprojections SET servicenames='[]' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let state = Arc::new(Mutex::new(DockerState {
        nodes: BTreeMap::from([(
            "node-1".into(),
            json!({"ID":"node-1","Version":{"Index":1},"Spec":{"Role":"manager","Availability":"active","Labels":{"zone":"a"}},"Description":{"Hostname":"manager","Platform":{"OS":"linux","Architecture":"amd64"}},"Status":{"State":"ready"},"ManagerStatus":{"Leader":true,"Reachability":"reachable"}}),
        )]),
        services: BTreeMap::from([(
            "service-1".into(),
            json!({"ID":"service-1","Version":{"Index":1},"Spec":{"Name":"redis","Mode":{"Replicated":{"Replicas":1}},"TaskTemplate":{"ContainerSpec":{"Image":"redis:latest","Env":["TOKEN=do-not-return"]},"ForceUpdate":0},"Labels":{}},"UpdateStatus":{"State":"completed"}}),
        )]),
        configs: BTreeMap::from([(
            "config-1".into(),
            json!({"ID":"config-1","Version":{"Index":1},"Spec":{"Name":"config","Data":"c2V0dGluZz10cnVl","Labels":{}}}),
        )]),
        secrets: BTreeMap::from([(
            "secret-1".into(),
            json!({"ID":"secret-1","Version":{"Index":1},"Spec":{"Name":"secret","Labels":{}}}),
        )]),
        cluster,
        ..Default::default()
    }));
    let listener = UnixListener::bind(&f.docker_socket).unwrap();
    let shared = state.clone();
    f.docker_server = tokio::spawn(async move {
        loop {
            let (mut connection, _) = listener.accept().await.unwrap();
            let shared = shared.clone();
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                let (end, length) = loop {
                    let n = connection.read(&mut buffer).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&bytes[..end]);
                        let len = header
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|s| s.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        break (end + 4, len);
                    }
                };
                while bytes.len() < end + length {
                    let n = connection.read(&mut buffer).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let head = String::from_utf8_lossy(&bytes[..end]);
                let mut parts = head.lines().next().unwrap().split_whitespace();
                let method = parts.next().unwrap();
                let path = parts.next().unwrap().split('?').next().unwrap();
                let body = if length == 0 {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes[end..end + length]).unwrap()
                };
                let (status, value) = reply(&mut *shared.lock().await, method, path, body);
                let body = if status == 204 {
                    String::new()
                } else {
                    value.to_string()
                };
                let response = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                connection.write_all(response.as_bytes()).await.unwrap();
                connection.shutdown().await.unwrap();
            });
        }
    });
    f.docker = DockerClient::new(&f.docker_socket, StdDuration::from_secs(2)).unwrap();
    let mut http_state = f.lookup_state.platforms.clone();
    refresh_runtime(&mut http_state, &f, f.docker.clone());
    f.lookup_state.platforms = http_state;
    f.app = platforms_http::router(f.lookup_state.platforms.clone());
    (f, state)
}
fn reply(state: &mut DockerState, method: &str, path: &str, body: Value) -> (u16, Value) {
    if path == "/version" {
        return (
            200,
            json!({"Version":"28.0","ApiVersion":"1.49","MinAPIVersion":"1.41"}),
        );
    }
    let path = path.strip_prefix("/v1.49/").unwrap();
    match path {
        "info" => {
            return (
                200,
                json!({"ID":"daemon-test","NCPU":1,"MemTotal":1048576,"OSType":"linux","Swarm":{"NodeID":"node-1","LocalNodeState":"active","ControlAvailable":true,"Nodes":1,"Managers":1,"Cluster":{"ID":state.cluster}}}),
            );
        }
        "containers/json" | "images/json" | "networks" => return (200, json!([])),
        "volumes" => return (200, json!({"Volumes":[]})),
        "system/df" => return (200, json!({"Volumes":[]})),
        "tasks" => {
            return (
                200,
                json!([{"ID":"task-1","ServiceID":"service-1","NodeID":"node-1","DesiredState":"running","Status":{"State":"running"},"Spec":{"ContainerSpec":{"Image":"redis"}}}]),
            );
        }
        _ => {}
    }
    if method != "GET" {
        state.mutations.push((path.into(), body.clone()));
    }
    let fields: Vec<_> = path.split('/').collect();
    let kind = fields[0];
    let collection = match kind {
        "nodes" => &mut state.nodes,
        "services" => &mut state.services,
        "secrets" => &mut state.secrets,
        "configs" => &mut state.configs,
        _ => panic!("unexpected {method} {path}"),
    };
    if fields.len() == 1 {
        return (200, Value::Array(collection.values().cloned().collect()));
    }
    if fields[1] == "create" {
        let id = format!(
            "{}-{}",
            kind.trim_end_matches('s'),
            body["Name"].as_str().unwrap()
        );
        let mut persisted = body.clone();
        if kind == "secrets" {
            persisted.as_object_mut().unwrap().remove("Data");
        }
        collection.insert(
            id.clone(),
            json!({"ID":id,"Version":{"Index":1},"Spec":persisted}),
        );
        return (201, json!({"ID":id}));
    }
    let id = fields[1];
    if method == "DELETE" {
        if state.rejected_delete.as_deref() == Some(id) {
            return (409, json!({"message":"resource is in use"}));
        }
        return if collection.remove(id).is_some() {
            (204, Value::Null)
        } else {
            (404, json!({"message":"not found"}))
        };
    }
    let Some(value) = collection.get_mut(id) else {
        return (404, json!({"message":"not found"}));
    };
    if fields.len() == 3 && fields[2] == "update" {
        if kind == "configs" {
            assert_eq!(
                value["Spec"]["Data"], body["Data"],
                "Config labels must preserve immutable content"
            );
        }
        value["Spec"] = body;
        value["Version"]["Index"] = json!(value["Version"]["Index"].as_u64().unwrap() + 1);
        return if state.fail_after_update {
            (500, json!({"message":"ambiguous accepted update"}))
        } else {
            (200, json!({}))
        };
    }
    (200, value.clone())
}
async fn assert_status(response: axum::response::Response, status: StatusCode) {
    let actual = response.status();
    let text = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(actual, status, "{}", String::from_utf8_lossy(&text));
}
fn url(f: &Fixture, path: &str) -> String {
    format!("/api/v1/platforms/{}/swarm/{path}", f.platform_id)
}
async fn cleanup(f: Fixture) {
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn native_swarm_mutations_persist_nodes_materials_and_restarted_services() {
    let (f, runtime) = fixture_swarm().await;
    let mut older_snapshot = snapshot(f.platform_id);
    older_snapshot.info.swarm.as_mut().unwrap().node_id = "node-1".into();
    assert_status(
        send_json(
            &f,
            Method::PATCH,
            &url(&f, "nodes/node-1"),
            f.administrator.clone(),
            json!({"versionIndex":1,"availability":"Drain","labels":{"zone":"maintenance"}}),
        )
        .await,
        StatusCode::NO_CONTENT,
    )
    .await;
    let node =
        json_body(send(&f, &url(&f, "nodes/node-1"), Some(f.administrator.clone())).await).await;
    assert_eq!(node["availability"], "Drain");
    assert_eq!(node["versionIndex"], 2);
    assert_eq!(node["labels"]["zone"], "maintenance");
    PostgresInventoryProjectionStore::new(f.pool.clone())
        .persist(&older_snapshot)
        .await
        .unwrap();
    let unchanged =
        json_body(send(&f, &url(&f, "nodes/node-1"), Some(f.administrator.clone())).await).await;
    assert_eq!(unchanged["versionIndex"], 2);
    assert_eq!(unchanged["availability"], "Drain");
    assert_status(
        send_json(
            &f,
            Method::PATCH,
            &url(&f, "nodes/availability"),
            f.administrator.clone(),
            json!({"availability":"Pause","nodes":[{"nodeId":"node-1","versionIndex":2}]}),
        )
        .await,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        runtime.lock().await.nodes["node-1"]["Spec"]["Labels"]["zone"],
        "maintenance"
    );
    for kind in ["secrets", "configs"] {
        assert_status(
            send_json(
                &f,
                Method::POST,
                &url(&f, kind),
                f.administrator.clone(),
                json!({"name":"new","data":"sensitive-value","labels":{"team":"ops"}}),
            )
            .await,
            StatusCode::NO_CONTENT,
        )
        .await;
        let id = format!("{}-new", kind.trim_end_matches('s'));
        let detail = json_body(
            send(
                &f,
                &url(&f, &format!("{kind}/{id}")),
                Some(f.administrator.clone()),
            )
            .await,
        )
        .await;
        assert!(detail.get("data").is_none());
        assert!(!detail.to_string().contains("sensitive-value"));
        assert_eq!(detail["labels"]["team"], "ops");
        assert_status(
            send_json(
                &f,
                Method::PATCH,
                &url(&f, &format!("{kind}/{id}/labels")),
                f.administrator.clone(),
                json!({"versionIndex":1,"labels":{"team":"platform"},"data":"must-not-replace"}),
            )
            .await,
            StatusCode::NO_CONTENT,
        )
        .await;
        let detail = json_body(
            send(
                &f,
                &url(&f, &format!("{kind}/{id}")),
                Some(f.administrator.clone()),
            )
            .await,
        )
        .await;
        assert_eq!(detail["versionIndex"], 2);
        assert_eq!(detail["labels"]["team"], "platform");
        if kind == "configs" {
            let data = json_body(
                send(
                    &f,
                    &url(&f, &format!("configs/{id}/content")),
                    Some(f.administrator.clone()),
                )
                .await,
            )
            .await;
            assert_eq!(data["content"], "sensitive-value");
        }
        assert_status(
            send_json(
                &f,
                Method::DELETE,
                &url(&f, kind),
                f.administrator.clone(),
                json!({"ids":[id]}),
            )
            .await,
            StatusCode::NO_CONTENT,
        )
        .await;
        assert_status(
            send(
                &f,
                &url(&f, &format!("{kind}/{id}")),
                Some(f.administrator.clone()),
            )
            .await,
            StatusCode::NOT_FOUND,
        )
        .await;
    }
    let inspected = json_body(
        send(
            &f,
            &url(&f, "services/service-1/inspect"),
            Some(f.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(inspected["runningTaskCount"], 1);
    assert_eq!(inspected["desiredTaskCount"], 1);
    assert!(!inspected.to_string().contains("do-not-return"));
    assert!(inspected.get("definition").is_none());
    let node = json_body(
        send(
            &f,
            &url(&f, "nodes/node-1/inspect"),
            Some(f.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(node["runningTaskCount"], 1);
    // Service validation rejects managed Services, not native
    // Stack members. Their inventory Restart action remains available.
    sqlx::query("UPDATE swarmserviceprojections SET ownership='CitadelStack' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "services/service-1/restart"),
            f.administrator.clone(),
            Value::Null,
        )
        .await,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        runtime.lock().await.services["service-1"]["Spec"]["TaskTemplate"]["ForceUpdate"],
        1
    );
    assert_status(
        send_json(
            &f,
            Method::DELETE,
            &url(&f, "services"),
            f.administrator.clone(),
            json!({"ids":["service-1"]}),
        )
        .await,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_status(
        send(
            &f,
            &url(&f, "services/service-1"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::NOT_FOUND,
    )
    .await;
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn native_swarm_preflights_entire_batches_permissions_usage_and_versions() {
    let (f, runtime) = fixture_swarm().await;
    let reader = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    for path in [
        "nodes/node-1/inspect",
        "services/service-1/inspect",
        "configs/config-1/content",
    ] {
        assert_status(
            send(&f, &url(&f, path), None).await,
            StatusCode::UNAUTHORIZED,
        )
        .await;
        assert_status(
            send(&f, &url(&f, path), Some(reader.clone())).await,
            StatusCode::FORBIDDEN,
        )
        .await;
    }
    for (method, path, body) in [
        (
            Method::PATCH,
            "nodes/node-1",
            json!({"versionIndex":1,"availability":"Pause"}),
        ),
        (
            Method::PATCH,
            "nodes/availability",
            json!({"availability":"Pause","nodes":[{"nodeId":"node-1","versionIndex":1}]}),
        ),
        (Method::POST, "services/service-1/restart", Value::Null),
        (Method::DELETE, "services", json!({"ids":["service-1"]})),
        (Method::POST, "secrets", json!({"name":"x","data":"x"})),
        (Method::POST, "configs", json!({"name":"x","data":"x"})),
        (Method::DELETE, "secrets", json!({"ids":["secret-1"]})),
        (Method::DELETE, "configs", json!({"ids":["config-1"]})),
        (
            Method::PATCH,
            "secrets/secret-1/labels",
            json!({"versionIndex":1,"labels":{}}),
        ),
        (
            Method::PATCH,
            "configs/config-1/labels",
            json!({"versionIndex":1,"labels":{}}),
        ),
    ] {
        assert_status(
            send_json(&f, method, &url(&f, path), reader.clone(), body).await,
            StatusCode::FORBIDDEN,
        )
        .await;
    }
    for (path, body, status) in [
        (
            "nodes/availability",
            json!({"availability":"Pause","nodes":[{"nodeId":"node-1","versionIndex":1},{"nodeId":"missing","versionIndex":1}]}),
            StatusCode::NOT_FOUND,
        ),
        (
            "nodes/availability",
            json!({"availability":"Pause","nodes":[]}),
            StatusCode::BAD_REQUEST,
        ),
        (
            "nodes/node-1",
            json!({"versionIndex":99,"availability":"Pause"}),
            StatusCode::CONFLICT,
        ),
        (
            "configs/config-1/labels",
            json!({"versionIndex":99,"labels":{}}),
            StatusCode::CONFLICT,
        ),
    ] {
        assert_status(
            send_json(
                &f,
                Method::PATCH,
                &url(&f, path),
                f.administrator.clone(),
                body,
            )
            .await,
            status,
        )
        .await;
    }
    for kind in ["secrets", "configs", "services"] {
        assert_status(
            send_json(
                &f,
                Method::DELETE,
                &url(&f, kind),
                f.administrator.clone(),
                json!({"ids":[format!("{}-1",kind.trim_end_matches('s')),"missing"]}),
            )
            .await,
            StatusCode::NOT_FOUND,
        )
        .await;
    }
    sqlx::query("UPDATE swarmconfigprojections SET servicenames='[\"redis\"]' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_status(
        send_json(
            &f,
            Method::DELETE,
            &url(&f, "configs"),
            f.administrator.clone(),
            json!({"ids":["config-1"]}),
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    sqlx::query("UPDATE swarmserviceprojections SET ownership='System' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_status(
        send_json(
            &f,
            Method::POST,
            &url(&f, "services/service-1/restart"),
            f.administrator.clone(),
            Value::Null,
        )
        .await,
        StatusCode::CONFLICT,
    )
    .await;
    assert!(runtime.lock().await.mutations.is_empty());
    // Use the application mutation path to invalidate the reader's cached denial.
    PostgresUserRepository::new(f.pool.clone())
        .patch(
            reader.subject_id,
            &UserPatchMutation {
                resource_accesses: Some(vec![ResourceAccessInput {
                    resource_type: ResourceType::Platform,
                    resource_id: f.platform_id,
                    permission_level: PermissionLevel::Write,
                    specific_permissions: vec![SpecificPermission::Inspect],
                }]),
                ..Default::default()
            },
            f.administrator.actor_id,
            Utc::now(),
            true,
        )
        .await
        .unwrap();
    assert_status(
        send_json(
            &f,
            Method::PATCH,
            &url(&f, "nodes/node-1"),
            reader.clone(),
            json!({"versionIndex":1,"availability":"Pause","labels":{}}),
        )
        .await,
        StatusCode::NO_CONTENT,
    )
    .await;
    runtime.lock().await.configs.get_mut("config-1").unwrap()["Spec"]["Data"] = json!("/w==");
    assert_status(
        send(
            &f,
            &url(&f, "configs/config-1/content"),
            Some(reader.clone()),
        )
        .await,
        StatusCode::BAD_REQUEST,
    )
    .await;
    runtime.lock().await.cluster = "wrong-cluster".into();
    assert_status(
        send(&f, &url(&f, "configs/config-1/content"), Some(reader)).await,
        StatusCode::CONFLICT,
    )
    .await;
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn native_swarm_partial_delete_reconciles_successful_siblings_and_never_retries() {
    let (f, runtime) = fixture_swarm().await;
    for name in ["first", "second"] {
        assert_status(
            send_json(
                &f,
                Method::POST,
                &url(&f, "configs"),
                f.administrator.clone(),
                json!({"name":name,"data":"x"}),
            )
            .await,
            StatusCode::NO_CONTENT,
        )
        .await;
    }
    runtime.lock().await.rejected_delete = Some("config-second".into());
    let response = send_json(
        &f,
        Method::DELETE,
        &url(&f, "configs"),
        f.administrator.clone(),
        json!({"ids":["config-first","config-second"]}),
    )
    .await;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    assert!(String::from_utf8_lossy(&bytes).contains("Deleted 1 of 2"));
    assert_status(
        send(
            &f,
            &url(&f, "configs/config-first"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_eq!(
        json_body(
            send(
                &f,
                &url(&f, "configs/config-second"),
                Some(f.administrator.clone())
            )
            .await
        )
        .await["id"],
        "config-second"
    );
    assert_eq!(
        runtime
            .lock()
            .await
            .mutations
            .iter()
            .filter(|(path, _)| path == "configs/config-second")
            .count(),
        1
    );
    {
        let mut state = runtime.lock().await;
        state.rejected_delete = None;
        state.configs.remove("config-second");
    }
    assert_status(
        send_json(
            &f,
            Method::DELETE,
            &url(&f, "configs"),
            f.administrator.clone(),
            json!({"ids":["config-second"]}),
        )
        .await,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_status(
        send(
            &f,
            &url(&f, "configs/config-second"),
            Some(f.administrator.clone()),
        )
        .await,
        StatusCode::NOT_FOUND,
    )
    .await;
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn network_deletion_precondition_requires_current_unused_platform_projection() {
    use citadel_platforms::AuthorizedReadError;

    let f = fixture().await;
    let reads = &f.lookup_state.platforms.platforms;
    assert!(matches!(
        reads
            .validate_swarm_network_deletion(f.platform_id, "overlay", "app")
            .await,
        Err(AuthorizedReadError::Conflict(_))
    ));
    sqlx::query("INSERT INTO swarmnetworkprojections(platformid,dockernetworkid,driver,enableipv6,isattachable,isencrypted,isingress,isinternal,labels,name,observedat,scope,servicenames,subnets) VALUES($1,'overlay','overlay',false,false,false,false,false,'{}','app',now(),'swarm','[]','[]')")
        .bind(f.platform_id).execute(&f.pool).await.unwrap();
    reads
        .validate_swarm_network_deletion(f.platform_id, "overlay", "app")
        .await
        .unwrap();
    // A projection on one Platform cannot authorize deletion on another.
    assert!(matches!(
        reads
            .validate_swarm_network_deletion(Uuid::now_v7(), "overlay", "app")
            .await,
        Err(AuthorizedReadError::Conflict(_))
    ));
    sqlx::query(
        "UPDATE swarmnetworkprojections SET servicenames='[\"service\"]' WHERE platformid=$1",
    )
    .bind(f.platform_id)
    .execute(&f.pool)
    .await
    .unwrap();
    assert!(matches!(
        reads.validate_swarm_network_deletion(f.platform_id, "overlay", "app").await,
        Err(AuthorizedReadError::Conflict(message)) if message.contains("used by")
    ));
    sqlx::query(
        "UPDATE swarmnetworkprojections SET servicenames='[]',isstale=true WHERE platformid=$1",
    )
    .bind(f.platform_id)
    .execute(&f.pool)
    .await
    .unwrap();
    assert!(matches!(
        reads.validate_swarm_network_deletion(f.platform_id, "overlay", "app").await,
        Err(AuthorizedReadError::Conflict(message)) if message.contains("no current inventory")
    ));
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn persisted_service_ownership_and_confirmed_removals_are_platform_scoped() {
    use citadel_platforms::swarm_mutations::SwarmResourceKind;
    let (f, runtime) = fixture_swarm().await;
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,dockerserviceid,createdbyactorid,desiredspechash,health,synchronizationstate,spec,updatedat) VALUES($1,$2,$2,$3,'service-1',$4,'hash','Unknown','NeverApplied','{}',now())")
        .bind(Uuid::now_v7()).bind(format!("managed-{}",Uuid::now_v7())).bind(f.platform_id).bind(SYSTEM_ACTOR_ID)
        .execute(&f.pool).await.unwrap();
    let services = &f.lookup_state.platforms.services;
    assert!(
        services
            .owns_runtime_service(f.platform_id, "service-1")
            .await
            .unwrap()
    );
    assert!(
        !services
            .owns_runtime_service(Uuid::now_v7(), "service-1")
            .await
            .unwrap()
    );
    assert!(
        !services
            .owns_runtime_service(f.platform_id, "other-service")
            .await
            .unwrap()
    );
    for (method, suffix, body) in [
        (Method::POST, "services/service-1/restart", Value::Null),
        (Method::DELETE, "services", json!({"ids":["service-1"]})),
    ] {
        assert_status(
            send_json(&f, method, &url(&f, suffix), f.administrator.clone(), body).await,
            StatusCode::CONFLICT,
        )
        .await;
    }
    assert!(runtime.lock().await.mutations.is_empty());
    // Deletion updates are limited to the specified resource kind and Platform.
    let store = &f.lookup_state.platforms.projections;
    for (kind, id, suffix) in [
        (
            SwarmResourceKind::Service,
            "service-1",
            "services/service-1",
        ),
        (SwarmResourceKind::Secret, "secret-1", "secrets/secret-1"),
        (SwarmResourceKind::Config, "config-1", "configs/config-1"),
    ] {
        assert_status(
            send(&f, &url(&f, suffix), Some(f.administrator.clone())).await,
            StatusCode::OK,
        )
        .await;
        store
            .remove_swarm_resources(Uuid::now_v7(), kind, &[id.into()])
            .await
            .unwrap();
        store
            .remove_swarm_resources(f.platform_id, kind, &[])
            .await
            .unwrap();
        assert_status(
            send(&f, &url(&f, suffix), Some(f.administrator.clone())).await,
            StatusCode::OK,
        )
        .await;
        store
            .remove_swarm_resources(f.platform_id, kind, &[id.into()])
            .await
            .unwrap();
        assert_status(
            send(&f, &url(&f, suffix), Some(f.administrator.clone())).await,
            StatusCode::NOT_FOUND,
        )
        .await;
    }
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn feature_swarm_operations_enforce_preflight_identity_and_partial_completion() {
    use citadel_platforms::swarm_mutations::{
        DeleteSwarmResourcesInput, SwarmOperation as Op, SwarmOperations, SwarmResourceKind,
        UpdateSwarmNodeInput,
    };
    let (f, runtime) = fixture_swarm().await;
    let state = &f.lookup_state.platforms;
    let platform = state
        .platforms
        .get_platform(f.platform_id)
        .await
        .unwrap()
        .unwrap();
    let changes = std::sync::Mutex::new(Vec::new());
    let changed = |kind: &str, action: &str, id: &str| {
        changes
            .lock()
            .unwrap()
            .push((kind.to_owned(), action.to_owned(), id.to_owned()))
    };
    let operations = SwarmOperations {
        reads: &state.platforms,
        services: state.services.as_ref(),
        runtime: state.runtime.as_ref(),
        projections: state.projections.as_ref(),
        changed: &changed,
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    let update = |version_index| Op::UpdateNode {
        id: "node-1".into(),
        input: UpdateSwarmNodeInput {
            version_index,
            availability: "Drain".into(),
            labels: BTreeMap::new(),
        },
    };
    assert!(
        operations
            .execute(&platform, update(99), &cancel)
            .await
            .is_err()
    );
    assert!(runtime.lock().await.mutations.is_empty());
    let cluster = runtime.lock().await.cluster.clone();
    runtime.lock().await.cluster = "wrong-cluster".into();
    assert!(
        operations
            .execute(&platform, update(1), &cancel)
            .await
            .is_err()
    );
    assert!(runtime.lock().await.mutations.is_empty());
    runtime.lock().await.cluster = cluster;
    operations
        .execute(&platform, update(1), &cancel)
        .await
        .unwrap();
    assert_eq!(
        state
            .platforms
            .get_swarm_node(f.platform_id, "node-1")
            .await
            .unwrap()
            .unwrap()
            .availability,
        "Drain"
    );
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,dockerserviceid,createdbyactorid,desiredspechash,health,synchronizationstate,spec,updatedat) VALUES($1,$2,$2,$3,'service-1',$4,'hash','Unknown','NeverApplied','{}',now())")
        .bind(Uuid::now_v7()).bind(format!("managed-{}",Uuid::now_v7())).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let count = runtime.lock().await.mutations.len();
    assert!(
        operations
            .execute(&platform, Op::RestartService("service-1".into()), &cancel)
            .await
            .is_err()
    );
    assert_eq!(runtime.lock().await.mutations.len(), count);
    // Second sibling fails. The first confirmed removal must still be projected.
    let mut secret = runtime.lock().await.secrets["secret-1"].clone();
    secret["ID"] = json!("secret-2");
    runtime
        .lock()
        .await
        .secrets
        .insert("secret-2".into(), secret);
    operations
        .execute(&platform, update(2), &cancel)
        .await
        .unwrap();
    runtime.lock().await.rejected_delete = Some("secret-2".into());
    assert!(
        operations
            .execute(
                &platform,
                Op::Delete {
                    kind: SwarmResourceKind::Secret,
                    input: DeleteSwarmResourcesInput {
                        ids: vec!["secret-1".into(), "secret-2".into()]
                    }
                },
                &cancel
            )
            .await
            .is_err()
    );
    assert!(
        state
            .platforms
            .get_swarm_secret(f.platform_id, "secret-1")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        state
            .platforms
            .get_swarm_secret(f.platform_id, "secret-2")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        changes
            .lock()
            .unwrap()
            .iter()
            .any(|(_, action, id)| action == "remove" && id == "secret-1")
    );
    let closed = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@localhost/unused")
        .unwrap();
    closed.close().await;
    let failing_store = PostgresInventoryProjectionStore::new(closed);
    let failing = SwarmOperations {
        projections: &failing_store,
        ..operations
    };
    let error = failing
        .execute(&platform, update(3), &cancel)
        .await
        .unwrap_err();
    assert!(
        matches!(error, citadel_platforms::swarm_mutations::SwarmCompletionError::Runtime(error)
        if error.kind == citadel_platforms::RuntimeErrorKind::Remote)
    );
    assert_eq!(runtime.lock().await.nodes["node-1"]["Version"]["Index"], 4);
    citadel_platforms::swarm_mutations::refresh_inventory(
        state.projections.as_ref(),
        state.runtime.as_ref(),
        &platform,
        |_| {},
    )
    .await
    .unwrap();
    let operations = SwarmOperations {
        projections: state.projections.as_ref(),
        ..failing
    };
    runtime.lock().await.fail_after_update = true;
    assert!(
        operations
            .execute(&platform, update(4), &cancel)
            .await
            .is_err()
    );
    assert_eq!(
        state
            .platforms
            .get_swarm_node(f.platform_id, "node-1")
            .await
            .unwrap()
            .unwrap()
            .version_index,
        5,
        "ambiguous writes still refresh the committed projection without replay"
    );
    cleanup(f).await;
}
