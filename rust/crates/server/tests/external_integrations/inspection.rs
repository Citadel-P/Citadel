use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use citadel_adapters::{
    connectors::{
        docker::DockerClient,
        edge::EdgeRegistry,
        routing::{
            containers::ContainerRuntimeRouter,
            platforms::registration::PlatformRegistrationRuntimeRouter,
            volumes::content::VolumeContentAdapter,
        },
    },
    persistence::postgres::platforms::{
        PostgresPlatformReader, inventory::store::PostgresInventoryProjectionStore,
        registration::PostgresPlatformRegistrationRepository,
    },
};
use citadel_platforms::{
    InventoryProjectionStore, PlatformReadService, PlatformRegistrationService,
    jobs::{InventoryCollectionTarget, collect_inventory},
};
use citadel_server::api::routes::{platforms as platforms_http, platforms::PlatformsHttpState};
use serde_json::Value;
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use uuid::Uuid;

/// The .NET Vault acceptance checks both public inspection routes after actual
/// deployment. Use the production collector/store so Stack ownership and the
/// browser's persisted container ID are not supplied by a hand-written fixture.
pub async fn verify(
    pool: &PgPool,
    docker: &DockerClient,
    provider: &super::vault_tests::VerifiedProvider,
    platform: Uuid,
    stack: Uuid,
    version: &str,
) {
    let snapshot = collect_inventory(
        docker,
        &InventoryCollectionTarget {
            platform_id: platform,
            platform_type: citadel_platforms::PlatformKind::Docker,
        },
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    PostgresInventoryProjectionStore::new(pool.clone())
        .persist(&snapshot)
        .await
        .unwrap();
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,dockercontainerid FROM containers WHERE stackid=$1 AND state='Running'",
    )
    .bind(stack)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    let (id, docker_id) = &rows[0];
    let edge = EdgeRegistry::default();
    let app = platforms_http::router(PlatformsHttpState {
        tasks: citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        volume_content: Arc::new(VolumeContentAdapter::new(
            pool.clone(),
            docker.clone(),
            None,
            edge.clone(),
            "unused".into(),
        citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
)),
        containers: Arc::new(
            ContainerRuntimeRouter::new(pool.clone(), docker.clone(), None, edge.clone())
                .into_service(std::sync::Arc::new(citadel_server::tasks::platforms::TrackedContainerTasks::new(citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new())))),
        ),
        identity: provider.identity.clone(),
        platforms: Arc::new(PlatformReadService::new(Arc::new(
            PostgresPlatformReader::new(pool.clone()),
        ))),
        registrations: Arc::new(PlatformRegistrationService::new(
            Arc::new(PostgresPlatformRegistrationRepository::new(pool.clone())),
            Arc::new(PlatformRegistrationRuntimeRouter::new(docker.clone(), None)),
        )),
        pool: pool.clone(),
        registries: Arc::new(
            citadel_adapters::persistence::postgres::registries::PostgresRegistryRepository::new(pool.clone()),
        ),
        platform_metadata: Arc::new(
            citadel_adapters::persistence::postgres::platforms::PostgresPlatformMetadataRepository::new(
                pool.clone(),
            ),
        ),
        docker: docker.clone(),
        agent: None,
        edge,
        realtime: None,
        stats_sample_max_age: Duration::from_secs(30),
    })
    .layer(axum::middleware::from_fn_with_state(
        provider.identity.clone(),
        citadel_server::api::routes::authentication::authentication_middleware,
    ));
    let data = get(
        &app,
        &provider.bearer,
        &format!("/api/v1/stacks/{stack}/data"),
    )
    .await;
    let containers = data["containers"].as_array().unwrap();
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0]["state"], "Running");
    let returned_id = containers[0]["id"].as_str().unwrap();
    assert_eq!(returned_id, docker_id);
    for path in [
        format!("/api/v1/containers/{id}/inspect"),
        format!("/api/v1/containers/{returned_id}/inspect"),
        format!("/api/v1/stacks/{stack}/containers/{docker_id}/inspect"),
    ] {
        let body = get(&app, &provider.bearer, &path).await;
        assert!(!body.to_string().contains(super::VALUE));
        assert_eq!(body["id"], *docker_id);
        assert_eq!(body["state"]["running"], true);
        assert!(body["config"]["image"].as_str().unwrap().contains("alpine"));
        let environment = body["config"]["env"].as_array().unwrap();
        assert!(environment.iter().any(|entry| entry == "TOKEN=********"));
        assert!(
            environment
                .iter()
                .any(|entry| entry == &format!("VERSION={version}"))
        );
    }
}

async fn get(app: &axum::Router, bearer: &str, path: &str) -> Value {
    let request = Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(status, StatusCode::OK, "{path}: {body}");
    body
}
