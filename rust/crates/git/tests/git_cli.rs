use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use citadel_git::{GitChangedPathStatus, GitCli, GitEntryType};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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
            .expect("Git must be installed for the Phase 7 gate");
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
    let cli = GitCli::new(Duration::from_secs(10));
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
    let cli = GitCli::new(Duration::from_secs(10));
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
