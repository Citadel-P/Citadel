use super::*;
use axum::{Json, Router, response::IntoResponse};
use std::sync::Mutex;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn lifecycle_events_use_no_docker_reads_and_unknown_identity_stays_container_scoped() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let platform = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(platform).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,projectionobservedat) VALUES($1,$2,'fixture','image','original',1,100,'Created','[]',100)")
        .bind(uuid::Uuid::now_v7()).bind(platform).execute(&pool).await.unwrap();
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
    let app = Router::new().fallback({
        let calls = calls.clone();
        move |req: axum::extract::Request| {
            let calls = calls.clone();
            async move {
                calls.lock().unwrap().push(req.uri().to_string());
                match req.uri().path() {
                    "/version" => Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response(),
                    "/v1.49/containers/json" => {
                        assert!(req.uri().query().unwrap_or_default().contains("filters="));
                        Json(serde_json::json!([{"Id":"fixture","Names":["/metadata"],"ImageID":"image","State":"running","Created":1,"Labels":{},"Ports":[]}])).into_response()
                    }
                    _ => panic!("unexpected Docker request {}",req.uri()),
                }
            }
        }
    });
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(stop.cancelled_owned())
            .await
            .unwrap();
    });
    let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
    targets.refresh().await.unwrap();
    let event = |action: &str, id: &str, time: u128| InventoryEvent {
        resource: None,
        stream_recovered: false,
        swarm_scope: false,
        kind: citadel_adapters::connectors::docker::events::classify("container", action, "local")
            .unwrap(),
        platform_id: None,
        container_id: Some(id.into()),
        container_state: None,
        container_name: None,
        container: None,
        source: ReconciliationTrigger::LocalEvent,
        action: action.into(),
        event_time_millis: Some(time * 1000),
    };
    for (i, action, state) in [
        (101, "start", "Running"),
        (102, "die", "Exited"),
        (103, "pause", "Paused"),
        (104, "unpause", "Running"),
    ] {
        assert!(
            apply_container_event(
                &event(action, "fixture", i),
                &docker,
                &pool,
                None,
                &cancel,
                &targets
            )
            .await
            .unwrap()
        );
        let row: (String, String, i64) = sqlx::query_as(
            "SELECT state,name,projectionobservedat FROM containers WHERE platformid=$1",
        )
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row, (state.into(), "original".into(), i as i64));
    }
    assert!(
        calls.lock().unwrap().is_empty(),
        "known transitions must not even negotiate Docker"
    );
    assert!(
        apply_container_event(
            &event("die", "fixture", 101),
            &docker,
            &pool,
            None,
            &cancel,
            &targets
        )
        .await
        .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Running"
    );
    let unknown = event("start", "missing", 105);
    assert!(
        !apply_container_event(&unknown, &docker, &pool, None, &cancel, &targets)
            .await
            .unwrap()
    );
    assert_eq!(
        event_decision(unknown.kind, false, false, DeltaOutcome::Unavailable),
        ReconciliationDecision::Reconcile(citadel_platforms::jobs::ReconciliationScope::Containers)
    );
    assert!(calls.lock().unwrap().is_empty());
    // Destroy is an ID-only tombstone. Create and rename still hydrate metadata.
    assert!(
        apply_container_event(
            &event("destroy", "fixture", 105),
            &docker,
            &pool,
            None,
            &cancel,
            &targets
        )
        .await
        .unwrap()
    );
    assert!(calls.lock().unwrap().is_empty());
    for action in ["create", "rename"] {
        assert!(
            apply_container_event(
                &event(action, "fixture", 106),
                &docker,
                &pool,
                None,
                &cancel,
                &targets
            )
            .await
            .unwrap()
        );
    }
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.starts_with("/v1.49/containers/json"))
            .count(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT name FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "metadata"
    );
    // Direct Core accepts the new ID-only stream and ignores legacy metadata on
    // a state-only event, so neither form can overwrite known names/image IDs.
    sqlx::query("UPDATE platforms SET connectortype='Agent' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    targets.refresh().await.unwrap();
    let before = calls.lock().unwrap().len();
    let mut direct = event("die", "fixture", 107);
    direct.source = ReconciliationTrigger::AgentEvent;
    direct.platform_id = Some(platform);
    assert!(
        apply_container_event(&direct, &docker, &pool, None, &cancel, &targets)
            .await
            .unwrap()
    );
    direct.kind = RuntimeEventKind::Container(ContainerChange::Running);
    direct.action = "start".into();
    direct.event_time_millis = Some(108_000);
    direct.container = Some(citadel_platforms::RuntimeContainerSummary {
        id: "fixture".into(),
        name: "must-not-overwrite".into(),
        image_id: "wrong".into(),
        ..Default::default()
    });
    assert!(
        apply_container_event(&direct, &docker, &pool, None, &cancel, &targets)
            .await
            .unwrap()
    );
    let preserved: (String, String, String) =
        sqlx::query_as("SELECT name,dockerimageid,state FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        preserved,
        ("metadata".into(), "image".into(), "Running".into())
    );
    assert_eq!(calls.lock().unwrap().len(), before);
    cancel.cancel();
    server.await.unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

fn batch_iterations() -> u64 {
    let mut metrics = String::new();
    citadel_runtime::runtime_metrics::render_runtime_metrics(&mut metrics);
    metrics
        .lines()
        .find_map(|line| {
            line.strip_prefix(
                "citadel_runtime_iterations_total{family=\"ContainerStateDeltaBatch\"} ",
            )
        })
        .unwrap()
        .parse()
        .unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL; run with --test-threads=1"]
async fn local_and_direct_bursts_commit_once_and_publish_six_ids() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    for source in [
        ReconciliationTrigger::LocalEvent,
        ReconciliationTrigger::AgentEvent,
    ] {
        let platform = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,$2,1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')")
            .bind(platform).bind(if source == ReconciliationTrigger::LocalEvent {"Local"} else {"Agent"}).execute(&pool).await.unwrap();
        for i in 0..6 {
            sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,projectionobservedat) VALUES($1,$2,$3,'image','preserved',1,10,'Running','[]',10)")
                .bind(uuid::Uuid::now_v7()).bind(platform).bind(format!("{platform}-{i}")).execute(&pool).await.unwrap();
        }
        let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
        targets.refresh().await.unwrap();
        let passes = if source == ReconciliationTrigger::LocalEvent {
            targets
                .snapshot()
                .await
                .iter()
                .filter(|t| t.connector_type == citadel_platforms::ConnectorKind::Local)
                .count() as u64
        } else {
            1
        };
        // Closed endpoint: any accidental Docker enrichment would fail this test.
        let docker = DockerClient::with_endpoint(
            "http://127.0.0.1:1".parse().unwrap(),
            Duration::from_millis(100),
            "/host",
        )
        .unwrap();
        let metrics = Arc::new(Metrics::default());
        let hub = RealtimeHub::new(32, metrics.clone());
        let mut notifications = hub.subscribe();
        let cancellation = CancellationToken::new();
        let (sender, receiver) = bounded_channel(32, QueueOverflowPolicy::Wait);
        for i in 0..6 {
            sender
                .send(
                    InventoryEvent {
                        resource: None,
                        stream_recovered: false,
                        swarm_scope: false,
                        kind: RuntimeEventKind::Container(ContainerChange::Exited),
                        source,
                        platform_id: (source == ReconciliationTrigger::AgentEvent)
                            .then_some(platform),
                        container_id: Some(format!("{platform}-{i}")),
                        container_state: None,
                        container_name: None,
                        container: None,
                        action: "die".into(),
                        event_time_millis: Some(20_000),
                    },
                    &cancellation,
                )
                .await
                .unwrap();
        }
        drop(sender);
        let (event_refresh, _scopes) = event_refresh_channels(256);
        let (local, _local) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let (agent, _agent) = bounded_channel(1, QueueOverflowPolicy::Reject);
        let before = batch_iterations();
        tokio::time::timeout(
            Duration::from_secs(3),
            event_consumer(
                cancellation.clone(),
                receiver,
                EventWorker {
                    event_refresh,
                    metrics,
                    docker,
                    pool: pool.clone(),
                    realtime: Some(hub),
                    targets,
                    local_reconciliation: local,
                    agent_reconciliation: AgentReconciliationSignal {
                        queue: agent,
                        full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    },
                },
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(batch_iterations() - before, passes);
        let notification = notifications.recv().await.unwrap();
        assert_eq!(notification.platform_id, Some(platform));
        assert_eq!(notification.container_ids().unwrap().len(), 6);
        assert!(notifications.try_recv().is_err());
        let exited: i64 = sqlx::query_scalar("SELECT count(*) FROM containers WHERE platformid=$1 AND state='Exited' AND name='preserved'").bind(platform).fetch_one(&pool).await.unwrap();
        assert_eq!(exited, 6);
        sqlx::query("DELETE FROM containers WHERE platformid=$1")
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM platforms WHERE id=$1")
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
    }
}
