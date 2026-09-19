use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn node_agent_coverage_ports_dotnet_manager_worker_and_service_drift_cases() {
    let fixture = fixture().await;
    let id = fixture.platform_id;
    let cluster = format!("cluster-{id}");
    let service_id = format!("node-agent-{id}");
    let url = format!("/api/v1/platforms/{id}/node-agent-coverage");
    assert_eq!(
        send(&fixture, &url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let mut denied = fixture.administrator.clone();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    denied.roles.clear();
    assert_eq!(
        send(&fixture, &url, Some(denied)).await.status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE platforms SET clusterid=$2,platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"node-1\"}' WHERE id=$1").bind(id).bind(&cluster).execute(&fixture.pool).await.unwrap();
    sqlx::query("UPDATE swarmnodeprojections SET operatingsystem='linux',architecture='x86_64',availability='Active',status='Ready',isstale=false WHERE platformid=$1").bind(id).execute(&fixture.pool).await.unwrap();
    let get = || async {
        json_body(send(&fixture, &url, Some(fixture.administrator.clone())).await).await
    };
    let coverage = get().await;
    assert_eq!(coverage["state"], "Complete");
    assert_eq!(coverage["coveredNodes"], 1);
    assert_eq!(coverage["isInstalled"], false);
    assert_eq!(coverage["canManageNodeAgents"], true);
    let mut reader = fixture.administrator.clone();
    reader.actor_id = ActorId::new(fixture.actor_id);
    reader.roles.clear();
    let restricted = json_body(send(&fixture, &url, Some(reader)).await).await;
    assert_eq!(restricted["canManageNodeAgents"], false);
    sqlx::query("INSERT INTO swarmnodeprojections(platformid,dockernodeid,address,architecture,availability,desiredtaskcount,engineversion,hostname,isleader,labels,observedat,operatingsystem,reachability,role,runningtaskcount,status,versionindex) VALUES($1,'worker-1','10.0.0.2','x86_64','Active',0,'29.0','worker',false,'{}',now(),'linux','','Worker',0,'Down',1)").bind(id).execute(&fixture.pool).await.unwrap();
    let coverage = get().await;
    assert_eq!(coverage["state"], "NotInstalled");
    assert_eq!(coverage["eligibleNodes"], 2);
    assert_eq!(coverage["unsupportedNodes"], 0);
    assert_eq!(coverage["unschedulableNodes"], 1);
    assert_eq!(coverage["missingNodes"], 1);
    sqlx::query("INSERT INTO swarmnodeagentinstallations(platformid,agentimagedigest,agentimagereference,clusterid,desiredstate,dockerservicename,dockerserviceid,managerdockerdaemonid,managerdockernodeid) VALUES($1,'sha256:abc','agent@sha256:abc',$2,'Installed','citadel-node-agent',$3,'daemon','node-1')").bind(id).bind(&cluster).bind(&service_id).execute(&fixture.pool).await.unwrap();
    let coverage = get().await;
    assert_eq!(coverage["state"], "Partial");
    assert_eq!(coverage["reasons"], json!(["NodeAgentServiceDrifted"]));
    // Existing system Services must count for coverage even though inventory
    // tables hide them. A replacement with the same name is not the pinned ID.
    sqlx::query("UPDATE swarmserviceprojections SET dockerserviceid=$3,mode='Global',image='agent@sha256:abc',labels=$2 WHERE platformid=$1")
        .bind(id).bind(json!({"com.citadel.system":"true","com.citadel.system-role":"swarm-node-agent","com.citadel.platform-id":id,"com.citadel.swarm-cluster-id":cluster})).bind(&service_id).execute(&fixture.pool).await.unwrap();
    assert_eq!(get().await["reasons"], json!([]));
    sqlx::query(
        "UPDATE swarmserviceprojections SET image='agent@sha256:wrong' WHERE platformid=$1",
    )
    .bind(id)
    .execute(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(get().await["reasons"], json!(["NodeAgentServiceDrifted"]));
    for (kind, state) in [
        ("Install", "Installing"),
        ("Repair", "Installing"),
        ("Upgrade", "Installing"),
        ("Remove", "Removing"),
    ] {
        sqlx::query("UPDATE swarmnodeagentinstallations SET operationid=$2,operationkind=$3,operationstate='Running',operationstartedatutc=now() WHERE platformid=$1").bind(id).bind(Uuid::now_v7()).bind(kind).execute(&fixture.pool).await.unwrap();
        assert_eq!(get().await["state"], state);
    }
    sqlx::query("DELETE FROM swarmnodeprojections WHERE platformid=$1")
        .bind(id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    // A disconnected transport must surface its error, not silently report
    // missing Agents. No empty/failed collection is persisted.
    fixture.docker_server.abort();
    let unavailable = send(&fixture, &url, Some(fixture.administrator.clone())).await;
    assert!(!unavailable.status().is_success());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM swarmnodeprojections WHERE platformid=$1"
        )
        .bind(id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        0
    );
    sqlx::query(
        "UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerStandalone\"}' WHERE id=$1",
    )
    .bind(id)
    .execute(&fixture.pool)
    .await
    .unwrap();
    assert_eq!(
        send(&fixture, &url, Some(fixture.administrator.clone()))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn empty_node_coverage_initializes_inventory_once_and_fences_stale_initialization() {
    let cluster = format!("coverage-{}", Uuid::now_v7());
    let fixture = fixture_for_cluster(cluster.clone()).await;
    let id = fixture.platform_id;
    fixture.docker_server.abort();
    // get_info shares its version document with negotiation and includes the
    // existing visible-container count query: thirteen requests for the pass.
    let (docker, server, socket) = docker_fixture_for_cluster(13, cluster.clone()).await;
    // Replace the router's transport with the fixture using the same persisted
    // identity/permission services as the other Platform HTTP scenarios.
    let mut state = fixture.lookup_state.platforms.clone();
    state.docker = docker;
    let app = platforms_http::router(state);
    sqlx::query("UPDATE platforms SET clusterid=$2,platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"node-1\"}' WHERE id=$1").bind(id).bind(&cluster).execute(&fixture.pool).await.unwrap();
    sqlx::query("DELETE FROM swarmnodeprojections WHERE platformid=$1")
        .bind(id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let get = || async {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/platforms/{id}/node-agent-coverage"))
                    .extension(fixture.administrator.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = json_body(response).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    };
    let (first, concurrent) = tokio::join!(get(), get());
    assert_eq!(first["state"], "Complete");
    assert_eq!(
        concurrent["state"], "Complete",
        "concurrent initialization shares one bounded Docker enumeration"
    );
    tokio::time::timeout(StdDuration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    // No second Docker query is needed, even after the fixture transport exits.
    assert_eq!(get().await["coveredNodes"], 1);
    let store = PostgresInventoryProjectionStore::new(fixture.pool.clone());
    let mut older = snapshot(id);
    older.info.swarm = Some(citadel_platforms::RuntimeSwarmInfo {
        node_id: "node-1".into(),
        cluster_id: Some(cluster),
        local_node_state: "active".into(),
        control_available: true,
        ..Default::default()
    });
    older.swarm.as_mut().unwrap().nodes[0].hostname = "stale".into();
    assert!(!store.initialize_swarm(&older).await.unwrap());
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT hostname FROM swarmnodeprojections WHERE platformid=$1"
        )
        .bind(id)
        .fetch_one(&fixture.pool)
        .await
        .unwrap(),
        "manager"
    );
    older.info.swarm.as_mut().unwrap().cluster_id = Some("other-cluster".into());
    assert!(store.initialize_swarm(&older).await.is_err());
    std::fs::remove_file(socket).unwrap();
}
