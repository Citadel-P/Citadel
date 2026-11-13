//! Conservative ingress accounting, shared by both persistence writers.
use citadel_platforms::{RuntimeContainerStat, stats_ingestion::StatsScope};
use citadel_runtime::runtime_metrics::{BufferedBytes, RuntimeWork};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub const RETAINED_BYTES: usize = 64 * 1024 * 1024;
pub const BATCH_BYTES: usize = 8 * 1024 * 1024;

pub struct Lease {
    _permit: OwnedSemaphorePermit,
    _metric: BufferedBytes,
}
impl Lease {
    pub fn split(parent: &mut Arc<Self>, bytes: usize) -> Arc<Self> {
        let parent = Arc::get_mut(parent).expect("split before sharing telemetry ownership");
        Arc::new(Self {
            _permit: parent
                ._permit
                .split(bytes)
                .expect("reserved cycle covers its parts"),
            _metric: parent
                ._metric
                .split(bytes)
                .expect("metric follows reserved bytes"),
        })
    }
}
#[derive(Clone)]
pub struct Budget {
    semaphore: Arc<Semaphore>,
    last_warning: Arc<AtomicU64>,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(RETAINED_BYTES)),
            last_warning: Arc::new(AtomicU64::new(0)),
        }
    }
}
impl Budget {
    pub fn reject(&self, oversized: bool) {
        if oversized {
            RuntimeWork::StatsOversized.units(1);
        } else {
            RuntimeWork::StatsCapacityDrop.units(1);
        }
        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let last = self.last_warning.load(Ordering::Relaxed);
        if now.saturating_sub(last) >= 60
            && self
                .last_warning
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            tracing::warn!(
                oversized,
                retained_limit = RETAINED_BYTES,
                batch_limit = BATCH_BYTES,
                "Statistics cycle rejected before persistence: telemetry byte budget exhausted"
            );
        }
    }
    pub fn reserve(&self, bytes: usize) -> Option<Arc<Lease>> {
        let bytes = u32::try_from(bytes).ok()?;
        let permit = self.semaphore.clone().try_acquire_many_owned(bytes).ok()?;
        Some(Arc::new(Lease {
            _permit: permit,
            _metric: RuntimeWork::StatsRetainedBytes.retain_bytes(bytes as usize),
        }))
    }
    #[cfg(test)]
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }
}
// Count serialization without allocating a second representation. This avoids
// charging six escaped bytes for every normal Docker-ID byte in dense captures.
struct EncodedSize(usize);
impl std::io::Write for EncodedSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn encoded(value: &impl serde::Serialize) -> usize {
    let mut size = EncodedSize(0);
    if serde_json::to_writer(&mut size, value).is_err() {
        return usize::MAX;
    }
    size.0
}
pub fn scope_bytes(scope: &StatsScope) -> usize {
    encoded(scope)
}
pub fn io_bytes(stat: &RuntimeContainerStat) -> usize {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Io<'a> {
        container_id: &'a str,
        rx_bytes: f64,
        tx_bytes: f64,
        block_read_bytes: Option<f64>,
        block_write_bytes: Option<f64>,
    }
    encoded(&Io {
        container_id: &stat.docker_container_id,
        rx_bytes: stat.rx_bytes,
        tx_bytes: stat.tx_bytes,
        block_read_bytes: stat.block_read_bytes,
        block_write_bytes: stat.block_write_bytes,
    })
    .saturating_add(1)
}
#[cfg(test)]
thread_local! {
    pub(super) static CONTAINER_ESTIMATES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub fn container_bytes(scope: usize, stat: &RuntimeContainerStat) -> usize {
    #[cfg(test)]
    CONTAINER_ESTIMATES.with(|count| count.set(count.get() + 1));
    // Flattening removes two braces and adds one comma. Keep extra allowance
    // for the surrounding batch array, separators and UUID scope-lock arguments.
    scope.saturating_add(encoded(stat)).saturating_add(64)
}
fn scope_owned(scope: &StatsScope) -> usize {
    scope
        .connector
        .capacity()
        .saturating_add(scope.address.as_ref().map_or(0, String::capacity))
        .saturating_add(scope.node_id.as_ref().map_or(0, String::capacity))
}
/// Split one admission into independently reclaimable writer reservations. Each
/// part includes object/vector capacity plus two encoded/driver scratch buffers.
/// The residual reservation covers the original collection until submit returns.
pub fn cycle_bytes(
    scope: &StatsScope,
    stats: &Vec<RuntimeContainerStat>,
    estimates: &Vec<usize>,
    captures: usize,
    capture_bytes: usize,
) -> (usize, usize, usize) {
    assert_eq!(stats.len(), estimates.len());
    let container = stats
        .iter()
        .zip(estimates)
        .fold(128usize, |total, (stat, encoded)| {
            total.saturating_add(
                encoded
                    .saturating_mul(2)
                    .saturating_add(
                        2 * std::mem::size_of::<
                            citadel_platforms::stats_ingestion::ContainerStatsSample,
                        >(),
                    )
                    .saturating_add(scope_owned(scope))
                    .saturating_add(stat.docker_container_id.capacity())
                    .saturating_add(64),
            )
        });
    let platform = stats
        .iter()
        .fold(capture_bytes.saturating_mul(2), |total, stat| {
            total
                .saturating_add(2 * std::mem::size_of::<citadel_platforms::ContainerIoSnapshot>())
                .saturating_add(stat.docker_container_id.capacity())
        })
        .saturating_add(
            captures
                .max(1)
                .saturating_mul(1024usize.saturating_add(scope_owned(scope))),
        );
    let input = stats
        .capacity()
        .saturating_mul(std::mem::size_of::<RuntimeContainerStat>())
        .saturating_add(stats.iter().fold(0usize, |total, s| {
            total.saturating_add(s.docker_container_id.capacity())
        }))
        .saturating_add(
            estimates
                .capacity()
                .saturating_mul(std::mem::size_of::<usize>()),
        )
        .saturating_add(4096)
        .saturating_add(scope_owned(scope));
    (
        container.saturating_add(platform).saturating_add(input),
        container,
        platform,
    )
}
