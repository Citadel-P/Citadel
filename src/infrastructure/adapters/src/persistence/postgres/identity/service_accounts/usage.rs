use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};
use citadel_identity::{ServiceAccountLastUsedStore, ServiceAccountLastUsedTracker};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use citadel_runtime::BoundedReceiver;
use citadel_runtime::BoundedSender;
use citadel_runtime::QueueOverflowPolicy;
use citadel_runtime::bounded_channel;

#[derive(Debug, Clone, Copy)]
struct UsageCandidate {
    credential_id: Uuid,
    used_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct BoundedServiceAccountLastUsedTracker {
    sender: BoundedSender<UsageCandidate>,
}

pub struct ServiceAccountLastUsedWorker {
    receiver: BoundedReceiver<UsageCandidate>,
    store: Arc<dyn ServiceAccountLastUsedStore>,
    capacity: usize,
    flush_interval: StdDuration,
    coalesce_window: StdDuration,
}

struct UsageState {
    candidate: DateTime<Utc>,
    last_persisted: Option<Instant>,
    dirty: bool,
}

#[must_use]
pub fn service_account_last_used_channel(
    store: Arc<dyn ServiceAccountLastUsedStore>,
    capacity: usize,
    flush_interval: StdDuration,
    coalesce_window: StdDuration,
) -> (
    BoundedServiceAccountLastUsedTracker,
    ServiceAccountLastUsedWorker,
) {
    assert!(capacity > 0, "last-used capacity must be greater than zero");
    assert!(
        !flush_interval.is_zero(),
        "last-used flush interval must be greater than zero"
    );
    assert!(
        !coalesce_window.is_zero(),
        "last-used coalesce window must be greater than zero"
    );
    let (sender, receiver) = bounded_channel(capacity, QueueOverflowPolicy::Reject);
    (
        BoundedServiceAccountLastUsedTracker { sender },
        ServiceAccountLastUsedWorker {
            receiver,
            store,
            capacity,
            flush_interval,
            coalesce_window,
        },
    )
}

impl ServiceAccountLastUsedTracker for BoundedServiceAccountLastUsedTracker {
    fn track(&self, credential_id: Uuid, used_at: DateTime<Utc>) {
        if self
            .sender
            .try_send(UsageCandidate {
                credential_id,
                used_at,
            })
            .is_err()
        {
            citadel_runtime::runtime_metrics::RuntimeWork::ServiceAccountUsageDropped.units(1);
        }
    }
}

impl ServiceAccountLastUsedWorker {
    pub async fn run(
        mut self,
        cancellation: CancellationToken,
    ) -> Result<(), std::convert::Infallible> {
        let mut states = HashMap::<Uuid, UsageState>::with_capacity(self.capacity.min(1_024));
        let mut ticker = tokio::time::interval(self.flush_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                candidate = self.receiver.recv(&cancellation) => {
                    let Some(candidate) = candidate else { return Ok(()); };
                    if let Some(state) = states.get_mut(&candidate.credential_id) {
                        state.candidate = state.candidate.max(candidate.used_at);
                        state.dirty = true;
                    } else if states.len() < self.capacity {
                        states.insert(candidate.credential_id, UsageState {
                            candidate: candidate.used_at,
                            last_persisted: None,
                            dirty: true,
                        });
                    } else {
                        citadel_runtime::runtime_metrics::RuntimeWork::ServiceAccountUsageDropped.units(1);
                    }
                }
                _ = ticker.tick() => {
                    self.flush_due(&mut states).await;
                }
            }
        }
    }

    async fn flush_due(&self, states: &mut HashMap<Uuid, UsageState>) {
        let now = Instant::now();
        let due = states
            .iter()
            .filter_map(|(credential_id, state)| {
                let window_elapsed = state
                    .last_persisted
                    .is_none_or(|last| now.duration_since(last) >= self.coalesce_window);
                (state.dirty && window_elapsed).then_some((*credential_id, state.candidate))
            })
            .collect::<Vec<_>>();

        for (credential_id, candidate) in due {
            match self
                .store
                .update_service_account_last_used(credential_id, candidate)
                .await
            {
                Ok(()) => {
                    if let Some(state) = states.get_mut(&credential_id) {
                        state.dirty = false;
                        state.last_persisted = Some(Instant::now());
                    }
                }
                Err(error) => {
                    tracing::warn!(%credential_id, %error, "could not persist Service Account token usage");
                }
            }
        }

        let now = Instant::now();
        states.retain(|_, state| {
            state.dirty
                || state
                    .last_persisted
                    .is_some_and(|last| now.duration_since(last) < self.coalesce_window)
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use citadel_identity::IdentityError;
    use futures_util::future::BoxFuture;

    #[derive(Default)]
    struct RecordingStore {
        writes: Mutex<Vec<(Uuid, DateTime<Utc>)>>,
    }

    impl ServiceAccountLastUsedStore for RecordingStore {
        fn update_service_account_last_used(
            &self,
            credential_id: Uuid,
            used_at: DateTime<Utc>,
        ) -> BoxFuture<'_, Result<(), IdentityError>> {
            Box::pin(async move {
                self.writes.lock().unwrap().push((credential_id, used_at));
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn a_full_usage_queue_rejects_without_blocking_or_overwriting() {
        let (tracker, mut worker) = service_account_last_used_channel(
            Arc::new(RecordingStore::default()),
            1,
            StdDuration::from_secs(1),
            StdDuration::from_secs(60),
        );
        let first = Uuid::now_v7();
        tracker.track(first, Utc::now());
        tracker.track(Uuid::now_v7(), Utc::now());
        assert_eq!(
            worker
                .receiver
                .recv(&CancellationToken::new())
                .await
                .unwrap()
                .credential_id,
            first
        );
        let mut metrics = String::new();
        citadel_runtime::runtime_metrics::render_runtime_metrics(&mut metrics);
        let dropped = metrics
            .lines()
            .find(|line| {
                line.starts_with(
                    "citadel_runtime_units_total{family=\"ServiceAccountUsageDropped\"}",
                )
            })
            .unwrap();
        assert!(
            dropped
                .split_whitespace()
                .last()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                >= 1
        );
        let cancel = CancellationToken::new();
        cancel.cancel();
        worker.run(cancel).await.unwrap();
    }

    #[tokio::test]
    async fn coalesces_each_credential_until_the_window_expires() {
        let store = Arc::new(RecordingStore::default());
        let (_, worker) = service_account_last_used_channel(
            store.clone(),
            4,
            StdDuration::from_secs(1),
            StdDuration::from_secs(60),
        );
        let credential_id = Uuid::now_v7();
        let first = Utc::now();
        let mut states = HashMap::from([(
            credential_id,
            UsageState {
                candidate: first,
                last_persisted: None,
                dirty: true,
            },
        )]);

        worker.flush_due(&mut states).await;
        assert_eq!(store.writes.lock().unwrap().len(), 1);

        let state = states.get_mut(&credential_id).unwrap();
        state.candidate = first + chrono::Duration::seconds(5);
        state.dirty = true;
        worker.flush_due(&mut states).await;
        assert_eq!(store.writes.lock().unwrap().len(), 1);

        states.get_mut(&credential_id).unwrap().last_persisted =
            Some(Instant::now() - StdDuration::from_secs(61));
        worker.flush_due(&mut states).await;
        let writes = store.writes.lock().unwrap();
        assert_eq!(writes.len(), 2);
        assert_eq!(
            writes[1],
            (credential_id, first + chrono::Duration::seconds(5))
        );
    }
}
