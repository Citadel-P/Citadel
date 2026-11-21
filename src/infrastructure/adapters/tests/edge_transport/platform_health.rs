use super::*;
use citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore;
use citadel_platforms::jobs::{
    ResourceInventory, ResourceSnapshot, background::EdgeHealthReader,
    observations::PlatformLifecycleStore,
};
use citadel_platforms::{RuntimeContainerSummary, RuntimeInventorySnapshot, RuntimePlatformInfo};

async fn activity(pool: &PgPool, platform: Uuid) -> Vec<String> {
    sqlx::query_scalar("SELECT eventtype FROM activityevents WHERE resourceid=$1 AND eventtype IN ('PlatformConnected','PlatformDisconnected') ORDER BY createdat,id")
        .bind(platform).fetch_all(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn platform_activity_tracks_confirmed_health_across_edge_session_reconnects() {
    let (pool, store, target) = setup().await;
    let platform = target.platform_id;
    let lifecycle = PostgresInventoryProjectionStore::new(pool.clone());
    let registry = EdgeRegistry::default();
    let (_, token, _) = store
        .create_enrollment(&target, SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    let binding = store
        .enroll(&enrollment(
            token,
            &SigningKey::from_bytes(&[47; 32]),
            Uuid::now_v7().to_string(),
        ))
        .await
        .unwrap();
    let (first, _outbound) = registry.register(target.clone(), binding.agent_id).unwrap();
    store.connected(&binding, first.connected_at).await.unwrap();
    let evidence = store.health(&[platform]).await.unwrap()[0];
    assert!(evidence.healthy);
    let snapshot = RuntimeInventorySnapshot {
        platform_id: platform,
        observed_at: chrono::Utc::now(),
        info: RuntimePlatformInfo {
            daemon_id: binding.daemon_id.clone(),
            ..Default::default()
        },
        containers: vec![RuntimeContainerSummary {
            id: "running".into(),
            name: "app".into(),
            state: "running".into(),
            ..Default::default()
        }],
        volumes: vec![],
        networks: vec![],
        images: vec![],
        swarm: None,
    };
    store.persist_inventory(&first, &snapshot).await.unwrap();
    let metadata = ResourceSnapshot {
        platform_id: platform,
        observed_at: snapshot.observed_at,
        inventory: ResourceInventory::Platform(Box::new(snapshot.info.clone())),
    };
    store.persist_resource(&first, &metadata).await.unwrap();
    assert!(activity(&pool, platform).await.is_empty());
    assert!(
        lifecycle
            .set_online(platform, true, Some(evidence.binding))
            .await
            .unwrap()
    );

    store
        .disconnected(binding.agent_id, first.connected_at)
        .await
        .unwrap();
    assert!(!store.health(&[platform]).await.unwrap()[0].healthy);
    assert!(store.persist_resource(&first, &metadata).await.is_err());
    let state: String = sqlx::query_scalar("SELECT state FROM containers WHERE platformid=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "Running");
    let (second, _outbound) = registry.register(target.clone(), binding.agent_id).unwrap();
    store
        .connected(&binding, second.connected_at)
        .await
        .unwrap();
    store
        .disconnected(binding.agent_id, first.connected_at)
        .await
        .unwrap();
    store.persist_resource(&second, &metadata).await.unwrap();
    assert!(store.health(&[platform]).await.unwrap()[0].healthy);
    assert_eq!(activity(&pool, platform).await, ["PlatformConnected"]);

    store
        .disconnected(binding.agent_id, second.connected_at)
        .await
        .unwrap();
    assert!(
        lifecycle
            .set_online(platform, false, Some(evidence.binding))
            .await
            .unwrap()
    );
    let (third, _outbound) = registry.register(target.clone(), binding.agent_id).unwrap();
    store.connected(&binding, third.connected_at).await.unwrap();
    store.persist_inventory(&third, &snapshot).await.unwrap();
    assert_eq!(
        activity(&pool, platform).await,
        ["PlatformConnected", "PlatformDisconnected"]
    );
    assert!(
        lifecycle
            .set_online(platform, true, Some(evidence.binding))
            .await
            .unwrap()
    );
    assert_eq!(
        activity(&pool, platform).await,
        [
            "PlatformConnected",
            "PlatformDisconnected",
            "PlatformConnected"
        ]
    );
    store.revoke(&target).await.unwrap();
    assert!(
        !lifecycle
            .set_online(platform, true, Some(evidence.binding))
            .await
            .unwrap()
    );
}
