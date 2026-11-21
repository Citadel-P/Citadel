//! DeploymentImageScannerJob: one successful registry observation per unique
//! image each cycle, shared by Deployment, Stack, and managed Service checks.
use crate::connectors::registries::digest_cache::ImageDigestCache;
use crate::persistence::postgres::platforms::image_scan;
use citadel_deployments::DeploymentRuntime;
use citadel_stacks::{StackReleaseSource, StackSpec};
use futures_util::future::BoxFuture;
use sqlx::PgPool;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ImageScanTask {
    pub platform: Uuid,
    pub registry: Uuid,
    pub reference: String,
}
pub trait ImageScanRuntime: Send + Sync {
    fn inspect<'a>(
        &'a self,
        task: &'a ImageScanTask,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, String>>;
}
impl ImageScanRuntime for crate::connectors::routing::deployments::DeploymentRuntimeRouter {
    fn inspect<'a>(
        &'a self,
        task: &'a ImageScanTask,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            self.remote_image_digest(task.platform, task.registry, &task.reference, cancel)
                .await
                .map_err(|error| error.to_string())
        })
    }
}
pub struct ImageScanner {
    pool: PgPool,
    runtime: Arc<dyn ImageScanRuntime>,
    cache: Arc<ImageDigestCache>,
    git: Option<Arc<citadel_git::GitRepositoryExecutionService>>,
}
impl ImageScanner {
    pub fn new(
        pool: PgPool,
        runtime: Arc<dyn ImageScanRuntime>,
        cache: Arc<ImageDigestCache>,
    ) -> Self {
        Self {
            pool,
            runtime,
            cache,
            git: None,
        }
    }
    pub fn with_git(mut self, git: Arc<citadel_git::GitRepositoryExecutionService>) -> Self {
        self.git = Some(git);
        self
    }
    pub async fn run_cycle(&self, cancel: &CancellationToken) -> Result<usize, sqlx::Error> {
        let tasks = self.load_tasks(cancel).await?;
        let scanned = scan_unique(self.runtime.as_ref(), &self.cache, tasks, cancel).await;
        if !cancel.is_cancelled() {
            self.cache.mark_ready();
        }
        Ok(scanned)
    }
    async fn load_tasks(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<ImageScanTask>, sqlx::Error> {
        let mut tasks = Vec::new();
        self.external_tasks(false, cancel, &mut tasks).await?;
        let mut after = Uuid::nil();
        while !cancel.is_cancelled() {
            let page = image_scan::stack_images(&self.pool, after).await?;
            let full = page.len() == image_scan::STACK_PAGE as usize;
            for (id, platform, spec, source) in page {
                if cancel.is_cancelled() {
                    return Ok(tasks);
                }
                after = id;
                let Ok(spec) = StackSpec::from_storage_value(spec) else {
                    continue;
                };
                let Ok(source) = source
                    .map(StackReleaseSource::from_storage_value)
                    .transpose()
                else {
                    continue;
                };
                if let Ok(files) = super::super::stacks::update_scanner::applied_compose_files(
                    self.git.as_deref(),
                    &spec,
                    source.as_ref(),
                    cancel,
                )
                .await
                    && let Ok(checks) = citadel_stacks::stack_checks_for_compose(&spec, &files)
                {
                    tasks.extend(checks.into_iter().map(|check| ImageScanTask {
                        platform,
                        registry: check.key.registry_id,
                        reference: check.image_name,
                    }));
                }
            }
            if !full {
                break;
            }
        }
        self.external_tasks(true, cancel, &mut tasks).await?;
        Ok(tasks)
    }
    async fn external_tasks(
        &self,
        swarm: bool,
        cancel: &CancellationToken,
        tasks: &mut Vec<ImageScanTask>,
    ) -> Result<(), sqlx::Error> {
        let mut after = Uuid::nil();
        while !cancel.is_cancelled() {
            let page = image_scan::external_images(&self.pool, after, swarm).await?;
            let full = page.len() == image_scan::IMAGE_PAGE as usize;
            for (id, platform, registry, reference) in page {
                after = id;
                if let Some(registry) = registry.and_then(|v| Uuid::parse_str(&v).ok())
                    && let Some(reference) = reference
                {
                    tasks.push(ImageScanTask {
                        platform,
                        registry,
                        reference,
                    });
                }
            }
            if !full {
                break;
            }
        }
        Ok(())
    }
}
async fn scan_unique(
    runtime: &dyn ImageScanRuntime,
    cache: &ImageDigestCache,
    tasks: Vec<ImageScanTask>,
    cancel: &CancellationToken,
) -> usize {
    let mut unique = BTreeMap::new();
    for task in tasks {
        if let Some(key) =
            crate::connectors::registries::digest_cache::key(task.registry, &task.reference)
        {
            unique.entry(key).or_insert(task);
        }
    }
    let mut count = 0;
    for task in unique.into_values() {
        let started = chrono::Utc::now();
        let result = tokio::select! {biased;()=cancel.cancelled()=>break,result=tokio::time::timeout(Duration::from_secs(30),runtime.inspect(&task,cancel))=>result};
        match result {
            Ok(Ok(digest)) => {
                cache.set(task.registry, &task.reference, digest, started);
                count += 1;
            }
            _ => {
                tracing::warn!(platform_id=%task.platform,"Registry image scan failed; retaining the last successful observation")
            }
        }
    }
    count
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Runtime(Mutex<Vec<String>>);
    impl ImageScanRuntime for Runtime {
        fn inspect<'a>(
            &'a self,
            task: &'a ImageScanTask,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<String, String>> {
            self.0.lock().unwrap().push(task.reference.clone());
            Box::pin(async move {
                if task.reference == "bad" {
                    Err("unavailable".into())
                } else {
                    Ok("sha256:remote".into())
                }
            })
        }
    }
    #[tokio::test]
    async fn scanner_inspects_unique_image_once_across_platforms_and_continues_after_failure() {
        let runtime = Runtime::default();
        let cache = ImageDigestCache::default();
        let registry = Uuid::now_v7();
        let tasks = ["nginx", "nginx:latest", "bad", "redis"]
            .into_iter()
            .map(|reference| ImageScanTask {
                platform: Uuid::now_v7(),
                registry,
                reference: reference.into(),
            })
            .collect();
        assert_eq!(
            scan_unique(&runtime, &cache, tasks, &CancellationToken::new()).await,
            2
        );
        assert_eq!(runtime.0.lock().unwrap().len(), 3);
        assert_eq!(
            cache.get(registry, "nginx").unwrap().digest,
            "sha256:remote"
        );
        assert!(cache.get(registry, "bad").is_none());
    }
}
