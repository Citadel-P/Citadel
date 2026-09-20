use super::*;
use citadel_adapters::node_agent_lifecycle_store::PostgresNodeAgentLifecycleStore;
use citadel_platforms::{PlatformRuntimePort, node_agents::lifecycle::NodeAgentLifecycleStore};

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn removal_routes_to_exact_edge_manager_and_checks_ownership_before_each_delete() {
    use citadel_adapters::{
        edge::{EdgeRegistry, EdgeTarget},
        node_agent_runtime::NodeAgentRuntimeRouter,
    };
    use citadel_contracts::citadel::{
        edge::v1::{EdgeCommandKind, core_envelope},
        shared_models::v1::{PlatformInfoResponse, SwarmInfoMessage},
        swarm::v1::{SwarmConfigMessage, SwarmSecretMessage, SwarmServiceMessage},
    };
    use citadel_platforms::node_agents::lifecycle::{
        NodeAgentLifecycleRuntime, NodeAgentRemovalClaim, NodeAgentResource,
    };
    use prost::Message;
    let fixture = fixture().await;
    let id = fixture.platform_id;
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id=$1")
        .bind(id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    // Any accidental Local fallback now fails: only the registered manager may execute commands.
    fixture.docker_server.abort();
    let registry = EdgeRegistry::default();
    let (manager, mut outbound) = registry
        .register(EdgeTarget::platform(id), Uuid::now_v7())
        .unwrap();
    let (_worker, mut worker_outbound) = registry
        .register(EdgeTarget::node(id, "worker".into()), Uuid::now_v7())
        .unwrap();
    let claim = NodeAgentRemovalClaim {
        platform_id: id,
        platform_name: "fixture".into(),
        actor: fixture.administrator.actor_id,
        operation_id: Uuid::now_v7(),
        cluster_id: "cluster".into(),
        manager_node_id: "manager".into(),
        manager_daemon_id: "daemon".into(),
        service_id: None,
        ca_config_id: None,
        secret_ids: vec![],
    };
    let runtime = NodeAgentRuntimeRouter {
        pool: fixture.pool.clone(),
        docker: fixture.lookup_state.platforms.docker.clone(),
        agent: None,
        edge: registry.clone(),
    };
    let peer = tokio::spawn(async move {
        let mut received = Vec::new();
        let labels = std::collections::HashMap::from([
            ("com.citadel.system".into(), "true".into()),
            ("com.citadel.system-role".into(), "swarm-node-agent".into()),
            ("com.citadel.platform-id".into(), id.to_string()),
            ("com.citadel.swarm-cluster-id".into(), "cluster".into()),
        ]);
        // Three authorized deletes, then an unrelated Service inspection. No fourth delete.
        for _ in 0..11 {
            let envelope = outbound.recv().await.unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected command");
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
                EdgeCommandKind::SwarmServiceInspect => SwarmServiceMessage {
                    labels: if received.len() > 8 {
                        Default::default()
                    } else {
                        labels.clone()
                    },
                    ..Default::default()
                }
                .encode_to_vec(),
                EdgeCommandKind::SwarmSecretInspect => SwarmSecretMessage {
                    labels: labels.clone(),
                    ..Default::default()
                }
                .encode_to_vec(),
                EdgeCommandKind::SwarmConfigInspect => SwarmConfigMessage {
                    labels: labels.clone(),
                    ..Default::default()
                }
                .encode_to_vec(),
                EdgeCommandKind::SwarmServiceDelete
                | EdgeCommandKind::SwarmSecretDelete
                | EdgeCommandKind::SwarmConfigDelete => ().encode_to_vec(),
                other => panic!("unexpected command {other:?}"),
            };
            received.push(kind);
            let command_id = Uuid::parse_str(&command.command_id).unwrap();
            manager.output(command_id, payload);
            manager.complete(command_id, true);
        }
        received
    });
    let cancellation = tokio_util::sync::CancellationToken::new();
    for kind in [
        NodeAgentResource::Service,
        NodeAgentResource::Secret,
        NodeAgentResource::Config,
    ] {
        runtime
            .delete_owned(&claim, kind, "owned-id", &cancellation)
            .await
            .unwrap();
    }
    assert!(
        runtime
            .delete_owned(
                &claim,
                NodeAgentResource::Service,
                "foreign-id",
                &cancellation
            )
            .await
            .unwrap_err()
            .message
            .contains("ownership")
    );
    let received = peer.await.unwrap();
    assert_eq!(
        received
            .iter()
            .filter(|kind| matches!(
                kind,
                EdgeCommandKind::SwarmServiceDelete
                    | EdgeCommandKind::SwarmSecretDelete
                    | EdgeCommandKind::SwarmConfigDelete
            ))
            .count(),
        3
    );
    assert!(worker_outbound.try_recv().is_err());
    registry.disconnect(&EdgeTarget::platform(id));
    assert!(
        runtime
            .delete_owned(&claim, NodeAgentResource::Config, "owned-id", &cancellation)
            .await
            .is_err()
    );
}

// Ports LifecycleEndpoints_ShouldRequirePlatformScopedManageNodeAgentsPermission
// and Remove_ShouldRevokeNodeBindingsAndPersistRemovedDesiredState through HTTP.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn remove_node_agents_authorizes_revokes_and_commits_lifecycle_activities() {
    let cluster = format!("removal-{}", Uuid::now_v7());
    let fixture = fixture_for_cluster(cluster.clone()).await;
    let id = fixture.platform_id;
    sqlx::query("UPDATE platforms SET clusterid=$2,platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"node-1\",\"daemonId\":\"daemon-test\"}' WHERE id=$1").bind(id).bind(&cluster).execute(&fixture.pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeagentinstallations(platformid,clusterid,managerdockernodeid,managerdockerdaemonid,dockerservicename,agentimagereference,agentimagedigest,desiredstate) VALUES($1,$2,'node-1','daemon-test','fixture','','','Installed')")
        .bind(id).bind(&cluster).execute(&fixture.pool).await.unwrap();
    let node_binding = Uuid::now_v7();
    let manager_binding = Uuid::now_v7();
    for (binding, node, profile) in [
        (node_binding, Some("worker-1"), "SwarmNode"),
        (manager_binding, None, "Ordinary"),
    ] {
        sqlx::query("INSERT INTO edgeagentbindings(id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid,profile,dockernodeid) VALUES($1,$2,$1,'fixture','Connected',$3,$3,$4,$5)")
            .bind(binding).bind(binding.to_string()).bind(id).bind(profile).bind(node).execute(&fixture.pool).await.unwrap();
    }
    let bootstrap = Uuid::now_v7();
    sqlx::query("INSERT INTO swarmnodeagentbootstraps(id,platformid,clusterid,createdbyactorid,dockersecretname,expiresatutc,tokenhash,version) VALUES($1,$2,$5,$3,'bootstrap',now()+interval '1 hour',$4,1)")
        .bind(bootstrap).bind(id).bind(SYSTEM_ACTOR_ID).bind(bootstrap.to_string()).bind(&cluster).execute(&fixture.pool).await.unwrap();
    let url = format!("/api/v1/platforms/{id}/node-agents");
    let send = |principal: Option<ActorPrincipal>| async {
        let mut req = Request::builder()
            .method(Method::DELETE)
            .uri(&url)
            .body(Body::empty())
            .unwrap();
        if let Some(principal) = principal {
            req.extensions_mut().insert(principal);
        }
        fixture.app.clone().oneshot(req).await.unwrap()
    };
    assert_eq!(send(None).await.status(), StatusCode::UNAUTHORIZED);
    let mut reader = fixture.administrator.clone();
    reader.actor_id = ActorId::new(fixture.actor_id);
    reader.roles.clear();
    assert_eq!(
        send(Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE resourceaccesses SET permissionlevel=4 WHERE resourceid=$1 AND actorid=$2")
        .bind(id)
        .bind(fixture.actor_id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(
        send(Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN,
        "Execute alone cannot manage node agents"
    );
    sqlx::query(
        "UPDATE resourceaccesses SET specificpermissions=$3 WHERE resourceid=$1 AND actorid=$2",
    )
    .bind(id)
    .bind(fixture.actor_id)
    .bind(citadel_primitives::SpecificPermission::ManageNodeAgents as i32)
    .execute(&fixture.pool)
    .await
    .unwrap();
    let response = send(Some(reader)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let progress = json_body(response).await;
    assert!(
        progress
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["stage"].is_string())
    );
    assert_eq!(
        progress.as_array().unwrap().last().unwrap()["isCompleted"],
        true,
        "{progress}"
    );
    assert!(
        progress.as_array().unwrap().last().unwrap()["errorMessage"].is_null(),
        "{progress}"
    );
    let persisted: (String, String) = sqlx::query_as(
        "SELECT desiredstate,operationstate FROM swarmnodeagentinstallations WHERE platformid=$1",
    )
    .bind(id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(persisted, ("Removed".into(), "Completed".into()));
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT revokedatutc IS NOT NULL FROM edgeagentbindings WHERE id=$1"
        )
        .bind(node_binding)
        .fetch_one(&fixture.pool)
        .await
        .unwrap()
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT revokedatutc IS NULL FROM edgeagentbindings WHERE id=$1"
        )
        .bind(manager_binding)
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        "do not revoke the manager's control-plane connection"
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT revokedatutc IS NOT NULL FROM swarmnodeagentbootstraps WHERE id=$1"
        )
        .bind(bootstrap)
        .fetch_one(&fixture.pool)
        .await
        .unwrap()
    );
    let statuses:Vec<String>=sqlx::query_scalar("SELECT status FROM activityevents WHERE resourceid=$1 AND eventtype='PlatformNodeAgentLifecycle' ORDER BY createdat").bind(id).fetch_all(&fixture.pool).await.unwrap();
    assert_eq!(statuses, vec!["Information", "Success"]);
    // The existing browser event is sent only to an authorized Platform group.
    use citadel_server::realtime_groups::{Group, GroupReadPort};
    let groups = super::realtime_groups::reader(&fixture);
    let group = Group::parse(&format!("docker-daemon:{id}")).unwrap();
    let hub = citadel_server::realtime::RealtimeHub::new(
        4,
        Arc::new(citadel_server::metrics::Metrics::default()),
    );
    let mut events = hub.subscribe();
    hub.publish_runtime_change(id, "nodeAgentCoverage", "update", id.to_string());
    let change = events.recv().await.unwrap();
    assert!(group.affected_by(&change));
    assert!(
        !Group::parse(&format!("docker-daemon:{}", Uuid::now_v7()))
            .unwrap()
            .affected_by(&change)
    );
    let updated = groups
        .read(&fixture.administrator, &group, Some(&change))
        .await
        .unwrap();
    assert_eq!(updated.events[0].target, "SwarmNodeAgentCoverageChanged");
    assert_eq!(updated.events[0].arguments, vec![json!(id)]);
    let mut denied = fixture.administrator.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert!(groups.read(&denied, &group, Some(&change)).await.is_err());
    // CAS prevents both a competing operation and a delayed prior completion.
    let info = PlatformRuntimePort::get_info(
        &fixture.lookup_state.platforms.docker,
        &tokio_util::sync::CancellationToken::new(),
    )
    .await
    .unwrap();
    let store = PostgresNodeAgentLifecycleStore(fixture.pool.clone());
    let old = store
        .claim_remove(ActorId::new(SYSTEM_ACTOR_ID), id, info.clone())
        .await
        .unwrap();
    assert!(
        store
            .claim_remove(ActorId::new(SYSTEM_ACTOR_ID), id, info.clone())
            .await
            .is_err()
    );
    sqlx::query("UPDATE swarmnodeagentinstallations SET operationstartedatutc=now()-interval '31 minutes' WHERE platformid=$1").bind(id).execute(&fixture.pool).await.unwrap();
    let platform = fixture
        .lookup_state
        .platforms
        .platforms
        .get_platform(id)
        .await
        .unwrap()
        .unwrap();
    let coverage = citadel_adapters::node_agent_coverage::read(
        &fixture.pool,
        &fixture.lookup_state.platforms.edge,
        &platform,
    )
    .await
    .unwrap();
    assert_eq!(
        coverage.operation.unwrap().state,
        "Failed",
        "expired operations must not keep the UI actions disabled"
    );
    let current = store
        .claim_remove(ActorId::new(SYSTEM_ACTOR_ID), id, info.clone())
        .await
        .unwrap();
    assert!(store.finish(&old, None).await.is_err());
    assert!(store.revoke(&old).await.is_err());
    store
        .finish(&current, Some("fixture failure"))
        .await
        .unwrap();
    let event: String=sqlx::query_scalar("SELECT info FROM activityevents WHERE resourceid=$1 AND eventtype='PlatformNodeAgentLifecycle' ORDER BY createdat DESC LIMIT 1").bind(id).fetch_one(&fixture.pool).await.unwrap();
    let event: Value = serde_json::from_str(&event).unwrap();
    assert_eq!(event["$type"], "PlatformNodeAgentLifecycle");
    assert_eq!(event["State"], "Failed");
    let mut wrong_manager = info.clone();
    wrong_manager.daemon_id = "replacement-daemon".into();
    assert!(
        store
            .claim_remove(ActorId::new(SYSTEM_ACTOR_ID), id, wrong_manager)
            .await
            .is_err()
    );
    // A nonexistent actor makes persistence fail: no partial operation replaces the prior claim.
    assert!(
        store
            .claim_remove(ActorId::new(Uuid::now_v7()), id, info)
            .await
            .is_err()
    );
    let after: Uuid = sqlx::query_scalar(
        "SELECT operationid FROM swarmnodeagentinstallations WHERE platformid=$1",
    )
    .bind(id)
    .fetch_one(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(after, current.operation_id);
    fixture.docker_server.abort();
}
