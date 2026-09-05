use super::*;
use citadel_adapters::{
    activity_store::PostgresActivityStore, alert_store::PostgresAlertStore,
    automation_store::PostgresAutomationStore, backup_store::PostgresBackupStore,
    build_store::PostgresBuildStore, deployment_store::PostgresDeploymentStore,
    stack_store::PostgresStackStore, swarm_service_store::PostgresSwarmServiceStore,
};
use citadel_domain::ResourceType;
use citadel_identity::{AccessTokenClaims, SessionTokenCodec};
use citadel_server::realtime::{IdentityRealtimeReader, RealtimeService};
use citadel_server::realtime_groups::{ApplicationGroupReader, Group, GroupReadPort};
use citadel_server::{config::RealtimeConfig, metrics::Metrics};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use tokio_util::sync::CancellationToken;
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

fn reader(f: &Fixture) -> ApplicationGroupReader {
    ApplicationGroupReader {
        identity: f.lookup_state.platforms.identity.clone(),
        platforms: f.lookup_state.platforms.platforms.clone(),
        deployments: Arc::new(PostgresDeploymentStore::new(f.pool.clone())),
        stacks: Arc::new(PostgresStackStore::new(f.pool.clone())),
        services: Arc::new(PostgresSwarmServiceStore::new(f.pool.clone())),
        resources: Arc::new(PostgresResourceMetadataStore::new(f.pool.clone())),
        automation: Arc::new(PostgresAutomationStore::new(f.pool.clone())),
        builds: Arc::new(PostgresBuildStore::new(f.pool.clone())),
        backups: Arc::new(PostgresBackupStore::new(f.pool.clone())),
        activities: Arc::new(citadel_application::ActivityService::new(Arc::new(
            PostgresActivityStore::new(f.pool.clone()),
        ))),
        alerts: Arc::new(PostgresAlertStore::new(f.pool.clone())),
        docker: f.lookup_state.platforms.clone(),
    }
}
fn token(p: &ActorPrincipal) -> String {
    JwtSessionTokenCodec::new(&[7; 32], "fixture".into(), "fixture".into())
        .unwrap()
        .encode_access(&AccessTokenClaims {
            subject_id: p.subject_id,
            actor_id: p.actor_id,
            principal_type: p.principal_type,
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(5),
            automation_run_id: None,
        })
        .unwrap()
}
async fn receive(socket: &mut Socket) -> Value {
    let frame = tokio::time::timeout(StdDuration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_str(frame.to_text().unwrap()).unwrap()
}
async fn invoke(socket: &mut Socket, target: &str, group: &str) -> Value {
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"invoke","invocationId":"1","target":target,"arguments":[group]}).to_string().into())).await.unwrap();
    let completion = receive(socket).await;
    assert_eq!(completion["kind"], "completion");
    completion
}
async fn cleanup(f: Fixture) {
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn managed_service_group_requires_parent_platform_and_preserves_persisted_spec() {
    let f = fixture().await;
    let principal = super::lookup::subject(&f).await;
    let id = Uuid::now_v7();
    let spec: citadel_swarm_services::SwarmServiceSpec = serde_json::from_value(json!({
        "image":{"$type":"External","registryId":Uuid::now_v7(),"imageTag":"redis","resolvedDigest":"sha256:applied"},
        "replicas":2
    })).unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,desiredspechash,health,spec,synchronizationstate,updatedat,autoupdatestate_status) VALUES($1,$2,$2,$3,$4,'hash','Unknown',$5,'Unknown',now(),'Unknown')")
        .bind(id).bind(format!("service-{id}")).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).bind(spec.to_storage_value().unwrap()).execute(&f.pool).await.unwrap();
    let groups = reader(&f);
    let group = Group::parse(&format!("swarm-service:{id}")).unwrap();
    assert!(groups.read(&principal, &group, None).await.is_err());
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::SwarmService,
        id,
        0,
    )
    .await;
    assert!(
        groups.read(&principal, &group, None).await.is_err(),
        "Service Read alone must not bypass parent Platform visibility"
    );
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let snapshot = groups.read(&principal, &group, None).await.unwrap();
    assert_eq!(snapshot.rows[0].target, "SwarmServiceInfoUpdated");
    let row = &snapshot.rows[0].rows[0];
    assert_eq!(row["spec"]["image"]["$type"], "External");
    assert_eq!(row["spec"]["image"]["resolvedDigest"], "sha256:applied");
    assert_eq!(row["spec"]["replicas"], 2);
    sqlx::query("UPDATE swarmservices SET spec=jsonb_set(spec,'{Replicas}','3') WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    let changed = groups.read(&principal, &group, None).await.unwrap();
    assert_eq!(changed.rows[0].rows[0]["spec"]["replicas"], 3);
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn dotnet_group_permission_matrix_uses_current_database_permissions() {
    let f = fixture().await;
    let principal = super::lookup::subject(&f).await;
    let groups = reader(&f);
    for name in [
        format!("containers:{}", f.platform_id),
        "platforms".into(),
        "deployments".into(),
        format!("activity:User:{}", principal.subject_id),
    ] {
        assert!(
            groups
                .read(&principal, &Group::parse(&name).unwrap(), None)
                .await
                .is_err(),
            "{name}"
        );
    }
    super::lookup::grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let group = Group::parse(&format!("containers:{}", f.platform_id)).unwrap();
    let snapshot = groups.read(&principal, &group, None).await.unwrap();
    assert_eq!(snapshot.events[0].target, "ContainersInfoUpdated");
    assert_eq!(
        snapshot.events[0].arguments[0]["containers"][0]["name"],
        "web"
    );
    let daemon = groups
        .read(
            &principal,
            &Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap(),
            None,
        )
        .await
        .unwrap();
    let inventory = daemon
        .events
        .iter()
        .find(|event| event.target == "SwarmInventoryUpdated")
        .unwrap();
    for kind in [
        "nodes", "services", "tasks", "networks", "configs", "secrets",
    ] {
        assert!(
            inventory.arguments[0][kind]["items"].is_array(),
            "{kind} must use the existing SwarmItemsView wrapper"
        );
    }
    assert!(
        daemon
            .rows
            .iter()
            .any(|batch| batch.target == "VolumeEventReceived")
    );
    assert!(
        daemon
            .rows
            .iter()
            .any(|batch| batch.target == "NetworkEventReceived")
    );
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1 AND resourceid=$2")
        .bind(principal.actor_id.value())
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(groups.read(&principal, &group, None).await.is_err());
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn group_wire_acceptance_joins_once_delivers_committed_rows_and_leaves_without_refetch() {
    let f = fixture().await;
    let cancellation = CancellationToken::new();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 4,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_secs(1),
            snapshot_limit: 1000,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancellation.clone(),
    )
    .with_groups(Arc::new(reader(&f)));
    let hub = service.hub();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancellation.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap()
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&f.administrator)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    let group = format!("containers:{}", f.platform_id);
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    let restricted = super::lookup::subject(&f).await;
    let (mut second, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    second.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":token(&restricted)}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut second).await["kind"], "subscribed");
    assert!(invoke(&mut second, "JoinGroup", &group).await["error"].is_string());
    super::lookup::grant(
        &f,
        restricted.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    assert!(invoke(&mut second, "JoinGroup", &group).await["error"].is_null());
    assert_eq!(
        receive(&mut second).await["target"],
        "ContainersInfoUpdated"
    );
    let initial = receive(&mut socket).await;
    assert_eq!(initial["target"], "ContainersInfoUpdated");
    assert!(initial["arguments"][0]["containers"].is_array());
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    sqlx::query("UPDATE containers SET name='renamed-live' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_runtime_change(f.platform_id, "container", "rename", "container-1");
    let changed = receive(&mut socket).await;
    assert_eq!(
        changed["arguments"][0]["containers"][0]["name"],
        "renamed-live"
    );
    assert_eq!(
        receive(&mut second).await["arguments"],
        changed["arguments"]
    );
    // A resource ACL removal must affect an existing authenticated socket,
    // including a repeated join. A cached token role is insufficient.
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1 AND resourceid=$2")
        .bind(restricted.actor_id.value())
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(invoke(&mut second, "JoinGroup", &group).await["error"].is_string());
    // Existing statistics handlers require Citadel IDs, not Docker IDs.
    hub.publish_container_stats(
        f.platform_id,
        &[citadel_platforms::RuntimeContainerStat {
            docker_container_id: "container-1".into(),
            cpu_usage: 42.0,
            memory_active: 10.0,
            memory_cache: 2.0,
            memory_limit: 128.0,
            rx_bytes: 0.0,
            tx_bytes: 0.0,
            created: 2,
        }],
    );
    let stats = receive(&mut socket).await;
    let revoked = tokio::time::timeout(StdDuration::from_secs(5), second.next())
        .await
        .unwrap();
    assert!(
        matches!(revoked, Some(Ok(Message::Close(_))) | None),
        "revoked subscriber received {revoked:?}"
    );
    assert_eq!(stats["target"], "ContainersStatsUpdated");
    assert_eq!(
        stats["arguments"][0][0]["containerId"],
        initial["arguments"][0]["containers"][0]["id"]
    );
    assert_eq!(stats["arguments"][0][0]["cpuUsage"], 42.0);
    assert!(invoke(&mut socket, "LeaveGroup", &group).await["error"].is_null());
    hub.publish_runtime_change(f.platform_id, "container", "stop", "container-1");
    // A completion acts as an ordering barrier: no event from the left group.
    assert!(invoke(&mut socket, "LeaveGroup", &group).await["error"].is_null());
    for denied in [
        "unknown",
        "activity:User",
        "alert-events:private",
        "container-log:container-1",
    ] {
        assert!(invoke(&mut socket, "JoinGroup", denied).await["error"].is_string());
    }
    socket.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}
