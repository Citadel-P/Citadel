#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use citadel_adapters::connectors::docker::DockerClient;
use citadel_adapters::connectors::routing::deployments::DeploymentRuntimeRouter;
use citadel_database::MigrationRunner;
use citadel_deployments::{
    ContainerRestartPolicy, DeploymentImageInfo, DeploymentRuntime, DeploymentSpec, LifeCycleSpec,
    RuntimeContainerState, RuntimeDeploymentCommand, UpdateBehavior,
};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL and a Docker Unix socket"]
async fn local_apply_creates_observes_and_deletes_a_real_container() {
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
    let deployment_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO platforms(
               id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,
               platformdescriptor,status,volumecount)
           VALUES($1,'local','Local',1,0,1048576,$2,0,
                  '{"$type":"Docker"}'::json,'Online',0)"#,
    )
    .bind(platform_id)
    .bind(format!("phase6-runtime-{}", platform_id.simple()))
    .execute(&pool)
    .await
    .unwrap();

    let docker =
        DockerClient::new(Path::new("/var/run/docker.sock"), Duration::from_secs(10)).unwrap();
    let runtime = DeploymentRuntimeRouter::new(pool.clone(), docker.clone(), None);
    let cancellation = CancellationToken::new();
    let command = RuntimeDeploymentCommand {
        deployment_id,
        name: format!("citadel-rust-phase6b-{}", deployment_id.simple()),
        image_id: image,
        spec: DeploymentSpec {
            image: DeploymentImageInfo::Local {
                image_id: Uuid::now_v7().to_string(),
            },
            update_behavior: UpdateBehavior::Disabled,
            life_cycle_spec: Some(LifeCycleSpec {
                restart_policy: ContainerRestartPolicy::No,
                stop_signal: None,
                stop_timeout: Some(5),
            }),
            resource_spec: None,
            labels: None,
            ports: None,
            volumes: None,
            networks: None,
            command: Some(vec!["sleep".to_owned(), "30".to_owned()]),
            environment_variables: None,
        },
        environment_variables: Vec::new(),
    };

    let applied = runtime
        .apply_container(platform_id, &command, &cancellation)
        .await;
    let container_id = match &applied {
        Ok(result) => Some(result.docker_container_id.clone()),
        Err(_) => docker
            .list_containers(true)
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|container| {
                container.labels.get("com.citadel.deployment-id")
                    == Some(&deployment_id.to_string())
            })
            .map(|container| container.id),
    };
    if let Some(container_id) = &container_id {
        runtime
            .delete_container(platform_id, container_id, &cancellation)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    let mut offline_command = command.clone();
    offline_command.deployment_id = Uuid::now_v7();
    offline_command.name = format!(
        "citadel-rust-phase6b-offline-{}",
        offline_command.deployment_id.simple()
    );
    let offline_error = runtime
        .apply_container(platform_id, &offline_command, &cancellation)
        .await
        .unwrap_err();
    assert!(offline_error.to_string().contains("disconnected"));
    assert!(
        docker
            .list_containers(true)
            .await
            .unwrap()
            .iter()
            .all(|container| {
                container.labels.get("com.citadel.deployment-id")
                    != Some(&offline_command.deployment_id.to_string())
            })
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let applied = applied.unwrap();
    assert_eq!(applied.state, RuntimeContainerState::Running);
    assert!(container_id.is_some());
}
