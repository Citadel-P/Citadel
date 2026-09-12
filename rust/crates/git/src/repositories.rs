use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use citadel_domain::ActorId;
use citadel_resources::webhooks::{WebhookError, repository_matches, webhook_branch};
use futures_util::future::BoxFuture;
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    GitAccountService, GitAuthConfiguration, GitChangedPath, GitCli, GitEntryType, GitError,
    GitTransport, GitTreeEntry, RemoteBranch, SyncResult,
};

const MAX_DIRECTORY_ENTRIES: usize = 1_000;
const MAX_STRUCTURED_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_TEXT_PREVIEW_BYTES: usize = 1024 * 1024;
const MAX_STACK_SOURCE_FILES: usize = 512;
const MAX_STACK_SOURCE_BYTES: usize = 12 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDirectoryListing {
    pub repository_id: Uuid,
    pub commit_sha: String,
    pub path: String,
    pub entries: Vec<GitDirectoryEntry>,
    pub is_truncated: bool,
    pub provider_repository_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDirectoryEntry {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: GitEntryTypeView,
    pub size: Option<u64>,
    pub mode: String,
    pub target_commit_sha: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum GitEntryTypeView {
    Directory,
    File,
    Symlink,
    Submodule,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileContent {
    pub repository_id: Uuid,
    pub commit_sha: String,
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: GitEntryTypeView,
    pub size: u64,
    pub is_binary: bool,
    pub is_truncated: bool,
    pub content: Option<String>,
    pub preview_unavailable_reason: Option<String>,
    pub provider_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitComparison {
    pub repository_id: Uuid,
    pub base_commit_sha: String,
    pub head_commit_sha: String,
    pub files: Vec<GitChangedPath>,
    pub is_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitComposeDiscovery {
    pub repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: String,
    pub projects: Vec<GitComposeProjectCandidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitComposeProjectCandidate {
    pub working_directory: String,
    pub compose_paths: Vec<String>,
    pub env_file_paths: Vec<String>,
    pub suggested_watch_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSnapshotFile {
    pub relative_path: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSnapshot {
    pub resolved_commit_sha: String,
    pub files: Vec<GitSnapshotFile>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GitRepositorySyncMode {
    Manual,
    #[default]
    PullInterval,
}

#[derive(Debug, Clone)]
pub struct GitRepositorySource {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub default_branch: String,
    pub git_account_id: Option<Uuid>,
    pub sync_mode: GitRepositorySyncMode,
    pub sync_interval_minutes: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct GitSyncClaim {
    pub trigger: String,
    pub previous_commit: Option<String>,
    pub previous_error: Option<String>,
    pub repository: GitRepositorySource,
    pub reference_id: Uuid,
    pub branch: String,
    pub actor_id: ActorId,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryRefView {
    pub id: Uuid,
    pub git_repository_id: Uuid,
    pub branch: String,
    pub resolved_commit_sha: Option<String>,
    pub status: String,
    pub last_error: Option<String>,
    pub last_synced_at: DateTime<Utc>,
}

pub use citadel_resources::webhooks::WebhookConfiguration as GitRepositoryWebhook;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitWebhookOutcome {
    Queued { branch: String },
    Ignored,
}

pub trait GitRepositoryExecutionStore: Send + Sync {
    fn get_source<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepositorySource, GitRepositoryExecutionError>>;
    fn enqueue_sync<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn enqueue_apply<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>> {
        self.enqueue_sync(actor_id, id, Some(branch))
    }
    fn enqueue_webhook<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        branch: &'a str,
        expected: &'a GitRepositoryWebhook,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn enqueue_due<'a>(
        &'a self,
        limit: usize,
    ) -> BoxFuture<'a, Result<usize, GitRepositoryExecutionError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<GitSyncClaim>, GitRepositoryExecutionError>>;
    fn complete<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        result: &'a SyncResult,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn fail<'a>(
        &'a self,
        claim: &'a GitSyncClaim,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), GitRepositoryExecutionError>>;
    fn list_refs<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<GitRepositoryRefView>, GitRepositoryExecutionError>>;
    fn get_ref<'a>(
        &'a self,
        id: Uuid,
        branch: &'a str,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryRefView>, GitRepositoryExecutionError>>;
    fn resolve_reference<'a>(
        &'a self,
        id: Uuid,
        revision: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String, GitRepositoryExecutionError>>;
    fn get_webhook<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<GitRepositoryWebhook>, GitRepositoryExecutionError>>;
}

#[derive(Debug, thiserror::Error)]
pub enum GitRepositoryExecutionError {
    #[error("{0}")]
    Validation(String),
    #[error("Git repository was not found")]
    NotFound,
    #[error("Git repository has not been synchronized yet")]
    NotSynchronized,
    #[error("Git repository operation is already queued or running")]
    Conflict,
    #[error("Git repository execution failed: {0}")]
    Git(#[from] GitError),
    #[error("Git credential configuration is invalid")]
    Credential,
    #[error("Webhook authentication failed")]
    Authentication,
    #[error("Git repository storage failed: {0}")]
    Storage(String),
}

pub struct GitRepositoryExecutionService {
    store: Arc<dyn GitRepositoryExecutionStore>,
    accounts: Arc<GitAccountService>,
    cli: Arc<GitCli>,
    cache_root: PathBuf,
    stale_after: Duration,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl GitRepositoryExecutionService {
    #[must_use]
    pub fn new(
        store: Arc<dyn GitRepositoryExecutionStore>,
        accounts: Arc<GitAccountService>,
        cli: Arc<GitCli>,
        cache_root: PathBuf,
        stale_after: Duration,
    ) -> Self {
        Self {
            store,
            accounts,
            cli,
            cache_root,
            stale_after,
            on_change: None,
        }
    }

    #[must_use]
    pub fn with_change_notifier(mut self, on_change: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.on_change = Some(on_change);
        self
    }

    fn changed(&self) {
        if let Some(on_change) = &self.on_change {
            on_change();
        }
    }

    pub async fn request_sync(
        &self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&str>,
    ) -> Result<(), GitRepositoryExecutionError> {
        if let Some(branch) = branch {
            validate_branch_input(branch)?;
        }
        self.store.enqueue_sync(actor_id, id, branch).await
    }

    /// Reuse the durable repository worker rather than fetching concurrently
    /// into its cache. The caller's cancellation stops waiting, not other
    /// consumers of the same repository synchronization.
    pub async fn synchronize_commit(
        &self,
        actor: ActorId,
        id: Uuid,
        branch: &str,
        cancellation: &CancellationToken,
    ) -> Result<String, GitRepositoryExecutionError> {
        validate_branch_input(branch)?;
        if cancellation.is_cancelled() {
            return Err(GitError::Process(citadel_execution::ProcessError::Cancelled).into());
        }
        self.store.enqueue_apply(actor, id, branch).await?;
        self.changed();
        // Enqueue changes Healthy/Failed to Pending under the repository lock,
        // and a request arriving during Syncing causes another Pending pass.
        let wait = async {
            loop {
                if let Some(reference) = self.store.get_ref(id, branch).await? {
                    match reference.status.as_str() {
                        "Healthy" => {
                            let commit = reference
                                .resolved_commit_sha
                                .ok_or(GitRepositoryExecutionError::NotSynchronized)?;
                            if !is_full_object_id(&commit) {
                                return Err(GitRepositoryExecutionError::Validation(
                                    "Repository returned an invalid commit ID.".into(),
                                ));
                            }
                            return Ok(commit);
                        }
                        "Failed" | "Degraded" => {
                            return Err(GitRepositoryExecutionError::Validation(
                                "Repository synchronization failed. See its activity for details."
                                    .into(),
                            ));
                        }
                        _ => {}
                    }
                } else {
                    return Err(GitRepositoryExecutionError::NotFound);
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        };
        tokio::select! {
            () = cancellation.cancelled() => Err(GitError::Process(citadel_execution::ProcessError::Cancelled).into()),
            result = tokio::time::timeout(Duration::from_secs(300), wait) => {
                result.unwrap_or_else(|_| Err(GitError::Process(citadel_execution::ProcessError::Timeout(Duration::from_secs(300))).into()))
            }
        }
    }

    pub async fn list_refs(
        &self,
        id: Uuid,
    ) -> Result<Vec<GitRepositoryRefView>, GitRepositoryExecutionError> {
        self.store.list_refs(id).await
    }

    pub async fn source(
        &self,
        id: Uuid,
    ) -> Result<GitRepositorySource, GitRepositoryExecutionError> {
        self.store.get_source(id).await
    }

    pub async fn discover_branches(
        &self,
        id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<Vec<RemoteBranch>, GitRepositoryExecutionError> {
        let source = self.store.get_source(id).await?;
        let remote = self.prepare_remote(&source).await?;
        self.cli
            .list_remote_branches_with_environment(&remote.url, &remote.environment, cancellation)
            .await
            .map_err(Into::into)
    }

    pub async fn process_one(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<bool, GitRepositoryExecutionError> {
        let stale_before = Utc::now()
            - chrono::Duration::from_std(self.stale_after)
                .map_err(|error| GitRepositoryExecutionError::Storage(error.to_string()))?;
        let Some(claim) = self.store.claim_next(stale_before).await? else {
            return Ok(false);
        };
        self.changed();
        let result = self.synchronize_claim(&claim, cancellation).await;
        match result {
            Ok(result) => self.store.complete(&claim, &result).await?,
            Err(error) => {
                let message = bounded_error(&error.to_string());
                self.store.fail(&claim, &message).await?;
            }
        }
        self.changed();
        Ok(true)
    }

    pub async fn enqueue_due(&self, limit: usize) -> Result<usize, GitRepositoryExecutionError> {
        let count = self.store.enqueue_due(limit.clamp(1, 100)).await?;
        if count > 0 {
            self.changed();
        }
        Ok(count)
    }

    pub async fn receive_webhook(
        &self,
        actor_id: ActorId,
        id: Uuid,
        auth_type: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<GitWebhookOutcome, GitRepositoryExecutionError> {
        if body.len() > 1024 * 1024 {
            return Err(GitRepositoryExecutionError::Validation(
                "Webhook payload exceeds the 1 MiB limit.".to_owned(),
            ));
        }
        let original_webhook = self
            .store
            .get_webhook(id)
            .await?
            .filter(|configuration| configuration.enabled)
            .ok_or(GitRepositoryExecutionError::NotFound)?;
        let mut webhook = original_webhook.clone();
        let source = self.store.get_source(id).await?;
        if webhook
            .branch_filter
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            webhook.branch_filter = Some(source.default_branch.clone());
        }
        if webhook
            .evaluate(auth_type, headers, body)
            .map_err(map_webhook_error)?
            .is_some()
        {
            return Ok(GitWebhookOutcome::Ignored);
        }
        let payload = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
        if !repository_matches(&source.url, &payload) {
            return Ok(GitWebhookOutcome::Ignored);
        }
        let (_, payload_branch) =
            webhook_branch(&webhook.provider, headers, body).map_err(map_webhook_error)?;
        let branch = payload_branch
            .as_deref()
            .or(webhook.branch_filter.as_deref());
        self.store
            .enqueue_webhook(
                actor_id,
                id,
                branch.unwrap_or(&source.default_branch),
                &original_webhook,
            )
            .await?;
        self.changed();
        Ok(GitWebhookOutcome::Queued {
            branch: branch.unwrap_or(&source.default_branch).to_owned(),
        })
    }

    pub async fn resolve_commit(
        &self,
        id: Uuid,
        revision: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<String, GitRepositoryExecutionError> {
        let source = self.store.get_source(id).await?;
        let stored = self.store.resolve_reference(id, revision).await?;
        self.cli
            .resolve_named_commit(&self.cache_path(source.id), &stored, cancellation)
            .await
            .map_err(Into::into)
    }

    /// Materializes one immutable, bounded Git tree in memory for transport to
    /// the Stack runtime. Git links and submodules are rejected because their
    /// filesystem semantics cannot be reproduced safely by a byte-file bundle.
    pub async fn stack_snapshot(
        &self,
        id: Uuid,
        revision: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<GitSnapshot, GitRepositoryExecutionError> {
        let commit = self.resolve_commit(id, revision, cancellation).await?;
        let listing = self
            .cli
            .list_tree_recursive(
                &self.cache_path(id),
                &commit,
                MAX_STACK_SOURCE_FILES + 1,
                MAX_STRUCTURED_OUTPUT_BYTES,
                cancellation,
            )
            .await?;
        if listing.truncated || listing.entries.len() > MAX_STACK_SOURCE_FILES {
            return Err(GitRepositoryExecutionError::Validation(format!(
                "Git Stack source contains more than {MAX_STACK_SOURCE_FILES} files."
            )));
        }
        let total = listing
            .entries
            .iter()
            .try_fold(0_u64, |total, entry| match entry.entry_type {
                GitEntryType::File => total.checked_add(entry.size.unwrap_or_default()),
                GitEntryType::Symlink => None,
                GitEntryType::Submodule | GitEntryType::Directory => Some(total),
            });
        let Some(total) = total else {
            return Err(GitRepositoryExecutionError::Validation(
                "Git Stack source contains a symbolic link, which cannot be transported safely."
                    .to_owned(),
            ));
        };
        if listing
            .entries
            .iter()
            .any(|entry| entry.entry_type == GitEntryType::Submodule)
        {
            return Err(GitRepositoryExecutionError::Validation(
                "Git Stack source contains a submodule, which is not supported.".to_owned(),
            ));
        }
        if total > MAX_STACK_SOURCE_BYTES as u64 {
            return Err(GitRepositoryExecutionError::Validation(format!(
                "Git Stack source exceeds the {} MiB transport limit.",
                MAX_STACK_SOURCE_BYTES / (1024 * 1024)
            )));
        }
        let mut files = Vec::with_capacity(listing.entries.len());
        for entry in listing
            .entries
            .into_iter()
            .filter(|entry| entry.entry_type == GitEntryType::File)
        {
            let size = usize::try_from(entry.size.unwrap_or_default()).map_err(|_| {
                GitRepositoryExecutionError::Validation(
                    "Git Stack source file is too large for this platform.".to_owned(),
                )
            })?;
            let content = if size == 0 {
                Vec::new()
            } else {
                let blob = self
                    .cli
                    .read_blob(&self.cache_path(id), &entry.object_id, size, cancellation)
                    .await?;
                if blob.truncated || blob.content.len() != size {
                    return Err(GitRepositoryExecutionError::Git(GitError::InvalidOutput(
                        format!("Git returned incomplete content for '{}'.", entry.path),
                    )));
                }
                blob.content
            };
            files.push(GitSnapshotFile {
                relative_path: validate_repository_path(&entry.path, true)?,
                content,
            });
        }
        Ok(GitSnapshot {
            resolved_commit_sha: commit,
            files,
        })
    }

    pub async fn list_directory(
        &self,
        id: Uuid,
        revision: Option<&str>,
        path: &str,
        cancellation: &CancellationToken,
    ) -> Result<GitDirectoryListing, GitRepositoryExecutionError> {
        let path = validate_repository_path(path, false)?;
        let commit = self.resolve_commit(id, revision, cancellation).await?;
        let mut listing = self
            .cli
            .list_tree(
                &self.cache_path(id),
                &commit,
                &path,
                MAX_DIRECTORY_ENTRIES,
                MAX_STRUCTURED_OUTPUT_BYTES,
                cancellation,
            )
            .await?;
        listing.entries.sort_unstable_by(|left, right| {
            entry_bucket(left)
                .cmp(&entry_bucket(right))
                .then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(GitDirectoryListing {
            repository_id: id,
            commit_sha: commit,
            path,
            entries: listing
                .entries
                .into_iter()
                .map(map_directory_entry)
                .collect(),
            is_truncated: listing.truncated,
            provider_repository_url: None,
        })
    }

    pub async fn read_file(
        &self,
        id: Uuid,
        revision: Option<&str>,
        path: &str,
        cancellation: &CancellationToken,
    ) -> Result<GitFileContent, GitRepositoryExecutionError> {
        let path = validate_repository_path(path, true)?;
        let commit = self.resolve_commit(id, revision, cancellation).await?;
        let entry = self
            .cli
            .get_tree_entry(&self.cache_path(id), &commit, &path, cancellation)
            .await?
            .ok_or(GitRepositoryExecutionError::NotFound)?;
        if entry.entry_type == GitEntryType::Directory {
            return Err(GitRepositoryExecutionError::Validation(
                "Repository path is a directory.".to_owned(),
            ));
        }
        if entry.entry_type == GitEntryType::Submodule {
            return Ok(unavailable_file(
                id,
                commit,
                path,
                &entry,
                false,
                false,
                "Submodule browsing is not supported in this version.",
            ));
        }
        let size = entry.size.ok_or_else(|| {
            GitRepositoryExecutionError::Git(GitError::InvalidOutput(
                "repository file size is unavailable".to_owned(),
            ))
        })?;
        if size > MAX_TEXT_PREVIEW_BYTES as u64 {
            return Ok(unavailable_file(
                id,
                commit,
                path,
                &entry,
                false,
                true,
                "File is larger than the preview limit.",
            ));
        }
        let blob = self
            .cli
            .read_blob(
                &self.cache_path(id),
                &entry.object_id,
                MAX_TEXT_PREVIEW_BYTES,
                cancellation,
            )
            .await?;
        if blob.truncated {
            return Ok(unavailable_file(
                id,
                commit,
                path,
                &entry,
                false,
                true,
                "File is larger than the preview limit.",
            ));
        }
        if blob.content.contains(&0) {
            return Ok(unavailable_file(
                id,
                commit,
                path,
                &entry,
                true,
                false,
                "Binary files cannot be previewed.",
            ));
        }
        let content = match String::from_utf8(blob.content) {
            Ok(content) => content,
            Err(_) => {
                return Ok(unavailable_file(
                    id,
                    commit,
                    path,
                    &entry,
                    true,
                    false,
                    "File is not valid UTF-8 text.",
                ));
            }
        };
        Ok(GitFileContent {
            repository_id: id,
            commit_sha: commit,
            path,
            entry_type: map_entry_type(entry.entry_type),
            size,
            is_binary: false,
            is_truncated: false,
            content: Some(content),
            preview_unavailable_reason: None,
            provider_url: None,
        })
    }

    pub async fn compare(
        &self,
        id: Uuid,
        base_commit: &str,
        head_commit: &str,
        cancellation: &CancellationToken,
    ) -> Result<GitCommitComparison, GitRepositoryExecutionError> {
        self.store.get_source(id).await?;
        if !is_full_object_id(base_commit) || !is_full_object_id(head_commit) {
            return Err(GitRepositoryExecutionError::Validation(
                "Both revisions must be full commit SHAs.".to_owned(),
            ));
        }
        let base = self
            .cli
            .resolve_named_commit(&self.cache_path(id), base_commit, cancellation)
            .await?;
        let head = self
            .cli
            .resolve_named_commit(&self.cache_path(id), head_commit, cancellation)
            .await?;
        let listing = self
            .cli
            .compare_commits(
                &self.cache_path(id),
                &base,
                &head,
                MAX_DIRECTORY_ENTRIES,
                MAX_STRUCTURED_OUTPUT_BYTES,
                cancellation,
            )
            .await?;
        Ok(GitCommitComparison {
            repository_id: id,
            base_commit_sha: base,
            head_commit_sha: head,
            files: listing.files,
            is_truncated: listing.truncated,
        })
    }

    pub async fn discover_compose_projects(
        &self,
        id: Uuid,
        branch: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<GitComposeDiscovery, GitRepositoryExecutionError> {
        let source = self.store.get_source(id).await?;
        let branch = branch
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&source.default_branch);
        validate_branch_input(branch)?;
        let commit = self.resolve_commit(id, Some(branch), cancellation).await?;
        let tree = self
            .cli
            .list_tree_recursive(
                &self.cache_path(id),
                &commit,
                20_000,
                16 * 1024 * 1024,
                cancellation,
            )
            .await?;
        if tree.truncated {
            return Err(GitRepositoryExecutionError::Validation(
                "Repository is too large for bounded Compose project discovery.".to_owned(),
            ));
        }
        Ok(GitComposeDiscovery {
            repository_id: id,
            branch: branch.to_owned(),
            resolved_commit_sha: commit,
            projects: compose_projects(&tree.entries),
        })
    }

    #[must_use]
    pub fn cache_path(&self, id: Uuid) -> PathBuf {
        self.cache_root.join(id.to_string())
    }

    async fn synchronize_claim(
        &self,
        claim: &GitSyncClaim,
        cancellation: &CancellationToken,
    ) -> Result<SyncResult, GitRepositoryExecutionError> {
        let remote = self.prepare_remote(&claim.repository).await?;
        self.cli
            .synchronize_with_environment(
                &remote.url,
                &self.cache_path(claim.repository.id),
                &claim.branch,
                &remote.environment,
                cancellation,
            )
            .await
            .map_err(Into::into)
    }

    async fn prepare_remote(
        &self,
        source: &GitRepositorySource,
    ) -> Result<PreparedRemote, GitRepositoryExecutionError> {
        let Some(account_id) = source.git_account_id else {
            return Ok(PreparedRemote::new(normalize_remote_url(
                &source.url,
                None,
            )?));
        };
        let account = self
            .accounts
            .get_config(account_id)
            .await
            .map_err(|_| GitRepositoryExecutionError::Credential)?;
        let ssh_username = match &account.configuration {
            GitAuthConfiguration::SshKey { username, .. } => Some(username.as_str()),
            _ => None,
        };
        let url = normalize_remote_url(
            &source.url,
            Some((&account.domain, account.transport, ssh_username)),
        )?;
        let mut remote = PreparedRemote::new(url);
        match account.configuration {
            GitAuthConfiguration::Basic { username, password } => {
                remote.add_basic_header(&username, &password);
            }
            GitAuthConfiguration::Token { token } => {
                remote.add_basic_header("git", &token);
            }
            GitAuthConfiguration::SshKey {
                private_key,
                passphrase,
                ..
            } => {
                if passphrase.as_deref().is_some_and(|value| !value.is_empty()) {
                    return Err(GitRepositoryExecutionError::Validation(
                        "Passphrase-protected SSH keys are not supported by unattended Git synchronization."
                            .to_owned(),
                    ));
                }
                remote
                    .add_ssh_key(&self.cache_root, account_id, &private_key)
                    .await?;
            }
        }
        Ok(remote)
    }
}

fn map_entry_type(entry_type: GitEntryType) -> GitEntryTypeView {
    match entry_type {
        GitEntryType::Directory => GitEntryTypeView::Directory,
        GitEntryType::File => GitEntryTypeView::File,
        GitEntryType::Symlink => GitEntryTypeView::Symlink,
        GitEntryType::Submodule => GitEntryTypeView::Submodule,
    }
}

fn entry_bucket(entry: &GitTreeEntry) -> u8 {
    match entry.entry_type {
        GitEntryType::Directory => 0,
        GitEntryType::File | GitEntryType::Symlink => 1,
        GitEntryType::Submodule => 2,
    }
}

fn map_directory_entry(entry: GitTreeEntry) -> GitDirectoryEntry {
    GitDirectoryEntry {
        name: entry.name,
        path: entry.path,
        entry_type: map_entry_type(entry.entry_type),
        size: entry.size,
        mode: entry.mode,
        target_commit_sha: (entry.entry_type == GitEntryType::Submodule).then_some(entry.object_id),
    }
}

fn unavailable_file(
    repository_id: Uuid,
    commit_sha: String,
    path: String,
    entry: &GitTreeEntry,
    is_binary: bool,
    is_truncated: bool,
    reason: &str,
) -> GitFileContent {
    GitFileContent {
        repository_id,
        commit_sha,
        path,
        entry_type: map_entry_type(entry.entry_type),
        size: entry.size.unwrap_or(0),
        is_binary,
        is_truncated,
        content: None,
        preview_unavailable_reason: Some(reason.to_owned()),
        provider_url: None,
    }
}

fn validate_repository_path(
    path: &str,
    require_value: bool,
) -> Result<String, GitRepositoryExecutionError> {
    let path = path.trim();
    if path.chars().count() > 4_096 || path.len() > 16 * 1_024 || path.contains(['\0', '\\']) {
        return Err(GitRepositoryExecutionError::Validation(
            "Repository path is invalid or too long.".to_owned(),
        ));
    }
    let normalized = path.trim_start_matches("./").trim_matches('/');
    if (require_value && normalized.is_empty())
        || path.starts_with('/')
        || (!normalized.is_empty()
            && (normalized
                .split('/')
                .any(|component| component.is_empty() || matches!(component, "." | ".."))
                || normalized
                    .split('/')
                    .next()
                    .is_some_and(|component| component.eq_ignore_ascii_case(".git"))))
    {
        return Err(GitRepositoryExecutionError::Validation(
            "Repository path must be a relative path inside the selected Git tree.".to_owned(),
        ));
    }
    Ok(normalized.to_owned())
}

fn is_full_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn compose_projects(entries: &[GitTreeEntry]) -> Vec<GitComposeProjectCandidate> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut env_files = BTreeSet::new();
    for entry in entries {
        if entry.entry_type != GitEntryType::File {
            continue;
        }
        let (directory, name) = entry
            .path
            .rsplit_once('/')
            .map_or((".", entry.path.as_str()), |(directory, name)| {
                (directory, name)
            });
        if name == ".env" {
            env_files.insert(entry.path.clone());
        }
        if is_compose_file(name) {
            groups
                .entry(directory.to_owned())
                .or_default()
                .push(entry.path.clone());
        }
    }
    groups
        .into_iter()
        .map(|(working_directory, mut compose_paths)| {
            compose_paths.sort_unstable_by(|left, right| {
                compose_order(left)
                    .cmp(&compose_order(right))
                    .then_with(|| left.to_ascii_lowercase().cmp(&right.to_ascii_lowercase()))
            });
            let env_file_paths = env_files
                .iter()
                .filter(|path| {
                    path.rsplit_once('/')
                        .map_or(working_directory == ".", |(directory, _)| {
                            directory == working_directory
                        })
                })
                .cloned()
                .collect::<Vec<_>>();
            let suggested_watch_paths = if working_directory == "." {
                compose_paths
                    .iter()
                    .chain(&env_file_paths)
                    .cloned()
                    .collect()
            } else {
                vec![format!("{working_directory}/**")]
            };
            GitComposeProjectCandidate {
                working_directory,
                compose_paths,
                env_file_paths,
                suggested_watch_paths,
            }
        })
        .collect()
}

fn is_compose_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "compose.yml" | "compose.yaml" | "docker-compose.yml" | "docker-compose.yaml"
    ) {
        return true;
    }
    let Some(stem) = lower
        .strip_suffix(".yml")
        .or_else(|| lower.strip_suffix(".yaml"))
    else {
        return false;
    };
    stem == "compose"
        || stem == "docker-compose"
        || stem.ends_with(".compose")
        || stem.starts_with("compose.")
        || stem.starts_with("compose-")
        || stem.starts_with("docker-compose.")
        || stem.starts_with("docker-compose-")
}

fn compose_order(path: &str) -> u8 {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "compose.yml" | "compose.yaml" | "docker-compose.yml" | "docker-compose.yaml"
    ) {
        0
    } else if lower.contains(".override.") {
        10
    } else {
        20
    }
}

struct PreparedRemote {
    url: String,
    environment: Vec<(OsString, OsString)>,
    credential_file: Option<PathBuf>,
}

impl PreparedRemote {
    fn new(url: String) -> Self {
        Self {
            url,
            environment: Vec::new(),
            credential_file: None,
        }
    }

    fn add_basic_header(&mut self, username: &str, password: &str) {
        let encoded = STANDARD.encode(format!("{username}:{password}"));
        self.environment.extend([
            (OsString::from("GIT_CONFIG_COUNT"), OsString::from("1")),
            (
                OsString::from("GIT_CONFIG_KEY_0"),
                OsString::from("http.extraHeader"),
            ),
            (
                OsString::from("GIT_CONFIG_VALUE_0"),
                OsString::from(format!("Authorization: Basic {encoded}")),
            ),
        ]);
    }

    async fn add_ssh_key(
        &mut self,
        cache_root: &Path,
        account_id: Uuid,
        private_key: &str,
    ) -> Result<(), GitRepositoryExecutionError> {
        let directory = cache_root.join(".credentials");
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(storage_error)?;
        let path = directory.join(format!(
            "{}-{}.key",
            account_id.simple(),
            Uuid::now_v7().simple()
        ));
        if path.to_string_lossy().contains(['\"', '\r', '\n']) {
            return Err(GitRepositoryExecutionError::Credential);
        }
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        let mut file = options.open(&path).await.map_err(storage_error)?;
        self.credential_file = Some(path.clone());
        use tokio::io::AsyncWriteExt;
        file.write_all(private_key.trim_end().as_bytes())
            .await
            .map_err(storage_error)?;
        file.write_all(b"\n").await.map_err(storage_error)?;
        file.flush().await.map_err(storage_error)?;
        drop(file);
        let command = format!(
            "ssh -i \"{}\" -o IdentitiesOnly=yes -o StrictHostKeyChecking=no -o BatchMode=yes",
            path.display()
        );
        self.environment
            .push((OsString::from("GIT_SSH_COMMAND"), OsString::from(command)));
        Ok(())
    }
}

impl Drop for PreparedRemote {
    fn drop(&mut self) {
        if let Some(path) = self.credential_file.take()
            && let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(path = %path.display(), %error, "failed to delete temporary Git SSH key");
        }
    }
}

fn normalize_remote_url(
    url: &str,
    account: Option<(&str, GitTransport, Option<&str>)>,
) -> Result<String, GitRepositoryExecutionError> {
    let url = url.trim();
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("file://")
        || url.starts_with("git@")
    {
        return Ok(url.to_owned());
    }
    let Some((domain, transport, username)) = account else {
        return Err(GitRepositoryExecutionError::Validation(
            "A complete repository URL is required when no Git account is selected.".to_owned(),
        ));
    };
    let path = url.trim_start_matches('/');
    let scheme = match transport {
        GitTransport::Http => "http",
        GitTransport::Https => "https",
        GitTransport::Ssh => {
            let username = username
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("git");
            return Ok(format!("{username}@{domain}:{path}"));
        }
    };
    Ok(format!("{scheme}://{domain}/{path}"))
}

fn storage_error(error: std::io::Error) -> GitRepositoryExecutionError {
    GitRepositoryExecutionError::Storage(error.to_string())
}

fn validate_branch_input(branch: &str) -> Result<(), GitRepositoryExecutionError> {
    let branch = branch.trim();
    if branch.is_empty()
        || branch.len() > 255
        || branch.starts_with('-')
        || branch.contains(['\r', '\n'])
    {
        Err(GitRepositoryExecutionError::Validation(
            "Git branch name is invalid.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn bounded_error(message: &str) -> String {
    const MAX_ERROR_BYTES: usize = 4096;
    if message.len() <= MAX_ERROR_BYTES {
        return message.to_owned();
    }
    let mut end = MAX_ERROR_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> GitTreeEntry {
        GitTreeEntry {
            name: path
                .rsplit_once('/')
                .map_or(path, |(_, name)| name)
                .to_owned(),
            path: path.to_owned(),
            entry_type: GitEntryType::File,
            size: Some(1),
            mode: "100644".to_owned(),
            object_id: "a".repeat(40),
        }
    }

    #[test]
    fn account_relative_urls_preserve_transport_and_ssh_username() {
        assert_eq!(
            normalize_remote_url(
                "team/repository.git",
                Some(("git.example.test", GitTransport::Https, None)),
            )
            .unwrap(),
            "https://git.example.test/team/repository.git"
        );
        assert_eq!(
            normalize_remote_url(
                "team/repository.git",
                Some(("git.example.test", GitTransport::Ssh, Some("deploy"))),
            )
            .unwrap(),
            "deploy@git.example.test:team/repository.git"
        );
    }

    #[test]
    fn repository_paths_reject_metadata_and_traversal() {
        assert!(validate_repository_path("services/api", true).is_ok());
        assert!(validate_repository_path("../secret", true).is_err());
        assert!(validate_repository_path(".git/config", true).is_err());
        assert!(validate_repository_path("", true).is_err());
        assert_eq!(validate_repository_path("", false).unwrap(), "");
    }

    #[test]
    fn compose_discovery_groups_compose_and_env_files_without_materialization() {
        let projects = compose_projects(&[
            entry("compose.yaml"),
            entry("compose.override.yml"),
            entry(".env"),
            entry("apps/api/docker-compose.yml"),
            entry("apps/api/.env"),
            entry("README.md"),
        ]);

        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].working_directory, ".");
        assert_eq!(projects[0].compose_paths[0], "compose.yaml");
        assert_eq!(projects[0].env_file_paths, [".env"]);
        assert_eq!(projects[1].working_directory, "apps/api");
        assert_eq!(projects[1].suggested_watch_paths, ["apps/api/**"]);
    }
}

fn map_webhook_error(error: WebhookError) -> GitRepositoryExecutionError {
    match error {
        WebhookError::Authentication => GitRepositoryExecutionError::Authentication,
        WebhookError::Validation(message) => GitRepositoryExecutionError::Validation(message),
    }
}
