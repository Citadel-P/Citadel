use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn container_owner_search_filters_grants_and_exact_runtime_identity() {
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
    let mut inventory = snapshot(platform, true);
    inventory.swarm.as_mut().unwrap().tasks[0].container_id = Some("task-container".into());
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&inventory)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    let service = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'Search Deployment',$2,'{}','Healthy',$3)")
        .bind(deployment).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,'Search Stack',$2,'WebEditor','{}','{}')")
        .bind(stack).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,'{}',$4,'Healthy','1')")
        .bind(release).bind(actor).bind(platform).bind(stack).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,dockerserviceid,platformid,createdbyactorid,desiredspechash,spec,health,synchronizationstate,updatedat) VALUES($1,'Search Service','web','service-1',$2,$3,'test','{}','Healthy','Unknown',now())")
        .bind(service).bind(platform).bind(actor).execute(&pool).await.unwrap();
    let deployment_container: Uuid = sqlx::query_scalar("UPDATE containers SET deploymentid=$1 WHERE platformid=$2 AND dockercontainerid='container-1' RETURNING id")
        .bind(deployment).bind(platform).fetch_one(&pool).await.unwrap();
    let stack_container = Uuid::now_v7();
    let task_container = Uuid::now_v7();
    let wrong_node_container = Uuid::now_v7();
    for (id, docker_id, node, stack_id) in [
        (stack_container, "stack-container", "node-1", Some(stack)),
        (task_container, "task-container", "node-1", None),
        (wrong_node_container, "task-container", "node-2", None),
    ] {
        sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated,dockernodeid,stackid) VALUES($1,0,$2,'image',$3,'{}',$2,'Running',0,$4,$5)")
            .bind(id).bind(docker_id).bind(platform).bind(node).bind(stack_id).execute(&pool).await.unwrap();
    }
    let ids = [
        deployment_container,
        stack_container,
        task_container,
        wrong_node_container,
    ];
    let reader = PostgresPlatformReader::new(pool.clone());
    let read = || reader.container_owner_names(ActorId::new(actor), false, &ids);
    assert!(
        read().await.unwrap().is_empty(),
        "platform visibility does not grant workload name visibility"
    );
    let all = reader
        .container_owner_names(ActorId::new(actor), true, &ids)
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[&deployment_container], vec!["Search Deployment"]);
    assert_eq!(all[&stack_container], vec!["Search Stack"]);
    assert_eq!(all[&task_container], vec!["Search Service"]);
    assert!(!all.contains_key(&wrong_node_container));

    // A direct resource grant exposes only its owner. Revoking it removes the name.
    let grant = Uuid::now_v7();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,1,0)")
        .bind(grant).bind(actor).bind(deployment).execute(&pool).await.unwrap();
    assert_eq!(
        read().await.unwrap(),
        BTreeMap::from([(deployment_container, vec!["Search Deployment".into()])])
    );
    sqlx::query("DELETE FROM resourceaccesses WHERE id=$1")
        .bind(grant)
        .execute(&pool)
        .await
        .unwrap();
    assert!(read().await.unwrap().is_empty());

    // Enabled team roles use the same write/execute-implies-read rules as catalogs.
    let team_actor = Uuid::now_v7();
    let team = Uuid::now_v7();
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'Team')")
        .bind(team_actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id,actorid,name) VALUES($1,$2,$1::text)")
        .bind(team)
        .bind(team_actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships(teamid,memberactorid) VALUES($1,$2)")
        .bind(team)
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$1::text,'Custom')")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(team_actor)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO permissions(id,permissionlevel,resourcetype,roleid,specificpermissions) VALUES($1,4,2,$2,0)").bind(Uuid::now_v7()).bind(role).execute(&pool).await.unwrap();
    assert_eq!(
        read().await.unwrap(),
        BTreeMap::from([(stack_container, vec!["Search Stack".into()])])
    );

    // A Swarm Stack is resolved through the task and service projection, not labels.
    sqlx::query("DELETE FROM swarmservices WHERE id=$1")
        .bind(service)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmserviceprojections SET stackid=$1 WHERE platformid=$2 AND dockerserviceid='service-1'").bind(stack).bind(platform).execute(&pool).await.unwrap();
    let names = read().await.unwrap();
    assert_eq!(names[&task_container], vec!["Search Stack"]);
    assert!(!names.contains_key(&wrong_node_container));
    sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1")
        .bind(team_actor)
        .execute(&pool)
        .await
        .unwrap();
    assert!(read().await.unwrap().is_empty());
    assert!(
        reader
            .container_owner_names(ActorId::new(actor), true, &[])
            .await
            .unwrap()
            .is_empty()
    );
}
