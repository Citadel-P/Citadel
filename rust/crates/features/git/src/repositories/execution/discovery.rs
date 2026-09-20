use super::*;
impl GitRepositoryExecutionService {
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
}

pub(super) fn compose_projects(entries: &[GitTreeEntry]) -> Vec<GitComposeProjectCandidate> {
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

pub(super) fn is_compose_file(name: &str) -> bool {
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

pub(super) fn compose_order(path: &str) -> u8 {
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
