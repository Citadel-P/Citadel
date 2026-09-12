use super::*;

// Ports the requested-window and batched-history behavior from .NET
// PlatformsStatsWriterJobTests / ContainerStatsWriterJobTests through HTTP.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn history_endpoints_authorize_validate_windows_and_read_persisted_samples() {
    let f = fixture().await;
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let now = chrono::Utc::now().timestamp();
    for hours in [1_i64, 25, 47, 71, 73] {
        sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,$3,10,20,2,100,3,4)")
            .bind(Uuid::now_v7()).bind(container).bind(now-hours*3600).execute(&f.pool).await.unwrap();
        sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes) VALUES($1,$2,$3,10,20,3,4)")
            .bind(Uuid::now_v7()).bind(f.platform_id).bind(now-hours*3600).execute(&f.pool).await.unwrap();
    }
    let denied = super::lookup::subject(&f).await;
    for path in [
        format!("/api/v1/containers/{container}/stats"),
        format!("/api/v1/platforms/{}/stats", f.platform_id),
    ] {
        assert_eq!(
            send(&f, &path, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&f, &path, Some(denied.clone())).await.status(),
            StatusCode::FORBIDDEN
        );
        for invalid in ["0", "1", "25", "96", "-1", "abc"] {
            assert_eq!(
                send(
                    &f,
                    &format!("{path}?hours={invalid}"),
                    Some(f.administrator.clone())
                )
                .await
                .status(),
                StatusCode::BAD_REQUEST
            );
        }
        for (hours, count) in [(24, 1), (48, 3), (72, 4)] {
            let response = send(
                &f,
                &format!("{path}?hours={hours}"),
                Some(f.administrator.clone()),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = json_body(response).await;
            let stats = body["stats"].as_array().unwrap();
            assert_eq!(stats.len(), count);
            assert!(
                stats
                    .windows(2)
                    .all(|p| p[0]["created"].as_i64() < p[1]["created"].as_i64())
            );
            assert_eq!(stats[0]["cpuUsage"], 10.0);
        }
    }
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn service_history_survives_task_replacement_without_double_counting_or_crossing_nodes() {
    use citadel_adapters::{
        container_stats_store::PostgresContainerStatsStore,
        edge::{EdgeRegistry, EdgeTarget, PostgresEdgeStore},
        statistics_read_store::PostgresStatisticsReadStore,
    };
    use citadel_platforms::{
        ContainerStatsStore, RuntimeContainerStat, ServiceStatIdentity, StatisticsReadStore,
        StatsWindow,
    };
    let f = fixture().await;
    let manager_writer = PostgresContainerStatsStore::new(f.pool.clone());
    let writer = PostgresEdgeStore::new(f.pool.clone());
    let registry = EdgeRegistry::default();
    let (session, _receiver) = registry
        .register(
            EdgeTarget::node(f.platform_id, "node-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    // Enrollment/identity is covered by edge_transport. Seed an authenticated
    // Node session here to exercise the production Node statistics writer.
    sqlx::query("INSERT INTO edgeagentbindings(id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid,profile,dockernodeid,lastconnectedatutc) VALUES($1,$2,$1,'fixture','Connected',$3,$3,'SwarmNode','node-1',$4)")
        .bind(session.agent_id).bind(session.agent_id.to_string()).bind(f.platform_id).bind(session.connected_at).execute(&f.pool).await.unwrap();
    let reader = PostgresStatisticsReadStore::new(f.pool.clone());
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let bucket = Utc::now().timestamp() / 60 * 60 - 120;
    sqlx::query("UPDATE containers SET isswarmtask=true,dockernodeid='node-1' WHERE id=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmtaskprojections SET dockercontainerid='container-1',slot=1 WHERE platformid=$1 AND dockertaskid='task-1'").bind(f.platform_id).execute(&f.pool).await.unwrap();
    let mut sample = RuntimeContainerStat {
        docker_container_id: "container-1".into(),
        cpu_usage: 10.0,
        memory_active: 20.0,
        memory_cache: 2.0,
        memory_limit: 100.0,
        rx_bytes: 3.0,
        tx_bytes: 4.0,
        created: bucket + 1,
    };
    assert_eq!(
        manager_writer
            .persist(f.platform_id, std::slice::from_ref(&sample))
            .await
            .unwrap(),
        0,
        "manager samples cannot populate worker history"
    );
    assert_eq!(
        writer
            .persist_stats(&session, std::slice::from_ref(&sample))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        writer
            .persist_stats(&session, std::slice::from_ref(&sample))
            .await
            .unwrap(),
        1
    );
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM swarmservicestats WHERE platformid=$1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(count, 1, "retry must not duplicate a sample");
    // A replacement in the same logical slot contributes to that slot's average.
    sqlx::query("UPDATE swarmtaskprojections SET dockertaskid='replacement' WHERE platformid=$1 AND dockertaskid='task-1'").bind(f.platform_id).execute(&f.pool).await.unwrap();
    sample.created = bucket + 20;
    sample.cpu_usage = 30.0;
    writer
        .persist_stats(&session, std::slice::from_ref(&sample))
        .await
        .unwrap();
    // A delayed sample still belongs to the managed task after its container
    // projection has been deleted (ContainerStatsWriterJobTests parity).
    sqlx::query("DELETE FROM containers WHERE id=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    sample.created = bucket + 25;
    sample.cpu_usage = 20.0;
    sample.memory_active = 20.0;
    assert_eq!(
        writer
            .persist_stats(&session, std::slice::from_ref(&sample))
            .await
            .unwrap(),
        0,
        "no container row is resurrected for a delayed service sample"
    );
    let retained: (f64, f64) = sqlx::query_as(
        "SELECT cpuusage,memoryactive FROM swarmservicestats WHERE platformid=$1 AND created=$2",
    )
    .bind(f.platform_id)
    .bind(sample.created)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(retained, (20.0, 20.0));
    // Same Docker ID on a different node must not be attributed to this Task.
    sqlx::query("UPDATE swarmtaskprojections SET dockernodeid='other-node' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    sample.created = bucket + 30;
    sample.cpu_usage = 1000.0;
    writer.persist_stats(&session, &[sample]).await.unwrap();
    // Deleting runtime projections does not remove Service history.
    sqlx::query("DELETE FROM containers WHERE id=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM swarmtaskprojections WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let stats = reader
        .service(
            ServiceStatIdentity {
                platform_id: f.platform_id,
                docker_service_id: "service-1",
                managed_service_id: None,
                stack_id: None,
                service_name: "web",
            },
            StatsWindow::new(24).unwrap(),
            Utc::now().timestamp(),
        )
        .await
        .unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].cpu_usage, 20.0);
    assert_eq!(stats[0].memory_active, 20.0);
    let other = reader
        .service(
            ServiceStatIdentity {
                platform_id: f.platform_id,
                docker_service_id: "another-service",
                managed_service_id: None,
                stack_id: None,
                service_name: "web",
            },
            StatsWindow::new(24).unwrap(),
            Utc::now().timestamp(),
        )
        .await
        .unwrap();
    assert!(other.is_empty());
    let managed = Uuid::now_v7();
    sqlx::query("UPDATE swarmservicestats SET swarmserviceid=$2 WHERE platformid=$1")
        .bind(f.platform_id)
        .bind(managed)
        .execute(&f.pool)
        .await
        .unwrap();
    let stats = reader
        .service(
            ServiceStatIdentity {
                platform_id: f.platform_id,
                docker_service_id: "recreated-service",
                managed_service_id: Some(managed),
                stack_id: None,
                service_name: "renamed",
            },
            StatsWindow::new(24).unwrap(),
            Utc::now().timestamp(),
        )
        .await
        .unwrap();
    assert_eq!(
        stats[0].cpu_usage, 20.0,
        "managed identity survives Service replacement and rename"
    );
    let stack = Uuid::now_v7();
    sqlx::query("UPDATE swarmservicestats SET swarmserviceid=NULL,stackid=$2 WHERE platformid=$1")
        .bind(f.platform_id)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    let stats = reader
        .service(
            ServiceStatIdentity {
                platform_id: f.platform_id,
                docker_service_id: "recreated-service",
                managed_service_id: None,
                stack_id: Some(stack),
                service_name: "web",
            },
            StatsWindow::new(24).unwrap(),
            Utc::now().timestamp(),
        )
        .await
        .unwrap();
    assert_eq!(
        stats[0].cpu_usage, 20.0,
        "Stack member identity survives Service replacement"
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn service_stats_reports_missing_stale_and_fresh_node_coverage() {
    let f = fixture().await;
    let path = format!(
        "/api/v1/platforms/{}/swarm/services/service-1/stats",
        f.platform_id
    );
    let denied = super::lookup::subject(&f).await;
    assert_eq!(
        send(&f, &path, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &path, Some(denied)).await.status(),
        StatusCode::FORBIDDEN
    );
    let response = send(&f, &path, Some(f.administrator.clone())).await;
    let status = response.status();
    let body = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["complete"], false);
    assert_eq!(body["observedTasks"], 0);
    assert_eq!(body["expectedTasks"], 1);
    assert_eq!(body["missingDockerNodeIds"], json!(["node-1"]));
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    sqlx::query("UPDATE containers SET isswarmtask=true,dockernodeid='node-1' WHERE id=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE swarmtaskprojections SET dockercontainerid='container-1',slot=1 WHERE platformid=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    // Projection IDs are returned before a sample arrives, so the unchanged UI can subscribe.
    let body = json_body(send(&f, &path, Some(f.administrator.clone())).await).await;
    assert_eq!(body["observedContainerProjectionIds"], json!([container]));
    assert_eq!(body["complete"], false);
    sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,$3,10,20,2,100,3,4)").bind(Uuid::now_v7()).bind(container).bind(Utc::now().timestamp()).execute(&f.pool).await.unwrap();
    let body = json_body(send(&f, &path, Some(f.administrator.clone())).await).await;
    assert_eq!(body["complete"], true);
    assert_eq!(body["observedTasks"], 1);
    assert_eq!(body["missingDockerNodeIds"], json!([]));
    assert!(body["newestSampleAt"].is_string());
    sqlx::query("UPDATE containerstats SET created=created-60 WHERE containerid=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    let body = json_body(send(&f, &path, Some(f.administrator.clone())).await).await;
    assert_eq!(body["complete"], false);
    assert_eq!(body["observedTasks"], 0);
    assert_eq!(body["missingDockerNodeIds"], json!(["node-1"]));
    assert_eq!(
        send(
            &f,
            &format!("{path}?hours=1"),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &f,
            &path.replace("service-1", "missing"),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn workload_history_requires_its_own_permission_and_preserves_stack_grouping() {
    use citadel_domain::ResourceType;
    let f = fixture().await;
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,'{}','Created',$4)").bind(deployment).bind(format!("stats-{deployment}")).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$2,$3,'WebEditor','{}','{}')").bind(stack).bind(format!("stats-{stack}")).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let reader = super::lookup::subject(&f).await;
    for (kind, resource, id) in [
        (ResourceType::Deployment, "deployments", deployment),
        (ResourceType::Stack, "stacks", stack),
    ] {
        let path = format!("/api/v1/{resource}/{id}/stats");
        assert_eq!(
            send(&f, &path, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&f, &path, Some(reader.clone())).await.status(),
            StatusCode::FORBIDDEN
        );
        super::lookup::grant(&f, reader.actor_id.value(), kind, id, 0).await;
        let response = send(&f, &path, Some(reader.clone())).await;
        if resource == "deployments" {
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        } else {
            assert_eq!(json_body(response).await["containers"], json!([]));
        }
    }
    sqlx::query("UPDATE containers SET deploymentid=$2 WHERE id=$1")
        .bind(container)
        .bind(deployment)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,$3,10,20,2,100,3,4)").bind(Uuid::now_v7()).bind(container).bind(Utc::now().timestamp()-60).execute(&f.pool).await.unwrap();
    let path = format!("/api/v1/deployments/{deployment}/stats");
    let body = json_body(send(&f, &path, Some(reader.clone())).await).await;
    assert_eq!(body["stats"][0]["containerId"], container.to_string());
    sqlx::query("UPDATE containers SET deploymentid=NULL,stackid=$2 WHERE id=$1")
        .bind(container)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    let path = format!("/api/v1/stacks/{stack}/stats");
    let body = json_body(send(&f, &path, Some(reader.clone())).await).await;
    assert_eq!(body["containers"][0]["containerId"], "container-1");
    assert_eq!(body["containers"][0]["containerName"], "web");
    assert_eq!(body["containers"][0]["stats"][0]["cpuUsage"], 10.0);
    assert_eq!(
        send(
            &f,
            &format!("/api/v1/containers/{container}/stats"),
            Some(reader)
        )
        .await
        .status(),
        StatusCode::FORBIDDEN,
        "workload Read must not grant direct Platform history access"
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn task_history_checks_live_docker_identity_and_node_before_returning_samples() {
    let f = fixture().await;
    let path = format!(
        "/api/v1/platforms/{}/swarm/tasks/task-1/stats",
        f.platform_id
    );
    let denied = super::lookup::subject(&f).await;
    assert_eq!(
        send(&f, &path, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &path, Some(denied)).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &f,
            &format!("{path}?hours=13"),
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(&f, &path, Some(f.administrator.clone()))
            .await
            .status(),
        StatusCode::CONFLICT,
        "projection has not synchronized"
    );
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    sqlx::query("UPDATE containers SET isswarmtask=true,dockernodeid='node-1' WHERE id=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,$3,10,20,2,100,3,4)").bind(Uuid::now_v7()).bind(container).bind(Utc::now().timestamp()-60).execute(&f.pool).await.unwrap();
    let response = send(&f, &path, Some(f.administrator.clone())).await;
    let status = response.status();
    let body = json_body(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["containerProjectionId"], container.to_string());
    assert_eq!(body["dockerContainerId"], "container-1");
    assert_eq!(body["stats"][0]["cpuUsage"], 10.0);
    sqlx::query("UPDATE swarmtaskprojections SET dockernodeid='moved-node' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &path, Some(f.administrator.clone()))
            .await
            .status(),
        StatusCode::CONFLICT,
        "live Task identity disagrees with cached node"
    );
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &path, Some(f.administrator.clone()))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn history_buckets_samples_and_resolves_legacy_docker_ids_without_ambiguity() {
    let f = fixture().await;
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let docker_id = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    sqlx::query("UPDATE containers SET dockercontainerid=$2 WHERE id=$1")
        .bind(container)
        .bind(&docker_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let bucket = Utc::now().timestamp() / 60 * 60 - 120;
    for (offset, cpu) in [(1, 10.0), (10, 30.0)] {
        sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) VALUES($1,$2,$3,$4,0,2,100,3,4)").bind(Uuid::now_v7()).bind(container).bind(bucket+offset).bind(cpu).execute(&f.pool).await.unwrap();
    }
    let path = format!("/api/v1/containers/{}/stats", &docker_id[..40]);
    let response = send(&f, &path, Some(f.administrator.clone())).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["stats"].as_array().unwrap().len(), 1);
    assert_eq!(body["stats"][0]["cpuUsage"], 20.0);
    assert_eq!(body["stats"][0]["memoryActive"], 0.0);
    assert_eq!(body["stats"][0]["created"], bucket + 1);
    assert_eq!(body["stats"][0]["containerId"], container.to_string());
    // A second projection with the same Docker prefix must not choose an arbitrary owner.
    let mut both = snapshot(f.platform_id);
    both.containers[0].id = docker_id.clone();
    let mut second = both.containers[0].clone();
    second.id.push('b');
    second.name = "another".into();
    both.containers.push(second);
    PostgresInventoryProjectionStore::new(f.pool.clone())
        .persist(&both)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &path, Some(f.administrator.clone()))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(
            &f,
            "/api/v1/containers/invalid/stats",
            Some(f.administrator.clone())
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn statistics_retention_is_batched_and_failed_writes_roll_back_all_sample_tables() {
    use citadel_adapters::container_stats_store::PostgresContainerStatsStore;
    use citadel_platforms::{ContainerStatsStore, RuntimeContainerStat};
    let f = fixture().await;
    let writer = PostgresContainerStatsStore::new(f.pool.clone());
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let now = Utc::now().timestamp();
    let sample = RuntimeContainerStat {
        docker_container_id: "container-1".into(),
        cpu_usage: 1.0,
        memory_active: 1.0,
        memory_cache: 0.0,
        memory_limit: 1.0,
        rx_bytes: 0.0,
        tx_bytes: 0.0,
        created: now,
    };
    // Duplicate conflict keys abort the write; no partial Container/Platform batch is committed.
    assert!(
        writer
            .persist(f.platform_id, &[sample.clone(), sample.clone()])
            .await
            .is_err()
    );
    let counts:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM containerstats WHERE containerid=$1),(SELECT count(*) FROM platformstats WHERE platformid=$2),(SELECT count(*) FROM swarmservicestats WHERE platformid=$2)").bind(container).bind(f.platform_id).fetch_one(&f.pool).await.unwrap();
    assert_eq!(counts, (0, 0, 0));
    sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) SELECT gen_random_uuid(),$1,$2::bigint-series.value,1,1,0,1,0,0 FROM generate_series(1,5001) AS series(value)").bind(container).bind(now-8*24*3600).execute(&f.pool).await.unwrap();
    writer
        .persist(f.platform_id, std::slice::from_ref(&sample))
        .await
        .unwrap();
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM containerstats WHERE containerid=$1 AND created<$2",
    )
    .bind(container)
    .bind(now - 7 * 24 * 3600)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(
        remaining, 1,
        "one transaction prunes at most 5000 expired rows"
    );
    writer.persist(f.platform_id, &[sample]).await.unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM containerstats WHERE containerid=$1")
            .bind(container)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(
        remaining, 1,
        "subsequent write prunes the remainder but keeps the fresh sample"
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn connected_manager_history_uses_reported_node_identity_without_replacing_container_ids() {
    use citadel_adapters::container_stats_store::PostgresContainerStatsStore;
    use citadel_platforms::{ContainerStatsStore, RuntimeContainerStat, RuntimeSwarmInfo};
    let f = fixture().await;
    // This case models the connected manager, matching the fake daemon.
    sqlx::query("UPDATE platforms SET platformdescriptor=jsonb_set(platformdescriptor::jsonb, '{nodeID}', '\"node-1\"') WHERE id=$1")
        .bind(f.platform_id).execute(&f.pool).await.unwrap();
    let container: Uuid =
        sqlx::query_scalar("SELECT id FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    let mut observed = snapshot(f.platform_id);
    observed.info.swarm = Some(RuntimeSwarmInfo {
        node_id: "node-1".into(),
        control_available: true,
        local_node_state: "active".into(),
        cluster_id: Some(format!("cluster-{}", f.platform_id)),
        ..Default::default()
    });
    observed.containers[0].is_swarm_task = true;
    observed.swarm.as_mut().unwrap().tasks[0].node_id = "node-1".into();
    observed.swarm.as_mut().unwrap().tasks[0].container_id = Some("container-1".into());
    observed.swarm.as_mut().unwrap().tasks[0].slot = Some(1);
    let inventory = PostgresInventoryProjectionStore::new(f.pool.clone());
    inventory.persist(&observed).await.unwrap();
    let writer = PostgresContainerStatsStore::new(f.pool.clone());
    let sample = RuntimeContainerStat {
        docker_container_id: "container-1".into(),
        cpu_usage: 10.0,
        memory_active: 20.0,
        memory_cache: 0.0,
        memory_limit: 100.0,
        rx_bytes: 1.0,
        tx_bytes: 2.0,
        created: Utc::now().timestamp(),
    };
    writer.persist(f.platform_id, &[sample]).await.unwrap();
    let service_path = format!(
        "/api/v1/platforms/{}/swarm/services/service-1/stats",
        f.platform_id
    );
    let body = json_body(send(&f, &service_path, Some(f.administrator.clone())).await).await;
    assert_eq!(body["complete"], true);
    assert_eq!(body["observedContainerProjectionIds"], json!([container]));
    assert_eq!(body["stats"][0]["cpuUsage"], 10.0);
    let task_path = format!(
        "/api/v1/platforms/{}/swarm/tasks/task-1/stats",
        f.platform_id
    );
    let body = json_body(send(&f, &task_path, Some(f.administrator.clone())).await).await;
    assert_eq!(body["containerProjectionId"], container.to_string());
    // A changed or absent manager identity must be rejected before overwriting
    // the accepted projection or attributing samples to another daemon.
    observed.info.swarm.as_mut().unwrap().node_id = "different-manager".into();
    assert!(inventory.persist(&observed).await.is_err());
    observed.info.swarm = None;
    assert!(inventory.persist(&observed).await.is_err());
    let node: Option<String> = sqlx::query_scalar(
        "SELECT platformdescriptor::jsonb->>'nodeID' FROM platforms WHERE id=$1",
    )
    .bind(f.platform_id)
    .fetch_one(&f.pool)
    .await
    .unwrap();
    assert_eq!(node.as_deref(), Some("node-1"));
    let body = json_body(send(&f, &service_path, Some(f.administrator.clone())).await).await;
    assert_eq!(body["complete"], true);
    assert_eq!(body["observedContainerProjectionIds"], json!([container]));
    // Coexisting legacy and explicit node projections must not count one Task twice.
    observed.info.swarm = Some(RuntimeSwarmInfo {
        node_id: "node-1".into(),
        control_available: true,
        local_node_state: "active".into(),
        cluster_id: Some(format!("cluster-{}", f.platform_id)),
        ..Default::default()
    });
    sqlx::query("UPDATE containers SET dockernodeid='node-1' WHERE id=$1")
        .bind(container)
        .execute(&f.pool)
        .await
        .unwrap();
    observed.swarm.as_mut().unwrap().services[0].desired_task_count = 2;
    inventory.persist(&observed).await.unwrap();
    sqlx::query("INSERT INTO containerstats(id,containerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes) SELECT gen_random_uuid(),id,$2,1,1,0,1,0,0 FROM containers WHERE platformid=$1 AND dockernodeid IS NULL").bind(f.platform_id).bind(Utc::now().timestamp()).execute(&f.pool).await.unwrap();
    let body = json_body(send(&f, &service_path, Some(f.administrator.clone())).await).await;
    assert_eq!(body["complete"], false);
    assert_eq!(body["observedTasks"], 0);
    assert_eq!(
        send(&f, &task_path, Some(f.administrator.clone()))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}
