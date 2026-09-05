use std::collections::HashSet;
use std::sync::Arc;

use citadel_git::GitRepositoryExecutionService;
use citadel_stacks::{
    StackApplySource, StackError, StackOperationClaim, StackSourceFile,
    StackSourceMaterializerPort, StackSpec,
};
use futures_util::{FutureExt, future::BoxFuture};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct GitStackSourceMaterializer {
    git: Arc<GitRepositoryExecutionService>,
}

impl GitStackSourceMaterializer {
    #[must_use]
    pub fn new(git: Arc<GitRepositoryExecutionService>) -> Self {
        Self { git }
    }
}

impl StackSourceMaterializerPort for GitStackSourceMaterializer {
    fn materialize<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackApplySource, StackError>> {
        async move {
            let StackSpec::Git {
                git_repo_id,
                branch,
                commit_sha,
                compose_paths,
                working_directory,
                compose_env_files_from_repo,
                additional_env_file_from_repo,
                ..
            } = &claim.spec
            else {
                return Err(StackError::Validation(
                    "Only Git Stack sources require Git materialization.".to_owned(),
                ));
            };
            let revision = commit_sha.as_deref().unwrap_or(branch);
            let snapshot = self
                .git
                .stack_snapshot(*git_repo_id, Some(revision), cancellation)
                .await
                .map_err(|error| StackError::Validation(error.to_string()))?;
            let available = snapshot
                .files
                .iter()
                .map(|file| file.relative_path.as_str())
                .collect::<HashSet<_>>();
            for path in compose_paths {
                if !available.contains(path.as_str()) {
                    return Err(StackError::Validation(format!(
                        "Git Stack Compose file '{path}' does not exist at commit {}.",
                        snapshot.resolved_commit_sha
                    )));
                }
            }
            let env_file_paths = if compose_env_files_from_repo.is_empty() {
                additional_env_file_from_repo.clone()
            } else {
                compose_env_files_from_repo.clone()
            };
            for path in &env_file_paths {
                if !available.contains(path.as_str()) {
                    return Err(StackError::Validation(format!(
                        "Git Stack environment file '{path}' does not exist at commit {}.",
                        snapshot.resolved_commit_sha
                    )));
                }
            }
            let working_directory = working_directory.clone().unwrap_or_else(|| {
                compose_paths[0]
                    .rsplit_once('/')
                    .map_or_else(|| ".".to_owned(), |(parent, _)| parent.to_owned())
            });
            if working_directory != "." {
                let prefix = format!("{}/", working_directory.trim_end_matches('/'));
                if !available.iter().any(|path| path.starts_with(&prefix)) {
                    return Err(StackError::Validation(format!(
                        "Git Stack working directory '{working_directory}' does not exist at commit {}.",
                        snapshot.resolved_commit_sha
                    )));
                }
            }
            Ok(StackApplySource {
                files: snapshot
                    .files
                    .into_iter()
                    .map(|file| StackSourceFile {
                        relative_path: file.relative_path,
                        content: file.content,
                    })
                    .collect(),
                compose_paths: compose_paths.clone(),
                env_file_paths,
                working_directory,
                labels_override_path: None,
                resolved_commit_sha: Some(snapshot.resolved_commit_sha),
            })
        }
        .boxed()
    }
}
