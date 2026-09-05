#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use citadel_adapters::docker::DockerClient;
use citadel_adapters::stack_runtime::StackRuntimeRouter;
use citadel_database::MigrationRunner;
use citadel_stacks::{
    StackApplySource, StackDeletionClaim, StackOperationClaim, StackRuntimePort, StackSourceFile,
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
        .apply(&fixture.claim(), &source, &[], &fixture.cancellation)
        .await
        .unwrap();
    assert_eq!(applied.status, citadel_stacks::StackReleaseStatus::Healthy);

    let containers = fixture.docker.list_containers(true).await.unwrap();
    let owned = containers
        .iter()
        .filter(|container| {
            container.labels.get("com.citadel.stack-id") == Some(&fixture.stack_id.to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(owned.len(), 1);
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
        .apply(&fixture.claim(), &source, &[], &fixture.cancellation)
        .await
        .unwrap();
    assert_eq!(applied.status, citadel_stacks::StackReleaseStatus::Healthy);

    let services = fixture.docker.list_swarm_services().await.unwrap();
    assert!(services.iter().any(|service| {
        service.spec.get("Name").and_then(serde_json::Value::as_str)
            == Some(&format!("{}_web", fixture.project_name))
    }));

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
        let docker =
            DockerClient::new(Path::new("/var/run/docker.sock"), Duration::from_secs(15)).unwrap();
        let runtime = StackRuntimeRouter::new(pool.clone(), docker.clone(), None);
        Self {
            pool,
            docker,
            runtime,
            cancellation: CancellationToken::new(),
            platform_id,
            stack_id,
            release_id: Uuid::now_v7(),
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
        }
    }

    async fn dispose(self) {
        sqlx::query("DELETE FROM platforms WHERE id=$1")
            .bind(self.platform_id)
            .execute(&self.pool)
            .await
            .unwrap();
        self.pool.close().await;
    }
}
