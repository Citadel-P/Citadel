use super::*;
use citadel_adapters::edge::EdgeRuntime;
use citadel_platforms::logs::{LogReadPort, LogResource};
use citadel_platforms::terminal::{
    ContainerTerminalPort, TerminalInput, TerminalOutput, TerminalShell,
};
use futures_util::StreamExt;

pub(super) async fn verify(
    cluster: &Cluster,
    pool: &sqlx::PgPool,
    agent: &AgentClient,
    platform: Uuid,
    nodes: &[String],
    registry: &EdgeRegistry,
) {
    let service_id = String::from_utf8(
        cluster
            .node(
                0,
                &[
                    "service",
                    "create",
                    "--detach=true",
                    "--no-resolve-image",
                    "--name",
                    "interactive-worker-fixture",
                    "--constraint",
                    &format!("node.id=={}", nodes[1]),
                    "--restart-condition",
                    "none",
                    "--entrypoint",
                    "sh",
                    IMAGE,
                    "-c",
                    "echo citadel-worker-log; sleep 300",
                ],
                None,
            )
            .await,
    )
    .unwrap()
    .trim()
    .to_owned();
    let cancel = CancellationToken::new();
    let task = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if let Some(task) = agent
                .list_swarm_tasks(&cancel)
                .await
                .unwrap()
                .into_iter()
                .find(|task| {
                    task.service_id == service_id && task.state.eq_ignore_ascii_case("running")
                })
            {
                break task;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await;
    if task.is_err() {
        let diagnostics = cluster
            .node(0, &["service", "ps", "--no-trunc", &service_id], None)
            .await;
        eprintln!(
            "Interactive Service did not run: {}",
            String::from_utf8_lossy(&diagnostics)
        );
        eprintln!("Agent Tasks: {:?}", agent.list_swarm_tasks(&cancel).await);
    }
    let task = task.unwrap();
    assert_eq!(task.node_id, nodes[1]);
    let container = task.container_id.unwrap();
    let runtime = EdgeRuntime {
        session: registry
            .get(&EdgeTarget::node(platform, nodes[1].clone()))
            .unwrap(),
    };
    let logs = runtime
        .read_logs(LogResource::Container(&container), 10, &cancel)
        .await
        .unwrap();
    assert!(
        logs.lines
            .iter()
            .any(|line| line.contains("citadel-worker-log")),
        "{logs:?}"
    );
    let other_worker = EdgeRuntime {
        session: registry
            .get(&EdgeTarget::node(platform, nodes[2].clone()))
            .unwrap(),
    };
    assert!(
        other_worker
            .read_logs(LogResource::Container(&container), 10, &cancel)
            .await
            .is_err(),
        "a task ID cannot be redirected to another worker"
    );
    let mut terminal = runtime
        .container_terminal(&container, TerminalShell::Sh, &cancel)
        .await
        .unwrap();
    terminal
        .input
        .try_send(TerminalInput::Resize {
            cols: 100,
            rows: 30,
        })
        .unwrap();
    terminal
        .input
        .try_send(TerminalInput::Stdin(
            b"printf '%s%s\\n' 'citadel-' 'worker-exec'\n".to_vec(),
        ))
        .unwrap();
    let mut output = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(item) = terminal.output.next().await {
            if let TerminalOutput::Data(bytes) = item.unwrap() {
                output.extend(bytes);
                assert!(output.len() < 64 * 1024);
                if String::from_utf8_lossy(&output).contains("citadel-worker-exec\r\n") {
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    assert!(String::from_utf8_lossy(&output).contains("citadel-worker-exec\r\n"));
    cancel.cancel();
    tokio::time::timeout(Duration::from_secs(5), async {
        while terminal.output.next().await.is_some() {}
    })
    .await
    .unwrap();
    cluster.node(0, &["service", "rm", &service_id], None).await;
    verify_container_actions(cluster, pool, agent, platform, nodes, registry).await;
}

async fn verify_container_actions(
    cluster: &Cluster,
    pool: &sqlx::PgPool,
    agent: &AgentClient,
    platform: Uuid,
    nodes: &[String],
    registry: &EdgeRegistry,
) {
    use citadel_adapters::{container_mutations::ContainerRuntimeRouter, docker::DockerClient};
    use citadel_platforms::containers::{
        ContainerAction, ContainerMutationRuntime, ContainerTarget, DeleteContainerOptions,
    };
    let container = String::from_utf8(
        cluster
            .node(
                1,
                &[
                    "run",
                    "--detach",
                    "--volume",
                    "/data",
                    "--entrypoint",
                    "sh",
                    IMAGE,
                    "-c",
                    "trap 'exit 0' TERM; while :; do sleep 1; done",
                ],
                None,
            )
            .await,
    )
    .unwrap()
    .trim()
    .to_owned();
    let volume = String::from_utf8(
        cluster
            .node(
                1,
                &[
                    "inspect",
                    "--format",
                    "{{(index .Mounts 0).Name}}",
                    &container,
                ],
                None,
            )
            .await,
    )
    .unwrap()
    .trim()
    .to_owned();
    let runtime = ContainerRuntimeRouter::new(
        pool.clone(),
        DockerClient::new("/no-core-docker-socket", Duration::from_secs(2)).unwrap(),
        Some(agent.clone()),
        registry.clone(),
    );
    let target = ContainerTarget {
        id: Uuid::now_v7(),
        platform_id: platform,
        docker_id: container,
        node_id: Some(nodes[1].clone()),
    };
    let cancel = CancellationToken::new();
    for (action, expected) in [
        (ContainerAction::Pause, "paused"),
        (ContainerAction::Unpause, "running"),
        (ContainerAction::Stop, "exited"),
        (ContainerAction::Start, "running"),
        (ContainerAction::Restart, "running"),
    ] {
        runtime.mutate(&target, action, &cancel).await.unwrap();
        assert!(
            runtime
                .observe(&target, &cancel)
                .await
                .unwrap()
                .unwrap()
                .eq_ignore_ascii_case(expected)
        );
    }
    runtime
        .mutate(&target, ContainerAction::Stop, &cancel)
        .await
        .unwrap();
    runtime
        .mutate(
            &target,
            ContainerAction::Delete(DeleteContainerOptions::default()),
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!(runtime.observe(&target, &cancel).await.unwrap(), None);
    // Normal Delete must not inherit backup-helper cleanup's v=true default.
    cluster.node(1, &["volume", "inspect", &volume], None).await;
    cluster.node(1, &["volume", "rm", &volume], None).await;
}
