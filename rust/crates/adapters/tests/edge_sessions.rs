use std::{sync::Arc, time::Duration};

use citadel_adapters::edge::{EdgeRegistry, EdgeTarget};
use citadel_contracts::citadel::edge::v1::{EdgeCommandKind, core_envelope};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn concurrency_limits_and_full_cancel_queue_close_the_session() {
    let registry = EdgeRegistry::default();
    let (session, _outbound) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let mut commands = Vec::new();
    for _ in 0..16 {
        commands.push(
            session
                .command(
                    EdgeCommandKind::ContainerInspect,
                    vec![],
                    Duration::from_secs(10),
                    false,
                )
                .unwrap(),
        );
    }
    assert!(
        session
            .command(
                EdgeCommandKind::ContainerInspect,
                vec![],
                Duration::from_secs(10),
                false
            )
            .is_err()
    );
    let last = commands.pop().unwrap();
    drop(commands); // 16 sends + 15 cancels = 31 queued frames.
    let extra = session
        .command(
            EdgeCommandKind::ContainerInspect,
            vec![],
            Duration::from_secs(10),
            false,
        )
        .unwrap();
    drop(last); // Cancellation cannot be delivered; remote work must terminate.
    assert!(session.is_closed());
    assert_eq!(session.pending_count(), 0);
    drop(extra);
}

#[tokio::test]
async fn outbound_payload_budget_is_shared_between_sessions() {
    let registry = EdgeRegistry::default();
    let (first, receiver1) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let (second, receiver2) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let one = first
        .command(
            EdgeCommandKind::ImageBuildStream,
            vec![0; 16 * 1024 * 1024],
            Duration::from_secs(10),
            true,
        )
        .unwrap();
    let two = second
        .command(
            EdgeCommandKind::ImageBuildStream,
            vec![0; 16 * 1024 * 1024],
            Duration::from_secs(10),
            true,
        )
        .unwrap();
    assert!(
        first
            .command(
                EdgeCommandKind::ContainerInspect,
                vec![0],
                Duration::from_secs(10),
                false
            )
            .is_err()
    );
    drop(one);
    drop(two);
    drop(receiver1);
    drop(receiver2);
}

// Ports EdgeAgentSessionRegistryTests and EdgeAgentCommandRouterTests. The
// registry is supplied only identities already authenticated by the intake.
#[tokio::test]
async fn reconnect_replaces_only_its_node_and_late_disconnect_keeps_replacement() {
    let registry = EdgeRegistry::default();
    let platform = Uuid::now_v7();
    let agent = Uuid::now_v7();
    let target = EdgeTarget::node(platform, "node-1".into());
    let (old, _) = registry.register(target.clone(), agent).unwrap();
    let (other, _) = registry
        .register(EdgeTarget::node(platform, "node-2".into()), Uuid::now_v7())
        .unwrap();
    assert!(registry.register(target.clone(), Uuid::now_v7()).is_err());
    let (new, _) = registry.register(target.clone(), agent).unwrap();
    assert!(old.is_closed());
    assert!(!registry.remove(&old));
    assert!(Arc::ptr_eq(&registry.get(&target).unwrap(), &new));
    assert!(!other.is_closed());
    assert!(registry.remove(&new));
    assert!(registry.get(&target).is_err());
}

#[tokio::test]
async fn command_drop_delivers_cancel_and_releases_slot() {
    let registry = EdgeRegistry::default();
    let (session, mut outbound) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let pending = session
        .command(
            EdgeCommandKind::ContainerInspect,
            vec![],
            Duration::from_secs(1),
            false,
        )
        .unwrap();
    let command = outbound.recv().await.unwrap();
    drop(pending);
    let cancel = outbound.recv().await.unwrap();
    assert_eq!(command.command_id, cancel.command_id);
    assert!(matches!(
        cancel.body,
        Some(core_envelope::Body::CancelCommand(_))
    ));
    assert_eq!(session.pending_count(), 0);
}

#[tokio::test]
async fn timeout_and_cancellation_release_pending_without_retrying_mutations() {
    let registry = EdgeRegistry::default();
    let (session, mut outbound) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let mut pending = session
        .command(
            EdgeCommandKind::ContainerCreate,
            vec![],
            Duration::from_millis(1),
            false,
        )
        .unwrap();
    assert!(pending.next(&CancellationToken::new()).await.is_err());
    assert_eq!(session.pending_count(), 0);
    assert!(matches!(
        outbound.recv().await.unwrap().body,
        Some(core_envelope::Body::Command(_))
    ));
    assert!(matches!(
        outbound.recv().await.unwrap().body,
        Some(core_envelope::Body::CancelCommand(_))
    ));
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut pending = session
        .command(
            EdgeCommandKind::ContainerCreate,
            vec![],
            Duration::from_secs(10),
            false,
        )
        .unwrap();
    assert!(pending.next(&cancellation).await.is_err());
    assert_eq!(session.pending_count(), 0);
}

#[tokio::test]
async fn output_overflow_and_disconnect_fail_commands_and_bound_memory() {
    let registry = EdgeRegistry::default();
    let (session, _outbound) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let mut pending = session
        .command(
            EdgeCommandKind::ContainerLogsStream,
            vec![],
            Duration::from_secs(5),
            true,
        )
        .unwrap();
    let id = pending.id();
    for _ in 0..17 {
        session.output(id, vec![1]);
    }
    assert!(pending.next(&CancellationToken::new()).await.is_err());
    assert_eq!(session.pending_count(), 0);
    let mut pending = session
        .command(
            EdgeCommandKind::ContainerInspect,
            vec![],
            Duration::from_secs(5),
            false,
        )
        .unwrap();
    session.close();
    assert!(pending.next(&CancellationToken::new()).await.is_err());
    assert_eq!(session.pending_count(), 0);
}

#[tokio::test]
async fn node_routing_is_exact_and_rejects_manager_commands() {
    let registry = EdgeRegistry::default();
    let platform = Uuid::now_v7();
    let (node, _outbound) = registry
        .register(EdgeTarget::node(platform, "node-1".into()), Uuid::now_v7())
        .unwrap();
    assert!(
        registry
            .get(&EdgeTarget::node(platform, "node-2".into()))
            .is_err()
    );
    assert!(registry.get(&EdgeTarget::platform(platform)).is_err());
    assert!(
        node.command(
            EdgeCommandKind::SwarmNodeUpdate,
            vec![],
            Duration::from_secs(1),
            false
        )
        .is_err()
    );
    assert!(
        node.command(
            EdgeCommandKind::ImageDelete,
            vec![],
            Duration::from_secs(1),
            false
        )
        .is_err()
    );
    assert!(
        node.command(
            EdgeCommandKind::ContainerExecBinary,
            vec![],
            Duration::from_secs(1),
            true
        )
        .is_ok()
    );
}
