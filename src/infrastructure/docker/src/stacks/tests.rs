use super::*;
use std::time::Duration;

#[test]
fn transported_stack_paths_cannot_escape_the_staging_root() {
    let root = Path::new("/tmp/citadel-stack-source");
    assert!(resolve_staged_path(root, "compose.yml").is_ok());
    assert!(resolve_staged_path(root, "apps/web/compose.yml").is_ok());
    assert!(resolve_staged_path(root, "../compose.yml").is_err());
    assert!(resolve_staged_path(root, "/compose.yml").is_err());
    assert!(resolve_staged_path(root, "").is_err());
}

#[test]
fn compose_arguments_preserve_source_file_and_environment_order() {
    let args = compose_args(
        &[PathBuf::from("base.yml"), PathBuf::from("prod.yml")],
        Some(Path::new("citadel.labels.yml")),
        &[PathBuf::from("repo.env"), PathBuf::from("resolved.env")],
        "demo",
        &["up", "-d"],
    );
    assert_eq!(
        args,
        [
            "compose",
            "-p",
            "demo",
            "-f",
            "base.yml",
            "-f",
            "prod.yml",
            "-f",
            "citadel.labels.yml",
            "--env-file",
            "repo.env",
            "--env-file",
            "resolved.env",
            "up",
            "-d",
        ]
    );
}

// ProcessRunner/StackService yield stdout and stderr before command exit.
#[cfg(unix)]
#[tokio::test]
async fn process_streams_both_pipes_before_exit_and_preserves_failure() {
    let root = std::env::temp_dir().join(format!("citadel-stack-stream-{}", Uuid::now_v7()));
    tokio::fs::create_dir_all(&root).await.unwrap();
    let cancellation = CancellationToken::new();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let progress = citadel_stacks::StackProgress::new(
        sender,
        vec!["private-token".into()],
        cancellation.clone(),
    );
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg("printf 'stdout private-token\\n'; printf 'stderr ready\\r' >&2; while [ ! -f release ]; do sleep 0.02; done; printf 'last line'; exit 7")
            .current_dir(&root).stdout(Stdio::piped()).stderr(Stdio::piped());
    let process = collect_process(command, &cancellation, Some(&progress));
    let observe = async {
        let first = receiver.recv().await.unwrap();
        let second = receiver.recv().await.unwrap();
        assert!(
            [&first, &second]
                .iter()
                .any(|item| item.event_type == StackApplyEventType::StdOut)
        );
        assert!(
            [&first, &second]
                .iter()
                .any(|item| item.event_type == StackApplyEventType::StdErr)
        );
        assert!(
            [&first, &second].iter().all(|item| !item
                .message
                .as_deref()
                .unwrap()
                .contains("private-token"))
        );
        // The process cannot finish until progress is received by the caller.
        tokio::fs::write(root.join("release"), b"").await.unwrap();
        assert_eq!(
            receiver.recv().await.unwrap().message.as_deref(),
            Some("last line")
        );
        assert_eq!(receiver.recv().await.unwrap().exit_code, Some(7));
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(process, observe)
    })
    .await
    .unwrap();
    assert_eq!(result.unwrap().status, StackReleaseStatus::Failed);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn secrets_split_across_pipe_reads_are_redacted_before_streaming() {
    use tokio::io::AsyncWriteExt;
    let (mut writer, reader) = tokio::io::duplex(4);
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let progress = citadel_stacks::StackProgress::new(
        sender,
        vec!["private-token".into()],
        CancellationToken::new(),
    );
    let read = read_process_output(reader, StackApplyEventType::StdOut, Some(&progress));
    let write = async {
        writer.write_all(b"value private-token\n").await.unwrap();
        drop(writer);
    };
    let (result, ()) =
        tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(read, write) })
            .await
            .unwrap();
    result.unwrap();
    let text = receiver.recv().await.unwrap().message.unwrap();
    assert!(text.starts_with("value "));
    assert!(!text.contains("private-token"));
}

#[tokio::test]
async fn output_streams_past_retention_limit_and_discards_oversized_lines() {
    let cancellation = CancellationToken::new();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let progress = citadel_stacks::StackProgress::new(sender, vec![], cancellation);
    let mut bytes = vec![b'x'; MAX_PROCESS_OUTPUT_BYTES + 1];
    bytes.extend_from_slice(b"\n");
    for _ in 0..300 {
        bytes.extend_from_slice(format!("{}\n", "y".repeat(1024)).as_bytes());
    }
    bytes.extend_from_slice(b"last");
    let read = read_process_output(
        bytes.as_slice(),
        StackApplyEventType::StdErr,
        Some(&progress),
    );
    let observe = async {
        for _ in 0..300 {
            assert_eq!(
                receiver.recv().await.unwrap().message.unwrap(),
                "y".repeat(1024)
            );
        }
        assert_eq!(
            receiver.recv().await.unwrap().message.as_deref(),
            Some("last")
        );
    };
    let (retained, ()) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(read, observe)
    })
    .await
    .unwrap();
    let retained = retained.unwrap();
    assert!(retained.len() < 300);
    assert!(
        retained
            .iter()
            .map(|item| item.message.as_ref().unwrap().len() + 64)
            .sum::<usize>()
            <= MAX_PROCESS_OUTPUT_BYTES
    );
}

#[cfg(unix)]
#[tokio::test]
async fn cancellation_interrupts_a_process_with_a_full_progress_queue() {
    let cancellation = CancellationToken::new();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let progress = citadel_stacks::StackProgress::new(sender, vec![], cancellation.clone());
    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg("printf '%s\\n' \"$$\"; while :; do printf 'progress\\n'; done")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let process = collect_process(command, &cancellation, Some(&progress));
    let cancel = async {
        let pid: u32 = receiver
            .recv()
            .await
            .unwrap()
            .message
            .unwrap()
            .parse()
            .unwrap();
        cancellation.cancel();
        pid
    };
    let (result, _pid) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(process, cancel)
    })
    .await
    .unwrap();
    assert!(matches!(result, Err(StackError::Cancelled)));
    #[cfg(target_os = "linux")]
    assert!(
        !std::path::Path::new(&format!("/proc/{_pid}")).exists(),
        "child must already be reaped"
    );
}

#[test]
fn shell_commands_are_passed_as_one_argument_to_the_platform_shell() {
    let (_, arguments) = shell_invocation("printf '%s' \"$TOKEN\"");
    assert_eq!(
        arguments.last().map(String::as_str),
        Some("printf '%s' \"$TOKEN\"")
    );
}

#[test]
fn docker_configuration_is_a_global_cli_option() {
    let mut arguments = vec!["stack".to_owned(), "deploy".to_owned()];
    prepend_docker_config(&mut arguments, Some(Path::new("/tmp/docker-config")));
    assert_eq!(
        arguments,
        ["--config", "/tmp/docker-config", "stack", "deploy"]
    );
}

#[tokio::test]
async fn stack_commands_use_the_staged_directory_and_resolved_environment() {
    let root = std::env::temp_dir().join(format!(
        "citadel-stack-command-test-{}",
        Uuid::now_v7().simple()
    ));
    tokio::fs::create_dir_all(&root).await.unwrap();
    let command = if cfg!(windows) {
        "echo %CITADEL_TEST_VALUE%>marker.txt"
    } else {
        "printf %s \"$CITADEL_TEST_VALUE\" > marker.txt"
    };
    let result = run_stack_commands(
        &StackCommand {
            commands: vec![command.to_owned()],
            path: ".".to_owned(),
        },
        &root,
        &["CITADEL_TEST_VALUE=resolved".to_owned()],
        &CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.status, StackReleaseStatus::Healthy);
    assert_eq!(
        tokio::fs::read_to_string(root.join("marker.txt"))
            .await
            .unwrap()
            .trim(),
        "resolved"
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[test]
fn compose_environment_helper() {
    if std::env::var_os("CITADEL_COMPOSE_ENV_HELPER").is_none() {
        return;
    }
    let root = temporary_run_root(Uuid::now_v7());
    std::fs::create_dir_all(&root).unwrap();
    let _cleanup = TransientDirectory(root.clone());
    std::fs::write(root.join("compose.yml"), "services:\n  fixture:\n    image: scratch\n    environment:\n      PARENT: ${CITADEL_TEST_CORE_ONLY_SECRET:-absent}\n      WORKLOAD: ${CITADEL_WORKLOAD}\n").unwrap();
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run_process(
            "docker",
            &[
                "compose".into(),
                "-f".into(),
                "compose.yml".into(),
                "config".into(),
            ],
            &root,
            &["CITADEL_WORKLOAD=explicit-value".into()],
            &CancellationToken::new(),
            None,
        ))
        .unwrap();
    assert_eq!(result.status, StackReleaseStatus::Healthy);
    let output = result
        .messages
        .iter()
        .filter_map(|item| item.message.as_deref())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(output.contains("explicit-value"), "{output}");
    assert!(output.contains("absent"), "{output}");
    assert!(!output.contains("parent-only-value"), "{output}");
}

#[test]
#[ignore = "requires docker compose executable; no daemon needed"]
fn compose_interpolation_excludes_core_secrets() {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "stacks::tests::compose_environment_helper",
            "--nocapture",
        ])
        .env("CITADEL_COMPOSE_ENV_HELPER", "1")
        .env("CITADEL_TEST_CORE_ONLY_SECRET", "parent-only-value")
        .status()
        .unwrap();
    assert!(status.success());
}
