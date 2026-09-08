#![cfg(unix)]

use citadel_adapters::agent::{AgentClient, AgentRequestSigner};
use citadel_execution::{ProcessLimits, ProcessRequest, run};
use citadel_platforms::{
    RuntimeErrorKind,
    containers::ContainerInspectionPort,
    logs::{LogReadPort, LogResource},
    terminal::{ContainerTerminalPort, TerminalInput, TerminalOutput, TerminalShell},
};
use futures_util::{FutureExt, StreamExt};
use std::{panic::AssertUnwindSafe, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

async fn docker(args: &[&str]) -> Vec<u8> {
    let output = run(
        ProcessRequest::new("docker")
            .args(args.iter().copied())
            .limits(ProcessLimits {
                timeout: Duration::from_secs(60),
                ..Default::default()
            }),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(
        output.succeeded(),
        "Docker fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

// Real published .NET Agent, not a protobuf test peer. The fixture creates only
// uniquely named containers and never prunes or changes existing workloads.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE7_AGENT_IMAGE and CITADEL_PHASE7_AGENT_NETWORK; run Test-Phase7AgentCandidate.ps1"]
async fn published_agent_authenticates_bounded_logs_and_interactive_terminal() {
    let image = std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap();
    let network = std::env::var("CITADEL_PHASE7_AGENT_NETWORK").unwrap();
    let suffix = Uuid::now_v7().simple().to_string();
    let agent = format!("citadel-acceptance-agent-{suffix}");
    let workload = format!("citadel-acceptance-terminal-{suffix}");
    let mut key = [0; 32];
    getrandom::fill(&mut key).unwrap();
    let signer = AgentRequestSigner::from_bytes(&key);
    key.fill(0);
    let public_key = format!("HUB_PUBLIC_KEY={}", signer.public_key_base64());
    let result = AssertUnwindSafe(async {
        docker(&[
            "run",
            "--detach",
            "--name",
            &agent,
            "--network",
            &network,
            "--mount",
            "type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock",
            "--env",
            &public_key,
            "--env",
            "CITADEL_AGENT_TLS_MODE=Disabled",
            &image,
        ])
        .await;
        let id = docker(&[
            "run",
            "--detach",
            "--name",
            &workload,
            "--env",
            "CITADEL_VAULT_SECRET=citadel-inspect-fixture-secret",
            "--env",
            "APP_MODE=production",
            "--entrypoint",
            "/bin/sh",
            "restic/restic:0.18.1",
            "-c",
            "echo citadel-agent-log-check; sleep 300",
        ])
        .await;
        let id = String::from_utf8(id).unwrap().trim().to_owned();
        let address = format!("http://{agent}:9000");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        let client = loop {
            if let Ok(client) =
                // The existing Agent handshake samples running-container statistics
                // in batches of four; use the production RPC deadline, not five
                // seconds regardless of how many host containers are running.
                AgentClient::connect(&address, signer.clone(), Duration::from_secs(30), true)
                        .await
                && client.handshake(&CancellationToken::new()).await.is_ok()
            {
                break client;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "Agent candidate did not become ready"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        };
        let bad = AgentClient::connect(
            &address,
            AgentRequestSigner::from_bytes(&[9; 32]),
            Duration::from_secs(3),
            true,
        )
        .await
        .unwrap();
        assert_eq!(
            bad.handshake(&CancellationToken::new())
                .await
                .unwrap_err()
                .kind,
            RuntimeErrorKind::Authentication
        );
        let cancel = CancellationToken::new();
        // RegularAgentCompatibilityTests inspects the deployed runtime container;
        // also retain VaultKvV2CompatibilityTests' no-plaintext inspection assertion.
        let container = client.inspection(&id, &cancel).await.unwrap();
        assert_eq!(container["id"], id);
        assert_eq!(container["state"]["status"], "Running");
        assert_eq!(container["config"]["image"], "restic/restic:0.18.1");
        let env = container["config"]["env"].as_array().unwrap();
        assert!(env.contains(&serde_json::json!("CITADEL_VAULT_SECRET=********")));
        assert!(env.contains(&serde_json::json!("APP_MODE=production")));
        assert!(
            !container
                .to_string()
                .contains("citadel-inspect-fixture-secret")
        );
        assert_eq!(
            bad.inspection(&id, &cancel).await.unwrap_err().kind,
            RuntimeErrorKind::Authentication
        );
        let inspected = citadel_platforms::images::ImageInspectionPort::inspect_image(
            &client,
            "restic/restic:0.18.1",
            &cancel,
        )
        .await
        .unwrap();
        assert!(inspected.id.starts_with("sha256:"));
        assert!(
            inspected
                .containers
                .iter()
                .any(|container| container.id == id)
        );
        assert!(!inspected.layers.is_empty());
        assert_eq!(
            citadel_platforms::images::ImageInspectionPort::inspect_image(
                &bad,
                "restic/restic:0.18.1",
                &cancel
            )
            .await
            .unwrap_err()
            .kind,
            RuntimeErrorKind::Authentication
        );
        let logs = client
            .read_logs(LogResource::Container(&id), 10, &cancel)
            .await
            .unwrap();
        assert!(
            logs.lines
                .iter()
                .any(|line| line.contains("citadel-agent-log-check")),
            "{logs:?}"
        );
        assert!(!logs.truncated);
        let mut terminal = client
            .container_terminal(&id, TerminalShell::Sh, &cancel)
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
                b"printf 'citadel-exec-check\\n'\n".to_vec(),
            ))
            .unwrap();
        let mut output = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(item) = terminal.output.next().await {
                if let TerminalOutput::Data(bytes) = item.unwrap() {
                    output.extend(bytes);
                    assert!(output.len() < 64 * 1024);
                    if String::from_utf8_lossy(&output).contains("citadel-exec-check\r\n") {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
        assert!(String::from_utf8_lossy(&output).contains("citadel-exec-check\r\n"));
        cancel.cancel();
        assert!(
            tokio::time::timeout(Duration::from_secs(3), terminal.output.next())
                .await
                .unwrap()
                .is_none()
        );
    })
    .catch_unwind()
    .await;
    if result.is_err() {
        let logs = run(
            ProcessRequest::new("docker").args(["logs", "--tail", "30", &agent]),
            &CancellationToken::new(),
        )
        .await;
        if let Ok(logs) = logs {
            eprintln!(
                "Agent fixture: {}{}",
                String::from_utf8_lossy(&logs.stdout),
                String::from_utf8_lossy(&logs.stderr)
            );
        }
    }
    for name in [&workload, &agent] {
        let _ = run(
            ProcessRequest::new("docker")
                .args(["rm", "--force", "--volumes", name])
                .limits(ProcessLimits {
                    timeout: Duration::from_secs(30),
                    ..Default::default()
                }),
            &CancellationToken::new(),
        )
        .await;
    }
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}
