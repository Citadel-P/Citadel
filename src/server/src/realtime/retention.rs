//! Shared accounting for queued events and their lazily populated read caches.
//! This bounds retained work, not transient query/serialization or allocator RSS.
use super::RealtimeReadError;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{OnceCell, OwnedSemaphorePermit, Semaphore};

const RETAINED_BYTES: usize = 32 * 1024 * 1024;
#[derive(Debug, Clone)]
pub(crate) struct Budget(Arc<Semaphore>);
impl Default for Budget {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(RETAINED_BYTES)))
    }
}
impl Budget {
    pub(crate) fn reserve(&self, bytes: usize) -> Result<OwnedSemaphorePermit, Exceeded> {
        let bytes = u32::try_from(bytes.max(1)).map_err(|_| Exceeded)?;
        self.0
            .clone()
            .try_acquire_many_owned(bytes)
            .map_err(|_| Exceeded)
    }
    pub(crate) fn value(
        &self,
        value: &serde_json::Value,
    ) -> Result<OwnedSemaphorePermit, Exceeded> {
        self.reserve(json_bytes(value).saturating_add(1024))
    }
}
#[derive(Debug)]
pub(crate) struct Exceeded;
impl From<Exceeded> for RealtimeReadError {
    fn from(_: Exceeded) -> Self {
        Self::Overloaded
    }
}
fn json_bytes(value: &serde_json::Value) -> usize {
    use serde_json::Value::*;
    let nested = match value {
        String(s) => s.capacity(),
        Array(a) => a
            .capacity()
            .saturating_mul(std::mem::size_of::<serde_json::Value>())
            .saturating_add(a.iter().map(json_bytes).fold(0usize, usize::saturating_add)),
        Object(o) => o.iter().fold(0usize, |n, (k, v)| {
            n.saturating_add(128)
                .saturating_add(k.capacity())
                .saturating_add(json_bytes(v))
        }),
        _ => 0,
    };
    std::mem::size_of::<serde_json::Value>().saturating_add(nested)
}
// A conservative encoding-based estimate for typed DB/runtime projections. No
// second JSON tree or encoded buffer is allocated to perform this accounting.
fn projection_bytes(value: &impl Serialize) -> Result<usize, Exceeded> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| Exceeded)?;
    Ok(counter
        .0
        .saturating_mul(8)
        .saturating_add(std::mem::size_of_val(value))
        .saturating_add(1024))
}
#[derive(Debug)]
struct Retained<T> {
    value: T,
    _permit: OwnedSemaphorePermit,
}
#[derive(Debug)]
pub(crate) struct Cache<T> {
    // Only our own admission rejection is sticky for this event. Loader errors
    // and cancelled initialization leave the cell available for a later attempt.
    cell: OnceCell<Result<Retained<T>, Exceeded>>,
    budget: Budget,
}
impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self::new(Budget::default())
    }
}
impl<T> Cache<T> {
    pub(crate) fn new(budget: Budget) -> Self {
        Self {
            cell: OnceCell::new(),
            budget,
        }
    }
    pub(crate) async fn get_or_try_init<F, Fut>(&self, init: F) -> Result<&T, RealtimeReadError>
    where
        T: Serialize,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, RealtimeReadError>>,
    {
        self.cell
            .get_or_try_init(|| async {
                let value = init().await?;
                let admitted = projection_bytes(&value)
                    .and_then(|bytes| self.budget.reserve(bytes))
                    .map(|permit| Retained {
                        value,
                        _permit: permit,
                    });
                // Store a small rejection marker, dropping the rejected projection.
                // Other subscribers must not repeat this event's expensive read.
                Ok::<_, RealtimeReadError>(admitted)
            })
            .await?
            .as_ref()
            .map(|entry| &entry.value)
            .map_err(|_| RealtimeReadError::Overloaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn concurrent_readers_share_one_value_and_release_its_permit() {
        let budget = Budget(Arc::new(Semaphore::new(8192)));
        let cache = Cache::new(budget.clone());
        let loads = AtomicUsize::new(0);
        let values = futures_util::future::join_all((0..8).map(|_| {
            cache.get_or_try_init(|| async {
                loads.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
                Ok("x".repeat(600))
            })
        }))
        .await;
        assert_eq!(loads.load(Ordering::SeqCst), 1);
        let first = values[0].as_ref().unwrap();
        for value in &values {
            assert!(std::ptr::eq(*first, *value.as_ref().unwrap()));
        }
        assert!(budget.0.available_permits() < 8192);
        drop(values);
        drop(cache);
        assert_eq!(budget.0.available_permits(), 8192);
    }

    #[tokio::test]
    async fn admission_rejection_is_shared_until_a_new_event() {
        // Oversized projection, then a fitting projection with capacity held elsewhere.
        for (held_bytes, value_len) in [(0, 1200), (4096, 600)] {
            let budget = Budget(Arc::new(Semaphore::new(8192)));
            let held = (held_bytes > 0).then(|| budget.reserve(held_bytes).unwrap());
            let cache = Cache::new(budget.clone());
            let loads = AtomicUsize::new(0);
            let results = futures_util::future::join_all((0..8).map(|_| {
                cache.get_or_try_init(|| async {
                    loads.fetch_add(1, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                    Ok("x".repeat(value_len))
                })
            }))
            .await;
            assert!(
                results
                    .iter()
                    .all(|r| matches!(r, Err(RealtimeReadError::Overloaded)))
            );
            assert_eq!(loads.load(Ordering::SeqCst), 1);
            assert!(
                matches!(cache.cell.get(), Some(Err(Exceeded))),
                "only the marker is retained"
            );
            assert_eq!(budget.0.available_permits(), 8192 - held_bytes);
            drop(held);
            assert!(matches!(
                cache
                    .get_or_try_init(|| async {
                        panic!("capacity recovery must not reload the same event");
                        #[allow(unreachable_code)]
                        Ok(String::new())
                    })
                    .await,
                Err(RealtimeReadError::Overloaded)
            ));
            let fresh = Cache::new(budget.clone());
            assert_eq!(
                fresh
                    .get_or_try_init(|| async { Ok("x".repeat(600)) })
                    .await
                    .unwrap()
                    .len(),
                600
            );
            drop(fresh);
            drop(cache);
            assert_eq!(budget.0.available_permits(), 8192);
        }
    }

    #[tokio::test]
    async fn loader_errors_are_not_remembered_as_admission_rejections() {
        let budget = Budget(Arc::new(Semaphore::new(8192)));
        let cache = Cache::<String>::new(budget.clone());
        for error in [
            RealtimeReadError::Authentication,
            RealtimeReadError::Authorization,
            RealtimeReadError::Storage("unavailable".into()),
            RealtimeReadError::Overloaded,
        ] {
            assert!(
                cache
                    .get_or_try_init(|| async { Err(error) })
                    .await
                    .is_err()
            );
            assert!(cache.cell.get().is_none());
            assert_eq!(budget.0.available_permits(), 8192);
        }
        cache
            .get_or_try_init(|| async { Ok(String::new()) })
            .await
            .unwrap();
        drop(cache);
        assert_eq!(budget.0.available_permits(), 8192);
    }
    #[tokio::test]
    async fn cancelling_a_read_does_not_retain_a_cache_or_reservation() {
        let budget = Budget(Arc::new(Semaphore::new(8192)));
        let cache = Cache::<String>::new(budget.clone());
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(10),
            cache.get_or_try_init(std::future::pending::<Result<String, RealtimeReadError>>),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(budget.0.available_permits(), 8192);
        cache
            .get_or_try_init(|| async { Ok::<_, RealtimeReadError>(String::new()) })
            .await
            .unwrap();
        drop(cache);
        assert_eq!(budget.0.available_permits(), 8192);
    }
}
