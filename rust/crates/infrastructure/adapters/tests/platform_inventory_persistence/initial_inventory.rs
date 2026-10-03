use super::*;
use citadel_platforms::jobs::{ResourceInventory, ResourceSnapshot};

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn scoped_discovery_updates_counts_without_statistics_or_created_alerts() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    let inventory = snapshot(platform, false);
    let mut containers = vec![];
    for (index, state) in ["running", "restarting", "paused", "exited"]
        .into_iter()
        .enumerate()
    {
        let mut container = inventory.containers[0].clone();
        container.id = format!("discovered-{index}");
        container.state = state.into();
        containers.push(container);
    }
    let mut listener = sqlx::postgres::PgListener::connect(&url).await.unwrap();
    listener.listen("citadel_container_created").await.unwrap();
    for resources in [
        ResourceInventory::Containers(containers.clone()),
        ResourceInventory::Images(inventory.images.clone()),
        ResourceInventory::Networks(inventory.networks.clone()),
        ResourceInventory::Volumes(inventory.volumes.clone()),
    ] {
        assert!(
            store
                .persist_resource_committed(
                    &ResourceSnapshot {
                        platform_id: platform,
                        observed_at: Utc::now(),
                        inventory: resources,
                    },
                    None
                )
                .await
                .unwrap()
                .changed()
        );
    }
    let (descriptor, images, networks, volumes): (serde_json::Value, i32, i32, i32) =
        sqlx::query_as("SELECT platformdescriptor,imagecount,networkcount,volumecount FROM platforms WHERE id=$1")
            .bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(descriptor["containerCount"], 4);
    assert_eq!(descriptor["containersRunning"], 2);
    assert_eq!(descriptor["containersPaused"], 1);
    assert_eq!(descriptor["containersStopped"], 1);
    assert_eq!(images as usize, inventory.images.len());
    assert_eq!(networks as usize, inventory.networks.len());
    assert_eq!(volumes as usize, inventory.volumes.len());
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM platformstats WHERE platformid=$1)"
        )
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    // Discovery is silent for both initial connection and subsequent recovery.
    for values in [containers, vec![]] {
        store
            .persist_resource(&ResourceSnapshot {
                platform_id: platform,
                observed_at: Utc::now(),
                inventory: ResourceInventory::Containers(values),
            })
            .await
            .unwrap();
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.recv())
            .await
            .is_err()
    );
    let descriptor: serde_json::Value =
        sqlx::query_scalar("SELECT platformdescriptor FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(descriptor["containerCount"], 0);
    assert_eq!(descriptor["containersRunning"], 0);
    assert_eq!(descriptor["containersPaused"], 0);
    assert_eq!(descriptor["containersStopped"], 0);
    drop(listener);
    pool.close().await;
}
