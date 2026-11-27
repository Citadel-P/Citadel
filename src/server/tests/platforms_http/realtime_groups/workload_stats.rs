use super::scoped_fixture::*;
use super::*;

fn stat(docker: String) -> citadel_platforms::RuntimeContainerStat {
    citadel_platforms::RuntimeContainerStat {
        docker_container_id: docker,
        created: 100,
        memory_active: 1.,
        memory_limit: 10.,
        memory_cache: 0.,
        memory_active_cache: None,
        cpu_usage: 2.,
        rx_bytes: 0.,
        tx_bytes: 0.,
        block_read_bytes: None,
        block_write_bytes: None,
    }
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn deployment_samples_stay_scoped_across_inventory_growth_and_replacement() {
    let f = fixture().await;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'metric workload',$2,'{}','Healthy',$3)")
        .bind(id).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let restricted = super::super::lookup::subject(&f).await;
    grant(
        &f,
        restricted.actor_id.value(),
        ResourceType::Deployment,
        id,
        0,
    )
    .await;
    let member:Uuid=sqlx::query_scalar("INSERT INTO containers(id,name,platformid,dockercontainerid,dockerimageid,state,created,updated,ports,deploymentid) VALUES(gen_random_uuid(),'current',$1,'selected','image','Running',1,1,'[]',$2) RETURNING id").bind(f.platform_id).bind(id).fetch_one(&f.pool).await.unwrap();
    let (hub, reads, address, cancel, server) = serve(&f).await;
    let group = format!("deployment-stats:{id}");
    let mut socket = connect(&f, &restricted, address).await;
    let mut previous_size = 0;
    for (start, size) in [(1, 10), (11, 1000)] {
        sqlx::query("INSERT INTO containers(id,name,platformid,dockercontainerid,dockerimageid,state,created,updated,ports) SELECT gen_random_uuid(),'other-'||i,$1,'other-'||i,'image','Running',1,1,'[]' FROM generate_series($2::int,$3::int)i").bind(f.platform_id).bind(start).bind(size).execute(&f.pool).await.unwrap();
        let mut lock = f.pool.begin().await.unwrap();
        sqlx::query("LOCK TABLE backuppolicies, images, stacks IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *lock)
            .await
            .unwrap();
        assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
        let initial = receive(&mut socket).await;
        assert_eq!(initial["target"], "WorkloadStatsMembership");
        assert_eq!(initial["arguments"][0]["members"], json!([member]));
        assert_eq!(reads.take(), [group.as_str()]);
        sqlx::query("LOCK TABLE containers IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *lock)
            .await
            .unwrap();
        let batch = (1..=size)
            .map(|i| stat(format!("other-{i}")))
            .chain([stat("selected".into())])
            .collect::<Vec<_>>();
        hub.publish_container_stats(f.platform_id, &batch);
        barrier(&hub);
        let events = read_until_barrier(&mut socket).await;
        assert_eq!(events.len(), 1);
        let rows = events[0]["arguments"][0]["stats"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["containerId"], json!(member));
        let bytes = serde_json::to_vec(&events[0]).unwrap().len();
        if previous_size > 0 {
            assert_eq!(bytes, previous_size);
        }
        lock.rollback().await.unwrap();
        previous_size = bytes;
        assert!(reads.take().is_empty());
        eprintln!(
            "{size} unrelated containers: one selected sample, {bytes} event bytes, zero event snapshot reads"
        );
        assert!(invoke(&mut socket, "LeaveGroup", &group).await["error"].is_null());
    }
    join(&mut socket, &group, 1).await;
    let replacement:Uuid=sqlx::query_scalar("UPDATE containers SET deploymentid=$2,updated=100 WHERE platformid=$1 AND dockercontainerid='other-1' RETURNING id").bind(f.platform_id).bind(id).fetch_one(&f.pool).await.unwrap();
    hub.publish_resource_change("Deployment", id, "updated");
    assert_eq!(
        receive(&mut socket).await["arguments"][0]["members"],
        json!([replacement])
    );
    hub.publish_container_stats(
        f.platform_id,
        &[stat("selected".into()), stat("other-1".into())],
    );
    assert_eq!(
        receive(&mut socket).await["arguments"][0]["stats"][0]["containerId"],
        json!(replacement)
    );
    assert!(invoke(&mut socket, "LeaveGroup", &group).await["error"].is_null());
    hub.publish_container_stats(f.platform_id, &[stat("other-1".into())]);
    barrier(&hub);
    assert!(read_until_barrier(&mut socket).await.is_empty());
    assert!(
        invoke(
            &mut socket,
            "JoinGroup",
            &format!("deployment-stats:{}", Uuid::now_v7())
        )
        .await["error"]
            .is_string()
    );
    socket.close(None).await.unwrap();
    cancel.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn service_samples_require_both_grants_and_complete_current_task_membership() {
    let (f, _) = super::super::swarm_inventory::fixture_swarm().await;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,spec,health,desiredspechash,synchronizationstate,updatedat,dockerserviceid) VALUES($1,'metrics','metrics',$2,$3,'{}','Healthy','hash','InSync',now(),'metric-svc')").bind(id).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,configids,desiredtaskcount,image,labels,mode,name,networkids,observedat,ports,runningtaskcount,secretids,updatestate,versionindex) VALUES($1,'metric-svc','[]',2,'image','{}','Replicated','metrics','[]',now(),'[]',2,'[]','',1)").bind(f.platform_id).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockercontainerid,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,slot,state,versionindex)
        SELECT $1,'metric-'||i,'running','replica-'||i,'node','metric-svc','image','task','node',now(),'[]','metrics',i,'running',1 FROM generate_series(1,2)i").bind(f.platform_id).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,name,platformid,dockercontainerid,dockerimageid,dockernodeid,state,created,updated,ports) SELECT gen_random_uuid(),'replica-'||i,$1,'replica-'||i,'image','node','Running',1,1,'[]' FROM generate_series(1,3)i").bind(f.platform_id).execute(&f.pool).await.unwrap();
    let principal = super::super::lookup::subject(&f).await;
    grant(
        &f,
        principal.actor_id.value(),
        ResourceType::SwarmService,
        id,
        0,
    )
    .await;
    let (hub, _, address, cancel, server) = serve(&f).await;
    let group = format!("service-stats:{id}");
    let mut socket = connect(&f, &principal, address).await;
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_string());
    socket.close(None).await.unwrap();
    grant(
        &f,
        principal.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let mut socket = connect(&f, &principal, address).await;
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_null());
    let initial = receive(&mut socket).await;
    let members = initial["arguments"][0]["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    let batch = (1..=3)
        .map(|n| stat(format!("replica-{n}")))
        .collect::<Vec<_>>();
    hub.publish_scoped_container_stats(f.platform_id, Some("node"), &batch);
    let values = receive(&mut socket).await;
    let stats = values["arguments"][0]["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 2);
    assert!(
        stats
            .iter()
            .all(|r| members.contains(&r["containerId"]) && r.get("dockerContainerId").is_none())
    );
    sqlx::query("UPDATE swarmserviceprojections SET desiredtaskcount=3 WHERE platformid=$1 AND dockerserviceid='metric-svc'").bind(f.platform_id).execute(&f.pool).await.unwrap();
    hub.publish_runtime_change(f.platform_id, "service", "update", "metric-svc");
    let changed = receive(&mut socket).await;
    assert_eq!(changed["arguments"][0]["expected"], 3);
    assert_eq!(changed["arguments"][0]["members"], json!([]));
    hub.publish_scoped_container_stats(f.platform_id, Some("node"), &batch);
    barrier(&hub);
    assert!(read_until_barrier(&mut socket).await.is_empty());
    remove_platform_access(&f, &principal).await;
    hub.publish_scoped_container_stats(f.platform_id, Some("node"), &batch);
    let closed = tokio::time::timeout(StdDuration::from_secs(5), socket.next())
        .await
        .unwrap();
    assert!(
        matches!(closed, Some(Ok(Message::Close(_))) | None),
        "{closed:?}"
    );
    let mut socket = connect(&f, &principal, address).await;
    assert!(invoke(&mut socket, "JoinGroup", &group).await["error"].is_string());
    socket.close(None).await.unwrap();
    cancel.cancel();
    server.await.unwrap();
    cleanup(f).await;
}
