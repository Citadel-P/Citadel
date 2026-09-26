use super::PLATFORM_RESOURCE_TYPE;
use crate::metrics::Metrics;
use axum::{Router, extract::State, response::Response};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Coarse invalidations for resource routers whose writes finish inside the
/// handler. Jobs publish their claim/completion separately; no payload or IDs
/// are broadcast here, and every follow-up read applies its normal ACL.
pub fn notify_mutations(router: Router, resource_type: &'static str) -> Router {
    router.layer(axum::middleware::from_fn_with_state(
        resource_type,
        mutation_notification,
    ))
}

async fn mutation_notification(
    State(resource_type): State<&'static str>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let hub = (!request.method().is_safe())
        .then(|| request.extensions().get::<RealtimeHub>().cloned())
        .flatten();
    let response = next.run(request).await;
    if response.status().is_success()
        && let Some(hub) = hub
    {
        hub.publish_resource_change(resource_type, Uuid::nil(), "resourceChanged");
    }
    response
}

pub fn change_callback(
    hub: Option<RealtimeHub>,
    resource_type: &'static str,
) -> Arc<dyn Fn() + Send + Sync> {
    Arc::new(move || {
        if let Some(hub) = &hub {
            hub.publish_resource_change(resource_type, Uuid::nil(), "resourceChanged");
        }
    })
}

pub fn build_log_callback(hub: Option<RealtimeHub>) -> citadel_builds::BuildLogNotifier {
    Arc::new(move |run_id, entry| {
        if let Some(hub) = &hub {
            hub.publish_build_log(run_id, entry);
        }
    })
}

#[derive(Debug, Clone)]
pub struct PublishedRuntimeEvent {
    pub(crate) platform_id: Option<Uuid>,
    pub(crate) resource_type: &'static str,
    pub(crate) resource_id: Uuid,
    pub(crate) event_kind: &'static str,
    pub(crate) resource_revision: u64,
    pub(crate) payload: Value,
    // Shared only for this committed invalidation; readers still enforce ACLs.
    pub(crate) containers: Arc<tokio::sync::OnceCell<Vec<citadel_platforms::ContainerDetails>>>,
}

impl PublishedRuntimeEvent {
    pub(crate) fn container_ids(&self) -> Option<Vec<String>> {
        if self.event_kind != "runtimeChanged" || self.payload["dockerResourceType"] != "container"
        {
            return None;
        }
        if let Some(ids) = self.payload["runtimeResourceIds"].as_array() {
            return Some(
                ids.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            );
        }
        self.payload["runtimeResourceId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(|id| vec![id.to_owned()])
    }

    pub(crate) fn affects_resource(&self, id: Option<Uuid>) -> bool {
        if let Some(ids) = self.payload["resourceIds"].as_array() {
            return id.is_none_or(|id| ids.contains(&json!(id)));
        }
        self.resource_id.is_nil() || id.is_none_or(|id| id == self.resource_id)
    }

    #[must_use]
    pub fn resource_type(&self) -> &'static str {
        self.resource_type
    }
}

#[derive(Clone)]
pub struct RealtimeHub {
    inner: Arc<RealtimeHubInner>,
}

struct RealtimeHubInner {
    revision: AtomicU64,
    sender: broadcast::Sender<Arc<PublishedRuntimeEvent>>,
    metrics: Arc<Metrics>,
    pending_containers: std::sync::Mutex<Option<ContainerChanges>>,
}

type ContainerChanges = BTreeMap<(Uuid, String), BTreeSet<String>>;

impl RealtimeHub {
    #[must_use]
    pub fn new(capacity: usize, metrics: Arc<Metrics>) -> Self {
        let (sender, receiver) = broadcast::channel(capacity);
        drop(receiver);
        Self {
            inner: Arc::new(RealtimeHubInner {
                revision: AtomicU64::new(0),
                sender,
                metrics,
                pending_containers: Default::default(),
            }),
        }
    }

    #[must_use]
    pub fn current_revision(&self) -> u64 {
        self.inner.revision.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<PublishedRuntimeEvent>> {
        self.inner.sender.subscribe()
    }

    pub fn publish_runtime_change(
        &self,
        platform_id: Uuid,
        docker_resource_type: impl Into<String>,
        action: impl Into<String>,
        runtime_resource_id: impl Into<String>,
    ) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            Some(platform_id),
            PLATFORM_RESOURCE_TYPE,
            platform_id,
            "runtimeChanged",
            json!({
                "dockerResourceType": docker_resource_type.into(),
                "action": action.into(),
                "runtimeResourceId": runtime_resource_id.into(),
            }),
        )
    }

    /// A claim or completion is already a batch; publish it immediately.
    pub fn publish_container_changes(
        &self,
        platform_id: Uuid,
        action: &str,
        ids: &[String],
    ) -> u64 {
        if ids.is_empty() || self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            Some(platform_id),
            PLATFORM_RESOURCE_TYPE,
            platform_id,
            "runtimeChanged",
            json!({"dockerResourceType":"container", "action":action,"runtimeResourceIds":ids}),
        )
    }

    /// Coalesce committed Docker observations within a fixed window. This does
    /// not delay mutation claims, audit events, logs, or terminal streams.
    pub fn publish_container_observation(&self, platform_id: Uuid, action: &str, id: String) {
        if id.is_empty() || self.inner.sender.receiver_count() == 0 {
            return;
        }
        let mut pending = self.inner.pending_containers.lock().unwrap();
        let schedule = pending.is_none();
        let changes = pending.get_or_insert_with(Default::default);
        changes
            .entry((platform_id, action.to_owned()))
            .or_default()
            .insert(id);
        // Bound retained IDs even during a sustained event burst.
        let flush = (changes.values().map(BTreeSet::len).sum::<usize>() >= 1024)
            .then(|| std::mem::take(changes));
        drop(pending);
        if let Some(changes) = flush {
            self.flush_container_changes(changes);
        }
        if schedule {
            let hub = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let changes = hub
                    .inner
                    .pending_containers
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap_or_default();
                hub.flush_container_changes(changes);
            });
        }
    }

    fn flush_container_changes(&self, changes: ContainerChanges) {
        for ((platform, action), ids) in changes {
            self.publish_container_changes(platform, &action, &ids.into_iter().collect::<Vec<_>>());
        }
    }

    pub fn publish_resource_changes(&self, resource_type: &'static str, ids: &[Uuid]) -> u64 {
        if ids.is_empty() || self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            None,
            resource_type,
            Uuid::nil(),
            "updated",
            json!({"resourceIds":ids}),
        )
    }

    pub fn publish_container_stats(
        &self,
        platform_id: Uuid,
        stats: &[citadel_platforms::RuntimeContainerStat],
    ) -> u64 {
        self.publish_scoped_container_stats(platform_id, None, stats)
    }

    pub fn publish_scoped_container_stats(
        &self,
        platform_id: Uuid,
        node_id: Option<&str>,
        stats: &[citadel_platforms::RuntimeContainerStat],
    ) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            Some(platform_id),
            PLATFORM_RESOURCE_TYPE,
            platform_id,
            "runtimeChanged",
            json!({
                "dockerResourceType": "containerStats",
                "dockerNodeId": node_id,
                "action": "sample",
                "runtimeResourceId": platform_id,
                "stats": stats,
            }),
        )
    }

    pub fn publish_resource_change(
        &self,
        resource_type: &'static str,
        resource_id: Uuid,
        event_kind: &'static str,
    ) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(None, resource_type, resource_id, event_kind, json!({}))
    }

    pub fn publish_build_log(&self, run_id: Uuid, entry: citadel_builds::BuildLogEntry) -> u64 {
        if self.inner.sender.receiver_count() == 0 {
            return self.current_revision();
        }
        self.publish(
            None,
            "Build",
            run_id,
            "buildLogs",
            json!({"entries":[crate::api::resources::builds::views::BuildLogEntry::from(entry)]}),
        )
    }

    fn publish(
        &self,
        platform_id: Option<Uuid>,
        resource_type: &'static str,
        resource_id: Uuid,
        event_kind: &'static str,
        payload: Value,
    ) -> u64 {
        if event_kind == "runtimeChanged" {
            citadel_runtime::runtime_metrics::RuntimeWork::RealtimeRuntimeInvalidation.units(1);
        }
        let revision = self.inner.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let event = Arc::new(PublishedRuntimeEvent {
            platform_id,
            resource_type,
            resource_id,
            event_kind,
            resource_revision: revision,
            payload,
            containers: Default::default(),
        });
        let _ = self.inner.sender.send(event);
        self.inner.metrics.realtime_event_published();
        revision
    }
}

#[cfg(test)]
mod stats_notification_tests {
    use super::*;

    #[tokio::test]
    async fn observations_batch_and_deduplicate_without_delaying_claims_or_logs() {
        let hub = RealtimeHub::new(16, Arc::new(Metrics::default()));
        let platform = Uuid::now_v7();
        let other = Uuid::now_v7();
        let ids: Vec<_> = (0..5).map(|i| format!("docker-{i}")).collect();
        let mut a = hub.subscribe();
        let mut b = hub.subscribe();
        for id in &ids {
            hub.publish_container_observation(platform, "die", id.clone());
            hub.publish_container_observation(platform, "die", id.clone());
        }
        hub.publish_container_observation(other, "die", "other".into());
        hub.publish_container_changes(platform, "update", &ids);
        assert_eq!(a.try_recv().unwrap().container_ids().unwrap(), ids);
        assert_eq!(b.try_recv().unwrap().container_ids().unwrap(), ids);
        hub.publish_resource_change("Build", Uuid::now_v7(), "buildLogs");
        assert_eq!(a.try_recv().unwrap().event_kind, "buildLogs");
        b.try_recv().unwrap();
        let mut seen = BTreeMap::new();
        for _ in 0..2 {
            let event = tokio::time::timeout(std::time::Duration::from_secs(2), a.recv())
                .await
                .unwrap()
                .unwrap();
            let same = b.recv().await.unwrap();
            assert!(
                Arc::ptr_eq(&event, &same),
                "all connections share one invalidation"
            );
            seen.insert(event.platform_id.unwrap(), event.container_ids().unwrap());
        }
        assert_eq!(seen[&platform], ids);
        assert_eq!(seen[&other], vec!["other"]);
        assert!(a.try_recv().is_err());
        // The next fixed window must start after the previous batch was drained.
        hub.publish_container_observation(platform, "start", "new".into());
        let event = tokio::time::timeout(std::time::Duration::from_secs(2), a.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.container_ids().unwrap(), vec!["new"]);
    }

    #[tokio::test]
    async fn sustained_observations_flush_at_the_capacity_bound_without_losing_ids() {
        let hub = RealtimeHub::new(8, Arc::new(Metrics::default()));
        let platform = Uuid::now_v7();
        let mut receiver = hub.subscribe();
        for i in 0..1025 {
            hub.publish_container_observation(platform, "die", i.to_string());
        }
        assert_eq!(
            receiver.try_recv().unwrap().container_ids().unwrap().len(),
            1024
        );
        let final_event = tokio::time::timeout(std::time::Duration::from_secs(2), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(final_event.container_ids().unwrap(), vec!["1024"]);
    }

    // ContainerStatsWriterJobTests: no subscribers means no notification payload.
    #[test]
    fn statistics_notifications_only_advance_with_subscribers() {
        let hub = RealtimeHub::new(8, Arc::new(Metrics::default()));
        let platform = Uuid::now_v7();
        assert_eq!(hub.publish_container_stats(platform, &[]), 0);
        assert_eq!(
            hub.publish_scoped_container_stats(platform, Some("node-a"), &[]),
            0
        );
        let mut subscriber = hub.subscribe();
        assert_eq!(
            hub.publish_scoped_container_stats(platform, Some("node-a"), &[]),
            1
        );
        let event = subscriber.try_recv().unwrap();
        assert_eq!(event.platform_id, Some(platform));
        assert_eq!(event.payload["dockerNodeId"], "node-a");
        assert_eq!(event.payload["stats"], json!([]));
        drop(subscriber);
        assert_eq!(hub.publish_container_stats(platform, &[]), 1);
    }
}
