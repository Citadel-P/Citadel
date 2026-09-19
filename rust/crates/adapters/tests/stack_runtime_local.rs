#![cfg(unix)]

use std::time::Duration;

use citadel_adapters::docker::DockerClient;
use citadel_adapters::stack_runtime::StackRuntimeRouter;
use citadel_database::MigrationRunner;
use citadel_stacks::{
    StackApplySource, StackDeletionClaim, StackOperationClaim, StackRuntime, StackSourceFile,
    StackSpec, StackSpecCommon, StackUpdateBehavior, create_ownership_labels_override,
};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL, CITADEL_PHASE6_RUNTIME_IMAGE, Docker CLI, Compose, and a Unix socket"]
async fn local_compose_stack_apply_creates_owned_runtime_and_cleans_it() {
    let fixture = Fixture::new("Docker").await;
    let source = fixture.source();
    let applied = fixture
        .runtime
        .apply(&fixture.claim(), &source, &[], &fixture.cancellation, None)
        .await
        .unwrap();
    assert_eq!(
        applied.status,
        citadel_stacks::StackReleaseStatus::Healthy,
        "{:?}",
        applied.messages
    );

    let containers = fixture.docker.list_containers(true).await.unwrap();
    let owned = containers
        .iter()
        .filter(|container| {
            container.labels.get("com.citadel.stack-id") == Some(&fixture.stack_id.to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(owned.len(), 1);
    let first_container = owned[0].id.clone();
    let mut replacement = fixture.claim();
    replacement.spec.common_mut().destroy_before_deploy = true;
    let reapplied = fixture
        .runtime
        .apply(&replacement, &source, &[], &fixture.cancellation, None)
        .await
        .unwrap();
    assert_eq!(
        reapplied.status,
        citadel_stacks::StackReleaseStatus::Healthy,
        "{:?}",
        reapplied.messages
    );
    let containers = fixture.docker.list_containers(true).await.unwrap();
    let owned = containers
        .iter()
        .filter(|container| {
            container.labels.get("com.citadel.stack-id") == Some(&fixture.stack_id.to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(owned.len(), 1);
    assert_ne!(
        owned[0].id, first_container,
        "DestroyBeforeDeploy replaces the previous container"
    );
    for container in owned {
        fixture
            .docker
            .delete_container(&container.id, true, true)
            .await
            .unwrap();
    }
    for network in fixture
        .docker
        .list_networks()
        .await
        .unwrap()
        .into_iter()
        .filter(|network| {
            network
                .labels
                .get("com.docker.compose.project")
                .is_some_and(|value| value == &fixture.project_name)
        })
    {
        fixture.docker.delete_network(&network.id).await.unwrap();
    }
    fixture.dispose().await;
}

#[tokio::test]
#[ignore = "requires the Phase 6 fixture and a Docker daemon already initialized as a Swarm manager"]
async fn local_swarm_stack_apply_and_delete_use_the_native_stack_lifecycle() {
    let fixture = Fixture::new("DockerSwarm").await;
    let source = fixture.source();
    let applied = fixture
        .runtime
        .apply(&fixture.claim(), &source, &[], &fixture.cancellation, None)
        .await
        .unwrap();
    assert_eq!(
        applied.status,
        citadel_stacks::StackReleaseStatus::Healthy,
        "{:?}",
        applied.messages
    );

    let services = fixture.docker.list_swarm_services().await.unwrap();
    assert!(services.iter().any(|service| {
        service.spec.get("Name").and_then(serde_json::Value::as_str)
            == Some(&format!("{}_web", fixture.project_name))
    }));

    use citadel_platforms::InventoryProjectionStore;
    let snapshot = citadel_platforms::jobs::collect_inventory(
        &fixture.docker,
        &citadel_platforms::jobs::InventoryCollectionTarget {
            platform_id: fixture.platform_id,
            platform_type: "DockerSwarm".into(),
        },
        &fixture.cancellation,
    )
    .await
    .unwrap();
    citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore::new(
        fixture.pool.clone(),
    )
    .persist(&snapshot)
    .await
    .unwrap();
    // A failed deployment that rolled back to healthy tasks is still a failed
    // release. Exercise the runtime observation boundary with Docker's states.
    for state in ["rollback_completed", "rollback_paused"] {
        let updated = sqlx::query("UPDATE swarmserviceprojections SET updatestate=$3 WHERE platformid=$1 AND dockerstacknamespace=$2")
            .bind(fixture.platform_id)
            .bind(&fixture.project_name)
            .bind(state)
            .execute(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(updated.rows_affected(), 1);
        let observed = fixture
            .runtime
            .observe(&fixture.claim(), &fixture.cancellation)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observed.status, citadel_stacks::StackReleaseStatus::Failed);
    }

    fixture
        .runtime
        .delete(
            &StackDeletionClaim {
                stack_id: fixture.stack_id,
                platform_id: fixture.platform_id,
                project_name: fixture.project_name.clone(),
                platform_type: "DockerSwarm".to_owned(),
            },
            &fixture.cancellation,
        )
        .await
        .unwrap();
    fixture.dispose().await;
}

struct Fixture {
    pool: sqlx::PgPool,
    docker: DockerClient,
    runtime: StackRuntimeRouter,
    cancellation: CancellationToken,
    platform_id: Uuid,
    stack_id: Uuid,
    release_id: Uuid,
    project_name: String,
    platform_type: &'static str,
    image: String,
}

impl Fixture {
    async fn new(platform_type: &'static str) -> Self {
        let database_url = std::env::var("CITADEL_PHASE6_DATABASE_URL")
            .expect("CITADEL_PHASE6_DATABASE_URL is required for this fixture");
        let image = std::env::var("CITADEL_PHASE6_RUNTIME_IMAGE")
            .expect("CITADEL_PHASE6_RUNTIME_IMAGE is required for this fixture");
        MigrationRunner::migrate(&database_url).await.unwrap();
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
            .unwrap();
        let platform_id = Uuid::now_v7();
        let stack_id = Uuid::now_v7();
        let project_name = format!("citadel-rust-{}", stack_id.simple());
        sqlx::query(
            r#"INSERT INTO platforms(
                   id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,
                   platformdescriptor,status,volumecount)
               VALUES($1,'unix:///var/run/docker.sock','Local',1,0,1048576,$2,0,
                      json_build_object('$type',$3),'Online',0)"#,
        )
        .bind(platform_id)
        .bind(format!("phase6-stack-{}", platform_id.simple()))
        .bind(platform_type)
        .execute(&pool)
        .await
        .unwrap();
        let docker = DockerClient::new(
            std::env::var("CITADEL_PHASE6_DOCKER_SOCKET")
                .unwrap_or_else(|_| "/var/run/docker.sock".into()),
            Duration::from_secs(15),
        )
        .unwrap();
        let runtime = StackRuntimeRouter::new(pool.clone(), docker.clone(), None);
        if platform_type == "DockerSwarm" {
            use citadel_platforms::InventoryProjectionStore;
            let snapshot = citadel_platforms::jobs::collect_inventory(
                &docker,
                &citadel_platforms::jobs::InventoryCollectionTarget {
                    platform_id,
                    platform_type: platform_type.into(),
                },
                &CancellationToken::new(),
            )
            .await
            .unwrap();
            citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore::new(
                pool.clone(),
            )
            .persist(&snapshot)
            .await
            .unwrap();
        }
        let release_id = Uuid::now_v7();
        sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate,currentstackreleaseid,controlstate,rowversion) VALUES($1,$2,$3,'WebEditor','{}','{}',$4,'Processing',1)")
            .bind(stack_id).bind(&project_name).bind(Uuid::from_u128(1)).bind(release_id).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,'{}',$4,'Applying','1')")
            .bind(release_id).bind(Uuid::from_u128(1)).bind(platform_id).bind(stack_id).execute(&pool).await.unwrap();
        Self {
            pool,
            docker,
            runtime,
            cancellation: CancellationToken::new(),
            platform_id,
            stack_id,
            release_id,
            project_name,
            platform_type,
            image,
        }
    }

    fn spec(&self) -> StackSpec {
        StackSpec::WebEditor {
            compose_file: format!(
                "services:\n  web:\n    image: {}\n    command: [\"sh\", \"-c\", \"sleep 60\"]\n",
                self.image
            ),
            update_behavior: StackUpdateBehavior::Disabled,
            common: StackSpecCommon {
                project_name: Some(self.project_name.clone()),
                destroy_before_deploy: false,
                ..Default::default()
            },
        }
    }

    fn source(&self) -> StackApplySource {
        let compose = self.spec().compose_file().unwrap().to_owned();
        let labels = create_ownership_labels_override(
            std::slice::from_ref(&compose),
            self.stack_id,
            self.release_id,
            self.platform_type == "DockerSwarm",
        )
        .unwrap();
        StackApplySource {
            files: vec![
                StackSourceFile {
                    relative_path: "compose.yml".to_owned(),
                    content: compose.into_bytes(),
                },
                StackSourceFile {
                    relative_path: ".citadel/citadel.labels.yml".to_owned(),
                    content: labels.into_bytes(),
                },
            ],
            compose_paths: vec!["compose.yml".to_owned()],
            env_file_paths: Vec::new(),
            working_directory: ".".to_owned(),
            labels_override_path: Some(".citadel/citadel.labels.yml".to_owned()),
            resolved_commit_sha: None,
        }
    }

    fn claim(&self) -> StackOperationClaim {
        StackOperationClaim {
            stack_id: self.stack_id,
            release_id: self.release_id,
            platform_id: self.platform_id,
            name: self.project_name.clone(),
            project_name: self.project_name.clone(),
            platform_type: self.platform_type.to_owned(),
            spec: self.spec(),
            row_version: 1,
            actor_id: Uuid::nil(),
            operation: "Apply".to_owned(),
            service_names: Vec::new(),
        }
    }

    async fn dispose(self) {
        sqlx::query("DELETE FROM stacks WHERE id=$1")
            .bind(self.stack_id)
            .execute(&self.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM platforms WHERE id=$1")
            .bind(self.platform_id)
            .execute(&self.pool)
            .await
            .unwrap();
        self.pool.close().await;
    }
}

#[tokio::test]
#[ignore = "requires an isolated two-node Swarm, CITADEL_PHASE6_DOCKER_SOCKET and disposable CITADEL_PHASE6_DATABASE_URL"]
async fn swarm_material_capture_preserves_mounts_fences_late_results_and_recovers_interrupted_release()
 {
    use citadel_platforms::InventoryProjectionStore;
    let f = Fixture::new("DockerSwarm").await;
    let compose = format!(
        "version: '3.8'\nservices:\n  web:\n    image: {}\n    command: ['sh', '-c', 'sleep 600']\n    configs:\n      - source: settings\n        target: /etc/settings\n    secrets:\n      - source: token\n        target: auth-token\n    deploy:\n      replicas: 2\n      placement:\n        max_replicas_per_node: 1\nconfigs:\n  settings:\n    file: ./settings.txt\nsecrets:\n  token:\n    file: ./token.txt\n",
        f.image
    );
    let labels = create_ownership_labels_override(
        std::slice::from_ref(&compose),
        f.stack_id,
        f.release_id,
        true,
    )
    .unwrap();
    let source = StackApplySource {
        files: vec![
            StackSourceFile {
                relative_path: "compose.yml".into(),
                content: compose.into_bytes(),
            },
            StackSourceFile {
                relative_path: "labels.yml".into(),
                content: labels.into_bytes(),
            },
            StackSourceFile {
                relative_path: "settings.txt".into(),
                content: b"test-settings".to_vec(),
            },
            StackSourceFile {
                relative_path: "token.txt".into(),
                content: b"test-only-secret-value".to_vec(),
            },
        ],
        compose_paths: vec!["compose.yml".into()],
        env_file_paths: vec![],
        working_directory: ".".into(),
        labels_override_path: Some("labels.yml".into()),
        resolved_commit_sha: None,
    };
    let claim = f.claim();
    let result = f
        .runtime
        .apply(&claim, &source, &[], &f.cancellation, None)
        .await
        .unwrap();
    assert_eq!(
        result.status,
        citadel_stacks::StackReleaseStatus::Healthy,
        "{:?}",
        result.messages
    );
    let rows: Vec<(String,String,String,serde_json::Value)> = sqlx::query_as("SELECT kind,dockerresourceid,composeresourcename,mounts FROM stackreleaseswarmresources WHERE stackreleaseid=$1 ORDER BY kind").bind(f.release_id).fetch_all(&f.pool).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "Config");
    assert_eq!(rows[0].2, "settings");
    assert_eq!(
        rows[0].3,
        serde_json::json!([{"ServiceName":"web","TargetName":"/etc/settings"}])
    );
    assert_eq!(rows[1].0, "Secret");
    assert_eq!(rows[1].2, "token");
    assert_eq!(
        rows[1].3,
        serde_json::json!([{"ServiceName":"web","TargetName":"auth-token"}])
    );
    assert!(
        !serde_json::to_string(&rows)
            .unwrap()
            .contains("test-only-secret-value")
    );
    let mut late = claim.clone();
    late.row_version += 1;
    assert!(
        f.runtime
            .apply(&late, &source, &[], &f.cancellation, None)
            .await
            .is_err()
    );
    let after: Vec<(String,String,String,serde_json::Value)> = sqlx::query_as("SELECT kind,dockerresourceid,composeresourcename,mounts FROM stackreleaseswarmresources WHERE stackreleaseid=$1 ORDER BY kind").bind(f.release_id).fetch_all(&f.pool).await.unwrap();
    assert_eq!(
        after, rows,
        "a late operation must not replace retained metadata"
    );
    // Crash after resource capture, before healthy completion: periodic inventory
    // can now recover using the exact immutable resource records.
    sqlx::query("UPDATE stacks SET controlstate='Idle' WHERE id=$1")
        .bind(f.stack_id)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE stackreleases SET status='TimedOut' WHERE id=$1")
        .bind(f.release_id)
        .execute(&f.pool)
        .await
        .unwrap();
    let snapshot = citadel_platforms::jobs::collect_inventory(
        &f.docker,
        &citadel_platforms::jobs::InventoryCollectionTarget {
            platform_id: f.platform_id,
            platform_type: "DockerSwarm".into(),
        },
        &f.cancellation,
    )
    .await
    .unwrap();
    assert_eq!(
        snapshot.swarm.as_ref().unwrap().nodes.len(),
        2,
        "fixture must exercise two real Swarm nodes"
    );
    let nodes: std::collections::BTreeSet<_> = snapshot
        .swarm
        .as_ref()
        .unwrap()
        .tasks
        .iter()
        .filter(|t| t.state.eq_ignore_ascii_case("running"))
        .map(|t| &t.node_id)
        .collect();
    assert_eq!(nodes.len(), 2, "both nodes must run a task");
    citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore::new(
        f.pool.clone(),
    )
    .persist(&snapshot)
    .await
    .unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
        .bind(f.release_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(status, "Healthy");
    // Stop only the explicitly supplied, labelled disposable outer DinD worker.
    // No event is delivered to Core here: periodic reconciliation must repair it.
    let worker = std::env::var("CITADEL_PHASE6_SWARM_WORKER").expect("disposable worker required");
    let label = tokio::process::Command::new("docker")
        .env_remove("DOCKER_HOST")
        .args([
            "inspect",
            "--format",
            "{{index .Config.Labels \"citadel.test\"}}",
            &worker,
        ])
        .output()
        .await
        .unwrap();
    assert!(label.status.success());
    assert_eq!(String::from_utf8_lossy(&label.stdout).trim(), "jobs-live");
    let stopped = tokio::process::Command::new("docker")
        .env_remove("DOCKER_HOST")
        .args(["stop", "--time", "2", &worker])
        .output()
        .await
        .unwrap();
    assert!(stopped.status.success());
    let degraded = wait_for_live_stack_status(&f, "Degraded").await;
    // Always restart this fixture before asserting, including a failed observation.
    let restarted = tokio::process::Command::new("docker")
        .env_remove("DOCKER_HOST")
        .args(["start", &worker])
        .output()
        .await
        .unwrap();
    assert!(restarted.status.success());
    assert!(
        degraded,
        "lost worker must degrade the Stack without a daemon event"
    );
    assert!(
        wait_for_live_stack_status(&f, "Healthy").await,
        "worker rejoin must restore healthy status"
    );
    for service in f
        .docker
        .list_swarm_services()
        .await
        .unwrap()
        .into_iter()
        .filter(|s| {
            s.spec
                .pointer("/Labels/com.citadel.stack-id")
                .and_then(serde_json::Value::as_str)
                == Some(&f.stack_id.to_string())
        })
    {
        f.docker.delete_swarm_service(&service.id).await.unwrap();
    }
    f.dispose().await;
}

async fn wait_for_live_stack_status(f: &Fixture, expected: &str) -> bool {
    use citadel_platforms::InventoryProjectionStore;
    tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let snapshot = citadel_platforms::jobs::collect_inventory(
                &f.docker,
                &citadel_platforms::jobs::InventoryCollectionTarget {
                    platform_id: f.platform_id,
                    platform_type: "DockerSwarm".into(),
                },
                &f.cancellation,
            )
            .await
            .unwrap();
            citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore::new(
                f.pool.clone(),
            )
            .persist(&snapshot)
            .await
            .unwrap();
            let status: String = sqlx::query_scalar("SELECT status FROM stackreleases WHERE id=$1")
                .bind(f.release_id)
                .fetch_one(&f.pool)
                .await
                .unwrap();
            if status == expected {
                return;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
    .await
    .is_ok()
}
