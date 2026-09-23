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
    StackUpdates,
    StackWebhooks,
    StackRecovery,
    StackDrift,
    DeploymentRecovery,
    ServiceRecovery,
    Readiness,
    EdgeInventory,
    ContainerRecovery,
    VolumeRecovery,
    Pruning,
    AgentStats,
    NotificationReconnect,
}

const FAMILIES: [RuntimeWork; 39] = [
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
    RuntimeWork::StackUpdates,
    RuntimeWork::StackWebhooks,
    RuntimeWork::StackRecovery,
    RuntimeWork::StackDrift,
    RuntimeWork::DeploymentRecovery,
    RuntimeWork::ServiceRecovery,
    RuntimeWork::Readiness,
    RuntimeWork::EdgeInventory,
    RuntimeWork::ContainerRecovery,
    RuntimeWork::VolumeRecovery,
    RuntimeWork::Pruning,
    RuntimeWork::AgentStats,
    RuntimeWork::NotificationReconnect,
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
}
static COUNTERS: [Counters; 39] = [const {
    Counters {
        started: AtomicU64::new(0),
        finished: AtomicU64::new(0),
        elapsed_us: AtomicU64::new(0),
        max_us: AtomicU64::new(0),
        units: AtomicU64::new(0),
        failures: AtomicU64::new(0),
        wait_us: AtomicU64::new(0),
        saturated: AtomicU64::new(0),
    }
}; 39];

impl RuntimeWork {
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
    for family in FAMILIES {
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
