use std::{sync::Arc, time::Duration};

use citadel_adapters::connectors::edge::EdgeRegistry;
use citadel_adapters::connectors::edge::EdgeRuntime;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_contracts::citadel::containers::v1::{
    ExecClientMessage, ExecOutput, ExecServerMessage, exec_client_message, exec_server_message,
};
use citadel_contracts::citadel::edge::v1::{EdgeCommandKind, core_envelope};
use citadel_platforms::terminal::*;
use futures_util::StreamExt;
use prost::Message;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[test]
fn disconnect_platform_closes_manager_and_nodes_but_preserves_other_targets() {
    let registry = EdgeRegistry::default();
    let platform = Uuid::now_v7();
    let (manager, _) = registry
        .register(EdgeTarget::platform(platform), Uuid::now_v7())
        .unwrap();
    let (node, _) = registry
        .register(
            EdgeTarget::node(platform, "worker-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (other, _) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let (builder, _) = registry
        .register(EdgeTarget::build_pool(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    registry.disconnect_platform(platform);
    registry.disconnect_platform(platform);
    assert!(manager.is_closed());
    assert!(node.is_closed());
    assert!(!other.is_closed());
    assert!(!builder.is_closed());
    assert_eq!(registry.current_sessions().len(), 2);
}

#[tokio::test]
async fn image_deletion_preserves_options_and_does_not_replay_a_failed_command() {
    use citadel_contracts::citadel::images::v1::{
        DeleteImageRequest, DeleteImageResponse, DeleteImageResponseItem,
    };
    use citadel_platforms::images::ImageDeletionPort;
    let registry = EdgeRegistry::default();
    let (session, mut outbound) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let cancel = CancellationToken::new();
    for success in [true, false] {
        let (result, ()) = tokio::join!(
            runtime.delete_image("sha256:abc", true, true, &cancel),
            async {
                let envelope = outbound.recv().await.unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("expected deletion command")
                };
                assert_eq!(command.kind, EdgeCommandKind::ImageDelete as i32);
                let request = DeleteImageRequest::decode(command.payload.as_slice()).unwrap();
                assert_eq!(request.ids, ["sha256:abc"]);
                assert!(request.force && request.noprune);
                let id = Uuid::parse_str(&command.command_id).unwrap();
                if success {
                    session.output(
                        id,
                        DeleteImageResponse {
                            items: vec![DeleteImageResponseItem {
                                result: [("Deleted".into(), "sha256:abc".into())].into(),
                            }],
                        }
                        .encode_to_vec(),
                    );
                }
                session.complete(id, success);
            }
        );
        if success {
            assert_eq!(result.unwrap()[0]["Deleted"], "sha256:abc");
        } else {
            assert!(result.is_err());
        }
        while let Ok(envelope) = outbound.try_recv() {
            assert!(
                !matches!(envelope.body, Some(core_envelope::Body::Command(_))),
                "destructive command was replayed"
            );
        }
    }
}

#[tokio::test]
async fn service_log_reads_preserve_tail_truncation_and_reject_oversized_agent_output() {
    use citadel_contracts::citadel::swarm::v1::{SwarmLogsRequest, SwarmLogsResponse};
    use citadel_platforms::logs::{LogReadPort, LogResource, MAX_LOG_FRAME};
    let registry = EdgeRegistry::default();
    let (session, mut outbound) = registry
        .register(EdgeTarget::platform(Uuid::now_v7()), Uuid::now_v7())
        .unwrap();
    let runtime = EdgeRuntime {
        session: session.clone(),
    };
    let cancel = CancellationToken::new();
    for oversized in [false, true] {
        let (result, ()) = tokio::join!(
            runtime.read_logs(LogResource::Service("service-1"), 25, &cancel),
            async {
                let envelope = outbound.recv().await.unwrap();
                let Some(core_envelope::Body::Command(command)) = envelope.body else {
                    panic!("logs command")
                };
                assert_eq!(command.kind, EdgeCommandKind::SwarmServiceLogs as i32);
                let request = SwarmLogsRequest::decode(command.payload.as_slice()).unwrap();
                assert_eq!(request.resource_id, "service-1");
                assert_eq!(request.tail, 25);
                let id = Uuid::parse_str(&command.command_id).unwrap();
                session.output(
                    id,
                    SwarmLogsResponse {
                        lines: vec![if oversized {
                            "x".repeat(MAX_LOG_FRAME + 1)
                        } else {
                            "service output".into()
                        }],
                        truncated: true,
                    }
                    .encode_to_vec(),
                );
                session.complete(id, true);
            }
        );
        if oversized {
            assert!(result.is_err());
        } else {
            let result = result.unwrap();
            assert_eq!(result.lines, ["service output"]);
            assert!(result.truncated);
        }
    }
}

// Verify the Edge Agent interactive-session contract and
// ExecSessionManagerTests disposal requirement; no manager fallback is possible.
#[tokio::test]
async fn terminal_routes_open_input_resize_to_exact_node_and_drop_cancels() {
    let registry = EdgeRegistry::default();
    let platform = Uuid::now_v7();
    let (session, mut outbound) = registry
        .register(
            EdgeTarget::node(platform, "worker-1".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let (_, mut other) = registry
        .register(
            EdgeTarget::node(platform, "worker-2".into()),
            Uuid::now_v7(),
        )
        .unwrap();
    let mut terminal = EdgeRuntime {
        session: session.clone(),
    }
    .container_terminal(
        "task-container",
        TerminalShell::Sh,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let command = outbound.recv().await.unwrap();
    let command_id = Uuid::parse_str(&command.command_id).unwrap();
    let Some(core_envelope::Body::Command(open)) = command.body else {
        panic!("command")
    };
    assert_eq!(open.node_id, "worker-1");
    assert_eq!(open.kind, EdgeCommandKind::ContainerExec as i32);
    let open = ExecClientMessage::decode(open.payload.as_slice()).unwrap();
    let Some(exec_client_message::Msg::Open(open)) = open.msg else {
        panic!("Open")
    };
    assert_eq!(open.container_id, "task-container");
    assert_eq!(open.cmd, ["/bin/sh"]);
    for input in [
        TerminalInput::Stdin(b"ls\n".to_vec()),
        TerminalInput::Resize { cols: 80, rows: 24 },
    ] {
        let resize = matches!(input, TerminalInput::Resize { .. });
        terminal.input.try_send(input).unwrap();
        let peer = async {
            let sent = outbound.recv().await.unwrap();
            assert_eq!(sent.command_id, command.command_id);
            let Some(core_envelope::Body::StreamInput(bytes)) = sent.body else {
                panic!("StreamInput")
            };
            let message = ExecClientMessage::decode(bytes.payload.as_slice()).unwrap();
            if resize {
                assert!(
                    matches!(message.msg,Some(exec_client_message::Msg::Resize(r)) if r.cols==80 && r.rows==24)
                );
            } else {
                assert!(
                    matches!(message.msg,Some(exec_client_message::Msg::Stdin(s)) if s.data==b"ls\n")
                );
            }
            session.output(
                command_id,
                ExecServerMessage {
                    msg: Some(exec_server_message::Msg::Output(ExecOutput {
                        data: b"ok".to_vec(),
                        stream: 0,
                    })),
                }
                .encode_to_vec(),
            );
        };
        let (_, output) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(peer, terminal.output.next())
        })
        .await
        .unwrap();
        assert!(matches!(output.unwrap().unwrap(),TerminalOutput::Data(bytes) if bytes==b"ok"));
    }
    assert!(other.try_recv().is_err());
    drop(terminal.output);
    assert!(matches!(
        outbound.recv().await.unwrap().body,
        Some(core_envelope::Body::CancelCommand(_))
    ));
    assert_eq!(session.pending_count(), 0);
    assert!(
        terminal
            .input
            .try_send(TerminalInput::Stdin(vec![1]))
            .is_err()
    );
}

#[tokio::test]
async fn terminal_reconnect_cancels_old_command_without_sending_input_to_replacement() {
    let registry = EdgeRegistry::default();
    let target = EdgeTarget::node(Uuid::now_v7(), "worker".into());
    let agent = Uuid::now_v7();
    let (old, mut outbound) = registry.register(target.clone(), agent).unwrap();
    let mut terminal = EdgeRuntime { session: old }
        .container_terminal("task", TerminalShell::Bash, &CancellationToken::new())
        .await
        .unwrap();
    outbound.recv().await.unwrap();
    let (_, mut replacement) = registry.register(target, agent).unwrap();
    terminal
        .input
        .try_send(TerminalInput::Stdin(vec![1]))
        .unwrap();
    assert!(terminal.output.next().await.unwrap().is_err());
    assert!(replacement.try_recv().is_err());
}

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
