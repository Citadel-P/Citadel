use super::*;
use citadel_platforms::network_usage::*;

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn network_usage_filters_owners_and_saved_references_by_actor_and_node() {
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
    let owner = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'Private application',$2,'{\"Networks\":[\"shared\"]}','Healthy',$3)")
        .bind(owner).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,'Private Stack',$2,'WebEditor','{}','{}')")
        .bind(stack).bind(actor).execute(&pool).await.unwrap();
    let compose = "services:\n  app:\n    image: nginx\n    networks: [shared]\nnetworks:\n  shared:\n    external: true\n";
    sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')")
        .bind(release).bind(actor).bind(platform).bind(json!({"composeFile":compose})).bind(stack).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&pool)
        .await
        .unwrap();
    let container = Uuid::now_v7();
    for (id, node, deployment) in [
        (container, "node-a", Some(owner)),
        (Uuid::now_v7(), "node-b", None),
    ] {
        sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated,dockernodeid,deploymentid) VALUES($1,0,'same-runtime-id','image',$2,'{}','private-container','Running',0,$3,$4)")
            .bind(id).bind(platform).bind(node).bind(deployment).execute(&pool).await.unwrap();
    }
    let reader = PostgresPlatformReader::new(pool.clone());
    let inputs = vec![
        NetworkUsageQuery {
            id: "network-a".into(),
            name: "shared".into(),
            scope: "local".into(),
            docker_node_id: Some("node-a".into()),
            container_ids: vec!["same-runtime-id".into()],
            stack_project: None,
        },
        NetworkUsageQuery {
            id: "cluster".into(),
            name: "shared".into(),
            scope: "swarm".into(),
            docker_node_id: None,
            container_ids: vec!["same-runtime-id".into()],
            stack_project: None,
        },
    ];
    let denied = reader
        .network_usage(ActorId::new(actor), false, platform, &inputs)
        .await
        .unwrap();
    assert!(denied.iter().all(|usage| usage.entries.is_empty()));
    let admin = reader
        .network_usage(ActorId::new(actor), true, platform, &inputs)
        .await
        .unwrap();
    assert_eq!(
        admin[0].entries.len(),
        1,
        "worker-local names must not match manager saved configuration"
    );
    assert_eq!(admin[0].entries[0].container_id, Some(container));
    assert_eq!(
        admin[0].entries[0].container_state.as_deref(),
        Some("Running")
    );
    assert!(!admin[0].entries[0].is_stale);
    assert_eq!(admin[0].entries[0].workload_id, Some(owner));
    assert_eq!(
        admin[0].entries[0].workload_status.as_deref(),
        Some("Healthy")
    );
    assert!(!admin[0].entries[0].workload_processing);
    assert_eq!(
        admin[1]
            .entries
            .iter()
            .find(|entry| entry.workload_id == Some(stack))
            .unwrap()
            .workload_status
            .as_deref(),
        Some("Healthy")
    );
    assert_eq!(
        admin[1].entries.len(),
        2,
        "only saved Deployment and Stack references; ambiguous container identity is omitted"
    );
    assert!(
        admin[1]
            .entries
            .iter()
            .all(|entry| entry.relationship == NetworkRelationship::SavedReference)
    );
    assert!(admin.iter().all(|usage| usage.is_partial));
    sqlx::query("UPDATE containers SET state='Paused',projectionstalesince=1 WHERE id=$1")
        .bind(container)
        .execute(&pool)
        .await
        .unwrap();
    let stale = reader
        .network_usage(ActorId::new(actor), true, platform, &inputs)
        .await
        .unwrap();
    assert_eq!(
        stale[0].entries[0].container_state.as_deref(),
        Some("Paused")
    );
    assert!(stale[0].entries[0].is_stale);
    assert!(
        stale[1]
            .entries
            .iter()
            .all(|entry| entry.container_state.is_none())
    );
    let visible = reader
        .network_containers(ActorId::new(actor), false, platform, None)
        .await
        .unwrap();
    assert!(visible.iter().all(|c| c.container_id != container));
    assert!(
        reader
            .network_containers(ActorId::new(actor), false, platform, Some(container))
            .await
            .unwrap()
            .is_empty()
    );
    let grant = Uuid::now_v7();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,1,0)")
        .bind(grant).bind(actor).bind(owner).execute(&pool).await.unwrap();
    let allowed = reader
        .network_usage(ActorId::new(actor), false, platform, &inputs)
        .await
        .unwrap();
    assert_eq!(
        reader
            .network_containers(ActorId::new(actor), false, platform, Some(container))
            .await
            .unwrap()[0]
            .workload_id,
        Some(owner)
    );
    assert_eq!(allowed[0].entries.len(), 1);
    assert_eq!(allowed[1].entries.len(), 1);
    assert_eq!(allowed[1].entries[0].workload_id, Some(owner));
    sqlx::query("DELETE FROM resourceaccesses WHERE id=$1")
        .bind(grant)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        reader
            .network_usage(ActorId::new(actor), false, platform, &inputs)
            .await
            .unwrap()
            .iter()
            .all(|usage| usage.entries.is_empty())
    );
    // A container can outlive its Citadel owner and retain its Docker labels.
    // Administrators may inspect it, but unresolved ownership must not grant
    // ordinary actors access through either the map or the attachment list.
    let orphan = Uuid::now_v7();
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated,hascitadelownershiplabels) VALUES($1,0,'orphan-runtime-id','image',$2,'{}','orphan','Running',0,true)")
        .bind(orphan).bind(platform).execute(&pool).await.unwrap();
    let orphan_network = [NetworkUsageQuery {
        id: "orphan-network".into(),
        name: "orphan-network".into(),
        scope: "local".into(),
        docker_node_id: None,
        container_ids: vec!["orphan-runtime-id".into()],
        stack_project: None,
    }];
    for administrator in [false, true] {
        let selected = reader
            .network_containers(ActorId::new(actor), administrator, platform, Some(orphan))
            .await
            .unwrap();
        assert_eq!(selected.len(), usize::from(administrator));
        if administrator {
            assert_eq!(selected[0].container_id, orphan);
            assert!(selected[0].workload_id.is_none());
        }
        let catalog = reader
            .network_containers(ActorId::new(actor), administrator, platform, None)
            .await
            .unwrap();
        assert_eq!(
            catalog.iter().any(|entry| entry.container_id == orphan),
            administrator
        );
        let usage = reader
            .network_usage(
                ActorId::new(actor),
                administrator,
                platform,
                &orphan_network,
            )
            .await
            .unwrap();
        assert_eq!(usage[0].entries.len(), usize::from(administrator));
        if administrator {
            assert_eq!(usage[0].entries[0].container_id, Some(orphan));
            assert!(usage[0].entries[0].workload_id.is_none());
        }
    }
    // Administrator visibility must still be scoped to the requested platform.
    let other_platform = Uuid::now_v7();
    assert!(
        reader
            .network_containers(ActorId::new(actor), true, other_platform, Some(orphan))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        reader
            .network_usage(ActorId::new(actor), true, other_platform, &orphan_network)
            .await
            .unwrap()[0]
            .entries
            .is_empty()
    );

    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated) SELECT gen_random_uuid(),0,'public-'||n,'image',$1,'{}','public-'||n,'Running',0 FROM generate_series(1,51) n")
        .bind(platform).execute(&pool).await.unwrap();
    let many = [NetworkUsageQuery {
        id: "many".into(),
        name: "many".into(),
        scope: "local".into(),
        docker_node_id: None,
        container_ids: (1..=51).map(|n| format!("public-{n}")).collect(),
        stack_project: None,
    }];
    let limited = reader
        .network_usage(ActorId::new(actor), false, platform, &many)
        .await
        .unwrap();
    assert_eq!(limited[0].entries.len(), 50);
    assert!(limited[0].truncated);
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,configids,desiredtaskcount,image,labels,mode,name,networkids,observedat,ports,runningtaskcount,secretids,updatestate,versionindex,ownership,stackid) VALUES($1,'service-full-id','[]',2,'nginx','{}','Replicated','private-service','[\"cluster\"]',now(),'[]',0,'[]','',1,'StackOwned',$2)")
        .bind(platform).bind(stack).execute(&pool).await.unwrap();
    let hidden = reader
        .network_usage(ActorId::new(actor), false, platform, &inputs)
        .await
        .unwrap();
    assert!(
        hidden[1]
            .entries
            .iter()
            .all(|entry| entry.docker_service_id.is_none())
    );
    let services = reader
        .network_usage(ActorId::new(actor), true, platform, &inputs)
        .await
        .unwrap();
    let service = services[1]
        .entries
        .iter()
        .find(|entry| entry.relationship == NetworkRelationship::ServiceNetwork)
        .unwrap();
    assert_eq!(
        service.docker_service_id.as_deref(),
        Some("service-full-id")
    );
    assert!(service.container_id.is_none());
    assert!(service.observed_at.is_some());
    sqlx::query("UPDATE deployments SET status='Degraded',controlstate='Processing' WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let catalog = reader
        .network_containers(ActorId::new(actor), true, platform, Some(container))
        .await
        .unwrap();
    assert_eq!(catalog[0].workload_status.as_deref(), Some("Degraded"));
    assert!(catalog[0].workload_processing);

    let managed_service = Uuid::now_v7();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,desiredspechash,health,spec,synchronizationstate,updatedat,dockerserviceid) VALUES($1,'managed-service','managed-service',$2,$3,'hash','Healthy','{}','InSync',now(),'service-full-id')")
        .bind(managed_service).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("UPDATE swarmserviceprojections SET stackid=NULL,swarmserviceid=$1,ownership='CitadelService',runningtaskcount=1 WHERE platformid=$2 AND dockerserviceid='service-full-id'")
        .bind(managed_service).bind(platform).execute(&pool).await.unwrap();
    for (stale, status) in [(false, "Degraded"), (true, "Unknown")] {
        sqlx::query("UPDATE swarmserviceprojections SET isstale=$1 WHERE platformid=$2 AND dockerserviceid='service-full-id'")
            .bind(stale).bind(platform).execute(&pool).await.unwrap();
        let usage = reader
            .network_usage(ActorId::new(actor), true, platform, &inputs)
            .await
            .unwrap();
        let entry = usage[1]
            .entries
            .iter()
            .find(|entry| entry.workload_id == Some(managed_service))
            .unwrap();
        assert_eq!(entry.workload_kind, Some(NetworkWorkloadKind::SwarmService));
        assert_eq!(entry.workload_status.as_deref(), Some(status));
        let hidden = reader
            .network_usage(ActorId::new(actor), false, platform, &inputs)
            .await
            .unwrap();
        assert!(hidden.iter().all(|usage| {
            usage
                .entries
                .iter()
                .all(|entry| entry.workload_id != Some(managed_service))
        }));
    }

    // Manager task containers are stored in daemon scope (NULL), workers in node scope.
    // Service labels are not inherited by task containers, so missing this join leaks
    // a private service's tasks through the unmanaged-container branch.
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\",\"nodeID\":\"manager\"}' WHERE id=$1")
        .bind(platform).execute(&pool).await.unwrap();
    let manager_container = Uuid::now_v7();
    let worker_container = Uuid::now_v7();
    for (id, node, state) in [
        (manager_container, None, "Running"),
        (worker_container, Some("worker"), "Paused"),
    ] {
        sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated,dockernodeid,isswarmtask) VALUES($1,0,'task-container','image',$2,'{}','private-task',$3,0,$4,true)")
            .bind(id).bind(platform).bind(state).bind(node).execute(&pool).await.unwrap();
    }
    for node in ["manager", "worker"] {
        sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockercontainerid,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,slot,state,versionindex) VALUES($1,$2,'running','task-container',$2,'service-full-id','image','private-task',$2,now(),'[]','service',1,'running',1)")
            .bind(platform).bind(node).execute(&pool).await.unwrap();
    }
    let task_networks: Vec<_> = [None, Some("worker")]
        .into_iter()
        .map(|node| NetworkUsageQuery {
            id: "task-network".into(),
            name: "task-network".into(),
            scope: "local".into(),
            docker_node_id: node.map(str::to_owned),
            container_ids: vec!["task-container".into()],
            stack_project: None,
        })
        .collect();
    for id in [manager_container, worker_container] {
        assert!(
            reader
                .network_containers(ActorId::new(actor), false, platform, Some(id))
                .await
                .unwrap()
                .is_empty()
        );
    }
    assert!(
        reader
            .network_usage(ActorId::new(actor), false, platform, &task_networks)
            .await
            .unwrap()
            .iter()
            .all(|usage| usage.entries.is_empty())
    );
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,20,0)")
        .bind(Uuid::now_v7()).bind(actor).bind(managed_service).execute(&pool).await.unwrap();
    // Exact reads must remain usable when the picker catalog is truncated.
    sqlx::query("UPDATE containers SET name='zzz-private-task' WHERE id IN ($1,$2)")
        .bind(manager_container)
        .bind(worker_container)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated) SELECT gen_random_uuid(),0,'catalog-'||n,'image',$1,'{}','catalog-'||n,'Running',0 FROM generate_series(1,501) n")
        .bind(platform).execute(&pool).await.unwrap();
    let catalog = reader
        .network_containers(ActorId::new(actor), false, platform, None)
        .await
        .unwrap();
    assert_eq!(catalog.len(), 501);
    assert!(catalog.iter().all(
        |item| item.container_id != manager_container && item.container_id != worker_container
    ));
    let usage = reader
        .network_usage(ActorId::new(actor), false, platform, &task_networks)
        .await
        .unwrap();
    for (index, (id, state)) in [(manager_container, "Running"), (worker_container, "Paused")]
        .into_iter()
        .enumerate()
    {
        assert_eq!(usage[index].entries.len(), 1);
        assert_eq!(usage[index].entries[0].container_id, Some(id));
        assert_eq!(usage[index].entries[0].workload_id, Some(managed_service));
        let selected = reader
            .network_containers(ActorId::new(actor), false, platform, Some(id))
            .await
            .unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].workload_id, Some(managed_service));
        assert_eq!(selected[0].container_state.as_deref(), Some(state));
        assert!(!selected[0].is_stale);
    }
    sqlx::query("UPDATE containers SET projectionstalesince=1 WHERE id=$1")
        .bind(worker_container)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        reader
            .network_containers(ActorId::new(actor), false, platform, Some(worker_container))
            .await
            .unwrap()[0]
            .is_stale
    );
    pool.close().await;
}
