use super::*;
impl GitRepositoryExecutionService {
    /// Initializes pinned submodules with the source repository's Git account.
    pub async fn initialize_submodules(
        &self,
        id: Uuid,
        workspace: &Path,
        cancellation: &CancellationToken,
    ) -> Result<(), GitRepositoryExecutionError> {
        if !workspace.join(".gitmodules").is_file() {
            return Ok(());
        }
        let source = self.store.get_source(id).await?;
        let remote = self.prepare_remote(&source).await?;
        self.cli
            .initialize_submodules(workspace, &remote.url, &remote.environment, cancellation)
            .await?;
        Ok(())
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
}
