mod agents;
pub(crate) use agents::*;

mod stats;
pub(crate) use stats::*;

pub(crate) mod event_refresh;
pub(crate) use event_refresh::*;
pub(crate) mod scoped_reconciler;
pub(crate) mod swarm_reconciliation;
pub(crate) use scoped_reconciler::*;

mod events;
pub(crate) use events::*;

#[cfg(test)]
mod state_delta_tests;

mod health;
pub(crate) use health::*;
mod lifecycle;
pub(crate) use lifecycle::*;

mod inventory;
pub(crate) use inventory::*;

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(test)]
use super::test_support::PgPool;
use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_alerts::{AlertEventSink, AlertObservation};
use citadel_docker::client::DockerClient;
use citadel_docker::client::DockerError;
use citadel_docker::client::events::ContainerChange;
use citadel_docker::client::events::RuntimeEventKind;
use citadel_platforms::jobs::observations::{PlatformLifecycleStore, RuntimeProjectionStore};
use citadel_platforms::jobs::{DeltaOutcome, EventRefresh, ReconciliationDecision, event_decision};
use citadel_platforms::{PlatformInventoryPort, RuntimeCapabilityError};
use citadel_runtime::{BoundedReceiver, BoundedSender, QueueOverflowPolicy, bounded_channel};
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

use citadel_adapters::connectors::routing::platforms::registry::{
    HEALTH_CONCURRENCY, PlatformRuntimeRegistry, PlatformTarget as ReconciliationTarget,
    STATS_CONCURRENCY,
};
use citadel_platforms::PlatformHealthPort;
use citadel_runtime::{IoBudget, runtime_metrics::RuntimeWork};

use crate::metrics::Metrics;
use crate::realtime::RealtimeHub;

#[derive(Clone)]
pub(crate) struct StatsWorkerContext {
    pub(crate) targets: Arc<PlatformRuntimeRegistry>,
    pub(crate) ingress: super::statistics::StatsIngress,
    pub(crate) metrics: Arc<Metrics>,
    pub(crate) fetch_interval: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReconciliationTrigger {
    LocalEvent,
    AgentEvent,
}

#[derive(Clone)]
pub(crate) struct AgentReconciliationSignal {
    pub(crate) queue: BoundedSender<Option<uuid::Uuid>>,
    pub(crate) full: Arc<std::sync::atomic::AtomicBool>,
}
impl AgentReconciliationSignal {
    fn request(&self, platform: Option<uuid::Uuid>) {
        if self.queue.try_send(platform).is_err() {
            // The queue is already waking the consumer. Upgrade that pass to all
            // persisted Agents instead of silently losing a platform scope.
            self.full.store(true, std::sync::atomic::Ordering::Release);
            // If the consumer drained between try_send and the flag write,
            // ensure it receives another wake instead of waiting for the timer.
            let _ = self.queue.try_send(None);
        }
    }
}

#[derive(Debug)]
pub(crate) struct InventoryEvent {
    pub resource: Option<citadel_platforms::jobs::ResourceDelta>,
    stream_recovered: bool,
    swarm_scope: bool,
    kind: RuntimeEventKind,
    platform_id: Option<uuid::Uuid>,
    container_id: Option<String>,
    container_state: Option<String>,
    container_name: Option<String>,
    container: Option<citadel_platforms::RuntimeContainerSummary>,
    source: ReconciliationTrigger,
    action: String,
    event_time_millis: Option<u128>,
}

#[cfg(test)]
pub(crate) use super::test_support::observe_platform_metrics;

async fn wait_to_reconnect(cancellation: &CancellationToken, reconnect_delay: Duration) -> bool {
    tokio::select! {
        biased;
        () = cancellation.cancelled() => true,
        () = tokio::time::sleep(reconnect_delay) => false,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod health_tests {
    use super::HealthState;
    #[test]
    fn transient_failures_do_not_flap_and_confirmed_transitions_emit_once() {
        let mut state = HealthState::default();
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(true), Some(true));
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(false), Some(false));
        assert_eq!(state.observe(false), None);
        assert_eq!(state.observe(true), None);
        assert_eq!(state.observe(true), Some(true));
        assert_eq!(state.observe(true), None);
    }
}

#[cfg(test)]
mod subscription_tests;

#[cfg(test)]
mod inventory_tests;

#[cfg(test)]
mod event_tests;

#[cfg(test)]
mod event_policy_tests;
