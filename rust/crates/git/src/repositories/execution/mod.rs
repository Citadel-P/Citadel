mod browser;
mod credentials;
mod discovery;
mod materialization;
mod sync;
use validation::*;
#[cfg(test)]
mod tests;
mod validation;
use super::*;

const MAX_DIRECTORY_ENTRIES: usize = 1_000;

const MAX_STRUCTURED_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

const MAX_TEXT_PREVIEW_BYTES: usize = 1024 * 1024;

const MAX_STACK_SOURCE_FILES: usize = 512;

const MAX_STACK_SOURCE_BYTES: usize = 12 * 1024 * 1024;

pub struct GitRepositoryExecutionService {
    store: Arc<dyn GitRepositoryExecutionPersistence>,
    accounts: Arc<GitAccountService>,
    cli: Arc<GitCli>,
    cache_root: PathBuf,
    stale_after: Duration,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl GitRepositoryExecutionService {
    #[must_use]
    pub fn new(
        store: Arc<dyn GitRepositoryExecutionPersistence>,
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

    pub(super) fn changed(&self) {
        if let Some(on_change) = &self.on_change {
            on_change();
        }
    }

    pub async fn source(
        &self,
        id: Uuid,
    ) -> Result<GitRepositorySource, GitRepositoryExecutionError> {
        self.store.get_source(id).await
    }

    #[must_use]
    pub fn cache_path(&self, id: Uuid) -> PathBuf {
        self.cache_root.join(id.to_string())
    }
}
