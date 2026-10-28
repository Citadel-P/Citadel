//! Reconcile terminal attempts without replaying work or changing run outcomes.
use crate::{BuildError, BuildExecutorCapacity, BuildPlatformSnapshot};
use citadel_primitives::ActorId;
mod recovery;
use futures_util::future::BoxFuture;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct BuildCleanupAttempt {
    pub run_id: Uuid,
    pub attempt_id: Uuid,
    pub target: BuildPlatformSnapshot,
    pub daemon_id: String,
    pub authority_id: Uuid,
    pub admission_generation: u64,
    pub receipt_recorded: bool,
}

/// A terminal attempt still holding capacity, including attempts whose admission reply was lost.
#[derive(Clone, Debug)]
pub struct BuildRetainedReservation {
    pub run_id: Uuid,
    pub attempt_id: Uuid,
    pub target: BuildPlatformSnapshot,
    pub daemon_id: String,
    pub authority_id: Uuid,
}

pub trait BuildCleanupStore: Send + Sync {
    fn retained_reservation(
        &self,
        run_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<BuildRetainedReservation>, BuildError>>;
    /// Recheck administrator access, fence the reservation and audit its release in one transaction.
    fn resolve_reservation<'a>(
        &'a self,
        reservation: &'a BuildRetainedReservation,
        actor: ActorId,
        reason: &'a str,
        replacement_authority: Uuid,
    ) -> BoxFuture<'a, Result<bool, BuildError>>;
    fn pending_cleanup(
        &self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<BuildCleanupAttempt>, BuildError>>;
    /// Atomically record evidence, release the exact terminal reservation and log it.
    fn finish_cleanup<'a>(
        &'a self,
        attempt: &'a BuildCleanupAttempt,
    ) -> BoxFuture<'a, Result<bool, BuildError>>;
}

pub trait BuildCleanupChecker: Send + Sync {
    fn recovery_host<'a>(
        &'a self,
        target: &'a BuildPlatformSnapshot,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BuildExecutorCapacity, BuildError>>;
    /// True only for authenticated same-authority/daemon cleanup confirmation.
    fn confirmed<'a>(
        &'a self,
        attempt: &'a BuildCleanupAttempt,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, BuildError>>;
}

pub(crate) struct BuildCleanupService {
    pub store: Arc<dyn BuildCleanupStore>,
    pub checker: Arc<dyn BuildCleanupChecker>,
}

impl BuildCleanupService {
    pub async fn process_batch(
        &self,
        after: Option<Uuid>,
        cancellation: &CancellationToken,
    ) -> Result<(Option<Uuid>, bool), BuildError> {
        let attempts = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok((after, false)),
            result = self.store.pending_cleanup(after, 16) => result?,
        };
        let mut cursor = None;
        let mut changed = false;
        for attempt in attempts {
            // Advance past unavailable hosts too, so they cannot starve later reservations.
            cursor = Some(attempt.attempt_id);
            let confirmed = if attempt.receipt_recorded {
                true
            } else {
                tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Ok((cursor, changed)),
                    result = tokio::time::timeout(Duration::from_secs(5), self.checker.confirmed(&attempt, cancellation)) => {
                        matches!(result, Ok(Ok(true)))
                    }
                }
            };
            if confirmed {
                let result = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Ok((cursor, changed)),
                    result = self.store.finish_cleanup(&attempt) => result,
                };
                match result {
                    Ok(released) => changed |= released,
                    // The next sweep retries the idempotent check/transaction. Never replay execution.
                    Err(_) => {
                        tracing::warn!(run_id=%attempt.run_id, "Build cleanup evidence could not be persisted")
                    }
                }
            }
        }
        Ok((cursor, changed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    struct Store {
        attempts: Vec<BuildCleanupAttempt>,
        completed: Mutex<Vec<Uuid>>,
        fail: AtomicBool,
    }
    impl BuildCleanupStore for Store {
        fn retained_reservation(
            &self,
            _: Uuid,
        ) -> BoxFuture<'_, Result<Option<BuildRetainedReservation>, BuildError>> {
            unimplemented!()
        }
        fn resolve_reservation<'a>(
            &'a self,
            _: &'a BuildRetainedReservation,
            _: ActorId,
            _: &'a str,
            _: Uuid,
        ) -> BoxFuture<'a, Result<bool, BuildError>> {
            unimplemented!()
        }
        fn pending_cleanup(
            &self,
            after: Option<Uuid>,
            limit: usize,
        ) -> BoxFuture<'_, Result<Vec<BuildCleanupAttempt>, BuildError>> {
            Box::pin(async move {
                Ok(self
                    .attempts
                    .iter()
                    .filter(|a| {
                        after.is_none_or(|id| a.attempt_id > id)
                            && !self.completed.lock().unwrap().contains(&a.attempt_id)
                    })
                    .take(limit)
                    .cloned()
                    .collect())
            })
        }
        fn finish_cleanup<'a>(
            &'a self,
            attempt: &'a BuildCleanupAttempt,
        ) -> BoxFuture<'a, Result<bool, BuildError>> {
            Box::pin(async move {
                if self.fail.load(Ordering::SeqCst) {
                    return Err(BuildError::Storage("fixture".into()));
                }
                self.completed.lock().unwrap().push(attempt.attempt_id);
                Ok(true)
            })
        }
    }
    struct Checker {
        calls: Mutex<Vec<Uuid>>,
        block: bool,
    }
    impl BuildCleanupChecker for Checker {
        fn recovery_host<'a>(
            &'a self,
            _: &'a BuildPlatformSnapshot,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<BuildExecutorCapacity, BuildError>> {
            unimplemented!()
        }
        fn confirmed<'a>(
            &'a self,
            attempt: &'a BuildCleanupAttempt,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<bool, BuildError>> {
            Box::pin(async move {
                self.calls.lock().unwrap().push(attempt.attempt_id);
                if self.block {
                    std::future::pending::<()>().await;
                }
                // First page stays unavailable. Later candidates must still be visited.
                if attempt.attempt_id.as_u128() <= 16 {
                    return Err(BuildError::NotFound);
                }
                Ok(true)
            })
        }
    }
    pub(super) fn attempt(id: u128) -> BuildCleanupAttempt {
        BuildCleanupAttempt {
            run_id: Uuid::now_v7(),
            attempt_id: Uuid::from_u128(id),
            target: BuildPlatformSnapshot {
                id: Some(Uuid::now_v7()),
                name: None,
                address: None,
                builder_kind: Some("Platform".into()),
                build_agent_pool_id: None,
            },
            daemon_id: "fixture".into(),
            authority_id: Uuid::now_v7(),
            admission_generation: 1,
            receipt_recorded: false,
        }
    }
    #[tokio::test]
    async fn sweeps_past_unreachable_attempts_and_retries_failed_persistence() {
        let store = Arc::new(Store {
            attempts: (1..=20).map(attempt).collect(),
            completed: Mutex::default(),
            fail: AtomicBool::new(true),
        });
        let checker = Arc::new(Checker {
            calls: Mutex::default(),
            block: false,
        });
        let service = BuildCleanupService {
            store: store.clone(),
            checker: checker.clone(),
        };
        let cancel = CancellationToken::new();
        let (cursor, changed) = service.process_batch(None, &cancel).await.unwrap();
        assert_eq!(cursor, Some(Uuid::from_u128(16)));
        assert!(!changed);
        let (cursor, changed) = service.process_batch(cursor, &cancel).await.unwrap();
        assert_eq!(cursor, Some(Uuid::from_u128(20)));
        assert!(!changed);
        assert_eq!(
            service.process_batch(cursor, &cancel).await.unwrap(),
            (None, false)
        );
        store.fail.store(false, Ordering::SeqCst);
        let (cursor, _) = service.process_batch(None, &cancel).await.unwrap();
        assert!(service.process_batch(cursor, &cancel).await.unwrap().1);
        assert_eq!(store.completed.lock().unwrap().len(), 4);
        assert_eq!(checker.calls.lock().unwrap().len(), 40);
    }
    #[tokio::test]
    async fn durable_receipts_do_not_require_a_reachable_agent() {
        let mut known = attempt(1);
        known.receipt_recorded = true;
        let store = Arc::new(Store {
            attempts: vec![known],
            completed: Mutex::default(),
            fail: AtomicBool::new(false),
        });
        let checker = Arc::new(Checker {
            calls: Mutex::default(),
            block: true,
        });
        let service = BuildCleanupService {
            store,
            checker: checker.clone(),
        };
        assert!(
            service
                .process_batch(None, &CancellationToken::new())
                .await
                .unwrap()
                .1
        );
        assert!(checker.calls.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn cancellation_interrupts_a_stalled_check_without_releasing_capacity() {
        let store = Arc::new(Store {
            attempts: vec![attempt(17)],
            completed: Mutex::default(),
            fail: AtomicBool::new(false),
        });
        let checker = Arc::new(Checker {
            calls: Mutex::default(),
            block: true,
        });
        let service = BuildCleanupService {
            store: store.clone(),
            checker: checker.clone(),
        };
        let cancel = CancellationToken::new();
        let stop = async {
            while checker.calls.lock().unwrap().is_empty() {
                tokio::task::yield_now().await;
            }
            cancel.cancel();
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(service.process_batch(None, &cancel), stop)
        })
        .await
        .unwrap();
        assert!(!result.unwrap().1);
        assert!(store.completed.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn timed_out_checks_do_not_block_the_rest_of_the_batch() {
        let mut known = attempt(18);
        known.receipt_recorded = true;
        let store = Arc::new(Store {
            attempts: vec![attempt(17), known],
            completed: Mutex::default(),
            fail: AtomicBool::new(false),
        });
        let checker = Arc::new(Checker {
            calls: Mutex::default(),
            block: true,
        });
        let service = BuildCleanupService {
            store: store.clone(),
            checker,
        };
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            service.process_batch(None, &CancellationToken::new()),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(result.1);
        assert_eq!(*store.completed.lock().unwrap(), vec![Uuid::from_u128(18)]);
    }
}
