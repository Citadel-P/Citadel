use super::*;
use citadel_platforms::network_usage::{
    NetworkRelationship, NetworkUsageQuery, NetworkWorkloadKind,
};

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn stopped_workloads_resolve_configured_networks_without_endpoints_and_preserve_authorization()
 {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'Stopped deployment',$2,'{\"Networks\":[\"shared\"]}','Stopped',$3)")
        .bind(deployment).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,'Renamed Stack',$2,'WebEditor','{}','{}')")
        .bind(stack).bind(actor).execute(&pool).await.unwrap();
    let compose = "services:\n  app:\n    image: nginx\n    networks: [backend, shared]\n  worker:\n    image: redis\nnetworks:\n  backend: {}\n  shared:\n    external: true\n  dynamic:\n    name: ${NETWORK}\n";
    sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,$4,$5,'Stopped','1')")
        .bind(release).bind(actor).bind(platform).bind(json!({"composeFile":compose,"projectName":"frozen-project"})).bind(stack).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&pool)
        .await
        .unwrap();
    let mut networks: Vec<_> = [
        "frozen-project_default",
        "frozen-project_backend",
        "shared",
        "other-project_default",
        "frozen-project_dynamic",
    ]
    .into_iter()
    .map(|name| NetworkUsageQuery {
        id: format!("id-{name}"),
        name: name.into(),
        scope: "local".into(),
        docker_node_id: None,
        container_ids: vec![],
        stack_project: None,
    })
    .collect();
    for network in &mut networks[..2] {
        network.stack_project = Some("frozen-project".into());
    }
    // The same saved project identity supports a retained Swarm overlay.
    networks[1].scope = "swarm".into();
    networks[1].name = "custom-overlay-name".into();
    networks[3].stack_project = Some("other-project".into());

    networks.push(NetworkUsageQuery {
        docker_node_id: Some("worker-node".into()),
        ..networks[0].clone()
    });
    let reader = PostgresPlatformReader::new(pool.clone());
    let denied = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(denied.iter().all(|usage| usage.entries.is_empty()));
    let admin = reader
        .network_usage(ActorId::new(actor), true, platform, &networks)
        .await
        .unwrap();
    for usage in &admin[..2] {
        assert_eq!(usage.entries.len(), 1);
        let entry = &usage.entries[0];
        assert_eq!(entry.workload_id, Some(stack));
        assert_eq!(entry.relationship, NetworkRelationship::SavedReference);
        assert_eq!(entry.workload_status.as_deref(), Some("Stopped"));
        assert!(entry.container_id.is_none());
        assert!(entry.docker_container_id.is_none());
    }
    assert_eq!(admin[2].entries.len(), 2);
    assert!(admin[3..].iter().all(|usage| usage.entries.is_empty()));
    let grant = Uuid::now_v7();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,1,0)")
        .bind(grant).bind(actor).bind(deployment).execute(&pool).await.unwrap();
    let allowed = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(allowed[..2].iter().all(|usage| usage.entries.is_empty()));
    assert_eq!(allowed[2].entries.len(), 1);
    assert_eq!(
        allowed[2].entries[0].workload_kind,
        Some(NetworkWorkloadKind::Deployment)
    );
    sqlx::query("UPDATE resourceaccesses SET resourceid=$1,resourcetype=2 WHERE id=$2")
        .bind(stack)
        .bind(grant)
        .execute(&pool)
        .await
        .unwrap();
    let allowed = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(
        allowed[..3]
            .iter()
            .all(|usage| usage.entries.len() == 1 && usage.entries[0].workload_id == Some(stack))
    );
    // Git releases have no inline Compose. Runtime project labels still
    // identify retained project networks, without fetching source files.
    sqlx::query("UPDATE stackreleases SET spec=spec-'composeFile' WHERE id=$1")
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    let git = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(git[..2].iter().all(|usage| usage.entries.len() == 1));
    assert!(git[2..].iter().all(|usage| usage.entries.is_empty()));
    // Without an explicit project name, use the same normalization as Stack execution.
    sqlx::query("UPDATE stackreleases SET spec=spec-'projectName' WHERE id=$1")
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    for network in &mut networks[..2] {
        network.stack_project = Some("renamed-stack".into());
    }
    let fallback = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(fallback[..2].iter().all(|usage| usage.entries.len() == 1));
    sqlx::query("UPDATE deployments SET spec='{}' WHERE id=$1")
        .bind(deployment)
        .execute(&pool)
        .await
        .unwrap();
    let bridge = [NetworkUsageQuery {
        id: "default-bridge-id".into(),
        name: "bridge".into(),
        scope: "local".into(),
        docker_node_id: None,
        container_ids: vec![],
        stack_project: None,
    }];
    let hidden_bridge = reader
        .network_usage(ActorId::new(actor), false, platform, &bridge)
        .await
        .unwrap();
    assert!(hidden_bridge[0].entries.is_empty());
    sqlx::query("UPDATE resourceaccesses SET resourceid=$1,resourcetype=1 WHERE id=$2")
        .bind(deployment)
        .bind(grant)
        .execute(&pool)
        .await
        .unwrap();
    let default_bridge = reader
        .network_usage(ActorId::new(actor), false, platform, &bridge)
        .await
        .unwrap();
    assert_eq!(default_bridge[0].entries.len(), 1);
    assert_eq!(default_bridge[0].entries[0].workload_id, Some(deployment));
    assert_eq!(
        default_bridge[0].entries[0].relationship,
        NetworkRelationship::SavedReference
    );
    sqlx::query("DELETE FROM resourceaccesses WHERE id=$1")
        .bind(grant)
        .execute(&pool)
        .await
        .unwrap();
    let revoked = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(revoked.iter().all(|usage| usage.entries.is_empty()));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn swarm_networks_survive_scale_to_zero_and_runtime_removal() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    let service = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,desiredspechash,health,spec,synchronizationstate,updatedat,dockerserviceid) VALUES($1,'worker','worker',$2,$3,'hash','Stopped','{\"networkIds\":[\"overlay-id\"]}','InSync',now(),'docker-worker')")
        .bind(service).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,configids,desiredtaskcount,image,labels,mode,name,networkids,observedat,ports,runningtaskcount,secretids,updatestate,versionindex,ownership,swarmserviceid) VALUES($1,'docker-worker','[]',0,'nginx','{}','Replicated','worker','[\"overlay-id\"]',now(),'[]',0,'[]','',1,'CitadelService',$2)")
        .bind(platform).bind(service).execute(&pool).await.unwrap();
    let networks = [NetworkUsageQuery {
        id: "overlay-id".into(),
        name: "shared".into(),
        scope: "swarm".into(),
        docker_node_id: None,
        container_ids: vec![],
        stack_project: None,
    }];
    let reader = PostgresPlatformReader::new(pool.clone());
    let hidden = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(hidden[0].entries.is_empty());
    let grant = Uuid::now_v7();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,20,0)")
        .bind(grant).bind(actor).bind(service).execute(&pool).await.unwrap();
    for replicas in [2, 0] {
        sqlx::query("UPDATE swarmserviceprojections SET runningtaskcount=$1,desiredtaskcount=$1 WHERE platformid=$2")
            .bind(replicas).bind(platform).execute(&pool).await.unwrap();
        let usage = reader
            .network_usage(ActorId::new(actor), false, platform, &networks)
            .await
            .unwrap();
        assert_eq!(usage[0].entries.len(), 2);
        assert!(
            usage[0]
                .entries
                .iter()
                .all(|entry| entry.workload_id == Some(service) && entry.container_id.is_none())
        );
        let observed = usage[0]
            .entries
            .iter()
            .find(|entry| entry.relationship == NetworkRelationship::ServiceNetwork)
            .unwrap();
        assert_eq!(observed.docker_service_id.as_deref(), Some("docker-worker"));
        if replicas == 0 {
            assert_eq!(observed.workload_status.as_deref(), Some("Stopped"));
        }
    }
    sqlx::query("DELETE FROM swarmserviceprojections WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let saved = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert_eq!(saved[0].entries.len(), 1);
    assert_eq!(
        saved[0].entries[0].relationship,
        NetworkRelationship::SavedReference
    );
    assert_eq!(saved[0].entries[0].workload_id, Some(service));
    assert!(saved[0].entries[0].docker_service_id.is_none());
    sqlx::query("DELETE FROM resourceaccesses WHERE id=$1")
        .bind(grant)
        .execute(&pool)
        .await
        .unwrap();
    let revoked = reader
        .network_usage(ActorId::new(actor), false, platform, &networks)
        .await
        .unwrap();
    assert!(revoked[0].entries.is_empty());
    pool.close().await;
}
