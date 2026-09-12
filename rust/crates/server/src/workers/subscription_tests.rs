use super::*;
use citadel_adapters::agent::AgentRequestSigner;
use citadel_contracts::citadel::platforms::v1::{
    platform_service_server::{PlatformService, PlatformServiceServer},
    *,
};
use citadel_contracts::citadel::shared_models::v1::PlatformInfoResponse;
use std::{
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};
use tonic::{Request, Response, Status};

#[derive(Clone, Default)]
struct RejectingAgent {
    delay: bool,
    events: Arc<AtomicUsize>,
    handshakes: Arc<AtomicUsize>,
}
#[tonic::async_trait]
impl PlatformService for RejectingAgent {
    async fn get_platform_info(
        &self,
        _: Request<()>,
    ) -> Result<Response<PlatformInfoResponse>, Status> {
        self.handshakes.fetch_add(1, Ordering::SeqCst);
        if self.delay {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
        Err(Status::permission_denied("fixture rejection"))
    }
    async fn check_health(&self, _: Request<()>) -> Result<Response<CheckHealthResponse>, Status> {
        Err(Status::permission_denied("fixture"))
    }
    async fn prune(&self, _: Request<PruneRequest>) -> Result<Response<PruneResponse>, Status> {
        unreachable!()
    }
    type StreamPlatformStatsStream =
        Pin<Box<dyn futures_util::Stream<Item = Result<PlatformStatsResponse, Status>> + Send>>;
    async fn stream_platform_stats(
        &self,
        _: Request<PlatformStatsRequest>,
    ) -> Result<Response<Self::StreamPlatformStatsStream>, Status> {
        unreachable!()
    }
    type StreamDaemonEventStream =
        Pin<Box<dyn futures_util::Stream<Item = Result<DaemonEventResponse, Status>> + Send>>;
    async fn stream_daemon_event(
        &self,
        _: Request<()>,
    ) -> Result<Response<Self::StreamDaemonEventStream>, Status> {
        self.events.fetch_add(1, Ordering::SeqCst);
        Err(Status::permission_denied("fixture rejection"))
    }
}
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn all_direct_agents_subscribe_and_permanent_rejections_do_not_exit_core_worker() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(6)
        .connect(&url)
        .await
        .unwrap();
    let token = CancellationToken::new();
    let mut fixtures = Vec::new();
    let mut servers = Vec::new();
    let mut addresses = Vec::new();
    let mut ids = Vec::new();
    for index in 0..3 {
        let fixture = RejectingAgent::default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let stop = token.clone();
        let service = fixture.clone();
        servers.push(tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(PlatformServiceServer::new(service))
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    stop.cancelled_owned(),
                )
                .await
                .unwrap();
        }));
        let id = uuid::Uuid::now_v7();
        if index < 2 {
            sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$2,$3,'Agent',0,0,0,0,0,'{\"$type\":\"Docker\"}','Online')")
            .bind(id).bind(id.to_string()).bind(&address).execute(&pool).await.unwrap();
        }
        fixtures.push(fixture);
        addresses.push(address);
        ids.push(id);
    }
    let base = AgentClient::connect(
        &addresses[0],
        AgentRequestSigner::from_bytes(&[42; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let (sender, _receiver) = bounded_channel(16, QueueOverflowPolicy::Wait);
    let worker = tokio::spawn(agent_subscriptions(
        token.clone(),
        base,
        sender,
        StatsWorkerContext {
            pool: pool.clone(),
            metrics: Arc::new(Metrics::default()),
            realtime: None,
            fetch_interval: Duration::from_millis(50),
        },
        Duration::from_millis(20),
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while !fixtures[..2].iter().all(|f| {
            f.events.load(Ordering::SeqCst) >= 2 && f.handshakes.load(Ordering::SeqCst) >= 2
        }) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("both Agent event and statistics loops must retry independently");
    assert!(
        !worker.is_finished(),
        "one endpoint rejection must not stop Core"
    );
    assert_eq!(fixtures[2].events.load(Ordering::SeqCst), 0);
    let unaffected = fixtures[1].events.load(Ordering::SeqCst);
    sqlx::query("UPDATE platforms SET address=$2 WHERE id=$1")
        .bind(ids[0])
        .bind(&addresses[2])
        .execute(&pool)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(12), async {
        while fixtures[2].events.load(Ordering::SeqCst) < 2
            || fixtures[2].handshakes.load(Ordering::SeqCst) < 2
            || fixtures[1].events.load(Ordering::SeqCst) <= unaffected
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("reconfigured Agent must resubscribe without interrupting its peer");
    assert!(!worker.is_finished());
    token.cancel();
    tokio::time::timeout(Duration::from_secs(3), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    for server in servers {
        server.await.unwrap();
    }
    sqlx::query("DELETE FROM platforms WHERE id=ANY($1)")
        .bind(ids)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

// Port: PlatformHealthMonitorJobTests Edge enrollment/revocation cases.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn edge_health_suppresses_unenrolled_and_revoked_bindings() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let id = uuid::Uuid::now_v7();
    assert_eq!(edge_health(&pool, id).await.unwrap(), None);
    sqlx::query("INSERT INTO edgeagentbindings(id,agentid,platformid,resourceid,agentfingerprint,agentpublickey,connectionstatus) VALUES($1,$1,$1,$1,$2,'fixture','Offline')")
        .bind(id).bind(id.to_string()).execute(&pool).await.unwrap();
    assert_eq!(edge_health(&pool, id).await.unwrap(), Some(false));
    sqlx::query("UPDATE edgeagentbindings SET connectionstatus='Connected',lastheartbeatatutc=CURRENT_TIMESTAMP WHERE id=$1").bind(id).execute(&pool).await.unwrap();
    assert_eq!(edge_health(&pool, id).await.unwrap(), Some(true));
    sqlx::query("UPDATE edgeagentbindings SET revokedatutc=CURRENT_TIMESTAMP,connectionstatus='Revoked' WHERE id=$1").bind(id).execute(&pool).await.unwrap();
    assert_eq!(edge_health(&pool, id).await.unwrap(), None);
    sqlx::query("DELETE FROM edgeagentbindings WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

// Port: PlatformHealthMonitorJobTests.CheckHealthAsync_ShouldTreatTimeoutAsOffline.
#[tokio::test]
async fn health_probe_times_out_an_unresponsive_agent() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let token = CancellationToken::new();
    let stop = token.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(PlatformServiceServer::new(RejectingAgent {
                delay: true,
                ..Default::default()
            }))
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(listener),
                stop.cancelled_owned(),
            )
            .await
            .unwrap();
    });
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[42; 32]),
        Duration::from_secs(5),
        true,
    )
    .await
    .unwrap();
    let start = tokio::time::Instant::now();
    assert!(!probe_health(&client, &token, Duration::from_millis(20)).await);
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(client);
    token.cancel();
    server.await.unwrap();
}
