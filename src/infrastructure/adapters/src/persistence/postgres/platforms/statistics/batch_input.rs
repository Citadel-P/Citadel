//! Borrowed wire inputs: scope locks and metadata never carry nested telemetry.
use citadel_platforms::stats_ingestion::{PlatformStatsSample, StatsScope};
use citadel_platforms::{ContainerIoSnapshot, RuntimePlatformStats};
use serde::Serialize;
use uuid::Uuid;

pub(super) fn live(scope: &StatsScope) -> bool {
    !scope
        .closed
        .as_ref()
        .is_some_and(|token| token.is_cancelled())
}

pub(super) fn scope_ids<'a>(
    scopes: impl Iterator<Item = &'a StatsScope>,
) -> (Vec<Uuid>, Vec<Uuid>) {
    let mut platforms = Vec::new();
    let mut agents = Vec::new();
    for scope in scopes {
        platforms.push(scope.platform_id);
        agents.extend(scope.agent_id);
    }
    platforms.sort_unstable();
    platforms.dedup();
    agents.sort_unstable();
    agents.dedup();
    (platforms, agents)
}

#[derive(Serialize)]
pub(super) struct MetadataInput<'a> {
    #[serde(flatten)]
    scope: &'a StatsScope,
    created: i64,
    metadata: Metadata,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Metadata {
    network_count: i32,
    volume_count: i32,
    image_count: i64,
    mem_total: i64,
    container_count: i64,
    containers_running: i64,
    containers_paused: i64,
    containers_stopped: i64,
    image_used_bytes: Option<i64>,
    volume_used_bytes: Option<i64>,
}
impl<'a> MetadataInput<'a> {
    pub fn from_sample(sample: &'a PlatformStatsSample) -> Option<Self> {
        let m = sample.metadata.as_ref()?;
        // Node metadata never updates the host projection.
        if sample.scope.node_id.is_some() {
            return None;
        }
        Some(Self {
            scope: &sample.scope,
            created: sample.created,
            metadata: Metadata {
                network_count: m.network_count,
                volume_count: m.volume_count,
                image_count: m.image_count,
                mem_total: m.mem_total,
                container_count: m.container_count,
                containers_running: m.containers_running,
                containers_paused: m.containers_paused,
                containers_stopped: m.containers_stopped,
                image_used_bytes: m.image_used_bytes,
                volume_used_bytes: m.volume_used_bytes,
            },
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureMetadata {
    mem_total: i64,
    disk_used_bytes: Option<i64>,
    disk_total_bytes: Option<i64>,
    disk_usage: Option<f64>,
}
impl From<&RuntimePlatformStats> for CaptureMetadata {
    fn from(metadata: &RuntimePlatformStats) -> Self {
        let disk = metadata.disk();
        Self {
            mem_total: metadata.mem_total,
            disk_used_bytes: disk.map(|d| d.used_bytes),
            disk_total_bytes: disk.map(|d| d.total_bytes),
            disk_usage: disk.map(|d| d.usage_percent),
        }
    }
}
#[derive(Serialize)]
pub(super) struct PlatformCapture<'a> {
    #[serde(flatten)]
    scope: &'a StatsScope,
    created: i64,
    memory_active: f64,
    cpu_usage: f64,
    rx_bytes: f64,
    tx_bytes: f64,
    metadata: Option<CaptureMetadata>,
    container_io: &'a [ContainerIoSnapshot],
}
impl<'a> From<&'a PlatformStatsSample> for PlatformCapture<'a> {
    fn from(s: &'a PlatformStatsSample) -> Self {
        Self {
            scope: &s.scope,
            created: s.created,
            memory_active: s.memory_active,
            cpu_usage: s.cpu_usage,
            rx_bytes: s.rx_bytes,
            tx_bytes: s.tx_bytes,
            metadata: s.metadata.as_ref().map(Into::into),
            container_io: &s.container_io,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_platforms::stats_ingestion::PlatformStatsSample;

    fn sample(containers: usize) -> PlatformStatsSample {
        PlatformStatsSample {
            scope: StatsScope {
                platform_id: Uuid::from_u128(1),
                node_id: None,
                connector: "EdgeAgent".into(),
                address: None,
                agent_id: Some(Uuid::from_u128(2)),
                connected_at: Some(chrono::Utc::now()),
                closed: None,
            },
            created: 100,
            memory_active: 123.,
            cpu_usage: 2.,
            rx_bytes: 4.,
            tx_bytes: 5.,
            metadata: Some(RuntimePlatformStats {
                mem_total: 1000,
                disk_used_bytes: Some(50),
                disk_total_bytes: Some(100),
                disk_usage: Some(50.),
                ..Default::default()
            }),
            container_io: (0..containers)
                .map(|i| ContainerIoSnapshot {
                    container_id: format!("{i:064x}"),
                    node_id: None,
                    created: None,
                    rx_bytes: i as f64,
                    tx_bytes: i as f64,
                    block_read_bytes: None,
                    block_write_bytes: Some(20.),
                })
                .collect(),
        }
    }
    #[test]
    fn dense_capture_only_submits_nested_observations_once() {
        for containers in [10, 1000, 10000] {
            let sample = sample(containers);
            let metadata = MetadataInput::from_sample(&sample).unwrap();
            let wire = serde_json::to_value(&metadata).unwrap();
            assert!(wire.get("container_io").is_none());
            assert!(wire.get("cpu_usage").is_none());
            assert!(wire["metadata"].get("cpuUsage").is_none());
            let ids = scope_ids([&sample.scope, &sample.scope].into_iter());
            assert_eq!(
                ids,
                (
                    vec![sample.scope.platform_id],
                    vec![sample.scope.agent_id.unwrap()]
                )
            );
            let old_bytes = serde_json::to_vec(&[&sample]).unwrap().len() * 4;
            // PostgreSQL UUID-array encoding: dimensions + element length + UUID bytes.
            let lock_bytes = 2 * (20 + 4 + 16);
            let new_bytes = lock_bytes
                + serde_json::to_vec(&[metadata]).unwrap().len()
                + serde_json::to_vec(&[PlatformCapture::from(&sample)])
                    .unwrap()
                    .len();
            eprintln!(
                "containers={containers} old_argument_bytes={old_bytes} new_argument_bytes={new_bytes}"
            );
            assert!(new_bytes * 3 < old_bytes);
            let capture = serde_json::to_value(PlatformCapture::from(&sample)).unwrap();
            let original = serde_json::to_value(&sample).unwrap();
            for key in [
                "container_io",
                "created",
                "platform_id",
                "node_id",
                "memory_active",
                "cpu_usage",
                "rx_bytes",
                "tx_bytes",
            ] {
                assert_eq!(capture[key], original[key], "{key}");
            }
            assert_eq!(capture["metadata"]["diskUsage"], 50.);
        }
    }
    #[test]
    fn invalid_disk_values_and_closed_scopes_preserve_existing_semantics() {
        let mut sample = sample(1);
        sample.metadata.as_mut().unwrap().disk_total_bytes = Some(0);
        let capture = serde_json::to_value(PlatformCapture::from(&sample)).unwrap();
        for field in ["diskUsedBytes", "diskTotalBytes", "diskUsage"] {
            assert!(capture["metadata"][field].is_null());
        }
        let closed = tokio_util::sync::CancellationToken::new();
        sample.scope.closed = Some(closed.clone());
        assert!(live(&sample.scope));
        closed.cancel();
        assert!(!live(&sample.scope));
        sample.scope.node_id = Some("node".into());
        assert!(MetadataInput::from_sample(&sample).is_none());
    }
}
