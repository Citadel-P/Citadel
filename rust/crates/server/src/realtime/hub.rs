use super::PLATFORM_RESOURCE_TYPE;
use crate::metrics::Metrics;
use axum::{Router, extract::State, response::Response};
use serde_json::{Value, json};
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
}

impl PublishedRuntimeEvent {
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
}

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
            json!({"entries":[crate::api::builds::views::BuildLogEntry::from(entry)]}),
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
        let revision = self.inner.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let event = Arc::new(PublishedRuntimeEvent {
            platform_id,
            resource_type,
            resource_id,
            event_kind,
            resource_revision: revision,
            payload,
        });
        let _ = self.inner.sender.send(event);
        self.inner.metrics.realtime_event_published();
        revision
    }
}

#[cfg(test)]
mod stats_notification_tests {
    use super::*;

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
