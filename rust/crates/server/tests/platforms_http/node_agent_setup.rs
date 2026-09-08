use super::*;
use citadel_adapters::{
    edge::{EdgeRegistry, PostgresEdgeStore},
    node_agent_lifecycle_store::PostgresNodeAgentLifecycleStore,
};
use citadel_platforms::{
    PlatformRuntimePort,
    node_agents::{
        lifecycle::NodeAgentLifecycleStore,
        setup::{NodeAgentSetupStore, SetupKind},
    },
};
use citadel_server::platforms_http::EdgeHttpContext;

// Ports Install_AfterRemoval_ShouldRestoreInstalledDesiredState and the install/repair/upgrade
// permission theory through actual HTTP handlers and PostgreSQL transactions.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn setup_endpoints_authorize_and_restore_removed_manager_only_installation() {
    let cluster = format!("setup-{}", Uuid::now_v7());
    let f = fixture_for_cluster(cluster.clone()).await;
    let id = f.platform_id;
    f.docker_server.abort();
    let (docker, server, socket) = docker_fixture_for_cluster(100, cluster.clone()).await;
    let mut state = f.lookup_state.platforms.clone();
    state.docker = docker;
    let app = platforms_http::router(state).layer(axum::Extension(EdgeHttpContext {
        store: PostgresEdgeStore::new(f.pool.clone()),
        registry: EdgeRegistry::default(),
        core_url: "https://core.example.test".into(),
        agent_image: "agent:latest".into(),
        node_agent_ca_bundle: None,
    }));
    sqlx::query("UPDATE platforms SET clusterid=$2,platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"node-1\",\"daemonId\":\"daemon-test\"}' WHERE id=$1").bind(id).bind(&cluster).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeagentinstallations(platformid,clusterid,managerdockernodeid,managerdockerdaemonid,dockerservicename,agentimagereference,agentimagedigest,desiredstate) VALUES($1,$2,'node-1','daemon-test','fixture','','','Removed')").bind(id).bind(&cluster).execute(&f.pool).await.unwrap();
    let mut reader = f.administrator.clone();
    reader.actor_id = ActorId::new(f.actor_id);
    reader.roles.clear();
    for action in ["install", "repair", "upgrade"] {
        let path = format!("/api/v1/platforms/{id}/node-agents/{action}");
        let send = |principal: Option<ActorPrincipal>| {
            let app = app.clone();
            let path = path.clone();
            async move {
                let mut request = Request::builder()
                    .method(Method::POST)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap();
                if let Some(principal) = principal {
                    request.extensions_mut().insert(principal);
                }
                app.oneshot(request).await.unwrap()
            }
        };
        assert_eq!(send(None).await.status(), StatusCode::UNAUTHORIZED);
        sqlx::query("UPDATE resourceaccesses SET permissionlevel=4,specificpermissions=0 WHERE resourceid=$1 AND actorid=$2").bind(id).bind(f.actor_id).execute(&f.pool).await.unwrap();
        assert_eq!(
            send(Some(reader.clone())).await.status(),
            StatusCode::FORBIDDEN,
            "Execute alone is insufficient"
        );
        sqlx::query(
            "UPDATE resourceaccesses SET specificpermissions=$3 WHERE resourceid=$1 AND actorid=$2",
        )
        .bind(id)
        .bind(f.actor_id)
        .bind(citadel_domain::SpecificPermission::ManageNodeAgents as i32)
        .execute(&f.pool)
        .await
        .unwrap();
        let response = send(Some(reader.clone())).await;
        assert_eq!(response.status(), StatusCode::OK);
        let progress = json_body(response).await;
        let last = progress.as_array().unwrap().last().unwrap();
        assert_eq!(last["isCompleted"], true, "{progress}");
        assert!(last["errorMessage"].is_null(), "{progress}");
        let persisted:(String,String,String)=sqlx::query_as("SELECT desiredstate,operationkind,operationstate FROM swarmnodeagentinstallations WHERE platformid=$1").bind(id).fetch_one(&f.pool).await.unwrap();
        assert_eq!(
            persisted,
            (
                "Installed".into(),
                format!("{}{}", action[..1].to_uppercase(), &action[1..]),
                "Completed".into()
            )
        );
        let event:String=sqlx::query_scalar("SELECT info FROM activityevents WHERE resourceid=$1 AND eventtype='PlatformNodeAgentLifecycle' ORDER BY createdat DESC LIMIT 1").bind(id).fetch_one(&f.pool).await.unwrap();
        let event: Value = serde_json::from_str(&event).unwrap();
        assert_eq!(event["Kind"], persisted.1);
        assert_eq!(event["State"], "Completed");
    }
    let bootstrap_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM swarmnodeagentbootstraps WHERE platformid=$1")
            .bind(id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(
        bootstrap_count, 0,
        "manager-only coverage needs no bootstrap credential"
    );
    server.abort();
    let _ = std::fs::remove_file(socket);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn setup_bootstrap_is_hashed_short_lived_revoked_and_fenced_by_operation() {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    let cluster = format!("bootstrap-{}", Uuid::now_v7());
    let f = fixture_for_cluster(cluster.clone()).await;
    let id = f.platform_id;
    sqlx::query("UPDATE platforms SET clusterid=$2,platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"node-1\",\"daemonId\":\"daemon-test\"}' WHERE id=$1").bind(id).bind(&cluster).execute(&f.pool).await.unwrap();
    let store = PostgresNodeAgentLifecycleStore(f.pool.clone());
    let info = f
        .lookup_state
        .platforms
        .docker
        .get_info(&tokio_util::sync::CancellationToken::new())
        .await
        .unwrap();
    let claim = store
        .claim_setup(
            f.administrator.actor_id,
            id,
            info.clone(),
            SetupKind::Install,
        )
        .await
        .unwrap();
    assert!(
        store
            .claim_remove(f.administrator.actor_id, id, info.clone())
            .await
            .is_err()
    );
    let first = store.bootstrap(&claim).await.unwrap();
    store
        .secret_created(&claim, first.id, &format!("secret-{}", first.id))
        .await
        .unwrap();
    let row:(String,i32,bool)=sqlx::query_as("SELECT tokenhash,version,expiresatutc>now() AND expiresatutc<=now()+interval '10 minutes' FROM swarmnodeagentbootstraps WHERE id=$1").bind(first.id).fetch_one(&f.pool).await.unwrap();
    assert_eq!(
        row.0,
        URL_SAFE_NO_PAD.encode(Sha256::digest(first.token.as_slice()))
    );
    assert_ne!(row.0, String::from_utf8(first.token.to_vec()).unwrap());
    assert_eq!(row.1, 1);
    assert!(row.2);
    let second = store.bootstrap(&claim).await.unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT revokedatutc IS NOT NULL FROM swarmnodeagentbootstraps WHERE id=$1"
        )
        .bind(first.id)
        .fetch_one(&f.pool)
        .await
        .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT version FROM swarmnodeagentbootstraps WHERE id=$1")
            .bind(second.id)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        2
    );
    store
        .finish_setup(
            &claim,
            SetupKind::Install,
            Some("Docker rejected the Service"),
        )
        .await
        .unwrap();
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM swarmnodeagentbootstraps WHERE platformid=$1 AND revokedatutc IS NULL").bind(id).fetch_one(&f.pool).await.unwrap(),0);
    let retry = store
        .claim_setup(f.administrator.actor_id, id, info, SetupKind::Repair)
        .await
        .unwrap();
    assert!(store.bootstrap(&claim).await.is_err());
    assert!(
        store
            .secret_created(&claim, second.id, "stale")
            .await
            .is_err()
    );
    assert!(
        store
            .finish_setup(&claim, SetupKind::Install, None)
            .await
            .is_err()
    );
    store
        .finish_setup(&retry, SetupKind::Repair, None)
        .await
        .unwrap();
    f.docker_server.abort();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn setup_uses_exact_edge_manager_and_canonical_system_service_commands() {
    use citadel_adapters::{edge::EdgeTarget, node_agent_runtime::NodeAgentRuntimeRouter};
    use citadel_contracts::citadel::{
        edge::v1::{EdgeCommandKind, core_envelope},
        images::v1::*,
        shared_models::v1::{PlatformInfoResponse, SwarmInfoMessage},
        swarm::v1::*,
    };
    use citadel_platforms::node_agents::{
        lifecycle::{NodeAgentRemovalClaim, NodeAgentResource},
        setup::{NodeAgentSetupRuntime, SystemAgentSpec, ownership},
    };
    use prost::Message;
    let f = fixture().await;
    let id = f.platform_id;
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    f.docker_server.abort();
    let registry = EdgeRegistry::default();
    let (manager, mut outbound) = registry
        .register(EdgeTarget::platform(id), Uuid::now_v7())
        .unwrap();
    let (_worker, mut worker_outbound) = registry
        .register(EdgeTarget::node(id, "worker".into()), Uuid::now_v7())
        .unwrap();
    let c = NodeAgentRemovalClaim {
        platform_id: id,
        platform_name: "fixture".into(),
        actor: f.administrator.actor_id,
        operation_id: Uuid::now_v7(),
        cluster_id: "cluster".into(),
        manager_node_id: "manager".into(),
        manager_daemon_id: "daemon".into(),
        service_id: None,
        secret_ids: vec![],
        ca_config_id: None,
    };
    let labels: std::collections::HashMap<_, _> = ownership(&c).into_iter().collect();
    let peer = tokio::spawn(async move {
        let mut calls = vec![];
        for _ in 0..11 {
            let envelope = outbound.recv().await.unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected command")
            };
            let kind = EdgeCommandKind::try_from(command.kind).unwrap();
            let payload = match kind {
                EdgeCommandKind::PlatformGetInfo => PlatformInfoResponse {
                    id: "daemon".into(),
                    swarm_info: Some(SwarmInfoMessage {
                        node_id: "manager".into(),
                        cluster_id: "cluster".into(),
                        control_available: true,
                        local_node_state: "active".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
                .encode_to_vec(),
                EdgeCommandKind::ImageDistributionInspect => {
                    DistributionInspectResponse::default().encode_to_vec()
                }
                EdgeCommandKind::SwarmSecretCreate => {
                    let request =
                        CreateSwarmSecretRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(request.data, b"bootstrap-token");
                    assert_eq!(request.labels, labels);
                    SwarmResourceCreateResponse {
                        resource_id: "secret-id".into(),
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::SwarmConfigCreate => {
                    let request =
                        CreateSwarmConfigRequest::decode(command.payload.as_slice()).unwrap();
                    assert_eq!(request.data, b"ca-data");
                    assert_eq!(request.labels, labels);
                    SwarmResourceCreateResponse {
                        resource_id: "ca-id".into(),
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::SwarmSystemServiceCreate => {
                    let request =
                        CreateSystemSwarmServiceRequest::decode(command.payload.as_slice())
                            .unwrap();
                    let spec = request.spec.unwrap();
                    assert_eq!(spec.bootstrap_secret_id, "secret-id");
                    assert_eq!(spec.manager_node_id, "manager");
                    assert!(
                        spec.environment
                            .iter()
                            .all(|e| !e.contains("bootstrap-token"))
                    );
                    assert_eq!(request.labels, labels);
                    SwarmServiceMutationResponse {
                        service_id: "agent-service".into(),
                        ..Default::default()
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::SwarmServiceList => ListSwarmServicesResponse {
                    services: vec![SwarmServiceMessage {
                        id: "agent-service".into(),
                        version_index: 42,
                        labels: labels.clone(),
                        ..Default::default()
                    }],
                }
                .encode_to_vec(),
                EdgeCommandKind::SwarmSystemServiceUpdate => {
                    let request =
                        UpdateSystemSwarmServiceRequest::decode(command.payload.as_slice())
                            .unwrap();
                    assert_eq!(request.version_index, 42);
                    assert_eq!(request.service_id, "agent-service");
                    assert_eq!(request.labels, labels);
                    SwarmServiceMutationResponse {
                        service_id: "agent-service".into(),
                        ..Default::default()
                    }
                    .encode_to_vec()
                }
                other => panic!("unexpected {other:?}"),
            };
            calls.push(kind);
            let command_id = Uuid::parse_str(&command.command_id).unwrap();
            manager.output(command_id, payload);
            manager.complete(command_id, true);
        }
        calls
    });
    let runtime = NodeAgentRuntimeRouter {
        pool: f.pool.clone(),
        docker: f.lookup_state.platforms.docker.clone(),
        agent: None,
        edge: registry.clone(),
    };
    let cancel = tokio_util::sync::CancellationToken::new();
    runtime
        .distribution(&c, "agent:latest", &cancel)
        .await
        .unwrap();
    runtime
        .create_material(
            &c,
            NodeAgentResource::Secret,
            "bootstrap",
            b"bootstrap-token",
            &cancel,
        )
        .await
        .unwrap();
    runtime
        .create_material(&c, NodeAgentResource::Config, "ca", b"ca-data", &cancel)
        .await
        .unwrap();
    let spec = SystemAgentSpec {
        name: "agent".into(),
        image: format!("agent@sha256:{}", "a".repeat(64)),
        environment: vec![],
        manager_node_id: "manager".into(),
        volume_name: "state".into(),
        secret_id: "secret-id".into(),
        secret_name: "bootstrap".into(),
        ca_config_id: Some("ca-id".into()),
        ca_config_name: Some("ca".into()),
        architectures: vec!["amd64".into()],
        labels: ownership(&c),
    };
    assert_eq!(
        runtime
            .apply_system(&c, &spec, None, &cancel)
            .await
            .unwrap(),
        "agent-service"
    );
    let current = citadel_platforms::RuntimeSwarmService {
        id: "agent-service".into(),
        version_index: 42,
        labels: ownership(&c),
        ..Default::default()
    };
    assert_eq!(
        runtime
            .apply_system(&c, &spec, Some(&current), &cancel)
            .await
            .unwrap(),
        "agent-service"
    );
    assert_eq!(peer.await.unwrap().len(), 11);
    assert!(worker_outbound.try_recv().is_err());
    registry.disconnect(&EdgeTarget::platform(id));
    assert!(
        runtime
            .create_material(
                &c,
                NodeAgentResource::Secret,
                "bootstrap",
                b"token",
                &cancel
            )
            .await
            .is_err()
    );
}
