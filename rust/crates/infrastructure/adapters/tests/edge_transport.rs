use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use citadel_adapters::connectors::edge::EdgeIntake;
use citadel_adapters::connectors::edge::EdgeRegistry;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore;
use citadel_contracts::citadel::edge::v1::{
    AgentEnvelope, AgentHello, AuthChallengeResponse, CommandCompleted, EdgeCommandKind,
    EnrollmentRequest, agent_envelope, core_envelope,
    edge_agent_service_client::EdgeAgentServiceClient,
    edge_agent_service_server::EdgeAgentServiceServer,
};
use citadel_database::MigrationRunner;
use citadel_identity::SYSTEM_ACTOR_ID;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const CAPABILITIES: &str =
    r#"{"commands":["platform.checkHealth","containers.list","containers.logs"]}"#;

async fn setup() -> (PgPool, PostgresEdgeStore, EdgeTarget) {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").expect("CITADEL_PHASE7_DATABASE_URL");
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let target = EdgeTarget::platform(Uuid::now_v7());
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'EdgeAgent',0,0,0,$2,0,'{\"$type\":\"DockerStandalone\"}','Offline',0)")
        .bind(target.resource_id).bind(format!("edge-{}", target.resource_id)).execute(&pool).await.unwrap();
    (pool.clone(), PostgresEdgeStore::new(pool), target)
}
fn enrollment(token: String, key: &SigningKey, daemon: String) -> EnrollmentRequest {
    EnrollmentRequest {
        enrollment_token: token,
        public_key: key.verifying_key().to_bytes().to_vec(),
        protocol_version: 2,
        capabilities_json: CAPABILITIES.into(),
        daemon_id: daemon,
        ..Default::default()
    }
}
fn envelope(body: agent_envelope::Body) -> AgentEnvelope {
    AgentEnvelope {
        body: Some(body),
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn manager_inventory_pins_cluster_and_rejects_identity_changes() {
    use citadel_platforms::{
        RuntimeInventorySnapshot, RuntimePlatformInfo, RuntimeSwarmInfo, RuntimeSwarmInventory,
    };
    let (pool, store, target) = setup().await;
    sqlx::query(
        "UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\"}' WHERE id=$1",
    )
    .bind(target.platform_id)
    .execute(&pool)
    .await
    .unwrap();
    let (_, token, _) = store
        .create_enrollment(&target, SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    let key = SigningKey::from_bytes(&[29; 32]);
    let binding = store
        .enroll(&enrollment(token, &key, Uuid::now_v7().to_string()))
        .await
        .unwrap();
    let (session, _receiver) = EdgeRegistry::default()
        .register(target.clone(), binding.agent_id)
        .unwrap();
    store
        .connected(&binding, session.connected_at)
        .await
        .unwrap();
    let cluster = Uuid::now_v7().to_string();
    let snapshot = RuntimeInventorySnapshot {
        platform_id: target.platform_id,
        observed_at: chrono::Utc::now(),
        info: RuntimePlatformInfo {
            daemon_id: binding.daemon_id,
            swarm: Some(RuntimeSwarmInfo {
                node_id: "manager".into(),
                cluster_id: Some(cluster.clone()),
                control_available: true,
                local_node_state: "active".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        containers: vec![],
        images: vec![],
        networks: vec![],
        volumes: vec![],
        swarm: Some(RuntimeSwarmInventory::default()),
    };
    store.persist_inventory(&session, &snapshot).await.unwrap();
    for change in ["cluster", "manager", "role", "type"] {
        let mut invalid = snapshot.clone();
        invalid.observed_at += chrono::Duration::seconds(1);
        let swarm = invalid.info.swarm.as_mut().unwrap();
        match change {
            "cluster" => swarm.cluster_id = Some("different-cluster".into()),
            "manager" => swarm.node_id = "different-manager".into(),
            "role" => swarm.control_available = false,
            _ => {
                invalid.info.swarm = None;
                invalid.swarm = None;
            }
        }
        assert!(
            store.persist_inventory(&session, &invalid).await.is_err(),
            "{change}"
        );
    }
    let saved: (String, String) =
        sqlx::query_as("SELECT clusterid,platformdescriptor->>'nodeID' FROM platforms WHERE id=$1")
            .bind(target.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(saved, (cluster, "manager".into()));
    store.persist_inventory(&session, &snapshot).await.unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn build_pool_enrollment_requires_build_capabilities_and_preserves_pool_identity() {
    let (pool, store, _) = setup().await;
    let target = EdgeTarget::build_pool(Uuid::now_v7());
    sqlx::query("INSERT INTO buildagentpools(id,name,normalizedname,createdbyactorid,provider,providerspec) VALUES($1,$2,$2,$3,'SelfManagedVm','{\"$type\":\"SelfManagedVm\",\"ConnectionMode\":\"EdgeAgent\"}')")
        .bind(target.resource_id).bind(target.resource_id.to_string()).bind(SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    let (_, token, _) = store
        .create_enrollment(&target, SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).unwrap();
    let key = SigningKey::from_bytes(&bytes);
    let mut request = enrollment(token, &key, Uuid::now_v7().to_string());
    assert!(
        store.enroll(&request).await.is_err(),
        "ordinary capabilities are insufficient for a builder"
    );
    request.capabilities_json = serde_json::json!({"commands":["platform.checkHealth","containers.list","containers.logs","images.build","images.push","images.checkBuildHost"]}).to_string();
    let binding = store.enroll(&request).await.unwrap();
    assert_eq!(binding.target, target);
    assert!(binding.target.platform_id.is_nil());
    assert!(
        store.enroll(&request).await.is_err(),
        "Pool enrollment is single-use"
    );
    let mut hello = AgentHello {
        agent_id: binding.agent_id.to_string(),
        agent_fingerprint: sqlx::query_scalar(
            "SELECT agentfingerprint FROM edgeagentbindings WHERE agentid=$1",
        )
        .bind(binding.agent_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        platform_id: Uuid::nil().to_string(),
        resource_id: target.resource_id.to_string(),
        resource_type: 1,
        daemon_id: request.daemon_id,
        protocol_version: 2,
        capabilities_json: request.capabilities_json,
        ..Default::default()
    };
    assert_eq!(store.reconnect(&hello).await.unwrap().target, target);
    hello.resource_id = Uuid::now_v7().to_string();
    assert!(
        store.reconnect(&hello).await.is_err(),
        "a builder cannot reconnect as another Pool"
    );
    hello.resource_id = target.resource_id.to_string();
    store.revoke(&target).await.unwrap();
    assert!(store.reconnect(&hello).await.is_err());
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn node_enrollment_requires_manager_verified_installation_and_exact_task_identity() {
    let (pool, store, target) = setup().await;
    let platform = target.platform_id;
    let token = Uuid::now_v7().to_string();
    let cluster = Uuid::now_v7().to_string();
    let service_id = Uuid::now_v7().to_string();
    sqlx::query("UPDATE platforms SET connectortype='Local',clusterid=$2,platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"manager\"}' WHERE id=$1")
        .bind(platform).bind(&cluster).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeagentbootstraps(id,platformid,clusterid,createdbyactorid,dockersecretname,expiresatutc,tokenhash,version) VALUES($1,$2,$3,$4,'bootstrap',now()+interval '1 hour',$5,1)")
        .bind(Uuid::now_v7()).bind(platform).bind(&cluster).bind(SYSTEM_ACTOR_ID).bind(URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeagentinstallations(platformid,agentimagedigest,agentimagereference,clusterid,desiredstate,dockerserviceid,dockerservicename,managerdockerdaemonid,managerdockernodeid) VALUES($1,'digest','agent',$2,'Installed',$3,'node-agents','daemon-manager','manager')")
        .bind(platform).bind(&cluster).bind(&service_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeprojections(platformid,dockernodeid,address,architecture,availability,desiredtaskcount,engineversion,hostname,isleader,labels,observedat,operatingsystem,reachability,role,runningtaskcount,status,versionindex) VALUES($1,'worker','10.0.0.2','amd64','active',1,'29','worker-host',false,'{}',now(),'linux','reachable','worker',1,'ready',1)")
        .bind(platform).execute(&pool).await.unwrap();
    let labels = serde_json::json!({"com.citadel.system":"true","com.citadel.system-role":"swarm-node-agent","com.citadel.platform-id":platform,"com.citadel.swarm-cluster-id":cluster});
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,configids,desiredtaskcount,image,labels,mode,name,networkids,observedat,ports,runningtaskcount,secretids,updatestate,versionindex) VALUES($1,$3,'[]',1,'agent',$2,'global','node-agents','[]',now(),'[]',1,'[]','completed',1)")
        .bind(platform).bind(&labels).bind(&service_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,state,versionindex) VALUES($1,'agent-task','running','worker',$2,'agent','agent-task','worker-host',now(),'[]','node-agents','running',1)")
        .bind(platform).bind(&service_id).execute(&pool).await.unwrap();
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).unwrap();
    let key = SigningKey::from_bytes(&bytes);
    let mut request = enrollment(token, &key, Uuid::now_v7().to_string());
    request.profile = 1;
    request.cluster_id = cluster;
    request.node_id = "worker".into();
    request.docker_hostname = "worker-host".into();
    request.swarm_role = "worker".into();
    request.service_id = service_id;
    request.task_id = "wrong-task".into();
    request.capabilities_json = serde_json::json!({"commands":["platform.checkHealth","containers.list","containers.logs","platform.getInfo","platform.events","containers.inspect","containers.patch","containers.delete","containers.stats","containers.exec"]}).to_string();
    assert!(store.enroll(&request).await.is_err());
    request.task_id = "agent-task".into();
    request.node_id = "manager".into();
    assert!(store.enroll(&request).await.is_err());
    request.node_id = "worker".into();
    sqlx::query("UPDATE swarmtaskprojections SET isstale=true WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    assert!(store.enroll(&request).await.is_err());
    sqlx::query("UPDATE swarmtaskprojections SET isstale=false WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmserviceprojections SET labels=labels-'com.citadel.swarm-cluster-id' WHERE platformid=$1")
        .bind(platform).execute(&pool).await.unwrap();
    assert!(
        store.enroll(&request).await.is_err(),
        "a service from another cluster cannot enroll a node"
    );
    sqlx::query("UPDATE swarmserviceprojections SET labels=$2 WHERE platformid=$1")
        .bind(platform)
        .bind(labels)
        .execute(&pool)
        .await
        .unwrap();
    let binding = store.enroll(&request).await.unwrap();
    assert_eq!(binding.target, EdgeTarget::node(platform, "worker".into()));
    assert!(
        store.enroll(&request).await.is_err(),
        "duplicate active node identity"
    );
    verify_same_node_task_replacement(&pool, &store, &binding, &request).await;
    verify_node_projection_isolation(&pool, &store, &binding).await;
    let rebound = verify_node_rebind(&pool, &store, &binding, &request).await;
    store.revoke(&rebound).await.unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM edgeagentbindings WHERE platformid=$1 AND revokedatutc IS NULL",
    )
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
    pool.close().await;
}

// Repair/Upgrade replaces the Task while preserving the Node's state volume/key.
async fn verify_same_node_task_replacement(
    pool: &PgPool,
    store: &PostgresEdgeStore,
    binding: &citadel_adapters::persistence::postgres::platforms::edge::store::EdgeBinding,
    request: &EnrollmentRequest,
) {
    let mut hello = AgentHello {
        agent_id: binding.agent_id.to_string(),
        agent_fingerprint: sqlx::query_scalar(
            "SELECT agentfingerprint FROM edgeagentbindings WHERE agentid=$1",
        )
        .bind(binding.agent_id)
        .fetch_one(pool)
        .await
        .unwrap(),
        platform_id: binding.target.platform_id.to_string(),
        resource_id: binding.target.resource_id.to_string(),
        profile: 1,
        protocol_version: 2,
        capabilities_json: request.capabilities_json.clone(),
        daemon_id: request.daemon_id.clone(),
        cluster_id: request.cluster_id.clone(),
        node_id: request.node_id.clone(),
        docker_hostname: request.docker_hostname.clone(),
        swarm_role: request.swarm_role.clone(),
        service_id: request.service_id.clone(),
        task_id: "replacement-task".into(),
        ..Default::default()
    };
    assert!(
        store.reconnect(&hello).await.is_err(),
        "unknown Tasks cannot reconnect"
    );
    sqlx::query("UPDATE swarmtaskprojections SET dockertaskid='replacement-task' WHERE platformid=$1 AND dockertaskid='agent-task'")
        .bind(binding.target.platform_id).execute(pool).await.unwrap();
    let candidate = store.reconnect(&hello).await.unwrap();
    let persisted: String =
        sqlx::query_scalar("SELECT lastobservedtaskid FROM edgeagentbindings WHERE agentid=$1")
            .bind(binding.agent_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(persisted, "agent-task", "Hello is not signature proof");
    sqlx::query("UPDATE swarmtaskprojections SET isstale=true WHERE platformid=$1")
        .bind(binding.target.platform_id)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        store
            .connected(&candidate, chrono::Utc::now())
            .await
            .is_err(),
        "membership must be rechecked after proof"
    );
    sqlx::query("UPDATE swarmtaskprojections SET isstale=false WHERE platformid=$1")
        .bind(binding.target.platform_id)
        .execute(pool)
        .await
        .unwrap();
    let connected_at = chrono::Utc::now();
    store.connected(&candidate, connected_at).await.unwrap();
    let persisted: (String, String) = sqlx::query_as(
        "SELECT dockernodeid,lastobservedtaskid FROM edgeagentbindings WHERE agentid=$1",
    )
    .bind(binding.agent_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(persisted, ("worker".into(), "replacement-task".into()));
    assert!(
        store
            .connected(&candidate, connected_at - chrono::Duration::seconds(1))
            .await
            .is_err(),
        "old sessions cannot overwrite a newer connection"
    );

    // Restore the fixture's task name through the same authenticated path.
    sqlx::query("UPDATE swarmtaskprojections SET dockertaskid='agent-task' WHERE platformid=$1 AND dockertaskid='replacement-task'")
        .bind(binding.target.platform_id).execute(pool).await.unwrap();
    assert!(
        store
            .connected(&candidate, chrono::Utc::now())
            .await
            .is_err(),
        "a superseded Task cannot reconnect"
    );
    hello.task_id = request.task_id.clone();
    store
        .connected(&store.reconnect(&hello).await.unwrap(), chrono::Utc::now())
        .await
        .unwrap();
}

// Ports SwarmNodeAgentLifecycleTests.Reconnect_ShouldRebindPersistedIdentity_WhenNodeRejoinsAfterRemovalGrace.
async fn verify_node_rebind(
    pool: &PgPool,
    store: &PostgresEdgeStore,
    binding: &citadel_adapters::persistence::postgres::platforms::edge::store::EdgeBinding,
    request: &EnrollmentRequest,
) -> EdgeTarget {
    let platform = binding.target.platform_id;
    let new_node = "worker-rejoined";
    sqlx::query("INSERT INTO swarmnodeprojections SELECT (jsonb_populate_record(NULL::swarmnodeprojections,to_jsonb(n)||jsonb_build_object('dockernodeid',$2::text))).* FROM swarmnodeprojections n WHERE platformid=$1 AND dockernodeid='worker'")
        .bind(platform).bind(new_node).execute(pool).await.unwrap();
    sqlx::query("UPDATE swarmtaskprojections SET dockernodeid=$2 WHERE platformid=$1 AND dockertaskid='agent-task'").bind(platform).bind(new_node).execute(pool).await.unwrap();
    let fingerprint: String =
        sqlx::query_scalar("SELECT agentfingerprint FROM edgeagentbindings WHERE agentid=$1")
            .bind(binding.agent_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let hello = AgentHello {
        agent_id: binding.agent_id.to_string(),
        agent_fingerprint: fingerprint,
        platform_id: platform.to_string(),
        resource_id: platform.to_string(),
        resource_type: 0,
        profile: 1,
        protocol_version: 2,
        capabilities_json: request.capabilities_json.clone(),
        daemon_id: request.daemon_id.clone(),
        cluster_id: request.cluster_id.clone(),
        node_id: new_node.into(),
        docker_hostname: request.docker_hostname.clone(),
        swarm_role: request.swarm_role.clone(),
        service_id: request.service_id.clone(),
        task_id: request.task_id.clone(),
        ..Default::default()
    };
    assert!(
        store.reconnect(&hello).await.is_err(),
        "the previous node is still a member"
    );
    sqlx::query("DELETE FROM swarmnodeprojections WHERE platformid=$1 AND dockernodeid='worker'")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        store.reconnect(&hello).await.is_err(),
        "removal grace has not elapsed"
    );
    sqlx::query("UPDATE edgeagentbindings SET lastheartbeatatutc=now()-interval '20 minutes',lastdisconnectedatutc=now()-interval '20 minutes',lastauthenticatedatutc=now()-interval '20 minutes',firstenrolledatutc=now()-interval '20 minutes',createdatutc=now()-interval '20 minutes' WHERE agentid=$1")
        .bind(binding.agent_id).execute(pool).await.unwrap();
    let mut invalid = hello.clone();
    invalid.daemon_id = "different-daemon".into();
    assert!(store.reconnect(&invalid).await.is_err());
    invalid = hello.clone();
    invalid.cluster_id = "different-cluster".into();
    assert!(store.reconnect(&invalid).await.is_err());
    invalid = hello.clone();
    invalid.task_id = "unknown-task".into();
    assert!(store.reconnect(&invalid).await.is_err());
    let candidate = store.reconnect(&hello).await.unwrap();
    let persisted: String =
        sqlx::query_scalar("SELECT dockernodeid FROM edgeagentbindings WHERE agentid=$1")
            .bind(binding.agent_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        persisted, "worker",
        "a Hello without proof must not change the binding"
    );
    sqlx::query("UPDATE swarmnodeprojections SET isstale=true WHERE platformid=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        store
            .connected(&candidate, chrono::Utc::now())
            .await
            .is_err(),
        "recheck membership after proof"
    );
    sqlx::query("UPDATE swarmnodeprojections SET isstale=false WHERE platformid=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
    store
        .connected(&candidate, chrono::Utc::now())
        .await
        .unwrap();
    let persisted: (String, String) = sqlx::query_as(
        "SELECT dockernodeid,lastobservedtaskid FROM edgeagentbindings WHERE agentid=$1",
    )
    .bind(binding.agent_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(persisted, (new_node.into(), "agent-task".into()));
    assert!(
        store.connected(binding, chrono::Utc::now()).await.is_err(),
        "old node identity is fenced"
    );
    store.reconnect(&hello).await.unwrap().target
}

async fn verify_node_projection_isolation(
    pool: &PgPool,
    store: &PostgresEdgeStore,
    binding: &citadel_adapters::persistence::postgres::platforms::edge::store::EdgeBinding,
) {
    use citadel_platforms::{
        RuntimeContainerStat, RuntimeInventorySnapshot, RuntimePlatformInfo, RuntimeSwarmInfo,
    };
    let registry = EdgeRegistry::default();
    let (session, _receiver) = registry
        .register(binding.target.clone(), binding.agent_id)
        .unwrap();
    store
        .connected(binding, session.connected_at)
        .await
        .unwrap();
    let platform = binding.target.platform_id;
    let observed = chrono::Utc::now();
    for node in ["worker", "other-worker"] {
        sqlx::query("INSERT INTO containers(id,platformid,dockernodeid,dockercontainerid,dockerimageid,name,created,updated,state,ports,projectionobservedat) VALUES($1,$2,$3,'shared-docker-id','image','container',1,1,'Running','[]',$4)")
            .bind(Uuid::now_v7()).bind(platform).bind(node).bind(observed.timestamp()+10).execute(pool).await.unwrap();
    }
    let snapshot = RuntimeInventorySnapshot {
        platform_id: platform,
        observed_at: observed,
        info: RuntimePlatformInfo {
            daemon_id: binding.daemon_id.clone(),
            swarm: Some(RuntimeSwarmInfo {
                node_id: "worker".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
        containers: vec![],
        images: vec![],
        networks: vec![],
        volumes: vec![],
        swarm: None,
    };
    // An older snapshot cannot delete a
    // Container observed by a newer event, nor another Node's resources.
    store.persist_inventory(&session, &snapshot).await.unwrap();
    verify_node_local_resources(pool, store, &session, &snapshot).await;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM containers WHERE platformid=$1")
        .bind(platform)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    let stats = [RuntimeContainerStat {
        docker_container_id: "shared-docker-id".into(),
        memory_active: 100.,
        memory_cache: 0.,
        memory_limit: 1000.,
        cpu_usage: 1.,
        rx_bytes: 2.,
        tx_bytes: 3.,
        created: observed.timestamp(),
    }];
    assert_eq!(
        store
            .persist_stats_with_disk(
                &session,
                &stats,
                citadel_platforms::HostDiskUsage::new(95, 100, 95.0)
            )
            .await
            .unwrap(),
        1
    );
    let nodes: Vec<String> = sqlx::query_scalar("SELECT container.dockernodeid FROM containerstats stat JOIN containers container ON container.id=stat.containerid WHERE container.platformid=$1").bind(platform).fetch_all(pool).await.unwrap();
    assert_eq!(nodes, ["worker"]);
    let platform_stats: i64 =
        sqlx::query_scalar("SELECT count(*) FROM platformstats WHERE platformid=$1")
            .bind(platform)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        platform_stats, 0,
        "worker data cannot overwrite manager statistics"
    );
    let mut newer = snapshot;
    newer.observed_at += chrono::Duration::seconds(20);
    store.persist_inventory(&session, &newer).await.unwrap();
    let nodes: Vec<String> =
        sqlx::query_scalar("SELECT dockernodeid FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_all(pool)
            .await
            .unwrap();
    assert_eq!(nodes, ["other-worker"]);
    registry.remove(&session);
    let (replacement, _receiver) = registry
        .register(binding.target.clone(), binding.agent_id)
        .unwrap();
    store
        .connected(binding, replacement.connected_at)
        .await
        .unwrap();
    assert!(
        store
            .persist_stats_with_disk(
                &session,
                &stats,
                citadel_platforms::HostDiskUsage::new(95, 100, 95.0)
            )
            .await
            .is_err()
    );
    assert!(
        store
            .persist_stats_with_disk(
                &session,
                &[],
                citadel_platforms::HostDiskUsage::new(95, 100, 95.0)
            )
            .await
            .is_err()
    );
    assert!(store.persist_inventory(&session, &newer).await.is_err());
    let stale_event = citadel_adapters::connectors::agent::client::AgentDaemonEvent {
        container: None,
        resource_type: "container",
        action: "destroy".into(),
        container_id: Some("new-container".into()),
        container_state: None,
        container_name: None,
    };
    assert!(
        store
            .persist_container_event(&session, &stale_event)
            .await
            .is_err()
    );

    sqlx::query("INSERT INTO containers(id,platformid,dockernodeid,dockercontainerid,dockerimageid,name,created,updated,state,ports) VALUES($1,$2,'worker','new-container','image','new',1,1,'Running','[]')")
        .bind(Uuid::now_v7()).bind(platform).execute(pool).await.unwrap();
    store
        .disconnected(binding.agent_id, session.connected_at)
        .await
        .unwrap();
    let stale: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM containers WHERE platformid=$1 AND projectionstalesince IS NOT NULL",
    )
    .bind(platform)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        stale, 0,
        "an old disconnect cannot stale the replacement's inventory"
    );
    store
        .disconnected(binding.agent_id, replacement.connected_at)
        .await
        .unwrap();
    let nodes: Vec<String> = sqlx::query_scalar("SELECT dockernodeid FROM containers WHERE platformid=$1 AND projectionstalesince IS NOT NULL")
        .bind(platform).fetch_all(pool).await.unwrap();
    assert_eq!(nodes, ["worker"], "disconnect must not stale another Node");
    assert!(store.persist_stats(&replacement, &stats).await.is_err());
}

// Ports SwarmPersistenceTests.NodeLocalResources_ShouldPreservePerNodeIdentity
// and the late-snapshot invariant from SwarmNodeDataPlaneJobTests.
async fn verify_node_local_resources(
    pool: &PgPool,
    store: &PostgresEdgeStore,
    session: &citadel_adapters::connectors::edge::EdgeSession,
    initial: &citadel_platforms::RuntimeInventorySnapshot,
) {
    use citadel_platforms::{RuntimeImageSummary, RuntimeNetworkSummary, RuntimeVolumeSummary};
    let mut snapshot = initial.clone();
    snapshot.observed_at += chrono::Duration::seconds(1);
    snapshot.images = vec![RuntimeImageSummary {
        id: "shared-image".into(),
        repo_tags: vec!["redis:latest".into()],
        repo_digests: vec!["redis@sha256:abc".into()],
        ..Default::default()
    }];
    snapshot.volumes = vec![RuntimeVolumeSummary {
        name: "data".into(),
        ..Default::default()
    }];
    snapshot.networks = vec![RuntimeNetworkSummary {
        id: "bridge".into(),
        ..Default::default()
    }];
    snapshot.networks.push(RuntimeNetworkSummary {
        id: "overlay".into(),
        scope: "Swarm".into(),
        ..Default::default()
    });
    store.persist_inventory(session, &snapshot).await.unwrap();
    let platform = snapshot.platform_id;
    let image_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM swarmnodeimageprojections WHERE platformid=$1 AND dockernodeid='worker'",
    )
    .bind(platform)
    .fetch_one(pool)
    .await
    .unwrap();
    // Identical daemon IDs/names on another Node must remain independent.
    for query in [
        "INSERT INTO swarmnodeimageprojections SELECT gen_random_uuid(),contentidentity,dockerimageid,'other-worker',false,observedat,platformid,resource FROM swarmnodeimageprojections WHERE platformid=$1 AND dockernodeid='worker'",
        "INSERT INTO swarmnodevolumeprojections SELECT platformid,'other-worker',volumename,false,observedat,resource FROM swarmnodevolumeprojections WHERE platformid=$1 AND dockernodeid='worker'",
        "INSERT INTO swarmnodenetworkprojections SELECT platformid,'other-worker',dockernetworkid,false,observedat,resource FROM swarmnodenetworkprojections WHERE platformid=$1 AND dockernodeid='worker'",
    ] {
        sqlx::query(query)
            .bind(platform)
            .execute(pool)
            .await
            .unwrap();
    }
    let old = snapshot.clone();
    snapshot.observed_at += chrono::Duration::seconds(1);
    snapshot.images[0].repo_tags = vec!["redis:updated".into()];
    store.persist_inventory(session, &snapshot).await.unwrap();
    store.persist_inventory(session, &old).await.unwrap();
    let (stable_id, tags, identity): (Uuid, serde_json::Value, String) = sqlx::query_as("SELECT id,resource->'repo_tags',contentidentity FROM swarmnodeimageprojections WHERE platformid=$1 AND dockernodeid='worker'")
        .bind(platform).fetch_one(pool).await.unwrap();
    assert_eq!(stable_id, image_id);
    assert_eq!(tags, serde_json::json!(["redis:updated"]));
    assert_eq!(identity, "redis@sha256:abc");
    {
        use citadel_platforms::PlatformReader;
        let images =
            citadel_adapters::persistence::postgres::platforms::PostgresPlatformReader::new(
                pool.clone(),
            )
            .list_images(platform)
            .await
            .unwrap();
        assert_eq!(images.len(), 2);
        let image = images
            .iter()
            .find(|image| image.docker_node_id.as_deref() == Some("worker"))
            .unwrap();
        assert_eq!(image.id, image_id);
        assert_eq!(image.node_hostname.as_deref(), Some("worker-host"));
        assert_eq!(
            image.repo_digests.as_deref(),
            Some(["redis@sha256:abc".into()].as_slice())
        );
        assert!(!image.is_stale);
        let store = citadel_adapters::persistence::postgres::platforms::PostgresPlatformReader::new(
            pool.clone(),
        );
        let volumes = store.list_node_volumes(platform).await.unwrap();
        assert_eq!(volumes.len(), 2);
        let networks = store.list_node_networks(platform).await.unwrap();
        assert_eq!(
            networks.len(),
            2,
            "cluster overlays must not be duplicated per Node"
        );
        assert!(
            networks
                .iter()
                .all(|network| network.resource.id == "bridge")
        );
    }

    // A failed resource batch must roll back the watermark and all projections.
    let mut invalid = snapshot.clone();
    invalid.observed_at += chrono::Duration::seconds(1);
    invalid.images[0].repo_tags = vec!["must-not-persist".into()];
    invalid.volumes.push(invalid.volumes[0].clone());
    assert!(store.persist_inventory(session, &invalid).await.is_err());
    let tags: serde_json::Value = sqlx::query_scalar("SELECT resource->'repo_tags' FROM swarmnodeimageprojections WHERE platformid=$1 AND dockernodeid='worker'")
        .bind(platform).fetch_one(pool).await.unwrap();
    assert_eq!(tags, serde_json::json!(["redis:updated"]));

    snapshot.observed_at = invalid.observed_at;
    snapshot.images.clear();
    snapshot.volumes.clear();
    snapshot.networks.clear();
    store.persist_inventory(session, &snapshot).await.unwrap();
    store.persist_inventory(session, &old).await.unwrap();
    for query in [
        "SELECT dockernodeid FROM swarmnodeimageprojections WHERE platformid=$1",
        "SELECT dockernodeid FROM swarmnodevolumeprojections WHERE platformid=$1",
        "SELECT dockernodeid FROM swarmnodenetworkprojections WHERE platformid=$1",
    ] {
        let nodes: Vec<String> = sqlx::query_scalar(query)
            .bind(platform)
            .fetch_all(pool)
            .await
            .unwrap();
        assert_eq!(
            nodes,
            ["other-worker"],
            "late scan cannot resurrect deleted resources"
        );
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn persisted_edge_platform_routes_deployment_apply_without_using_local_docker() {
    use citadel_adapters::connectors::docker::DockerClient;
    use citadel_adapters::connectors::routing::deployments::DeploymentRuntimeRouter;
    use citadel_contracts::citadel::deployments::v1::{
        ApplyDeploymentRequest, ApplyDeploymentResponse, DeployedContainerState,
    };
    use citadel_deployments::{DeploymentRuntime, RuntimeContainerState, RuntimeDeploymentCommand};
    use prost::Message;
    let (pool, _store, target) = setup().await;
    sqlx::query("UPDATE platforms SET status='Online' WHERE id=$1")
        .bind(target.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    let registry = EdgeRegistry::default();
    let (session, mut receiver) = registry.register(target.clone(), Uuid::now_v7()).unwrap();
    let router = DeploymentRuntimeRouter::new(
        pool.clone(),
        DockerClient::new("/nonexistent/socket", Duration::from_secs(1)).unwrap(),
        None,
    )
    .with_edge(registry.clone());
    let command = RuntimeDeploymentCommand {
        deployment_id: Uuid::now_v7(), name: "edge-deployment".into(), image_id: "image".into(), environment_variables: vec!["RESOLVED=value".into()],
        spec: serde_json::from_value(serde_json::json!({"image":{"$type":"External","registryId":Uuid::nil(),"imageTag":"nginx"},"networks":["bridge"]})).unwrap(),
    };
    let cancellation = CancellationToken::new();
    let (result, ()) = tokio::join!(
        router.apply_container(target.platform_id, &command, &cancellation),
        async {
            let envelope = tokio::time::timeout(Duration::from_secs(3), receiver.recv())
                .await
                .unwrap()
                .unwrap();
            let id = Uuid::parse_str(&envelope.command_id).unwrap();
            let Some(core_envelope::Body::Command(request)) = envelope.body else {
                panic!("expected command")
            };
            assert_eq!(request.kind, EdgeCommandKind::DeploymentApply as i32);
            let request = ApplyDeploymentRequest::decode(request.payload.as_slice()).unwrap();
            let spec = request.spec.unwrap();
            assert_eq!(
                spec.labels["com.citadel.deployment-id"],
                command.deployment_id.to_string()
            );
            assert_eq!(spec.env_vars, ["RESOLVED=value"]);
            session.output(
                id,
                ApplyDeploymentResponse {
                    container_id: "edge-container".into(),
                    deployed_container_state: DeployedContainerState::Running as i32,
                }
                .encode_to_vec(),
            );
            session.complete(id, true);
        }
    );
    assert_eq!(result.unwrap().state, RuntimeContainerState::Running);
    registry.remove(&session);
    assert!(
        router
            .apply_container(target.platform_id, &command, &cancellation)
            .await
            .is_err()
    );
    pool.close().await;
}

// Ports EdgeAgentTests enrollment lifecycle and EdgeAgentRepositoryTests
// one-time consumption using PostgreSQL rather than an in-memory substitute.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn enrollment_is_hashed_atomic_expiring_and_revocable() {
    let (pool, store, target) = setup().await;
    let (id, token, _) = store
        .create_enrollment(&target, SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    let hash: String = sqlx::query_scalar("SELECT tokenhash FROM edgeagentenrollments WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(hash, token);
    assert_eq!(
        hash,
        URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
    );
    let key = SigningKey::from_bytes(&[29; 32]);
    let request = enrollment(token, &key, Uuid::now_v7().to_string());
    let mut incompatible = request.clone();
    incompatible.protocol_version = 1;
    assert!(store.enroll(&incompatible).await.is_err());
    let unused: bool =
        sqlx::query_scalar("SELECT usedatutc IS NULL FROM edgeagentenrollments WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(unused, "an incompatible Agent must not consume enrollment");
    let (a, b) = tokio::join!(store.enroll(&request), store.enroll(&request));
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let persisted_version: i32 =
        sqlx::query_scalar("SELECT protocolversion FROM edgeagentbindings WHERE platformid=$1")
            .bind(target.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(persisted_version, 2);
    store.revoke(&target).await.unwrap();
    assert!(store.enroll(&request).await.is_err());
    assert!(
        store
            .create_enrollment(&target, SYSTEM_ACTOR_ID)
            .await
            .is_err()
    );
    let (other_pool, store, target) = setup().await;
    let (id, token, _) = store
        .create_enrollment(&target, SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE edgeagentenrollments SET expiresatutc=now()-interval '1 minute' WHERE id=$1",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        store
            .enroll(&enrollment(token, &key, request.daemon_id))
            .await
            .is_err()
    );
    pool.close().await;
    other_pool.close().await;
}

// Exercise a real HTTP/2 bidirectional RPC with a protocol client,
// including Ed25519's little-endian timestamp + nonce challenge payload.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_DATABASE_URL"]
async fn grpc_enrollment_reconnect_command_and_revocation_lifecycle() {
    let (pool, store, target) = setup().await;
    let registry = EdgeRegistry::default();
    let service = EdgeAgentServiceServer::new(EdgeIntake::new(store.clone(), registry.clone()))
        .max_decoding_message_size(16 * 1024 * 1024 + 4096)
        .max_encoding_message_size(16 * 1024 * 1024 + 4096);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = CancellationToken::new();
    let stop = shutdown.clone();
    let task = tokio::spawn(async move {
        let incoming = async_stream::stream! { loop { yield listener.accept().await.map(|(socket,_)| socket); } };
        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming_shutdown(incoming, stop.cancelled())
            .await
            .unwrap();
    });
    let channel = tonic::transport::Endpoint::from_shared(format!("http://{address}"))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = EdgeAgentServiceClient::new(channel);
    let (_, token, _) = store
        .create_enrollment(&target, SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    let mut key_bytes = [0u8; 32];
    getrandom::fill(&mut key_bytes).unwrap();
    let key = SigningKey::from_bytes(&key_bytes);
    let daemon = Uuid::now_v7().to_string();
    let (send, mut receive) = mpsc::channel(8);
    send.send(envelope(agent_envelope::Body::EnrollmentRequest(
        enrollment(token.clone(), &key, daemon.clone()),
    )))
    .await
    .unwrap();
    let mut response = client
        .connect(
            async_stream::stream! { while let Some(item) = receive.recv().await { yield item; } },
        )
        .await
        .unwrap()
        .into_inner();
    let accepted = match response.message().await.unwrap().unwrap().body.unwrap() {
        core_envelope::Body::SessionAccepted(value) => value,
        _ => panic!("expected acceptance"),
    };
    assert_eq!(accepted.platform_id, target.platform_id.to_string());
    let previous = registry.get(&target).unwrap();
    let mut old_events = previous
        .command(
            EdgeCommandKind::PlatformDaemonEventsStream,
            vec![],
            Duration::from_secs(60),
            true,
        )
        .unwrap();
    let old_request = response.message().await.unwrap().unwrap();
    assert_eq!(old_request.command_id, old_events.id().to_string());
    let hex: String = Sha256::digest(key.verifying_key().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let hello = AgentHello {
        platform_id: accepted.platform_id.clone(),
        resource_id: accepted.resource_id.clone(),
        agent_id: accepted.agent_id.clone(),
        agent_fingerprint: format!("SHA256:{hex}"),
        daemon_id: daemon,
        protocol_version: 2,
        capabilities_json: CAPABILITIES.into(),
        ..Default::default()
    };
    // A reconnect cannot replace a live session until it proves key ownership.
    let (bad_send, mut bad_receive) = mpsc::channel(2);
    bad_send
        .send(envelope(agent_envelope::Body::Hello(hello.clone())))
        .await
        .unwrap();
    let mut bad_response = client
        .connect(async_stream::stream! {
            while let Some(message) = bad_receive.recv().await { yield message; }
        })
        .await
        .unwrap()
        .into_inner();
    let challenge = match bad_response.message().await.unwrap().unwrap().body.unwrap() {
        core_envelope::Body::AuthChallenge(value) => value,
        _ => panic!("expected challenge"),
    };
    bad_send
        .send(envelope(agent_envelope::Body::AuthChallengeResponse(
            AuthChallengeResponse {
                nonce: challenge.nonce,
                timestamp_unix_seconds: challenge.timestamp_unix_seconds,
                signature: vec![0; 64],
            },
        )))
        .await
        .unwrap();
    assert!(matches!(
        bad_response.message().await.unwrap().unwrap().body,
        Some(core_envelope::Body::SessionRejected(_))
    ));
    assert!(Arc::ptr_eq(&registry.get(&target).unwrap(), &previous));
    drop(bad_response);
    drop(bad_send);
    let (send2, mut receive2) = mpsc::channel(8);
    send2
        .send(envelope(agent_envelope::Body::Hello(hello)))
        .await
        .unwrap();
    let mut response2 = client
        .connect(
            async_stream::stream! { while let Some(item) = receive2.recv().await { yield item; } },
        )
        .await
        .unwrap()
        .into_inner();
    let challenge = match response2.message().await.unwrap().unwrap().body.unwrap() {
        core_envelope::Body::AuthChallenge(value) => value,
        _ => panic!("expected challenge"),
    };
    let mut payload = challenge.timestamp_unix_seconds.to_le_bytes().to_vec();
    payload.extend(&challenge.nonce);
    send2
        .send(envelope(agent_envelope::Body::AuthChallengeResponse(
            AuthChallengeResponse {
                nonce: challenge.nonce,
                timestamp_unix_seconds: challenge.timestamp_unix_seconds,
                signature: key.sign(&payload).to_bytes().to_vec(),
            },
        )))
        .await
        .unwrap();
    let accepted2 = match response2.message().await.unwrap().unwrap().body.unwrap() {
        core_envelope::Body::SessionAccepted(value) => value,
        _ => panic!("expected reconnect acceptance"),
    };
    assert!(previous.is_closed());
    drop(response);
    drop(send);
    let current = registry.get(&target).unwrap();
    assert!(!Arc::ptr_eq(&previous, &current));
    assert!(current.connected_at > previous.connected_at);
    assert!(
        tokio::time::timeout(
            Duration::from_secs(2),
            old_events.next(&CancellationToken::new())
        )
        .await
        .unwrap()
        .is_err(),
        "replacement must terminate the old daemon subscription"
    );
    let mut new_events = current
        .command(
            EdgeCommandKind::PlatformDaemonEventsStream,
            vec![],
            Duration::from_secs(60),
            true,
        )
        .unwrap();
    let new_request = response2.message().await.unwrap().unwrap();
    assert_eq!(new_request.command_id, new_events.id().to_string());
    use citadel_contracts::citadel::platforms::v1::{
        DaemonContainerEventResponse, DaemonEventResponse, daemon_event_response,
    };
    use prost::Message;
    let event = DaemonEventResponse {
        kind: Some(daemon_event_response::Kind::DaemonContainerEventResponse(
            DaemonContainerEventResponse {
                action: "die".into(),
                container_id: "container-after-reconnect".into(),
                container: None,
            },
        )),
        ..Default::default()
    };
    send2
        .send(AgentEnvelope {
            session_id: accepted2.session_id.clone(),
            command_id: new_request.command_id.clone(),
            body: Some(agent_envelope::Body::CommandOutput(
                citadel_contracts::citadel::edge::v1::CommandOutput {
                    payload: event.encode_to_vec(),
                },
            )),
            ..Default::default()
        })
        .await
        .unwrap();
    let bytes = tokio::time::timeout(
        Duration::from_secs(2),
        new_events.next(&CancellationToken::new()),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    let decoded = citadel_adapters::connectors::agent::client::decode_daemon_event(&bytes)
        .unwrap()
        .unwrap();
    assert_eq!(decoded.action, "die");
    assert_eq!(
        decoded.container_id.as_deref(),
        Some("container-after-reconnect")
    );
    // Storage uses the same session fence as commands: old samples cannot alter
    // the replacement's platform descriptor, even after successful re-enrollment.
    let sample = citadel_platforms::RuntimePlatformStats {
        image_used_bytes: Some(2048),
        volume_used_bytes: Some(4096),
        ..Default::default()
    };
    assert!(
        store
            .persist_stats_with_platform_stats(&previous, &[], Some(&sample))
            .await
            .is_err()
    );
    store
        .persist_stats_with_platform_stats(&current, &[], Some(&sample))
        .await
        .unwrap();
    let descriptor: serde_json::Value =
        sqlx::query_scalar("SELECT platformdescriptor::jsonb FROM platforms WHERE id=$1")
            .bind(target.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(descriptor["imageUsedBytes"], 2048);
    assert_eq!(descriptor["volumeUsedBytes"], 4096);
    // An old stream's late heartbeat/disconnect must not touch its replacement.
    store
        .disconnected(previous.agent_id, previous.connected_at)
        .await
        .unwrap();
    assert!(
        store
            .heartbeat(previous.agent_id, previous.connected_at)
            .await
            .is_err()
    );
    assert_eq!(
        store.status(&target).await.unwrap().connection_status,
        "Connected"
    );
    let mut command = current
        .command(
            EdgeCommandKind::ContainerInspect,
            vec![1],
            Duration::from_secs(3),
            false,
        )
        .unwrap();
    let outgoing = response2.message().await.unwrap().unwrap();
    assert_eq!(outgoing.command_id, command.id().to_string());
    send2
        .send(AgentEnvelope {
            session_id: accepted2.session_id,
            command_id: outgoing.command_id,
            body: Some(agent_envelope::Body::CommandCompleted(CommandCompleted {
                status_code: 0,
            })),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        command
            .next(&CancellationToken::new())
            .await
            .unwrap()
            .is_none()
    );
    store.revoke(&target).await.unwrap();
    registry.disconnect(&target);
    assert!(registry.get(&target).is_err());
    assert!(
        tokio::time::timeout(Duration::from_secs(3), response2.message())
            .await
            .unwrap()
            .unwrap()
            .is_none()
    );
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    pool.close().await;
}
