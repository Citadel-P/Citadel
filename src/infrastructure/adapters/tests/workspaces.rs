use citadel_adapters::filesystem::{
    automation_workspace::LocalAutomationWorkspace, git_workspace::LocalGitWorkspace,
};
use citadel_automation::workspace::AutomationWorkspacePort;
use citadel_git::workspace::GitWorkspacePort;
use citadel_runtime::DynamicTasks;
use std::{path::PathBuf, time::Duration};
use tokio_util::sync::CancellationToken;

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("citadel-workspace-test-{}", uuid::Uuid::now_v7())))
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn git_credentials_are_private_and_removed_on_drop() {
    let tasks = DynamicTasks::new(CancellationToken::new());
    let root = Root::new();
    let credential = LocalGitWorkspace::new(tasks.clone())
        .credential(&root.0, "private-key\n\n")
        .await
        .unwrap();
    let path = credential.path().to_owned();
    assert_eq!(
        tokio::fs::read_to_string(&path).await.unwrap(),
        "private-key\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    drop(credential);
    assert!(!path.exists());
}

#[tokio::test]
async fn git_staging_publishes_atomically_and_cleans_abandoned_clones() {
    let tasks = DynamicTasks::new(CancellationToken::new());
    let root = Root::new();
    let target = root.0.join("repo");
    let mut staging = LocalGitWorkspace::new(tasks.clone())
        .stage(&target)
        .await
        .unwrap();
    let staging_path = staging.path().to_owned();
    tokio::fs::create_dir(&staging_path).await.unwrap();
    tokio::fs::write(staging_path.join("file"), "committed")
        .await
        .unwrap();
    assert!(!target.exists());
    staging.publish(&target).await.unwrap();
    drop(staging);
    assert_eq!(
        tokio::fs::read_to_string(target.join("file"))
            .await
            .unwrap(),
        "committed"
    );
    let abandoned = LocalGitWorkspace::new(tasks.clone())
        .stage(&target)
        .await
        .unwrap();
    let path = abandoned.path().to_owned();
    tokio::fs::create_dir(&path).await.unwrap();
    drop(abandoned);
    tasks.drain(Duration::from_secs(5)).await.unwrap();
    assert!(!path.exists());
    assert!(target.exists());
}

#[tokio::test]
async fn automation_scripts_are_private_and_cleanup_covers_abandonment() {
    let tasks = DynamicTasks::new(CancellationToken::new());
    let root = Root::new();
    let run = root.0.join("run");
    let workspace = LocalAutomationWorkspace::new(tasks.clone())
        .prepare(&run, None, "short-lived credential")
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::read_to_string(workspace.script()).await.unwrap(),
        "short-lived credential"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&run).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    assert!(
        LocalAutomationWorkspace::new(tasks.clone())
            .prepare(&run, None, "replacement")
            .await
            .is_err()
    );
    drop(workspace);
    tasks.drain(Duration::from_secs(5)).await.unwrap();
    assert!(!run.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn automation_does_not_follow_an_existing_workspace_symlink() {
    let tasks = DynamicTasks::new(CancellationToken::new());
    let root = Root::new();
    let external = root.0.join("external");
    tokio::fs::create_dir_all(&external).await.unwrap();
    let run = root.0.join("run");
    std::os::unix::fs::symlink(&external, &run).unwrap();
    assert!(
        LocalAutomationWorkspace::new(tasks.clone())
            .prepare(&run, None, "secret")
            .await
            .is_err()
    );
    assert!(!external.join("action.ts").exists());
    assert!(external.exists());
}
