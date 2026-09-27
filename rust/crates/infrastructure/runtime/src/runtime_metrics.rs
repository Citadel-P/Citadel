//! Fixed-cardinality process diagnostics. No resource IDs or caller-provided labels.
use std::{
    fmt::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

#[derive(Clone, Copy, Debug)]
pub enum RuntimeWork {
    Health,
    Inventory,
    LocalStats,
    StatsPersistence,
    AlertFlush,
    Recovery,
    Retention,
    LeaseExpiry,
    Targets,
    AgentConnect,
    AgentReconnect,
    DockerPing,
    DockerInfo,
    DockerVersion,
    DockerList,
    DockerStats,
    DockerEvents,
    DockerStorage,
    AutomationClaim,
    BuildClaim,
    BackupClaim,
    RestoreClaim,
    BuildCompletion,
    ImageScan,
    HostDisk,
    GitSync,
    GitSyncStateRead,
    StackUpdateDuplicate,
    StackUpdates,
    StackWebhooks,
    StackRecovery,
    StackDrift,
    DeploymentRecovery,
    ServiceRecovery,
    ServiceObservation,
    Readiness,
    EdgeInventory,
    ContainerRecovery,
    VolumeRecovery,
    Pruning,
    AgentStats,
    NotificationReconnect,
    DockerRawEvent,
    AgentDaemonEvent,
    ContainerEventIgnored,
    ContainerEventInspect,
    ContainerEventApply,
    // Since CPU refactor Phase 5, units count committed semantic changes (Local/Direct).
    ContainerEventProjection,
    ContainerStateDeltaInput,
    ContainerStateDeltaBatch,
    ContainerParentReconcileDeployment,
    ContainerParentReconcileStack,
    ContainerParentReconcileSkipped,

    ContainerStateDeltaPersist,
    ContainerStateDeltaNoop,
    ContainerStateDeltaUnknownIdentity,
    // Iterations: identity hint lookups; units: hits. SQL still validates every hint.
    RuntimeIdentityLookup,
    RuntimeIdentityFallback,
    InventoryRequestContainer,
    InventoryRequestNetwork,
    InventoryRequestOther,
    InventoryRequestReconnect,
    InventoryRequestCoalesced,
    InventoryLocalEvent,
    InventoryAgentEvent,
    InventoryRecovery,
    ContainerRefreshRetry,
    ImageRefreshRetry,
    ContainerRefreshDiscarded,
    ImageRefreshDiscarded,
    PlatformRefreshRetry,
    PlatformRefreshDiscarded,
    NetworkRefreshRetry,
    NetworkRefreshDiscarded,
    VolumeRefreshRetry,
    VolumeRefreshDiscarded,
    SwarmRefreshRetry,
    SwarmRefreshDiscarded,
    EventPlatformRefresh,
    EventContainersRefresh,
    EventImagesRefresh,
    EventNetworksRefresh,
    EventVolumesRefresh,
    EventSwarmRefresh,
    DockerContainerList,
    DockerFilteredContainerList,
    DockerImageList,
    DockerNetworkList,
    DockerVolumeList,
    ContainerMutation,
    ContainerVerification,
    ContainerRuntimeResolve,
    ContainerIdResolveQuery,
    ContainerObservationBatch,
    ContainerRecoveryQuery,
    HealthDeploymentRecovery, // Retained for dashboard compatibility; no probe-driven work.
    PlatformLifecycle,
    LifecycleDeploymentRecovery,
    StatsCommit,
    StatsCommittedSamples,
    StatsRetry,
    PlatformStatsIngress,
    PlatformStatsFlush,
    PlatformStatsStale,
    ContainerStatsIngress,
    ContainerStatsFlush,
    ContainerStatsStale,
    StatsShutdownDrop,

    AuthorizationScopeLookup,
    AuthorizationResourceLookup,
    AuthorizationNegativeHit,
    AuthorizationScopeQuery,
    AuthorizationGlobalQuery,
    AuthorizationResourceQuery,
    AuthorizationInvalidation,
    RealtimeAuthentication,
    RealtimeAuthorizationInvalidation,
    RealtimePermissionLookup,
    RealtimePermissionMiss,
    RealtimeSharedRead,
    RealtimeObservationInput,
    RealtimeRuntimeInvalidation,
}

const FAMILIES: &[RuntimeWork] = &[
    RuntimeWork::Health,
    RuntimeWork::Inventory,
    RuntimeWork::LocalStats,
    RuntimeWork::StatsPersistence,
    RuntimeWork::AlertFlush,
    RuntimeWork::Recovery,
    RuntimeWork::Retention,
    RuntimeWork::LeaseExpiry,
    RuntimeWork::Targets,
    RuntimeWork::AgentConnect,
    RuntimeWork::AgentReconnect,
    RuntimeWork::DockerPing,
    RuntimeWork::DockerInfo,
    RuntimeWork::DockerVersion,
    RuntimeWork::DockerList,
    RuntimeWork::DockerStats,
    RuntimeWork::DockerEvents,
    RuntimeWork::DockerStorage,
    RuntimeWork::AutomationClaim,
    RuntimeWork::BuildClaim,
    RuntimeWork::BackupClaim,
    RuntimeWork::RestoreClaim,
    RuntimeWork::BuildCompletion,
    RuntimeWork::ImageScan,
    RuntimeWork::HostDisk,
    RuntimeWork::GitSync,
    RuntimeWork::GitSyncStateRead,
    RuntimeWork::StackUpdateDuplicate,
    RuntimeWork::StackUpdates,
    RuntimeWork::StackWebhooks,
    RuntimeWork::StackRecovery,
    RuntimeWork::StackDrift,
    RuntimeWork::DeploymentRecovery,
    RuntimeWork::ServiceRecovery,
    RuntimeWork::ServiceObservation,
    RuntimeWork::Readiness,
    RuntimeWork::EdgeInventory,
    RuntimeWork::ContainerRecovery,
    RuntimeWork::VolumeRecovery,
    RuntimeWork::Pruning,
    RuntimeWork::AgentStats,
    RuntimeWork::NotificationReconnect,
    RuntimeWork::DockerRawEvent,
    RuntimeWork::AgentDaemonEvent,
    RuntimeWork::ContainerEventIgnored,
    RuntimeWork::ContainerEventInspect,
    RuntimeWork::ContainerEventApply,
    RuntimeWork::ContainerEventProjection,
    RuntimeWork::ContainerStateDeltaInput,
    RuntimeWork::ContainerStateDeltaBatch,
    RuntimeWork::ContainerParentReconcileDeployment,
    RuntimeWork::ContainerParentReconcileStack,
    RuntimeWork::ContainerParentReconcileSkipped,
    RuntimeWork::ContainerStateDeltaPersist,
    RuntimeWork::ContainerStateDeltaNoop,
    RuntimeWork::ContainerStateDeltaUnknownIdentity,
    RuntimeWork::RuntimeIdentityLookup,
    RuntimeWork::RuntimeIdentityFallback,
    RuntimeWork::InventoryRequestContainer,
    RuntimeWork::InventoryRequestNetwork,
    RuntimeWork::InventoryRequestOther,
    RuntimeWork::InventoryRequestReconnect,
    RuntimeWork::InventoryRequestCoalesced,
    RuntimeWork::InventoryLocalEvent,
    RuntimeWork::InventoryAgentEvent,
    RuntimeWork::InventoryRecovery,
    RuntimeWork::ContainerRefreshRetry,
    RuntimeWork::ImageRefreshRetry,
    RuntimeWork::ContainerRefreshDiscarded,
    RuntimeWork::ImageRefreshDiscarded,
    RuntimeWork::PlatformRefreshRetry,
    RuntimeWork::PlatformRefreshDiscarded,
    RuntimeWork::NetworkRefreshRetry,
    RuntimeWork::NetworkRefreshDiscarded,
    RuntimeWork::VolumeRefreshRetry,
    RuntimeWork::VolumeRefreshDiscarded,
    RuntimeWork::SwarmRefreshRetry,
    RuntimeWork::SwarmRefreshDiscarded,
    RuntimeWork::EventPlatformRefresh,
    RuntimeWork::EventContainersRefresh,
    RuntimeWork::EventImagesRefresh,
    RuntimeWork::EventNetworksRefresh,
    RuntimeWork::EventVolumesRefresh,
    RuntimeWork::EventSwarmRefresh,
    RuntimeWork::DockerContainerList,
    RuntimeWork::DockerFilteredContainerList,
    RuntimeWork::DockerImageList,
    RuntimeWork::DockerNetworkList,
    RuntimeWork::DockerVolumeList,
    RuntimeWork::ContainerMutation,
    RuntimeWork::ContainerVerification,
    RuntimeWork::ContainerRuntimeResolve,
    RuntimeWork::ContainerIdResolveQuery,
    RuntimeWork::ContainerObservationBatch,
    RuntimeWork::ContainerRecoveryQuery,
    RuntimeWork::HealthDeploymentRecovery,
    RuntimeWork::PlatformLifecycle,
    RuntimeWork::LifecycleDeploymentRecovery,
    RuntimeWork::StatsCommit,
    RuntimeWork::StatsCommittedSamples,
    RuntimeWork::StatsRetry,
    RuntimeWork::PlatformStatsIngress,
    RuntimeWork::PlatformStatsFlush,
    RuntimeWork::PlatformStatsStale,
    RuntimeWork::ContainerStatsIngress,
    RuntimeWork::ContainerStatsFlush,
    RuntimeWork::ContainerStatsStale,
    RuntimeWork::StatsShutdownDrop,
    RuntimeWork::AuthorizationScopeLookup,
    RuntimeWork::AuthorizationResourceLookup,
    RuntimeWork::AuthorizationNegativeHit,
    RuntimeWork::AuthorizationScopeQuery,
    RuntimeWork::AuthorizationGlobalQuery,
    RuntimeWork::AuthorizationResourceQuery,
    RuntimeWork::AuthorizationInvalidation,
    RuntimeWork::RealtimeAuthentication,
    RuntimeWork::RealtimeAuthorizationInvalidation,
    RuntimeWork::RealtimePermissionLookup,
    RuntimeWork::RealtimePermissionMiss,
    RuntimeWork::RealtimeSharedRead,
    RuntimeWork::RealtimeObservationInput,
    RuntimeWork::RealtimeRuntimeInvalidation,
];

struct Counters {
    started: AtomicU64,
    finished: AtomicU64,
    elapsed_us: AtomicU64,
    max_us: AtomicU64,
    units: AtomicU64,
    failures: AtomicU64,
    wait_us: AtomicU64,
    saturated: AtomicU64,
    buffered: AtomicU64,
    queued: AtomicU64,
    successes: AtomicU64,
}
static COUNTERS: [Counters; FAMILIES.len()] = [const {
    Counters {
        started: AtomicU64::new(0),
        finished: AtomicU64::new(0),
        elapsed_us: AtomicU64::new(0),
        max_us: AtomicU64::new(0),
        units: AtomicU64::new(0),
        failures: AtomicU64::new(0),
        wait_us: AtomicU64::new(0),
        saturated: AtomicU64::new(0),
        buffered: AtomicU64::new(0),
        queued: AtomicU64::new(0),
        successes: AtomicU64::new(0),
    }
}; FAMILIES.len()];

impl RuntimeWork {
    pub fn success(self) {
        COUNTERS[self as usize]
            .successes
            .fetch_add(1, Ordering::Relaxed);
    }
    pub fn buffered(self, count: usize) {
        COUNTERS[self as usize]
            .buffered
            .store(count as u64, Ordering::Relaxed);
    }
    pub fn queued(self, count: usize) {
        COUNTERS[self as usize]
            .queued
            .store(count as u64, Ordering::Relaxed);
    }

    pub fn permit_wait(self, duration: std::time::Duration, saturated: bool) {
        let counters = &COUNTERS[self as usize];
        counters.wait_us.fetch_add(
            u64::try_from(duration.as_micros()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        counters
            .saturated
            .fetch_add(u64::from(saturated), Ordering::Relaxed);
    }
    pub fn start(self) -> RuntimeTimer {
        COUNTERS[self as usize]
            .started
            .fetch_add(1, Ordering::Relaxed);
        RuntimeTimer {
            family: self,
            start: Instant::now(),
        }
    }

    pub fn units(self, count: u64) {
        COUNTERS[self as usize]
            .units
            .fetch_add(count, Ordering::Relaxed);
    }

    pub fn failures(self, count: u64) {
        COUNTERS[self as usize]
            .failures
            .fetch_add(count, Ordering::Relaxed);
    }
}

pub struct RuntimeTimer {
    family: RuntimeWork,
    start: Instant,
}

impl Drop for RuntimeTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let micros = u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
        let counters = &COUNTERS[self.family as usize];
        counters.elapsed_us.fetch_add(micros, Ordering::Relaxed);
        counters.max_us.fetch_max(micros, Ordering::Relaxed);
        counters.finished.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(family = ?self.family, duration_us = micros, "Runtime iteration completed");
    }
}

pub fn render_runtime_metrics(output: &mut String) {
    for &family in FAMILIES {
        let counters = &COUNTERS[family as usize];
        let started = counters.started.load(Ordering::Relaxed);
        let finished = counters.finished.load(Ordering::Relaxed);
        for (name, value) in [
            (
                "permit_wait_microseconds_total",
                counters.wait_us.load(Ordering::Relaxed),
            ),
            (
                "saturation_total",
                counters.saturated.load(Ordering::Relaxed),
            ),
            ("buffered", counters.buffered.load(Ordering::Relaxed)),
            ("queued", counters.queued.load(Ordering::Relaxed)),
            (
                "successes_total",
                counters.successes.load(Ordering::Relaxed),
            ),
            ("iterations_total", started),
            ("in_flight", started.saturating_sub(finished)),
            (
                "duration_microseconds_total",
                counters.elapsed_us.load(Ordering::Relaxed),
            ),
            (
                "duration_microseconds_max",
                counters.max_us.load(Ordering::Relaxed),
            ),
            ("units_total", counters.units.load(Ordering::Relaxed)),
            ("failures_total", counters.failures.load(Ordering::Relaxed)),
        ] {
            let _ = writeln!(
                output,
                "citadel_runtime_{name}{{family=\"{family:?}\"}} {value}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_keep_units_failures_waits_and_in_flight_distinct() {
        // No runtime unit test produces Agent daemon events; use that isolated slot.
        let family = RuntimeWork::AgentDaemonEvent;
        family.units(3);
        family.failures(1);
        family.success();
        family.buffered(5);
        family.queued(2);
        family.permit_wait(std::time::Duration::from_micros(25), true);
        let timer = family.start();
        let mut active = String::new();
        render_runtime_metrics(&mut active);
        assert!(active.contains("citadel_runtime_in_flight{family=\"AgentDaemonEvent\"} 1\n"));
        drop(timer);
        let mut finished = String::new();
        render_runtime_metrics(&mut finished);
        for (name, value) in [
            ("units_total", 3),
            ("successes_total", 1),
            ("buffered", 5),
            ("queued", 2),
            ("failures_total", 1),
            ("permit_wait_microseconds_total", 25),
            ("saturation_total", 1),
            ("iterations_total", 1),
            ("in_flight", 0),
        ] {
            assert!(finished.contains(&format!(
                "citadel_runtime_{name}{{family=\"AgentDaemonEvent\"}} {value}\n"
            )));
        }
    }

    #[test]
    fn exported_families_are_unique_and_match_counter_slots() {
        let mut names = std::collections::HashSet::new();
        let mut rendered = String::new();
        render_runtime_metrics(&mut rendered);
        for (index, family) in FAMILIES.iter().enumerate() {
            assert_eq!(*family as usize, index);
            let name = format!("{family:?}");
            assert!(names.insert(name.clone()));
            assert!(
                rendered.contains(&format!("citadel_runtime_units_total{{family=\"{name}\"}}"))
            );
        }
        assert_eq!(rendered.lines().count(), FAMILIES.len() * 11);
    }
}
