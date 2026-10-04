//! Node identity fixture uses a real worker/task and manager-observed projections.
use super::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use citadel_contracts::citadel::platforms::v1::CheckHealthResponse;
use serde_json::Value;

pub(super) async fn remove_stack(project: &str) {
    // Service removal is asynchronous; tasks can briefly retain the overlay network.
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let output = docker_output(&["stack", "rm", project]).await;
            if output.status.success() {
                return;
            }
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("Failed to remove network") && error.contains("is in use by task"),
                "docker stack rm {project}: {error}"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .expect("Swarm tasks must release the fixture network before cleanup times out");
}

pub(super) async fn check(
    pool: &sqlx::PgPool,
    store: &PostgresEdgeStore,
    registry: &EdgeRegistry,
    core_url: &str,
    root: &std::path::Path,
) {
    let worker = std::env::var("CITADEL_COMPAT_WORKER_HOST").unwrap();
    let manager: Value =
        serde_json::from_str(&docker(&["info", "--format", "{{json .}}"]).await).unwrap();
    let node: Value =
        serde_json::from_str(&docker(&["--host", &worker, "info", "--format", "{{json .}}"]).await)
            .unwrap();
    let node_id = node["Swarm"]["NodeID"].as_str().unwrap();
    let hostname = node["Name"].as_str().unwrap();
    let cluster = manager["Swarm"]["Cluster"]["ID"].as_str().unwrap();
    assert_ne!(node_id, manager["Swarm"]["NodeID"].as_str().unwrap());
    assert_eq!(node["Swarm"]["ControlAvailable"], false);
    let platform = Uuid::now_v7();
    let target = EdgeTarget::node(platform, node_id.into());
    let name = format!("compat-node-{}", platform.simple());
    let labels = serde_json::json!({
        "com.citadel.system":"true", "com.citadel.system-role":"swarm-node-agent",
        "com.citadel.platform-id":platform.to_string(), "com.citadel.swarm-cluster-id":cluster
    });
    // The task supplies a manager-verifiable identity; the Agent executable runs in
    // the test runner against the worker daemon so Core remains reachable locally.
    let mut args = vec![
        "service".to_owned(),
        "create".into(),
        "--detach".into(),
        "--no-resolve-image".into(),
        "--name".into(),
        name.clone(),
        "--constraint".into(),
        format!("node.id=={node_id}"),
    ];
    for (key, value) in labels.as_object().unwrap() {
        args.extend([
            "--label".into(),
            format!("{key}={}", value.as_str().unwrap()),
        ]);
    }
    args.extend([
        "compatibility-base:local".into(),
        "sleep".into(),
        "300".into(),
    ]);
    let service = docker(&args.iter().map(String::as_str).collect::<Vec<_>>()).await;
    let task = tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            let ids = docker(&[
                "service",
                "ps",
                "-q",
                "--no-trunc",
                "--filter",
                "desired-state=running",
                &service,
            ])
            .await;
            for id in ids.lines() {
                let task: Value =
                    serde_json::from_str(&docker(&["inspect", "--type", "task", id]).await)
                        .unwrap();
                if task[0]["Status"]["State"] == "running" {
                    assert_eq!(task[0]["NodeID"], node_id);
                    return id.to_owned();
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("worker fixture task must start");
    let token = Uuid::now_v7().to_string();
    let descriptor =
        serde_json::json!({"$type":"DockerSwarm", "nodeID":manager["Swarm"]["NodeID"]});
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount,clusterid) VALUES($1,$1::text,'Local',0,0,0,$1::text,0,$2,'Online',0,$3)")
        .bind(platform).bind(descriptor).bind(cluster).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeagentbootstraps(id,platformid,clusterid,createdbyactorid,dockersecretname,expiresatutc,tokenhash,version) VALUES($1,$2,$3,$4,'bootstrap',now()+interval '1 hour',$5,1)")
        .bind(Uuid::now_v7()).bind(platform).bind(cluster).bind(citadel_identity::SYSTEM_ACTOR_ID).bind(URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeagentinstallations(platformid,agentimagedigest,agentimagereference,clusterid,desiredstate,dockerserviceid,dockerservicename,managerdockerdaemonid,managerdockernodeid) VALUES($1,'fixture','fixture',$2,'Installed',$3,$4,$5,$6)")
        .bind(platform).bind(cluster).bind(&service).bind(&name).bind(manager["ID"].as_str().unwrap()).bind(manager["Swarm"]["NodeID"].as_str().unwrap()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmnodeprojections(platformid,dockernodeid,address,architecture,availability,desiredtaskcount,engineversion,hostname,isleader,labels,observedat,operatingsystem,reachability,role,runningtaskcount,status,versionindex) VALUES($1,$2,$3,$4,'active',1,$5,$6,false,'{}',now(),'linux','reachable','worker',1,'ready',1)")
        .bind(platform).bind(node_id).bind(node["Swarm"]["NodeAddr"].as_str().unwrap()).bind(node["Architecture"].as_str().unwrap()).bind(node["ServerVersion"].as_str().unwrap()).bind(hostname).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,configids,desiredtaskcount,image,labels,mode,name,networkids,observedat,ports,runningtaskcount,secretids,updatestate,versionindex) VALUES($1,$2,'[]',1,'fixture',$3,'replicated',$4,'[]',now(),'[]',1,'[]','completed',1)")
        .bind(platform).bind(&service).bind(&labels).bind(&name).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,state,versionindex) VALUES($1,$2,'running',$3,$4,'fixture',$2,$5,now(),'[]',$6,'running',1)")
        .bind(platform).bind(&task).bind(node_id).bind(&service).bind(hostname).bind(&name).execute(pool).await.unwrap();
    let bootstrap = root.join("bootstrap");
    std::fs::write(&bootstrap, token).unwrap();
    let identity = root.join("node.identity");
    let key = root.join("node.key");
    let mut values = HashMap::from([
        ("DOCKER_HOST", worker),
        ("CITADEL_AGENT_MODE", "edge".into()),
        ("CITADEL_CORE_URL", core_url.into()),
        ("CITADEL_EDGE_AGENT_PROFILE", "swarm-node".into()),
        (
            "CITADEL_EDGE_BOOTSTRAP_FILE",
            bootstrap.display().to_string(),
        ),
        ("CITADEL_EDGE_AGENT_KEY_PATH", key.display().to_string()),
        ("CITADEL_EDGE_IDENTITY_PATH", identity.display().to_string()),
        ("CITADEL_PLATFORM_ID", platform.to_string()),
        ("CITADEL_SWARM_SERVICE_ID", service.clone()),
        ("CITADEL_SWARM_TASK_ID", "wrong-task".into()),
        ("CITADEL_SWARM_NODE_ID", node_id.into()),
        ("CITADEL_SWARM_NODE_HOSTNAME", hostname.into()),
        ("CITADEL_SWARM_CLUSTER_ID", cluster.into()),
    ]);
    let (mut running, _) = start_agent(&values).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(
        registry.get(&target).is_err(),
        "unverified task must not enroll"
    );
    assert!(!identity.exists());
    running.kill().await.unwrap();
    values.insert("CITADEL_SWARM_TASK_ID", task);
    (running, _) = start_agent(&values).await;
    let first = current_session(registry, &target, None).await;
    let persisted_key = std::fs::read(&key).unwrap();
    running.kill().await.unwrap();
    // A reconnect uses the persisted identity, even after the bootstrap is removed.
    std::fs::remove_file(&bootstrap).unwrap();
    (running, _) = start_agent(&values).await;
    let next = current_session(registry, &target, Some(first.id)).await;
    assert_eq!(next.agent_id, first.agent_id);
    assert_eq!(std::fs::read(&key).unwrap(), persisted_key);
    let wire = Wire::Edge(next.clone());
    let health: CheckHealthResponse = wire.unary(Kind::PlatformCheckHealth, "", ()).await;
    assert!(health.healthy);
    let forbidden = next.command(
        Kind::ImageBuildStream,
        BuildImageRequest::default().encode_to_vec(),
        Duration::from_secs(5),
        true,
    );
    assert!(forbidden.is_err(), "Swarm node must reject builds");
    running.kill().await.unwrap();
    store.revoke(&target).await.unwrap();
    docker(&["service", "rm", &service]).await;
    println!(
        "swarm-node: real worker identity, wrong-task rejection, persisted reconnect and restricted commands passed"
    );
}
