#![cfg(unix)]
//! Production Agent regression, using an explicitly supplied isolated daemon.
use citadel_adapters::edge::{
    EdgeIntake, EdgeRegistry, EdgeRuntime, EdgeTarget, PostgresEdgeStore,
};
use citadel_contracts::citadel::edge::v1::{
    EdgeCommandKind, edge_agent_service_server::EdgeAgentServiceServer,
};
use citadel_execution::{ProcessLimits, ProcessRequest, run};
use citadel_platforms::PlatformRuntimePort;
use futures_util::{FutureExt, StreamExt};
use std::{panic::AssertUnwindSafe, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

async fn docker(args: &[&str]) -> String {
    let output = run(
        ProcessRequest::new("docker")
            .args(["--host", "unix:///var/run/docker.sock"])
            .args(args.iter().copied())
            .limits(ProcessLimits {
                timeout: Duration::from_secs(90),
                ..Default::default()
            }),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(
        output.succeeded(),
        "Docker fixture command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

async fn wait_session(
    registry: &EdgeRegistry,
    target: &EdgeTarget,
    previous: Option<Uuid>,
) -> std::sync::Arc<citadel_adapters::edge::EdgeSession> {
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if let Ok(session) = registry.get(target)
                && Some(session.id) != previous
            {
                break session;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("production Agent must authenticate/reconnect")
}

#[tokio::test]
#[ignore = "requires production CITADEL_PHASE7_AGENT_IMAGE, AGENT_NETWORK, AGENT_HOST and isolated CITADEL_PHASE6_SWARM_MANAGER / DOCKER_SOCKET"]
async fn production_edge_agent_reconnects_streams_events_and_fences_stale_writers() {
    let image = std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap();
    let network = std::env::var("CITADEL_PHASE7_AGENT_NETWORK").unwrap();
    let host = std::env::var("CITADEL_PHASE7_AGENT_HOST").unwrap();
    let manager = std::env::var("CITADEL_PHASE6_SWARM_MANAGER").unwrap();
    assert_eq!(
        docker(&[
            "inspect",
            "--format",
            "{{index .Config.Labels \"citadel.test\"}}",
            &manager
        ])
        .await,
        "jobs-live"
    );
    let socket = std::env::var("CITADEL_PHASE6_DOCKER_SOCKET").unwrap();
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let target = EdgeTarget::platform(Uuid::now_v7());
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,'EdgeAgent',0,0,0,$1::text,0,'{\"$type\":\"Docker\"}','Offline',0)")
        .bind(target.platform_id).execute(&pool).await.unwrap();
    let store = PostgresEdgeStore::new(pool.clone());
    let (_, token, _) = store
        .create_enrollment(&target, citadel_identity::SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    let registry = EdgeRegistry::default();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let stop = CancellationToken::new();
    let shutdown = stop.clone();
    let intake = EdgeIntake::new(store.clone(), registry.clone());
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(EdgeAgentServiceServer::new(intake))
            .serve_with_incoming_shutdown(
                futures_util::stream::unfold(listener, |listener| async move {
                    Some((listener.accept().await.map(|(stream, _)| stream), listener))
                }),
                shutdown.cancelled_owned(),
            )
            .await
            .unwrap();
    });
    let agent = format!("citadel-parity-edge-{}", target.platform_id.simple());
    let workload = format!("citadel-parity-event-{}", target.platform_id.simple());
    let result = AssertUnwindSafe(async {
        docker(&[
            "run",
            "-d",
            "--name",
            &agent,
            "--label",
            "citadel.test=jobs-live",
            "--network",
            &network,
            "--mount",
            &format!("type=bind,source={socket},target=/var/run/docker.sock"),
            "--env",
            "CITADEL_AGENT_MODE=edge",
            "--env",
            &format!("CITADEL_CORE_URL=http://{host}:{port}"),
            "--env",
            &format!("CITADEL_EDGE_ENROLLMENT_TOKEN={token}"),
            &image,
        ])
        .await;
        let first = wait_session(&registry, &target, None).await;
        let runtime = EdgeRuntime {
            session: first.clone(),
        };
        let info = runtime.get_info(&stop).await.unwrap();
        assert!(!info.daemon_id.is_empty());
        let mut events = first
            .command(
                EdgeCommandKind::PlatformDaemonEventsStream,
                vec![],
                Duration::from_secs(120),
                true,
            )
            .unwrap();
        let mut stats = runtime
            .stream_stats(Duration::from_secs(1), &stop)
            .await
            .unwrap();
        let sample = tokio::time::timeout(Duration::from_secs(30), stats.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(sample.mem_total > 0);
        tokio::time::sleep(Duration::from_millis(500)).await;
        let container = docker(&[
            "exec",
            &manager,
            "docker",
            "run",
            "-d",
            "--name",
            &workload,
            "alpine:3.21",
            "sleep",
            "300",
        ])
        .await;
        expect_event(&mut events, &container, "start", &stop).await;
        docker(&["exec", &manager, "docker", "stop", "-t", "1", &workload]).await;
        expect_event(&mut events, &container, "die", &stop).await;
        let containers = runtime.list_containers(&stop).await.unwrap();
        assert!(
            containers
                .iter()
                .any(|c| c.id == container && c.state.eq_ignore_ascii_case("exited"))
        );
        // Restart the actual process with its saved enrollment identity.
        docker(&["restart", "-t", "1", &agent]).await;
        assert!(
            tokio::time::timeout(Duration::from_secs(10), events.next(&stop))
                .await
                .unwrap()
                .is_err()
        );
        let second = wait_session(&registry, &target, Some(first.id)).await;
        assert_eq!(
            first.agent_id, second.agent_id,
            "restart reuses the enrolled Agent identity"
        );
        assert!(
            store
                .persist_stats_with_platform_stats(&first, &[], Some(&sample))
                .await
                .is_err()
        );
        store
            .persist_stats_with_platform_stats(&second, &[], Some(&sample))
            .await
            .unwrap();
        let mut next_events = second
            .command(
                EdgeCommandKind::PlatformDaemonEventsStream,
                vec![],
                Duration::from_secs(120),
                true,
            )
            .unwrap();
        let second_runtime = EdgeRuntime {
            session: second.clone(),
        };
        assert_eq!(
            second_runtime.get_info(&stop).await.unwrap().daemon_id,
            info.daemon_id
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
        docker(&["exec", &manager, "docker", "start", &workload]).await;
        expect_event(&mut next_events, &container, "start", &stop).await;
        // Drop the authenticated transport; the Agent must reconnect itself.
        registry.disconnect(&target);
        assert!(
            store
                .persist_stats_with_platform_stats(&second, &[], Some(&sample))
                .await
                .is_err(),
            "a closed session is rejected even before a successor persists its connection"
        );
        let third = wait_session(&registry, &target, Some(second.id)).await;
        assert_eq!(third.agent_id, first.agent_id);
        assert!(
            store
                .persist_stats_with_platform_stats(&second, &[], Some(&sample))
                .await
                .is_err()
        );
        let third_runtime = EdgeRuntime { session: third };
        assert_eq!(
            third_runtime.get_info(&stop).await.unwrap().daemon_id,
            info.daemon_id
        );
        store.revoke(&target).await.unwrap();
        registry.disconnect(&target);
        tokio::time::sleep(Duration::from_secs(4)).await;
        assert!(
            registry.get(&target).is_err(),
            "revoked Agent cannot reconnect"
        );
    })
    .catch_unwind()
    .await;
    // Cleanup runs even when an assertion fails. Only this fixture's names are touched.
    if result.is_err() {
        let logs = docker(&["logs", "--tail", "40", &agent]).await;
        eprintln!("Production Agent diagnostics: {logs}");
    }
    docker(&["rm", "-f", "-v", &agent]).await;
    // The workload might not have been created if enrollment failed.
    let _ = run(
        ProcessRequest::new("docker").args([
            "--host",
            "unix:///var/run/docker.sock",
            "exec",
            &manager,
            "docker",
            "rm",
            "-f",
            "-v",
            &workload,
        ]),
        &CancellationToken::new(),
    )
    .await;
    registry.disconnect(&target);
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    pool.close().await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

async fn expect_event(
    stream: &mut citadel_adapters::edge::EdgeCommandStream,
    container: &str,
    action: &str,
    stop: &CancellationToken,
) {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let payload = stream
                .next(stop)
                .await
                .unwrap()
                .expect("daemon stream ended");
            let Some(event) = citadel_adapters::agent::decode_daemon_event(&payload).unwrap()
            else {
                continue;
            };
            if event.container_id.as_deref() == Some(container) && event.action == action {
                break;
            }
        }
    })
    .await
    .expect("expected actual Docker event through production Agent");
}
