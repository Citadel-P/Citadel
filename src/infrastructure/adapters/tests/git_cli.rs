use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use citadel_git::{GitChangedPathStatus, GitCli, GitEntryType};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
struct RecordingGitProcess {
    calls: Mutex<Vec<Vec<std::ffi::OsString>>>,
}

impl citadel_execution::ProcessRunner for RecordingGitProcess {
    fn run<'a>(
        &'a self,
        request: citadel_execution::ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> futures_util::future::BoxFuture<
        'a,
        Result<citadel_execution::ProcessOutput, citadel_execution::ProcessError>,
    > {
        self.calls.lock().unwrap().push(request.arguments.clone());
        Box::pin(async move {
            citadel_execution::ProcessRunner::run(
                &citadel_processes::SystemProcess,
                request,
                cancellation,
            )
            .await
        })
    }
}

fn sync_cli() -> (GitCli, Arc<RecordingGitProcess>) {
    let process = Arc::new(RecordingGitProcess::default());
    (
        GitCli::new(
            process.clone(),
            Arc::new(
                citadel_adapters::filesystem::git_workspace::LocalGitWorkspace::new(
                    citadel_runtime::DynamicTasks::new(CancellationToken::new()),
                ),
            ),
            Duration::from_secs(10),
        ),
        process,
    )
}

fn used_command(process: &RecordingGitProcess, command: &str) -> bool {
    process
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|args| args.iter().any(|arg| arg == command))
}

struct TemporaryGitRepository {
    path: PathBuf,
}

impl TemporaryGitRepository {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("citadel-git-browser-{}", Uuid::now_v7()));
        std::fs::create_dir_all(&path).unwrap();
        let repository = Self { path };
        repository.run(["init"]);
        repository.run(["config", "user.email", "citadel-tests@example.invalid"]);
        repository.run(["config", "user.name", "Citadel Tests"]);
        repository
    }

    fn run<const N: usize>(&self, arguments: [&str; N]) -> String {
        let output = Command::new("git")
            .current_dir(&self.path)
            .args(arguments)
            .output()
            .expect("Git must be installed for the execution gate");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn commit(&self, message: &str) -> String {
        self.run(["add", "-A"]);
        self.run(["commit", "-m", message]);
        self.run(["rev-parse", "HEAD"]).trim().to_owned()
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryGitRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[tokio::test]
async fn unchanged_sync_fetches_without_rewriting_the_worktree_and_changed_commits_still_apply() {
    let remote = TemporaryGitRepository::new();
    std::fs::write(remote.path().join("tracked.txt"), "original\n").unwrap();
    let initial = remote.commit("initial");
    remote.run(["branch", "-M", "main"]);
    let directory = TemporaryGitRepository::new();
    let cache = directory.path().join("cache");
    let (cli, process) = sync_cli();
    let cancel = CancellationToken::new();
    let url = remote.path().to_str().unwrap();
    let first = cli.synchronize(url, &cache, "main", &cancel).await.unwrap();
    assert!(first.cloned);
    assert_eq!(first.commit, initial);
    let head_log = std::fs::read(cache.join(".git/logs/HEAD")).unwrap();
    process.calls.lock().unwrap().clear();
    let unchanged = cli.synchronize(url, &cache, "main", &cancel).await.unwrap();
    assert!(!unchanged.cloned);
    assert_eq!(unchanged.commit, initial);
    assert!(used_command(&process, "fetch"));
    assert!(!used_command(&process, "checkout"));
    assert!(!used_command(&process, "reset"));
    assert_eq!(process.calls.lock().unwrap().len(), 5);
    assert_eq!(
        std::fs::read(cache.join(".git/logs/HEAD")).unwrap(),
        head_log
    );

    // Equal trees are not equal commits. An empty remote commit must advance HEAD.
    remote.run(["commit", "--allow-empty", "-m", "empty change"]);
    let empty_commit = remote.run(["rev-parse", "HEAD"]).trim().to_owned();
    process.calls.lock().unwrap().clear();
    assert_eq!(
        cli.synchronize(url, &cache, "main", &cancel)
            .await
            .unwrap()
            .commit,
        empty_commit
    );
    assert!(used_command(&process, "reset"));
    std::fs::write(remote.path().join("tracked.txt"), "updated\n").unwrap();
    let updated = remote.commit("update file");
    assert_eq!(
        cli.synchronize(url, &cache, "main", &cancel)
            .await
            .unwrap()
            .commit,
        updated
    );
    assert_eq!(
        std::fs::read_to_string(cache.join("tracked.txt")).unwrap(),
        "updated\n"
    );
    // Force-pushed branches must still move backwards to the fetched commit.
    remote.run(["reset", "--hard", &initial]);
    assert_eq!(
        cli.synchronize(url, &cache, "main", &cancel)
            .await
            .unwrap()
            .commit,
        initial
    );
    assert_eq!(
        std::fs::read_to_string(cache.join("tracked.txt")).unwrap(),
        "original\n"
    );
    let long_branch = "b".repeat(240);
    remote.run(["branch", &long_branch]);
    assert_eq!(
        cli.synchronize(url, &cache, &long_branch, &cancel)
            .await
            .unwrap()
            .commit,
        initial
    );
    process.calls.lock().unwrap().clear();
    cli.synchronize(url, &cache, &long_branch, &cancel)
        .await
        .unwrap();
    assert!(!used_command(&process, "reset"));
}

#[tokio::test]
async fn unchanged_commit_repairs_dirty_staged_deleted_and_wrong_branch_caches() {
    let remote = TemporaryGitRepository::new();
    std::fs::write(remote.path().join("tracked.txt"), "original\n").unwrap();
    let commit = remote.commit("initial");
    remote.run(["branch", "-M", "main"]);
    let directory = TemporaryGitRepository::new();
    let cache = TemporaryGitRepository {
        path: directory.path().join("cache"),
    };
    let (cli, process) = sync_cli();
    let cancel = CancellationToken::new();
    let url = remote.path().to_str().unwrap();
    cli.synchronize(url, cache.path(), "main", &cancel)
        .await
        .unwrap();
    for state in [
        "dirty",
        "staged",
        "deleted",
        "untracked",
        "branch",
        "detached",
    ] {
        match state {
            "dirty" => std::fs::write(cache.path().join("tracked.txt"), "dirty\n").unwrap(),
            "staged" => {
                std::fs::write(cache.path().join("tracked.txt"), "staged\n").unwrap();
                cache.run(["add", "tracked.txt"]);
            }
            "deleted" => std::fs::remove_file(cache.path().join("tracked.txt")).unwrap(),
            "untracked" => {
                cache.run(["config", "status.showUntrackedFiles", "no"]);
                std::fs::write(cache.path().join("untracked.txt"), "preserve\n").unwrap();
            }
            "branch" => {
                cache.run(["checkout", "-b", "other"]);
            }
            "detached" => {
                cache.run(["checkout", "--detach"]);
            }
            _ => unreachable!(),
        }
        process.calls.lock().unwrap().clear();
        let result = cli
            .synchronize(url, cache.path(), "main", &cancel)
            .await
            .unwrap();
        assert_eq!(result.commit, commit, "{state}");
        assert!(used_command(&process, "checkout"), "{state}");
        assert!(used_command(&process, "reset"), "{state}");
        assert_eq!(
            cache.run(["symbolic-ref", "--short", "HEAD"]).trim(),
            "main"
        );
        assert_eq!(
            std::fs::read_to_string(cache.path().join("tracked.txt")).unwrap(),
            "original\n"
        );
        assert!(cache.run(["diff", "--cached"]).is_empty());
        if state == "untracked" {
            // Preserve existing reset semantics: do not introduce git clean.
            assert_eq!(
                std::fs::read_to_string(cache.path().join("untracked.txt")).unwrap(),
                "preserve\n"
            );
            std::fs::remove_file(cache.path().join("untracked.txt")).unwrap();
        }
    }
}

#[tokio::test]
async fn unchanged_sync_repairs_files_hidden_by_assume_unchanged_and_ignorestat() {
    let remote = TemporaryGitRepository::new();
    std::fs::write(remote.path().join("tracked.txt"), "original\n").unwrap();
    let commit = remote.commit("initial");
    remote.run(["branch", "-M", "main"]);
    let directory = TemporaryGitRepository::new();
    let cache = TemporaryGitRepository {
        path: directory.path().join("cache"),
    };
    let (cli, process) = sync_cli();
    let cancel = CancellationToken::new();
    let url = remote.path().to_str().unwrap();
    cli.synchronize(url, cache.path(), "main", &cancel)
        .await
        .unwrap();

    for ignorestat in [false, true] {
        cache.run([
            "config",
            "core.ignorestat",
            if ignorestat { "true" } else { "false" },
        ]);
        for state in ["modified", "deleted", "clean"] {
            if ignorestat {
                // Git itself marks entries assume-unchanged with this setting.
                cache.run(["reset", "--hard", "HEAD"]);
                cache.run(["update-index", "--really-refresh"]);
            } else {
                cache.run(["update-index", "--assume-unchanged", "tracked.txt"]);
            }
            assert_eq!(cache.run(["ls-files", "-v"]), "h tracked.txt\n");
            let tracked = cache.path().join("tracked.txt");
            let times = std::fs::metadata(&tracked).unwrap();
            match state {
                "modified" => {
                    // Same-length edits with the original timestamp must still
                    // be repaired when Git trusts assume-unchanged entries.
                    std::fs::write(&tracked, "modified\n").unwrap();
                    std::fs::File::options()
                        .write(true)
                        .open(&tracked)
                        .unwrap()
                        .set_times(
                            std::fs::FileTimes::new()
                                .set_accessed(times.accessed().unwrap())
                                .set_modified(times.modified().unwrap()),
                        )
                        .unwrap();
                }
                "deleted" => std::fs::remove_file(cache.path().join("tracked.txt")).unwrap(),
                _ => {}
            }
            assert!(cache.run(["status", "--porcelain=v2"]).is_empty());
            process.calls.lock().unwrap().clear();
            let result = cli
                .synchronize(url, cache.path(), "main", &cancel)
                .await
                .unwrap();
            assert_eq!(result.commit, commit);
            assert_eq!(
                std::fs::read_to_string(cache.path().join("tracked.txt")).unwrap(),
                "original\n",
                "ignorestat={ignorestat}, {state}"
            );
            assert!(
                used_command(&process, "reset"),
                "ignorestat={ignorestat}, {state}"
            );
        }
    }
    cache.run(["config", "core.ignorestat", "false"]);
    cache.run(["update-index", "--no-assume-unchanged", "tracked.txt"]);
    process.calls.lock().unwrap().clear();
    cli.synchronize(url, cache.path(), "main", &cancel)
        .await
        .unwrap();
    assert!(!used_command(&process, "reset"));
}

#[tokio::test]
async fn failed_or_cancelled_fetch_does_not_report_cached_success_or_reset_files() {
    let remote = TemporaryGitRepository::new();
    std::fs::write(remote.path().join("tracked.txt"), "original\n").unwrap();
    remote.commit("initial");
    remote.run(["branch", "-M", "main"]);
    let directory = TemporaryGitRepository::new();
    let cache = directory.path().join("cache");
    let (cli, process) = sync_cli();
    let url = remote.path().to_str().unwrap();
    cli.synchronize(url, &cache, "main", &CancellationToken::new())
        .await
        .unwrap();
    std::fs::write(cache.join("tracked.txt"), "dirty\n").unwrap();
    process.calls.lock().unwrap().clear();
    assert!(
        cli.synchronize(url, &cache, "missing", &CancellationToken::new())
            .await
            .is_err()
    );
    assert!(!used_command(&process, "reset"));
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        cli.synchronize(url, &cache, "main", &cancel).await,
        Err(citadel_git::GitError::Process(
            citadel_execution::ProcessError::Cancelled
        ))
    ));
    assert_eq!(
        std::fs::read_to_string(cache.join("tracked.txt")).unwrap(),
        "dirty\n"
    );
}

#[tokio::test]
async fn browser_operations_read_immutable_git_objects() {
    let repository = TemporaryGitRepository::new();
    std::fs::create_dir(repository.path().join("config")).unwrap();
    std::fs::write(
        repository.path().join("compose.yaml"),
        "services:\n  web:\n    image: nginx\n",
    )
    .unwrap();
    std::fs::write(repository.path().join("config/binary.dat"), b"A\0B").unwrap();
    let first_commit = repository.commit("initial");
    let cli = GitCli::new(
        std::sync::Arc::new(citadel_processes::SystemProcess),
        std::sync::Arc::new(
            citadel_adapters::filesystem::git_workspace::LocalGitWorkspace::new(
                citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
            ),
        ),
        Duration::from_secs(10),
    );
    let cancellation = CancellationToken::new();

    let resolved = cli
        .resolve_named_commit(repository.path(), &first_commit, &cancellation)
        .await
        .unwrap();
    assert_eq!(resolved, first_commit);

    let root = cli
        .list_tree(
            repository.path(),
            &first_commit,
            "",
            100,
            1024 * 1024,
            &cancellation,
        )
        .await
        .unwrap();
    assert!(
        root.entries
            .iter()
            .any(|entry| { entry.path == "config" && entry.entry_type == GitEntryType::Directory })
    );
    assert!(
        root.entries.iter().any(|entry| {
            entry.path == "compose.yaml" && entry.entry_type == GitEntryType::File
        })
    );

    let nested = cli
        .list_tree(
            repository.path(),
            &first_commit,
            "config",
            100,
            1024 * 1024,
            &cancellation,
        )
        .await
        .unwrap();
    let binary = nested.entries.first().unwrap();
    assert_eq!(binary.path, "config/binary.dat");
    let blob = cli
        .read_blob(repository.path(), &binary.object_id, 1024, &cancellation)
        .await
        .unwrap();
    assert_eq!(blob.content, b"A\0B");
    assert!(!blob.truncated);

    std::fs::rename(
        repository.path().join("compose.yaml"),
        repository.path().join("stack.yaml"),
    )
    .unwrap();
    std::fs::write(repository.path().join("README.md"), "# stack\n").unwrap();
    let second_commit = repository.commit("rename compose");
    let comparison = cli
        .compare_commits(
            repository.path(),
            &first_commit,
            &second_commit,
            100,
            1024 * 1024,
            &cancellation,
        )
        .await
        .unwrap();
    assert!(comparison.files.iter().any(|file| {
        file.status == GitChangedPathStatus::Renamed
            && file.previous_path.as_deref() == Some("compose.yaml")
            && file.path == "stack.yaml"
    }));
    assert!(
        comparison
            .files
            .iter()
            .any(|file| { file.status == GitChangedPathStatus::Added && file.path == "README.md" })
    );
}

#[tokio::test]
async fn blob_reads_are_bounded_and_report_truncation() {
    let repository = TemporaryGitRepository::new();
    std::fs::write(repository.path().join("large.txt"), vec![b'x'; 4096]).unwrap();
    let commit = repository.commit("large blob");
    let cli = GitCli::new(
        std::sync::Arc::new(citadel_processes::SystemProcess),
        std::sync::Arc::new(
            citadel_adapters::filesystem::git_workspace::LocalGitWorkspace::new(
                citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
            ),
        ),
        Duration::from_secs(10),
    );
    let cancellation = CancellationToken::new();
    let listing = cli
        .list_tree(
            repository.path(),
            &commit,
            "",
            10,
            1024 * 1024,
            &cancellation,
        )
        .await
        .unwrap();
    let blob = cli
        .read_blob(
            repository.path(),
            &listing.entries[0].object_id,
            32,
            &cancellation,
        )
        .await
        .unwrap();

    assert_eq!(blob.content.len(), 32);
    assert!(blob.truncated);
}
