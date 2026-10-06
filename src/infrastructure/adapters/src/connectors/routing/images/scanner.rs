//! DeploymentImageScannerJob: one successful registry observation per unique
//! image each cycle, shared by Deployment, Stack, and managed Service checks.
use crate::connectors::registries::digest_cache::ImageDigestCache;
use citadel_deployments::{DeploymentRepository, DeploymentRuntime};
use citadel_primitives::ActorId;
use citadel_stacks::StackRepository;
use citadel_swarm_services::SwarmServiceRepository;
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
        }
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
        let actor = ActorId::new(Uuid::from_u128(1));
        let deployments =
            crate::persistence::postgres::deployments::PostgresDeploymentRepository::new(
                self.pool.clone(),
            );
        let stacks =
            crate::persistence::postgres::stacks::PostgresStackRepository::new(self.pool.clone());
        let services =
            crate::persistence::postgres::swarm_services::PostgresSwarmServiceRepository::new(
                self.pool.clone(),
            );
        let mut tasks = Vec::new();
        for kind in ["Deployment", "Stack", "SwarmService"] {
            let mut after = Uuid::nil();
            loop {
                if cancel.is_cancelled() {
                    return Ok(tasks);
                }
                let ids:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM (SELECT d.id,'Deployment' kind FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE p.platformdescriptor->>'$type'='Docker' AND p.status='Online' AND d.controlstate='Idle' UNION ALL SELECT s.id,'Stack' FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE p.platformdescriptor->>'$type' IN ('Docker','DockerStandalone','DockerSwarm') AND p.status='Online' AND s.controlstate='Idle' UNION ALL SELECT s.id,'SwarmService' FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE p.platformdescriptor->>'$type'='DockerSwarm' AND p.status='Online' AND s.controlstate='Idle') candidates WHERE kind=$1 AND id>$2 ORDER BY id LIMIT 100")
                    .bind(kind).bind(after).fetch_all(&self.pool).await?;
                if ids.is_empty() {
                    break;
                }
                for id in ids {
                    after = id;
                    match kind {
                        "Deployment" => {
                            if let Ok(view) = deployments.get_authorized(actor, true, id).await
                                && view.spec.update_behavior
                                    != citadel_deployments::UpdateBehavior::Disabled
                                && let citadel_deployments::DeploymentImageInfo::External {
                                    registry_id: registry,
                                    image_tag: reference,
                                    ..
                                } = &view.spec.image
                            {
                                tasks.push(ImageScanTask {
                                    platform: view.platform_id,
                                    registry: *registry,
                                    reference: reference.clone(),
                                });
                            }
                        }
                        "Stack" => {
                            if let Ok(view) = stacks.get_authorized(actor, true, id).await
                                && let Some(platform) = view.platform_id
                                && let Ok(checks) =
                                    citadel_stacks::build_manual_stack_checks(&view, true)
                            {
                                tasks.extend(checks.into_iter().map(|check| ImageScanTask {
                                    platform,
                                    registry: check.key.registry_id,
                                    reference: check.image_name,
                                }));
                            }
                        }
                        _ => {
                            if let Ok(view) = services.get_authorized(actor, true, id).await
                                && view.spec.update_behavior
                                    != citadel_swarm_services::UpdateBehavior::Disabled
                                && let citadel_swarm_services::SwarmServiceImageInfo::External {
                                    registry_id: registry,
                                    image_tag: reference,
                                    ..
                                } = &view.spec.image
                            {
                                tasks.push(ImageScanTask {
                                    platform: view.platform_id,
                                    registry: *registry,
                                    reference: reference.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(tasks)
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
