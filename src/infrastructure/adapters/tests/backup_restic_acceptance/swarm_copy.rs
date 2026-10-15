use super::*;
use citadel_adapters::{
    connectors::{
        docker::DockerClient,
        routing::{
            platforms::runtime::PlatformRuntimeRouter, resource_copies::CopyStorageRuntime,
            swarm_services::SwarmServiceRuntimeRouter,
        },
    },
    persistence::postgres::{
        resource_copies::PostgresCopyRepository, swarm_services::PostgresSwarmServiceRepository,
    },
};
use citadel_resource_copies::*;
use citadel_swarm_services::{SwarmServiceRepository, SwarmServiceRuntime};

pub(super) async fn verify(
    cluster: &Cluster,
    pool: &sqlx::PgPool,
    backups: &PostgresBackupPersistence,
    executor: &DockerResticBackupExecutor,
    run: &BackupRun,
    agent: &AgentClient,
    edge: &EdgeRegistry,
) {
    let actor = super::super::resource_copy::administrator(pool).await;
    let platform = run.items[0].platform_id;
    let node = run.items[0].docker_node_id.as_ref().unwrap();
    let volume = &run.items[0].volume_name;
    let registry = Uuid::now_v7();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}'::json,$2,$3,'docker.io','Enabled')")
        .bind(registry).bind(actor.value()).bind(format!("copy-registry-{registry}")).execute(pool).await.unwrap();
    let services = PostgresSwarmServiceRepository::new(pool.clone());
    let source = services.create(actor,true,&citadel_swarm_services::CreateSwarmService {
        name:format!("copy-source-{}",Uuid::now_v7().simple()),platform_id:platform,description:None,tag_ids:vec![],duplicate_source:None,
        spec:serde_json::from_value(json!({"image":{"$type":"External","registryId":registry,"imageTag":IMAGE},"replicas":1,"command":["sh"],"arguments":["-c","exec sleep 3600"],"mounts":[{"kind":"Volume","source":volume,"target":"/data","readOnly":false}],"placementConstraints":[format!("node.id == {node}")]})).unwrap(),
    }).await.unwrap();
    let docker = DockerClient::new("/no-core-docker-allowed.sock", Duration::from_secs(2)).unwrap();
    let router = Arc::new(PlatformRuntimeRouter::new(
        pool.clone(),
        docker.clone(),
        Some(agent.clone()),
        edge.clone(),
    ));
    let runtime = SwarmServiceRuntimeRouter::new(pool.clone(), docker, Some(agent.clone()))
        .with_edge(edge.clone());
    apply(&services, &runtime, actor, source.id, &cluster.cancellation).await;
    let copies = PostgresCopyRepository::new(pool.clone());
    let source = copies
        .source(actor, CopyKind::SwarmService, source.id)
        .await
        .unwrap();
    for mode in [CopyDataMode::ExistingBackup, CopyDataMode::CurrentData] {
        let request = CopyRequest {
            destination_platform_id: None,
            destination_node_id: None,
            data_mode: mode,
            consistency_acknowledged: mode == CopyDataMode::CurrentData,
            kind: CopyKind::SwarmService,
            source_id: source.id,
            name: format!("copy-target-{}", Uuid::now_v7().simple()),
            selections: if mode == CopyDataMode::ExistingBackup {
                vec![CopySelection {
                    volume: volume.clone(),
                    backup_run_id: run.id,
                    backup_run_item_id: run.items[0].id,
                }]
            } else {
                vec![]
            },
        };
        let plan = planning::plan(
            Uuid::now_v7(),
            source.clone(),
            request,
            copies.snapshots(actor, &source).await.unwrap(),
        )
        .unwrap();
        assert!(
            plan.preview.blockers.is_empty(),
            "{:?}",
            plan.preview.blockers
        );
        assert_eq!(plan.preview.volumes[0].docker_node_id.as_ref(), Some(node));
        let storage = CopyStorageRuntime {
            image: "restic/restic:0.18.1".into(),
            runtime: router.clone(),
        };
        storage
            .check(&plan, None, &cluster.cancellation)
            .await
            .unwrap();
        copies.save_review(actor, &plan).await.unwrap();
        let started = copies.start(actor, plan.preview.id).await.unwrap();
        if mode == CopyDataMode::CurrentData {
            let (claimed, job) = copies
                .claim_current(actor, plan.preview.id)
                .await
                .unwrap()
                .unwrap();
            let result = storage
                .copy_current(&claimed, &job, &cluster.cancellation)
                .await;
            assert!(result.is_ok(), "{result:?}");
            copies
                .finish_current(
                    actor,
                    plan.preview.id,
                    job.id,
                    result.map_err(|e| e.to_string()),
                )
                .await
                .unwrap();
        } else {
            let claim = backups
                .claim_restore(Utc::now() - chrono::Duration::hours(1))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(claim.run.id, started.restores[0].id);
            let result = executor.restore(&claim, &cluster.cancellation).await;
            assert_eq!(
                result.status,
                BackupRestoreStatus::Succeeded,
                "{:?}",
                result.error_message
            );
            backups.finish_restore(&claim, &result).await.unwrap();
        }
        let progress = copies.get(actor, plan.preview.id).await.unwrap();
        storage
            .check(&plan, Some(&progress.restores), &cluster.cancellation)
            .await
            .unwrap();
        let completed = copies
            .advance(actor, plan.preview.id, Some(Ok(())))
            .await
            .unwrap();
        assert_eq!(
            completed.status,
            CopyStatus::Succeeded,
            "{:?}",
            completed.message
        );
        let destination = completed.destination_id.unwrap();
        // The real managed-service Apply passes the restored-volume guard and uses
        // the exact worker. Same-named source volumes on other nodes contain junk.
        apply(
            &services,
            &runtime,
            actor,
            destination,
            &cluster.cancellation,
        )
        .await;
        let target = &completed.restores[0].target;
        assert_eq!(
            cluster
                .node(
                    1,
                    &[
                        "run",
                        "--rm",
                        "--volume",
                        &format!("{target}:/data:ro"),
                        "--entrypoint",
                        "stat",
                        IMAGE,
                        "-c",
                        "%u:%g:%a",
                        "/data"
                    ],
                    None
                )
                .await,
            b"1123:2211:751\n",
            "Both copy modes must preserve the volume root on the pinned worker"
        );
        assert_eq!(
            cluster
                .node(
                    1,
                    &[
                        "run",
                        "--rm",
                        "--volume",
                        &format!("{target}:/data:ro"),
                        "--entrypoint",
                        "cat",
                        IMAGE,
                        "/data/payload.txt"
                    ],
                    None
                )
                .await,
            PAYLOAD
        );
        for other in [0, 2] {
            assert!(
                cluster
                    .node(
                        other,
                        &[
                            "volume",
                            "ls",
                            "--quiet",
                            "--filter",
                            &format!("name=^{target}$")
                        ],
                        None
                    )
                    .await
                    .is_empty(),
                "copy volume appeared on the wrong node"
            );
        }
        assert_eq!(
            cluster
                .node(
                    1,
                    &[
                        "run",
                        "--rm",
                        "--volume",
                        &format!("{volume}:/data:ro"),
                        "--entrypoint",
                        "cat",
                        IMAGE,
                        "/data/payload.txt"
                    ],
                    None
                )
                .await,
            PAYLOAD
        );
        // An attached volume must be retained, even when its ownership labels match.
        assert!(
            storage
                .cleanup(&plan, &completed.restores, &cluster.cancellation)
                .await
                .is_err()
        );
    }
}

async fn apply(
    store: &PostgresSwarmServiceRepository,
    runtime: &SwarmServiceRuntimeRouter,
    actor: ActorId,
    id: Uuid,
    cancel: &CancellationToken,
) {
    let claim = store
        .claim_operation(
            actor,
            true,
            citadel_swarm_services::ServiceOperationRequest {
                id,
                kind: citadel_swarm_services::ServiceOperationKind::Apply,
                replicas: None,
                expected_version: None,
            },
        )
        .await
        .unwrap();
    store.mark_attempted(&claim).await.unwrap();
    let mut result = runtime.apply(&claim, cancel).await.unwrap();
    store.mark_accepted(&claim, &result).await.unwrap();
    // Reconciliation reloads the claim after acceptance persists the Docker ID.
    let claim = store
        .active_operation_claims(None, 100)
        .await
        .unwrap()
        .into_iter()
        .find(|(_, active)| active.id == id)
        .expect("accepted service operation must remain active")
        .1;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    while !result.rollout_complete {
        assert!(result.rollout_error.is_none(), "{:?}", result.rollout_error);
        assert!(
            tokio::time::Instant::now() < deadline,
            "Swarm copy rollout timed out"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(observed) = runtime.observe(&claim, cancel).await.unwrap() {
            result = observed;
        }
    }
    store
        .complete_operation(actor, &claim, &result)
        .await
        .unwrap();
}
