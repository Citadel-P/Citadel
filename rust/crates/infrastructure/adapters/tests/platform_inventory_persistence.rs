use std::collections::BTreeMap;

use chrono::Utc;
use citadel_adapters::persistence::postgres::platforms::PostgresPlatformReader;
use citadel_adapters::persistence::postgres::platforms::inventory::store::PostgresInventoryProjectionStore;
use citadel_adapters::persistence::postgres::platforms::statistics::store::PostgresContainerStatsStore;
use citadel_database::MigrationRunner;
use citadel_platforms::{
    ContainerStatsStore, InventoryProjectionStore, PlatformReader, RuntimeContainerStat,
    RuntimeContainerSummary, RuntimeImageSummary, RuntimeInventorySnapshot, RuntimeNetworkSummary,
    RuntimePlatformInfo, RuntimeSwarmConfig, RuntimeSwarmInventory, RuntimeSwarmNode,
    RuntimeSwarmSecret, RuntimeSwarmService, RuntimeSwarmTask, RuntimeVolumeSummary,
};
use citadel_primitives::ActorId;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[path = "platform_inventory_persistence/state_delta.rs"]
mod state_delta;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn inventory_commits_while_a_deployment_is_locked_and_reconciliation_catches_up() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,'{}','Created','Idle',$4)")
        .bind(deployment).bind(format!("concurrent-{deployment}")).bind(platform).bind(actor).execute(&pool).await.unwrap();
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    inventory.containers[0].is_swarm_task = false;
    inventory.containers[0]
        .labels
        .insert("com.citadel.deployment-id".into(), deployment.to_string());
    inventory.containers[0]
        .labels
        .insert("com.citadel.managed".into(), "true".into());
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&inventory).await.unwrap();
    sqlx::query("UPDATE deployments SET status='Healthy' WHERE id=$1")
        .bind(deployment)
        .execute(&pool)
        .await
        .unwrap();
    let mut apply = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM deployments WHERE id=$1 FOR NO KEY UPDATE")
        .bind(deployment)
        .fetch_one(&mut *apply)
        .await
        .unwrap();
    inventory.containers[0].state = "exited".into();
    let mut replacement = inventory.containers[0].clone();
    replacement.id = "replacement".into();
    inventory.containers.push(replacement);
    inventory.observed_at += chrono::Duration::seconds(2);
    tokio::time::timeout(std::time::Duration::from_secs(5), store.persist(&inventory))
        .await
        .unwrap()
        .unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM containers WHERE deploymentid=$1")
        .bind(deployment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        state, "Exited",
        "observations commit independently of the deployment lock"
    );
    let associated: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM containers WHERE deploymentid=$1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        associated, 2,
        "a new container's foreign-key check must not block on the deployment lock"
    );
    // Simulate a process stopping before reconciliation. A fresh sweep must
    // derive the status from the committed observation without replaying Docker.
    apply.commit().await.unwrap();
    let changed =
        citadel_adapters::persistence::postgres::platforms::status::reconcile_deployments(
            &pool, platform, None, false,
        )
        .await
        .unwrap();
    assert_eq!(changed, 1);
    let state: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=$1")
        .bind(deployment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "Stopped");
    pool.close().await;
}

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
    let reads = PostgresPlatformReader::new(pool.clone());
    let platforms = reads
        .list_authorized(ActorId::new(actor_id), true, &[tag_id.to_string()])
        .await
        .unwrap();
    assert_eq!(platforms.len(), 1);
    assert_eq!(platforms[0].server_version.as_deref(), Some("28.0.0"));
    assert_eq!(platforms[0].image_count, 2);
    assert_eq!(platforms[0].platform_descriptor.metadata["nodes"], 1);
    assert_eq!(platforms[0].platform_descriptor.metadata["serviceCount"], 1);
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
        "Unmanaged" // A missing managed owner must remain adoptable.
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

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn inventory_restores_only_platform_scoped_ownership_and_preserves_adoption() {
    let database = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&database).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let other_platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    for id in [platform, other_platform] {
        seed_platform(&pool, id, actor, Uuid::now_v7()).await;
    }
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$2,$3,'WebEditor','{}','{}')")
        .bind(stack).bind(stack.to_string()).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,'{}',$4,'Healthy','1')")
        .bind(release).bind(actor).bind(platform).bind(stack).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&pool)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,createdbyactorid,platformid,spec,status) VALUES($1,$2,$3,$4,'{}','Created')")
        .bind(deployment).bind(deployment.to_string()).bind(actor).bind(platform).execute(&pool).await.unwrap();
    let mut snapshot = snapshot(platform, false);
    snapshot.swarm = None;
    snapshot.info.swarm = None;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let template = snapshot.containers[0].clone();
    snapshot.containers.clear();
    for (name, stack_label, deployment_label, managed) in [
        ("stack", Some(stack.to_string()), None, true),
        ("deployment", None, Some(deployment.to_string()), true),
        (
            "conflicting",
            Some(stack.to_string()),
            Some(deployment.to_string()),
            true,
        ),
        ("malformed", Some("not-a-uuid".into()), None, true),
        ("orphaned", Some(Uuid::now_v7().to_string()), None, true),
        ("unmanaged", Some(stack.to_string()), None, false),
    ] {
        let mut container = template.clone();
        container.id = name.into();
        if managed {
            container
                .labels
                .insert("com.citadel.managed".into(), "true".into());
        }
        if let Some(value) = stack_label {
            container
                .labels
                .insert("com.citadel.stack-id".into(), value);
        }
        if let Some(value) = deployment_label {
            container
                .labels
                .insert("com.citadel.deployment-id".into(), value);
        }
        snapshot.containers.push(container);
    }
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&snapshot).await.unwrap();
    let reads = PostgresPlatformReader::new(pool.clone());
    for container in reads.list_containers(platform).await.unwrap() {
        assert_eq!(
            container.stack_id,
            (container.container_id == "stack").then_some(stack)
        );
        assert_eq!(
            container.deployment_id,
            (container.container_id == "deployment").then_some(deployment)
        );
    }
    // Identical Docker labels on another Platform never grant Stack/Deployment access.
    snapshot.platform_id = other_platform;
    snapshot.info.daemon_id = format!("daemon-{other_platform}");
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(other_platform)
        .execute(&pool)
        .await
        .unwrap();
    store.persist(&snapshot).await.unwrap();
    for container in reads.list_containers(other_platform).await.unwrap() {
        assert!(container.stack_id.is_none() && container.deployment_id.is_none());
    }
    // Adoption can attach ownership without changing Docker labels. Keep those
    // authoritative links, even if a subsequent daemon snapshot disagrees.
    snapshot.platform_id = platform;
    snapshot.info.daemon_id = format!("daemon-{platform}");
    sqlx::query(
        "UPDATE containers SET stackid=$1 WHERE platformid=$2 AND dockercontainerid='unmanaged'",
    )
    .bind(stack)
    .bind(platform)
    .execute(&pool)
    .await
    .unwrap();
    for container in &mut snapshot.containers {
        container.labels.clear();
        container
            .labels
            .insert("com.citadel.managed".into(), "true".into());
        container
            .labels
            .insert("com.citadel.deployment-id".into(), deployment.to_string());
    }
    store.persist(&snapshot).await.unwrap();
    for container in reads.list_containers(platform).await.unwrap() {
        if ["stack", "unmanaged"].contains(&container.container_id.as_str()) {
            assert_eq!(container.stack_id, Some(stack));
            assert_eq!(container.deployment_id, None);
        }
    }
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
            daemon_id: format!("daemon-{platform_id}"),
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
            swarm: Some(citadel_platforms::RuntimeSwarmInfo {
                cluster_id: Some(format!("cluster-{platform_id}")),
                node_id: "node-1".into(),
                control_available: true,
                local_node_state: "active".into(),
                ..Default::default()
            }),
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
            running_task_count: usize::from(include_task),
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

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn disk_metrics_survive_persistence_dashboard_and_history_reads() {
    use citadel_platforms::{HostDiskUsage, StatisticsReader, StatsWindow};
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    let tag = Uuid::now_v7();
    seed_platform(&pool, platform, actor, tag).await;
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&snapshot(platform, true))
        .await
        .unwrap();
    let created = Utc::now().timestamp();
    let stats = [RuntimeContainerStat {
        docker_container_id: "container-1".into(),
        memory_active: 128.0,
        memory_cache: 32.0,
        cpu_usage: 4.5,
        memory_limit: 1024.0,
        rx_bytes: 12.0,
        tx_bytes: 34.0,
        created,
    }];
    let store = PostgresContainerStatsStore::new(pool.clone());
    assert_eq!(
        store
            .persist_with_disk(platform, &stats, HostDiskUsage::new(90, 100, 90.0))
            .await
            .unwrap(),
        1
    );
    let views = PostgresPlatformReader::new(pool.clone())
        .list_authorized(ActorId::new(actor), true, &[tag.to_string()])
        .await
        .unwrap();
    let current = &views[0].stats.as_ref().unwrap()[0];
    assert_eq!(current.cpu_usage, 1.125);
    assert_eq!(current.disk_used_bytes, Some(90));
    assert_eq!(current.disk_total_bytes, Some(100));
    assert_eq!(current.disk_usage, Some(90.0));
    let history =
        citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader::new(pool.clone())
            .platform(platform, StatsWindow::new(24).unwrap(), created)
            .await
            .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].disk_used_bytes, Some(90));
    assert_eq!(history[0].disk_total_bytes, Some(100));
    assert_eq!(history[0].disk_usage, Some(90.0));
    // Invalid or missing remote telemetry must not leave the previous disk value.
    store
        .persist_with_disk(
            platform,
            &stats,
            Some(HostDiskUsage {
                used_bytes: -1,
                total_bytes: 100,
                usage_percent: 95.0,
            }),
        )
        .await
        .unwrap();
    let row: (Option<i64>,Option<i64>,Option<f64>,f64) = sqlx::query_as("SELECT diskusedbytes,disktotalbytes,diskusage,cpuusage FROM platformstats WHERE platformid=$1 ORDER BY created DESC LIMIT 1").bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(row, (None, None, None, 1.125));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn deployment_status_tracks_container_inventory_without_overwriting_active_operations() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,'{}','Healthy','Idle',$4)")
        .bind(deployment).bind(format!("status-{deployment}")).bind(platform).bind(actor)
        .execute(&pool).await.unwrap();
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    store.persist(&inventory).await.unwrap();
    // Adopted containers have no Citadel labels; their persisted binding is authoritative.
    sqlx::query("UPDATE containers SET deploymentid=$1 WHERE platformid=$2")
        .bind(deployment)
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    for (state, expected) in [
        ("exited", "Stopped"),
        ("running", "Healthy"),
        ("paused", "Pending"),
        ("restarting", "Pending"),
        ("created", "Created"),
        ("dead", "Degraded"),
        ("offline", "Degraded"),
        ("unknown", "Failed"),
    ] {
        inventory.containers[0].state = state.into();
        inventory.observed_at += chrono::Duration::seconds(1);
        store.persist(&inventory).await.unwrap();
        let status: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=$1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, expected, "container state {state}");
    }
    sqlx::query("UPDATE deployments SET status='Applying',controlstate='Processing' WHERE id=$1")
        .bind(deployment)
        .execute(&pool)
        .await
        .unwrap();
    inventory.containers[0].state = "exited".into();
    inventory.observed_at += chrono::Duration::seconds(1);
    store.persist(&inventory).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=$1")
        .bind(deployment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Applying");
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn removing_replaced_container_preserves_current_deployment_runtime_status() {
    use citadel_adapters::persistence::postgres::platforms::status::container_event;

    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    for event in [false, true] {
        for (state, expected) in [
            ("running", "Healthy"),
            ("exited", "Stopped"),
            ("paused", "Pending"),
        ] {
            let platform = Uuid::now_v7();
            let actor = Uuid::now_v7();
            seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
            sqlx::query(
                "UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1",
            )
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
            let deployment = Uuid::now_v7();
            sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,'{}','Healthy','Idle',$4)")
                .bind(deployment).bind(format!("replacement-{deployment}")).bind(platform).bind(actor)
                .execute(&pool).await.unwrap();
            let store = PostgresInventoryProjectionStore::new(pool.clone());
            let mut inventory = snapshot(platform, false);
            inventory.swarm = None;
            inventory.info.swarm = None;
            let mut replacement = inventory.containers[0].clone();
            replacement.id = "replacement".into();
            replacement.state = state.into();
            inventory.containers.push(replacement);
            store.persist(&inventory).await.unwrap();
            // Apply has linked the replacement before cleanup observes the old
            // container's removal. Both inventory rows still exist at this point.
            sqlx::query("UPDATE containers SET deploymentid=$2 WHERE platformid=$1")
                .bind(platform)
                .bind(deployment)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("UPDATE deployments SET status='Healthy' WHERE id=$1")
                .bind(deployment)
                .execute(&pool)
                .await
                .unwrap();
            inventory.observed_at += chrono::Duration::seconds(1);
            if event {
                container_event(
                    &pool,
                    platform,
                    None,
                    "container-1",
                    None,
                    None,
                    inventory.observed_at.timestamp(),
                )
                .await
                .unwrap();
            } else {
                inventory
                    .containers
                    .retain(|container| container.id == "replacement");
                store.persist(&inventory).await.unwrap();
            }
            let actual: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=$1")
                .bind(deployment)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(
                actual, expected,
                "replacement state={state}, deletion event={event}"
            );
            let false_degradations: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='DeploymentDegraded'")
                .bind(deployment).fetch_one(&pool).await.unwrap();
            assert_eq!(false_degradations, 0);

            // Removing the final runtime really does degrade the deployment.
            container_event(
                &pool,
                platform,
                None,
                "replacement",
                None,
                None,
                inventory.observed_at.timestamp() + 1,
            )
            .await
            .unwrap();
            let actual: String = sqlx::query_scalar("SELECT status FROM deployments WHERE id=$1")
                .bind(deployment)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(actual, "Degraded");
        }
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn daemon_events_and_missed_event_reconciliation_update_resources_and_activities() {
    use citadel_adapters::persistence::postgres::platforms::status::container_event;
    use citadel_adapters::persistence::postgres::platforms::status::platform_offline;
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,'{}','Healthy','Idle',$4)")
        .bind(deployment).bind(format!("events-{deployment}")).bind(platform).bind(actor).execute(&pool).await.unwrap();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) VALUES($1,$2,$3,'{}','WebEditor','{}')")
        .bind(stack).bind(format!("stack-{stack}")).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,'{}','Healthy','1')")
        .bind(release).bind(stack).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
        .bind(stack)
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let mut web = inventory.containers[0].clone();
    web.id = "web-1".into();
    let mut worker = web.clone();
    worker.id = "web-2".into();
    inventory.containers.extend([web, worker]);
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&inventory).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$2 WHERE platformid=$1 AND dockercontainerid='container-1'").bind(platform).bind(deployment).execute(&pool).await.unwrap();
    sqlx::query("UPDATE containers SET stackid=$2,isswarmtask=FALSE WHERE platformid=$1 AND dockercontainerid LIKE 'web-%'").bind(platform).bind(stack).execute(&pool).await.unwrap();
    // Binding adoption completes before the event sequence begins.
    sqlx::query("UPDATE stackreleases SET status='Healthy' WHERE id=$1")
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    let now = inventory.observed_at.timestamp() + 1;
    for (id, state, expected) in [
        ("container-1", "exited", "Stopped"),
        ("web-1", "exited", "Degraded"),
        ("web-2", "exited", "Stopped"),
        ("web-1", "running", "Degraded"),
        ("web-2", "running", "Healthy"),
    ] {
        assert!(
            container_event(&pool, platform, None, id, Some(state), None, now)
                .await
                .unwrap()
        );
        let status: String = if id == "container-1" {
            sqlx::query_scalar("SELECT status FROM deployments WHERE id=$1")
                .bind(deployment)
                .fetch_one(&pool)
                .await
                .unwrap()
        } else {
            sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
                .bind(release)
                .fetch_one(&pool)
                .await
                .unwrap()
        };
        assert_eq!(status, expected);
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE platformid=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 5);
    container_event(
        &pool,
        platform,
        None,
        "container-1",
        Some("exited"),
        None,
        now,
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM activityevents WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        count,
        "replayed events must not duplicate activities"
    );
    assert!(
        !container_event(
            &pool,
            platform,
            None,
            "container-1",
            Some("running"),
            None,
            now - 1
        )
        .await
        .unwrap(),
        "old observations cannot restore stale health"
    );
    // A different platform cannot update this runtime identity.
    assert!(
        !container_event(
            &pool,
            Uuid::now_v7(),
            None,
            "container-1",
            Some("running"),
            None,
            now
        )
        .await
        .unwrap()
    );
    container_event(&pool, platform, None, "container-1", None, None, now)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM deployments WHERE id=$1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Degraded"
    );
    // Swarm tasks must not drive Compose stack state.
    sqlx::query(
        "UPDATE containers SET isswarmtask=TRUE WHERE platformid=$1 AND dockercontainerid='web-1'",
    )
    .bind(platform)
    .execute(&pool)
    .await
    .unwrap();
    container_event(&pool, platform, None, "web-1", Some("exited"), None, now)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Healthy"
    );
    sqlx::query("UPDATE containers SET isswarmtask=FALSE,state='Running' WHERE platformid=$1 AND dockercontainerid='web-1'")
        .bind(platform).execute(&pool).await.unwrap();
    // The event cannot complete or fail an Apply whose command still owns its claim.
    sqlx::query("UPDATE stackreleases SET status='Applying' WHERE id=$1")
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    container_event(&pool, platform, None, "web-1", Some("exited"), None, now)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Applying"
    );
    sqlx::query("UPDATE stackreleases SET status='Healthy' WHERE id=$1")
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    // Simulate a dropped destroy event. The full inventory scan must capture bindings before deleting rows.
    inventory.containers.retain(|c| c.id == "web-2");
    inventory.observed_at += chrono::Duration::seconds(2);
    store.persist(&inventory).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Degraded"
    );
    platform_offline(&pool, platform).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Offline"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Degraded"
    );
    // Successful full reconciliation restores the observed runtime state.
    inventory.observed_at += chrono::Duration::seconds(1);
    store.persist(&inventory).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Online"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT state FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Running"
    );
    pool.close().await;
}

// Port of StackSyncJobTests, with additional empty/Created release regressions.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn stack_sync_preserves_active_releases_and_recovers_stale_processing() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    inventory.containers.clear();
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    for (status, control, owner, expected_status, expected_control) in [
        ("Healthy", "Idle", None, "Degraded", "Idle"),
        ("Created", "Idle", None, "Created", "Idle"),
        ("Applying", "Processing", None, "Applying", "Processing"),
        ("Pending", "Processing", None, "Pending", "Processing"),
        ("Degraded", "Processing", None, "Degraded", "Idle"),
        (
            "Healthy",
            "Processing",
            Some(actor),
            "Healthy",
            "Processing",
        ),
        (
            "Stopped",
            "Processing",
            Some(actor),
            "Stopped",
            "Processing",
        ),
        (
            "Degraded",
            "Processing",
            Some(actor),
            "Degraded",
            "Processing",
        ),
    ] {
        let stack = Uuid::now_v7();
        let release = Uuid::now_v7();
        sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate,controlstate,controltriggeredby) VALUES($1,$2,$3,'{}','WebEditor','{}',$4,$5)")
            .bind(stack).bind(stack.to_string()).bind(actor).bind(control).bind(owner).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,'{}',$5,'1')")
            .bind(release).bind(stack).bind(platform).bind(actor).bind(status).execute(&pool).await.unwrap();
        sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
            .bind(stack)
            .bind(release)
            .execute(&pool)
            .await
            .unwrap();
        store.persist(&inventory).await.unwrap();
        let actual: (String,String) = sqlx::query_as("SELECT r.status,s.controlstate FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1")
            .bind(stack).fetch_one(&pool).await.unwrap();
        assert_eq!(
            actual,
            (expected_status.into(), expected_control.into()),
            "{status}/{control}"
        );
    }
    pool.close().await;
}

// PlatformSyncJobTests.DockerPlatformSync_ShouldRejectChangedDaemonIdentity
// and SwarmPlatformSync_ShouldRejectChangedPinnedManagerIdentity.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn inventory_rejects_changed_pinned_runtime_identity_without_overwriting_projection() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    let original = snapshot(platform, true);
    store.persist(&original).await.unwrap();
    for change in ["daemon", "cluster", "node", "manager", "type"] {
        let mut changed = original.clone();
        changed.observed_at += chrono::Duration::seconds(1);
        match change {
            "daemon" => changed.info.daemon_id = "replacement".into(),
            "cluster" => {
                changed.info.swarm.as_mut().unwrap().cluster_id = Some("replacement".into())
            }
            "node" => changed.info.swarm.as_mut().unwrap().node_id = "replacement".into(),
            "manager" => changed.info.swarm.as_mut().unwrap().control_available = false,
            _ => changed.swarm = None,
        }
        assert_eq!(
            store.persist(&changed).await.unwrap_err().kind,
            citadel_platforms::RuntimeErrorKind::Conflict,
            "{change}"
        );
    }
    let daemon: String =
        sqlx::query_scalar("SELECT platformdescriptor->>'daemonId' FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(daemon, original.info.daemon_id);
    pool.close().await;
}

// ReconcilableResourceJobTests: orphan images/actions recover; active work survives.
// CleanupJob retention additionally runs on an idle installation.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn maintenance_recovers_disabled_orphans_and_purges_only_expired_terminal_data() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&snapshot(platform, false))
        .await
        .unwrap();
    sqlx::query("UPDATE images SET controlstate='Processing',controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-120 WHERE platformid=$1").bind(platform).execute(&pool).await.unwrap();
    let action = Uuid::now_v7();
    sqlx::query("INSERT INTO actions(id,name,createdbyactorid,runasactorid,code,enabled,scheduleenabled,scheduletimezone,timeoutseconds,alertonfailure,controlstate) VALUES($1,$2,$3,$3,'',false,false,'UTC',30,false,'Processing')")
        .bind(action).bind(action.to_string()).bind(actor).execute(&pool).await.unwrap();
    let expired = Uuid::now_v7();
    let active = Uuid::now_v7();
    for (id, status) in [(expired, "Succeeded"), (active, "Queued")] {
        sqlx::query("INSERT INTO actionruns(id,actionid,actionname,codehash,codesnapshot,runasactorid,status,timeoutseconds,trigger,finishedat) VALUES($1,$2,'test','hash','',$3,$4,30,'Manual',CURRENT_TIMESTAMP-INTERVAL '91 days')")
            .bind(id).bind(action).bind(actor).bind(status).execute(&pool).await.unwrap();
    }
    citadel_adapters::persistence::postgres::maintenance::reconcile(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT controlstate FROM actions WHERE id=$1")
            .bind(action)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Idle"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM images WHERE platformid=$1 AND controlstate<>'Idle'"
        )
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    citadel_adapters::persistence::postgres::maintenance::cleanup(&pool, Some(90))
        .await
        .unwrap();
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM actionruns WHERE id=$1)")
            .bind(expired)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM actionruns WHERE id=$1)")
            .bind(active)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    pool.close().await;
}

// ContainerSyncJobTests.UpdatedEvent_ShouldRefreshSystemClassification and
// ContainerEventWorkItemConsistencyTests image-reference fallback.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn container_event_refreshes_classification_and_rejects_older_metadata() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let inventory = snapshot(platform, false);
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&inventory)
        .await
        .unwrap();
    let mut observed = inventory.containers[0].clone();
    observed.is_system = true;
    observed.system_role = Some("BackupHelper".into());
    observed.image_id.clear();
    observed.image = "helper:latest".into();
    let now = inventory.observed_at.timestamp() + 2;
    assert!(
        citadel_adapters::persistence::postgres::platforms::status::container_observation(
            &pool, platform, None, &observed, now
        )
        .await
        .unwrap()
    );
    let saved:(bool,String)=sqlx::query_as("SELECT issystem,dockerimageid FROM containers WHERE platformid=$1 AND dockercontainerid=$2")
        .bind(platform).bind(&observed.id).fetch_one(&pool).await.unwrap();
    assert_eq!(saved, (true, "helper:latest".into()));
    observed.is_system = false;
    assert!(
        !citadel_adapters::persistence::postgres::platforms::status::container_observation(
            &pool,
            platform,
            None,
            &observed,
            now - 1
        )
        .await
        .unwrap()
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT issystem FROM containers WHERE platformid=$1 AND dockercontainerid=$2"
        )
        .bind(platform)
        .bind(&observed.id)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    pool.close().await;
}

// Ports ContainerDependentResourceSynchronizerTests and both
// ContainerDestroyedWorkItemTests processing-state scenarios.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn dependent_sync_isolates_failures_and_direct_container_operations_release_stack() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,'{}','Healthy',$4)")
        .bind(deployment).bind(deployment.to_string()).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) VALUES($1,$2,$3,'{}','WebEditor','{}')")
        .bind(stack).bind(stack.to_string()).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,'{}','Healthy','1')")
        .bind(release).bind(stack).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
        .bind(stack)
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    let mut second = inventory.containers[0].clone();
    second.id = "owned-stack".into();
    second.is_swarm_task = false;
    inventory.containers[0].is_swarm_task = false;
    inventory.containers.push(second);
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&inventory).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$2 WHERE platformid=$1 AND dockercontainerid='container-1'").bind(platform).bind(deployment).execute(&pool).await.unwrap();
    sqlx::query(
        "UPDATE containers SET stackid=$2 WHERE platformid=$1 AND dockercontainerid='owned-stack'",
    )
    .bind(platform)
    .bind(stack)
    .execute(&pool)
    .await
    .unwrap();
    // Complete ownership adoption before testing failure isolation.
    store.persist(&inventory).await.unwrap();
    let constraint = format!("reject_stopped_{}", deployment.simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("ALTER TABLE deployments ADD CONSTRAINT {constraint} CHECK (id<>'{deployment}' OR status<>'Stopped')"))).execute(&pool).await.unwrap();
    for container in &mut inventory.containers {
        container.state = "exited".into();
    }
    inventory.observed_at += chrono::Duration::seconds(2);
    store.persist(&inventory).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM deployments WHERE id=$1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Healthy"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Stopped"
    );
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "ALTER TABLE deployments DROP CONSTRAINT {constraint}"
    )))
    .execute(&pool)
    .await
    .unwrap();
    store.persist(&inventory).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM deployments WHERE id=$1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Stopped"
    );
    let mut sibling = inventory.containers[1].clone();
    sibling.id = "sibling".into();
    sibling.state = "running".into();
    inventory.containers[1].state = "running".into();
    inventory.containers.push(sibling);
    inventory.observed_at += chrono::Duration::seconds(2);
    store.persist(&inventory).await.unwrap();
    sqlx::query(
        "UPDATE containers SET stackid=$2 WHERE platformid=$1 AND dockercontainerid='sibling'",
    )
    .bind(platform)
    .bind(stack)
    .execute(&pool)
    .await
    .unwrap();
    for status in ["Healthy", "Applying"] {
        sqlx::query("UPDATE stackreleases SET status=$2 WHERE id=$1")
            .bind(release)
            .bind(status)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE stacks SET controlstate='Processing' WHERE id=$1")
            .bind(stack)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE containers SET controlstate='Processing' WHERE platformid=$1 AND dockercontainerid='owned-stack'").bind(platform).execute(&pool).await.unwrap();
        citadel_adapters::persistence::postgres::platforms::status::container_event(
            &pool,
            platform,
            None,
            "owned-stack",
            Some("exited"),
            None,
            inventory.observed_at.timestamp() + 10,
        )
        .await
        .unwrap();
        let actual:(String,String)=sqlx::query_as("SELECT r.status,s.controlstate FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=$1").bind(stack).fetch_one(&pool).await.unwrap();
        assert_eq!(
            actual,
            if status == "Applying" {
                ("Applying".into(), "Processing".into())
            } else {
                ("Degraded".into(), "Idle".into())
            }
        );
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn startup_recovers_recent_abandoned_runs_but_periodic_maintenance_preserves_live_runs() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let action = Uuid::now_v7();
    let run = Uuid::now_v7();
    let actor = citadel_identity::SYSTEM_ACTOR_ID;
    sqlx::query("INSERT INTO actions(id,name,createdbyactorid,runasactorid,code,enabled,scheduleenabled,scheduletimezone,timeoutseconds,alertonfailure,controlstate,currentrunid) VALUES($1,$2,$3,$3,'',false,false,'UTC',3600,false,'Processing',$4)")
        .bind(action).bind(action.to_string()).bind(actor).bind(run).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO actionruns(id,actionid,actionname,codehash,codesnapshot,runasactorid,status,timeoutseconds,trigger,startedat) VALUES($1,$2,'fixture','hash','',$3,'Running',3600,'Manual',CURRENT_TIMESTAMP)")
        .bind(run).bind(action).bind(actor).execute(&pool).await.unwrap();
    citadel_adapters::persistence::postgres::maintenance::reconcile(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM actionruns WHERE id=$1")
            .bind(run)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Running"
    );
    let mut lease = pool.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(4848495441444547)")
        .execute(&mut *lease)
        .await
        .unwrap();
    citadel_adapters::persistence::postgres::maintenance::recover_on_startup(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM actionruns WHERE id=$1")
            .bind(run)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Failed"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT controlstate FROM actions WHERE id=$1")
            .bind(action)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Idle"
    );
    sqlx::query("SELECT pg_advisory_unlock(4848495441444547)")
        .execute(&mut *lease)
        .await
        .unwrap();
    drop(lease);
    pool.close().await;
}

// Port: ContainerEventWorkItemConsistencyTests. A failed commit must not publish
// work based on uncommitted inventory; successful writes publish after commit.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn inventory_creation_notifications_are_transactional() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    let mut inventory = snapshot(platform, false);
    store.persist(&inventory).await.unwrap();
    let mut listener = sqlx::postgres::PgListener::connect(&url).await.unwrap();
    listener.listen("citadel_container_created").await.unwrap();
    let mut additional = inventory.containers[0].clone();
    additional.id = "uncommitted-container".into();
    additional.name = "reject-this-container".into();
    inventory.containers.push(additional);
    inventory.observed_at += chrono::Duration::seconds(1);
    let constraint = format!("reject_inventory_{}", platform.simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("ALTER TABLE containers ADD CONSTRAINT {constraint} CHECK (platformid<>'{platform}' OR name<>'reject-this-container')"))).execute(&pool).await.unwrap();
    assert!(store.persist(&inventory).await.is_err());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.recv())
            .await
            .is_err()
    );
    assert!(!sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM containers WHERE platformid=$1 AND dockercontainerid='uncommitted-container')").bind(platform).fetch_one(&pool).await.unwrap());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "ALTER TABLE containers DROP CONSTRAINT {constraint}"
    )))
    .execute(&pool)
    .await
    .unwrap();
    store.persist(&inventory).await.unwrap();
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
        .await
        .unwrap()
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(event.payload()).unwrap();
    assert_eq!(payload["platform"], platform.to_string());
    assert_eq!(payload["container"], "uncommitted-container");
    drop(listener);
    pool.close().await;
}

// Port: ReconcilableResourceJobTests orphan-state matrix. Operation-specific
// claims and active runs are covered separately; these have no owning operation.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn maintenance_releases_each_orphan_resource_kind() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let id = Uuid::now_v7();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let statements = [
        "INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,$2,'InternalEncrypted')",
        "INSERT INTO registries(id,name,createdbyactorid,configuration,registryhost,status) VALUES($1,$2,$3,'{}','fixture.invalid','Enabled')",
        "INSERT INTO gitrepositories(id,name,createdbyactorid,defaultbranch,url,status) VALUES($1,$2,$3,'main','https://fixture.invalid/git','Healthy')",
        "INSERT INTO buildagentpools(id,name,normalizedname,createdbyactorid,provider,providerspec) VALUES($1,$2,$2,$3,'DirectAgent','{}')",
        "INSERT INTO buildprojects(id,name,normalizedname,createdbyactorid,gitrepositoryid,registryid,branch,imagerepository,platformid) VALUES($1,$2,$2,$3,$1,$1,'main','fixture/image',$4)",
        "INSERT INTO backuprepositories(id,name,normalizedname,createdbyactorid,passwordsecretid,spec,status,type) VALUES($1,$2,$2,$3,$1,'{}','Ready','Local')",
        "INSERT INTO backuppolicies(id,name,normalizedname,createdbyactorid,runasactorid,backuprepositoryid,source) VALUES($1,$2,$2,$3,$3,$1,'{}')",
        "INSERT INTO deployments(id,name,createdbyactorid,platformid,spec,status) VALUES($1,$2,$3,$4,'{}','Healthy')",
        "INSERT INTO containers(id,name,platformid,dockercontainerid,dockerimageid,state,created,updated,ports) VALUES($1,$2,$4,$2,'image','Running',0,0,'{}')",
        "INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) VALUES($1,$2,$3,'{}','WebEditor','{}')",
        "INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$1,$4,$3,'{}','Healthy','1')",
    ];
    for statement in statements {
        // Each fixture statement uses up to four positional parameters. Bind
        // unused lower positions with types fixed by a harmless CTE.
        let sql = format!("WITH args AS (SELECT $1::uuid,$2::text,$3::uuid,$4::uuid) {statement}");
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .bind(id.to_string())
            .bind(actor)
            .bind(platform)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let kinds = [
        "deployments",
        "containers",
        "stacks",
        "gitrepositories",
        "buildagentpools",
        "buildprojects",
        "backuprepositories",
        "backuppolicies",
    ];
    for table in kinds {
        let sql = format!(
            "UPDATE {table} SET controlstate='Processing',controlstartedat=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-7200 WHERE id=$1"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    citadel_adapters::persistence::postgres::maintenance::reconcile(&pool)
        .await
        .unwrap();
    for table in kinds {
        let sql = format!("SELECT controlstate,controlstartedat FROM {table} WHERE id=$1");
        let actual: (String, Option<i64>) = sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(actual, ("Idle".into(), None), "{table}");
    }
    for (table, status) in [
        ("deployments", "Unknown"),
        ("stackreleases", "Unknown"),
        ("gitrepositories", "Degraded"),
    ] {
        let actual: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT status FROM {table} WHERE id=$1"
        )))
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(actual, status, "{table}");
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn swarm_stack_observations_recover_complete_releases_and_preserve_incomplete_metadata() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,currentstackreleaseid) VALUES($1,$2,$3,'WebEditor','{}','{}',$4)")
        .bind(stack).bind(format!("stack-{stack}")).bind(actor).bind(release).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,'{}',$4,'TimedOut','1')")
        .bind(release).bind(actor).bind(platform).bind(stack).execute(&pool).await.unwrap();
    sqlx::query(
        "UPDATE stackreleases SET spec=jsonb_build_object('projectName','fixture') WHERE id=$1",
    )
    .bind(release)
    .execute(&pool)
    .await
    .unwrap();
    let mut snapshot = snapshot(platform, true);
    let swarm = snapshot.swarm.as_mut().unwrap();
    let service = &mut swarm.services[0];
    service.labels = BTreeMap::from([
        ("com.citadel.managed".into(), "true".into()),
        ("com.citadel.stack-id".into(), stack.to_string()),
        ("com.citadel.release-id".into(), release.to_string()),
        ("com.citadel.stack-service-count".into(), "2".into()),
        ("com.docker.stack.namespace".into(), "fixture".into()),
    ]);
    *service = service.clone().normalize_ownership();
    swarm.configs[0]
        .labels
        .insert("com.docker.stack.namespace".into(), "fixture".into());
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    async fn status(pool: &sqlx::PgPool, release: Uuid) -> String {
        sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
            .bind(release)
            .fetch_one(pool)
            .await
            .unwrap()
    }
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        status(&pool, release).await,
        "TimedOut",
        "partial service set is not proof of convergence"
    );
    let mut second = snapshot.swarm.as_ref().unwrap().services[0].clone();
    second.id = "second".into();
    snapshot.swarm.as_mut().unwrap().services.push(second);
    snapshot.observed_at = Utc::now();
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        status(&pool, release).await,
        "TimedOut",
        "missing immutable config metadata must keep recovery pending"
    );
    sqlx::query("INSERT INTO stackreleaseswarmresources(id,composeresourcename,dockerresourceid,dockerresourcename,kind,mounts,platformid,stackreleaseid) VALUES($1,'config','config-1','config','Config','[{\"target\":\"/config\"}]',$2,$3)")
        .bind(Uuid::now_v7()).bind(platform).bind(release).execute(&pool).await.unwrap();
    snapshot.observed_at = Utc::now();
    store.persist(&snapshot).await.unwrap();
    assert_eq!(status(&pool, release).await, "Healthy");
    // Recovery must not accept a missing referenced resource or resolve a moving
    // Git branch in place of the immutable commit captured before dispatch.
    sqlx::query("UPDATE stackreleases SET status='TimedOut' WHERE id=$1")
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    let configs = std::mem::take(&mut snapshot.swarm.as_mut().unwrap().configs);
    snapshot.observed_at = Utc::now();
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        status(&pool, release).await,
        "TimedOut",
        "missing referenced Config is incomplete evidence"
    );
    snapshot.swarm.as_mut().unwrap().configs = configs;
    let git_update = citadel_stacks::StackUpdateState::Git {
        recreate_stack_on_new_image_state: Default::default(),
        recreate_stack_on_new_commit_state: citadel_stacks::RecreateStackOnNewCommitState {
            current_commit_sha: "previous".into(),
            remote_commit_sha: Some("moving-branch-head".into()),
            last_checked_at: Utc::now(),
        },
    };
    sqlx::query("UPDATE stacks SET stacksource='Git',stackupdatestate=$2 WHERE id=$1")
        .bind(stack)
        .bind(git_update.to_storage_value().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    snapshot.observed_at = Utc::now();
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        status(&pool, release).await,
        "TimedOut",
        "Git recovery requires its captured commit"
    );
    sqlx::query("UPDATE stackreleases SET source=$2 WHERE id=$1")
        .bind(release)
        .bind(serde_json::json!({"ResolvedCommitSha":"captured-immutable-commit"}))
        .execute(&pool)
        .await
        .unwrap();
    snapshot.observed_at = Utc::now();
    store.persist(&snapshot).await.unwrap();
    assert_eq!(status(&pool, release).await, "Healthy");
    let update: serde_json::Value =
        sqlx::query_scalar("SELECT stackupdatestate::jsonb FROM stacks WHERE id=$1")
            .bind(stack)
            .fetch_one(&pool)
            .await
            .unwrap();
    let update = citadel_stacks::StackUpdateState::from_storage_value(update).unwrap();
    let citadel_stacks::StackUpdateState::Git {
        recreate_stack_on_new_commit_state,
        ..
    } = update
    else {
        panic!("expected Git update state")
    };
    assert_eq!(
        recreate_stack_on_new_commit_state.current_commit_sha,
        "captured-immutable-commit"
    );
    assert!(
        recreate_stack_on_new_commit_state
            .remote_commit_sha
            .is_none()
    );
    snapshot.swarm.as_mut().unwrap().services[0].running_task_count = 0;
    snapshot.observed_at = Utc::now();
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        status(&pool, release).await,
        "Degraded",
        "idle Stack tracks a stopped owned service"
    );
    sqlx::query("UPDATE stacks SET controlstate='Processing' WHERE id=$1")
        .bind(stack)
        .execute(&pool)
        .await
        .unwrap();
    snapshot.swarm.as_mut().unwrap().services[0].running_task_count = 1;
    snapshot.observed_at = Utc::now();
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        status(&pool, release).await,
        "Degraded",
        "active operation owns its release status"
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn failed_swarm_refresh_marks_previous_rows_stale_without_clobbering_a_newer_refresh() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let mut snapshot = snapshot(platform, true);
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&snapshot).await.unwrap();
    store
        .mark_swarm_stale(platform, snapshot.observed_at)
        .await
        .unwrap();
    let rows: Vec<bool> =
        sqlx::query_scalar("SELECT isstale FROM swarmserviceprojections WHERE platformid=$1")
            .bind(platform)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [true],
        "failed refresh retains inventory for diagnostics"
    );
    let failed_started = snapshot.observed_at;
    snapshot.observed_at += chrono::Duration::seconds(1);
    store.persist(&snapshot).await.unwrap();
    store
        .mark_swarm_stale(platform, failed_started)
        .await
        .unwrap();
    let rows: Vec<bool> =
        sqlx::query_scalar("SELECT isstale FROM swarmserviceprojections WHERE platformid=$1")
            .bind(platform)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [false],
        "failure cannot mark a newer successful refresh stale"
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn swarm_node_infrastructure_revokes_missing_service_bootstrap_and_only_old_absent_bindings()
{
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let mut snapshot = snapshot(platform, true);
    let cluster = snapshot
        .info
        .swarm
        .as_ref()
        .unwrap()
        .cluster_id
        .clone()
        .unwrap();
    snapshot
        .swarm
        .as_mut()
        .unwrap()
        .nodes
        .push(RuntimeSwarmNode {
            id: "worker".into(),
            status: "ready".into(),
            availability: "active".into(),
            operating_system: "linux".into(),
            architecture: "x86_64".into(),
            ..Default::default()
        });
    sqlx::query("INSERT INTO swarmnodeagentinstallations(platformid,agentimagedigest,agentimagereference,clusterid,desiredstate,dockerserviceid,dockerservicename,managerdockerdaemonid,managerdockernodeid) VALUES($1,'sha256:agent','agent:latest',$2,'Installed','missing-agent-service','citadel-agent','daemon','node-1')")
        .bind(platform).bind(&cluster).execute(&pool).await.unwrap();
    let bootstrap = Uuid::now_v7();
    sqlx::query("INSERT INTO swarmnodeagentbootstraps(id,clusterid,createdbyactorid,dockersecretname,expiresatutc,platformid,tokenhash,version) VALUES($1,$2,$3,'bootstrap',now()+interval '1 hour',$4,'test-hash',1)")
        .bind(bootstrap).bind(&cluster).bind(actor).bind(platform).execute(&pool).await.unwrap();
    for (node, profile, recent) in [
        ("worker", "SwarmNode", false),
        ("absent", "SwarmNode", false),
        ("recent", "SwarmNode", true),
        ("ordinary", "Ordinary", false),
    ] {
        sqlx::query("INSERT INTO edgeagentbindings(id,agentfingerprint,agentid,agentpublickey,connectionstatus,platformid,resourceid,profile,dockernodeid,updatedatutc,lastheartbeatatutc) VALUES($1,$1::text,$2,'fixture','Disconnected',$3,$3,$4,$5,now()-interval '20 minutes',CASE WHEN $6 THEN now() ELSE now()-interval '20 minutes' END)")
            .bind(Uuid::now_v7()).bind(Uuid::now_v7()).bind(platform).bind(profile).bind(node).bind(recent).execute(&pool).await.unwrap();
    }
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&snapshot)
        .await
        .unwrap();
    let revoked: bool = sqlx::query_scalar(
        "SELECT revokedatutc IS NOT NULL FROM swarmnodeagentbootstraps WHERE id=$1",
    )
    .bind(bootstrap)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(revoked);
    let revoked:Vec<String>=sqlx::query_scalar("SELECT dockernodeid FROM edgeagentbindings WHERE platformid=$1 AND revokedatutc IS NOT NULL ORDER BY dockernodeid").bind(platform).fetch_all(&pool).await.unwrap();
    assert_eq!(revoked, ["absent"]);
    pool.close().await;
}

#[tokio::test]
async fn inventory_initialization_serializes_one_platform_without_blocking_other_platforms() {
    use std::{sync::Arc, time::Duration};
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let service = citadel_platforms::PlatformReadService::new(Arc::new(
        citadel_adapters::persistence::postgres::platforms::PostgresPlatformReader::new(pool),
    ));
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    let held = service.inventory_guard(first).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(30), service.inventory_guard(first))
            .await
            .is_err(),
        "same-platform readers coalesce behind the active refresh"
    );
    let other = tokio::time::timeout(Duration::from_millis(100), service.inventory_guard(second))
        .await
        .expect("one stalled platform cannot serialize independent platforms");
    drop(held);
    tokio::time::timeout(Duration::from_millis(100), service.inventory_guard(first))
        .await
        .unwrap();
    drop(other);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn orphaned_stack_labels_do_not_abort_the_swarm_inventory_transaction() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let mut snapshot = snapshot(platform, true);
    let service = &mut snapshot.swarm.as_mut().unwrap().services[0];
    service.labels = BTreeMap::from([
        ("com.citadel.managed".into(), "true".into()),
        ("com.citadel.stack-id".into(), Uuid::now_v7().to_string()),
        ("com.citadel.release-id".into(), Uuid::now_v7().to_string()),
        ("com.docker.stack.namespace".into(), "orphan".into()),
    ]);
    *service = service.clone().normalize_ownership();
    let service_id = service.id.clone();
    let mut conflicting = service.clone();
    conflicting.id = "conflicting-owner".into();
    conflicting
        .labels
        .insert("com.citadel.service-id".into(), Uuid::now_v7().to_string());
    snapshot
        .swarm
        .as_mut()
        .unwrap()
        .services
        .push(conflicting.normalize_ownership());

    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&snapshot)
        .await
        .unwrap();
    let row: (Option<Uuid>, String, Option<String>) = sqlx::query_as("SELECT stackid,ownership,ownershipdiagnostic FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid=$2").bind(platform).bind(service_id).fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, None);
    assert_eq!(row.1, "DockerStackExternal");
    assert!(row.2.unwrap().contains("no longer exists"));
    let conflict: String = sqlx::query_scalar("SELECT ownership FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid='conflicting-owner'").bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(
        conflict, "OwnershipConflict",
        "orphan normalization must preserve conflicting ownership diagnostics"
    );
    let nodes: i64 =
        sqlx::query_scalar("SELECT count(*) FROM swarmnodeprojections WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(nodes, snapshot.swarm.as_ref().unwrap().nodes.len() as i64);
    pool.close().await;
}

// A valid owner ID is insufficient when the
// namespace differs; an explicit import association takes precedence over labels.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn stack_namespace_claims_are_checked_and_imported_associations_are_preserved() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,currentstackreleaseid) VALUES($1,'Expected Namespace',$2,'WebEditor','{}','{}',$3)")
        .bind(stack).bind(actor).bind(release).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,'{}',$4,'TimedOut','1')")
        .bind(release).bind(actor).bind(platform).bind(stack).execute(&pool).await.unwrap();
    let mut snapshot = snapshot(platform, true);
    let service = &mut snapshot.swarm.as_mut().unwrap().services[0];
    service.labels = BTreeMap::from([
        ("com.citadel.managed".into(), "true".into()),
        ("com.citadel.stack-id".into(), stack.to_string()),
        ("com.citadel.release-id".into(), release.to_string()),
        ("com.citadel.stack-service-count".into(), "1".into()),
        ("com.docker.stack.namespace".into(), "wrong".into()),
    ]);
    *service = service.clone().normalize_ownership();
    // Everything except ownership is sufficient to recover Healthy. This
    // ensures the assertion below actually exercises the namespace fence.
    service.config_ids.clear();
    service.secret_ids.clear();
    service.running_task_count = 1;
    service.desired_task_count = 1;
    service.update_state = "Completed".into();
    let id = service.id.clone();
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    let read = || {
        sqlx::query_as::<_, (Option<Uuid>, String, Option<String>)>(
        "SELECT stackid,ownership,ownershipdiagnostic FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid=$2")
        .bind(platform).bind(&id)
    };
    store.persist(&snapshot).await.unwrap();
    let row = read().fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, None);
    assert_eq!(row.1, "OwnershipConflict");
    assert!(row.2.unwrap().contains("namespace"));
    let status: String = sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
        .bind(release)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        status, "TimedOut",
        "untrusted namespace cannot recover the release"
    );
    // Fallback to normalized Stack name must agree with the deployment resolver.
    let service = &mut snapshot.swarm.as_mut().unwrap().services[0];
    service.labels.insert(
        "com.docker.stack.namespace".into(),
        "expected-namespace".into(),
    );
    *service = service.clone().normalize_ownership();
    snapshot.observed_at += chrono::Duration::seconds(1);
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        read().fetch_one(&pool).await.unwrap(),
        (Some(stack), "CitadelStack".into(), None)
    );
    // Simulate an imported association facing a stale foreign Docker owner label.
    let service = &mut snapshot.swarm.as_mut().unwrap().services[0];
    service
        .labels
        .insert("com.citadel.stack-id".into(), Uuid::now_v7().to_string());
    *service = service.clone().normalize_ownership();
    snapshot.observed_at += chrono::Duration::seconds(1);
    store.persist(&snapshot).await.unwrap();
    assert_eq!(
        read().fetch_one(&pool).await.unwrap(),
        (Some(stack), "CitadelStack".into(), None)
    );
    // A new Docker identity must use the explicit project name when supplied.
    sqlx::query(
        "UPDATE stackreleases SET spec=jsonb_build_object('projectName','custom') WHERE id=$1",
    )
    .bind(release)
    .execute(&pool)
    .await
    .unwrap();
    let service = &mut snapshot.swarm.as_mut().unwrap().services[0];
    service.id = "new-namespace-claim".into();
    service
        .labels
        .insert("com.citadel.stack-id".into(), stack.to_string());
    *service = service.clone().normalize_ownership();
    snapshot.observed_at += chrono::Duration::seconds(1);
    store.persist(&snapshot).await.unwrap();
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT stackid FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid='new-namespace-claim'")
        .bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(owner, None);
    let service = &mut snapshot.swarm.as_mut().unwrap().services[0];
    service
        .labels
        .insert("com.docker.stack.namespace".into(), "custom".into());
    *service = service.clone().normalize_ownership();
    snapshot.observed_at += chrono::Duration::seconds(1);
    store.persist(&snapshot).await.unwrap();
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT stackid FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid='new-namespace-claim'")
        .bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(owner, Some(stack));
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn stack_inventory_skips_busy_resources_and_reconciles_on_the_next_sweep() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) VALUES($1,$2,$3,'{}','WebEditor','{}')")
        .bind(stack).bind(stack.to_string()).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,'{}','Healthy','1')")
        .bind(release).bind(stack).bind(platform).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$2 WHERE id=$1")
        .bind(stack)
        .bind(release)
        .execute(&pool)
        .await
        .unwrap();
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    inventory.containers[0].is_swarm_task = false;
    inventory.containers[0]
        .labels
        .insert("com.citadel.managed".into(), "true".into());
    inventory.containers[0]
        .labels
        .insert("com.citadel.stack-id".into(), stack.to_string());
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&inventory).await.unwrap();
    let mut operation = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM stacks WHERE id=$1 FOR NO KEY UPDATE")
        .bind(stack)
        .fetch_one(&mut *operation)
        .await
        .unwrap();
    inventory.containers[0].state = "exited".into();
    let mut replacement = inventory.containers[0].clone();
    replacement.id = "new-stack-container".into();
    inventory.containers.push(replacement);
    inventory.observed_at += chrono::Duration::seconds(2);
    tokio::time::timeout(std::time::Duration::from_secs(5), store.persist(&inventory))
        .await
        .unwrap()
        .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM containers WHERE stackid=$1 AND state='Exited'")
            .bind(stack)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 2,
        "observations must commit while their resource is busy"
    );
    let status: String = sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
        .bind(release)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Healthy");
    operation.commit().await.unwrap();
    store.persist(&inventory).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
        .bind(release)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Stopped");
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn event_resource_writes_preserve_unrelated_projections_and_swarm_identity() {
    use citadel_platforms::jobs::{ResourceInventory, ResourceSnapshot};
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let baseline = snapshot(platform, true);
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&baseline).await.unwrap();
    let read_counts = || async {
        sqlx::query_as::<_, (i64,i64,i64)>("SELECT (SELECT count(*) FROM containers WHERE platformid=$1),(SELECT count(*) FROM images WHERE platformid=$1),(SELECT count(*) FROM swarmserviceprojections WHERE platformid=$1 AND NOT isstale)").bind(platform).fetch_one(&pool).await.unwrap()
    };
    assert_eq!(read_counts().await, (1, 2, 1));
    // Platform metadata and post-mutation Swarm observations cannot replace
    // Container/Image sets or their independently maintained counts.
    let before: (i32, i32, i32) =
        sqlx::query_as("SELECT imagecount,networkcount,volumecount FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    let mut info = baseline.info.clone();
    info.cpu_count = 8;
    info.container_count = 999;
    store
        .persist_resource(&ResourceSnapshot {
            platform_id: platform,
            observed_at: Utc::now(),
            inventory: ResourceInventory::Platform(Box::new(info.clone())),
        })
        .await
        .unwrap();
    assert_eq!(read_counts().await, (1, 2, 1));
    let after: (i32, i32, i32) =
        sqlx::query_as("SELECT imagecount,networkcount,volumecount FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    info.daemon_id = "wrong-daemon".into();
    assert!(
        store
            .persist_resource(&ResourceSnapshot {
                platform_id: platform,
                observed_at: Utc::now(),
                inventory: ResourceInventory::Platform(Box::new(info))
            })
            .await
            .is_err()
    );
    let mut narrow = baseline.clone();
    narrow.containers.clear();
    narrow.images.clear();
    narrow.volumes.clear();
    citadel_adapters::persistence::postgres::platforms::swarm::inventory::refresh(&pool, &narrow)
        .await
        .unwrap();
    assert_eq!(read_counts().await, (1, 2, 1));
    let cpu: i32 = sqlx::query_scalar("SELECT cpucount FROM platforms WHERE id=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        cpu, 8,
        "Swarm refresh cannot overwrite newer Platform metadata"
    );
    sqlx::query("UPDATE platforms SET prunehistoricalswarmtaskcontainers=true WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let mut historical = baseline.containers.clone();
    historical[0].is_swarm_task = true;
    historical[0].state = "exited".into();
    store
        .persist_resource(&ResourceSnapshot {
            platform_id: platform,
            observed_at: Utc::now(),
            inventory: ResourceInventory::Containers(historical),
        })
        .await
        .unwrap();
    assert_eq!(
        read_counts().await,
        (1, 2, 1),
        "historical tasks must remain discoverable by the pruning worker"
    );
    sqlx::query("UPDATE containers SET imageid=NULL WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    store
        .persist_resource(&ResourceSnapshot {
            platform_id: platform,
            observed_at: Utc::now(),
            inventory: ResourceInventory::Images(baseline.images.clone()),
        })
        .await
        .unwrap();
    let linked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM containers WHERE platformid=$1 AND imageid IS NOT NULL",
    )
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        linked, 1,
        "image refresh repairs links without container enumeration"
    );
    let mut refresh = ResourceSnapshot {
        platform_id: platform,
        observed_at: Utc::now(),
        inventory: ResourceInventory::Containers(vec![]),
    };
    store.persist_resource(&refresh).await.unwrap();
    assert_eq!(
        read_counts().await,
        (0, 2, 1),
        "container absence cannot delete images or Swarm projections"
    );
    refresh.inventory = ResourceInventory::Images(baseline.images[..1].to_vec());
    store.persist_resource(&refresh).await.unwrap();
    assert_eq!(read_counts().await, (0, 1, 1));
    refresh.inventory = ResourceInventory::Networks(vec![]);
    store.persist_resource(&refresh).await.unwrap();
    let counts: (i32, i32, i32) =
        sqlx::query_as("SELECT imagecount,networkcount,volumecount FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(counts, (1, 0, 1));
    refresh.inventory = ResourceInventory::Volumes(vec![]);
    store.persist_resource(&refresh).await.unwrap();
    let mut swarm = baseline.swarm.clone().unwrap();
    swarm.nodes[0].id = "wrong-manager".into();
    refresh.inventory = ResourceInventory::Swarm {
        inventory: swarm,
        networks: baseline.networks.clone(),
    };
    assert!(store.persist_resource(&refresh).await.is_err());
    assert_eq!(
        read_counts().await,
        (0, 1, 1),
        "invalid identity must not stale the accepted projections"
    );
    refresh.inventory = ResourceInventory::Swarm {
        inventory: baseline.swarm.unwrap(),
        networks: baseline.networks,
    };
    store.persist_resource(&refresh).await.unwrap();
    assert_eq!(read_counts().await, (0, 1, 1));
    let counts: (i32, i32, i32) =
        sqlx::query_as("SELECT imagecount,networkcount,volumecount FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        counts,
        (1, 0, 0),
        "Swarm refresh cannot overwrite unrelated platform metadata"
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn scoped_generations_reject_same_timestamp_updates_and_resurrection_without_cross_scope_retries()
 {
    use citadel_platforms::jobs::{
        ProjectionKind, ResourceInventory, ResourceSnapshot, SnapshotGeneration,
    };
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let baseline = snapshot(platform, true);
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&baseline).await.unwrap();
    let containers = ResourceSnapshot {
        platform_id: platform,
        observed_at: baseline.observed_at,
        inventory: ResourceInventory::Containers(baseline.containers.clone()),
    };
    let images = ResourceSnapshot {
        platform_id: platform,
        observed_at: baseline.observed_at,
        inventory: ResourceInventory::Images(baseline.images.clone()),
    };
    let capture_containers =
        || SnapshotGeneration::capture(platform, None, ProjectionKind::Containers);
    let capture_images = || SnapshotGeneration::capture(platform, None, ProjectionKind::Images);
    let old_containers = capture_containers().await;
    let old_images = capture_images().await;
    // The exact same persisted second cannot establish ordering; the causal fence can.
    citadel_adapters::persistence::postgres::platforms::status::container_event(
        &pool,
        platform,
        None,
        &baseline.containers[0].id,
        Some("exited"),
        None,
        baseline.observed_at.timestamp(),
    )
    .await
    .unwrap();
    assert!(
        !store
            .persist_resource_checked(&containers, Some(&old_containers))
            .await
            .unwrap()
    );
    assert!(
        store
            .persist_resource_checked(&images, Some(&old_images))
            .await
            .unwrap(),
        "container observation must not invalidate an image read"
    );
    let state: String = sqlx::query_scalar("SELECT state FROM containers WHERE platformid=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "Exited");
    let before_delete = capture_containers().await;
    citadel_adapters::persistence::postgres::platforms::status::container_event(
        &pool,
        platform,
        None,
        &baseline.containers[0].id,
        None,
        None,
        baseline.observed_at.timestamp(),
    )
    .await
    .unwrap();
    assert!(
        !store
            .persist_resource_checked(&containers, Some(&before_delete))
            .await
            .unwrap(),
        "an old complete set cannot resurrect a tombstone"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM containers WHERE platformid=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let current_containers = capture_containers().await;
    let old_images = capture_images().await;
    store
        .persist_resource(&ResourceSnapshot {
            inventory: ResourceInventory::Images(vec![]),
            ..images.clone()
        })
        .await
        .unwrap();
    assert!(
        !store
            .persist_resource_checked(&images, Some(&old_images))
            .await
            .unwrap()
    );
    assert!(
        store
            .persist_resource_checked(
                &ResourceSnapshot {
                    inventory: ResourceInventory::Containers(vec![]),
                    ..containers
                },
                Some(&current_containers)
            )
            .await
            .unwrap(),
        "image writes must not invalidate container reads"
    );
    let current_images = capture_images().await;
    let mut invalid = baseline.images[0].clone();
    invalid.id.clear();
    // A transaction that fails after entering the writer must not advance the fence.
    let mut invalid_images = images.clone();
    invalid_images.inventory = ResourceInventory::Images(vec![invalid.clone(), invalid]);
    assert!(store.persist_resource(&invalid_images).await.is_err());
    assert!(
        store
            .persist_resource_checked(&images, Some(&current_images))
            .await
            .unwrap()
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn semantic_container_replays_advance_fences_without_revisions_or_notifications() {
    use citadel_adapters::persistence::postgres::platforms::status::{
        container_event_committed, container_observation_committed,
    };
    use citadel_platforms::jobs::{ProjectionChange, ResourceInventory, ResourceSnapshot};
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let mut inventory = snapshot(platform, false);
    inventory.swarm = None;
    inventory.info.swarm = None;
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&inventory).await.unwrap();
    let mut listener = sqlx::postgres::PgListener::connect(&url).await.unwrap();
    listener.listen("citadel_swarm_prune").await.unwrap();
    listener.listen("citadel_stack_drift").await.unwrap();
    let mut container = inventory.containers[0].clone();
    container.state = "exited".into();
    container.is_swarm_task = true;
    let observed = inventory.observed_at.timestamp() + 10;
    let before: i64 = sqlx::query_scalar(
        "SELECT rowversion FROM containers WHERE platformid=$1 AND dockercontainerid=$2",
    )
    .bind(platform)
    .bind(&container.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        container_observation_committed(&pool, platform, None, &container, observed)
            .await
            .unwrap(),
        ProjectionChange::Changed
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
            .await
            .unwrap()
            .unwrap()
            .channel(),
        "citadel_swarm_prune"
    );
    let after: (i64, i64) = sqlx::query_as(
        "SELECT rowversion,updated FROM containers WHERE platformid=$1 AND dockercontainerid=$2",
    )
    .bind(platform)
    .bind(&container.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        after,
        (before + 1, observed),
        "one authoritative metadata/state write"
    );
    let activities: i64 =
        sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        container_observation_committed(&pool, platform, None, &container, observed + 1)
            .await
            .unwrap(),
        ProjectionChange::Unchanged
    );
    assert_eq!(
        container_event_committed(
            &pool,
            platform,
            None,
            &container.id,
            Some("exited"),
            None,
            observed + 2
        )
        .await
        .unwrap(),
        ProjectionChange::Unchanged
    );
    inventory.containers[0] = container.clone();
    assert_eq!(
        store
            .persist_resource_committed(
                &ResourceSnapshot {
                    platform_id: platform,
                    observed_at: chrono::DateTime::from_timestamp(observed + 3, 0).unwrap(),
                    inventory: ResourceInventory::Containers(inventory.containers.clone()),
                },
                None
            )
            .await
            .unwrap(),
        ProjectionChange::Unchanged
    );
    container.state = "running".into();
    assert_eq!(
        container_observation_committed(&pool, platform, None, &container, observed + 2)
            .await
            .unwrap(),
        ProjectionChange::Unchanged
    );
    let saved: (i64,i64,i64,String) = sqlx::query_as("SELECT rowversion,updated,projectionobservedat,state FROM containers WHERE platformid=$1 AND dockercontainerid=$2")
        .bind(platform).bind(&container.id).fetch_one(&pool).await.unwrap();
    assert_eq!(saved, (after.0, after.1, observed + 3, "Exited".into()));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM activityevents WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        activities
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(150), listener.recv())
            .await
            .is_err(),
        "no repeated drift/prune notification"
    );
    assert_eq!(
        store
            .persist_resource_committed(
                &ResourceSnapshot {
                    platform_id: platform,
                    observed_at: Utc::now(),
                    inventory: ResourceInventory::Images(inventory.images.clone()),
                },
                None
            )
            .await
            .unwrap(),
        ProjectionChange::Unchanged
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn runtime_identity_index_follows_commits_rebuilds_and_repairs_stale_hints() {
    use citadel_adapters::persistence::postgres::platforms::{
        runtime_index::RuntimeIdentityIndex, status::container_event_committed,
    };
    use citadel_platforms::jobs::{
        ProjectionKind, ProjectionWrite, ResourceInventory, ResourceSnapshot, SnapshotGeneration,
    };
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let baseline = snapshot(platform, true);
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&baseline).await.unwrap();
    let index = RuntimeIdentityIndex::attach(pool.clone());
    assert!(!index.initialized());
    assert!(index.rebuild().await.unwrap());
    let docker = &baseline.containers[0].id;
    let old = index.lookup(platform, None, docker).unwrap();
    let expected: Uuid = sqlx::query_scalar(
        "SELECT id FROM containers WHERE platformid=$1 AND dockercontainerid=$2",
    )
    .bind(platform)
    .bind(docker)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(old, expected);
    let stamp = SnapshotGeneration::capture(platform, None, ProjectionKind::Containers).await;
    let invalid = ResourceSnapshot {
        platform_id: platform,
        observed_at: baseline.observed_at,
        inventory: ResourceInventory::Containers(vec![
            baseline.containers[0].clone(),
            baseline.containers[0].clone(),
        ]),
    };
    assert!(store.persist_resource(&invalid).await.is_err());
    assert_eq!(index.lookup(platform, None, docker), Some(old));
    assert!(
        stamp.matches(&ProjectionWrite::begin(platform, None, ProjectionKind::Containers).await)
    );

    // Simulate another process replacing an identity. The stale UUID still exists,
    // but belongs to a different Docker identity: SQL must not update that row.
    sqlx::query("UPDATE containers SET dockercontainerid='old-identity' WHERE id=$1")
        .bind(old)
        .execute(&pool)
        .await
        .unwrap();
    let replacement = Uuid::now_v7();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,updated,created,ports) VALUES($1,$2,$3,'image','replacement','Running',0,0,'[]')")
        .bind(replacement).bind(platform).bind(docker).execute(&pool).await.unwrap();
    container_event_committed(
        &pool,
        platform,
        None,
        docker,
        Some("exited"),
        None,
        baseline.observed_at.timestamp() + 1,
    )
    .await
    .unwrap();
    assert_eq!(index.lookup(platform, None, docker), Some(replacement));
    let old_state: String = sqlx::query_scalar("SELECT state FROM containers WHERE id=$1")
        .bind(old)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(old_state, "Running");
    container_event_committed(
        &pool,
        platform,
        None,
        docker,
        None,
        None,
        baseline.observed_at.timestamp() + 2,
    )
    .await
    .unwrap();
    assert_eq!(index.lookup(platform, None, docker), None);
    store
        .persist_resource(&ResourceSnapshot {
            platform_id: platform,
            observed_at: baseline.observed_at + chrono::Duration::seconds(3),
            inventory: ResourceInventory::Containers(vec![]),
        })
        .await
        .unwrap();
    assert_eq!(index.lookup(platform, None, "old-identity"), None);
    drop(index);
    let restarted = RuntimeIdentityIndex::attach(pool.clone());
    assert!(!restarted.initialized());
    assert!(restarted.rebuild().await.unwrap());
    assert_eq!(restarted.lookup(platform, None, docker), None);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn six_container_verification_is_sequential_and_keeps_every_authoritative_result() {
    use axum::{Json, Router, response::IntoResponse};
    use citadel_adapters::connectors::{
        docker::DockerClient, edge::EdgeRegistry, routing::containers::ContainerRuntimeRouter,
    };
    use citadel_platforms::containers::{ContainerMutationRuntime, ContainerTarget};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let docker = DockerClient::with_endpoint(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
        Duration::from_secs(3),
        "/host",
    )
    .unwrap();
    let router=Router::new().fallback({let active=active.clone();let peak=peak.clone();let calls=calls.clone();move |req:axum::extract::Request|{let active=active.clone();let peak=peak.clone();let calls=calls.clone();async move {
        if req.uri().path()=="/version" {return Json(json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})).into_response();}
        assert!(req.uri().path().ends_with("/json"));calls.fetch_add(1,Ordering::SeqCst);let current=active.fetch_add(1,Ordering::SeqCst)+1;peak.fetch_max(current,Ordering::SeqCst);tokio::time::sleep(Duration::from_millis(15)).await;active.fetch_sub(1,Ordering::SeqCst);Json(json!({"Id":req.uri().path().split('/').nth(3).unwrap(),"State":{"Status":"running"}})).into_response()
    }}});
    let cancel = tokio_util::sync::CancellationToken::new();
    let stop = cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(stop.cancelled_owned())
            .await
            .unwrap();
    });
    let runtime = ContainerRuntimeRouter::new(pool.clone(), docker, None, EdgeRegistry::default());
    let targets: Vec<_> = (0..6)
        .map(|i| ContainerTarget {
            id: Uuid::now_v7(),
            platform_id: platform,
            docker_id: format!("c{i}"),
            node_id: None,
        })
        .collect();
    let result = runtime.observe_batch(&targets, &cancel).await;
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.observed.len(), 6);
    assert!(
        result
            .observed
            .iter()
            .all(|o| o.state.as_deref() == Some("running"))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 6);
    assert_eq!(peak.load(Ordering::SeqCst), 1);
    cancel.cancel();
    server.await.unwrap();
    pool.close().await;
}
