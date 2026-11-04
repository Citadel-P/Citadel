use super::swarm::resume_nodes;
use crate::connectors::edge::{EdgeRegistry, EdgeTarget};
use citadel_contracts::citadel::edge::v1::EdgeCommandKind;
use citadel_contracts::citadel::edge::v1::core_envelope;
use citadel_contracts::citadel::platforms::v1::AgentUpdateDrainRequest;
use prost::Message;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn node_sessions_allow_update_metadata_and_drain_but_not_self_replacement() {
    let edge = EdgeRegistry::default();
    let (session, _receiver) = edge
        .register(
            EdgeTarget::node(Uuid::now_v7(), "worker".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    for kind in [
        EdgeCommandKind::PlatformAgentUpdateInfo,
        EdgeCommandKind::PlatformAgentUpdateDrain,
    ] {
        let command = session
            .command(kind, vec![], std::time::Duration::from_secs(5), false)
            .unwrap();
        drop(command);
    }
    assert!(
        session
            .command(
                EdgeCommandKind::PlatformUpdateAgent,
                vec![],
                std::time::Duration::from_secs(5),
                false
            )
            .is_err()
    );
}

#[tokio::test]
async fn disconnected_node_cleanup_is_retried_on_its_new_session() {
    let edge = EdgeRegistry::default();
    let platform = Uuid::now_v7();
    let operation = Uuid::now_v7();
    let nodes = vec!["worker".to_owned()];
    let cancel = CancellationToken::new();
    assert!(
        resume_nodes(&edge, platform, &nodes, operation, &cancel)
            .await
            .is_err()
    );
    let (session, mut receiver) = edge
        .register(EdgeTarget::node(platform, "worker".into()), Uuid::now_v7())
        .unwrap();
    let (result, ()) = tokio::join!(
        resume_nodes(&edge, platform, &nodes, operation, &cancel),
        async {
            let envelope = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .unwrap()
                .unwrap();
            let Some(core_envelope::Body::Command(command)) = envelope.body else {
                panic!("expected command")
            };
            assert_eq!(
                command.kind,
                EdgeCommandKind::PlatformAgentUpdateDrain as i32
            );
            let request = AgentUpdateDrainRequest::decode(command.payload.as_slice()).unwrap();
            assert_eq!(request.operation_id, operation.to_string());
            assert!(request.resume);
            session.complete(Uuid::parse_str(&command.command_id).unwrap(), true);
        }
    );
    result.unwrap();
}

#[tokio::test(start_paused = true)]
async fn missing_resume_acknowledgement_is_not_success() {
    let edge = EdgeRegistry::default();
    let platform = Uuid::now_v7();
    let (session, _receiver) = edge
        .register(EdgeTarget::node(platform, "worker".into()), Uuid::now_v7())
        .unwrap();
    let started = tokio::time::Instant::now();
    let result = resume_nodes(
        &edge,
        platform,
        &["worker".into()],
        Uuid::now_v7(),
        &CancellationToken::new(),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(started.elapsed(), std::time::Duration::from_secs(5));
    assert_eq!(session.pending_count(), 0);
}

#[tokio::test]
async fn cleanup_attempts_reachable_nodes_even_when_another_node_is_offline() {
    let edge = EdgeRegistry::default();
    let platform = Uuid::now_v7();
    let operation = Uuid::now_v7();
    let (session, mut receiver) = edge
        .register(EdgeTarget::node(platform, "online".into()), Uuid::now_v7())
        .unwrap();
    let cancel = CancellationToken::new();
    let nodes = ["offline".into(), "online".into()];
    let (result, ()) = tokio::join!(
        resume_nodes(&edge, platform, &nodes, operation, &cancel),
        async {
            let command = tokio::time::timeout(std::time::Duration::from_secs(1), receiver.recv())
                .await
                .unwrap()
                .unwrap();
            session.complete(Uuid::parse_str(&command.command_id).unwrap(), true);
        }
    );
    assert!(result.is_err());
    assert_eq!(session.pending_count(), 0);
}
