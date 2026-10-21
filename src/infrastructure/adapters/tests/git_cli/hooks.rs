use super::*;
use citadel_git::RepoCommand;

fn hook(commands: &[&str], path: &str) -> RepoCommand {
    RepoCommand {
        commands: commands.iter().map(|value| (*value).into()).collect(),
        path: path.into(),
    }
}

#[tokio::test]
async fn hook_commands_run_in_order_with_relative_directory_and_stop_without_disclosing_output() {
    let repo = TemporaryGitRepository::new();
    std::fs::create_dir(repo.path().join("scripts")).unwrap();
    let (cli, _) = sync_cli();
    cli.run_hook(
        repo.path(),
        &hook(
            &["printf first > result", "printf second >> result"],
            "scripts",
        ),
        "On Clone",
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("scripts/result")).unwrap(),
        "firstsecond"
    );
    let error = cli
        .run_hook(
            repo.path(),
            &hook(
                &[
                    "printf secret-output; printf secret-error >&2; exit 7",
                    "touch should-not-run",
                ],
                ".",
            ),
            "On Pull",
            &CancellationToken::new(),
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("On Pull command 1"), "{error}");
    assert!(error.contains('7'), "{error}");
    assert!(!error.contains("secret"), "{error}");
    assert!(!repo.path().join("should-not-run").exists());
}

#[tokio::test]
async fn hook_directories_reject_traversal_missing_paths_files_and_external_symlinks() {
    let repo = TemporaryGitRepository::new();
    let outside = TemporaryGitRepository::new();
    std::fs::write(repo.path().join("file"), "data").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), repo.path().join("escape")).unwrap();
    let (cli, process) = sync_cli();
    for path in [
        "../", "/tmp", "missing", "file", "escape", "C:/temp", "a/../b",
    ] {
        assert!(
            cli.run_hook(
                repo.path(),
                &hook(&["touch should-not-run"], path),
                "On Clone",
                &CancellationToken::new()
            )
            .await
            .is_err(),
            "{path}"
        );
    }
    assert!(process.calls.lock().unwrap().is_empty());
    assert!(!outside.path().join("should-not-run").exists());
}

#[tokio::test]
async fn hook_timeout_cancellation_and_output_limits_stop_later_commands() {
    let repo = TemporaryGitRepository::new();
    let cli = GitCli::new(
        Arc::new(citadel_processes::SystemProcess),
        Arc::new(
            citadel_adapters::filesystem::git_workspace::LocalGitWorkspace::new(
                citadel_runtime::DynamicTasks::new(CancellationToken::new()),
            ),
        ),
        Duration::from_millis(100),
    );
    let slow = hook(&["sleep 10", "touch should-not-run"], ".");
    let error = cli
        .run_hook(repo.path(), &slow, "On Pull", &CancellationToken::new())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("timed out"), "{error}");
    let cancel = CancellationToken::new();
    let execution = cli.run_hook(repo.path(), &slow, "On Pull", &cancel);
    let cancellation = async {
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(execution, cancellation);
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    let (cli, _) = sync_cli();
    let error = cli
        .run_hook(
            repo.path(),
            &hook(&["head -c 100000 /dev/zero", "touch should-not-run"], "."),
            "On Pull",
            &CancellationToken::new(),
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("output limit"), "{error}");
    assert!(!repo.path().join("should-not-run").exists());
}
