//! Shared successful image observations. Cache identity deliberately excludes
//! Platform: registry credentials and repository/tag determine the remote image.
use chrono::{DateTime, Utc};
use std::{collections::BTreeMap, sync::Mutex};
use uuid::Uuid;

const TTL: chrono::Duration = chrono::Duration::days(1);
const MAX_ENTRIES: usize = 4096;
const LOW_WATER_MARK: usize = 3584;
#[derive(Clone, Debug)]
pub struct ImageDigestEntry {
    pub digest: String,
    pub checked_at: DateTime<Utc>,
}
#[derive(Default)]
pub struct ImageDigestCache {
    ready: std::sync::atomic::AtomicBool,
    completed: tokio::sync::Notify,
    entries: Mutex<BTreeMap<(Uuid, String, String), ImageDigestEntry>>,
}
impl ImageDigestCache {
    pub(crate) fn mark_ready(&self) {
        self.ready.store(true, std::sync::atomic::Ordering::Release);
        self.completed.notify_waiters();
    }
    pub(crate) async fn wait_ready(&self, cancel: &tokio_util::sync::CancellationToken) -> bool {
        loop {
            let completed = self.completed.notified();
            tokio::pin!(completed);
            completed.as_mut().enable();
            if self.ready.load(std::sync::atomic::Ordering::Acquire) {
                return true;
            }
            tokio::select! {()=cancel.cancelled()=>return false, _=completed=>{}}
        }
    }
    pub fn get(&self, registry: Uuid, reference: &str) -> Option<ImageDigestEntry> {
        self.get_at(registry, reference, Utc::now())
    }
    fn get_at(
        &self,
        registry: Uuid,
        reference: &str,
        now: DateTime<Utc>,
    ) -> Option<ImageDigestEntry> {
        let key = key(registry, reference)?;
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if entries
            .get(&key)
            .is_some_and(|entry| now - entry.checked_at > TTL)
        {
            entries.remove(&key);
        }
        entries.get(&key).cloned()
    }
    pub fn set(&self, registry: Uuid, reference: &str, digest: String, checked_at: DateTime<Utc>) {
        let Some(key) = key(registry, reference) else {
            return;
        };
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if entries
            .get(&key)
            .is_some_and(|entry| entry.checked_at > checked_at)
        {
            return;
        }
        entries.insert(key, ImageDigestEntry { digest, checked_at });
        if entries.len() > MAX_ENTRIES {
            entries.retain(|_, entry| checked_at - entry.checked_at <= TTL);
            if entries.len() > MAX_ENTRIES {
                let mut oldest: Vec<_> = entries
                    .iter()
                    .map(|(key, entry)| (entry.checked_at, key.clone()))
                    .collect();
                oldest.sort();
                for (_, key) in oldest.into_iter().take(entries.len() - LOW_WATER_MARK) {
                    entries.remove(&key);
                }
            }
        }
    }
}
pub(crate) fn key(registry: Uuid, reference: &str) -> Option<(Uuid, String, String)> {
    let reference = reference.trim();
    if registry.is_nil() || reference.is_empty() || reference.contains('@') {
        return None;
    }
    let (repository, tag) = reference
        .rsplit_once(':')
        .filter(|(_, tag)| !tag.contains('/'))
        .unwrap_or((reference, "latest"));
    if repository.is_empty() || tag.is_empty() {
        return None;
    }
    Some((registry, repository.into(), tag.into()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn initial_scan_barrier_wakes_all_consumers_and_honors_cancellation() {
        let cache = ImageDigestCache::default();
        let cancel = tokio_util::sync::CancellationToken::new();
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(10),
                cache.wait_ready(&cancel)
            )
            .await
            .is_err()
        );
        cancel.cancel();
        assert!(!cache.wait_ready(&cancel).await);
        let cancel = tokio_util::sync::CancellationToken::new();
        let (a, b, ()) = tokio::join!(
            cache.wait_ready(&cancel),
            cache.wait_ready(&cancel),
            async {
                tokio::task::yield_now().await;
                cache.mark_ready();
            }
        );
        assert!(a && b);
    }
    #[test]
    fn cache_expires_after_one_day_and_preserves_newer_observations() {
        let cache = ImageDigestCache::default();
        let registry = Uuid::now_v7();
        let now = Utc::now();
        cache.set(registry, "nginx", "new".into(), now);
        cache.set(
            registry,
            "nginx:latest",
            "old".into(),
            now - chrono::Duration::seconds(1),
        );
        assert_eq!(
            cache.get_at(registry, "nginx:latest", now).unwrap().digest,
            "new"
        );
        assert!(
            cache
                .get_at(registry, "nginx", now + TTL + chrono::Duration::seconds(1))
                .is_none()
        );
        assert!(cache.get(registry, "nginx@sha256:123").is_none());
    }
    #[test]
    fn compaction_is_bounded_and_keeps_recent_entries() {
        let cache = ImageDigestCache::default();
        let registry = Uuid::now_v7();
        let now = Utc::now();
        for i in 0..=MAX_ENTRIES {
            cache.set(
                registry,
                &format!("image-{i}"),
                "digest".into(),
                now + chrono::Duration::seconds(i as i64),
            );
        }
        assert_eq!(cache.entries.lock().unwrap().len(), LOW_WATER_MARK);
        assert!(cache.get(registry, "image-0").is_none());
        assert!(
            cache
                .get(registry, &format!("image-{MAX_ENTRIES}"))
                .is_some()
        );
    }
}
