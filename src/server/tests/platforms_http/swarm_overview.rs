use super::*;

// Ports SwarmEndpointTests overview authorization/aggregation and the quorum
// cases: an online socket does not imply a healthy manager quorum.
#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn overview_reports_persisted_quorum_workloads_and_stale_inventory() {
    let f = fixture().await;
    let uri = format!("/api/v1/platforms/{}/swarm", f.platform_id);
    let reader = super::lookup::subject(&f).await;
    assert_eq!(
        send(&f, &uri, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &uri, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"DockerSwarm\",\"controlAvailable\":true}' WHERE id=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE swarmnodeprojections SET role='Manager',reachability='Reachable',isleader=true,isstale=false WHERE platformid=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    let read = || async {
        let response = send(&f, &uri, Some(reader.clone())).await;
        let status = response.status();
        let body = json_body(response).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    };
    let body = read().await;
    assert_eq!(body["health"], "Healthy");
    assert_eq!(body["nodeCount"], 1);
    assert_eq!(
        body["quorum"],
        json!({"state":"Healthy","reachableManagers":1,"requiredManagers":1,"hasLeader":true})
    );
    assert_eq!(body["capabilities"]["canRead"], true);
    assert_eq!(body["capabilities"]["canWrite"], false);
    for (running, desired, bucket) in [
        (2, 2, "healthy"),
        (1, 2, "degraded"),
        (0, 2, "failed"),
        (0, 0, "stopped"),
    ] {
        sqlx::query("UPDATE swarmserviceprojections SET runningtaskcount=$2,desiredtaskcount=$3 WHERE platformid=$1").bind(f.platform_id).bind(running).bind(desired).execute(&f.pool).await.unwrap();
        let body = read().await;
        assert_eq!(body["serviceCount"], 1);
        assert_eq!(body["serviceStatusCounts"][bucket], 1);
        assert_eq!(body["runningTaskCount"], running);
        assert_eq!(body["desiredTaskCount"], desired);
    }
    sqlx::query("UPDATE swarmserviceprojections SET ownership='System' WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(read().await["serviceCount"], 0);
    sqlx::query("UPDATE swarmnodeprojections SET isleader=false WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let lost = read().await;
    assert_eq!(lost["quorum"]["state"], "Lost");
    assert_eq!(lost["health"], "Degraded");
    sqlx::query("UPDATE swarmconfigprojections SET isstale=true WHERE platformid=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(read().await["health"], "Stale");
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let offline = read().await;
    assert_eq!(offline["health"], "Offline");
    assert_eq!(offline["quorum"]["state"], "Unknown");
    assert_eq!(offline["nodeCount"], 1);
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(f.platform_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        send(&f, &uri, Some(reader)).await.status(),
        StatusCode::BAD_REQUEST
    );
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn http_and_live_overviews_share_node_freshness_and_actor_specific_capabilities() {
    use citadel_server::realtime_groups::{Group, GroupReadPort};
    let f = fixture().await;
    let actor = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        actor.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let unauthorized = super::lookup::subject(&f).await;
    let reader = super::realtime_groups::reader(&f);
    let group = Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap();
    let hub = citadel_server::realtime::RealtimeHub::new(
        16,
        Arc::new(citadel_server::metrics::Metrics::default()),
    );
    let mut events = hub.subscribe();
    let volume = citadel_platforms::RuntimeVolumeSummary {
        name: "worker-data".into(),
        driver: "local".into(),
        ..Default::default()
    };
    sqlx::query("INSERT INTO swarmnodevolumeprojections(platformid,dockernodeid,volumename,observedat,resource) VALUES($1,'worker','worker-data',NOW(),$2)")
        .bind(f.platform_id).bind(serde_json::to_value(&volume).unwrap()).execute(&f.pool).await.unwrap();

    for (stale, online, control, error, leader, manager_stale, health, quorum) in [
        (true, true, true, None, true, false, "Stale", "Healthy"),
        (false, true, true, None, true, false, "Healthy", "Healthy"),
        (false, true, false, None, true, false, "Degraded", "Healthy"),
        (
            false,
            true,
            true,
            Some("manager observation failed"),
            true,
            false,
            "Degraded",
            "Healthy",
        ),
        (false, false, true, None, true, false, "Offline", "Unknown"),
        (false, true, true, None, false, false, "Degraded", "Lost"),
        (false, true, true, None, true, true, "Stale", "Unknown"),
        (false, true, true, None, true, false, "Healthy", "Healthy"),
    ] {
        sqlx::query("UPDATE platforms SET status=$2,platformdescriptor=$3 WHERE id=$1")
            .bind(f.platform_id)
            .bind(if online { "Online" } else { "Offline" })
            .bind(json!({"$type":"DockerSwarm","controlAvailable":control,"error":error}))
            .execute(&f.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE swarmnodeprojections SET role='Manager',reachability='Reachable',isleader=$2,isstale=$3 WHERE platformid=$1")
            .bind(f.platform_id).bind(leader).bind(manager_stale).execute(&f.pool).await.unwrap();
        sqlx::query("UPDATE swarmnodevolumeprojections SET isstale=$2 WHERE platformid=$1")
            .bind(f.platform_id)
            .bind(stale)
            .execute(&f.pool)
            .await
            .unwrap();
        let http = json_body(
            send(
                &f,
                &format!("/api/v1/platforms/{}/swarm", f.platform_id),
                Some(actor.clone()),
            )
            .await,
        )
        .await;
        assert_eq!(http["health"], health);
        assert_eq!(http["quorum"]["state"], quorum);
        assert_eq!(http["volumeCount"], 1);
        for node_local in [false, true] {
            if node_local {
                hub.publish_resource_observation(
                    f.platform_id,
                    Some("worker"),
                    "update",
                    &citadel_platforms::jobs::ResourceDelta::Volume {
                        id: "worker-data".into(),
                        value: Some(volume.clone()),
                    },
                );
            } else {
                hub.publish_inventory_observation(f.platform_id, "swarm");
            }
            let event = events.recv().await.unwrap();
            let limited = reader.read(&actor, &group, Some(&event)).await.unwrap();
            let target = if node_local {
                "SwarmNodeLocalResourcesUpdated"
            } else {
                "SwarmInventoryUpdated"
            };
            let live = &limited
                .events
                .iter()
                .find(|event| event.target == target)
                .unwrap()
                .arguments[0]["overview"];
            assert_eq!(live, &http, "HTTP/live mismatch for {target}");
            assert_eq!(live["capabilities"]["canWrite"], false);
            let admin = reader
                .read(&f.administrator, &group, Some(&event))
                .await
                .unwrap();
            let admin = &admin
                .events
                .iter()
                .find(|event| event.target == target)
                .unwrap()
                .arguments[0]["overview"];
            assert_eq!(admin["capabilities"]["canWrite"], true);
            assert_eq!(admin["health"], live["health"]);
            assert!(
                reader
                    .read(&unauthorized, &group, Some(&event))
                    .await
                    .is_err()
            );
        }
    }
    super::realtime_groups::cleanup(f).await;
}
