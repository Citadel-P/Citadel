pub use citadel_execution::ProcessRunner as GitProcessPort;
use std::ffi::{OsStr, OsString};

use std::path::Path;

use std::sync::Arc;

use std::time::Duration;

use citadel_execution::{
    OutputLimitPolicy, ProcessError, ProcessLimits, ProcessOutput, ProcessRequest,
};

#[cfg(test)]
use futures_util::future::BoxFuture;

use tokio_util::sync::CancellationToken;

#[cfg(test)]
use uuid::Uuid;

const GIT_STDOUT_LIMIT: usize = 64 * 1024;

const GIT_STDERR_LIMIT: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBranch {
    pub name: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResult {
    pub commit: String,
    pub cloned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitEntryType {
    Directory,
    File,
    Symlink,
    Submodule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTreeEntry {
    pub name: String,
    pub path: String,
    pub entry_type: GitEntryType,
    pub size: Option<u64>,
    pub mode: String,
    pub object_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTreeListing {
    pub entries: Vec<GitTreeEntry>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBlob {
    pub content: Vec<u8>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitChangedPathStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitChangedPath {
    pub status: GitChangedPathStatus,
    pub path: String,
    pub previous_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitChangedPathListing {
    pub files: Vec<GitChangedPath>,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("{0}")]
    Validation(String),
    #[error("Git execution failed: {0}")]
    Process(#[from] ProcessError),
    #[error("Git command failed: {0}")]
    Command(String),
    #[error("Git returned invalid output: {0}")]
    InvalidOutput(String),
    #[error("repository cache I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub struct GitCli {
    process: Arc<dyn GitProcessPort>,
    pub(crate) workspace: Arc<dyn crate::workspace::GitWorkspacePort>,
    executable: OsString,
    timeout: Duration,
    known_hosts: Option<std::path::PathBuf>,
}

impl GitCli {
    pub async fn initialize_submodules(
        &self,
        workspace: &Path,
        origin: &str,
        environment: &[(OsString, OsString)],
        cancellation: &CancellationToken,
    ) -> Result<(), GitError> {
        validate_remote(origin)?;
        // A workspace cloned from the local cache needs the upstream origin
        // to resolve relative submodule URLs.
        self.success(
            self.request(["remote", "set-url", "origin", origin])
                .current_dir(workspace),
            cancellation,
        )
        .await?;
        self.success(
            self.remote_request(
                [
                    "-c",
                    "protocol.file.allow=never",
                    "-c",
                    "protocol.ext.allow=never",
                    "submodule",
                    "update",
                    "--init",
                    "--recursive",
                    "--checkout",
                ],
                environment,
            )
            .current_dir(workspace),
            cancellation,
        )
        .await?;
        Ok(())
    }

    #[must_use]
    pub fn new(
        process: Arc<dyn GitProcessPort>,
        workspace: Arc<dyn crate::workspace::GitWorkspacePort>,
        timeout: Duration,
    ) -> Self {
        Self::with_process(process, workspace, "git", timeout)
    }

    #[must_use]
    pub fn with_process(
        process: Arc<dyn GitProcessPort>,
        workspace: Arc<dyn crate::workspace::GitWorkspacePort>,
        executable: impl Into<OsString>,
        timeout: Duration,
    ) -> Self {
        Self {
            process,
            workspace,
            executable: executable.into(),
            timeout,
            known_hosts: None,
        }
    }

    /// Trust is managed by the operator, never learned from an unverified connection.
    pub fn with_known_hosts(mut self, path: std::path::PathBuf) -> Self {
        self.known_hosts = Some(path);
        self
    }
    pub(crate) fn ssh_command(&self, key: Option<&Path>) -> String {
        let quote = |value: &str| format!("'{}'", value.replace('\'', "'\"'\"'"));
        let mut command =
            String::from("ssh -F /dev/null -o StrictHostKeyChecking=yes -o BatchMode=yes");
        if let Some(path) = &self.known_hosts {
            command.push_str(&format!(
                " -o UserKnownHostsFile={} -o GlobalKnownHostsFile=/dev/null",
                quote(&format!(
                    "\"{}\"",
                    path.to_string_lossy()
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                ))
            ));
        }
        if let Some(key) = key {
            command.push_str(&format!(
                " -i {} -o IdentitiesOnly=yes",
                quote(&key.to_string_lossy())
            ));
        }
        command
    }

    pub async fn test_connection(
        &self,
        url: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), GitError> {
        self.test_connection_with_environment(url, &[], cancellation)
            .await
    }

    pub async fn test_connection_with_environment(
        &self,
        url: &str,
        environment: &[(OsString, OsString)],
        cancellation: &CancellationToken,
    ) -> Result<(), GitError> {
        validate_remote(url)?;
        self.success(
            self.remote_request(["ls-remote", "--heads", "--", url], environment),
            cancellation,
        )
        .await
        .map(|_| ())
    }

    pub async fn list_remote_branches(
        &self,
        url: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<RemoteBranch>, GitError> {
        self.list_remote_branches_with_environment(url, &[], cancellation)
            .await
    }

    pub async fn list_remote_branches_with_environment(
        &self,
        url: &str,
        environment: &[(OsString, OsString)],
        cancellation: &CancellationToken,
    ) -> Result<Vec<RemoteBranch>, GitError> {
        validate_remote(url)?;
        let output = self
            .success(
                self.remote_request(["ls-remote", "--heads", "--", url], environment),
                cancellation,
            )
            .await?;
        parse_remote_branches(&output.stdout)
    }

    /// Synchronizes one tracked branch into a cache directory. A first clone
    /// is built in a sibling staging directory and renamed only after Git has
    /// completed, so cancellation and failures cannot expose a partial cache.
    pub async fn synchronize(
        &self,
        url: &str,
        target: &Path,
        branch: &str,
        cancellation: &CancellationToken,
    ) -> Result<SyncResult, GitError> {
        self.synchronize_with_environment(url, target, branch, &[], cancellation)
            .await
    }

    pub async fn synchronize_with_environment(
        &self,
        url: &str,
        target: &Path,
        branch: &str,
        environment: &[(OsString, OsString)],
        cancellation: &CancellationToken,
    ) -> Result<SyncResult, GitError> {
        validate_remote(url)?;
        validate_branch(branch)?;
        let git_directory = target.join(".git");
        let cloned = if self.workspace.exists(&git_directory).await? {
            self.fetch_and_reset(url, target, branch, environment, cancellation)
                .await?;
            false
        } else {
            if self.workspace.exists(target).await? {
                return Err(GitError::Validation(
                    "Repository cache exists but is not a Git worktree.".to_owned(),
                ));
            }
            self.clone_atomically(url, target, branch, environment, cancellation)
                .await?;
            true
        };
        let commit = self.resolve_commit(target, cancellation).await?;
        Ok(SyncResult { commit, cloned })
    }

    pub async fn resolve_commit(
        &self,
        repository: &Path,
        cancellation: &CancellationToken,
    ) -> Result<String, GitError> {
        let repository = repository.as_os_str();
        let output = self
            .success(
                self.request_os([
                    OsStr::new("-C"),
                    repository,
                    OsStr::new("rev-parse"),
                    OsStr::new("--verify"),
                    OsStr::new("HEAD^{commit}"),
                ]),
                cancellation,
            )
            .await?;
        let commit = std::str::from_utf8(&output.stdout)
            .map_err(|_| GitError::InvalidOutput("commit is not UTF-8".to_owned()))?
            .trim();
        if is_object_id(commit) {
            Ok(commit.to_ascii_lowercase())
        } else {
            Err(GitError::InvalidOutput(
                "commit identifier is not a full object ID".to_owned(),
            ))
        }
    }

    pub async fn resolve_named_commit(
        &self,
        repository: &Path,
        revision: &str,
        cancellation: &CancellationToken,
    ) -> Result<String, GitError> {
        validate_revision(revision)?;
        let candidate = format!("{revision}^{{commit}}");
        let output = self
            .success(
                self.request_os([
                    OsStr::new("-C"),
                    repository.as_os_str(),
                    OsStr::new("rev-parse"),
                    OsStr::new("--verify"),
                    OsStr::new(&candidate),
                ]),
                cancellation,
            )
            .await?;
        parse_commit(&output.stdout)
    }

    pub async fn list_tree(
        &self,
        repository: &Path,
        commit: &str,
        path: &str,
        maximum_entries: usize,
        maximum_output_bytes: usize,
        cancellation: &CancellationToken,
    ) -> Result<GitTreeListing, GitError> {
        validate_object_id(commit)?;
        let path = normalize_repository_path(path)?;
        validate_bounds(maximum_entries, maximum_output_bytes)?;
        let treeish = if path.is_empty() {
            commit.to_owned()
        } else {
            format!("{commit}:{path}")
        };
        let output = self
            .success(
                self.request_os_with_output_limit(
                    [
                        OsStr::new("-C"),
                        repository.as_os_str(),
                        OsStr::new("ls-tree"),
                        OsStr::new("-z"),
                        OsStr::new("-l"),
                        OsStr::new(&treeish),
                    ],
                    maximum_output_bytes,
                ),
                cancellation,
            )
            .await?;
        parse_tree(
            &output.stdout,
            &path,
            maximum_entries,
            output.stdout_truncated,
        )
    }

    pub async fn list_tree_recursive(
        &self,
        repository: &Path,
        commit: &str,
        maximum_entries: usize,
        maximum_output_bytes: usize,
        cancellation: &CancellationToken,
    ) -> Result<GitTreeListing, GitError> {
        validate_object_id(commit)?;
        validate_bounds(maximum_entries, maximum_output_bytes)?;
        let output = self
            .success(
                self.request_os_with_output_limit(
                    [
                        OsStr::new("-C"),
                        repository.as_os_str(),
                        OsStr::new("ls-tree"),
                        OsStr::new("-r"),
                        OsStr::new("-z"),
                        OsStr::new("-l"),
                        OsStr::new(commit),
                    ],
                    maximum_output_bytes,
                ),
                cancellation,
            )
            .await?;
        let mut listing = parse_tree(&output.stdout, "", maximum_entries, output.stdout_truncated)?;
        for entry in &mut listing.entries {
            entry.name = entry
                .path
                .rsplit_once('/')
                .map_or_else(|| entry.path.clone(), |(_, name)| name.to_owned());
        }
        Ok(listing)
    }

    pub async fn read_blob(
        &self,
        repository: &Path,
        object_id: &str,
        maximum_bytes: usize,
        cancellation: &CancellationToken,
    ) -> Result<GitBlob, GitError> {
        validate_object_id(object_id)?;
        if maximum_bytes == 0 {
            return Err(GitError::Validation(
                "Maximum blob size must be greater than zero.".to_owned(),
            ));
        }
        let output = self
            .success(
                self.request_os_with_output_limit(
                    [
                        OsStr::new("-C"),
                        repository.as_os_str(),
                        OsStr::new("cat-file"),
                        OsStr::new("blob"),
                        OsStr::new(object_id),
                    ],
                    maximum_bytes,
                ),
                cancellation,
            )
            .await?;
        Ok(GitBlob {
            content: output.stdout,
            truncated: output.stdout_truncated,
        })
    }

    pub async fn get_tree_entry(
        &self,
        repository: &Path,
        commit: &str,
        path: &str,
        cancellation: &CancellationToken,
    ) -> Result<Option<GitTreeEntry>, GitError> {
        validate_object_id(commit)?;
        let path = normalize_repository_path(path)?;
        if path.is_empty() {
            return Err(GitError::Validation(
                "Repository file path is required.".to_owned(),
            ));
        }
        let output = self
            .success(
                self.request_os_with_output_limit(
                    [
                        OsStr::new("-C"),
                        repository.as_os_str(),
                        OsStr::new("ls-tree"),
                        OsStr::new("-z"),
                        OsStr::new("-l"),
                        OsStr::new(commit),
                        OsStr::new("--"),
                        OsStr::new(&path),
                    ],
                    GIT_STDOUT_LIMIT,
                ),
                cancellation,
            )
            .await?;
        let (parent, _) = path.rsplit_once('/').unwrap_or(("", path.as_str()));
        let mut listing = parse_tree(&output.stdout, parent, 2, output.stdout_truncated)?;
        if listing.truncated || listing.entries.len() > 1 {
            return Err(GitError::InvalidOutput(
                "Git returned an ambiguous tree entry.".to_owned(),
            ));
        }
        Ok(listing.entries.pop())
    }

    pub async fn compare_commits(
        &self,
        repository: &Path,
        base_commit: &str,
        head_commit: &str,
        maximum_entries: usize,
        maximum_output_bytes: usize,
        cancellation: &CancellationToken,
    ) -> Result<GitChangedPathListing, GitError> {
        validate_object_id(base_commit)?;
        validate_object_id(head_commit)?;
        validate_bounds(maximum_entries, maximum_output_bytes)?;
        let output = self
            .success(
                self.request_os_with_output_limit(
                    [
                        OsStr::new("-C"),
                        repository.as_os_str(),
                        OsStr::new("diff"),
                        OsStr::new("--name-status"),
                        OsStr::new("-z"),
                        OsStr::new("-M"),
                        OsStr::new(base_commit),
                        OsStr::new(head_commit),
                        OsStr::new("--"),
                    ],
                    maximum_output_bytes,
                ),
                cancellation,
            )
            .await?;
        parse_changed_paths(&output.stdout, maximum_entries, output.stdout_truncated)
    }

    async fn fetch_and_reset(
        &self,
        url: &str,
        target: &Path,
        branch: &str,
        environment: &[(OsString, OsString)],
        cancellation: &CancellationToken,
    ) -> Result<(), GitError> {
        let target = target.as_os_str();
        self.success(
            self.remote_request_os(
                [
                    OsStr::new("-C"),
                    target,
                    OsStr::new("remote"),
                    OsStr::new("set-url"),
                    OsStr::new("origin"),
                    OsStr::new("--"),
                    OsStr::new(url),
                ],
                environment,
            ),
            cancellation,
        )
        .await?;
        let refspec = format!("+refs/heads/{branch}:refs/remotes/origin/{branch}");
        self.success(
            self.remote_request_os(
                [
                    OsStr::new("-C"),
                    target,
                    OsStr::new("fetch"),
                    OsStr::new("--prune"),
                    OsStr::new("origin"),
                    OsStr::new(&refspec),
                ],
                environment,
            ),
            cancellation,
        )
        .await?;
        let remote = format!("refs/remotes/origin/{branch}");
        self.success(
            self.remote_request_os(
                [
                    OsStr::new("-C"),
                    target,
                    OsStr::new("checkout"),
                    OsStr::new("-B"),
                    OsStr::new(branch),
                    OsStr::new(&remote),
                ],
                environment,
            ),
            cancellation,
        )
        .await?;
        self.success(
            self.remote_request_os(
                [
                    OsStr::new("-C"),
                    target,
                    OsStr::new("reset"),
                    OsStr::new("--hard"),
                    OsStr::new(&remote),
                ],
                environment,
            ),
            cancellation,
        )
        .await?;
        Ok(())
    }

    async fn clone_atomically(
        &self,
        url: &str,
        target: &Path,
        branch: &str,
        environment: &[(OsString, OsString)],
        cancellation: &CancellationToken,
    ) -> Result<(), GitError> {
        let mut staging = self.workspace.stage(target).await?;
        let result = self
            .success(
                self.remote_request_os(
                    [
                        OsStr::new("clone"),
                        OsStr::new("--single-branch"),
                        OsStr::new("--branch"),
                        OsStr::new(branch),
                        OsStr::new("--"),
                        OsStr::new(url),
                        staging.path().as_os_str(),
                    ],
                    environment,
                ),
                cancellation,
            )
            .await;
        if let Err(error) = result {
            let _ = staging.cleanup().await;
            return Err(error);
        }
        if let Err(error) = staging.publish(target).await {
            let _ = staging.cleanup().await;
            return Err(GitError::Io(error));
        }
        Ok(())
    }

    fn request<I, S>(&self, arguments: I) -> ProcessRequest
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.base_request().args(arguments)
    }

    fn remote_request<I, S>(
        &self,
        arguments: I,
        environment: &[(OsString, OsString)],
    ) -> ProcessRequest
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        apply_environment(self.request(arguments), environment)
    }

    fn request_os<I, S>(&self, arguments: I) -> ProcessRequest
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.base_request().args(
            arguments
                .into_iter()
                .map(|value| value.as_ref().to_os_string()),
        )
    }

    fn remote_request_os<I, S>(
        &self,
        arguments: I,
        environment: &[(OsString, OsString)],
    ) -> ProcessRequest
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        apply_environment(self.request_os(arguments), environment)
    }

    fn request_os_with_output_limit<I, S>(
        &self,
        arguments: I,
        maximum_stdout_bytes: usize,
    ) -> ProcessRequest
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.base_request()
            .args(
                arguments
                    .into_iter()
                    .map(|value| value.as_ref().to_os_string()),
            )
            .limits(ProcessLimits {
                timeout: self.timeout,
                maximum_stdout_bytes,
                maximum_stderr_bytes: GIT_STDERR_LIMIT,
                output_limit_policy: OutputLimitPolicy::Truncate,
            })
    }

    fn base_request(&self) -> ProcessRequest {
        ProcessRequest::new(self.executable.clone())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ASKPASS", "echo")
            .env("GIT_SSH_COMMAND", self.ssh_command(None))
            .env("GIT_LITERAL_PATHSPECS", "1")
            .env("LC_ALL", "C")
            .limits(ProcessLimits {
                timeout: self.timeout,
                maximum_stdout_bytes: GIT_STDOUT_LIMIT,
                maximum_stderr_bytes: GIT_STDERR_LIMIT,
                output_limit_policy: OutputLimitPolicy::Error,
            })
    }

    async fn success(
        &self,
        mut request: ProcessRequest,
        cancellation: &CancellationToken,
    ) -> Result<ProcessOutput, GitError> {
        // Git must wait for maintenance instead of orphaning a detached child.
        request.arguments.splice(
            0..0,
            [
                "-c",
                "maintenance.autoDetach=false",
                "-c",
                "gc.autoDetach=false",
            ]
            .map(OsString::from),
        );
        let output = self.process.run(request, cancellation).await?;
        if output.succeeded() {
            return Ok(output);
        }
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(GitError::Command(if message.is_empty() {
            format!("Git exited with status {:?}.", output.exit_code)
        } else {
            message
        }))
    }
}

fn validate_remote(url: &str) -> Result<(), GitError> {
    let url = url.trim();
    if url.is_empty() || url.len() > 2048 || url.starts_with('-') || url.contains(['\r', '\n']) {
        return Err(GitError::Validation(
            "Repository URL is invalid.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_branch(branch: &str) -> Result<(), GitError> {
    let valid = !branch.is_empty()
        && branch.len() <= 255
        && !branch.starts_with('-')
        && !branch.starts_with('.')
        && !branch.ends_with('.')
        && !branch.ends_with('/')
        && !branch.contains("..")
        && !branch.contains("@{")
        && !branch.contains([' ', '~', '^', ':', '?', '*', '[', '\\', '\r', '\n']);
    if valid {
        Ok(())
    } else {
        Err(GitError::Validation(
            "Git branch name is invalid.".to_owned(),
        ))
    }
}

fn validate_revision(revision: &str) -> Result<(), GitError> {
    if is_object_id(revision) || validate_branch(revision).is_ok() {
        Ok(())
    } else {
        Err(GitError::Validation("Git revision is invalid.".to_owned()))
    }
}

fn validate_object_id(object_id: &str) -> Result<(), GitError> {
    if is_object_id(object_id) {
        Ok(())
    } else {
        Err(GitError::Validation(
            "Git object identifier is invalid.".to_owned(),
        ))
    }
}

fn validate_bounds(maximum_entries: usize, maximum_output_bytes: usize) -> Result<(), GitError> {
    if maximum_entries == 0 || maximum_output_bytes == 0 {
        Err(GitError::Validation(
            "Git result limits must be greater than zero.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn normalize_repository_path(path: &str) -> Result<String, GitError> {
    let normalized = path.trim().replace('\\', "/");
    let valid = !normalized.starts_with('/')
        && !normalized.contains('\0')
        && normalized
            .split('/')
            .all(|component| !matches!(component, "." | ".."));
    if valid {
        Ok(normalized.trim_matches('/').to_owned())
    } else {
        Err(GitError::Validation(
            "Repository path must remain inside the selected Git tree.".to_owned(),
        ))
    }
}

fn parse_commit(output: &[u8]) -> Result<String, GitError> {
    let commit = std::str::from_utf8(output)
        .map_err(|_| GitError::InvalidOutput("commit is not UTF-8".to_owned()))?
        .trim();
    validate_object_id(commit)?;
    Ok(commit.to_ascii_lowercase())
}

fn parse_tree(
    output: &[u8],
    parent: &str,
    maximum_entries: usize,
    output_truncated: bool,
) -> Result<GitTreeListing, GitError> {
    let mut entries = Vec::with_capacity(maximum_entries.min(64));
    let mut truncated = output_truncated;
    for record in output
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
    {
        if entries.len() == maximum_entries {
            truncated = true;
            break;
        }
        let separator = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| {
                GitError::InvalidOutput("Git tree record has no path separator.".to_owned())
            })?;
        let metadata = std::str::from_utf8(&record[..separator])
            .map_err(|_| GitError::InvalidOutput("Git tree metadata is not UTF-8.".to_owned()))?;
        let name = std::str::from_utf8(&record[separator + 1..])
            .map_err(|_| GitError::InvalidOutput("Git tree path is not UTF-8.".to_owned()))?;
        let mut fields = metadata.split_ascii_whitespace();
        let mode = fields.next().unwrap_or_default();
        let kind = fields.next().unwrap_or_default();
        let object_id = fields.next().unwrap_or_default();
        let size = fields.next().and_then(|value| value.parse::<u64>().ok());
        validate_object_id(object_id)?;
        let entry_type = match (kind, mode) {
            ("tree", _) => GitEntryType::Directory,
            ("commit", _) => GitEntryType::Submodule,
            ("blob", "120000") => GitEntryType::Symlink,
            ("blob", _) => GitEntryType::File,
            _ => {
                return Err(GitError::InvalidOutput(format!(
                    "unsupported Git tree entry type '{kind}'"
                )));
            }
        };
        let path = if parent.is_empty() {
            name.to_owned()
        } else {
            format!("{parent}/{name}")
        };
        entries.push(GitTreeEntry {
            name: name.to_owned(),
            path,
            entry_type,
            size,
            mode: mode.to_owned(),
            object_id: object_id.to_ascii_lowercase(),
        });
    }
    Ok(GitTreeListing { entries, truncated })
}

fn parse_changed_paths(
    output: &[u8],
    maximum_entries: usize,
    output_truncated: bool,
) -> Result<GitChangedPathListing, GitError> {
    let fields = output
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let mut files = Vec::with_capacity(maximum_entries.min(64));
    let mut index = 0;
    let mut truncated = output_truncated;
    while index < fields.len() {
        if files.len() == maximum_entries {
            truncated = true;
            break;
        }
        let status = std::str::from_utf8(fields[index])
            .map_err(|_| GitError::InvalidOutput("Git change status is not UTF-8.".to_owned()))?;
        index += 1;
        let code = status.as_bytes().first().copied().unwrap_or_default();
        let changed_status = match code {
            b'A' => GitChangedPathStatus::Added,
            b'M' => GitChangedPathStatus::Modified,
            b'D' => GitChangedPathStatus::Deleted,
            b'R' => GitChangedPathStatus::Renamed,
            b'C' => GitChangedPathStatus::Copied,
            b'T' => GitChangedPathStatus::TypeChanged,
            _ => {
                return Err(GitError::InvalidOutput(format!(
                    "unsupported Git change status '{status}'"
                )));
            }
        };
        let renamed = matches!(
            changed_status,
            GitChangedPathStatus::Renamed | GitChangedPathStatus::Copied
        );
        let first = fields
            .get(index)
            .ok_or_else(|| GitError::InvalidOutput("Git change record has no path.".to_owned()))?;
        index += 1;
        let (previous_path, path_bytes) = if renamed {
            let second = fields.get(index).ok_or_else(|| {
                GitError::InvalidOutput("Git rename record has no destination.".to_owned())
            })?;
            index += 1;
            (Some(decode_git_path(first)?), *second)
        } else {
            (None, *first)
        };
        files.push(GitChangedPath {
            status: changed_status,
            path: decode_git_path(path_bytes)?,
            previous_path,
        });
    }
    Ok(GitChangedPathListing { files, truncated })
}

fn decode_git_path(path: &[u8]) -> Result<String, GitError> {
    std::str::from_utf8(path)
        .map(str::to_owned)
        .map_err(|_| GitError::InvalidOutput("Git path is not UTF-8.".to_owned()))
}

fn apply_environment(
    mut request: ProcessRequest,
    environment: &[(OsString, OsString)],
) -> ProcessRequest {
    request.environment.extend(environment.iter().cloned());
    request
}

fn parse_remote_branches(output: &[u8]) -> Result<Vec<RemoteBranch>, GitError> {
    let output = std::str::from_utf8(output)
        .map_err(|_| GitError::InvalidOutput("branch listing is not UTF-8".to_owned()))?;
    let mut branches = Vec::new();
    for line in output.lines() {
        let Some((commit, reference)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let Some(name) = reference.trim().strip_prefix("refs/heads/") else {
            continue;
        };
        if is_object_id(commit) && validate_branch(name).is_ok() {
            branches.push(RemoteBranch {
                name: name.to_owned(),
                commit: commit.to_ascii_lowercase(),
            });
        }
    }
    branches.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    branches.dedup_by(|left, right| left.name == right.name);
    Ok(branches)
}

fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn ssh_arguments_keep_strict_trust_and_quote_shell_metacharacters() {
        let path = Path::new("/tmp/key ' $(touch SHOULD_NOT_EXIST) `id` ; spaced");
        let cli = GitCli::new(
            Arc::new(FakeProcess::default()),
            Arc::new(TestWorkspace),
            Duration::from_secs(1),
        )
        .with_known_hosts(Path::new("/tmp/trusted hosts").into());
        let command = cli.ssh_command(Some(path));
        // Shell function captures the exact argv OpenSSH receives without network I/O.
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("ssh() {{ printf '%s\\n' \"$@\"; }}; {command}"))
            .output()
            .unwrap();
        assert!(output.status.success());
        let args = String::from_utf8(output.stdout).unwrap();
        assert!(args.lines().any(|s| s == path.to_str().unwrap()), "{args}");
        assert!(args.contains("StrictHostKeyChecking=yes"));
        assert!(args.contains("UserKnownHostsFile=\"/tmp/trusted hosts\""));
        assert!(cli.ssh_command(None).contains("StrictHostKeyChecking=yes"));
    }

    #[test]
    #[ignore = "requires CITADEL_SSH_TEST_ROOT and CITADEL_SSH_TEST_URL (disposable SSH Git server)"]
    fn ssh_trust_accepts_pinned_server_and_rejects_unknown_or_changed_host() {
        let root = std::path::PathBuf::from(std::env::var("CITADEL_SSH_TEST_ROOT").unwrap());
        let url = std::env::var("CITADEL_SSH_TEST_URL").unwrap();
        for (hosts, expected) in [("trusted", true), ("unknown", false), ("mismatched", false)] {
            let cli = GitCli::new(
                Arc::new(FakeProcess::default()),
                Arc::new(TestWorkspace),
                Duration::from_secs(5),
            )
            .with_known_hosts(root.join(hosts));
            let output = std::process::Command::new("git")
                .args(["ls-remote", "--", &url])
                .env(
                    "GIT_SSH_COMMAND",
                    cli.ssh_command(Some(&root.join("client key ' $literal"))),
                )
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .unwrap();
            assert_eq!(
                output.status.success(),
                expected,
                "{hosts}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if !expected {
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("Host key verification failed")
                );
            }
        }
    }

    struct TestWorkspace;
    impl crate::workspace::GitWorkspacePort for TestWorkspace {
        fn exists<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, std::io::Result<bool>> {
            Box::pin(tokio::fs::try_exists(path))
        }
        fn stage<'a>(
            &'a self,
            _: &'a Path,
        ) -> BoxFuture<'a, std::io::Result<Box<dyn crate::workspace::GitStagingDirectory>>>
        {
            Box::pin(async { panic!("unexpected clone") })
        }
        fn credential<'a>(
            &'a self,
            _: &'a Path,
            _: &'a str,
        ) -> BoxFuture<'a, std::io::Result<Box<dyn crate::workspace::GitCredentialFile>>> {
            Box::pin(async { panic!("unexpected credential") })
        }
    }

    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeProcess {
        outputs: Mutex<VecDeque<Result<ProcessOutput, ProcessError>>>,
        arguments: Mutex<Vec<Vec<OsString>>>,
    }

    impl FakeProcess {
        fn with_outputs(outputs: Vec<Result<ProcessOutput, ProcessError>>) -> Self {
            Self {
                outputs: Mutex::new(outputs.into()),
                arguments: Mutex::default(),
            }
        }
    }

    impl GitProcessPort for FakeProcess {
        fn run<'a>(
            &'a self,
            request: ProcessRequest,
            _cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
            self.arguments.lock().unwrap().push(request.arguments);
            let output = self.outputs.lock().unwrap().pop_front().unwrap();
            Box::pin(async move { output })
        }
    }

    fn output(stdout: &str) -> Result<ProcessOutput, ProcessError> {
        Ok(ProcessOutput {
            exit_code: Some(0),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
            stdout_truncated: false,
            stderr_truncated: false,
        })
    }

    #[tokio::test]
    async fn branch_listing_is_sorted_and_malformed_lines_are_ignored() {
        let process = Arc::new(FakeProcess::with_outputs(vec![output(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\trefs/heads/zeta\ninvalid\naaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/main\n",
        )]));
        let cli = GitCli::with_process(
            process,
            Arc::new(TestWorkspace),
            "git",
            Duration::from_secs(1),
        );

        let branches = cli
            .list_remote_branches("https://example.test/repo.git", &CancellationToken::new())
            .await
            .unwrap();

        assert_eq!(branches[0].name, "main");
        assert_eq!(branches[1].name, "zeta");
    }

    #[tokio::test]
    async fn remote_starting_with_an_option_is_rejected_without_execution() {
        let process = Arc::new(FakeProcess::default());
        let cli = GitCli::with_process(
            process.clone(),
            Arc::new(TestWorkspace),
            "git",
            Duration::from_secs(1),
        );

        let error = cli
            .test_connection("--upload-pack=malicious", &CancellationToken::new())
            .await
            .unwrap_err();

        assert!(matches!(error, GitError::Validation(_)));
        assert!(process.arguments.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn existing_cache_fetches_then_resets_before_resolving_commit() {
        let root = std::env::temp_dir().join(format!("citadel-git-test-{}", Uuid::now_v7()));
        tokio::fs::create_dir_all(root.join("repo/.git"))
            .await
            .unwrap();
        let commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let process = Arc::new(FakeProcess::with_outputs(vec![
            output(""),
            output(""),
            output(""),
            output(""),
            output(commit),
        ]));
        let cli = GitCli::with_process(
            process.clone(),
            Arc::new(TestWorkspace),
            "git",
            Duration::from_secs(1),
        );

        let result = cli
            .synchronize(
                "https://example.test/repo.git",
                &root.join("repo"),
                "main",
                &CancellationToken::new(),
            )
            .await
            .unwrap();

        assert!(!result.cloned);
        assert_eq!(result.commit, commit);
        {
            let calls = process.arguments.lock().unwrap();
            assert_eq!(calls.len(), 5);
            for call in calls.iter() {
                assert_eq!(
                    &call[..4],
                    &[
                        "-c",
                        "maintenance.autoDetach=false",
                        "-c",
                        "gc.autoDetach=false"
                    ]
                );
            }
            assert!(calls[0].iter().any(|value| value == "set-url"));
            assert!(calls[1].iter().any(|value| value == "fetch"));
            assert!(calls[3].iter().any(|value| value == "reset"));
        }
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[test]
    fn tree_parser_preserves_entry_types_and_applies_entry_limit() {
        let listing = parse_tree(
            concat!(
                "040000 tree aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa -\tconfig\0",
                "100644 blob bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb 12\tcompose.yaml\0",
                "120000 blob cccccccccccccccccccccccccccccccccccccccc 6\tcurrent\0"
            )
            .as_bytes(),
            "deploy",
            2,
            false,
        )
        .unwrap();

        assert_eq!(listing.entries.len(), 2);
        assert!(listing.truncated);
        assert_eq!(listing.entries[0].entry_type, GitEntryType::Directory);
        assert_eq!(listing.entries[0].path, "deploy/config");
        assert_eq!(listing.entries[1].entry_type, GitEntryType::File);
        assert_eq!(listing.entries[1].size, Some(12));
    }

    #[test]
    fn change_parser_handles_renames_without_losing_the_previous_path() {
        let listing =
            parse_changed_paths(b"R100\0compose.yaml\0stack.yaml\0A\0README.md\0", 10, false)
                .unwrap();

        assert_eq!(listing.files.len(), 2);
        assert_eq!(listing.files[0].status, GitChangedPathStatus::Renamed);
        assert_eq!(
            listing.files[0].previous_path.as_deref(),
            Some("compose.yaml")
        );
        assert_eq!(listing.files[0].path, "stack.yaml");
        assert_eq!(listing.files[1].status, GitChangedPathStatus::Added);
    }

    #[test]
    fn repository_paths_cannot_escape_the_selected_tree() {
        assert!(normalize_repository_path("services/api").is_ok());
        assert!(normalize_repository_path("../secrets").is_err());
        assert!(normalize_repository_path("services/../../secrets").is_err());
        assert!(normalize_repository_path("/absolute").is_err());
    }
}
