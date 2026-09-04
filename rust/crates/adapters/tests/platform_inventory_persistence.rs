use std::collections::BTreeMap;

use chrono::Utc;
use citadel_adapters::container_stats_store::PostgresContainerStatsStore;
use citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore;
use citadel_adapters::platform_read_store::PostgresPlatformReadStore;
use citadel_database::MigrationRunner;
use citadel_domain::ActorId;
use citadel_platforms::{
    ContainerStatsStore, InventoryProjectionStore, PlatformReadStore, RuntimeContainerStat,
    RuntimeContainerSummary, RuntimeImageSummary, RuntimeInventorySnapshot, RuntimeNetworkSummary,
    RuntimePlatformInfo, RuntimeSwarmConfig, RuntimeSwarmInventory, RuntimeSwarmNode,
    RuntimeSwarmSecret, RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn projections_stats_and_authorized_reads_survive_store_recreation() {
    let database_url = std::env::var("CITADEL_PHASE4_DATABASE_URL")
        .expect("CITADEL_PHASE4_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let platform_id = Uuid::now_v7();
    let actor_id = Uuid::now_v7();
    let tag_id = Uuid::now_v7();
    seed_platform(&pool, platform_id, actor_id, tag_id).await;

    let first_snapshot = snapshot(platform_id, true);
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&first_snapshot)
        .await
        .unwrap();
    let observed_at = Utc::now().timestamp();
    assert_eq!(
        PostgresContainerStatsStore::new(pool.clone())
            .persist(
                platform_id,
                &[RuntimeContainerStat {
                    docker_container_id: "container-1".into(),
                    memory_active: 128.0,
                    memory_cache: 32.0,
                    cpu_usage: 4.5,
                    memory_limit: 1024.0,
                    rx_bytes: 12.0,
                    tx_bytes: 34.0,
                    created: observed_at,
                }],
            )
            .await
            .unwrap(),
        1
    );
    let platform_sample_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM platformstats WHERE platformid = $1")
            .bind(platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        PostgresContainerStatsStore::new(pool.clone())
            .persist(
                platform_id,
                &[RuntimeContainerStat {
                    docker_container_id: "not-in-this-platform".into(),
                    memory_active: 4096.0,
                    memory_cache: 0.0,
                    cpu_usage: 100.0,
                    memory_limit: 4096.0,
                    rx_bytes: 1000.0,
                    tx_bytes: 1000.0,
                    created: observed_at + 1,
                }],
            )
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM platformstats WHERE platformid = $1")
            .bind(platform_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        platform_sample_count,
        "unknown Docker container statistics must not pollute Platform aggregates"
    );

    // Recreate the adapter to prove that reads come from PostgreSQL, not process memory.
    let reads = PostgresPlatformReadStore::new(pool.clone());
    let platforms = reads
        .list_authorized(ActorId::new(actor_id), true, &[tag_id])
        .await
        .unwrap();
    assert_eq!(platforms.len(), 1);
    assert_eq!(platforms[0].server_version.as_deref(), Some("28.0.0"));
    assert_eq!(platforms[0].image_count, 2);
    assert_eq!(platforms[0].platform_descriptor["nodes"], 1);
    assert_eq!(platforms[0].platform_descriptor["serviceCount"], 1);
    assert!(
        platforms[0]
            .stats
            .as_ref()
            .is_some_and(|stats| stats.len() == 1)
    );

    let container = reads
        .list_containers(platform_id)
        .await
        .unwrap()
        .into_iter()
        .find(|container| container.container_id == "container-1")
        .unwrap();
    assert_eq!(container.name, "web");
    assert_eq!(container.last_stats.unwrap().cpu_usage, 4.5);
    let images = reads.list_images(platform_id).await.unwrap();
    assert_eq!(images.len(), 2);
    assert!(
        images
            .iter()
            .any(|image| image.name == "localhost:5000/team/api")
    );

    let node = reads
        .get_swarm_node(platform_id, "node-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(node.running_task_count, 1);
    assert_eq!(
        reads.list_swarm_services(platform_id).await.unwrap()[0].ownership,
        "CitadelService"
    );
    assert_eq!(
        reads
            .list_swarm_tasks(platform_id, Some("service-1"), 1)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(reads.list_swarm_configs(platform_id).await.unwrap()[0].in_use);
    assert!(reads.list_swarm_secrets(platform_id).await.unwrap()[0].in_use);
    assert_eq!(
        reads.list_swarm_networks(platform_id).await.unwrap()[0].subnets,
        ["10.0.0.0/24"]
    );

    // A late Swarm projection failure must roll back Platform, Image, and
    // Container changes made earlier in the same snapshot transaction.
    let mut rejected = snapshot(platform_id, true);
    rejected.containers[0].name = "must-not-commit".into();
    rejected.networks[0].created = "not-a-timestamp".into();
    assert!(
        PostgresInventoryProjectionStore::new(pool.clone())
            .persist(&rejected)
            .await
            .is_err()
    );
    assert_eq!(
        reads.list_containers(platform_id).await.unwrap()[0].name,
        "web"
    );

    // A later complete snapshot repairs counts and marks missing resources stale.
    let mut second = snapshot(platform_id, false);
    second.containers.clear();
    second.images.clear();
    second.swarm.as_mut().unwrap().tasks.clear();
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&second)
        .await
        .unwrap();
    let node = reads
        .get_swarm_node(platform_id, "node-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!((node.running_task_count, node.desired_task_count), (0, 0));
    assert!(reads.list_containers(platform_id).await.unwrap().is_empty());
    assert!(reads.list_images(platform_id).await.unwrap().is_empty());

    pool.close().await;
}

fn snapshot(platform_id: Uuid, include_task: bool) -> RuntimeInventorySnapshot {
    let observed_at = Utc::now();
    let service_id = Uuid::now_v7();
    let service = RuntimeSwarmService {
        id: "service-1".into(),
        version_index: 2,
        name: "web".into(),
        mode: "Replicated".into(),
        image: "nginx:alpine".into(),
        running_task_count: i32::from(include_task),
        desired_task_count: i32::from(include_task),
        update_state: "completed".into(),
        network_ids: vec!["network-1".into()],
        secret_ids: vec!["secret-1".into()],
        config_ids: vec!["config-1".into()],
        labels: BTreeMap::from([
            ("com.citadel.managed".into(), "true".into()),
            ("com.citadel.service-id".into(), service_id.to_string()),
        ]),
        ..RuntimeSwarmService::default()
    }
    .normalize_ownership();
    RuntimeInventorySnapshot {
        platform_id,
        info: RuntimePlatformInfo {
            daemon_id: "daemon-1".into(),
            server_version: "28.0.0".into(),
            operating_system: "Linux".into(),
            os_type: "linux".into(),
            architecture: "x86_64".into(),
            cpu_count: 4,
            memory_total: 4096,
            container_count: 1,
            containers_running: 1,
            containers_paused: 0,
            containers_stopped: 0,
            api_version: "1.49".into(),
            minimum_api_version: "1.24".into(),
            agent_version: None,
            swarm: None,
        },
        containers: vec![RuntimeContainerSummary {
            id: "container-1".into(),
            name: "web".into(),
            image: "nginx:alpine".into(),
            image_id: "sha256:image-1".into(),
            created: observed_at.timestamp(),
            state: "running".into(),
            status: "Up".into(),
            labels: BTreeMap::new(),
            ports: json!([]),
            stack: None,
            is_system: false,
            system_role: None,
            has_citadel_ownership_labels: false,
            is_swarm_task: false,
        }],
        images: vec![
            RuntimeImageSummary {
                id: "sha256:image-1".into(),
                repo_tags: vec!["nginx:alpine".into()],
                repo_digests: vec![],
                created: observed_at.timestamp(),
                size: 1024,
                containers: 1,
            },
            RuntimeImageSummary {
                id: "sha256:image-2".into(),
                repo_tags: vec!["localhost:5000/team/api:latest".into()],
                repo_digests: vec![],
                created: observed_at.timestamp(),
                size: 2048,
                containers: 0,
            },
        ],
        networks: vec![RuntimeNetworkSummary {
            id: "network-1".into(),
            name: "frontend".into(),
            created: observed_at.to_rfc3339(),
            driver: "overlay".into(),
            scope: "swarm".into(),
            attachable: true,
            ipam: Some(json!({"Config":[{"Subnet":"10.0.0.0/24"}]})),
            ..RuntimeNetworkSummary::default()
        }],
        volumes: vec![RuntimeVolumeSummary {
            name: "data".into(),
            driver: "local".into(),
            scope: "local".into(),
            ..RuntimeVolumeSummary::default()
        }],
        swarm: Some(RuntimeSwarmInventory {
            nodes: vec![RuntimeSwarmNode {
                id: "node-1".into(),
                version_index: 1,
                hostname: "worker-1".into(),
                role: "manager".into(),
                is_leader: true,
                reachability: "reachable".into(),
                status: "ready".into(),
                availability: "active".into(),
                engine_version: "28.0.0".into(),
                operating_system: "linux".into(),
                architecture: "x86_64".into(),
                address: "10.0.0.1".into(),
                ..RuntimeSwarmNode::default()
            }],
            services: vec![service],
            tasks: include_task
                .then(|| RuntimeSwarmTask {
                    id: "task-1".into(),
                    version_index: 1,
                    name: "web.1".into(),
                    service_id: "service-1".into(),
                    slot: Some(1),
                    node_id: "node-1".into(),
                    desired_state: "running".into(),
                    state: "running".into(),
                    image: "nginx:alpine".into(),
                    ..RuntimeSwarmTask::default()
                })
                .into_iter()
                .collect(),
            configs: vec![RuntimeSwarmConfig {
                id: "config-1".into(),
                version_index: 1,
                name: "app-config".into(),
                ..RuntimeSwarmConfig::default()
            }],
            secrets: vec![RuntimeSwarmSecret {
                id: "secret-1".into(),
                version_index: 1,
                name: "password".into(),
                ..RuntimeSwarmSecret::default()
            }],
        }),
        observed_at,
    }
}

async fn seed_platform(pool: &sqlx::PgPool, platform_id: Uuid, actor_id: Uuid, tag_id: Uuid) {
    let marker = tag_id.simple().to_string();
    sqlx::query(
        "INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User') ON CONFLICT DO NOTHING",
    )
    .bind(actor_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO platforms (id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES ($1,$2,'Local',0,0,0,$3,0,'{\"$type\":\"DockerSwarm\"}','Online',0)")
        .bind(platform_id)
        .bind(format!("unix:///phase4/{platform_id}.sock"))
        .bind(format!("phase4-{platform_id}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tags (id,color,createdbyactorid,name,normalizedname) VALUES ($1,'blue',$2,$3,$4)")
        .bind(tag_id)
        .bind(actor_id)
        .bind(format!("phase4-{marker}"))
        .bind(format!("PHASE4-{marker}"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO resourcetags (resourcetype,resourceid,tagid,createdbyactorid) VALUES ('Platform',$1,$2,$3)")
        .bind(platform_id)
        .bind(tag_id)
        .bind(actor_id)
        .execute(pool)
        .await
        .unwrap();
}
