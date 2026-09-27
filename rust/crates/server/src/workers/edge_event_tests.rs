use super::*;
use base64::Engine;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::{
    containers::v1::ListContainersResponse,
    edge::v1::{EnrollmentRequest, core_envelope},
    platforms::v1::{
        DaemonContainerEventResponse, DaemonEventResponse, DaemonImageEventResponse,
        daemon_event_response,
    },
    shared_models::v1::{ContainerMessage, PlatformInfoResponse},
};
use prost::Message;
use std::sync::Mutex;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn edge_missing_delta_and_failed_scope_read_do_not_restart_full_inventory() {
    run_scoped_edge_failure(false, false).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn edge_image_refresh_finishes_while_containers_wait_for_retry() {
    run_scoped_edge_failure(true, false).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn edge_delta_committed_during_a_read_rejects_the_old_container_set() {
    run_scoped_edge_failure(false, true).await;
}

async fn run_scoped_edge_failure(concurrent_image: bool, stale_read: bool) {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let target = EdgeTarget::platform(Uuid::now_v7());
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'EdgeAgent',0,0,0,0,0,'{\"$type\":\"Docker\"}','Online')").bind(target.platform_id).execute(&pool).await.unwrap();
    let store = PostgresEdgeStore::new(pool.clone());
    let (_, token, _) = store
        .create_enrollment(&target, citadel_identity::SYSTEM_ACTOR_ID)
        .await
        .unwrap();
    let mut key_bytes = [0; 32];
    getrandom::fill(&mut key_bytes).unwrap();
    let key =
        citadel_adapters::connectors::agent::client::AgentRequestSigner::from_bytes(&key_bytes);
    let public_key = base64::engine::general_purpose::STANDARD
        .decode(key.public_key_base64())
        .unwrap();
    let binding = store
        .enroll(&EnrollmentRequest {
            enrollment_token: token,
            public_key,
            protocol_version: 2,
            capabilities_json:
                r#"{"commands":["platform.checkHealth","containers.list","containers.logs"]}"#
                    .into(),
            daemon_id: target.platform_id.to_string(),
            ..Default::default()
        })
        .await
        .unwrap();
    let (session, mut receiver) = EdgeRegistry::default()
        .register(target.clone(), binding.agent_id)
        .unwrap();
    store
        .connected(&binding, session.connected_at)
        .await
        .unwrap();
    let cancel = CancellationToken::new();
    let actor_cancel = cancel.clone();
    let actor_session = session.clone();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor_calls = calls.clone();
    let actor_pool = pool.clone();
    let actor = tokio::spawn(async move {
        let mut stream_id = None;
        let mut container_reads = 0;
        loop {
            let envelope = tokio::select! { () = actor_cancel.cancelled() => break, value = receiver.recv() => value.unwrap() };
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                continue;
            };
            let id = Uuid::parse_str(&command.command_id).unwrap();
            let kind = EdgeCommandKind::try_from(command.kind).unwrap();
            actor_calls.lock().unwrap().push(kind);
            let response = match kind {
                EdgeCommandKind::PlatformDaemonEventsStream => {
                    stream_id = Some(id);
                    continue;
                }
                EdgeCommandKind::PlatformGetInfo => PlatformInfoResponse {
                    id: binding.daemon_id.clone(),
                    ..Default::default()
                }
                .encode_to_vec(),
                EdgeCommandKind::ContainerList => {
                    container_reads += 1;
                    if container_reads == 2 && !stale_read {
                        actor_session.fail(id, "unavailable");
                        if concurrent_image {
                            actor_session.output(
                                stream_id.unwrap(),
                                DaemonEventResponse {
                                    scope: 1,
                                    kind: Some(
                                        daemon_event_response::Kind::DaemonImageEventResponse(
                                            DaemonImageEventResponse {
                                                action: "pull".into(),
                                                image_id: "image".into(),
                                                image: None,
                                            },
                                        ),
                                    ),
                                }
                                .encode_to_vec(),
                            );
                        }
                        continue;
                    }
                    if container_reads == 2 && stale_read {
                        actor_session.output(
                            stream_id.unwrap(),
                            DaemonEventResponse {
                                scope: 1,
                                kind: Some(
                                    daemon_event_response::Kind::DaemonContainerEventResponse(
                                        DaemonContainerEventResponse {
                                            action: "die".into(),
                                            container_id: "fixture".into(),
                                            container: Some(ContainerMessage {
                                                id: "fixture".into(),
                                                image: "alpine".into(),
                                                image_id: "image".into(),
                                                name: "web".into(),
                                                state: 5,
                                                ..Default::default()
                                            }),
                                        },
                                    ),
                                ),
                            }
                            .encode_to_vec(),
                        );
                        tokio::time::timeout(Duration::from_secs(3), async {
                            loop {
                                let committed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM containers WHERE platformid=$1 AND dockercontainerid='fixture' AND state='Exited')")
                                    .bind(actor_session.target.platform_id).fetch_one(&actor_pool).await.unwrap();
                                if committed { break; }
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        }).await.expect("delta must commit while the list RPC is still in flight");
                    }
                    ListContainersResponse {
                        containers: if container_reads == 1 {
                            Default::default()
                        } else {
                            [(
                                "fixture".into(),
                                ContainerMessage {
                                    id: "fixture".into(),
                                    image: "alpine".into(),
                                    image_id: "image".into(),
                                    name: "web".into(),
                                    state: if stale_read && container_reads == 2 {
                                        2
                                    } else {
                                        5
                                    },
                                    ..Default::default()
                                },
                            )]
                            .into()
                        },
                    }
                    .encode_to_vec()
                }
                EdgeCommandKind::ImageList
                | EdgeCommandKind::NetworkList
                | EdgeCommandKind::VolumeList => vec![],
                _ => panic!("unexpected command: {kind:?}"),
            };
            actor_session.output(id, response);
            actor_session.complete(id, true);
            if kind == EdgeCommandKind::VolumeList {
                actor_session.output(
                    stream_id.unwrap(),
                    DaemonEventResponse {
                        scope: 1,
                        kind: Some(daemon_event_response::Kind::DaemonContainerEventResponse(
                            DaemonContainerEventResponse {
                                action: "die".into(),
                                container_id: "fixture".into(),
                                container: None,
                            },
                        )),
                    }
                    .encode_to_vec(),
                );
            }
        }
    });
    let worker_session = session.clone();
    let worker_pool = pool.clone();
    let worker_cancel = cancel.clone();
    let worker = tokio::spawn(async move {
        observe(
            worker_session,
            worker_pool,
            None,
            IoBudget::new(
                2.try_into().unwrap(),
                citadel_runtime::runtime_metrics::RuntimeWork::Inventory,
            ),
            InventorySettings {
                node_policy: Default::default(),
                reconciliation_interval: Duration::from_secs(3600),
            },
            &worker_cancel,
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let state: Option<String> = sqlx::query_scalar(
                "SELECT state FROM containers WHERE platformid=$1 AND dockercontainerid='fixture'",
            )
            .bind(target.platform_id)
            .fetch_optional(&pool)
            .await
            .unwrap();
            if state.as_deref() == Some("Exited")
                && (!stale_read
                    || calls
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|&&kind| kind == EdgeCommandKind::ContainerList)
                        .count()
                        == 3)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let observed = calls.lock().unwrap().clone();
    for kind in [
        EdgeCommandKind::PlatformGetInfo,
        EdgeCommandKind::ImageList,
        EdgeCommandKind::NetworkList,
        EdgeCommandKind::VolumeList,
    ] {
        assert_eq!(
            observed.iter().filter(|&&v| v == kind).count(),
            if concurrent_image && kind == EdgeCommandKind::ImageList {
                2
            } else {
                1
            },
            "only bootstrap or its explicit resource event may call {kind:?}: {observed:?}"
        );
    }
    assert_eq!(
        observed
            .iter()
            .filter(|&&v| v == EdgeCommandKind::ContainerList)
            .count(),
        3
    );
    if concurrent_image {
        let image_refresh = observed
            .iter()
            .rposition(|&kind| kind == EdgeCommandKind::ImageList)
            .unwrap();
        let container_retry = observed
            .iter()
            .rposition(|&kind| kind == EdgeCommandKind::ContainerList)
            .unwrap();
        assert!(
            image_refresh < container_retry,
            "image work must not wait for the container retry"
        );
    }
    cancel.cancel();
    worker.await.unwrap().unwrap();
    actor.await.unwrap();
    session.close();
    pool.close().await;
}
