//! Bounded telemetry ingress and explicit Platform/Container persistence ports.
use crate::{RuntimeCapabilityError, RuntimeContainerStat, RuntimePlatformStats};
use futures_util::future::BoxFuture;
use serde::Serialize;
use uuid::Uuid;

/// Captured at collection; writers revalidate this identity against PostgreSQL.
#[derive(Clone, Debug, Serialize)]
pub struct StatsScope {
    pub platform_id: Uuid,
    pub node_id: Option<String>,
    pub connector: String,
    pub address: Option<String>,
    pub agent_id: Option<Uuid>,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip)]
    pub closed: Option<tokio_util::sync::CancellationToken>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContainerStatsSample {
    #[serde(flatten)]
    pub scope: StatsScope,
    #[serde(flatten)]
    pub sample: RuntimeContainerStat,
}

/// Host totals describe collection time, not the inventory at a later flush.
#[derive(Clone, Debug, Serialize)]
pub struct PlatformStatsSample {
    #[serde(flatten)]
    pub scope: StatsScope,
    pub created: i64,
    pub memory_active: f64,
    pub cpu_usage: f64,
    pub rx_bytes: f64,
    pub tx_bytes: f64,
    pub metadata: Option<RuntimePlatformStats>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StatsWriteOutcome {
    pub persisted: usize,
    pub stale: usize,
}

pub trait StatsBatchStore<T>: Send + Sync {
    fn persist_batch<'a>(
        &'a self,
        samples: &'a [T],
    ) -> BoxFuture<'a, Result<StatsWriteOutcome, RuntimeCapabilityError>>;
}
