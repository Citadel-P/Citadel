use chrono::{Duration, Utc};
use citadel_adapters::persistence::postgres::connection_events::{
    CHANNEL, ConnectionEvent, ConnectionResource, ConnectionState,
};
use citadel_adapters::{
    connectors::edge::EdgeTarget,
    persistence::postgres::{
        alerts::PostgresAlertRepository, builds::PostgresBuildRepository,
        platforms::edge::store::PostgresEdgeStore,
    },
};
use citadel_builds::{BuildAgentPoolConfiguration, BuildPoolCheck, BuildRepository};
use citadel_contracts::citadel::edge::v1::EnrollmentRequest;
use citadel_primitives::ActorId;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn assert_events(
    listener: &mut sqlx::postgres::PgListener,
    id: Uuid,
    expected: &[ConnectionState],
) {
    let mut actual = Vec::new();
    while let Ok(notification) =
        tokio::time::timeout(std::time::Duration::from_millis(150), listener.recv()).await
    {
        let event: ConnectionEvent = serde_json::from_str(notification.unwrap().payload()).unwrap();
        if event.resource_id == id {
            assert_eq!(event.resource, ConnectionResource::BuildAgentPool);
            actual.push(event.state);
        }
    }
    assert_eq!(actual, expected);
}

async fn database() -> PgPool {
    let url = std::env::var("CITADEL_POOL_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    PgPool::connect(&url).await.unwrap()
}
async fn pool(store: &PostgresBuildRepository) -> citadel_builds::BuildAgentPool {
    let input: BuildAgentPoolConfiguration = serde_json::from_value(json!({
        "name": format!("pool-{}", Uuid::now_v7()), "enabled":true,
        "providerSpec":{"$type":"SelfManagedVm","connectionMode":"EdgeAgent"}
    }))
    .unwrap();
    store
        .create_pool(ActorId::new(Uuid::from_u128(1)), &input)
        .await
        .unwrap()
}
async fn enroll(
    store: &PostgresEdgeStore,
    id: Uuid,
) -> citadel_adapters::persistence::postgres::platforms::edge::store::EdgeBinding {
    let (_, token, _) = store
        .create_enrollment(&EdgeTarget::build_pool(id), Uuid::from_u128(1))
        .await
        .unwrap();
    let mut key = [0u8; 32];
    key[..16].copy_from_slice(id.as_bytes());
    store.enroll(&EnrollmentRequest {
        enrollment_token: token,
        public_key: ed25519_dalek::SigningKey::from_bytes(&key).verifying_key().to_bytes().to_vec(),
        protocol_version: citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION,
        daemon_id: Uuid::now_v7().to_string(),
        capabilities_json: json!({"commands":["platform.checkHealth","containers.list","containers.logs","images.build","images.push","images.checkBuildHost"]}).to_string(),
        ..Default::default()
    }).await.unwrap()
}
async fn drain(alerts: &PostgresAlertRepository) {
    while alerts
        .process_pending_observation(Uuid::now_v7())
        .await
        .unwrap()
    {}
}
async fn active(db: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM alertevents WHERE resourceid=$1 AND type='BuildAgentPoolUnavailable' AND resolvedat IS NULL")
        .bind(id).fetch_one(db).await.unwrap()
}
async fn check(builds: &PostgresBuildRepository, id: Uuid, ready: bool) {
    let current = builds.get_pool(id).await.unwrap();
    builds
        .record_pool_health(
            &current,
            &BuildPoolCheck {
                ready,
                message: if ready {
                    "Docker ready"
                } else {
                    "Docker unavailable"
                }
                .into(),
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_POOL_DATABASE_URL"]
async fn connection_transitions_are_atomic_deduplicated_and_session_fenced() {
    let db = database().await;
    let builds = PostgresBuildRepository::new(db.clone());
    let pool = pool(&builds).await;
    let mut listener = sqlx::postgres::PgListener::connect_with(&db).await.unwrap();
    listener.listen(CHANNEL).await.unwrap();
    let edge = PostgresEdgeStore::new(db.clone());
    let binding = enroll(&edge, pool.id).await;
    let at = Utc::now();
    edge.connected(&binding, at).await.unwrap();
    edge.connected(&binding, at).await.unwrap();
    edge.disconnected(binding.agent_id, at - Duration::seconds(1))
        .await
        .unwrap();
    assert_events(&mut listener, pool.id, &[ConnectionState::Online]).await;
    assert_eq!(
        builds
            .get_pool(pool.id)
            .await
            .unwrap()
            .connection_status
            .as_deref(),
        Some("Connected")
    );
    edge.disconnected(binding.agent_id, at).await.unwrap();
    edge.disconnected(binding.agent_id, at).await.unwrap();
    assert_events(&mut listener, pool.id, &[ConnectionState::Offline]).await;
    let transitions: Vec<(String, String)> = sqlx::query_as("SELECT eventtype,status FROM activityevents WHERE resourceid=$1 AND eventtype IN ('BuildAgentPoolConnected','BuildAgentPoolDisconnected') ORDER BY id")
        .bind(pool.id).fetch_all(&db).await.unwrap();
    assert_eq!(
        transitions,
        vec![
            ("BuildAgentPoolConnected".into(), "Success".into()),
            ("BuildAgentPoolDisconnected".into(), "Warning".into())
        ]
    );
    let reconnected = at + Duration::seconds(1);
    edge.connected(&binding, reconnected).await.unwrap();
    edge.disconnected(binding.agent_id, at).await.unwrap();
    edge.expire_build_pool_connections().await.unwrap();
    assert_events(&mut listener, pool.id, &[ConnectionState::Online]).await;
    sqlx::query("UPDATE edgeagentbindings SET lastheartbeatatutc=now()-interval '2 minutes' WHERE agentid=$1").bind(binding.agent_id).execute(&db).await.unwrap();
    edge.expire_build_pool_connections().await.unwrap();
    edge.expire_build_pool_connections().await.unwrap();
    assert_events(&mut listener, pool.id, &[ConnectionState::Offline]).await;
    edge.connected(&binding, reconnected + Duration::seconds(1))
        .await
        .unwrap();
    // Reconnect can beat the expiry sweep; preserve both transitions atomically.
    sqlx::query("UPDATE edgeagentbindings SET lastheartbeatatutc=now()-interval '2 minutes' WHERE agentid=$1")
        .bind(binding.agent_id).execute(&db).await.unwrap();
    edge.connected(&binding, reconnected + Duration::seconds(2))
        .await
        .unwrap();
    assert_events(
        &mut listener,
        pool.id,
        &[
            ConnectionState::Online,
            ConnectionState::Offline,
            ConnectionState::Online,
        ],
    )
    .await;
    edge.revoke(&binding.target).await.unwrap();
    edge.disconnected(binding.agent_id, reconnected + Duration::seconds(1))
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='BuildAgentPoolDisconnected'").bind(pool.id).fetch_one(&db).await.unwrap();
    assert_eq!(count, 4);
    assert_events(&mut listener, pool.id, &[ConnectionState::Revoked]).await;
    edge.revoke(&binding.target).await.unwrap();
    assert_events(&mut listener, pool.id, &[]).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_POOL_DATABASE_URL"]
async fn platforms_publish_only_committed_status_transitions() {
    let db = database().await;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms (id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES ($1,'unix:///test.sock','Local',0,0,0,'Event test',0,'{\"$type\":\"Docker\"}','Offline',0)")
        .bind(id).execute(&db).await.unwrap();
    let mut listener = sqlx::postgres::PgListener::connect_with(&db).await.unwrap();
    listener.listen(CHANNEL).await.unwrap();
    use citadel_adapters::persistence::postgres::platforms::status::{
        platform_offline, platform_online,
    };
    platform_online(&db, id).await.unwrap();
    platform_online(&db, id).await.unwrap();
    platform_offline(&db, id).await.unwrap();
    platform_offline(&db, id).await.unwrap();
    for state in [ConnectionState::Online, ConnectionState::Offline] {
        let notification = tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<ConnectionEvent>(notification.payload()).unwrap(),
            ConnectionEvent {
                resource: ConnectionResource::Platform,
                resource_id: id,
                state
            }
        );
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(150), listener.recv())
            .await
            .is_err()
    );
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_POOL_DATABASE_URL"]
async fn availability_obeys_grace_recovery_and_monitoring_exclusions() {
    let db = database().await;
    let builds = PostgresBuildRepository::new(db.clone());
    let alerts = PostgresAlertRepository::new(db.clone());
    let edge = PostgresEdgeStore::new(db.clone());
    let pool = pool(&builds).await;
    check(&builds, pool.id, false).await;
    drain(&alerts).await;
    assert_eq!(active(&db, pool.id).await, 0, "never enrolled");
    let since: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT unavailablesince FROM buildagentpools WHERE id=$1")
            .bind(pool.id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(since.is_none());
    let binding = enroll(&edge, pool.id).await;
    edge.connected(&binding, Utc::now()).await.unwrap();
    // Transport remains connected while Docker fails.
    check(&builds, pool.id, false).await;
    drain(&alerts).await;
    assert_eq!(active(&db, pool.id).await, 0, "within grace period");
    sqlx::query(
        "UPDATE buildagentpools SET unavailablesince=now()-interval '91 seconds' WHERE id=$1",
    )
    .bind(pool.id)
    .execute(&db)
    .await
    .unwrap();
    check(&builds, pool.id, false).await;
    drain(&alerts).await;
    assert_eq!(active(&db, pool.id).await, 1);
    check(&builds, pool.id, false).await;
    drain(&alerts).await;
    assert_eq!(active(&db, pool.id).await, 1, "deduplicated incident");
    // A pending failure receipt must not reopen the incident after recovery.
    check(&builds, pool.id, false).await;
    check(&builds, pool.id, true).await;
    drain(&alerts).await;
    assert_eq!(active(&db, pool.id).await, 0);
    for archive in [false, true] {
        check(&builds, pool.id, false).await;
        sqlx::query(
            "UPDATE buildagentpools SET unavailablesince=now()-interval '91 seconds' WHERE id=$1",
        )
        .bind(pool.id)
        .execute(&db)
        .await
        .unwrap();
        check(&builds, pool.id, false).await;
        drain(&alerts).await;
        assert_eq!(active(&db, pool.id).await, 1);
        if archive {
            builds
                .archive_pool(pool.id, ActorId::new(Uuid::from_u128(1)))
                .await
                .unwrap();
        } else {
            let current = builds.get_pool(pool.id).await.unwrap();
            let mut input = BuildAgentPoolConfiguration::from(&current);
            input.enabled = false;
            builds
                .update_pool(&current, &input, ActorId::new(Uuid::from_u128(1)))
                .await
                .unwrap();
            drain(&alerts).await;
            assert_eq!(active(&db, pool.id).await, 0, "disabled pool");
            let current = builds.get_pool(pool.id).await.unwrap();
            input.enabled = true;
            builds
                .update_pool(&current, &input, ActorId::new(Uuid::from_u128(1)))
                .await
                .unwrap();
        }
        drain(&alerts).await;
        assert_eq!(active(&db, pool.id).await, 0);
    }
}
