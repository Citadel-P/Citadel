//! Process-local causal fences for resource collection and committed mutations. PostgreSQL remains
//! authoritative; a stamp lives only as long as a read or write is in flight.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectionKind {
    Platform,
    Networks,
    Volumes,
    Containers,
    Images,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    platform: Uuid,
    node: Option<String>,
    kind: ProjectionKind,
}

type Counter = AsyncMutex<u64>;
static COUNTERS: OnceLock<Mutex<BTreeMap<Key, Weak<Counter>>>> = OnceLock::new();

fn counter(key: &Key) -> Arc<Counter> {
    let mut counters = COUNTERS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(value) = counters.get(key).and_then(Weak::upgrade) {
        return value;
    }
    // Retain only active operations, plus at most 256 inactive weak entries.
    if counters.len() >= 256 {
        counters.retain(|_, value| value.strong_count() > 0);
    }
    let value = Arc::new(AsyncMutex::new(0));
    counters.insert(key.clone(), Arc::downgrade(&value));
    value
}

#[derive(Debug)]
pub struct SnapshotGeneration {
    key: Key,
    counter: Arc<Counter>,
    value: u64,
}

impl SnapshotGeneration {
    pub async fn capture(platform: Uuid, node: Option<&str>, kind: ProjectionKind) -> Self {
        let key = Key {
            platform,
            node: node.map(str::to_owned),
            kind,
        };
        let counter = counter(&key);
        let value = *counter.lock().await;
        Self {
            key,
            counter,
            value,
        }
    }

    pub fn matches(&self, write: &ProjectionWrite) -> bool {
        self.key == write.key
            && Arc::ptr_eq(&self.counter, &write.counter)
            && self.value == *write.guard
    }
}

/// Acquire before opening a transaction; release immediately after commit.
/// Dropping a failed/rolled-back write does not advance its generation.
pub struct ProjectionWrite {
    key: Key,
    counter: Arc<Counter>,
    guard: OwnedMutexGuard<u64>,
}

impl ProjectionWrite {
    pub async fn begin(platform: Uuid, node: Option<&str>, kind: ProjectionKind) -> Self {
        let key = Key {
            platform,
            node: node.map(str::to_owned),
            kind,
        };
        let counter = counter(&key);
        let guard = counter.clone().lock_owned().await;
        Self {
            key,
            counter,
            guard,
        }
    }
    pub async fn begin_many(
        keys: impl IntoIterator<Item = (Uuid, Option<String>, ProjectionKind)>,
    ) -> Vec<Self> {
        let keys: std::collections::BTreeSet<_> = keys.into_iter().collect();
        let mut writes = Vec::with_capacity(keys.len());
        for (platform, node, kind) in keys {
            writes.push(Self::begin(platform, node.as_deref(), kind).await);
        }
        writes
    }
    pub fn committed(mut self) {
        *self.guard = self
            .guard
            .checked_add(1)
            .expect("projection generation exhausted");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn only_committed_matching_resource_and_node_invalidates_a_snapshot() {
        let id = Uuid::now_v7();
        let stamp = SnapshotGeneration::capture(id, None, ProjectionKind::Containers).await;
        ProjectionWrite::begin(id, None, ProjectionKind::Images)
            .await
            .committed();
        ProjectionWrite::begin(id, Some("worker"), ProjectionKind::Containers)
            .await
            .committed();
        let aborted = ProjectionWrite::begin(id, None, ProjectionKind::Containers).await;
        assert!(stamp.matches(&aborted));
        drop(aborted);
        let committed = ProjectionWrite::begin(id, None, ProjectionKind::Containers).await;
        assert!(stamp.matches(&committed));
        committed.committed();
        assert!(
            !stamp.matches(&ProjectionWrite::begin(id, None, ProjectionKind::Containers).await)
        );
    }
    #[tokio::test]
    async fn every_resource_platform_and_node_has_an_independent_fence() {
        let platform = Uuid::now_v7();
        let other = Uuid::now_v7();
        let kinds = [
            ProjectionKind::Platform,
            ProjectionKind::Networks,
            ProjectionKind::Volumes,
            ProjectionKind::Containers,
            ProjectionKind::Images,
        ];
        let mut stamps = Vec::new();
        for kind in kinds {
            stamps.push(SnapshotGeneration::capture(platform, None, kind).await);
        }
        ProjectionWrite::begin(other, None, ProjectionKind::Containers)
            .await
            .committed();
        ProjectionWrite::begin(platform, Some("worker"), ProjectionKind::Containers)
            .await
            .committed();
        for stamp in &stamps {
            assert!(stamp.matches(&ProjectionWrite::begin(platform, None, stamp.key.kind).await));
        }
        for (i, kind) in kinds.into_iter().enumerate() {
            drop(ProjectionWrite::begin(platform, None, kind).await);
            assert!(stamps[i].matches(&ProjectionWrite::begin(platform, None, kind).await));
            ProjectionWrite::begin(platform, None, kind)
                .await
                .committed();
            for (j, stamp) in stamps.iter().enumerate() {
                assert_eq!(
                    stamp.matches(&ProjectionWrite::begin(platform, None, stamp.key.kind).await),
                    j > i
                );
            }
        }
    }
    #[tokio::test]
    async fn multi_scope_acquisition_deduplicates_and_orders_opposite_batches() {
        let a = (Uuid::now_v7(), None, ProjectionKind::Containers);
        let b = (Uuid::now_v7(), None, ProjectionKind::Images);
        let task = |keys| async move {
            let writes = ProjectionWrite::begin_many(keys).await;
            assert_eq!(writes.len(), 2);
            tokio::task::yield_now().await;
            for write in writes {
                write.committed();
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            tokio::join!(
                task(vec![a.clone(), b.clone(), a.clone()]),
                task(vec![b, a])
            );
        })
        .await
        .unwrap();
    }
}
