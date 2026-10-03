use super::*;
use axum::{Json, Router, body::Body, response::IntoResponse};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn stop_and_network_noise_never_read_metadata_even_when_docker_observation_would_fail() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    for fail_observation in [false, true] {
        let platform = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',3,9,1024,7,8,'{\"$type\":\"Docker\"}','Online')").bind(platform).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports) VALUES($1,$2,'fixture','image','web',1,1,'Running','[]')").bind(uuid::Uuid::now_v7()).bind(platform).execute(&pool).await.unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let lists = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let docker = DockerClient::with_endpoint(
            format!("http://{}", listener.local_addr().unwrap())
                .parse()
                .unwrap(),
            Duration::from_secs(2),
            "/host",
        )
        .unwrap();
        let router = Router::new().fallback({
            let calls = calls.clone(); let lists = lists.clone();
            move |request: axum::extract::Request| {
                let calls = calls.clone(); let lists = lists.clone();
                async move {
                    let path = request.uri().path().to_owned();
                    calls.lock().unwrap().push(path.clone());
                    match path.as_str() {
                        "/version" => Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(),
                        "/v1.49/events" => Body::from_stream(async_stream::stream! {
                            for (resource, action) in [("container","kill"),("container","die"),("container","stop"),("network","connect"),("network","disconnect")] {
                                yield Ok::<_, std::io::Error>(bytes::Bytes::from(format!("{}\n", serde_json::json!({"Type":resource,"Action":action,"scope":"local","Actor":{"ID":"fixture"}}))));
                            }
                            std::future::pending::<()>().await;
                        }).into_response(),
                        "/v1.49/containers/json" if request.uri().query().is_some_and(|q| q.contains("filters=")) => {
                            if fail_observation {
                                (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"message":"observation failed"}))).into_response()
                            } else {
                                Json(serde_json::json!([{"Id":"fixture","Names":["/web"],"Image":"alpine","ImageID":"image","State":"exited","Created":1,"Labels":{},"Ports":[]}])).into_response()
                            }
                        }
                        "/v1.49/containers/json" => {
                            if lists.fetch_add(1, Ordering::SeqCst) == 0 {
                                (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"message":"retry only containers"}))).into_response()
                            } else {
                                Json(serde_json::json!([{"Id":"fixture","Names":["/web"],"Image":"alpine","ImageID":"image","State":"exited","Created":1,"Labels":{},"Ports":[]}])).into_response()
                            }
                        }
                        _ => panic!("unrelated resource enumeration: {path}"),
                    }
                }
            }
        });
        let cancel = CancellationToken::new();
        let server_cancel = cancel.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(server_cancel.cancelled_owned())
                .await
                .unwrap();
        });
        let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
        targets.refresh().await.unwrap();
        let (events, receiver) = bounded_channel(1, QueueOverflowPolicy::Wait);
        let (scoped, scopes) = event_refresh_channels(1);
        let (local, mut local_recovery) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let (agent, mut agent_recovery) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let metrics = Arc::new(Metrics::default());
        let source = tokio::spawn(event_source(
            cancel.clone(),
            docker.clone(),
            events,
            metrics.clone(),
        ));
        let consumer = tokio::spawn(event_consumer(
            cancel.clone(),
            receiver,
            EventWorker {
                event_refresh: scoped.clone(),
                docker: docker.clone(),
                pool: pool.clone(),
                metrics,
                realtime: None,
                targets: targets.clone(),
                local_reconciliation: local,
                agent_reconciliation: AgentReconciliationSignal {
                    queue: agent,
                    full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                },
            },
        ));
        let refresh = tokio::spawn(event_resource_refresh(
            cancel.clone(),
            scopes,
            EventRefreshWorker {
                refreshes: scoped,
                targets,
                docker,
                pool: pool.clone(),
                realtime: None,
                node_policy: Default::default(),
                retry_delay: Duration::from_millis(250),
                swarm_interval: Duration::from_secs(3600),
            },
        ));
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let state: String = sqlx::query_scalar("SELECT state FROM containers WHERE platformid=$1 AND dockercontainerid='fixture'").bind(platform).fetch_one(&pool).await.unwrap();
                if state == "Exited" { break; }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }).await.unwrap();
        assert!(local_recovery.try_recv().is_none());
        assert!(agent_recovery.try_recv().is_none());
        assert_eq!(lists.load(Ordering::SeqCst), 0);
        let counts: (i32, i32, i32) =
            sqlx::query_as("SELECT imagecount,networkcount,volumecount FROM platforms WHERE id=$1")
                .bind(platform)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(counts, (9, 7, 8));
        assert!(
            calls
                .lock()
                .unwrap()
                .iter()
                .all(|path| matches!(path.as_str(), "/version" | "/v1.49/events"))
        );
        cancel.cancel();
        source.await.unwrap().unwrap();
        consumer.await.unwrap().unwrap();
        refresh.await.unwrap().unwrap();
        server.await.unwrap();
        sqlx::query("DELETE FROM platforms WHERE id=$1")
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
    }
    pool.close().await;
}

#[test]
fn daemon_action_cannot_forge_a_stream_recovery_signal() {
    let (local, mut local_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let (agent, mut agent_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let agent = AgentReconciliationSignal {
        queue: agent,
        full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    let mut event = InventoryEvent {
        resource: None,
        stream_recovered: false,
        swarm_scope: false,
        kind: RuntimeEventKind::Unknown,
        platform_id: None,
        container_id: None,
        container_state: None,
        container_name: None,
        container: None,
        source: ReconciliationTrigger::LocalEvent,
        action: "reconnect".into(),
        event_time_millis: None,
    };
    queue_stream_recovery(&event, &local, &agent);
    assert!(local_receiver.try_recv().is_none());
    assert!(agent_receiver.try_recv().is_none());
    event.stream_recovered = true;
    queue_stream_recovery(&event, &local, &agent);
    assert_eq!(local_receiver.try_recv(), Some(()));
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn hundred_swarm_events_collect_one_projection_and_no_standalone_inventory() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let platform = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status,clusterid) VALUES($1,$1::text,$1::text,'Local',3,9,1024,7,8,'{\"$type\":\"DockerSwarm\",\"nodeID\":\"manager\"}','Online','cluster')").bind(platform).execute(&pool).await.unwrap();
    let calls = Arc::new(Mutex::new(Vec::<String>::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let docker = DockerClient::with_endpoint(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
        Duration::from_secs(2),
        "/host",
    )
    .unwrap();
    let router = Router::new().fallback({
        let calls = calls.clone();
        move |request: axum::extract::Request| {
            let calls = calls.clone();
            async move {
                let path = request.uri().path().to_owned();
                calls.lock().unwrap().push(path.clone());
                match path.as_str() {
                    "/version" => Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(),
                    "/v1.49/events" => Body::from_stream(async_stream::stream! {
                        for i in 0..100 {
                            let resource = ["service", "task", "node", "network", "secret", "config"][i % 6];
                            yield Ok::<_, std::io::Error>(bytes::Bytes::from(format!("{}\n", serde_json::json!({"Type": resource, "Action":"update", "scope":"swarm", "Actor":{"ID":"fixture"}}))));
                        }
                        std::future::pending::<()>().await;
                    }).into_response(),
                    "/v1.49/nodes" => Json(serde_json::json!([{"ID":"manager","Spec":{"Role":"manager","Availability":"active"},"Description":{"Hostname":"manager"},"Status":{"State":"ready"}}])).into_response(),
                    "/v1.49/services" | "/v1.49/tasks" | "/v1.49/networks" | "/v1.49/configs" | "/v1.49/secrets" => Json(serde_json::json!([])).into_response(),
                    _ => panic!("Swarm event requested unrelated resource: {path}"),
                }
            }
        }
    });
    let cancel = CancellationToken::new();
    let server_cancel = cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(server_cancel.cancelled_owned())
            .await
            .unwrap();
    });
    let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
    targets.refresh().await.unwrap();
    let (events, receiver) = bounded_channel(128, QueueOverflowPolicy::Wait);
    let (sender, scopes) = event_refresh_channels(1);
    let (local, mut local_recovery) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let (agent, mut agent_recovery) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let metrics = Arc::new(Metrics::default());
    let source = tokio::spawn(event_source(
        cancel.clone(),
        docker.clone(),
        events,
        metrics.clone(),
    ));
    let consumer = tokio::spawn(event_consumer(
        cancel.clone(),
        receiver,
        EventWorker {
            event_refresh: sender.clone(),
            metrics,
            docker: docker.clone(),
            pool: pool.clone(),
            realtime: None,
            targets: targets.clone(),
            local_reconciliation: local,
            agent_reconciliation: AgentReconciliationSignal {
                queue: agent,
                full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            },
        },
    ));
    let refresh = tokio::spawn(event_resource_refresh(
        cancel.clone(),
        scopes,
        EventRefreshWorker {
            targets,
            refreshes: sender,
            docker,
            pool: pool.clone(),
            realtime: None,
            node_policy: Default::default(),
            retry_delay: Duration::from_secs(5),
            swarm_interval: Duration::from_secs(3600),
        },
    ));
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let fresh: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmnodeprojections WHERE platformid=$1 AND NOT isstale)").bind(platform).fetch_one(&pool).await.unwrap();
            if fresh { break; }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.unwrap();
    tokio::time::sleep(super::swarm_reconciliation::DEBOUNCE + Duration::from_millis(200)).await;
    for resource in [
        "nodes", "services", "tasks", "networks", "configs", "secrets",
    ] {
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|path| **path == format!("/v1.49/{resource}"))
                .count(),
            1,
            "one {resource} enumeration for the entire burst"
        );
    }
    assert_eq!(
        calls.lock().unwrap().len(),
        8,
        "version + stream + six Swarm reads only"
    );
    assert!(local_recovery.try_recv().is_none());
    assert!(agent_recovery.try_recv().is_none());
    let counts: (i32, i32, i32) =
        sqlx::query_as("SELECT imagecount,networkcount,volumecount FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(counts, (9, 7, 8));
    cancel.cancel();
    source.await.unwrap().unwrap();
    consumer.await.unwrap().unwrap();
    refresh.await.unwrap().unwrap();
    server.await.unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
