use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;

// Executes the actual worker loop: burst coalescing and one follow-up when an
// event arrives during Docker I/O. A closed realtime receiver cannot stale a commit.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn daemon_bursts_coalesce_and_events_during_refresh_schedule_one_follow_up() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let platform = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status,clusterid) VALUES($1,$1::text,$1::text,'Local',0,0,0,0,0,'{\"$type\":\"DockerSwarm\",\"nodeID\":\"manager\"}','Online','cluster')")
        .bind(platform).execute(&pool).await.unwrap();
    // Exclude fixtures belonging to other tests in this isolated database.
    let previous: Vec<(uuid::Uuid, String)> =
        sqlx::query_as("SELECT id,connectortype FROM platforms WHERE id<>$1")
            .bind(platform)
            .fetch_all(&pool)
            .await
            .unwrap();
    sqlx::query("UPDATE platforms SET connectortype='EdgeAgent' WHERE id<>$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let socket = std::env::temp_dir().join(format!("citadel-inventory-{}.sock", platform.simple()));
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let block = Arc::new(AtomicBool::new(false));
    let entered = Arc::new(Semaphore::new(0));
    let resume = Arc::new(Semaphore::new(0));
    let stop = CancellationToken::new();
    let server_stop = stop.clone();
    let (calls, blocking, reached, release) = (
        count.clone(),
        block.clone(),
        entered.clone(),
        resume.clone(),
    );
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = tokio::select! {
                () = server_stop.cancelled() => break,
                accepted = listener.accept() => accepted.unwrap(),
            };
            let mut bytes = vec![];
            let mut chunk = [0; 2048];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..n]);
            }
            // Cancellation may close a newly accepted connection before its
            // request is written. That is a normal transport shutdown.
            if !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            let path = text
                .split_whitespace()
                .nth(1)
                .unwrap()
                .split('?')
                .next()
                .unwrap();
            let body = if path.ends_with("/version") {
                r#"{"Version":"27","ApiVersion":"1.49","MinAPIVersion":"1.41"}"#
            } else if path.ends_with("/info") {
                if blocking.swap(false, Ordering::SeqCst) {
                    reached.add_permits(1);
                    release.acquire().await.unwrap().forget();
                }
                r#"{"ID":"daemon","NCPU":1,"MemTotal":1024,"OSType":"linux","Swarm":{"NodeID":"manager","LocalNodeState":"active","ControlAvailable":true,"Cluster":{"ID":"cluster"}}}"#
            } else if path.ends_with("/nodes") {
                calls.fetch_add(1, Ordering::SeqCst);
                r#"[{"ID":"manager","Spec":{"Role":"manager","Availability":"active"},"Description":{"Hostname":"manager"},"Status":{"State":"ready"}}]"#
            } else if path.ends_with("/volumes") {
                r#"{"Volumes":[],"Warnings":[]}"#
            } else {
                "[]"
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let (local, local_triggers) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let (_agent, agent_triggers) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let hub = RealtimeHub::new(16, Arc::new(Metrics::default()));
    let mut updates = hub.subscribe();
    // A session-local missing schema makes enumeration fail without changing
    // tables. The same production worker must retry once the connection recovers.
    sqlx::query("SET search_path TO citadel_missing_test_schema")
        .execute(&pool)
        .await
        .unwrap();
    let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
    assert!(targets.refresh().await.is_err());
    let worker = tokio::spawn(inventory_reconciliation(
        stop.clone(),
        InventoryReconciliationWorker {
            node_agent_policy: Default::default(),
            docker: DockerClient::new(&socket, Duration::from_secs(5)).unwrap(),
            targets: targets.clone(),
            budget: targets.inventory_budget.clone(),
            pool: pool.clone(),
            local_triggers,
            agent_triggers,
            agent_overflow: Arc::new(AtomicBool::new(false)),
            realtime: Some(hub),
            interval: Duration::from_secs(3600),
            retry_delay: Duration::from_millis(20),
        },
    ));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !worker.is_finished(),
        "enumeration errors must not terminate the worker"
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
    sqlx::query("SET search_path TO public")
        .execute(&pool)
        .await
        .unwrap();
    targets.refresh().await.unwrap();
    local.try_send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), updates.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    for _ in 0..100 {
        let _ = local.try_send(());
    }
    tokio::time::timeout(Duration::from_secs(5), updates.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert!(
        tokio::time::timeout(Duration::from_millis(600), updates.recv())
            .await
            .is_err()
    );
    block.store(true, Ordering::SeqCst);
    local.try_send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), entered.acquire())
        .await
        .unwrap()
        .unwrap()
        .forget();
    for _ in 0..100 {
        let _ = local.try_send(());
    }
    resume.add_permits(1);
    for _ in 0..2 {
        tokio::time::timeout(Duration::from_secs(5), updates.recv())
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(count.load(Ordering::SeqCst), 4);
    assert!(
        tokio::time::timeout(Duration::from_millis(600), updates.recv())
            .await
            .is_err()
    );
    drop(updates);
    sqlx::query("UPDATE swarmnodeprojections SET isstale=true WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    local.try_send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let fresh: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmnodeprojections WHERE platformid=$1 AND NOT isstale)")
                .bind(platform).fetch_one(&pool).await.unwrap();
            if fresh && count.load(Ordering::SeqCst) == 5 { break; }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.unwrap();
    stop.cancel();
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    server.await.unwrap();
    std::fs::remove_file(socket).unwrap();
    for (id, connector) in previous {
        sqlx::query("UPDATE platforms SET connectortype=$2 WHERE id=$1")
            .bind(id)
            .bind(connector)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
