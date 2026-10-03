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
