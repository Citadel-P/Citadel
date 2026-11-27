use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn saved_board_subscription_follows_committed_edits_and_private_visibility() {
    let f = fixture().await;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO dashboards(id,name,visibility,configuration) VALUES ($1,'Shared board','InstanceShared','{}')").bind(id).execute(&f.pool).await.unwrap();
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let service = RealtimeService::new(
        &RealtimeConfig {
            queue_capacity: 32,
            max_connections: 2,
            subscribe_timeout: StdDuration::from_secs(5),
            send_timeout: StdDuration::from_secs(2),
            authorization_recheck_interval: StdDuration::from_secs(30),
            snapshot_limit: 100,
        },
        Arc::new(IdentityRealtimeReader::new(
            f.lookup_state.platforms.identity.clone(),
            f.lookup_state.platforms.platforms.clone(),
        )),
        Arc::new(Metrics::default()),
        cancel.clone(),
    )
    .with_groups(Arc::new(reader(&f)));
    let hub = service.hub();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, service.router())
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{address}/api/v1/realtime"))
            .await
            .unwrap();
    let access_token = token(&f, &f.administrator).await;
    socket.send(Message::Text(json!({"protocolVersion":1,"kind":"subscribe","clientMode":"groups","accessToken":access_token}).to_string().into())).await.unwrap();
    assert_eq!(receive(&mut socket).await["kind"], "subscribed");
    let sub = Uuid::now_v7();
    let join = json!({"subscriptionId":sub,"group":"dashboard","projection":"getDashboard","tags":[],"query":{"id":id}});
    assert!(invoke_args(&mut socket, "JoinGroup", json!([join])).await["error"].is_null());
    let first = receive(&mut socket).await;
    assert_eq!(first["subscriptionId"], sub.to_string());
    assert_eq!(first["arguments"][0]["summary"]["name"], "Shared board");
    let mut changes = sqlx::postgres::PgListener::connect_with(&f.pool)
        .await
        .unwrap();
    changes.listen("citadel_dashboard_changes").await.unwrap();
    let mut tx = f.pool.begin().await.unwrap();
    sqlx::query("UPDATE dashboards SET name='Rolled back' WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert!(
        tokio::time::timeout(StdDuration::from_millis(100), changes.recv())
            .await
            .is_err()
    );
    sqlx::query("UPDATE dashboards SET configuration='{\"description\":\"Night shift\"}', revision=revision+1 WHERE id=$1").bind(id).execute(&f.pool).await.unwrap();
    let note = tokio::time::timeout(StdDuration::from_secs(3), changes.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(note.payload(), id.to_string());
    drop(changes);
    hub.publish_resource_change("Dashboard", id, "resourceChanged");
    assert_eq!(
        receive(&mut socket).await["arguments"][0]["configuration"]["description"],
        "Night shift"
    );
    let event = Uuid::now_v7();
    sqlx::query("INSERT INTO activityevents(id,resourceid,resourcename,resourcetype,eventtype,info,createdbyactorid,status) VALUES($1,$2,'Shared board','Dashboard','DashboardUpdated',$3,$4,'Success')")
        .bind(event).bind(id).bind(r#"{"$type":"DashboardUpdated","Changes":["Description"]}"#)
        .bind(f.administrator.actor_id.value()).execute(&f.pool).await.unwrap();
    let recent = json!({"subscriptionId":Uuid::now_v7(),"group":"dashboard","projection":"listRecentActivities","tags":[],"query":{"eventType":"DashboardUpdated","limit":5}});
    assert!(invoke_args(&mut socket, "JoinGroup", json!([recent])).await["error"].is_null());
    assert_eq!(
        receive(&mut socket).await["arguments"][0][0]["id"],
        event.to_string()
    );
    // A private dashboard and its activity are owner-only even for this administrator.
    sqlx::query("UPDATE dashboards SET visibility='Private', owneractorid=NULL WHERE id=$1")
        .bind(id)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_resource_change("Dashboard", id, "resourceChanged");
    let lost = receive(&mut socket).await;
    assert!(lost["arguments"][0].is_null());
    hub.publish_resource_change("Activity", Uuid::nil(), "resourceChanged");
    assert_eq!(receive(&mut socket).await["arguments"][0], json!([]));
    socket.close(None).await.unwrap();
    cancel.cancel();
    server.await.unwrap();
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn pinned_workloads_only_reread_on_their_platform_and_follow_moves() {
    use super::scoped_fixture::*;
    let f = fixture().await;
    let workload = Uuid::now_v7();
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,platformdescriptor,status,volumecount) VALUES($1,'Other host','','Local',1,0,1000,0,'{}','Online',0)").bind(other).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'Pinned',$2,'{}','Healthy',$3)").bind(workload).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let board: Uuid = sqlx::query_scalar("SELECT dashboardid FROM instancedashboarddefault")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let (hub, reads, address, cancel, server) = serve(&f).await;
    let mut socket = connect(&f, &f.administrator, address).await;
    let sub = Uuid::now_v7();
    for j in [
        json!({"group":"dashboard","subscriptionId":sub,"projection":"listDashboardSources","query":{"kind":"Deployment","ids":workload,"limit":1}}),
        json!({"group":"dashboard","subscriptionId":Uuid::now_v7(),"projection":"getDashboard","query":{"id":board}}),
    ] {
        assert!(invoke_args(&mut socket, "JoinGroup", json!([j])).await["error"].is_null());
        receive(&mut socket).await;
    }
    reads.take();
    for (n, platform, should_read) in [(1, other, false), (2, f.platform_id, true)] {
        hub.publish_runtime_change(platform, "container", "create", "example");
        // Another projection scheduled later provides a barrier even for suppressed snapshots.
        sqlx::query("UPDATE dashboards SET revision=revision+$2 WHERE id=$1")
            .bind(board)
            .bind(i64::from(n))
            .execute(&f.pool)
            .await
            .unwrap();
        hub.publish_resource_change("Dashboard", board, "resourceChanged");
        receive(&mut socket).await;
        assert_eq!(reads.take().contains(&sub.to_string()), should_read);
    }
    sqlx::query("UPDATE deployments SET platformid=$2 WHERE id=$1")
        .bind(workload)
        .bind(other)
        .execute(&f.pool)
        .await
        .unwrap();
    hub.publish_resource_change("Deployment", workload, "updated");
    assert_eq!(
        receive(&mut socket).await["arguments"][0][0]["platformId"],
        json!(other)
    );
    socket.close(None).await.unwrap();
    cancel.cancel();
    server.await.unwrap();
    cleanup(f).await;
}
