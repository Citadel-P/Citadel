use super::*;
impl GitRepositoryExecutionService {
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
}

pub(super) fn map_entry_type(entry_type: GitEntryType) -> GitBrowserEntryType {
    match entry_type {
        GitEntryType::Directory => GitBrowserEntryType::Directory,
        GitEntryType::File => GitBrowserEntryType::File,
        GitEntryType::Symlink => GitBrowserEntryType::Symlink,
        GitEntryType::Submodule => GitBrowserEntryType::Submodule,
    }
}

pub(super) fn entry_bucket(entry: &GitTreeEntry) -> u8 {
    match entry.entry_type {
        GitEntryType::Directory => 0,
        GitEntryType::File | GitEntryType::Symlink => 1,
        GitEntryType::Submodule => 2,
    }
}

pub(super) fn map_directory_entry(entry: GitTreeEntry) -> GitDirectoryEntry {
    GitDirectoryEntry {
        name: entry.name,
        path: entry.path,
        entry_type: map_entry_type(entry.entry_type),
        size: entry.size,
        mode: entry.mode,
        target_commit_sha: (entry.entry_type == GitEntryType::Submodule).then_some(entry.object_id),
    }
}

pub(super) fn unavailable_file(
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
